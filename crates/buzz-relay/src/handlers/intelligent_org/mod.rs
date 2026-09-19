//! Intelligent organization — the command executor's org module (Protocol §6.1).
//!
//! Ingest routes every `50001–50021` here through
//! [`super::command_executor::handle_command`] after signature, freshness,
//! identity, and scope checks. This module owns:
//!
//! - [`state`] — builders and the relay signature for `39100–39105`;
//! - [`authorize`] — the pure who-may-send decisions of §3.2;
//! - [`apply`] — the single write path: projection row, state event, ledger,
//!   and the `#shapers` roster, all on the command's transaction (V3);
//! - [`proposals`] — opening, the D1 opener vote, `50003`, the tally, and
//!   execution dispatch (§5.3);
//! - [`drafts`] — `50100` / `50101` / `50103` ingest, draft settlement on
//!   any command carrying `["e", id, "", "draft"]`, `50012`, `50017`;
//! - one handler per command: `shapers` for `50001`/`50019`/`50020` and the
//!   `shapers` execution rows, `proposals` for `50002`/`50004`/`50015`/`50003`
//!   and the `direction` / `dri` execution rows, `work` for `project`
//!   execution and `50005`–`50011` / `50018`.
//!
//! Client `EVENT`s of `39100–39105` never reach here: ingest rejects them as
//! `restricted: relay-only kind` before verification.

pub mod apply;
pub mod authorize;
mod drafts;
#[cfg(test)]
#[path = "drafts_postgres_tests.rs"]
mod drafts_postgres_tests;
#[cfg(test)]
mod postgres_tests;
mod proposals;
mod shapers;
pub mod state;
mod work;

use std::sync::Arc;

use buzz_core::intelligent_org::Shapers;
use buzz_core::kind::{
    is_intelligent_org_command_kind, KIND_IO_ACCEPT, KIND_IO_AGENT_NOTE, KIND_IO_DECLINE,
    KIND_IO_DIRECTION_PROPOSE, KIND_IO_DONE, KIND_IO_DRAFT, KIND_IO_DRAFT_DECIDE,
    KIND_IO_DRI_PROPOSE, KIND_IO_HEALTH, KIND_IO_HEALTH_RATE, KIND_IO_JOIN_PROPOSE,
    KIND_IO_MONEY_PROPOSE, KIND_IO_MONEY_RELEASED, KIND_IO_OFFER, KIND_IO_PROJECT_PROPOSE,
    KIND_IO_RELEASE, KIND_IO_REOPEN, KIND_IO_SET_DUE, KIND_IO_SHAPERS_PROPOSE,
    KIND_IO_SHAPER_ACCEPT, KIND_IO_SHAPER_STEP_DOWN, KIND_IO_TICKET_CREATE, KIND_IO_VOTE,
};
use buzz_core::tenant::TenantContext;
use buzz_db::intelligent_org::{self as store, LedgerEntry};
use nostr::Event;
use serde::de::DeserializeOwned;
use sqlx::{Postgres, Transaction};
use tracing::warn;
use uuid::Uuid;

use super::command_executor::{persist_command_event, PersistResult};
use super::ingest::{IngestAuth, IngestError, IngestResult};
use crate::state::AppState;

/// Route an intelligent-org command to its handler.
pub async fn handle_command(
    tenant: &TenantContext,
    state: &Arc<AppState>,
    event: &Event,
    auth: &IngestAuth,
) -> Result<IngestResult, IngestError> {
    let kind = event.kind.as_u16() as u32;
    let cmd = Command {
        tenant,
        state,
        event,
        actor_hex: auth.pubkey().to_hex(),
        actor_bytes: auth.pubkey().to_bytes().to_vec(),
        at: event.created_at.as_secs(),
    };
    match kind {
        KIND_IO_SHAPERS_PROPOSE => shapers::propose(&cmd).await,
        KIND_IO_DIRECTION_PROPOSE => proposals::direction_propose(&cmd).await,
        KIND_IO_PROJECT_PROPOSE => proposals::project_propose(&cmd).await,
        KIND_IO_DRI_PROPOSE => proposals::dri_propose(&cmd).await,
        KIND_IO_TICKET_CREATE => work::create(&cmd).await,
        KIND_IO_OFFER => work::offer(&cmd).await,
        KIND_IO_ACCEPT => work::accept(&cmd).await,
        KIND_IO_DECLINE => work::decline(&cmd).await,
        KIND_IO_DONE => work::done(&cmd).await,
        KIND_IO_RELEASE => work::release(&cmd).await,
        KIND_IO_SET_DUE => work::set_due(&cmd).await,
        KIND_IO_REOPEN => work::reopen(&cmd).await,
        KIND_IO_VOTE => proposals::vote(&cmd).await,
        KIND_IO_SHAPER_ACCEPT => shapers::accept(&cmd).await,
        KIND_IO_SHAPER_STEP_DOWN => shapers::step_down(&cmd).await,
        KIND_IO_DRAFT_DECIDE => drafts::decide(&cmd).await,
        KIND_IO_HEALTH_RATE => drafts::health_rate(&cmd).await,
        KIND_IO_MONEY_PROPOSE | KIND_IO_MONEY_RELEASED => Err(IngestError::Rejected(
            "restricted: money not enabled".into(),
        )),
        KIND_IO_JOIN_PROPOSE => Err(IngestError::Rejected("restricted: join not enabled".into())),
        k if is_intelligent_org_command_kind(k) => Err(IngestError::Rejected(format!(
            "invalid: kind {k} is not implemented yet"
        ))),
        _ => Err(IngestError::Rejected(format!(
            "unknown command kind: {kind}"
        ))),
    }
}

