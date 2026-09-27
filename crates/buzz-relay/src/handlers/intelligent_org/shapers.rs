//! `50001` propose, `50019` accept, `50020` step down, and the `shapers`
//! execution rows of §5.3 (Protocol §3.2, §4.5, §6.4).
//!
//! Each handler follows one shape: parse the tags and content with no
//! database; persist the command (`persist_command_event`, which returns the
//! open transaction or reports a replay); take the community's executor lock
//! and read the live `39103` on that transaction; decide with
//! [`super::authorize`]; build the next content; hand everything to
//! [`super::apply::apply`]; commit; fan out. A refusal after the persist
//! drops the transaction, so a rejected command is never stored (§3.2).
//!
//! A `shapers` proposal's `39102.payload` is the command content plus the
//! `op` and (when present) `p` its tags carried, so the proposal can be
//! executed from its own content when the passing vote arrives.

use buzz_core::channel::{ChannelType, ChannelVisibility};
use buzz_core::intelligent_org::{
    DecisionRule, DecisionRules, Executed, OfferedSeat, Proposal, ProposalKind, ProposalStatus,
    RulesContent, Shapers, ShapersOp, Vote, VoteChoice, WhyContent, DEFAULT_DECISION_WINDOW_SECS,
    DEFAULT_OFFER_WINDOW_SECS,
};
use buzz_db::intelligent_org::{self as store};
use buzz_db::relay_rooms;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::apply::{CastVote, Projection};
use super::proposals::{self, Execution, Opening};
use super::{
    authorize, begin, content_value, current_shapers, internal, object, parse_content,
    persist_write, pubkey_tag, tag_value, uuid_tag, Command, Persisted,
};
use crate::handlers::ingest::{IngestError, IngestResult};

/// The `#shapers` room's name as stored (the leading `#` is presentation).
pub const SHAPERS_ROOM_NAME: &str = "shapers";
/// The `#shapers` room's description.
pub const SHAPERS_ROOM_DESCRIPTION: &str =
    "Direction talk for the Shapers and the org agent. Membership follows the Shaper set.";

