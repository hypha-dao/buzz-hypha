//! Proposals and votes (Protocol §4.4, §5.3): opening, the D1 opener vote,
//! `50003`, the tally, and execution dispatch.
//!
//! A proposal is opened by the handler of the command that opens it (in this
//! slice `shapers::propose`), which knows the kind, the rule, the subject,
//! and the payload; everything from there on is shared. [`open`] freezes
//! `eligible` (the Shapers now, minus the subject) and resolves `needed`
//! against it, so a later Shaper change never moves the bar. [`settle`]
//! tallies after every vote — including the opener's own `["vote", "agree"]`
//! (Readiness D1) — and, on `passed`, executes in the same transaction
//! through the kind's executor (`shapers::execute` here; `direction` and
//! `dri` land in R-4b), then hands the `39102` and whatever executing
//! produced to [`super::apply::apply`] as one write.

use buzz_core::intelligent_org::{
    DecisionRule, Executed, Proposal, ProposalKind, ProposalStatus, Shapers, Vote, VoteChoice,
    VoteContent,
};
use buzz_db::intelligent_org::{self as store, LedgerEntry};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::apply::{apply, ApplyContext, CastVote, Projection};
use super::{
    authorize, begin, commit, current_shapers, internal, object, parse_content, shapers, tag_value,
    uuid_tag, wall_clock, Command, Persisted,
};
use crate::handlers::ingest::{IngestError, IngestResult};

/// What the opening command decided, before the relay adds the frozen parts.
pub(super) struct Opening {
    /// What it decides.
    pub kind: ProposalKind,
    /// The rule that applies — `39103.rules.<kind>`, or `all` where §4.5
    /// says so.
    pub rule: DecisionRule,
    /// The §4.4 subject: left out of `eligible`, tagged
    /// `["p", subject, "", "subject"]`.
    pub subject: Option<String>,
    /// The work item it is about, when it is about one.
    pub item: Option<String>,
    /// The `39102.payload`.
    pub payload: serde_json::Value,
    /// The `proposal_opened` ledger detail.
    pub detail: serde_json::Value,
}

/// What executing a passed proposal produced, for the same `apply` call as
/// the `39102`.
pub(super) struct Execution {
    /// `39102.executed`.
    pub executed: Executed,
    /// State the execution changed, in write order.
    pub projections: Vec<Projection>,
    /// Ledger rows the execution adds after `proposal_passed`.
    pub rows: Vec<LedgerEntry>,
}

/// Build the open proposal: `eligible` is every Shaper except the subject,
/// `needed` is `rule` resolved against it now (§4.5), `expires_at` is
/// `opened_at + decision_window_secs`.
pub(super) fn open(cmd: &Command<'_>, shapers: &Shapers, opening: &Opening) -> Proposal {
    let eligible: Vec<String> = shapers
        .shapers
        .iter()
        .filter(|p| Some(p.as_str()) != opening.subject.as_deref())
        .cloned()
        .collect();
    let needed = opening
        .rule
        .needed_for(u32::try_from(eligible.len()).unwrap_or(u32::MAX));
    Proposal {
        id: Uuid::new_v4().to_string(),
        kind: opening.kind,
        status: ProposalStatus::Open,
        opened_by: cmd.actor_hex.clone(),
        opened_at: cmd.at,
        expires_at: cmd.at.saturating_add(shapers.decision_window_secs),
        draft: None,
        payload: opening.payload.clone(),
        rule: opening.rule,
        needed,
        eligible,
        votes: vec![],
        decided_at: None,
        executed: None,
        settlement: None,
    }
}

/// The §5.3 verdict on the votes so far: `Passed` when `agrees ≥ needed`,
/// `Rejected` when the declines make passing impossible, `None` while
/// either is still reachable.
pub fn tally(proposal: &Proposal) -> Option<ProposalStatus> {
    let agrees = proposal
        .votes
        .iter()
        .filter(|v| v.vote == VoteChoice::Agree)
        .count();
    let declines = proposal.votes.len() - agrees;
    let needed = proposal.needed as usize;
    if agrees >= needed {
        Some(ProposalStatus::Passed)
    } else if declines > proposal.eligible.len().saturating_sub(needed) {
        Some(ProposalStatus::Rejected)
    } else {
        None
    }
}

fn vote_tag(cmd: &Command<'_>) -> Result<Option<VoteChoice>, IngestError> {
    let Some(value) = tag_value(cmd.event, "vote") else {
        return Ok(None);
    };
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map(Some)
        .map_err(|_| {
            IngestError::Rejected(format!(
                "invalid: unknown vote {value:?}; expected agree or decline"
            ))
        })
}

/// The `["vote", "agree"]` an opener may add (D1). `decline` makes no sense
/// on an opening command and is refused; the tag from a non-eligible opener
/// is ignored, not rejected.
fn opener_agrees(cmd: &Command<'_>, proposal: &Proposal) -> Result<bool, IngestError> {
    match vote_tag(cmd)? {
        None => Ok(false),
        Some(VoteChoice::Decline) => Err(IngestError::Rejected(
            "invalid: an opening command may only carry vote=agree".into(),
        )),
        Some(VoteChoice::Agree) => Ok(proposal.eligible.iter().any(|p| p == &cmd.actor_hex)),
    }
}

