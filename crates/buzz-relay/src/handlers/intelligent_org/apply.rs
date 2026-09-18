//! `apply` — the one write path for intelligent-org state (Protocol §6.1, V3).
//!
//! Every command handler ends in one call here, on the transaction
//! `persist_command_event` opened. For each [`Projection`] the function
//!
//! 1. builds the relay-signed state event with [`super::state`] at a
//!    `created_at` strictly after the coordinate's previous head (NIP-33
//!    ordering keeps the newest; a same-second rewrite would otherwise lose
//!    on a random id tiebreak) and no further than [`MAX_STATE_DRIFT_SECS`]
//!    ahead of the clock, so a future-dated head cannot ratchet forever;
//! 2. writes the typed projection row that mirrors it — for `39103` that
//!    includes making the `#shapers` roster equal to `shapers ∪ {agent}`
//!    (§6.4), so a roster that disagrees with the Shaper set cannot commit;
//!    for `39102` that includes the `io_votes` row of every vote the command
//!    cast, so a vote the table holds is always one the `39102` holds;
//! 3. stores the state event as the coordinate's live head;
//!
//! then appends the ledger rows. Nothing here commits: the handler commits
//! once, after `apply` returns, and a failure anywhere — including the room
//! roster — rolls back the command event, every projection row, every
//! ledger row, and every state event together. Nothing here touches the
//! pool.

use buzz_core::channel::MemberRole;
use buzz_core::event::StoredEvent;
use buzz_core::intelligent_org::{DirectionArtifact, Proposal, Shapers, WorkItem};
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{
    self as store, DirectionRow, LedgerEntry, ProposalRow, ShapersRow, VoteRow, WorkItemRow,
};
use buzz_db::relay_rooms::{self, DesiredMember, RosterChange};
use buzz_db::replaceable::{ParameterizedReplacePrecondition, ParameterizedReplaceStatus};
use buzz_db::{Db, DbError};
use nostr::Keys;
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use super::state::{self, StateDraft};
use crate::handlers::ingest::IngestError;

/// How far ahead of the relay clock a state event may be stamped to stay
/// after its predecessor. Mirrors the ±15 minute window ingest applies to
/// client events; a head further out than this is corrupt, not merely new.
pub const MAX_STATE_DRIFT_SECS: u64 = 900;

/// The role every Shaper holds in `#shapers`. Admins manage members and
/// settings but cannot delete the room or hand it to someone else — the
/// relay is the room's only authority, and a deleted room would strand every
/// later Shaper change.
pub const SHAPER_ROOM_ROLE: MemberRole = MemberRole::Admin;

/// The role the org agent holds in `#shapers` (R-8 settles its role in every
/// other room).
pub const AGENT_ROOM_ROLE: MemberRole = MemberRole::Member;

/// What the command changed, in the canonical §4 content type. `apply`
/// derives the state event and the projection row from it — there is no
/// way to write one without the other.
#[derive(Debug, Clone, PartialEq)]
pub enum Projection {
    /// The community's Shaper set (`39103`, `io_shapers`, the `#shapers` roster).
    Shapers(Shapers),
    /// A proposal (`39102`, `io_proposals`, `io_votes`). `subject` and `item`
    /// feed the §4.4 marker tags; `receipt` is the opening command; `cast`
    /// names the votes this command added to `proposal.votes`.
    Proposal {
        /// Canonical §4.4 content.
        proposal: Box<Proposal>,
        /// The named person / payee / seat, when the kind has one.
        subject: Option<String>,
        /// The work item it is about, when it is about one.
        item: Option<String>,
        /// The opening command's id, hex.
        receipt: String,
        /// The votes this command cast, each already present in
        /// `proposal.votes`; written to `io_votes`.
        cast: Vec<CastVote>,
    },
    /// A confirmed direction version (`39100`, `io_direction`). Versions are
    /// immutable: `apply` inserts, never overwrites.
    Direction(DirectionArtifact),
    /// A work item (`39101`, `io_work_items`). `receipt` is the command that
    /// produced this version.
    WorkItem {
        /// Canonical §4.2 content.
        item: Box<WorkItem>,
        /// The command id, hex.
        receipt: String,
    },
}

