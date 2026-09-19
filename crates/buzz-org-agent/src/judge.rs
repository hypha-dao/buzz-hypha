//! The 15 hard gates (Org agent § 9.1). Pure, cheapest first.

use buzz_core::intelligent_org::{
    DraftKind, DraftOutcomeStatus, DraftPayload, HealthBand, LineOp, Served, WorkItemState,
};
use serde_json::Value;

use crate::state::OrgState;
use crate::think::context::ContextBundle;

/// A draft (or health read) the judge sees.
#[derive(Debug, Clone)]
pub struct Draft {
    /// Move kind. `None` for a `50101` health read.
    pub kind: Option<DraftKind>,
    /// Typed payload, when a card.
    pub payload: Option<DraftPayload>,
    /// Health read, when judging move 4.
    pub health: Option<buzz_core::intelligent_org::HealthRead>,
    /// Raw JSON of the payload (schema + forbidden-field).
    pub raw: Value,
    /// `n` tag: pubkey or `shaper`.
    pub needs: String,
    /// Dedupe key.
    pub gap: String,
    /// Cited receipts.
    pub receipts: Vec<String>,
    /// Generation the job snapped.
    pub generation: String,
    /// Parent / item the dates and sequence gates need.
    pub parent: Option<String>,
    /// Item a `done` draft names.
    pub item: Option<String>,
}

/// Why the judge refused. The first failure wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgeReason {
    /// Gate 1.
    Schema,
    /// Gate 2 — id not in `bundle.ids`.
    ReceiptOutsideContext,
    /// Gate 2 — id in context but does not resolve.
    UnresolvedReceipt,
    /// Gate 3.
    Authority,
    /// Gate 4.
    Date,
    /// Gate 5.
    Duplicate,
    /// Gate 6.
    Nag,
    /// Gate 7.
    SelfContradiction,
    /// Gate 8 — suggested holder was not a candidate.
    NotCandidate,
    /// Gate 8 — a matched skill is not on their `39105`.
    UnmatchedSkill,
    /// Gate 8 — they are at `open_limit`.
    HolderAtLimit,
    /// Gate 9.
    ForbiddenField,
    /// Gate 10.
    OpenChildren,
    /// Gate 11.
    HealthGrounding,
    /// Gate 12.
    RedrawShape,
    /// Gate 13.
    Stale,
    /// Gate 14.
    TooLong,
    /// Gate 15.
    Sequence,
    /// Gate 15 — `requires` set, no holder, no `unfilled`.
    Unfilled,
}

impl JudgeReason {
    /// Wire reason code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::ReceiptOutsideContext => "receipt_outside_context",
            Self::UnresolvedReceipt => "unresolved_receipt",
            Self::Authority => "authority",
            Self::Date => "date",
            Self::Duplicate => "duplicate",
            Self::Nag => "nag",
            Self::SelfContradiction => "self_contradiction",
            Self::NotCandidate => "not_candidate",
            Self::UnmatchedSkill => "unmatched_skill",
            Self::HolderAtLimit => "holder_at_limit",
            Self::ForbiddenField => "forbidden_field",
            Self::OpenChildren => "open_children",
            Self::HealthGrounding => "health_grounding",
            Self::RedrawShape => "redraw_shape",
            Self::Stale => "stale",
            Self::TooLong => "too_long",
            Self::Sequence => "sequence",
            Self::Unfilled => "unfilled",
        }
    }
}

/// Run the gates in order. The first failure is the reason.
pub fn judge(state: &OrgState, bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    gate_schema(draft)?;
    gate_receipts(bundle, draft)?;
    gate_authority(state, draft)?;
    gate_date(state, bundle, draft)?;
    gate_duplicate(state, draft)?;
    gate_nag(bundle, draft)?;
    gate_self_contradiction(draft)?;
    gate_holder(state, bundle, draft)?;
    gate_forbidden_field(draft)?;
    gate_open_children(state, draft)?;
    gate_health(bundle, draft)?;
    gate_redraw(state, bundle, draft)?;
    gate_stale(bundle, draft)?;
    gate_too_long(draft)?;
    gate_sequence(state, bundle, draft)?;
    Ok(())
}

