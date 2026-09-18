//! `50001` bootstrap, `50019` accept, `50020` step down (Protocol §3.2, §6.4).
//!
//! Each handler follows one shape: parse the tags and content with no
//! database; persist the command (`persist_command_event`, which returns the
//! open transaction or reports a replay); take the community's executor lock
//! and read the live `39103` on that transaction; decide with
//! [`super::authorize`]; build the next content; hand everything to
//! [`super::apply::apply`]; commit; fan out. A refusal after the persist
//! drops the transaction, so a rejected command is never stored (§3.2).

use buzz_core::channel::{ChannelType, ChannelVisibility};
use buzz_core::intelligent_org::{
    DecisionRules, Executed, Proposal, ProposalKind, ProposalStatus, Shapers, ShapersOp, Vote,
    VoteChoice, WhyContent, DEFAULT_DECISION_WINDOW_SECS, DEFAULT_OFFER_WINDOW_SECS,
};
use buzz_db::intelligent_org::{self as store, LedgerEntry};
use buzz_db::relay_rooms;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::apply::{apply, ApplyContext, Projection};
use super::{authorize, content_value, parse_content, pubkey_tag, tag_value, uuid_tag, Command};
use crate::handlers::command_executor::{persist_command_event, PersistResult};
use crate::handlers::ingest::{IngestError, IngestResult};

/// The `#shapers` room's name as stored (the leading `#` is presentation).
pub const SHAPERS_ROOM_NAME: &str = "shapers";
/// The `#shapers` room's description.
pub const SHAPERS_ROOM_DESCRIPTION: &str =
    "Direction talk for the Shapers and the org agent. Membership follows the Shaper set.";

/// Ledger `object_type` for Shaper-set rows.
const OBJECT_SHAPERS: &str = "shapers";
const OBJECT_PROPOSAL: &str = "proposal";
const OBJECT_AGENT: &str = "agent";

fn internal(context: &str, error: impl std::fmt::Display) -> IngestError {
    IngestError::Internal(format!("error: {context}: {error}"))
}

fn wall_clock() -> u64 {
    chrono::Utc::now().timestamp().unsigned_abs()
}

fn ledger_at(at: u64) -> Result<chrono::DateTime<chrono::Utc>, IngestError> {
    store::ts(at).map_err(|e| internal("ledger time", e))
}

