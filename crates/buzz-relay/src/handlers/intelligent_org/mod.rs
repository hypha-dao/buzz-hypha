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
//! - one handler per command, `shapers` for `50001`/`50019`/`50020` in this
//!   slice.
//!
//! Client `EVENT`s of `39100–39105` never reach here: ingest rejects them as
//! `restricted: relay-only kind` before verification.

pub mod apply;
pub mod authorize;
#[cfg(test)]
mod postgres_tests;
mod shapers;
pub mod state;

use std::sync::Arc;

use buzz_core::intelligent_org::tag;
use buzz_core::kind::{
    is_intelligent_org_command_kind, KIND_IO_JOIN_PROPOSE, KIND_IO_MONEY_PROPOSE,
    KIND_IO_MONEY_RELEASED, KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT,
    KIND_IO_SHAPER_STEP_DOWN,
};
use buzz_core::tenant::TenantContext;
use nostr::Event;
use serde::de::DeserializeOwned;
use tracing::warn;
use uuid::Uuid;

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
    if has_draft_tag(event) {
        return Err(IngestError::Rejected(
            "invalid: draft settlement is not implemented yet".into(),
        ));
    }
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
        KIND_IO_SHAPER_ACCEPT => shapers::accept(&cmd).await,
        KIND_IO_SHAPER_STEP_DOWN => shapers::step_down(&cmd).await,
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
}

/// `["e", <id>, "", "draft"]` on any command (§3.2 draft settlement).
fn has_draft_tag(event: &Event) -> bool {
    event.tags.iter().any(|t| {
        let parts = t.as_slice();
        parts.len() >= 4 && parts[0] == "e" && parts[3] == tag::MARKER_DRAFT
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