/// Route an agent-facing read (`50100` / `50101` / `50103`). `50102` stays
/// unknown until Work sync.
pub async fn handle_read(
    tenant: &TenantContext,
    state: &Arc<AppState>,
    event: &Event,
    auth: &IngestAuth,
) -> Result<IngestResult, IngestError> {
    let pubkey_bytes = auth.pubkey().to_bytes().to_vec();
    if let Err(e) = state
        .db
        .ensure_user(tenant.community(), &pubkey_bytes)
        .await
    {
        warn!("intelligent-org: ensure_user failed: {e}");
    }
    let kind = event.kind.as_u16() as u32;
    let cmd = Command {
        tenant,
        state,
        event,
        actor_hex: auth.pubkey().to_hex(),
        actor_bytes: auth.pubkey().to_bytes().to_vec(),
        at: event.created_at.as_secs(),
    };
    match kind {
        KIND_IO_DRAFT => drafts::ingest_draft(&cmd).await,
        KIND_IO_HEALTH => drafts::ingest_health(&cmd).await,
        KIND_IO_AGENT_NOTE => drafts::ingest_note(&cmd).await,
        _ => Err(IngestError::Rejected(format!("unknown read kind: {kind}"))),
    }
}

/// One verified command and who sent it.
pub(crate) struct Command<'a> {
    /// The community.
    pub tenant: &'a TenantContext,
    /// Relay state.
    pub state: &'a Arc<AppState>,
    /// The signed command.
    pub event: &'a Event,
    /// The author, hex.
    pub actor_hex: String,
    /// The author, 32 bytes.
    pub actor_bytes: Vec<u8>,
    /// The command's `created_at` — the protocol time of everything it causes.
    pub at: u64,
}

impl Command<'_> {
    /// The command id as the `receipt` of what it produced.
    pub fn receipt_hex(&self) -> String {
        self.event.id.to_hex()
    }

    /// The command id for ledger `receipt_event_id`.
    pub fn receipt_bytes(&self) -> Vec<u8> {
        self.event.id.to_bytes().to_vec()
    }

    /// The accepted result for this command.
    pub fn accepted(&self, message: String) -> IngestResult {
        IngestResult {
            event_id: self.event.id.to_hex(),
            accepted: true,
            message,
        }
    }

    /// A ledger row this command causes, stamped at the command's time and
    /// pointing back at it as the receipt.
    pub fn ledger(
        &self,
        verb: &str,
        object_type: &str,
        object_id: &str,
        detail: serde_json::Value,
    ) -> Result<LedgerEntry, IngestError> {
        Ok(LedgerEntry {
            at: store::ts(self.at).map_err(|e| internal("ledger time", e))?,
            actor: self.actor_hex.clone(),
            verb: verb.to_owned(),
            object_type: object_type.to_owned(),
            object_id: object_id.to_owned(),
            receipt_event_id: Some(self.receipt_bytes()),
            detail,
        })
    }

    /// Whether `pubkey` holds a NIP-43 relay membership in this community.
    pub async fn is_member(&self, pubkey: &str) -> Result<bool, IngestError> {
        Ok(self
            .state
            .db
            .get_relay_member(self.tenant.community(), pubkey)
            .await
            .map_err(|e| internal("read relay membership", e))?
            .is_some())
    }
}

/// Ledger `object_type` values this module writes.
pub(crate) mod object {
    /// The Shaper set; `object_id` is a pubkey.
    pub const SHAPERS: &str = "shapers";
    /// A proposal; `object_id` is its uuid.
    pub const PROPOSAL: &str = "proposal";
    /// The org agent; `object_id` is its pubkey.
    pub const AGENT: &str = "agent";
    /// A direction artifact; `object_id` is the slug.
    pub const DIRECTION: &str = "direction";
    /// A work item; `object_id` is its uuid.
    pub const WORK_ITEM: &str = "work_item";
    /// A draft; `object_id` is the `50100` hex.
    pub const DRAFT: &str = "draft";
}