enum Persisted {
    Replay(IngestResult),
    Open(Transaction<'static, Postgres>),
}

/// Store the command and take the executor lock, or report a replay.
async fn begin(cmd: &Command<'_>) -> Result<Persisted, IngestError> {
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

async fn commit(tx: Transaction<'static, Postgres>) -> Result<(), IngestError> {
    tx.commit()
        .await
        .map_err(|e| internal("commit transaction", e))
}

async fn current_shapers(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
) -> Result<Option<Shapers>, IngestError> {
    Ok(store::get_shapers(tx, cmd.tenant.community())
        .await
        .map_err(|e| internal("read io_shapers", e))?
        .map(|row| row.content))
}

async fn actor_is_owner(cmd: &Command<'_>) -> Result<bool, IngestError> {
    let member = cmd
        .state
        .db
        .get_relay_member(cmd.tenant.community(), &cmd.actor_hex)
        .await
        .map_err(|e| internal("read relay membership", e))?;
    Ok(member.is_some_and(|m| m.role == "owner"))
}

fn shapers_op(cmd: &Command<'_>) -> Result<ShapersOp, IngestError> {
    let op = tag_value(cmd.event, "op")
        .ok_or_else(|| IngestError::Rejected("invalid: missing op tag".into()))?;
    serde_json::from_value(serde_json::Value::String(op.to_owned())).map_err(|_| {
        IngestError::Rejected(format!(
            "invalid: unknown op {op:?}; expected add, remove, rules, or agent"
        ))
    })
}

fn ledger(
    cmd: &Command<'_>,
    verb: &str,
    object_type: &str,
    object_id: &str,
    detail: serde_json::Value,
) -> Result<LedgerEntry, IngestError> {
    Ok(LedgerEntry {
        at: ledger_at(cmd.at)?,
        actor: cmd.actor_hex.clone(),
        verb: verb.to_owned(),
        object_type: object_type.to_owned(),
        object_id: object_id.to_owned(),
        receipt_event_id: Some(cmd.receipt_bytes()),
        detail,
    })
}

/// `io_shapers_propose`. This slice executes the bootstrap self-add (§6.4);
/// every other `shapers` proposal opens in R-4a.
pub(super) async fn propose(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let op = shapers_op(cmd)?;
    let subject = pubkey_tag(cmd.event, "p")?;
    if matches!(op, ShapersOp::Add | ShapersOp::Remove) && subject.is_none() {
        return Err(IngestError::Rejected(
            "invalid: op=add and op=remove need a p tag".into(),
        ));
    }
    let payload = content_value(cmd.event)?;
    if matches!(op, ShapersOp::Add | ShapersOp::Remove | ShapersOp::Agent) {
        parse_content::<WhyContent>(cmd.event)?;
    }

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    match current_shapers(&mut tx, cmd).await? {
        None => {
            let is_owner = actor_is_owner(cmd).await?;
            authorize::bootstrap(op, subject.as_deref(), &cmd.actor_hex, is_owner)?;
            bootstrap(cmd, tx, payload).await
        }
        Some(shapers) => {
            authorize::require_shaper(&shapers, &cmd.actor_hex)?;
            let op = serde_json::to_value(op)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            Err(IngestError::Rejected(format!(
                "invalid: shapers op={op} proposals are not implemented yet"
            )))
        }
    }
}

/// The owner's self-add when no `39103` exists: create `#shapers`, read the
/// hosted agent, pass the proposal at once, write `39102` and `39103`.
async fn bootstrap(
    cmd: &Command<'_>,
    mut tx: Transaction<'static, Postgres>,
    payload: serde_json::Value,
) -> Result<IngestResult, IngestError> {
    let community = cmd.tenant.community();
    let room = relay_rooms::create_room(
        &mut tx,
        community,
        SHAPERS_ROOM_NAME,
        ChannelType::Stream,
        ChannelVisibility::Private,
        Some(SHAPERS_ROOM_DESCRIPTION),
        &cmd.actor_bytes,
    )
    .await
    .map_err(|e| internal("create #shapers", e))?;
    let agent = store::get_live_hosted_agent(&mut tx, community)
        .await
        .map_err(|e| internal("read io_hosted_agents", e))?
        .map(|row| hex::encode(row.pubkey));

    let receipt = cmd.receipt_hex();
    let rules = DecisionRules::default();
    let shapers = Shapers {
        founder: cmd.actor_hex.clone(),
        shapers: vec![cmd.actor_hex.clone()],
        offered: vec![],
        room: Some(room.to_string()),
        agent_hosted: agent.is_some(),
        agent: agent.clone(),
        rules,
        decision_window_secs: DEFAULT_DECISION_WINDOW_SECS,
        offer_window_secs: DEFAULT_OFFER_WINDOW_SECS,
        updated_at: cmd.at,
        receipt: receipt.clone(),
    };
    let proposal_id = Uuid::new_v4().to_string();
    let proposal = Proposal {
        id: proposal_id.clone(),
        kind: ProposalKind::Shapers,
        status: ProposalStatus::Passed,
        opened_by: cmd.actor_hex.clone(),
        opened_at: cmd.at,
        expires_at: cmd.at.saturating_add(DEFAULT_DECISION_WINDOW_SECS),
        draft: None,
        payload,
        rule: rules.shapers,
        needed: 1,
        eligible: vec![cmd.actor_hex.clone()],
        votes: vec![Vote {
            p: cmd.actor_hex.clone(),
            vote: VoteChoice::Agree,
            at: cmd.at,
            receipt: receipt.clone(),
        }],
        decided_at: Some(cmd.at),
        executed: Some(Executed {
            kind: OBJECT_SHAPERS.to_owned(),
            id: OBJECT_SHAPERS.to_owned(),
        }),
        settlement: None,
    };

    let mut rows = vec![
        ledger(
            cmd,
            "proposal_opened",
            OBJECT_PROPOSAL,
            &proposal_id,
            serde_json::json!({
                "kind": "shapers", "op": "add", "subject": cmd.actor_hex, "bootstrap": true
            }),
        )?,
        ledger(
            cmd,
            "vote_cast",
            OBJECT_PROPOSAL,
            &proposal_id,
            serde_json::json!({ "vote": "agree", "with_open": true }),
        )?,
        ledger(
            cmd,
            "proposal_passed",
            OBJECT_PROPOSAL,
            &proposal_id,
            serde_json::json!({
                "needed": 1, "agrees": 1, "executed": { "kind": "shapers", "id": "shapers" }
            }),
        )?,
        ledger(
            cmd,
            "shaper_added",
            OBJECT_SHAPERS,
            &cmd.actor_hex,
            serde_json::json!({ "proposal": proposal_id, "founder": true, "room": room }),
        )?,
    ];
    if let Some(agent) = &agent {
        rows.push(ledger(
            cmd,
            "agent_changed",
            OBJECT_AGENT,
            agent,
            serde_json::json!({ "from": null, "to": agent, "hosted": true, "why": "bootstrap" }),
        )?);
    }

    let ctx = ApplyContext {
        community,
        relay: &cmd.state.relay_keypair,
        actor: &cmd.actor_bytes,
        now: wall_clock(),
    };
    let projections = [
        Projection::Proposal {
            proposal: Box::new(proposal),
            subject: Some(cmd.actor_hex.clone()),
            item: None,
            receipt,
        },
        Projection::Shapers(shapers),
    ];
    let applied = apply(&cmd.state.db, &mut tx, &ctx, &projections, &rows).await?;
    commit(tx).await?;
    super::finish(cmd, applied, Some(room)).await;
    Ok(cmd.accepted(serde_json::json!({ "proposal": proposal_id, "room": room }).to_string()))
}

/// `io_shaper_accept`: take the seat a passed `shapers/add` offered.
pub(super) async fn accept(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let proposal = uuid_tag(cmd.event, "e")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing e tag (proposal)".into()))?
        .to_string();
    parse_content::<buzz_core::intelligent_org::EmptyContent>(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let current = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("invalid: no Shapers yet".into()))?;
    authorize::accept_seat(&current, &cmd.actor_hex, &proposal, cmd.at)?;

    let mut next = current;
    next.shapers.push(cmd.actor_hex.clone());
    next.offered
        .retain(|seat| !(seat.p == cmd.actor_hex && seat.proposal == proposal));
    next.updated_at = cmd.at;
    next.receipt = cmd.receipt_hex();

    let rows = [ledger(
        cmd,
        "shaper_added",
        OBJECT_SHAPERS,
        &cmd.actor_hex,
        serde_json::json!({ "proposal": proposal }),
    )?];
    let ctx = ApplyContext {
        community: cmd.tenant.community(),
        relay: &cmd.state.relay_keypair,
        actor: &cmd.actor_bytes,
        now: wall_clock(),
    };
    let applied = apply(
        &cmd.state.db,
        &mut tx,
        &ctx,
        &[Projection::Shapers(next)],
        &rows,
    )
    .await?;
    commit(tx).await?;
    super::finish(cmd, applied, None).await;
    Ok(cmd.accepted("{}".into()))
}

/// `io_shaper_step_down`: leave the Shaper set, unless last.
pub(super) async fn step_down(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let content: WhyContent = parse_content(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let current = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("invalid: no Shapers yet".into()))?;
    authorize::step_down(&current, &cmd.actor_hex)?;

    let mut next = current;
    next.shapers.retain(|p| p != &cmd.actor_hex);
    next.updated_at = cmd.at;
    next.receipt = cmd.receipt_hex();

    let rows = [ledger(
        cmd,
        "shaper_stepped_down",
        OBJECT_SHAPERS,
        &cmd.actor_hex,
        serde_json::json!({ "why": content.why }),
    )?];
    let ctx = ApplyContext {
        community: cmd.tenant.community(),
        relay: &cmd.state.relay_keypair,
        actor: &cmd.actor_bytes,
        now: wall_clock(),
    };
    let applied = apply(
        &cmd.state.db,
        &mut tx,
        &ctx,
        &[Projection::Shapers(next)],
        &rows,
    )
    .await?;
    commit(tx).await?;
    super::finish(cmd, applied, None).await;
    Ok(cmd.accepted("{}".into()))
}
