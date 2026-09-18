//! Intelligent organization projections (Protocol §6.2).
//!
//! Typed reads and writes over the `io_*` tables that migration 0045 creates.
//! The relay-signed state events (`39100–39105`) remain the wire truth; these
//! rows are the executor's, scheduler's, and agent's query surface for the
//! same state. Every row keeps the canonical Protocol §4 content — the
//! [`buzz_core::intelligent_org`] type — next to the typed columns the relay
//! filters on, so a projection can re-emit its event without a second lookup.
//!
//! Every function takes a `&mut PgConnection` so the executor can run it on
//! the transaction `persist_command_event` returns (V3): the command event,
//! the projection change, the ledger row, and the state event either all
//! commit or none do. Nothing here opens a transaction or touches the pool —
//! except the [`Db`] reads at the bottom, which the relay's HTTP handlers
//! outside the executor (invite mint, the invite landing page) call as one
//! pool-scoped lookup each.
//!
//! Pubkeys and event ids are hex in content and `BYTEA` in the tables;
//! conversion failures surface as [`DbError::InvalidData`] rather than
//! panicking.

use buzz_core::intelligent_org::{
    DirectionArtifact, DirectionSlug, DraftKind, DraftOrigin, DraftOutcome, DraftOutcomeStatus,
    HealthBand, HealthRead, ProgressHint, ProgressNote, Proposal, ProposalStatus, Shapers,
    VoteChoice, WorkItem, WorkItemState,
};
use buzz_core::CommunityId;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::{observability, Db, DbError, Result};

// ── Conversions ───────────────────────────────────────────────────────────────

/// Decode a 32-byte hex id (pubkey or event id) for a `BYTEA` bind.
pub fn hex32(value: &str) -> Result<Vec<u8>> {
    let bytes = hex::decode(value)
        .map_err(|e| DbError::InvalidData(format!("expected 32-byte hex id: {e}")))?;
    if bytes.len() != 32 {
        return Err(DbError::InvalidData(format!(
            "expected 32-byte hex id, got {} bytes",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn opt_hex32(value: Option<&str>) -> Result<Option<Vec<u8>>> {
    value.map(hex32).transpose()
}

fn parse_uuid(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value).map_err(|e| DbError::InvalidData(format!("expected uuid: {e}")))
}

fn opt_uuid(value: Option<&str>) -> Result<Option<Uuid>> {
    value.map(parse_uuid).transpose()
}

/// Protocol timestamps are unix seconds; the tables store `TIMESTAMPTZ`.
pub fn ts(seconds: u64) -> Result<DateTime<Utc>> {
    let secs = i64::try_from(seconds).map_err(|_| DbError::InvalidTimestamp(i64::MAX))?;
    DateTime::from_timestamp(secs, 0).ok_or(DbError::InvalidTimestamp(secs))
}

fn opt_ts(seconds: Option<u64>) -> Result<Option<DateTime<Utc>>> {
    seconds.map(ts).transpose()
}

/// The serde wire string of a unit enum (`WorkItemState::InReview` → `in_review`).
fn wire<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(DbError::InvalidData(format!(
            "expected a string-valued enum, got {other}"
        ))),
    }
}

fn from_wire<T: DeserializeOwned>(value: &str) -> Result<T> {
    Ok(serde_json::from_value(serde_json::Value::String(
        value.to_owned(),
    ))?)
}

fn content<T: DeserializeOwned>(row: &sqlx::postgres::PgRow, column: &str) -> Result<T> {
    let value: serde_json::Value = row.try_get(column)?;
    Ok(serde_json::from_value(value)?)
}

// ── Executor serialization ────────────────────────────────────────────────────

const IO_EXECUTOR_LOCK_NAMESPACE: &str = "buzz_io_executor:";

/// Take the community's intelligent-org executor lock for the rest of the
/// caller's transaction (`pg_advisory_xact_lock`).
///
/// Every command that reads a projection and then writes it — bootstrap,
/// a seat accept, a step-down, a vote — takes this first, so two commands on
/// the same community serialize and the second sees the first's committed
/// state instead of a stale read (Protocol §6.1: one write path).
pub async fn lock_executor(conn: &mut PgConnection, community_id: CommunityId) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!(
            "{IO_EXECUTOR_LOCK_NAMESPACE}{}",
            community_id.as_uuid()
        ))
        .execute(conn)
        .await?;
    Ok(())
}

/// `created_at` (unix seconds) of the live head of a relay-signed state
/// coordinate — `(kind, relay pubkey, d tag)` — or `None` when never written.
///
/// The executor emits every new head strictly after the previous one so
/// NIP-33 ordering never drops a same-second rewrite on a random id tiebreak.
pub async fn state_head_created_at(
    conn: &mut PgConnection,
    community_id: CommunityId,
    kind: u32,
    relay_pubkey: &[u8],
    d_tag: &str,
) -> Result<Option<u64>> {
    let kind = i32::try_from(kind)
        .map_err(|_| DbError::InvalidData(format!("state kind {kind} overflows i32")))?;
    let value: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT created_at FROM events \
         WHERE community_id = $1 AND kind = $2 AND pubkey = $3 AND d_tag = $4 \
           AND deleted_at IS NULL \
         ORDER BY created_at DESC, id ASC LIMIT 1",
    )
    .bind(community_id.as_uuid())
    .bind(kind)
    .bind(relay_pubkey)
    .bind(d_tag)
    .fetch_optional(conn)
    .await?;
    Ok(value.map(|t| t.timestamp().unsigned_abs()))
}

// ── io_shapers (kind:39103) ───────────────────────────────────────────────────

/// The community's Shapers row: the latest `39103` and its event id.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapersRow {
    /// Canonical §4.5 content.
    pub content: Shapers,
    /// The channel id of `#shapers`, once created.
    pub room_channel_id: Option<Uuid>,
    /// Id of the relay-signed `39103` this row mirrors.
    pub event_id: Vec<u8>,
    /// The event's `created_at`.
    pub updated_at: DateTime<Utc>,
}