fn gate_schema(draft: &Draft) -> Result<(), JudgeReason> {
    if let Some(kind) = draft.kind {
        let raw = draft.raw.to_string();
        DraftPayload::parse(kind, &raw).map_err(|_| JudgeReason::Schema)?;
    } else if draft.health.is_none() {
        return Err(JudgeReason::Schema);
    }
    if let Some(h) = &draft.health {
        if !(0.0..=1.0).contains(&h.pct) || h.week.is_empty() || h.item.is_empty() {
            return Err(JudgeReason::Schema);
        }
    }
    Ok(())
}

fn gate_receipts(bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    for id in &draft.receipts {
        if !bundle.ids.contains(id) {
            return Err(JudgeReason::ReceiptOutsideContext);
        }
        if !bundle.resolved.contains(id) {
            return Err(JudgeReason::UnresolvedReceipt);
        }
    }
    Ok(())
}

fn gate_authority(state: &OrgState, draft: &Draft) -> Result<(), JudgeReason> {
    let Some(kind) = draft.kind else {
        return Ok(());
    };
    let want = match kind {
        DraftKind::Project
        | DraftKind::Objectives
        | DraftKind::Direction
        | DraftKind::Review
        | DraftKind::Dri => "shaper",
        DraftKind::Ticket => draft
            .parent
            .as_ref()
            .and_then(|p| state.items.get(p))
            .and_then(|i| i.dri.as_deref())
            .unwrap_or(""),
        DraftKind::Done => draft
            .item
            .as_ref()
            .and_then(|i| state.items.get(i))
            .and_then(|i| i.dri.as_deref())
            .unwrap_or(""),
        DraftKind::Profile => draft
            .payload
            .as_ref()
            .and_then(|p| match p {
                DraftPayload::Profile(d) => Some(d.pubkey.as_str()),
                _ => None,
            })
            .unwrap_or(""),
        DraftKind::Money => draft.needs.as_str(),
    };
    if kind == DraftKind::Project && draft.needs != "shaper" && !draft.needs.is_empty() {
        // A project from a member's DM needs that member — allowed.
        return Ok(());
    }
    if draft.needs != want && want != "shaper" {
        return Err(JudgeReason::Authority);
    }
    if want == "shaper" && draft.needs != "shaper" {
        // One-Shaper communities may pin the sole Shaper as `needs`.
        if !state
            .shapers
            .as_ref()
            .is_some_and(|s| s.shapers.len() == 1 && s.shapers[0] == draft.needs)
        {
            return Err(JudgeReason::Authority);
        }
    }
    Ok(())
}

fn gate_date(state: &OrgState, bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    let Some(payload) = &draft.payload else {
        return Ok(());
    };
    let due = match payload {
        DraftPayload::Project(d) => Some(d.due_at),
        DraftPayload::Ticket(d) => Some(d.due_at),
        _ => None,
    };
    let Some(due) = due else {
        return Ok(());
    };
    if due < bundle.now {
        return Err(JudgeReason::Date);
    }
    if let DraftPayload::Ticket(d) = payload {
        if let Some(parent) = state.items.get(&d.parent) {
            if due > parent.due_at {
                return Err(JudgeReason::Date);
            }
        }
    }
    Ok(())
}

fn gate_duplicate(state: &OrgState, draft: &Draft) -> Result<(), JudgeReason> {
    if draft.gap.is_empty() {
        return Ok(());
    }
    let open = state.drafts.values().any(|d| {
        d.gap == draft.gap
            && d.outcome
                .as_ref()
                .is_none_or(|o| o.status == DraftOutcomeStatus::Open)
    });
    if open {
        return Err(JudgeReason::Duplicate);
    }
    Ok(())
}