/// One vote the command cast, for its `io_votes` row: the `Vote` in the
/// proposal whose `receipt` this is, plus the reason the command carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastVote {
    /// The casting command's id, hex — equal to `Vote::receipt`.
    pub receipt: String,
    /// The `50003` content's `reason`, when given.
    pub reason: Option<String>,
}

impl Projection {
    fn draft(&self) -> Result<StateDraft, IngestError> {
        match self {
            Self::Shapers(s) => state::shapers(s),
            Self::Proposal {
                proposal,
                subject,
                item,
                receipt,
                ..
            } => state::proposal(proposal, subject.as_deref(), item.as_deref(), receipt),
            Self::Direction(artifact) => state::direction(artifact),
            Self::WorkItem { item, receipt } => state::work_item(item, receipt),
        }
    }
}

/// Everything `apply` needs besides the change itself.
pub struct ApplyContext<'a> {
    /// The community being written.
    pub community: CommunityId,
    /// The relay's signing key.
    pub relay: &'a Keys,
    /// The command author (32 bytes) — recorded on roster rows it causes.
    pub actor: &'a [u8],
    /// Relay wall-clock seconds; the floor for every state `created_at`.
    pub now: u64,
}

/// What `apply` wrote, for the handler's post-commit fan-out.
#[derive(Debug, Default)]
pub struct Applied {
    /// The state events now live, in projection order.
    pub state_events: Vec<StoredEvent>,
    /// Roster rows of `#shapers` that changed, with the room they belong to.
    pub roster: Vec<(Uuid, RosterChange)>,
}

fn internal(context: &str, error: impl std::fmt::Display) -> IngestError {
    IngestError::Internal(format!("error: {context}: {error}"))
}

/// Choose the `created_at` for the next head of a coordinate: never before
/// `now`, always after the previous head, never past the drift ceiling.
pub fn next_created_at(now: u64, previous_head: Option<u64>) -> Result<u64, IngestError> {
    let after_head = previous_head.map_or(0, |head| head.saturating_add(1));
    let created_at = now.max(after_head);
    if created_at > now.saturating_add(MAX_STATE_DRIFT_SECS) {
        return Err(IngestError::Internal(format!(
            "error: state head is {}s ahead of the relay clock; refusing to ratchet past it",
            created_at - now
        )));
    }
    Ok(created_at)
}

/// The roster `#shapers` must hold for this `39103`: every Shaper as
/// [`SHAPER_ROOM_ROLE`], the agent as [`AGENT_ROOM_ROLE`] unless it also
/// holds a seat.
pub fn desired_room_roster(shapers: &Shapers) -> Result<Vec<DesiredMember>, IngestError> {
    let mut desired = Vec::with_capacity(shapers.shapers.len() + 1);
    for p in &shapers.shapers {
        desired.push(DesiredMember {
            pubkey: store::hex32(p).map_err(|e| internal("shaper pubkey", e))?,
            role: SHAPER_ROOM_ROLE,
        });
    }
    if let Some(agent) = &shapers.agent {
        if !shapers.shapers.contains(agent) {
            desired.push(DesiredMember {
                pubkey: store::hex32(agent).map_err(|e| internal("agent pubkey", e))?,
                role: AGENT_ROOM_ROLE,
            });
        }
    }
    Ok(desired)
}

async fn write_shapers(
    conn: &mut PgConnection,
    ctx: &ApplyContext<'_>,
    shapers: &Shapers,
    event_id: &[u8],
    created_at: u64,
) -> Result<Vec<(Uuid, RosterChange)>, IngestError> {
    let room = shapers
        .room
        .as_deref()
        .ok_or_else(|| IngestError::Internal("error: 39103 without a room".into()))?;
    let room = Uuid::parse_str(room).map_err(|e| internal("39103 room id", e))?;
    let row = ShapersRow {
        content: shapers.clone(),
        room_channel_id: Some(room),
        event_id: event_id.to_vec(),
        updated_at: store::ts(created_at).map_err(|e| internal("state created_at", e))?,
    };
    store::upsert_shapers(conn, ctx.community, &row)
        .await
        .map_err(|e| internal("write io_shapers", e))?;

    let desired = desired_room_roster(shapers)?;
    let changes = relay_rooms::sync_room_roster(conn, ctx.community, room, &desired, ctx.actor)
        .await
        .map_err(|e| match e {
            DbError::ChannelNotFound(id) => IngestError::Internal(format!(
                "error: the #shapers room {id} is missing; restore it before changing the Shapers"
            )),
            other => internal("sync #shapers roster", other),
        })?;
    Ok(changes.into_iter().map(|c| (room, c)).collect())
}