/// Write the community's single Shapers row, replacing any previous one.
pub async fn upsert_shapers(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &ShapersRow,
) -> Result<()> {
    let shapers = row
        .content
        .shapers
        .iter()
        .map(|p| hex32(p))
        .collect::<Result<Vec<_>>>()?;
    sqlx::query(
        "INSERT INTO io_shapers \
            (community_id, founder, shapers, room_channel_id, agent, agent_hosted, \
             content, event_id, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (community_id) DO UPDATE SET \
            founder = EXCLUDED.founder, shapers = EXCLUDED.shapers, \
            room_channel_id = EXCLUDED.room_channel_id, agent = EXCLUDED.agent, \
            agent_hosted = EXCLUDED.agent_hosted, content = EXCLUDED.content, \
            event_id = EXCLUDED.event_id, updated_at = EXCLUDED.updated_at",
    )
    .bind(community_id.as_uuid())
    .bind(hex32(&row.content.founder)?)
    .bind(shapers)
    .bind(row.room_channel_id)
    .bind(opt_hex32(row.content.agent.as_deref())?)
    .bind(row.content.agent_hosted)
    .bind(serde_json::to_value(&row.content)?)
    .bind(&row.event_id)
    .bind(row.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Read the community's Shapers row; `None` before bootstrap.
pub async fn get_shapers(
    conn: &mut PgConnection,
    community_id: CommunityId,
) -> Result<Option<ShapersRow>> {
    let row = sqlx::query(
        "SELECT content, room_channel_id, event_id, updated_at \
         FROM io_shapers WHERE community_id = $1",
    )
    .bind(community_id.as_uuid())
    .fetch_optional(conn)
    .await?;
    row.map(|r| {
        Ok(ShapersRow {
            content: content(&r, "content")?,
            room_channel_id: r.try_get("room_channel_id")?,
            event_id: r.try_get("event_id")?,
            updated_at: r.try_get("updated_at")?,
        })
    })
    .transpose()
}

// ── io_direction (kind:39100) ─────────────────────────────────────────────────

/// One confirmed version of a direction artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectionRow {
    /// Canonical §4.1 content of this version.
    pub content: DirectionArtifact,
    /// Id of the relay-signed `39100` for this version.
    pub event_id: Vec<u8>,
}

/// Insert one direction version. Versions are immutable: a repeat of the same
/// `(slug, version)` is a unique-violation error, never an overwrite.
pub async fn insert_direction(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &DirectionRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_direction \
            (community_id, slug, version, content, event_id, proposal_id, confirmed_by, confirmed_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(community_id.as_uuid())
    .bind(wire(&row.content.slug)?)
    .bind(i32::try_from(row.content.version).map_err(|_| {
        DbError::InvalidData(format!("direction version {} overflows", row.content.version))
    })?)
    .bind(serde_json::to_value(&row.content)?)
    .bind(&row.event_id)
    .bind(opt_uuid(Some(row.content.proposal.as_str()))?)
    .bind(hex32(&row.content.confirmed_by)?)
    .bind(ts(row.content.confirmed_at)?)
    .execute(conn)
    .await?;
    Ok(())
}

/// The newest confirmed version of one artifact; `None` when never written.
pub async fn get_direction_head(
    conn: &mut PgConnection,
    community_id: CommunityId,
    slug: DirectionSlug,
) -> Result<Option<DirectionRow>> {
    let row = sqlx::query(
        "SELECT content, event_id FROM io_direction \
         WHERE community_id = $1 AND slug = $2 \
         ORDER BY version DESC LIMIT 1",
    )
    .bind(community_id.as_uuid())
    .bind(wire(&slug)?)
    .fetch_optional(conn)
    .await?;
    row.map(|r| {
        Ok(DirectionRow {
            content: content(&r, "content")?,
            event_id: r.try_get("event_id")?,
        })
    })
    .transpose()
}

// ── io_work_items (kind:39101) ────────────────────────────────────────────────

/// One node of the work tree: the latest `39101` plus the timestamps the
/// content does not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkItemRow {
    /// Canonical §4.2 content.
    pub content: WorkItem,
    /// Id of the relay-signed `39101` this row mirrors.
    pub event_id: Vec<u8>,
    /// `created_at` of the newest `50102`, for the Work door's _last moved_.
    pub last_progress_at: Option<DateTime<Utc>>,
    /// When the item reached `done`.
    pub done_at: Option<DateTime<Utc>>,
    /// When the item was created.
    pub created_at: DateTime<Utc>,
    /// The event's `created_at`.
    pub updated_at: DateTime<Utc>,
}

/// Write a work item, replacing the previous projection of the same id.
pub async fn upsert_work_item(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &WorkItemRow,
) -> Result<()> {
    let item = &row.content;
    let (channel_id, repo_coord, project_coord) = match &item.home {
        Some(home) => (
            Some(parse_uuid(&home.channel)?),
            home.repo.clone(),
            home.project.clone(),
        ),
        None => (None, None, None),
    };
    sqlx::query(
        "INSERT INTO io_work_items \
            (community_id, id, root_id, parent_id, depth, kind, state, dri, offered_to, \
             offered_at, due_at, approved_at, done_at, channel_id, repo_coord, project_coord, \
             branch, last_progress_at, content, event_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
                 $17, $18, $19, $20, $21, $22) \
         ON CONFLICT (community_id, id) DO UPDATE SET \
            root_id = EXCLUDED.root_id, parent_id = EXCLUDED.parent_id, \
            depth = EXCLUDED.depth, kind = EXCLUDED.kind, state = EXCLUDED.state, \
            dri = EXCLUDED.dri, offered_to = EXCLUDED.offered_to, \
            offered_at = EXCLUDED.offered_at, due_at = EXCLUDED.due_at, \
            approved_at = EXCLUDED.approved_at, done_at = EXCLUDED.done_at, \
            channel_id = EXCLUDED.channel_id, repo_coord = EXCLUDED.repo_coord, \
            project_coord = EXCLUDED.project_coord, branch = EXCLUDED.branch, \
            last_progress_at = EXCLUDED.last_progress_at, content = EXCLUDED.content, \
            event_id = EXCLUDED.event_id, updated_at = EXCLUDED.updated_at",
    )
    .bind(community_id.as_uuid())
    .bind(parse_uuid(&item.id)?)
    .bind(parse_uuid(&item.root)?)
    .bind(opt_uuid(item.parent.as_deref())?)
    .bind(
        i32::try_from(item.depth).map_err(|_| {
            DbError::InvalidData(format!("work item depth {} overflows", item.depth))
        })?,
    )
    .bind(if item.parent.is_none() {
        "project"
    } else {
        "ticket"
    })
    .bind(wire(&item.state)?)
    .bind(opt_hex32(item.dri.as_deref())?)
    .bind(opt_hex32(item.offered_to.as_deref())?)
    .bind(opt_ts(item.offered_at)?)
    .bind(ts(item.due_at)?)
    .bind(opt_ts(item.approved_at)?)
    .bind(row.done_at)
    .bind(channel_id)
    .bind(repo_coord)
    .bind(project_coord)
    .bind(&item.branch)
    .bind(row.last_progress_at)
    .bind(serde_json::to_value(item)?)
    .bind(&row.event_id)
    .bind(row.created_at)
    .bind(row.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

fn work_item_row(r: sqlx::postgres::PgRow) -> Result<WorkItemRow> {
    Ok(WorkItemRow {
        content: content(&r, "content")?,
        event_id: r.try_get("event_id")?,
        last_progress_at: r.try_get("last_progress_at")?,
        done_at: r.try_get("done_at")?,
        created_at: r.try_get("created_at")?,
        updated_at: r.try_get("updated_at")?,
    })
}

macro_rules! work_item_columns {
    () => {
        "content, event_id, last_progress_at, done_at, created_at, updated_at"
    };
}

/// Read one work item by id.
pub async fn get_work_item(
    conn: &mut PgConnection,
    community_id: CommunityId,
    id: Uuid,
) -> Result<Option<WorkItemRow>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        work_item_columns!(),
        " FROM io_work_items WHERE community_id = $1 AND id = $2"
    ))
    .bind(community_id.as_uuid())
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(work_item_row).transpose()
}

/// Direct children of an item, oldest first.
pub async fn list_children(
    conn: &mut PgConnection,
    community_id: CommunityId,
    parent_id: Uuid,
) -> Result<Vec<WorkItemRow>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        work_item_columns!(),
        " FROM io_work_items \
         WHERE community_id = $1 AND parent_id = $2 ORDER BY created_at, id"
    ))
    .bind(community_id.as_uuid())
    .bind(parent_id)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(work_item_row).collect()
}

/// Items in `state` whose `due_at` is at or before `due_before` — the §6.3
/// sweeps (offers past their window, roots entering review or closing).
pub async fn list_work_items_due(
    conn: &mut PgConnection,
    community_id: CommunityId,
    state: WorkItemState,
    due_before: DateTime<Utc>,
) -> Result<Vec<WorkItemRow>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        work_item_columns!(),
        " FROM io_work_items \
         WHERE community_id = $1 AND state = $2 AND due_at <= $3 ORDER BY due_at, id"
    ))
    .bind(community_id.as_uuid())
    .bind(wire(&state)?)
    .bind(due_before)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(work_item_row).collect()
}

// ── io_proposals / io_votes (kind:39102) ──────────────────────────────────────

/// One proposal: the latest `39102` and its event id.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposalRow {
    /// Canonical §4.4 content.
    pub content: Proposal,
    /// Id of the relay-signed `39102` this row mirrors.
    pub event_id: Vec<u8>,
    /// The event's `created_at`.
    pub updated_at: DateTime<Utc>,
}

/// Write a proposal, replacing the previous projection of the same id.
pub async fn upsert_proposal(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &ProposalRow,
) -> Result<()> {
    let p = &row.content;
    sqlx::query(
        "INSERT INTO io_proposals \
            (community_id, id, kind, status, opened_by, opened_at, expires_at, draft_event_id, \
             needed, decided_at, content, event_id, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
         ON CONFLICT (community_id, id) DO UPDATE SET \
            kind = EXCLUDED.kind, status = EXCLUDED.status, opened_by = EXCLUDED.opened_by, \
            opened_at = EXCLUDED.opened_at, expires_at = EXCLUDED.expires_at, \
            draft_event_id = EXCLUDED.draft_event_id, needed = EXCLUDED.needed, \
            decided_at = EXCLUDED.decided_at, content = EXCLUDED.content, \
            event_id = EXCLUDED.event_id, updated_at = EXCLUDED.updated_at",
    )
    .bind(community_id.as_uuid())
    .bind(parse_uuid(&p.id)?)
    .bind(wire(&p.kind)?)
    .bind(wire(&p.status)?)
    .bind(hex32(&p.opened_by)?)
    .bind(ts(p.opened_at)?)
    .bind(ts(p.expires_at)?)
    .bind(opt_hex32(p.draft.as_deref())?)
    .bind(
        i32::try_from(p.needed)
            .map_err(|_| DbError::InvalidData(format!("proposal needed {} overflows", p.needed)))?,
    )
    .bind(opt_ts(p.decided_at)?)
    .bind(serde_json::to_value(p)?)
    .bind(&row.event_id)
    .bind(row.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

fn proposal_row(r: sqlx::postgres::PgRow) -> Result<ProposalRow> {
    Ok(ProposalRow {
        content: content(&r, "content")?,
        event_id: r.try_get("event_id")?,
        updated_at: r.try_get("updated_at")?,
    })
}

/// Read one proposal by id.
pub async fn get_proposal(
    conn: &mut PgConnection,
    community_id: CommunityId,
    id: Uuid,
) -> Result<Option<ProposalRow>> {
    let row = sqlx::query(
        "SELECT content, event_id, updated_at FROM io_proposals \
         WHERE community_id = $1 AND id = $2",
    )
    .bind(community_id.as_uuid())
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(proposal_row).transpose()
}

/// The `["receipt", <opening-command-id>]` of a proposal's live `39102`
/// (Protocol §4.4), read from the stored head, so every rewrite of the
/// proposal keeps citing the command that opened it. `None` when the
/// proposal or its head is missing; an error when the head has no receipt
/// tag, which the executor never writes.
pub async fn get_proposal_opening_receipt(
    conn: &mut PgConnection,
    community_id: CommunityId,
    id: Uuid,
) -> Result<Option<String>> {
    let tags: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT e.tags FROM io_proposals p \
         JOIN events e ON e.community_id = p.community_id AND e.id = p.event_id \
         WHERE p.community_id = $1 AND p.id = $2 AND e.deleted_at IS NULL",
    )
    .bind(community_id.as_uuid())
    .bind(id)
    .fetch_optional(conn)
    .await?;
    let Some(tags) = tags else {
        return Ok(None);
    };
    let receipt = tags
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t.as_array())
        .find(|t| t.first().and_then(|v| v.as_str()) == Some("receipt"))
        .and_then(|t| t.get(1).and_then(|v| v.as_str()))
        .ok_or_else(|| {
            DbError::InvalidData(format!("39102 head of proposal {id} has no receipt tag"))
        })?;
    Ok(Some(receipt.to_owned()))
}