fn gate_nag(bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    if draft.gap.is_empty() {
        return Ok(());
    }
    match (&bundle.fingerprint, &bundle.declined_fingerprint) {
        (Some(now), Some(was)) if now == was => Err(JudgeReason::Nag),
        _ => Ok(()),
    }
}

fn gate_self_contradiction(draft: &Draft) -> Result<(), JudgeReason> {
    let Some(payload) = &draft.payload else {
        return Ok(());
    };
    match payload {
        DraftPayload::Project(d) => {
            if let Some(target) = &d.objective_ref {
                if d.gaps
                    .iter()
                    .any(|g| g.line_ref == *target && g.served == Served::Served)
                {
                    return Err(JudgeReason::SelfContradiction);
                }
            }
        }
        DraftPayload::Ticket(d)
            if d.coverage
                .iter()
                .any(|c| c.piece == d.covers && c.covered_by.is_some()) =>
        {
            return Err(JudgeReason::SelfContradiction);
        }
        _ => {}
    }
    Ok(())
}

fn gate_holder(state: &OrgState, bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    let Some(payload) = &draft.payload else {
        return Ok(());
    };
    let (suggested, matched) = match payload {
        DraftPayload::Ticket(d) => (d.suggested_holder.as_deref(), d.matched.as_ref()),
        DraftPayload::Project(d) => (d.suggested_dri.as_deref(), d.matched.as_ref()),
        DraftPayload::Dri(d) => (Some(d.suggested.as_str()), Some(&d.matched)),
        _ => return Ok(()),
    };
    let Some(pk) = suggested else {
        return Ok(());
    };
    let Some(cand) = bundle.candidates.iter().find(|c| c.pubkey == pk) else {
        return Err(JudgeReason::NotCandidate);
    };
    if !cand.is_member && !state.members.contains(pk) {
        return Err(JudgeReason::NotCandidate);
    }
    if let Some(m) = matched {
        let profile = state.profiles.get(pk);
        for skill in &m.skills {
            let on_profile = profile.is_some_and(|p| p.skills.iter().any(|s| s.slug == *skill));
            let on_candidate = cand.skills.iter().any(|s| s == skill);
            if !on_profile && !on_candidate {
                return Err(JudgeReason::UnmatchedSkill);
            }
        }
        for item in &m.items {
            if !cand.items.iter().any(|i| i == item) {
                return Err(JudgeReason::UnmatchedSkill);
            }
        }
    }
    if cand.open_limit.is_some_and(|l| cand.open_count >= l) {
        return Err(JudgeReason::HolderAtLimit);
    }
    Ok(())
}

fn gate_forbidden_field(draft: &Draft) -> Result<(), JudgeReason> {
    if matches!(
        draft.kind,
        Some(DraftKind::Ticket | DraftKind::Project | DraftKind::Done | DraftKind::Review)
    ) {
        if let Some(obj) = draft.raw.as_object() {
            for key in ["dri", "state", "amount", "budget", "pot", "currency"] {
                if obj.contains_key(key) {
                    return Err(JudgeReason::ForbiddenField);
                }
            }
        }
    }
    Ok(())
}

fn gate_open_children(state: &OrgState, draft: &Draft) -> Result<(), JudgeReason> {
    if draft.kind != Some(DraftKind::Done) {
        return Ok(());
    }
    let Some(item) = draft.item.as_deref() else {
        return Ok(());
    };
    let open = state.items.values().any(|i| {
        i.parent.as_deref() == Some(item)
            && matches!(
                i.state,
                WorkItemState::Open
                    | WorkItemState::Offered
                    | WorkItemState::Accepted
                    | WorkItemState::InReview
            )
    });
    if open {
        return Err(JudgeReason::OpenChildren);
    }
    Ok(())
}

