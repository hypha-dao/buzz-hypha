//! Who may send which Shapers and proposal command (Protocol §3.2, §4.5,
//! §5.3, §6.4).
//!
//! Pure decisions over the live `39103` and `39102` content: no database, no
//! clock other than the `now` the caller passes. Facts that need a table —
//! is the actor the owner, is a pubkey a member — arrive as booleans. A
//! refusal carries the exact wire text §3.2 fixes — `restricted: <reason>`
//! when the author lacks the role, `invalid: <reason>` when the target is
//! in the wrong state — so the handlers in [`super::shapers`] and
//! [`super::proposals`] only route and write.

use buzz_core::intelligent_org::{
    DecisionRule, OfferedSeat, Proposal, ProposalStatus, RulesContent, Shapers, ShapersOp,
};

use crate::handlers::ingest::IngestError;

fn restricted(reason: &str) -> IngestError {
    IngestError::Rejected(format!("restricted: {reason}"))
}

fn invalid(reason: &str) -> IngestError {
    IngestError::Rejected(format!("invalid: {reason}"))
}

/// Whether `actor` holds a live seat.
pub fn is_shaper(shapers: &Shapers, actor: &str) -> bool {
    shapers.shapers.iter().any(|p| p == actor)
}

/// A command only a Shaper may send.
pub fn require_shaper(shapers: &Shapers, actor: &str) -> Result<(), IngestError> {
    if is_shaper(shapers, actor) {
        Ok(())
    } else {
        Err(restricted("not a Shaper"))
    }
}

/// The one `io_shapers_propose` accepted while no `39103` exists (§6.4
/// Bootstrap): `op=add`, naming the sender, from the community owner.
pub fn bootstrap(
    op: ShapersOp,
    subject: Option<&str>,
    actor: &str,
    actor_is_owner: bool,
) -> Result<(), IngestError> {
    if op != ShapersOp::Add {
        return Err(invalid(
            "no Shapers yet; the community owner bootstraps with op=add naming themselves",
        ));
    }
    if subject != Some(actor) {
        return Err(invalid("bootstrap must name the owner themselves"));
    }
    if !actor_is_owner {
        return Err(restricted("only the community owner may bootstrap"));
    }
    Ok(())
}

/// Whether `p` holds a seat in `offered` that has not lapsed at `now`.
fn has_live_seat(shapers: &Shapers, p: &str, now: u64) -> bool {
    shapers
        .offered
        .iter()
        .any(|seat| seat.p == p && now < seat.at.saturating_add(shapers.offer_window_secs))
}

/// `io_shapers_propose` once a `39103` exists (§4.5): a Shaper opens it, and
/// the op must be able to act on its target. `subject_is_member` is the
/// NIP-43 membership of `subject`, consulted only for `op=agent`.
pub fn open_shapers(
    shapers: &Shapers,
    actor: &str,
    op: ShapersOp,
    subject: Option<&str>,
    subject_is_member: bool,
    now: u64,
) -> Result<(), IngestError> {
    require_shaper(shapers, actor)?;
    match (op, subject) {
        (ShapersOp::Add, Some(p)) => {
            if is_shaper(shapers, p) {
                return Err(invalid("already a Shaper"));
            }
            if has_live_seat(shapers, p, now) {
                return Err(invalid("a seat is already offered to p"));
            }
        }
        (ShapersOp::Remove, Some(p)) => {
            if !is_shaper(shapers, p) {
                return Err(invalid("p is not a Shaper"));
            }
            if shapers.shapers.len() == 1 {
                return Err(invalid("last shaper"));
            }
        }
        (ShapersOp::Add | ShapersOp::Remove, None) => {
            return Err(invalid("op=add and op=remove need a p tag"));
        }
        (ShapersOp::Rules, _) => {}
        (ShapersOp::Agent, Some(p)) => agent_candidate(shapers, p, subject_is_member)?,
        (ShapersOp::Agent, None) => {}
    }
    Ok(())
}