/// Proposals in `status` whose `expires_at` is at or before `before` — the
/// §6.3 rule 5 sweep when `status` is [`ProposalStatus::Open`].
pub async fn list_proposals_expiring(
    conn: &mut PgConnection,
    community_id: CommunityId,
    status: ProposalStatus,
    before: DateTime<Utc>,
) -> Result<Vec<ProposalRow>> {
    let rows = sqlx::query(
        "SELECT content, event_id, updated_at FROM io_proposals \
         WHERE community_id = $1 AND status = $2 AND expires_at <= $3 ORDER BY expires_at, id",
    )
    .bind(community_id.as_uuid())
    .bind(wire(&status)?)
    .bind(before)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(proposal_row).collect()
}

/// One voter's current vote on a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteRow {
    /// The proposal voted on.
    pub proposal_id: Uuid,
    /// The voter (32 bytes).
    pub voter: Vec<u8>,
    /// Agree or reject.
    pub vote: VoteChoice,
    /// Optional `reason` from the `50003` content.
    pub reason: Option<String>,
    /// The `50003` that cast it.
    pub receipt_event_id: Vec<u8>,
    /// When it was cast.
    pub cast_at: DateTime<Utc>,
}

/// Record a vote; a voter's second vote on the same proposal overwrites the
/// first (`vote_changed`). The `io_proposals` row must already exist.
pub async fn upsert_vote(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &VoteRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_votes \
            (community_id, proposal_id, voter, vote, reason, receipt_event_id, cast_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (community_id, proposal_id, voter) DO UPDATE SET \
            vote = EXCLUDED.vote, reason = EXCLUDED.reason, \
            receipt_event_id = EXCLUDED.receipt_event_id, cast_at = EXCLUDED.cast_at",
    )
    .bind(community_id.as_uuid())
    .bind(row.proposal_id)
    .bind(&row.voter)
    .bind(wire(&row.vote)?)
    .bind(&row.reason)
    .bind(&row.receipt_event_id)
    .bind(row.cast_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// All current votes on a proposal, oldest first.
pub async fn list_votes(
    conn: &mut PgConnection,
    community_id: CommunityId,
    proposal_id: Uuid,
) -> Result<Vec<VoteRow>> {
    let rows = sqlx::query(
        "SELECT voter, vote, reason, receipt_event_id, cast_at FROM io_votes \
         WHERE community_id = $1 AND proposal_id = $2 ORDER BY cast_at, voter",
    )
    .bind(community_id.as_uuid())
    .bind(proposal_id)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(VoteRow {
                proposal_id,
                voter: r.try_get("voter")?,
                vote: from_wire(r.try_get::<String, _>("vote")?.as_str())?,
                reason: r.try_get("reason")?,
                receipt_event_id: r.try_get("receipt_event_id")?,
                cast_at: r.try_get("cast_at")?,
            })
        })
        .collect()
}

// ── io_drafts (kind:50100 + kind:39104) ───────────────────────────────────────

/// A stored draft with its outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftRow {
    /// The `50100` event id.
    pub event_id: Vec<u8>,
    /// The agent that signed it (32 bytes).
    pub author: Vec<u8>,
    /// The `t` tag.
    pub draft_kind: DraftKind,
    /// The `n` tag: a pubkey hex or the literal `shaper`.
    pub needs: String,
    /// The `gap` tag — the dedupe key.
    pub gap: String,
    /// The `move` tag, when present.
    pub r#move: Option<i16>,
    /// The `origin` tag, when present.
    pub origin: Option<DraftOrigin>,
    /// The `i` tag (dri / review / money drafts).
    pub item_id: Option<Uuid>,
    /// The `u` tag (ticket / done drafts).
    pub parent_id: Option<Uuid>,
    /// `["shadow","true"]` present.
    pub shadow: bool,
    /// The `expiration` tag.
    pub expires_at: Option<DateTime<Utc>>,
    /// Canonical §4.3 payload, kept as JSON: the row is indexed by its tags,
    /// and callers parse through [`buzz_core::intelligent_org::DraftPayload::parse`].
    pub payload: serde_json::Value,
    /// Outcome (§4.6); `status = open` until decided.
    pub outcome: DraftOutcome,
    /// Id of the latest `39104` for this draft.
    pub outcome_event_id: Option<Vec<u8>>,
    /// The `50100`'s `created_at`.
    pub created_at: DateTime<Utc>,
}

/// Store a draft. Fails with a unique violation when another draft for the
/// same `gap` is still `open` (§6.1 "one open draft per gap").
pub async fn insert_draft(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &DraftRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_drafts \
            (community_id, event_id, author, draft_kind, needs, gap, move, origin, item_id, \
             parent_id, shadow, expires_at, payload, status, decided_by, decided_at, \
             decline_reason, result_event_id, outcome_event_id, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
                 $17, $18, $19, $20)",
    )
    .bind(community_id.as_uuid())
    .bind(&row.event_id)
    .bind(&row.author)
    .bind(wire(&row.draft_kind)?)
    .bind(&row.needs)
    .bind(&row.gap)
    .bind(row.r#move)
    .bind(row.origin.as_ref().map(wire).transpose()?)
    .bind(row.item_id)
    .bind(row.parent_id)
    .bind(row.shadow)
    .bind(row.expires_at)
    .bind(&row.payload)
    .bind(wire(&row.outcome.status)?)
    .bind(opt_hex32(row.outcome.decided_by.as_deref())?)
    .bind(opt_ts(row.outcome.decided_at)?)
    .bind(row.outcome.reason.as_ref().map(wire).transpose()?)
    .bind(opt_hex32(row.outcome.result.as_deref())?)
    .bind(&row.outcome_event_id)
    .bind(row.created_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Record a draft's outcome (the `39104` the relay just emitted).
///
/// Returns `false` when no such draft exists in this community.
pub async fn set_draft_outcome(
    conn: &mut PgConnection,
    community_id: CommunityId,
    outcome: &DraftOutcome,
    outcome_event_id: &[u8],
) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE io_drafts SET status = $3, decided_by = $4, decided_at = $5, \
            decline_reason = $6, result_event_id = $7, outcome_event_id = $8 \
         WHERE community_id = $1 AND event_id = $2",
    )
    .bind(community_id.as_uuid())
    .bind(hex32(&outcome.draft)?)
    .bind(wire(&outcome.status)?)
    .bind(opt_hex32(outcome.decided_by.as_deref())?)
    .bind(opt_ts(outcome.decided_at)?)
    .bind(outcome.reason.as_ref().map(wire).transpose()?)
    .bind(opt_hex32(outcome.result.as_deref())?)
    .bind(outcome_event_id)
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

macro_rules! draft_columns {
    () => {
        "event_id, author, draft_kind, needs, gap, move, origin, item_id, parent_id, shadow, \
         expires_at, payload, status, decided_by, decided_at, decline_reason, result_event_id, \
         outcome_event_id, created_at"
    };
}

fn draft_row(r: sqlx::postgres::PgRow) -> Result<DraftRow> {
    let event_id: Vec<u8> = r.try_get("event_id")?;
    let decided_by: Option<Vec<u8>> = r.try_get("decided_by")?;
    let decided_at: Option<DateTime<Utc>> = r.try_get("decided_at")?;
    let result: Option<Vec<u8>> = r.try_get("result_event_id")?;
    let reason: Option<String> = r.try_get("decline_reason")?;
    let origin: Option<String> = r.try_get("origin")?;
    Ok(DraftRow {
        author: r.try_get("author")?,
        draft_kind: from_wire(r.try_get::<String, _>("draft_kind")?.as_str())?,
        needs: r.try_get("needs")?,
        gap: r.try_get("gap")?,
        r#move: r.try_get("move")?,
        origin: origin.as_deref().map(from_wire).transpose()?,
        item_id: r.try_get("item_id")?,
        parent_id: r.try_get("parent_id")?,
        shadow: r.try_get("shadow")?,
        expires_at: r.try_get("expires_at")?,
        payload: r.try_get("payload")?,
        outcome: DraftOutcome {
            draft: hex::encode(&event_id),
            status: from_wire(r.try_get::<String, _>("status")?.as_str())?,
            decided_by: decided_by.map(hex::encode),
            decided_at: decided_at.map(|t| t.timestamp().unsigned_abs()),
            reason: reason.as_deref().map(from_wire).transpose()?,
            result: result.map(hex::encode),
        },
        outcome_event_id: r.try_get("outcome_event_id")?,
        created_at: r.try_get("created_at")?,
        event_id,
    })
}

/// Read one draft by its `50100` event id.
pub async fn get_draft(
    conn: &mut PgConnection,
    community_id: CommunityId,
    event_id: &[u8],
) -> Result<Option<DraftRow>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        draft_columns!(),
        " FROM io_drafts WHERE community_id = $1 AND event_id = $2"
    ))
    .bind(community_id.as_uuid())
    .bind(event_id)
    .fetch_optional(conn)
    .await?;
    row.map(draft_row).transpose()
}