fn gate_health(bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    let Some(h) = &draft.health else {
        return Ok(());
    };
    if let Some(want) = bundle.expected_band {
        if h.band != want {
            return Err(JudgeReason::HealthGrounding);
        }
    }
    if h.sentences.iter().any(|s| s.rows.is_empty()) {
        return Err(JudgeReason::HealthGrounding);
    }
    for s in &h.sentences {
        if !numerals_in_factors(&s.text, &bundle.factor_values) {
            return Err(JudgeReason::HealthGrounding);
        }
    }
    let _ = HealthBand::Healthy;
    Ok(())
}

fn numerals_in_factors(text: &str, factors: &[String]) -> bool {
    let nums: Vec<&str> = text
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_ascii_digit()))
        .collect();
    nums.iter().all(|n| factors.iter().any(|f| f.contains(n)))
}

fn gate_redraw(state: &OrgState, bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    let Some(DraftPayload::Objectives(d)) = &draft.payload else {
        return Ok(());
    };
    let Some(head) = state.direction.get("objectives") else {
        return Err(JudgeReason::RedrawShape);
    };
    if d.base_version != head.artifact.version {
        return Err(JudgeReason::RedrawShape);
    }
    let ids: Vec<&str> = head.artifact.lines.iter().map(|l| l.id.as_str()).collect();
    for op in &d.ops {
        match op {
            LineOp::Strike { id, .. } | LineOp::Move { id, .. } => {
                if !ids.contains(&id.as_str()) {
                    return Err(JudgeReason::RedrawShape);
                }
            }
            LineOp::Add { source, .. } => {
                if let Some(src) = source {
                    if !bundle.ids.contains(src) {
                        return Err(JudgeReason::RedrawShape);
                    }
                }
            }
        }
    }
    Ok(())
}

fn gate_stale(bundle: &ContextBundle, draft: &Draft) -> Result<(), JudgeReason> {
    if !bundle.generation.is_empty() && bundle.generation != draft.generation {
        return Err(JudgeReason::Stale);
    }
    Ok(())
}

fn gate_too_long(draft: &Draft) -> Result<(), JudgeReason> {
    let Some(payload) = &draft.payload else {
        return Ok(());
    };
    let (brief, why, cap) = match payload {
        DraftPayload::Project(d) => (d.brief.as_str(), d.why.as_str(), 60usize),
        DraftPayload::Ticket(d) => (d.brief.as_str(), "", 40),
        DraftPayload::Dri(d) => ("", d.why.as_str(), 40),
        DraftPayload::Done(d) => ("", d.why.as_str(), 40),
        _ => return Ok(()),
    };
    if word_count(brief) > cap {
        return Err(JudgeReason::TooLong);
    }
    if why.contains('\n') {
        return Err(JudgeReason::TooLong);
    }
    Ok(())
}

fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

fn gate_sequence(
    state: &OrgState,
    bundle: &ContextBundle,
    draft: &Draft,
) -> Result<(), JudgeReason> {
    let Some(DraftPayload::Ticket(d)) = &draft.payload else {
        return Ok(());
    };
    if !d.requires.is_empty() && d.suggested_holder.is_none() && d.unfilled.is_none() {
        return Err(JudgeReason::Unfilled);
    }
    if d.coverage
        .iter()
        .any(|c| c.piece == d.covers && c.held.is_some())
    {
        return Err(JudgeReason::Sequence);
    }
    for after in &d.after {
        let Some(sib) = state.items.get(after) else {
            return Err(JudgeReason::Sequence);
        };
        if sib.parent.as_deref() != Some(d.parent.as_str()) {
            return Err(JudgeReason::Sequence);
        }
        if !matches!(
            sib.state,
            WorkItemState::Accepted | WorkItemState::InReview | WorkItemState::Done
        ) {
            return Err(JudgeReason::Sequence);
        }
    }
    if d.gate {
        let unheld_dependant = d.coverage.iter().any(|c| {
            c.after.iter().any(|a| a == &d.covers)
                && c.held.is_none()
                && c.covered_by.is_none()
                && bundle.batch.iter().any(|b| b == &c.piece)
        });
        if unheld_dependant {
            return Err(JudgeReason::Sequence);
        }
    }
    Ok(())
}