async fn write_proposal(
    conn: &mut PgConnection,
    ctx: &ApplyContext<'_>,
    proposal: &Proposal,
    cast: &[CastVote],
    event_id: &[u8],
    created_at: u64,
) -> Result<(), IngestError> {
    let row = ProposalRow {
        content: proposal.clone(),
        event_id: event_id.to_vec(),
        updated_at: store::ts(created_at).map_err(|e| internal("state created_at", e))?,
    };
    store::upsert_proposal(conn, ctx.community, &row)
        .await
        .map_err(|e| internal("write io_proposals", e))?;

    let proposal_id = Uuid::parse_str(&proposal.id).map_err(|e| internal("proposal id", e))?;
    for cast in cast {
        let vote = proposal
            .votes
            .iter()
            .find(|v| v.receipt == cast.receipt)
            .ok_or_else(|| {
                IngestError::Internal(format!(
                    "error: cast vote {} is not in proposal {}",
                    cast.receipt, proposal.id
                ))
            })?;
        let row = VoteRow {
            proposal_id,
            voter: store::hex32(&vote.p).map_err(|e| internal("voter pubkey", e))?,
            vote: vote.vote,
            reason: cast.reason.clone(),
            receipt_event_id: store::hex32(&vote.receipt)
                .map_err(|e| internal("vote receipt", e))?,
            cast_at: store::ts(vote.at).map_err(|e| internal("vote time", e))?,
        };
        store::upsert_vote(conn, ctx.community, &row)
            .await
            .map_err(|e| internal("write io_votes", e))?;
    }
    Ok(())
}

async fn write_direction(
    conn: &mut PgConnection,
    ctx: &ApplyContext<'_>,
    artifact: &DirectionArtifact,
    event_id: &[u8],
) -> Result<(), IngestError> {
    let row = DirectionRow {
        content: artifact.clone(),
        event_id: event_id.to_vec(),
    };
    store::insert_direction(conn, ctx.community, &row)
        .await
        .map_err(|e| internal("write io_direction", e))
}

async fn write_work_item(
    conn: &mut PgConnection,
    ctx: &ApplyContext<'_>,
    item: &WorkItem,
    event_id: &[u8],
    created_at: u64,
) -> Result<(), IngestError> {
    let id = Uuid::parse_str(&item.id).map_err(|e| internal("work item id", e))?;
    let existing = store::get_work_item(conn, ctx.community, id)
        .await
        .map_err(|e| internal("read io_work_items", e))?;
    let updated_at = store::ts(created_at).map_err(|e| internal("state created_at", e))?;
    let row = WorkItemRow {
        content: item.clone(),
        event_id: event_id.to_vec(),
        last_progress_at: existing.as_ref().and_then(|row| row.last_progress_at),
        done_at: existing.as_ref().and_then(|row| row.done_at),
        created_at: existing
            .as_ref()
            .map(|row| row.created_at)
            .unwrap_or(updated_at),
        updated_at,
    };
    store::upsert_work_item(conn, ctx.community, &row)
        .await
        .map_err(|e| internal("write io_work_items", e))
}