/// The open draft for a `gap`, if one exists — the §6.1 dedupe check.
pub async fn find_open_draft_for_gap(
    conn: &mut PgConnection,
    community_id: CommunityId,
    gap: &str,
) -> Result<Option<DraftRow>> {
    let row = sqlx::query(concat!(
        "SELECT ",
        draft_columns!(),
        " FROM io_drafts \
         WHERE community_id = $1 AND gap = $2 AND status = 'open'"
    ))
    .bind(community_id.as_uuid())
    .bind(gap)
    .fetch_optional(conn)
    .await?;
    row.map(draft_row).transpose()
}

/// Drafts in `status` whose `expiration` is at or before `before` — the §6.3
/// rule 4 sweep when `status` is [`DraftOutcomeStatus::Open`].
pub async fn list_drafts_expiring(
    conn: &mut PgConnection,
    community_id: CommunityId,
    status: DraftOutcomeStatus,
    before: DateTime<Utc>,
) -> Result<Vec<DraftRow>> {
    let rows = sqlx::query(concat!(
        "SELECT ",
        draft_columns!(),
        " FROM io_drafts \
         WHERE community_id = $1 AND status = $2 AND expires_at IS NOT NULL AND expires_at <= $3 \
         ORDER BY expires_at, event_id"
    ))
    .bind(community_id.as_uuid())
    .bind(wire(&status)?)
    .bind(before)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(draft_row).collect()
}

// ── io_progress (kind:50102) ──────────────────────────────────────────────────

/// One progress note's indexed fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressRow {
    /// The `50102` event id.
    pub event_id: Vec<u8>,
    /// The item it reports on.
    pub item_id: Uuid,
    /// Who signed it: the holder or their attested agent (32 bytes).
    pub signer: Vec<u8>,
    /// The holder it reports for (32 bytes).
    pub dri: Vec<u8>,
    /// Branch or ref the work is on.
    pub git_ref: String,
    /// Commit sha at `to`.
    pub head: String,
    /// The holder's reading.
    pub hint: ProgressHint,
    /// Whether the relay found `head` in the home repository.
    pub head_verified: Option<bool>,
    /// Filled by the push hook when `head` reaches the default branch (D12).
    pub merged_into: Option<String>,
    /// The event's `created_at`.
    pub noted_at: DateTime<Utc>,
}

impl ProgressRow {
    /// Index a `50102` from its content, event id, signer, and `created_at`.
    pub fn from_note(
        note: &ProgressNote,
        event_id: Vec<u8>,
        signer: Vec<u8>,
        noted_at: DateTime<Utc>,
    ) -> Result<Self> {
        Ok(Self {
            event_id,
            item_id: parse_uuid(&note.item)?,
            signer,
            dri: hex32(&note.dri)?,
            git_ref: note.git_ref.clone(),
            head: note.head.clone(),
            hint: note.hint,
            head_verified: note.head_verified,
            merged_into: note.merged_into.clone(),
            noted_at,
        })
    }
}

/// Index a progress note.
pub async fn insert_progress(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &ProgressRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_progress \
            (community_id, event_id, item_id, signer, dri, git_ref, head, hint, head_verified, \
             merged_into, noted_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(community_id.as_uuid())
    .bind(&row.event_id)
    .bind(row.item_id)
    .bind(&row.signer)
    .bind(&row.dri)
    .bind(&row.git_ref)
    .bind(&row.head)
    .bind(wire(&row.hint)?)
    .bind(row.head_verified)
    .bind(&row.merged_into)
    .bind(row.noted_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Progress notes for an item, newest first, at most `limit`.
pub async fn list_progress_for_item(
    conn: &mut PgConnection,
    community_id: CommunityId,
    item_id: Uuid,
    limit: i64,
) -> Result<Vec<ProgressRow>> {
    let rows = sqlx::query(
        "SELECT event_id, signer, dri, git_ref, head, hint, head_verified, merged_into, noted_at \
         FROM io_progress WHERE community_id = $1 AND item_id = $2 \
         ORDER BY noted_at DESC, event_id LIMIT $3",
    )
    .bind(community_id.as_uuid())
    .bind(item_id)
    .bind(limit)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(ProgressRow {
                event_id: r.try_get("event_id")?,
                item_id,
                signer: r.try_get("signer")?,
                dri: r.try_get("dri")?,
                git_ref: r.try_get("git_ref")?,
                head: r.try_get("head")?,
                hint: from_wire(r.try_get::<String, _>("hint")?.as_str())?,
                head_verified: r.try_get("head_verified")?,
                merged_into: r.try_get("merged_into")?,
                noted_at: r.try_get("noted_at")?,
            })
        })
        .collect()
}

/// The push hook's `merged_into` fill (D12) for every note whose `head`
/// reached `git_ref` on the default branch. Returns the rows updated.
pub async fn set_progress_merged_into(
    conn: &mut PgConnection,
    community_id: CommunityId,
    item_id: Uuid,
    head: &str,
    merged_into: &str,
) -> Result<u64> {
    let result = sqlx::query(
        "UPDATE io_progress SET merged_into = $4 \
         WHERE community_id = $1 AND item_id = $2 AND head = $3 AND merged_into IS NULL",
    )
    .bind(community_id.as_uuid())
    .bind(item_id)
    .bind(head)
    .bind(merged_into)
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

// ── io_health / io_health_ratings (kind:50101, kind:50017) ────────────────────

/// One health read.
#[derive(Debug, Clone, PartialEq)]
pub struct HealthRow {
    /// The `50101` event id.
    pub event_id: Vec<u8>,
    /// Canonical §4.7 content.
    pub content: HealthRead,
    /// The event's `created_at`.
    pub read_at: DateTime<Utc>,
}

/// Index a health read.
pub async fn insert_health(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &HealthRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_health \
            (community_id, event_id, item_id, week, pct, band, content, read_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(community_id.as_uuid())
    .bind(&row.event_id)
    .bind(parse_uuid(&row.content.item)?)
    .bind(&row.content.week)
    .bind(row.content.pct)
    .bind(wire(&row.content.band)?)
    .bind(serde_json::to_value(&row.content)?)
    .bind(row.read_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The newest health read for an item.
pub async fn latest_health_for_item(
    conn: &mut PgConnection,
    community_id: CommunityId,
    item_id: Uuid,
) -> Result<Option<HealthRow>> {
    let row = sqlx::query(
        "SELECT event_id, content, read_at FROM io_health \
         WHERE community_id = $1 AND item_id = $2 ORDER BY read_at DESC, event_id LIMIT 1",
    )
    .bind(community_id.as_uuid())
    .bind(item_id)
    .fetch_optional(conn)
    .await?;
    row.map(|r| {
        Ok(HealthRow {
            event_id: r.try_get("event_id")?,
            content: content(&r, "content")?,
            read_at: r.try_get("read_at")?,
        })
    })
    .transpose()
}

/// A Shaper's blind band for an item and week (`50017`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthRatingRow {
    /// The item rated.
    pub item_id: Uuid,
    /// ISO week, as on the `50017`'s `week` tag.
    pub week: String,
    /// The Shaper (32 bytes).
    pub rater: Vec<u8>,
    /// Their band.
    pub band: HealthBand,
    /// The `50017` event id.
    pub receipt_event_id: Vec<u8>,
    /// The event's `created_at`.
    pub rated_at: DateTime<Utc>,
}

/// Record a rating; a Shaper's second rating for the same item and week
/// overwrites the first.
pub async fn upsert_health_rating(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &HealthRatingRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_health_ratings \
            (community_id, item_id, week, rater, band, receipt_event_id, rated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (community_id, item_id, week, rater) DO UPDATE SET \
            band = EXCLUDED.band, receipt_event_id = EXCLUDED.receipt_event_id, \
            rated_at = EXCLUDED.rated_at",
    )
    .bind(community_id.as_uuid())
    .bind(row.item_id)
    .bind(&row.week)
    .bind(&row.rater)
    .bind(wire(&row.band)?)
    .bind(&row.receipt_event_id)
    .bind(row.rated_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Every rating for an item and week.
pub async fn list_health_ratings(
    conn: &mut PgConnection,
    community_id: CommunityId,
    item_id: Uuid,
    week: &str,
) -> Result<Vec<HealthRatingRow>> {
    let rows = sqlx::query(
        "SELECT rater, band, receipt_event_id, rated_at FROM io_health_ratings \
         WHERE community_id = $1 AND item_id = $2 AND week = $3 ORDER BY rated_at, rater",
    )
    .bind(community_id.as_uuid())
    .bind(item_id)
    .bind(week)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(HealthRatingRow {
                item_id,
                week: week.to_owned(),
                rater: r.try_get("rater")?,
                band: from_wire(r.try_get::<String, _>("band")?.as_str())?,
                receipt_event_id: r.try_get("receipt_event_id")?,
                rated_at: r.try_get("rated_at")?,
            })
        })
        .collect()
}

// ── io_profiles (kind:39105) ──────────────────────────────────────────────────

/// A member's org profile: the latest `39105` and its event id.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileRow {
    /// Canonical §4.7a content.
    pub content: buzz_core::intelligent_org::OrgProfile,
    /// False once the member left the community (NIP-43 removal).
    pub active: bool,
    /// Id of the relay-signed `39105` this row mirrors.
    pub event_id: Vec<u8>,
    /// The event's `created_at`.
    pub updated_at: DateTime<Utc>,
}

/// Write a profile, replacing the previous projection for the same pubkey.
/// `skills` is denormalised from `content.skills[].slug` for the GIN index.
pub async fn upsert_profile(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &ProfileRow,
) -> Result<()> {
    let p = &row.content;
    let skills: Vec<&str> = p.skills.iter().map(|s| s.slug.as_str()).collect();
    sqlx::query(
        "INSERT INTO io_profiles \
            (community_id, pubkey, version, about, skills, open_limit, active, content, \
             event_id, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (community_id, pubkey) DO UPDATE SET \
            version = EXCLUDED.version, about = EXCLUDED.about, skills = EXCLUDED.skills, \
            open_limit = EXCLUDED.open_limit, active = EXCLUDED.active, \
            content = EXCLUDED.content, event_id = EXCLUDED.event_id, \
            updated_at = EXCLUDED.updated_at",
    )
    .bind(community_id.as_uuid())
    .bind(hex32(&p.pubkey)?)
    .bind(
        i32::try_from(p.version).map_err(|_| {
            DbError::InvalidData(format!("profile version {} overflows", p.version))
        })?,
    )
    .bind(&p.about)
    .bind(&skills)
    .bind(
        p.open_limit
            .map(i32::try_from)
            .transpose()
            .map_err(|_| DbError::InvalidData("profile open_limit overflows".into()))?,
    )
    .bind(row.active)
    .bind(serde_json::to_value(p)?)
    .bind(&row.event_id)
    .bind(row.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

fn profile_row(r: sqlx::postgres::PgRow) -> Result<ProfileRow> {
    Ok(ProfileRow {
        content: content(&r, "content")?,
        active: r.try_get("active")?,
        event_id: r.try_get("event_id")?,
        updated_at: r.try_get("updated_at")?,
    })
}

/// Read one member's profile.
pub async fn get_profile(
    conn: &mut PgConnection,
    community_id: CommunityId,
    pubkey: &[u8],
) -> Result<Option<ProfileRow>> {
    let row = sqlx::query(
        "SELECT content, active, event_id, updated_at FROM io_profiles \
         WHERE community_id = $1 AND pubkey = $2",
    )
    .bind(community_id.as_uuid())
    .bind(pubkey)
    .fetch_optional(conn)
    .await?;
    row.map(profile_row).transpose()
}

/// Active profiles carrying at least one of `skills` — the agent's candidate
/// query, served by the GIN index on `skills`.
pub async fn find_profiles_with_any_skill(
    conn: &mut PgConnection,
    community_id: CommunityId,
    skills: &[String],
) -> Result<Vec<ProfileRow>> {
    if skills.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        "SELECT content, active, event_id, updated_at FROM io_profiles \
         WHERE community_id = $1 AND active AND skills && $2 ORDER BY pubkey",
    )
    .bind(community_id.as_uuid())
    .bind(skills)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(profile_row).collect()
}

/// Flip a profile's `active` flag (false on NIP-43 removal, true on rejoin).
/// Returns `false` when the member has no profile row.
pub async fn set_profile_active(
    conn: &mut PgConnection,
    community_id: CommunityId,
    pubkey: &[u8],
    active: bool,
) -> Result<bool> {
    let result =
        sqlx::query("UPDATE io_profiles SET active = $3 WHERE community_id = $1 AND pubkey = $2")
            .bind(community_id.as_uuid())
            .bind(pubkey)
            .bind(active)
            .execute(conn)
            .await?;
    Ok(result.rows_affected() == 1)
}

// ── io_ledger ─────────────────────────────────────────────────────────────────

/// Actor of a rule-driven ledger row (§6.2).
pub const LEDGER_ACTOR_RELAY: &str = "relay";

/// One ledger entry to append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// When it happened — the receipt event's `created_at`, or the sweep time.
    pub at: DateTime<Utc>,
    /// A pubkey hex, or [`LEDGER_ACTOR_RELAY`].
    pub actor: String,
    /// One of the §6.2 verbs (`item_accepted`, `proposal_passed`, …).
    pub verb: String,
    /// `item`, `proposal`, `draft`, `shapers`, `direction`, `profile`, `member`, `agent`.
    pub object_type: String,
    /// The object's id (uuid, slug, pubkey, or event id hex).
    pub object_id: String,
    /// The event that caused it, when there is one.
    pub receipt_event_id: Option<Vec<u8>>,
    /// Verb-specific detail (`from`/`to`, `for`, `why`, …).
    pub detail: serde_json::Value,
}

/// A stored ledger row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRow {
    /// Monotonic per-community sequence.
    pub id: i64,
    /// The entry.
    pub entry: LedgerEntry,
}

/// Append one ledger row; returns its sequence id.
pub async fn insert_ledger(
    conn: &mut PgConnection,
    community_id: CommunityId,
    entry: &LedgerEntry,
) -> Result<i64> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO io_ledger \
            (community_id, at, actor, verb, object_type, object_id, receipt_event_id, detail) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(community_id.as_uuid())
    .bind(entry.at)
    .bind(&entry.actor)
    .bind(&entry.verb)
    .bind(&entry.object_type)
    .bind(&entry.object_id)
    .bind(&entry.receipt_event_id)
    .bind(&entry.detail)
    .fetch_one(conn)
    .await?;
    Ok(id)
}

/// Ledger rows for one object, newest first, at most `limit`.
pub async fn list_ledger_for_object(
    conn: &mut PgConnection,
    community_id: CommunityId,
    object_type: &str,
    object_id: &str,
    limit: i64,
) -> Result<Vec<LedgerRow>> {
    let rows = sqlx::query(
        "SELECT id, at, actor, verb, receipt_event_id, detail FROM io_ledger \
         WHERE community_id = $1 AND object_type = $2 AND object_id = $3 \
         ORDER BY at DESC, id DESC LIMIT $4",
    )
    .bind(community_id.as_uuid())
    .bind(object_type)
    .bind(object_id)
    .bind(limit)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(LedgerRow {
                id: r.try_get("id")?,
                entry: LedgerEntry {
                    at: r.try_get("at")?,
                    actor: r.try_get("actor")?,
                    verb: r.try_get("verb")?,
                    object_type: object_type.to_owned(),
                    object_id: object_id.to_owned(),
                    receipt_event_id: r.try_get("receipt_event_id")?,
                    detail: r.try_get("detail")?,
                },
            })
        })
        .collect()
}