/// The `p` of a `shapers/agent` proposal: a NIP-43 member that is not a
/// Shaper (§4.5). Checked when the proposal opens and again when it
/// executes, since either can change while the vote is open.
pub fn agent_candidate(shapers: &Shapers, p: &str, is_member: bool) -> Result<(), IngestError> {
    if !is_member {
        return Err(invalid("agent not a member"));
    }
    if is_shaper(shapers, p) {
        return Err(invalid("agent is a shaper"));
    }
    Ok(())
}

/// The content of a `shapers/rules` proposal (§4.5): every integer rule is
/// `N ≥ 1` and every window given is positive.
pub fn rules_content(content: &RulesContent) -> Result<(), IngestError> {
    let rules = &content.rules;
    let named = [
        ("direction", rules.direction),
        ("project", rules.project),
        ("dri", rules.dri),
        ("shapers", rules.shapers),
        ("money", rules.money),
        ("join", rules.join),
    ];
    for (kind, rule) in named {
        if rule == DecisionRule::AtLeast(0) {
            return Err(invalid(&format!("rules.{kind} must be at least 1")));
        }
    }
    if content.decision_window_secs == Some(0) {
        return Err(invalid("decision_window_secs must be positive"));
    }
    if content.offer_window_secs == Some(0) {
        return Err(invalid("offer_window_secs must be positive"));
    }
    Ok(())
}

/// `io_vote` (§5.3): from a pubkey in the proposal's frozen `eligible` that
/// still holds a seat, while the proposal is `open` and before
/// `expires_at`. The subject of a `shapers/remove` is not in `eligible`, so
/// the first check covers "the subject cannot vote".
pub fn vote(
    proposal: &Proposal,
    shapers: &Shapers,
    actor: &str,
    now: u64,
) -> Result<(), IngestError> {
    if !proposal.eligible.iter().any(|p| p == actor) {
        return Err(restricted("not eligible to vote on this proposal"));
    }
    require_shaper(shapers, actor)?;
    if proposal.status != ProposalStatus::Open {
        return Err(invalid("proposal is not open"));
    }
    if now >= proposal.expires_at {
        return Err(invalid("proposal has expired"));
    }
    Ok(())
}

/// `io_shaper_step_down`: a Shaper, unless they are the last one.
pub fn step_down(shapers: &Shapers, actor: &str) -> Result<(), IngestError> {
    require_shaper(shapers, actor)?;
    if shapers.shapers.len() == 1 {
        return Err(restricted("the last Shaper cannot step down"));
    }
    Ok(())
}