/// The §4.4 `subject` and `item` markers of a stored proposal, derived from
/// its content the same way its opener derived them.
fn markers(proposal: &Proposal) -> (Option<String>, Option<String>) {
    match proposal.kind {
        ProposalKind::Shapers => (shapers::subject_of(&proposal.payload), None),
        ProposalKind::Direction
        | ProposalKind::Project
        | ProposalKind::Dri
        | ProposalKind::Money
        | ProposalKind::Join => (None, None),
    }
}

/// Open `opening` on `tx`, record the opener's agree if the command carries
/// one, and settle.
pub(super) async fn open_and_settle(
    cmd: &Command<'_>,
    tx: Transaction<'static, Postgres>,
    shapers: Shapers,
    opening: Opening,
) -> Result<IngestResult, IngestError> {
    let mut proposal = open(cmd, &shapers, &opening);
    let mut rows = vec![cmd.ledger(
        "proposal_opened",
        object::PROPOSAL,
        &proposal.id,
        opening.detail.clone(),
    )?];
    let mut cast = Vec::new();
    if opener_agrees(cmd, &proposal)? {
        proposal.votes.push(Vote {
            p: cmd.actor_hex.clone(),
            vote: VoteChoice::Agree,
            at: cmd.at,
            receipt: cmd.receipt_hex(),
        });
        cast.push(CastVote {
            receipt: cmd.receipt_hex(),
            reason: None,
        });
        rows.push(cmd.ledger(
            "vote_cast",
            object::PROPOSAL,
            &proposal.id,
            serde_json::json!({ "vote": "agree", "with_open": true }),
        )?);
    }
    settle(
        cmd,
        tx,
        shapers,
        proposal,
        opening.subject,
        opening.item,
        cast,
        rows,
        cmd.receipt_hex(),
    )
    .await
}

/// `io_vote`: record the vote (replacing the sender's earlier one while the
/// proposal is open — ledger `vote_changed`), then settle.
pub(super) async fn vote(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let proposal_id = uuid_tag(cmd.event, "e")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing e tag (proposal)".into()))?;
    let choice =
        vote_tag(cmd)?.ok_or_else(|| IngestError::Rejected("invalid: missing vote tag".into()))?;
    let content: VoteContent = parse_content(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("invalid: no Shapers yet".into()))?;
    let row = store::get_proposal(&mut tx, cmd.tenant.community(), proposal_id)
        .await
        .map_err(|e| internal("read io_proposals", e))?
        .ok_or_else(|| IngestError::Rejected("invalid: unknown proposal".into()))?;
    let mut proposal = row.content;
    authorize::vote(&proposal, &shapers, &cmd.actor_hex, cmd.at)?;

    let vote = Vote {
        p: cmd.actor_hex.clone(),
        vote: choice,
        at: cmd.at,
        receipt: cmd.receipt_hex(),
    };
    let verb = match proposal.votes.iter_mut().find(|v| v.p == cmd.actor_hex) {
        Some(existing) => {
            *existing = vote;
            "vote_changed"
        }
        None => {
            proposal.votes.push(vote);
            "vote_cast"
        }
    };
    let rows = vec![cmd.ledger(
        verb,
        object::PROPOSAL,
        &proposal.id,
        serde_json::json!({ "vote": choice, "reason": content.reason }),
    )?];
    let cast = vec![CastVote {
        receipt: cmd.receipt_hex(),
        reason: content.reason,
    }];
    let (subject, item) = markers(&proposal);
    let receipt = store::get_proposal_opening_receipt(&mut tx, cmd.tenant.community(), proposal_id)
        .await
        .map_err(|e| internal("read 39102 head", e))?
        .ok_or_else(|| {
            IngestError::Internal(format!(
                "error: proposal {proposal_id} has no live 39102; restore it before voting"
            ))
        })?;
    settle(
        cmd, tx, shapers, proposal, subject, item, cast, rows, receipt,
    )
    .await
}