// ── io_hosted_agents ──────────────────────────────────────────────────────────

/// A hosted org-agent key provisioned by the relay operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedAgentRow {
    /// The agent's pubkey (32 bytes).
    pub pubkey: Vec<u8>,
    /// When the operator provisioned it.
    pub provisioned_at: DateTime<Utc>,
    /// Operator-defined budget, when set.
    pub budget: Option<serde_json::Value>,
    /// Set when a later key replaced it.
    pub retired_at: Option<DateTime<Utc>>,
}

/// Register a hosted key. Fails with a unique violation while another key is
/// live (`retired_at IS NULL`); retire it first.
pub async fn insert_hosted_agent(
    conn: &mut PgConnection,
    community_id: CommunityId,
    row: &HostedAgentRow,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO io_hosted_agents (community_id, pubkey, provisioned_at, budget, retired_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(community_id.as_uuid())
    .bind(&row.pubkey)
    .bind(row.provisioned_at)
    .bind(&row.budget)
    .bind(row.retired_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The live hosted key for this community, read at `39103` bootstrap and when
/// a `shapers/agent` proposal returns the community to the hosted default.
pub async fn get_live_hosted_agent(
    conn: &mut PgConnection,
    community_id: CommunityId,
) -> Result<Option<HostedAgentRow>> {
    let row = sqlx::query(
        "SELECT pubkey, provisioned_at, budget, retired_at FROM io_hosted_agents \
         WHERE community_id = $1 AND retired_at IS NULL",
    )
    .bind(community_id.as_uuid())
    .fetch_optional(conn)
    .await?;
    row.map(|r| {
        Ok(HostedAgentRow {
            pubkey: r.try_get("pubkey")?,
            provisioned_at: r.try_get("provisioned_at")?,
            budget: r.try_get("budget")?,
            retired_at: r.try_get("retired_at")?,
        })
    })
    .transpose()
}

/// Retire the live hosted key at `retired_at`. Returns `false` when none is live.
pub async fn retire_hosted_agent(
    conn: &mut PgConnection,
    community_id: CommunityId,
    retired_at: DateTime<Utc>,
) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE io_hosted_agents SET retired_at = $2 \
         WHERE community_id = $1 AND retired_at IS NULL",
    )
    .bind(community_id.as_uuid())
    .bind(retired_at)
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

// ── Pool-scoped reads for handlers outside the executor ───────────────────────

impl Db {
    /// Whether `pubkey_hex` is in the community's live `39103.shapers`, read
    /// from `io_shapers` at request time (Protocol §6.6 — invite mint
    /// authorisation). `false` before bootstrap or for a pubkey that is not a
    /// Shaper; a malformed pubkey is simply not one.
    pub async fn is_org_shaper(&self, community: CommunityId, pubkey_hex: &str) -> Result<bool> {
        let mut conn = observability::acquire_writer(
            &self.pool,
            observability::WriterOperation::Authorization,
        )
        .await?;
        Ok(get_shapers(&mut conn, community)
            .await?
            .is_some_and(|row| row.content.shapers.iter().any(|p| p == pubkey_hex)))
    }

    /// Whether the community is an intelligent organization for the purposes
    /// of the invite landing page's transparency notice (Readiness D7,
    /// Protocol §6.6): it has an `io_hosted_agents` row (the operator
    /// provisioned an org agent for it) or an `io_shapers` row (a `39103`
    /// has been bootstrapped).
    pub async fn is_org_community(&self, community: CommunityId) -> Result<bool> {
        let mut conn = observability::acquire_writer(
            &self.pool,
            observability::WriterOperation::Authorization,
        )
        .await?;
        let is_org: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM io_hosted_agents WHERE community_id = $1) \
                 OR EXISTS (SELECT 1 FROM io_shapers WHERE community_id = $1)",
        )
        .bind(community.as_uuid())
        .fetch_one(&mut *conn)
        .await?;
        Ok(is_org)
    }
}