/// `io_shaper_accept`: the `p` of a passed `shapers/add` whose seat is still
/// offered. Returns the seat being taken.
pub fn accept_seat<'a>(
    shapers: &'a Shapers,
    actor: &str,
    proposal: &str,
    now: u64,
) -> Result<&'a OfferedSeat, IngestError> {
    if is_shaper(shapers, actor) {
        return Err(invalid("already a Shaper"));
    }
    let seat = shapers
        .offered
        .iter()
        .find(|seat| seat.p == actor && seat.proposal == proposal)
        .ok_or_else(|| restricted("no seat is offered to you on that proposal"))?;
    if now >= seat.at.saturating_add(shapers.offer_window_secs) {
        return Err(invalid("the offer has lapsed"));
    }
    Ok(seat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::intelligent_org::{
        DecisionRules, DEFAULT_DECISION_WINDOW_SECS, DEFAULT_OFFER_WINDOW_SECS,
    };

    fn pk(seed: u8) -> String {
        hex::encode([seed; 32])
    }

    fn shapers(seats: &[u8], offered: &[(u8, &str, u64)]) -> Shapers {
        Shapers {
            founder: pk(1),
            shapers: seats.iter().map(|s| pk(*s)).collect(),
            offered: offered
                .iter()
                .map(|(p, proposal, at)| OfferedSeat {
                    p: pk(*p),
                    proposal: (*proposal).to_owned(),
                    at: *at,
                })
                .collect(),
            room: None,
            agent: None,
            agent_hosted: false,
            rules: DecisionRules::default(),
            decision_window_secs: DEFAULT_DECISION_WINDOW_SECS,
            offer_window_secs: DEFAULT_OFFER_WINDOW_SECS,
            updated_at: 0,
            receipt: pk(9),
        }
    }

    fn message(result: Result<impl std::fmt::Debug, IngestError>) -> String {
        match result {
            Err(IngestError::Rejected(m)) => m,
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_needs_add_naming_the_owner_from_the_owner() {
        assert!(bootstrap(ShapersOp::Add, Some(&pk(1)), &pk(1), true).is_ok());
        assert_eq!(
            message(bootstrap(ShapersOp::Remove, Some(&pk(1)), &pk(1), true)),
            "invalid: no Shapers yet; the community owner bootstraps with op=add naming themselves"
        );
        assert_eq!(
            message(bootstrap(ShapersOp::Add, Some(&pk(2)), &pk(1), true)),
            "invalid: bootstrap must name the owner themselves"
        );
        assert_eq!(
            message(bootstrap(ShapersOp::Add, None, &pk(1), true)),
            "invalid: bootstrap must name the owner themselves"
        );
        assert_eq!(
            message(bootstrap(ShapersOp::Add, Some(&pk(1)), &pk(1), false)),
            "restricted: only the community owner may bootstrap"
        );
    }

    #[test]
    fn step_down_needs_a_seat_and_another_shaper() {
        let two = shapers(&[1, 2], &[]);
        assert!(step_down(&two, &pk(1)).is_ok());
        assert_eq!(message(step_down(&two, &pk(3))), "restricted: not a Shaper");
        let one = shapers(&[1], &[]);
        assert_eq!(
            message(step_down(&one, &pk(1))),
            "restricted: the last Shaper cannot step down"
        );
    }

    #[test]
    fn accept_takes_only_a_live_seat_offered_to_the_sender() {
        let s = shapers(&[1], &[(2, "prop-a", 1_000)]);
        let seat = accept_seat(&s, &pk(2), "prop-a", 1_500).expect("seat");
        assert_eq!(seat.proposal, "prop-a");
        assert_eq!(
            message(accept_seat(&s, &pk(2), "prop-b", 1_500)),
            "restricted: no seat is offered to you on that proposal"
        );
        assert_eq!(
            message(accept_seat(&s, &pk(3), "prop-a", 1_500)),
            "restricted: no seat is offered to you on that proposal"
        );
        assert_eq!(
            message(accept_seat(&s, &pk(1), "prop-a", 1_500)),
            "invalid: already a Shaper"
        );
        assert_eq!(
            message(accept_seat(
                &s,
                &pk(2),
                "prop-a",
                1_000 + DEFAULT_OFFER_WINDOW_SECS
            )),
            "invalid: the offer has lapsed"
        );
        assert!(accept_seat(&s, &pk(2), "prop-a", 999 + DEFAULT_OFFER_WINDOW_SECS).is_ok());
    }

    #[test]
    fn open_shapers_needs_a_shaper_and_a_target_the_op_can_act_on() {
        let two = shapers(&[1, 2], &[(3, "prop-a", 1_000)]);
        assert_eq!(
            message(open_shapers(
                &two,
                &pk(9),
                ShapersOp::Add,
                Some(&pk(4)),
                false,
                0
            )),
            "restricted: not a Shaper"
        );

        // add
        assert!(open_shapers(&two, &pk(1), ShapersOp::Add, Some(&pk(4)), false, 0).is_ok());
        assert_eq!(
            message(open_shapers(
                &two,
                &pk(1),
                ShapersOp::Add,
                Some(&pk(2)),
                false,
                0
            )),
            "invalid: already a Shaper"
        );
        assert_eq!(
            message(open_shapers(
                &two,
                &pk(1),
                ShapersOp::Add,
                Some(&pk(3)),
                false,
                1_500
            )),
            "invalid: a seat is already offered to p"
        );
        assert!(
            open_shapers(
                &two,
                &pk(1),
                ShapersOp::Add,
                Some(&pk(3)),
                false,
                1_000 + DEFAULT_OFFER_WINDOW_SECS
            )
            .is_ok(),
            "a lapsed seat does not block a new offer"
        );
        assert_eq!(
            message(open_shapers(&two, &pk(1), ShapersOp::Add, None, false, 0)),
            "invalid: op=add and op=remove need a p tag"
        );

        // remove
        assert!(open_shapers(&two, &pk(1), ShapersOp::Remove, Some(&pk(2)), false, 0).is_ok());
        assert!(
            open_shapers(&two, &pk(1), ShapersOp::Remove, Some(&pk(1)), false, 0).is_ok(),
            "a Shaper may propose their own removal; they are just not eligible"
        );
        assert_eq!(
            message(open_shapers(
                &two,
                &pk(1),
                ShapersOp::Remove,
                Some(&pk(4)),
                false,
                0
            )),
            "invalid: p is not a Shaper"
        );
        let one = shapers(&[1], &[]);
        assert_eq!(
            message(open_shapers(
                &one,
                &pk(1),
                ShapersOp::Remove,
                Some(&pk(1)),
                false,
                0
            )),
            "invalid: last shaper"
        );

        // rules and agent
        assert!(open_shapers(&two, &pk(1), ShapersOp::Rules, None, false, 0).is_ok());
        assert!(open_shapers(&two, &pk(1), ShapersOp::Agent, None, false, 0).is_ok());
        assert!(open_shapers(&two, &pk(1), ShapersOp::Agent, Some(&pk(5)), true, 0).is_ok());
        assert_eq!(
            message(open_shapers(
                &two,
                &pk(1),
                ShapersOp::Agent,
                Some(&pk(5)),
                false,
                0
            )),
            "invalid: agent not a member"
        );
        assert_eq!(
            message(open_shapers(
                &two,
                &pk(1),
                ShapersOp::Agent,
                Some(&pk(2)),
                true,
                0
            )),
            "invalid: agent is a shaper"
        );
    }

    #[test]
    fn rules_content_rejects_zero_thresholds_and_zero_windows() {
        let mut content = RulesContent {
            rules: DecisionRules::default(),
            decision_window_secs: None,
            offer_window_secs: None,
        };
        assert!(rules_content(&content).is_ok());
        content.rules.dri = DecisionRule::AtLeast(1);
        content.rules.shapers = DecisionRule::ALL;
        content.decision_window_secs = Some(60);
        assert!(rules_content(&content).is_ok());
        content.rules.dri = DecisionRule::AtLeast(0);
        assert_eq!(
            message(rules_content(&content)),
            "invalid: rules.dri must be at least 1"
        );
        content.rules.dri = DecisionRule::MAJORITY;
        content.decision_window_secs = Some(0);
        assert_eq!(
            message(rules_content(&content)),
            "invalid: decision_window_secs must be positive"
        );
        content.decision_window_secs = None;
        content.offer_window_secs = Some(0);
        assert_eq!(
            message(rules_content(&content)),
            "invalid: offer_window_secs must be positive"
        );
    }

    #[test]
    fn vote_needs_a_frozen_eligible_seat_holder_on_an_open_live_proposal() {
        use buzz_core::intelligent_org::ProposalKind;
        let s = shapers(&[1, 2], &[]);
        let mut p = Proposal {
            id: "prop".into(),
            kind: ProposalKind::Shapers,
            status: ProposalStatus::Open,
            opened_by: pk(1),
            opened_at: 1_000,
            expires_at: 2_000,
            draft: None,
            payload: serde_json::json!({}),
            rule: DecisionRule::MAJORITY,
            needed: 1,
            eligible: vec![pk(1), pk(3)],
            votes: vec![],
            decided_at: None,
            executed: None,
            settlement: None,
        };
        assert!(vote(&p, &s, &pk(1), 1_500).is_ok());
        assert_eq!(
            message(vote(&p, &s, &pk(2), 1_500)),
            "restricted: not eligible to vote on this proposal",
            "a Shaper added after opening is not in the frozen set"
        );
        assert_eq!(
            message(vote(&p, &s, &pk(3), 1_500)),
            "restricted: not a Shaper",
            "eligible at opening but no longer seated"
        );
        assert_eq!(
            message(vote(&p, &s, &pk(1), 2_000)),
            "invalid: proposal has expired"
        );
        p.status = ProposalStatus::Passed;
        assert_eq!(
            message(vote(&p, &s, &pk(1), 1_500)),
            "invalid: proposal is not open"
        );
    }
}