pub(crate) fn internal(context: &str, error: impl std::fmt::Display) -> IngestError {
    IngestError::Internal(format!("error: {context}: {error}"))
}

/// Relay wall-clock seconds — the floor for state `created_at` (see `apply`).
pub(crate) fn wall_clock() -> u64 {
    chrono::Utc::now().timestamp().unsigned_abs()
}

/// What `persist_command_event` said about the command.
pub(crate) enum Persisted {
    /// Already processed; the accepted result to return.
    Replay(IngestResult),
    /// Stored on this transaction, which now also holds the executor lock.
    Open(Transaction<'static, Postgres>),
}

/// Store the command and take the executor lock, or report a replay.
pub(crate) async fn begin(cmd: &Command<'_>) -> Result<Persisted, IngestError> {
    let mut tx = match persist_command_event(&cmd.state.db, cmd.tenant, cmd.event, None).await? {
        PersistResult::Duplicate => {
            return Ok(Persisted::Replay(
                cmd.accepted("duplicate: already processed".into()),
            ));
        }
        PersistResult::Inserted(tx) => tx,
    };
    store::lock_executor(&mut tx, cmd.tenant.community())
        .await
        .map_err(|e| internal("take executor lock", e))?;
    Ok(Persisted::Open(tx))
}

pub(crate) async fn commit(tx: Transaction<'static, Postgres>) -> Result<(), IngestError> {
    tx.commit()
        .await
        .map_err(|e| internal("commit transaction", e))
}

/// Attach draft settlement (when tagged), write projections and ledger,
/// commit, and fan out. Every command and read that mutates org state ends
/// here so a draft tag and the command share one transaction. Boxed so the
/// caller's future stays small (debug `settle` overflowed the default stack).
pub(crate) fn persist_write<'a>(
    cmd: &'a Command<'_>,
    tx: Transaction<'static, Postgres>,
    projections: Vec<apply::Projection>,
    rows: Vec<LedgerEntry>,
    message: String,
    room_created: Option<Uuid>,
) -> impl std::future::Future<Output = Result<IngestResult, IngestError>> + 'a {
    Box::pin(persist_write_inner(
        cmd,
        tx,
        projections,
        rows,
        message,
        room_created,
    ))
}

async fn persist_write_inner(
    cmd: &Command<'_>,
    mut tx: Transaction<'static, Postgres>,
    mut projections: Vec<apply::Projection>,
    mut rows: Vec<LedgerEntry>,
    message: String,
    room_created: Option<Uuid>,
) -> Result<IngestResult, IngestError> {
    drafts::attach_settlement(cmd, &mut tx, &mut projections, &mut rows).await?;
    let ctx = apply::ApplyContext {
        community: cmd.tenant.community(),
        relay: &cmd.state.relay_keypair,
        actor: &cmd.actor_bytes,
        now: wall_clock(),
    };
    let applied = apply::apply(&cmd.state.db, &mut tx, &ctx, &projections, &rows).await?;
    commit(tx).await?;
    finish(cmd, applied, room_created).await;
    Ok(cmd.accepted(message))
}

/// The live `39103` content on `tx`, if the community has one.
pub(crate) async fn current_shapers(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
) -> Result<Option<Shapers>, IngestError> {
    Ok(store::get_shapers(tx, cmd.tenant.community())
        .await
        .map_err(|e| internal("read io_shapers", e))?
        .map(|row| row.content))
}

/// The first `name` tag as 32 raw bytes from a 64-char lowercase hex value.
/// Used for draft event ids (`50012` `e`, the settlement marker) — never
/// through nostr `EventId` helpers (C-2 leftover).
pub(crate) fn hex32_tag(event: &Event, name: &str) -> Result<Option<Vec<u8>>, IngestError> {
    let Some(value) = tag_value(event, name) else {
        return Ok(None);
    };
    let is_hex = value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !is_hex {
        return Err(IngestError::Rejected(format!(
            "invalid: {name} tag must be a 64-char lowercase hex id"
        )));
    }
    hex::decode(value).map(Some).map_err(|_| {
        IngestError::Rejected(format!(
            "invalid: {name} tag must be a 64-char lowercase hex id"
        ))
    })
}

/// The first value of tag `name`.
pub(crate) fn tag_value<'e>(event: &'e Event, name: &str) -> Option<&'e str> {
    event.tags.iter().find_map(|t| {
        let parts = t.as_slice();
        (parts.first().map(String::as_str) == Some(name))
            .then(|| parts.get(1).map(String::as_str))
            .flatten()
    })
}