#[cfg(test)]
mod postgres_tests {
    use super::*;
    use buzz_core::intelligent_org::{
        ChildrenCounts, DecisionRule, DecisionRules, DeclineReason, DirectionLine,
        GroundedSentence, HealthFactor, NamedRule, OrgProfile, ProjectHome, ProposalKind, Skill,
        Vote, DEFAULT_DECISION_WINDOW_SECS, DEFAULT_OFFER_WINDOW_SECS,
    };
    use sqlx::PgPool;

    async fn context() -> (PgPool, CommunityId) {
        let pool = PgPool::connect(&crate::test_support::database_url())
            .await
            .expect("connect intelligent-org test database");
        let db = crate::Db::from_pool(pool.clone());
        if std::env::var("BUZZ_TEST_SCHEMA_MODE").as_deref() != Ok("desired") {
            db.migrate().await.expect("migrate test database");
        }
        let host = format!("io-store-{}.example", Uuid::new_v4().simple());
        let community = db
            .ensure_configured_community(&host)
            .await
            .expect("create test community")
            .id;
        (pool, community)
    }

    fn hex_id(seed: u8) -> String {
        hex::encode([seed; 32])
    }

    fn at(seconds: u64) -> DateTime<Utc> {
        ts(seconds).expect("valid timestamp")
    }

    fn shapers(agent: Option<&str>) -> Shapers {
        Shapers {
            founder: hex_id(1),
            shapers: vec![hex_id(1), hex_id(2)],
            offered: vec![],
            room: Some(Uuid::nil().to_string()),
            agent: agent.map(str::to_owned),
            agent_hosted: agent.is_none(),
            rules: DecisionRules::default(),
            decision_window_secs: DEFAULT_DECISION_WINDOW_SECS,
            offer_window_secs: DEFAULT_OFFER_WINDOW_SECS,
            updated_at: 1_700_000_000,
            receipt: hex_id(9),
        }
    }

    fn work_item(id: Uuid, parent: Option<Uuid>, state: WorkItemState) -> WorkItem {
        let root = parent.unwrap_or(id);
        WorkItem {
            id: id.to_string(),
            parent: parent.map(|p| p.to_string()),
            root: root.to_string(),
            depth: u32::from(parent.is_some()),
            path: parent.map(|p| vec![p.to_string()]).unwrap_or_default(),
            title: "Ship the thing".into(),
            brief: "What done looks like.".into(),
            state,
            dri: matches!(state, WorkItemState::Accepted | WorkItemState::InReview)
                .then(|| hex_id(3)),
            offered_to: matches!(state, WorkItemState::Offered).then(|| hex_id(4)),
            offered_by: None,
            offered_at: matches!(state, WorkItemState::Offered).then_some(1_700_000_100),
            due_at: 1_700_600_000,
            approved_at: parent.is_none().then_some(1_700_000_000),
            objective_ref: Some("objectives@1#l_1".into()),
            created_from: hex_id(5),
            draft: None,
            done_receipt: None,
            closed_by: None,
            children: ChildrenCounts::default(),
            home: parent.is_none().then(|| ProjectHome {
                channel: Uuid::nil().to_string(),
                repo: Some("30617:relay:slug".into()),
                project: None,
            }),
            branch: parent.map(|_| "io/abcd-thing".into()),
            after: vec![],
            last_progress: None,
        }
    }

    fn proposal(id: Uuid, status: ProposalStatus, kind: ProposalKind) -> Proposal {
        Proposal {
            id: id.to_string(),
            kind,
            status,
            opened_by: hex_id(1),
            opened_at: 1_700_000_000,
            expires_at: 1_700_000_000 + DEFAULT_DECISION_WINDOW_SECS,
            draft: None,
            payload: serde_json::json!({ "title": "A project" }),
            rule: DecisionRule::Named(NamedRule::Majority),
            needed: 2,
            eligible: vec![hex_id(1), hex_id(2)],
            votes: vec![Vote {
                p: hex_id(1),
                vote: VoteChoice::Agree,
                at: 1_700_000_001,
                receipt: hex_id(7),
            }],
            decided_at: None,
            executed: None,
            settlement: None,
        }
    }