/// Write `projections` and `ledger` on `tx`. See the module docs for the
/// order and the guarantees.
pub async fn apply(
    db: &Db,
    tx: &mut Transaction<'static, Postgres>,
    ctx: &ApplyContext<'_>,
    projections: &[Projection],
    ledger: &[LedgerEntry],
) -> Result<Applied, IngestError> {
    let relay_pubkey = ctx.relay.public_key().to_bytes();
    let mut applied = Applied::default();

    for projection in projections {
        let draft = projection.draft()?;
        let head = store::state_head_created_at(
            tx,
            ctx.community,
            draft.kind,
            &relay_pubkey,
            &draft.d_tag,
        )
        .await
        .map_err(|e| internal("read state head", e))?;
        let created_at = next_created_at(ctx.now, head)?;
        let event = state::sign(&draft, ctx.relay, created_at)?;
        let event_id = event.id.to_bytes();

        match projection {
            Projection::Shapers(shapers) => {
                let changes = write_shapers(tx, ctx, shapers, &event_id, created_at).await?;
                applied.roster.extend(changes);
            }
            Projection::Proposal { proposal, cast, .. } => {
                write_proposal(tx, ctx, proposal, cast, &event_id, created_at).await?;
            }
            Projection::Direction(artifact) => {
                write_direction(tx, ctx, artifact, &event_id).await?;
            }
            Projection::WorkItem { item, .. } => {
                write_work_item(tx, ctx, item, &event_id, created_at).await?;
            }
        }

        let stored = db
            .replace_parameterized_event_in_transaction(
                tx,
                ctx.community,
                &event,
                &draft.d_tag,
                None,
                ParameterizedReplacePrecondition::Unconditional,
            )
            .await
            .map_err(|e| internal("store state event", e))?;
        if stored.status != ParameterizedReplaceStatus::Inserted {
            return Err(IngestError::Internal(format!(
                "error: state event {}/{} was not accepted as the live head ({:?})",
                draft.kind, draft.d_tag, stored.status
            )));
        }
        applied.state_events.push(stored.event);
    }

    for entry in ledger {
        store::insert_ledger(tx, ctx.community, entry)
            .await
            .map_err(|e| internal("write io_ledger", e))?;
    }

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::intelligent_org::{
        DecisionRules, DEFAULT_DECISION_WINDOW_SECS, DEFAULT_OFFER_WINDOW_SECS,
    };

    #[test]
    fn next_created_at_floors_at_now_and_steps_past_the_head() {
        assert_eq!(next_created_at(1_000, None).unwrap(), 1_000);
        assert_eq!(next_created_at(1_000, Some(900)).unwrap(), 1_000);
        assert_eq!(next_created_at(1_000, Some(1_000)).unwrap(), 1_001);
        assert_eq!(
            next_created_at(1_000, Some(1_000 + MAX_STATE_DRIFT_SECS - 1)).unwrap(),
            1_000 + MAX_STATE_DRIFT_SECS
        );
        assert!(matches!(
            next_created_at(1_000, Some(1_000 + MAX_STATE_DRIFT_SECS)),
            Err(IngestError::Internal(m)) if m.contains("ahead of the relay clock")
        ));
    }

    #[test]
    fn desired_roster_is_shapers_as_admins_plus_the_agent_as_member() {
        let pk = |seed: u8| hex::encode([seed; 32]);
        let mut shapers = Shapers {
            founder: pk(1),
            shapers: vec![pk(1), pk(2)],
            offered: vec![],
            room: Some(Uuid::nil().to_string()),
            agent: Some(pk(0xA0)),
            agent_hosted: true,
            rules: DecisionRules::default(),
            decision_window_secs: DEFAULT_DECISION_WINDOW_SECS,
            offer_window_secs: DEFAULT_OFFER_WINDOW_SECS,
            updated_at: 0,
            receipt: pk(9),
        };
        assert_eq!(
            desired_room_roster(&shapers).unwrap(),
            vec![
                DesiredMember {
                    pubkey: vec![1; 32],
                    role: MemberRole::Admin,
                },
                DesiredMember {
                    pubkey: vec![2; 32],
                    role: MemberRole::Admin,
                },
                DesiredMember {
                    pubkey: vec![0xA0; 32],
                    role: MemberRole::Member,
                },
            ]
        );
        shapers.agent = None;
        assert_eq!(desired_room_roster(&shapers).unwrap().len(), 2);
        shapers.agent = Some(pk(2));
        assert_eq!(
            desired_room_roster(&shapers).unwrap().len(),
            2,
            "an agent holding a seat is listed once, as a Shaper"
        );
    }
}