/// The first `name` tag as a lowercase 64-hex pubkey, or `None` when absent.
pub(crate) fn pubkey_tag(event: &Event, name: &str) -> Result<Option<String>, IngestError> {
    let Some(value) = tag_value(event, name) else {
        return Ok(None);
    };
    let is_hex = value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !is_hex {
        return Err(IngestError::Rejected(format!(
            "invalid: {name} tag must be a 64-char lowercase hex pubkey"
        )));
    }
    Ok(Some(value.to_owned()))
}

/// The first `name` tag as a unix-seconds timestamp (`due` on `50011`).
pub(crate) fn timestamp_tag(event: &Event, name: &str) -> Result<Option<u64>, IngestError> {
    tag_value(event, name)
        .map(|v| {
            v.parse::<u64>().map_err(|_| {
                IngestError::Rejected(format!("invalid: {name} tag must be a timestamp"))
            })
        })
        .transpose()
}

/// The first `name` tag as a `u32` version (`base` on `50002`).
pub(crate) fn version_tag(event: &Event, name: &str) -> Result<Option<u32>, IngestError> {
    tag_value(event, name)
        .map(|v| {
            v.parse::<u32>().map_err(|_| {
                IngestError::Rejected(format!("invalid: {name} tag must be a version number"))
            })
        })
        .transpose()
}

/// The first `name` tag as a UUID.
pub(crate) fn uuid_tag(event: &Event, name: &str) -> Result<Option<Uuid>, IngestError> {
    tag_value(event, name)
        .map(|v| {
            Uuid::parse_str(v)
                .map_err(|_| IngestError::Rejected(format!("invalid: {name} tag must be a uuid")))
        })
        .transpose()
}

/// Parse the command content (§4.8). Empty content reads as `{}`.
pub(crate) fn parse_content<T: DeserializeOwned + Default>(
    event: &Event,
) -> Result<T, IngestError> {
    if event.content.trim().is_empty() {
        return Ok(T::default());
    }
    serde_json::from_str(&event.content)
        .map_err(|e| IngestError::Rejected(format!("invalid: command content: {e}")))
}

/// The command content as JSON, verbatim, for a proposal's `payload`.
pub(crate) fn content_value(event: &Event) -> Result<serde_json::Value, IngestError> {
    if event.content.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(&event.content)
        .map_err(|e| IngestError::Rejected(format!("invalid: command content: {e}")))
}

/// Post-commit, best-effort fan-out of what a command produced: the command
/// itself and each state event to live subscribers, NIP-29 discovery and
/// membership notices for a room whose roster moved, and the membership
/// caches those rows feed. Nothing here can fail the command — it is
/// already durable.
pub(crate) async fn finish(cmd: &Command<'_>, applied: apply::Applied, room_created: Option<Uuid>) {
    use buzz_core::kind::{KIND_MEMBER_ADDED_NOTIFICATION, KIND_MEMBER_REMOVED_NOTIFICATION};
    use buzz_db::relay_rooms::RosterChange;

    let command = buzz_core::event::StoredEvent::with_received_at(
        cmd.event.clone(),
        chrono::Utc::now(),
        None,
        true,
    );
    super::event::dispatch_persistent_event(
        cmd.tenant,
        cmd.state,
        &command,
        cmd.event.kind.as_u16() as u32,
        &cmd.actor_hex,
        None,
    )
    .await;
    let relay_hex = cmd.state.relay_keypair.public_key().to_hex();
    for stored in &applied.state_events {
        super::event::dispatch_persistent_event(
            cmd.tenant,
            cmd.state,
            stored,
            stored.event.kind.as_u16() as u32,
            &relay_hex,
            None,
        )
        .await;
    }

    let mut rooms: Vec<Uuid> = room_created.into_iter().collect();
    for (room, change) in &applied.roster {
        if !rooms.contains(room) {
            rooms.push(*room);
        }
        cmd.state
            .invalidate_membership(cmd.tenant, *room, change.pubkey());
        let notification = match change {
            RosterChange::Added { .. } => KIND_MEMBER_ADDED_NOTIFICATION,
            RosterChange::Removed { .. } => KIND_MEMBER_REMOVED_NOTIFICATION,
        };
        if let Err(e) = super::side_effects::emit_membership_notification(
            cmd.tenant,
            cmd.state,
            *room,
            change.pubkey(),
            &cmd.actor_bytes,
            notification,
        )
        .await
        {
            warn!(room = %room, error = %e, "intelligent-org: membership notification failed");
        }
    }
    for room in rooms {
        if let Err(e) =
            super::side_effects::emit_group_discovery_events(cmd.tenant, cmd.state, room).await
        {
            warn!(room = %room, error = %e, "intelligent-org: NIP-29 discovery emission failed");
        }
    }
}