    fn draft(event: u8, gap: &str, needs: &str) -> DraftRow {
        DraftRow {
            event_id: vec![event; 32],
            author: vec![0xAA; 32],
            draft_kind: DraftKind::Ticket,
            needs: needs.to_owned(),
            gap: gap.to_owned(),
            r#move: Some(2),
            origin: Some(DraftOrigin::Gap),
            item_id: None,
            parent_id: Some(Uuid::nil()),
            shadow: false,
            expires_at: Some(at(1_700_100_000)),
            payload: serde_json::json!({ "title": "Do the piece" }),
            outcome: DraftOutcome {
                draft: hex::encode([event; 32]),
                status: DraftOutcomeStatus::Open,
                decided_by: None,
                decided_at: None,
                reason: None,
                result: None,
            },
            outcome_event_id: None,
            created_at: at(1_700_000_000),
        }
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn shapers_round_trip_and_replace() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let first = ShapersRow {
            content: shapers(None),
            room_channel_id: Some(Uuid::nil()),
            event_id: vec![1; 32],
            updated_at: at(1_700_000_000),
        };
        upsert_shapers(&mut conn, community, &first)
            .await
            .expect("insert shapers");
        assert_eq!(
            get_shapers(&mut conn, community).await.unwrap(),
            Some(first)
        );

        let replaced = ShapersRow {
            content: shapers(Some(&hex_id(8))),
            room_channel_id: Some(Uuid::nil()),
            event_id: vec![2; 32],
            updated_at: at(1_700_000_500),
        };
        upsert_shapers(&mut conn, community, &replaced)
            .await
            .expect("replace shapers");
        let read = get_shapers(&mut conn, community).await.unwrap().unwrap();
        assert_eq!(read, replaced);
        let agent: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT agent FROM io_shapers WHERE community_id = $1")
                .bind(community.as_uuid())
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(agent, Some(vec![8; 32]), "typed column follows the content");
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn direction_versions_are_immutable_and_head_is_newest() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let version = |n: u32| DirectionRow {
            content: DirectionArtifact {
                slug: DirectionSlug::Objectives,
                version: n,
                body: format!("v{n}"),
                lines: vec![DirectionLine {
                    n: 1,
                    id: "l_1".into(),
                    text: "Ship".into(),
                    date: None,
                }],
                confirmed_by: hex_id(1),
                confirmed_at: 1_700_000_000 + u64::from(n),
                proposed_by: hex_id(2),
                proposal: Uuid::nil().to_string(),
                prev: None,
            },
            event_id: vec![n as u8; 32],
        };
        insert_direction(&mut conn, community, &version(1))
            .await
            .unwrap();
        insert_direction(&mut conn, community, &version(2))
            .await
            .unwrap();
        assert!(
            insert_direction(&mut conn, community, &version(2))
                .await
                .is_err(),
            "a version is written once"
        );
        let head = get_direction_head(&mut conn, community, DirectionSlug::Objectives)
            .await
            .unwrap()
            .expect("head");
        assert_eq!(head, version(2));
        assert_eq!(
            get_direction_head(&mut conn, community, DirectionSlug::Mission)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn work_items_round_trip_every_state_and_sweep_by_due() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let root = Uuid::new_v4();
        let root_row = WorkItemRow {
            content: work_item(root, None, WorkItemState::Accepted),
            event_id: vec![1; 32],
            last_progress_at: Some(at(1_700_000_200)),
            done_at: None,
            created_at: at(1_700_000_000),
            updated_at: at(1_700_000_200),
        };
        upsert_work_item(&mut conn, community, &root_row)
            .await
            .unwrap();
        assert_eq!(
            get_work_item(&mut conn, community, root).await.unwrap(),
            Some(root_row.clone())
        );

        // Every state passes the CHECK vocabulary.
        let mut children = Vec::new();
        for (i, state) in [
            WorkItemState::Open,
            WorkItemState::Offered,
            WorkItemState::Accepted,
            WorkItemState::InReview,
            WorkItemState::Done,
        ]
        .into_iter()
        .enumerate()
        {
            let id = Uuid::new_v4();
            let row = WorkItemRow {
                content: work_item(id, Some(root), state),
                event_id: vec![10 + i as u8; 32],
                last_progress_at: None,
                done_at: matches!(state, WorkItemState::Done).then(|| at(1_700_000_900)),
                created_at: at(1_700_000_000 + i as u64),
                updated_at: at(1_700_000_000 + i as u64),
            };
            upsert_work_item(&mut conn, community, &row).await.unwrap();
            children.push(row);
        }
        assert_eq!(
            list_children(&mut conn, community, root).await.unwrap(),
            children
        );

        let due = list_work_items_due(
            &mut conn,
            community,
            WorkItemState::Accepted,
            at(1_700_600_000),
        )
        .await
        .unwrap();
        assert_eq!(due.len(), 2, "the root and its accepted child are due");
        assert!(list_work_items_due(
            &mut conn,
            community,
            WorkItemState::Accepted,
            at(1_700_599_999)
        )
        .await
        .unwrap()
        .is_empty());

        // Replace keeps one row per id.
        let mut moved = root_row.clone();
        moved.content.state = WorkItemState::InReview;
        moved.event_id = vec![2; 32];
        upsert_work_item(&mut conn, community, &moved)
            .await
            .unwrap();
        let read = get_work_item(&mut conn, community, root)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read.content.state, WorkItemState::InReview);
        assert_eq!(read.event_id, vec![2; 32]);
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn proposals_and_votes_round_trip() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let id = Uuid::new_v4();
        let row = ProposalRow {
            content: proposal(id, ProposalStatus::Open, ProposalKind::Project),
            event_id: vec![1; 32],
            updated_at: at(1_700_000_001),
        };
        upsert_proposal(&mut conn, community, &row).await.unwrap();
        assert_eq!(
            get_proposal(&mut conn, community, id).await.unwrap(),
            Some(row.clone())
        );

        let vote = VoteRow {
            proposal_id: id,
            voter: vec![1; 32],
            vote: VoteChoice::Agree,
            reason: None,
            receipt_event_id: vec![7; 32],
            cast_at: at(1_700_000_001),
        };
        upsert_vote(&mut conn, community, &vote).await.unwrap();
        let changed = VoteRow {
            vote: VoteChoice::Decline,
            reason: Some("on reflection".into()),
            receipt_event_id: vec![8; 32],
            cast_at: at(1_700_000_050),
            ..vote.clone()
        };
        upsert_vote(&mut conn, community, &changed).await.unwrap();
        assert_eq!(
            list_votes(&mut conn, community, id).await.unwrap(),
            vec![changed],
            "a second vote overwrites the first"
        );

        let orphan = VoteRow {
            proposal_id: Uuid::new_v4(),
            ..vote
        };
        assert!(
            upsert_vote(&mut conn, community, &orphan).await.is_err(),
            "votes need their proposal row"
        );

        // Every status and kind passes the CHECK vocabulary; the expiry sweep
        // sees only the requested status.
        let statuses = [
            ProposalStatus::Open,
            ProposalStatus::Passed,
            ProposalStatus::Rejected,
            ProposalStatus::Expired,
            ProposalStatus::Settled,
        ];
        let kinds = [
            ProposalKind::Direction,
            ProposalKind::Project,
            ProposalKind::Dri,
            ProposalKind::Shapers,
            ProposalKind::Money,
            ProposalKind::Join,
        ];
        for (i, (status, kind)) in statuses.iter().zip(kinds.iter().cycle()).enumerate() {
            let row = ProposalRow {
                content: proposal(Uuid::new_v4(), *status, *kind),
                event_id: vec![20 + i as u8; 32],
                updated_at: at(1_700_000_002),
            };
            upsert_proposal(&mut conn, community, &row).await.unwrap();
        }
        let expiring = list_proposals_expiring(
            &mut conn,
            community,
            ProposalStatus::Open,
            at(1_700_000_000 + DEFAULT_DECISION_WINDOW_SECS),
        )
        .await
        .unwrap();
        assert_eq!(expiring.len(), 2, "the first proposal and the open one");
        assert!(expiring
            .iter()
            .all(|p| p.content.status == ProposalStatus::Open));
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn drafts_dedupe_per_gap_and_record_outcomes() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let me = hex_id(3);
        let first = draft(1, "item#rota", &me);
        insert_draft(&mut conn, community, &first).await.unwrap();
        assert_eq!(
            get_draft(&mut conn, community, &first.event_id)
                .await
                .unwrap(),
            Some(first.clone())
        );
        assert_eq!(
            find_open_draft_for_gap(&mut conn, community, "item#rota")
                .await
                .unwrap(),
            Some(first.clone())
        );
        assert!(
            insert_draft(&mut conn, community, &draft(2, "item#rota", "shaper"))
                .await
                .is_err(),
            "one open draft per gap"
        );

        let expiring = list_drafts_expiring(
            &mut conn,
            community,
            DraftOutcomeStatus::Open,
            at(1_700_100_000),
        )
        .await
        .unwrap();
        assert_eq!(expiring, vec![first.clone()]);

        let declined = DraftOutcome {
            draft: hex::encode(&first.event_id),
            status: DraftOutcomeStatus::Declined,
            decided_by: Some(me.clone()),
            decided_at: Some(1_700_000_300),
            reason: Some(DeclineReason::WrongHolder),
            result: None,
        };
        assert!(set_draft_outcome(&mut conn, community, &declined, &[9; 32])
            .await
            .unwrap());
        let read = get_draft(&mut conn, community, &first.event_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read.outcome, declined);
        assert_eq!(read.outcome_event_id, Some(vec![9; 32]));
        assert_eq!(
            find_open_draft_for_gap(&mut conn, community, "item#rota")
                .await
                .unwrap(),
            None,
            "a decided draft frees its gap"
        );
        insert_draft(&mut conn, community, &draft(2, "item#rota", "shaper"))
            .await
            .expect("the gap is open again");

        let missing = DraftOutcome {
            draft: hex::encode([0xEE; 32]),
            ..declined
        };
        assert!(!set_draft_outcome(&mut conn, community, &missing, &[9; 32])
            .await
            .unwrap());

        // Every draft kind, status, and reason passes the CHECK vocabulary.
        let kinds = [
            DraftKind::Project,
            DraftKind::Dri,
            DraftKind::Ticket,
            DraftKind::Done,
            DraftKind::Review,
            DraftKind::Objectives,
            DraftKind::Direction,
            DraftKind::Profile,
            DraftKind::Money,
        ];
        for (i, kind) in kinds.iter().enumerate() {
            let mut row = draft(100 + i as u8, &format!("gap-{i}"), "shaper");
            row.draft_kind = *kind;
            insert_draft(&mut conn, community, &row).await.unwrap();
        }
        let reasons = [
            DeclineReason::AlreadyCovered,
            DeclineReason::NotWhatTheLineMeant,
            DeclineReason::TooBig,
            DeclineReason::TooSmall,
            DeclineReason::WrongHolder,
            DeclineReason::NotNow,
            DeclineReason::Other,
        ];
        for (i, reason) in reasons.iter().enumerate() {
            let outcome = DraftOutcome {
                draft: hex::encode([100 + i as u8; 32]),
                status: DraftOutcomeStatus::Declined,
                decided_by: Some(me.clone()),
                decided_at: Some(1_700_000_400),
                reason: Some(*reason),
                result: None,
            };
            assert!(set_draft_outcome(&mut conn, community, &outcome, &[9; 32])
                .await
                .unwrap());
        }
        for (i, status) in [
            (7u8, DraftOutcomeStatus::Accepted, Some(hex_id(0x11))),
            (8, DraftOutcomeStatus::Amended, Some(hex_id(0x12))),
        ]
        .into_iter()
        .map(|(i, s, r)| (i, (s, r)))
        {
            let outcome = DraftOutcome {
                draft: hex::encode([100 + i; 32]),
                status: status.0,
                decided_by: Some(me.clone()),
                decided_at: Some(1_700_000_400),
                reason: None,
                result: status.1,
            };
            assert!(set_draft_outcome(&mut conn, community, &outcome, &[9; 32])
                .await
                .unwrap());
        }
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn progress_health_and_ratings_round_trip() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let item = Uuid::new_v4();
        let note = ProgressNote {
            item: item.to_string(),
            dri: hex_id(3),
            from: 1_700_000_000,
            to: 1_700_003_600,
            summary: "Wired the store".into(),
            hint: ProgressHint::Progressing,
            git_ref: "io/abcd-thing".into(),
            head: "deadbeef".into(),
            commits: vec![],
            files_changed: 3,
            uncommitted: None,
            head_verified: Some(true),
            merged_into: None,
        };
        let mut rows = Vec::new();
        for (i, hint) in [
            ProgressHint::Progressing,
            ProgressHint::Blocked,
            ProgressHint::Ready,
        ]
        .into_iter()
        .enumerate()
        {
            let mut n = note.clone();
            n.hint = hint;
            let row = ProgressRow::from_note(
                &n,
                vec![30 + i as u8; 32],
                vec![3; 32],
                at(1_700_003_600 + i as u64),
            )
            .unwrap();
            insert_progress(&mut conn, community, &row).await.unwrap();
            rows.push(row);
        }
        rows.reverse();
        assert_eq!(
            list_progress_for_item(&mut conn, community, item, 10)
                .await
                .unwrap(),
            rows
        );
        assert_eq!(
            set_progress_merged_into(&mut conn, community, item, "deadbeef", "main")
                .await
                .unwrap(),
            3
        );
        assert!(list_progress_for_item(&mut conn, community, item, 10)
            .await
            .unwrap()
            .iter()
            .all(|r| r.merged_into.as_deref() == Some("main")));

        for (i, band) in [
            HealthBand::Struggling,
            HealthBand::Wobbly,
            HealthBand::Healthy,
        ]
        .into_iter()
        .enumerate()
        {
            let row = HealthRow {
                event_id: vec![40 + i as u8; 32],
                content: HealthRead {
                    item: item.to_string(),
                    week: "2026-W38".into(),
                    pct: 40.0 + i as f64,
                    band,
                    factors: vec![HealthFactor {
                        name: "cadence".into(),
                        value: 0.5,
                        weight: 1.0,
                        rows: vec![hex_id(30)],
                    }],
                    sentences: vec![GroundedSentence {
                        text: "Three notes this week.".into(),
                        rows: vec![hex_id(30)],
                    }],
                    formula: "cadence".into(),
                },
                read_at: at(1_700_010_000 + i as u64),
            };
            insert_health(&mut conn, community, &row).await.unwrap();
            assert_eq!(
                latest_health_for_item(&mut conn, community, item)
                    .await
                    .unwrap(),
                Some(row)
            );
            let rating = HealthRatingRow {
                item_id: item,
                week: "2026-W38".into(),
                rater: vec![1; 32],
                band,
                receipt_event_id: vec![50 + i as u8; 32],
                rated_at: at(1_700_010_100 + i as u64),
            };
            upsert_health_rating(&mut conn, community, &rating)
                .await
                .unwrap();
            assert_eq!(
                list_health_ratings(&mut conn, community, item, "2026-W38")
                    .await
                    .unwrap(),
                vec![rating],
                "a Shaper's later rating for the same week replaces the earlier one"
            );
        }
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn profiles_round_trip_and_match_on_skills() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let profile = |pubkey: u8, skills: &[&str]| ProfileRow {
            content: OrgProfile {
                pubkey: hex_id(pubkey),
                version: 1,
                about: "Builds relays.".into(),
                skills: skills
                    .iter()
                    .map(|s| Skill {
                        slug: (*s).into(),
                        label: s.to_uppercase(),
                    })
                    .collect(),
                open_limit: Some(2),
                updated_at: 1_700_000_000,
                receipt: hex_id(60 + pubkey),
            },
            active: true,
            event_id: vec![60 + pubkey; 32],
            updated_at: at(1_700_000_000),
        };
        let rust = profile(1, &["rust", "postgres"]);
        let design = profile(2, &["design"]);
        upsert_profile(&mut conn, community, &rust).await.unwrap();
        upsert_profile(&mut conn, community, &design).await.unwrap();
        assert_eq!(
            get_profile(&mut conn, community, &[1; 32]).await.unwrap(),
            Some(rust.clone())
        );
        assert_eq!(
            find_profiles_with_any_skill(&mut conn, community, &["postgres".into()])
                .await
                .unwrap(),
            vec![rust.clone()]
        );
        assert_eq!(
            find_profiles_with_any_skill(&mut conn, community, &["design".into(), "rust".into()])
                .await
                .unwrap()
                .len(),
            2
        );
        assert!(find_profiles_with_any_skill(&mut conn, community, &[])
            .await
            .unwrap()
            .is_empty());

        assert!(set_profile_active(&mut conn, community, &[1; 32], false)
            .await
            .unwrap());
        assert!(
            find_profiles_with_any_skill(&mut conn, community, &["rust".into()])
                .await
                .unwrap()
                .is_empty(),
            "inactive members are not candidates"
        );
        assert!(!set_profile_active(&mut conn, community, &[9; 32], true)
            .await
            .unwrap());

        let mut v2 = rust.clone();
        v2.content.version = 2;
        v2.content.skills.push(Skill {
            slug: "ops".into(),
            label: "Ops".into(),
        });
        v2.event_id = vec![99; 32];
        upsert_profile(&mut conn, community, &v2).await.unwrap();
        let skills: Vec<String> = sqlx::query_scalar(
            "SELECT skills FROM io_profiles WHERE community_id = $1 AND pubkey = $2",
        )
        .bind(community.as_uuid())
        .bind(vec![1u8; 32])
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        assert_eq!(skills, vec!["rust", "postgres", "ops"]);
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn ledger_appends_and_reads_per_object() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let item = Uuid::new_v4().to_string();
        let entry = |verb: &str, seconds: u64| LedgerEntry {
            at: at(seconds),
            actor: if verb == "item_closed_by_rule" {
                LEDGER_ACTOR_RELAY.into()
            } else {
                hex_id(3)
            },
            verb: verb.into(),
            object_type: "item".into(),
            object_id: item.clone(),
            receipt_event_id: Some(vec![1; 32]),
            detail: serde_json::json!({ "for": hex_id(3) }),
        };
        let first = insert_ledger(&mut conn, community, &entry("item_created", 1_700_000_000))
            .await
            .unwrap();
        let second = insert_ledger(
            &mut conn,
            community,
            &entry("item_closed_by_rule", 1_700_000_500),
        )
        .await
        .unwrap();
        assert!(second > first);
        insert_ledger(
            &mut conn,
            community,
            &LedgerEntry {
                object_id: "other".into(),
                ..entry("item_created", 1_700_000_600)
            },
        )
        .await
        .unwrap();

        let rows = list_ledger_for_object(&mut conn, community, "item", &item, 10)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, second);
        assert_eq!(rows[0].entry, entry("item_closed_by_rule", 1_700_000_500));
        assert_eq!(rows[1].entry, entry("item_created", 1_700_000_000));
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn hosted_agents_keep_one_live_key() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let first = HostedAgentRow {
            pubkey: vec![0xA1; 32],
            provisioned_at: at(1_700_000_000),
            budget: Some(serde_json::json!({ "usd_per_month": 20 })),
            retired_at: None,
        };
        insert_hosted_agent(&mut conn, community, &first)
            .await
            .unwrap();
        assert_eq!(
            get_live_hosted_agent(&mut conn, community).await.unwrap(),
            Some(first.clone())
        );
        let second = HostedAgentRow {
            pubkey: vec![0xA2; 32],
            budget: None,
            ..first.clone()
        };
        assert!(
            insert_hosted_agent(&mut conn, community, &second)
                .await
                .is_err(),
            "one live hosted key per community"
        );
        assert!(retire_hosted_agent(&mut conn, community, at(1_700_000_900))
            .await
            .unwrap());
        assert_eq!(
            get_live_hosted_agent(&mut conn, community).await.unwrap(),
            None
        );
        insert_hosted_agent(&mut conn, community, &second)
            .await
            .expect("a retired key frees the slot");
        assert_eq!(
            get_live_hosted_agent(&mut conn, community).await.unwrap(),
            Some(second)
        );
        assert!(!retire_hosted_agent(
            &mut conn,
            CommunityId::from_uuid(Uuid::new_v4()),
            at(1_700_000_901)
        )
        .await
        .unwrap_or(false));
    }

    /// The two pool-scoped reads the invite handlers use: Shaper membership
    /// comes from `io_shapers` at request time, and "is an org" from either a
    /// hosted-agent row or a bootstrapped `39103`.
    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn db_reads_shapers_and_org_presence_for_invite_handlers() {
        let (pool, community) = context().await;
        let db = crate::Db::from_pool(pool.clone());
        assert!(!db.is_org_community(community).await.unwrap());
        assert!(!db.is_org_shaper(community, &hex_id(1)).await.unwrap());

        let mut conn = pool.acquire().await.expect("acquire");
        insert_hosted_agent(
            &mut conn,
            community,
            &HostedAgentRow {
                pubkey: vec![0xA1; 32],
                provisioned_at: at(1_700_000_000),
                budget: None,
                retired_at: None,
            },
        )
        .await
        .unwrap();
        assert!(
            db.is_org_community(community).await.unwrap(),
            "a provisioned hosted agent makes the community an org"
        );
        assert!(
            !db.is_org_shaper(community, &hex_id(1)).await.unwrap(),
            "nobody is a Shaper before bootstrap"
        );

        let (other_pool, other) = context().await;
        let other_db = crate::Db::from_pool(other_pool.clone());
        let mut other_conn = other_pool.acquire().await.expect("acquire");
        upsert_shapers(
            &mut other_conn,
            other,
            &ShapersRow {
                content: shapers(None),
                room_channel_id: None,
                event_id: vec![1; 32],
                updated_at: at(1_700_000_000),
            },
        )
        .await
        .unwrap();
        assert!(
            other_db.is_org_community(other).await.unwrap(),
            "a bootstrapped 39103 makes the community an org"
        );
        assert!(other_db.is_org_shaper(other, &hex_id(1)).await.unwrap());
        assert!(other_db.is_org_shaper(other, &hex_id(2)).await.unwrap());
        assert!(!other_db.is_org_shaper(other, &hex_id(3)).await.unwrap());
        assert!(
            !other_db.is_org_shaper(other, "not-a-pubkey").await.unwrap(),
            "a malformed pubkey is simply not a Shaper"
        );
        assert!(
            !db.is_org_shaper(community, &hex_id(1)).await.unwrap(),
            "Shaper sets are per community"
        );
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn writes_roll_back_with_the_callers_transaction() {
        let (pool, community) = context().await;
        let mut tx = pool.begin().await.expect("begin");
        upsert_shapers(
            &mut tx,
            community,
            &ShapersRow {
                content: shapers(None),
                room_channel_id: None,
                event_id: vec![1; 32],
                updated_at: at(1_700_000_000),
            },
        )
        .await
        .unwrap();
        insert_ledger(
            &mut tx,
            community,
            &LedgerEntry {
                at: at(1_700_000_000),
                actor: hex_id(1),
                verb: "shaper_added".into(),
                object_type: "shapers".into(),
                object_id: hex_id(1),
                receipt_event_id: None,
                detail: serde_json::json!({}),
            },
        )
        .await
        .unwrap();
        tx.rollback().await.expect("roll back");

        let mut conn = pool.acquire().await.expect("acquire");
        assert_eq!(get_shapers(&mut conn, community).await.unwrap(), None);
        let ledger: i64 =
            sqlx::query_scalar("SELECT count(*) FROM io_ledger WHERE community_id = $1")
                .bind(community.as_uuid())
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(ledger, 0, "the ledger row leaves with the transaction");
    }
}
