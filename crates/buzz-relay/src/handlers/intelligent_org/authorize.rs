//! Who may send which Shapers, proposal, and work-tree command (Protocol
//! §3.2, §4.5, §5.1, §5.2, §5.3, §6.4).
//!
//! Pure decisions over the live `39103`, `39102`, `39101`, and `39100`
//! content: no database, no clock other than the `now` the caller passes.
//! Facts that need a table — is the actor the owner, is a pubkey a member,
//! which siblings exist — arrive as values. A refusal carries the exact
//! wire text §3.2 fixes — `restricted: <reason>` when the author lacks the
//! role, `invalid: <reason>` when the target is in the wrong state — so the
//! handlers in [`super::shapers`], [`super::proposals`], and [`super::work`]
//! only route and write.

use buzz_core::intelligent_org::{
    DecisionRule, DirectionArtifact, DirectionProposeContent, DirectionSlug, OfferedSeat, Proposal,
    ProposalStatus, RulesContent, Shapers, ShapersOp, WorkItem, WorkItemState,
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

/// A command any NIP-43 member may send (`50004`, `50015`). Membership is
/// ingest's gate when `REQUIRE_RELAY_MEMBERSHIP` is on; this check covers
/// the same rule when it is off.
pub fn require_member(is_member: bool) -> Result<(), IngestError> {
    if is_member {
        Ok(())
    } else {
        Err(restricted("not a member"))
    }
}

/// `io_direction_propose` (§5.2): a Shaper, and `base` equals the live head
/// version (0 when the slug has never been written).
pub fn open_direction(
    shapers: &Shapers,
    actor: &str,
    base: u32,
    head_version: Option<u32>,
) -> Result<(), IngestError> {
    require_shaper(shapers, actor)?;
    stale_base(base, head_version)
}

/// `base` must equal the current head. Used at opening and again at
/// execution, so a later confirm of the same slug cannot silently overwrite.
pub fn stale_base(base: u32, head_version: Option<u32>) -> Result<(), IngestError> {
    if base == head_version.unwrap_or(0) {
        Ok(())
    } else {
        Err(invalid("stale base"))
    }
}

/// The content of a `direction` proposal (§4.1): `mission` and `vision`
/// have no `lines`.
pub fn direction_content(
    slug: DirectionSlug,
    content: &DirectionProposeContent,
) -> Result<(), IngestError> {
    let lined = matches!(slug, DirectionSlug::Objectives | DirectionSlug::Strategy);
    if !lined
        && content
            .lines
            .as_ref()
            .is_some_and(|lines| !lines.is_empty())
    {
        return Err(invalid("mission and vision have no lines"));
    }
    Ok(())
}

/// `io_dri_propose` (§5.3): the item exists and has no holder. `subject` is
/// the named `p` — left out of `eligible` by [`super::proposals::open`].
pub fn open_dri(item: &WorkItem) -> Result<(), IngestError> {
    if item.dri.is_some() {
        return Err(invalid("item already has a holder"));
    }
    Ok(())
}

/// Title and brief on a `project` / `ticket` command: both required.
pub fn work_copy(title: &str, brief: &str) -> Result<(), IngestError> {
    if title.trim().is_empty() {
        return Err(invalid("title is required"));
    }
    if brief.trim().is_empty() {
        return Err(invalid("brief is required"));
    }
    Ok(())
}

/// §5.1 rule 6: item content may not carry a money field.
pub fn money_fields(content: &serde_json::Value) -> Result<(), IngestError> {
    let Some(object) = content.as_object() else {
        return Ok(());
    };
    for key in ["amount", "budget", "pot", "currency"] {
        if object.contains_key(key) {
            return Err(invalid("money fields are not allowed"));
        }
    }
    Ok(())
}

/// An `objective_ref` must name a line on the live `39100` objectives head
/// (`objectives@<version>#<line-id>`).
pub fn objective_ref(value: &str, head: Option<&DirectionArtifact>) -> Result<(), IngestError> {
    let Some((version, line_id)) = parse_objective_ref(value) else {
        return Err(invalid("objective_ref not a live line"));
    };
    let Some(head) = head else {
        return Err(invalid("objective_ref not a live line"));
    };
    if head.slug != DirectionSlug::Objectives || head.version != version {
        return Err(invalid("objective_ref not a live line"));
    }
    if head.lines.iter().any(|line| line.id == line_id) {
        Ok(())
    } else {
        Err(invalid("objective_ref not a live line"))
    }
}

fn parse_objective_ref(value: &str) -> Option<(u32, &str)> {
    let rest = value.strip_prefix("objectives@")?;
    let (version, line) = rest.split_once('#')?;
    let version = version.parse().ok()?;
    if line.is_empty() {
        return None;
    }
    Some((version, line))
}

/// Whether `actor` holds `item` (`accepted` / `in_review` and `dri = actor`).
pub fn is_holder(item: &WorkItem, actor: &str) -> bool {
    matches!(
        item.state,
        WorkItemState::Accepted | WorkItemState::InReview
    ) && item.dri.as_deref() == Some(actor)
}

/// `io_ticket_create` (§5.1 rule 1): only the holder of the parent.
pub fn create_child(parent: &WorkItem, actor: &str) -> Result<(), IngestError> {
    if is_holder(parent, actor) {
        Ok(())
    } else {
        Err(restricted("not the holder"))
    }
}

/// `after` on `50005`: every id is a live or done sibling under `parent`.
pub fn after_siblings(
    after: &[String],
    parent: &str,
    siblings: &[WorkItem],
) -> Result<(), IngestError> {
    for id in after {
        let sibling = siblings.iter().find(|item| item.id == *id);
        let Some(sibling) = sibling else {
            return Err(invalid("after not a sibling"));
        };
        if sibling.parent.as_deref() != Some(parent) {
            return Err(invalid("after not a sibling"));
        }
        // Every §5.1 state is live or done; existence as a sibling is the check.
        let _ = sibling.state;
    }
    Ok(())
}

/// `io_offer` (§3.2): a Shaper for a root; the parent holder for a child.
/// The item must be `open`.
pub fn offer(
    item: &WorkItem,
    parent: Option<&WorkItem>,
    shapers: &Shapers,
    actor: &str,
) -> Result<(), IngestError> {
    if item.state != WorkItemState::Open {
        return Err(invalid("item is not open"));
    }
    if item.parent.is_none() {
        require_shaper(shapers, actor)
    } else {
        let parent = parent.ok_or_else(|| invalid("unknown parent"))?;
        if is_holder(parent, actor) {
            Ok(())
        } else {
            Err(restricted("not the holder"))
        }
    }
}

/// `io_accept` / `io_decline` (§5.1 rule 2): only `offered_to`, and only
/// while the item is `offered`.
pub fn accept_or_decline(item: &WorkItem, actor: &str) -> Result<(), IngestError> {
    if item.state != WorkItemState::Offered {
        return Err(invalid("item is not offered"));
    }
    if item.offered_to.as_deref() == Some(actor) {
        Ok(())
    } else {
        Err(restricted("not the offered person"))
    }
}

/// `io_vote` (§5.3): from a pubkey in the proposal's frozen `eligible` that
/// still holds a seat, while the proposal is `open` and before
/// `expires_at`. The subject of a `shapers/remove` or `dri` is not in
/// `eligible`, so the first check covers "the subject cannot vote".
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

    #[test]
    fn open_direction_needs_a_shaper_and_a_live_base() {
        use buzz_core::intelligent_org::DirectionProposeContent;
        let s = shapers(&[1], &[]);
        assert!(open_direction(&s, &pk(1), 0, None).is_ok());
        assert!(open_direction(&s, &pk(1), 2, Some(2)).is_ok());
        assert_eq!(
            message(open_direction(&s, &pk(2), 0, None)),
            "restricted: not a Shaper"
        );
        assert_eq!(
            message(open_direction(&s, &pk(1), 0, Some(1))),
            "invalid: stale base"
        );
        assert_eq!(
            message(open_direction(&s, &pk(1), 1, None)),
            "invalid: stale base"
        );
        let empty = DirectionProposeContent {
            body: "b".into(),
            lines: None,
            why: None,
        };
        assert!(direction_content(DirectionSlug::Mission, &empty).is_ok());
        let lined = DirectionProposeContent {
            body: "b".into(),
            lines: Some(vec![buzz_core::intelligent_org::DirectionLineInput {
                id: None,
                text: "a line".into(),
                date: None,
            }]),
            why: None,
        };
        assert_eq!(
            message(direction_content(DirectionSlug::Vision, &lined)),
            "invalid: mission and vision have no lines"
        );
        assert!(direction_content(DirectionSlug::Objectives, &lined).is_ok());
    }

    #[test]
    fn open_dri_needs_an_item_with_no_holder() {
        let mut item = WorkItem {
            id: "item".into(),
            parent: None,
            root: "item".into(),
            depth: 0,
            path: vec!["item".into()],
            title: "t".into(),
            brief: "b".into(),
            state: buzz_core::intelligent_org::WorkItemState::Open,
            dri: None,
            offered_to: None,
            offered_by: None,
            offered_at: None,
            due_at: 1,
            approved_at: None,
            objective_ref: None,
            created_from: pk(8),
            draft: None,
            done_receipt: None,
            closed_by: None,
            children: buzz_core::intelligent_org::ChildrenCounts::default(),
            home: None,
            branch: None,
            after: vec![],
            last_progress: None,
        };
        assert!(open_dri(&item).is_ok());
        item.offered_to = Some(pk(2));
        item.state = buzz_core::intelligent_org::WorkItemState::Offered;
        assert!(
            open_dri(&item).is_ok(),
            "an offer is not a holder; passing withdraws it"
        );
        item.dri = Some(pk(2));
        item.state = buzz_core::intelligent_org::WorkItemState::Accepted;
        assert_eq!(
            message(open_dri(&item)),
            "invalid: item already has a holder"
        );
        assert_eq!(message(require_member(false)), "restricted: not a member");
        assert!(require_member(true).is_ok());
    }

    fn item(state: WorkItemState, dri: Option<String>, parent: Option<&str>) -> WorkItem {
        WorkItem {
            id: "child".into(),
            parent: parent.map(str::to_owned),
            root: "root".into(),
            depth: if parent.is_some() { 1 } else { 0 },
            path: if parent.is_some() {
                vec!["root".into()]
            } else {
                vec![]
            },
            title: "t".into(),
            brief: "b".into(),
            state,
            dri,
            offered_to: None,
            offered_by: None,
            offered_at: None,
            due_at: 1,
            approved_at: None,
            objective_ref: None,
            created_from: pk(8),
            draft: None,
            done_receipt: None,
            closed_by: None,
            children: buzz_core::intelligent_org::ChildrenCounts::default(),
            home: None,
            branch: None,
            after: vec![],
            last_progress: None,
        }
    }

    #[test]
    fn money_fields_and_work_copy_and_objective_ref() {
        assert!(work_copy("t", "b").is_ok());
        assert_eq!(message(work_copy("  ", "b")), "invalid: title is required");
        assert_eq!(message(work_copy("t", "")), "invalid: brief is required");
        assert!(money_fields(&serde_json::json!({"title": "t"})).is_ok());
        for key in ["amount", "budget", "pot", "currency"] {
            assert_eq!(
                message(money_fields(&serde_json::json!({ key: "1" }))),
                "invalid: money fields are not allowed",
                "{key}"
            );
        }
        let head = DirectionArtifact {
            slug: DirectionSlug::Objectives,
            version: 3,
            body: "b".into(),
            lines: vec![buzz_core::intelligent_org::DirectionLine {
                n: 1,
                id: "l_7f3a".into(),
                text: "Weekday hall".into(),
                date: None,
            }],
            confirmed_by: pk(1),
            confirmed_at: 1,
            proposed_by: pk(1),
            proposal: "prop".into(),
            prev: None,
        };
        assert!(objective_ref("objectives@3#l_7f3a", Some(&head)).is_ok());
        assert_eq!(
            message(objective_ref("objectives@3#l_nope", Some(&head))),
            "invalid: objective_ref not a live line"
        );
        assert_eq!(
            message(objective_ref("objectives@2#l_7f3a", Some(&head))),
            "invalid: objective_ref not a live line"
        );
        assert_eq!(
            message(objective_ref("objectives@3#l_7f3a", None)),
            "invalid: objective_ref not a live line"
        );
        assert_eq!(
            message(objective_ref("not-a-ref", Some(&head))),
            "invalid: objective_ref not a live line"
        );
    }

    #[test]
    fn only_the_holder_creates_children_and_after_must_be_a_sibling() {
        let held = item(WorkItemState::Accepted, Some(pk(1)), None);
        assert!(create_child(&held, &pk(1)).is_ok());
        assert_eq!(
            message(create_child(&held, &pk(2))),
            "restricted: not the holder"
        );
        let open = item(WorkItemState::Open, None, None);
        assert_eq!(
            message(create_child(&open, &pk(1))),
            "restricted: not the holder"
        );

        let sibling = item(WorkItemState::Done, Some(pk(1)), Some("root"));
        let mut sibling = sibling;
        sibling.id = "sib".into();
        assert!(after_siblings(&["sib".into()], "root", &[sibling.clone()]).is_ok());
        assert_eq!(
            message(after_siblings(
                &["missing".into()],
                "root",
                &[sibling.clone()]
            )),
            "invalid: after not a sibling"
        );
        let mut other_parent = sibling;
        other_parent.parent = Some("other".into());
        assert_eq!(
            message(after_siblings(&["sib".into()], "root", &[other_parent])),
            "invalid: after not a sibling"
        );
    }

    #[test]
    fn offer_and_accept_follow_the_named_person_and_the_holder() {
        let two = shapers(&[1, 2], &[]);
        let mut root = item(WorkItemState::Open, None, None);
        root.id = "root".into();
        root.root = "root".into();
        assert!(offer(&root, None, &two, &pk(1)).is_ok());
        assert_eq!(
            message(offer(&root, None, &two, &pk(9))),
            "restricted: not a Shaper"
        );
        root.state = WorkItemState::Accepted;
        root.dri = Some(pk(1));
        assert_eq!(
            message(offer(&root, None, &two, &pk(1))),
            "invalid: item is not open"
        );

        let parent = item(WorkItemState::Accepted, Some(pk(1)), None);
        let child = item(WorkItemState::Open, None, Some("root"));
        assert!(offer(&child, Some(&parent), &two, &pk(1)).is_ok());
        assert_eq!(
            message(offer(&child, Some(&parent), &two, &pk(2))),
            "restricted: not the holder"
        );

        let mut offered = child;
        offered.state = WorkItemState::Offered;
        offered.offered_to = Some(pk(3));
        assert!(accept_or_decline(&offered, &pk(3)).is_ok());
        assert_eq!(
            message(accept_or_decline(&offered, &pk(1))),
            "restricted: not the offered person"
        );
        offered.state = WorkItemState::Open;
        assert_eq!(
            message(accept_or_decline(&offered, &pk(3))),
            "invalid: item is not offered"
        );
    }
}