/// `39102.executed` for every `shapers` op: the one Shaper set.
fn executed_shapers() -> Executed {
    Executed {
        kind: object::SHAPERS.to_owned(),
        id: object::SHAPERS.to_owned(),
    }
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

fn op_wire(op: ShapersOp) -> String {
    serde_json::to_value(op)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The `op` and `p` a `shapers` proposal's payload carries alongside the
/// command content.
#[derive(Debug, Deserialize)]
struct ShapersPayload {
    op: ShapersOp,
    #[serde(default)]
    p: Option<String>,
}

/// The command content with `op` and `p` folded in (module docs).
fn payload_with_op(
    mut content: serde_json::Value,
    op: ShapersOp,
    subject: Option<&str>,
) -> Result<serde_json::Value, IngestError> {
    let fields = content.as_object_mut().ok_or_else(|| {
        IngestError::Rejected("invalid: command content must be a JSON object".into())
    })?;
    fields.insert("op".into(), serde_json::Value::String(op_wire(op)));
    if let Some(p) = subject {
        fields.insert("p".into(), serde_json::Value::String(p.to_owned()));
    }
    Ok(content)
}

/// The §4.4 subject of a `shapers` proposal — `p` for add and remove (the
/// person named or the seat), none for rules and agent.
pub(super) fn subject_of(payload: &serde_json::Value) -> Option<String> {
    let parsed: ShapersPayload = serde_json::from_value(payload.clone()).ok()?;
    match parsed.op {
        ShapersOp::Add | ShapersOp::Remove => parsed.p,
        ShapersOp::Rules | ShapersOp::Agent => None,
    }
}

/// `io_shapers_propose`: the owner's bootstrap self-add while no `39103`
/// exists (§6.4); otherwise a Shaper opens a `shapers` proposal — add and
/// remove under `rules.shapers`, rules and agent always under `all` (§4.5).
pub(super) async fn propose(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let op = shapers_op(cmd)?;
    let subject = pubkey_tag(cmd.event, "p")?;
    if matches!(op, ShapersOp::Add | ShapersOp::Remove) && subject.is_none() {
        return Err(IngestError::Rejected(
            "invalid: op=add and op=remove need a p tag".into(),
        ));
    }
    let content = content_value(cmd.event)?;
    match op {
        ShapersOp::Add | ShapersOp::Remove | ShapersOp::Agent => {
            parse_content::<WhyContent>(cmd.event)?;
        }
        ShapersOp::Rules => {
            // `rules` is required, so empty content is not `{}` here.
            let rules: RulesContent = serde_json::from_value(content.clone())
                .map_err(|e| IngestError::Rejected(format!("invalid: command content: {e}")))?;
            authorize::rules_content(&rules)?;
        }
    }
    let payload = payload_with_op(content, op, subject.as_deref())?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let Some(shapers) = current_shapers(&mut tx, cmd).await? else {
        let is_owner = actor_is_owner(cmd).await?;
        authorize::bootstrap(op, subject.as_deref(), &cmd.actor_hex, is_owner)?;
        return bootstrap(cmd, tx, payload).await;
    };

    let subject_is_member = match (op, subject.as_deref()) {
        (ShapersOp::Agent, Some(p)) => cmd.is_member(p).await?,
        _ => false,
    };
    authorize::open_shapers(
        &shapers,
        &cmd.actor_hex,
        op,
        subject.as_deref(),
        subject_is_member,
        cmd.at,
    )?;
    let rule = match op {
        ShapersOp::Add | ShapersOp::Remove => shapers.rules.shapers,
        ShapersOp::Rules | ShapersOp::Agent => DecisionRule::ALL,
    };
    let opening = Opening {
        kind: ProposalKind::Shapers,
        rule,
        subject: match op {
            ShapersOp::Add | ShapersOp::Remove => subject.clone(),
            ShapersOp::Rules | ShapersOp::Agent => None,
        },
        item: None,
        payload,
        detail: serde_json::json!({
            "kind": "shapers", "op": op_wire(op), "p": subject, "rule": rule,
        }),
    };
    proposals::open_and_settle(cmd, tx, shapers, opening).await
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
        executed: Some(executed_shapers()),
        settlement: None,
    };

    let mut rows = vec![
        cmd.ledger(
            "proposal_opened",
            object::PROPOSAL,
            &proposal_id,
            serde_json::json!({
                "kind": "shapers", "op": "add", "p": cmd.actor_hex, "bootstrap": true
            }),
        )?,
        cmd.ledger(
            "vote_cast",
            object::PROPOSAL,
            &proposal_id,
            serde_json::json!({ "vote": "agree", "with_open": true }),
        )?,
        cmd.ledger(
            "proposal_passed",
            object::PROPOSAL,
            &proposal_id,
            serde_json::json!({
                "needed": 1, "agrees": 1, "executed": executed_shapers()
            }),
        )?,
        cmd.ledger(
            "shaper_added",
            object::SHAPERS,
            &cmd.actor_hex,
            serde_json::json!({ "proposal": proposal_id, "founder": true, "room": room }),
        )?,
    ];
    if let Some(agent) = &agent {
        rows.push(cmd.ledger(
            "agent_changed",
            object::AGENT,
            agent,
            serde_json::json!({ "from": null, "to": agent, "hosted": true, "why": "bootstrap" }),
        )?);
        let agent_bytes = store::hex32(agent).map_err(|e| internal("agent pubkey", e))?;
        let counts = buzz_db::org_agent_membership::backfill_agent_membership(
            &mut tx,
            community,
            &agent_bytes,
            &cmd.actor_bytes,
            Some(room),
        )
        .await
        .map_err(|e| internal("backfill agent membership", e))?;
        rows.push(cmd.ledger(
            "agent_membership_synced",
            object::AGENT,
            agent,
            serde_json::json!({
                "agent": agent,
                "channels": counts.channels,
                "dms": counts.dms,
                "why": buzz_db::org_agent_membership::AGENT_MEMBERSHIP_WHY_BOOTSTRAP,
            }),
        )?);
    }

    let projections = vec![
        Projection::Proposal {
            proposal: Box::new(proposal),
            subject: Some(cmd.actor_hex.clone()),
            item: None,
            receipt: receipt.clone(),
            cast: vec![CastVote {
                receipt,
                reason: None,
            }],
        },
        Projection::Shapers(shapers),
    ];
    persist_write(
        cmd,
        tx,
        projections,
        rows,
        serde_json::json!({ "proposal": proposal_id, "room": room }).to_string(),
        Some(room),
    )
    .await
}

/// The §5.3 execution rows for a passed `shapers` proposal, against the
/// `39103` live when the passing vote arrives. An add whose `p` is already a
/// Shaper, or a remove whose `p` already left, passes with nothing to do —
/// the seat or the departure happened by another path — and writes no
/// `39103`; every other op returns the next Shaper set.
pub(super) async fn execute(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    shapers: &Shapers,
    proposal: &Proposal,
) -> Result<Execution, IngestError> {
    let payload: ShapersPayload = serde_json::from_value(proposal.payload.clone())
        .map_err(|e| internal("shapers proposal payload", e))?;
    let mut next = shapers.clone();
    let mut rows = Vec::new();
    match payload.op {
        ShapersOp::Add => {
            let p = payload
                .p
                .ok_or_else(|| internal("shapers/add payload", "no p"))?;
            if !authorize::is_shaper(&next, &p) {
                next.offered.retain(|seat| seat.p != p);
                next.offered.push(OfferedSeat {
                    p: p.clone(),
                    proposal: proposal.id.clone(),
                    at: cmd.at,
                });
                rows.push(cmd.ledger(
                    "shaper_offered",
                    object::SHAPERS,
                    &p,
                    serde_json::json!({
                        "proposal": proposal.id,
                        "lapses_at": cmd.at.saturating_add(next.offer_window_secs),
                    }),
                )?);
            }
        }
        ShapersOp::Remove => {
            let p = payload
                .p
                .ok_or_else(|| internal("shapers/remove payload", "no p"))?;
            if authorize::is_shaper(&next, &p) {
                if next.shapers.len() == 1 {
                    return Err(IngestError::Rejected("invalid: last shaper".into()));
                }
                next.shapers.retain(|s| s != &p);
                rows.push(cmd.ledger(
                    "shaper_removed",
                    object::SHAPERS,
                    &p,
                    serde_json::json!({ "proposal": proposal.id }),
                )?);
            }
        }
        ShapersOp::Rules => {
            let content: RulesContent = serde_json::from_value(proposal.payload.clone())
                .map_err(|e| internal("shapers/rules payload", e))?;
            authorize::rules_content(&content)?;
            let from = rules_detail(&next);
            next.rules = content.rules;
            if let Some(secs) = content.decision_window_secs {
                next.decision_window_secs = secs;
            }
            if let Some(secs) = content.offer_window_secs {
                next.offer_window_secs = secs;
            }
            if next != *shapers {
                rows.push(cmd.ledger(
                    "rules_changed",
                    object::SHAPERS,
                    object::SHAPERS,
                    serde_json::json!({
                        "proposal": proposal.id, "from": from, "to": rules_detail(&next),
                    }),
                )?);
            }
        }
        ShapersOp::Agent => {
            let from = next.agent.clone();
            match payload.p {
                Some(p) => {
                    let is_member = cmd.is_member(&p).await?;
                    authorize::agent_candidate(&next, &p, is_member)?;
                    next.agent = Some(p);
                    next.agent_hosted = false;
                }
                None => {
                    let hosted = store::get_live_hosted_agent(tx, cmd.tenant.community())
                        .await
                        .map_err(|e| internal("read io_hosted_agents", e))?
                        .map(|row| hex::encode(row.pubkey));
                    next.agent_hosted = hosted.is_some();
                    next.agent = hosted;
                }
            }
            if next != *shapers {
                rows.push(cmd.ledger(
                    "agent_changed",
                    object::AGENT,
                    next.agent.as_deref().unwrap_or("none"),
                    serde_json::json!({
                        "from": from, "to": next.agent, "hosted": next.agent_hosted,
                        "why": "proposal", "proposal": proposal.id,
                    }),
                )?);
                let from_bytes = from
                    .as_deref()
                    .map(store::hex32)
                    .transpose()
                    .map_err(|e| internal("previous agent pubkey", e))?;
                let to_bytes = next
                    .agent
                    .as_deref()
                    .map(store::hex32)
                    .transpose()
                    .map_err(|e| internal("next agent pubkey", e))?;
                if let Some(from_bytes) = from_bytes {
                    let counts = buzz_db::org_agent_membership::move_agent_membership(
                        tx,
                        cmd.tenant.community(),
                        &from_bytes,
                        to_bytes.as_deref(),
                        &cmd.actor_bytes,
                    )
                    .await
                    .map_err(|e| internal("move agent membership", e))?;
                    rows.push(cmd.ledger(
                        "agent_membership_synced",
                        object::AGENT,
                        next.agent.as_deref().unwrap_or("none"),
                        serde_json::json!({
                            "agent": next.agent,
                            "channels": counts.channels,
                            "dms": counts.dms,
                            "why": buzz_db::org_agent_membership::AGENT_MEMBERSHIP_WHY_AGENT_CHANGED,
                        }),
                    )?);
                } else if let Some(to_bytes) = to_bytes {
                    let counts = buzz_db::org_agent_membership::backfill_agent_membership(
                        tx,
                        cmd.tenant.community(),
                        &to_bytes,
                        &cmd.actor_bytes,
                        None,
                    )
                    .await
                    .map_err(|e| internal("backfill agent membership", e))?;
                    rows.push(cmd.ledger(
                        "agent_membership_synced",
                        object::AGENT,
                        next.agent.as_deref().unwrap_or("none"),
                        serde_json::json!({
                            "agent": next.agent,
                            "channels": counts.channels,
                            "dms": counts.dms,
                            "why": buzz_db::org_agent_membership::AGENT_MEMBERSHIP_WHY_AGENT_CHANGED,
                        }),
                    )?);
                }
            }
        }
    }

    let projections = if next == *shapers {
        vec![]
    } else {
        next.updated_at = cmd.at;
        next.receipt = cmd.receipt_hex();
        vec![Projection::Shapers(next)]
    };
    Ok(Execution {
        executed: executed_shapers(),
        projections,
        rows,
    })
}

fn rules_detail(shapers: &Shapers) -> serde_json::Value {
    serde_json::json!({
        "rules": shapers.rules,
        "decision_window_secs": shapers.decision_window_secs,
        "offer_window_secs": shapers.offer_window_secs,
    })
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

    let rows = vec![cmd.ledger(
        "shaper_added",
        object::SHAPERS,
        &cmd.actor_hex,
        serde_json::json!({ "proposal": proposal }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::Shapers(next)],
        rows,
        "{}".into(),
        None,
    )
    .await
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

    let rows = vec![cmd.ledger(
        "shaper_stepped_down",
        object::SHAPERS,
        &cmd.actor_hex,
        serde_json::json!({ "why": content.why }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::Shapers(next)],
        rows,
        "{}".into(),
        None,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_folds_op_and_p_into_the_content_and_subject_reads_them_back() {
        let p = hex::encode([7u8; 32]);
        let payload = payload_with_op(
            serde_json::json!({ "why": "knows the domain" }),
            ShapersOp::Add,
            Some(&p),
        )
        .unwrap();
        assert_eq!(
            payload,
            serde_json::json!({ "why": "knows the domain", "op": "add", "p": p })
        );
        assert_eq!(subject_of(&payload).as_deref(), Some(p.as_str()));

        let agent = payload_with_op(serde_json::json!({}), ShapersOp::Agent, Some(&p)).unwrap();
        assert_eq!(
            subject_of(&agent),
            None,
            "an agent's p is not a §4.4 subject"
        );
        let rules = payload_with_op(
            serde_json::json!({ "rules": { "shapers": "all" } }),
            ShapersOp::Rules,
            None,
        )
        .unwrap();
        assert_eq!(subject_of(&rules), None);
        let content: RulesContent = serde_json::from_value(rules).unwrap();
        assert_eq!(content.rules.shapers, DecisionRule::ALL);

        assert!(matches!(
            payload_with_op(serde_json::json!([]), ShapersOp::Rules, None),
            Err(IngestError::Rejected(m)) if m == "invalid: command content must be a JSON object"
        ));
    }
}