/// Tally; on `passed` execute; then write the `39102`, whatever executing
/// produced, and the ledger rows in one `apply`; commit; fan out.
#[allow(clippy::too_many_arguments)]
async fn settle(
    cmd: &Command<'_>,
    mut tx: Transaction<'static, Postgres>,
    shapers: Shapers,
    mut proposal: Proposal,
    subject: Option<String>,
    item: Option<String>,
    cast: Vec<CastVote>,
    mut rows: Vec<LedgerEntry>,
    receipt: String,
) -> Result<IngestResult, IngestError> {
    let mut projections = Vec::new();
    let agrees = proposal
        .votes
        .iter()
        .filter(|v| v.vote == VoteChoice::Agree)
        .count();
    match tally(&proposal) {
        Some(ProposalStatus::Passed) => {
            proposal.status = ProposalStatus::Passed;
            proposal.decided_at = Some(cmd.at);
            let execution = execute(cmd, &mut tx, &shapers, &proposal).await?;
            proposal.executed = Some(execution.executed.clone());
            rows.push(cmd.ledger(
                "proposal_passed",
                object::PROPOSAL,
                &proposal.id,
                serde_json::json!({
                    "needed": proposal.needed,
                    "agrees": agrees,
                    "executed": execution.executed,
                }),
            )?);
            rows.extend(execution.rows);
            projections.extend(execution.projections);
        }
        Some(ProposalStatus::Rejected) => {
            proposal.status = ProposalStatus::Rejected;
            proposal.decided_at = Some(cmd.at);
            rows.push(cmd.ledger(
                "proposal_rejected",
                object::PROPOSAL,
                &proposal.id,
                serde_json::json!({
                    "needed": proposal.needed,
                    "agrees": agrees,
                    "declines": proposal.votes.len() - agrees,
                }),
            )?);
        }
        _ => {}
    }

    let id = proposal.id.clone();
    let status = proposal.status;
    projections.insert(
        0,
        Projection::Proposal {
            proposal: Box::new(proposal),
            subject,
            item,
            receipt,
            cast,
        },
    );
    let ctx = ApplyContext {
        community: cmd.tenant.community(),
        relay: &cmd.state.relay_keypair,
        actor: &cmd.actor_bytes,
        now: wall_clock(),
    };
    let applied = apply(&cmd.state.db, &mut tx, &ctx, &projections, &rows).await?;
    commit(tx).await?;
    super::finish(cmd, applied, None).await;
    Ok(cmd.accepted(serde_json::json!({ "proposal": id, "status": status }).to_string()))
}

/// The §5.3 execution table, by kind. A kind whose execution has not landed
/// refuses the vote that would pass it, so the proposal stays open rather
/// than passing without effect.
async fn execute(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    shapers: &Shapers,
    proposal: &Proposal,
) -> Result<Execution, IngestError> {
    match proposal.kind {
        ProposalKind::Shapers => shapers::execute(cmd, tx, shapers, proposal).await,
        ProposalKind::Direction | ProposalKind::Project | ProposalKind::Dri => {
            Err(IngestError::Rejected(format!(
                "invalid: execution of {} proposals is not implemented yet",
                serde_json::to_value(proposal.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default()
            )))
        }
        ProposalKind::Money => Err(IngestError::Rejected(
            "restricted: money not enabled".into(),
        )),
        ProposalKind::Join => Err(IngestError::Rejected("restricted: join not enabled".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pk(seed: u8) -> String {
        hex::encode([seed; 32])
    }

    fn proposal(needed: u32, eligible: usize, votes: &[(u8, VoteChoice)]) -> Proposal {
        Proposal {
            id: "prop".into(),
            kind: ProposalKind::Shapers,
            status: ProposalStatus::Open,
            opened_by: pk(1),
            opened_at: 0,
            expires_at: 1,
            draft: None,
            payload: serde_json::json!({}),
            rule: DecisionRule::MAJORITY,
            needed,
            eligible: (1..=eligible).map(|s| pk(s as u8)).collect(),
            votes: votes
                .iter()
                .map(|(p, vote)| Vote {
                    p: pk(*p),
                    vote: *vote,
                    at: 0,
                    receipt: pk(9),
                })
                .collect(),
            decided_at: None,
            executed: None,
            settlement: None,
        }
    }

    #[test]
    fn tally_passes_at_needed_rejects_when_passing_is_impossible_else_stays_open() {
        use VoteChoice::{Agree, Decline};
        // majority of 3 → needed 2
        assert_eq!(tally(&proposal(2, 3, &[])), None);
        assert_eq!(tally(&proposal(2, 3, &[(1, Agree)])), None);
        assert_eq!(
            tally(&proposal(2, 3, &[(1, Agree), (2, Agree)])),
            Some(ProposalStatus::Passed)
        );
        assert_eq!(tally(&proposal(2, 3, &[(1, Decline)])), None);
        assert_eq!(
            tally(&proposal(2, 3, &[(1, Decline), (2, Decline)])),
            Some(ProposalStatus::Rejected),
            "two declines of three leave only one possible agree"
        );
        // all of 2 → needed 2: one decline rejects
        assert_eq!(
            tally(&proposal(2, 2, &[(1, Decline)])),
            Some(ProposalStatus::Rejected)
        );
        assert_eq!(
            tally(&proposal(2, 2, &[(1, Agree), (2, Decline)])),
            Some(ProposalStatus::Rejected)
        );
        // one eligible → needed 1
        assert_eq!(
            tally(&proposal(1, 1, &[(1, Agree)])),
            Some(ProposalStatus::Passed)
        );
        assert_eq!(
            tally(&proposal(1, 1, &[(1, Decline)])),
            Some(ProposalStatus::Rejected)
        );
    }
}
