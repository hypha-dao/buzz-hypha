//! One change plan per uncovered objective.
//!
//! The compiler decides the verdict. This module asks the model for a plan,
//! then judges it without another model call. Nothing is published unless
//! [`move_1_enabled`] is on (`IO_MOVE_1_ENABLED`).

use buzz_core::intelligent_org::{
    CoveragePiece, DraftKind, DraftOutcomeStatus, PlanChange, PlanStep, ProjectDraft, ProposalKind,
    StrategyLineType, TicketDraft, WorkItemState,
};
use nostr::Tag;

use crate::compile::{self, GapVerdict, ObjectiveCard};
use crate::jobs::JobLog;
use crate::pipeline::OrgAgent;
use crate::relay::link::RelayIo;
use crate::relay::publish::{sign, Permitted, PublishError};
use crate::state::OrgState;
use crate::think::model::{ModelClient, ModelRequest, Tier};

/// `IO_MOVE_1_ENABLED=1` publishes project drafts. Off, the job still
/// compiles and the model is not called.
pub fn move_1_enabled() -> bool {
    matches!(
        std::env::var("IO_MOVE_1_ENABLED").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

/// Why a change plan was dropped. The compiler did not call a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanReject {
    /// `gap.verdict` is not the compiler's verdict.
    VerdictMismatch,
    /// From, to, done when, or the option list is missing.
    ChangeShape,
    /// A `done_when` line is not produced by any step.
    PlanIncomplete,
    /// A step rests on the situation's open question and the plan does not
    /// start with a gate.
    Ungated,
    /// More than seven steps.
    Size,
    /// A kept option or a step repeats a strategy refusal.
    Refusal,
    /// A numeral in the prose is not on the card.
    UngroundedNumber,
}

const MAX_PLANS_PER_SCAN: usize = 4;
const MAX_STEPS: usize = 7;

/// Judge `raw` against the compiler card. No model call.
pub fn judge_change_plan(
    state: &OrgState,
    card: &ObjectiveCard,
    raw: &serde_json::Value,
) -> Result<ProjectDraft, PlanReject> {
    let draft: ProjectDraft =
        serde_json::from_value(raw.clone()).map_err(|_| PlanReject::ChangeShape)?;
    let line_ref = format!("objectives@{}#{}", card.version, card.line_id);
    let verdict = verdict_name(card.verdict);
    let gap = draft.gap.as_ref().ok_or(PlanReject::ChangeShape)?;
    if gap.line_ref != line_ref || gap.verdict != verdict {
        return Err(PlanReject::VerdictMismatch);
    }
    if draft.options.len() < 2 || draft.options.len() > 3 {
        return Err(PlanReject::ChangeShape);
    }
    if !draft.options.iter().any(|option| option.kept) {
        return Err(PlanReject::ChangeShape);
    }
    let change = draft.change.as_ref().ok_or(PlanReject::ChangeShape)?;
    if change.from.trim().is_empty()
        || change.to.trim().is_empty()
        || change.done_when.is_empty()
        || change.done_when.iter().any(|line| line.trim().is_empty())
        || !change.moves.iter().any(|m| m == &line_ref)
    {
        return Err(PlanReject::ChangeShape);
    }
    if draft.plan.is_empty() || draft.plan.len() > MAX_STEPS {
        return Err(if draft.plan.len() > MAX_STEPS {
            PlanReject::Size
        } else {
            PlanReject::PlanIncomplete
        });
    }
    if !plan_covers(change, &draft.plan) {
        return Err(PlanReject::PlanIncomplete);
    }
    if situation_has_open_question(state) && !draft.plan[0].gate {
        return Err(PlanReject::Ungated);
    }
    if kept_or_piece_matches_refusal(state, &draft) {
        return Err(PlanReject::Refusal);
    }
    if ungrounded_number(state, &draft, change) {
        return Err(PlanReject::UngroundedNumber);
    }
    Ok(draft)
}

fn verdict_name(verdict: GapVerdict) -> &'static str {
    match verdict {
        GapVerdict::Uncovered => "uncovered",
        GapVerdict::Partly => "partly",
        GapVerdict::Covered => "covered",
    }
}

fn plan_covers(change: &PlanChange, plan: &[PlanStep]) -> bool {
    change.done_when.iter().all(|line| {
        let want = line.trim().to_lowercase();
        plan.iter().any(|step| {
            step.produces
                .iter()
                .any(|produced| produced.trim().to_lowercase() == want)
                || step.piece.trim().to_lowercase() == want
        })
    })
}

fn situation_has_open_question(state: &OrgState) -> bool {
    let Some(head) = state.direction.get("situation") else {
        return false;
    };
    let body = head.artifact.body.to_lowercase();
    body.contains('?') || body.contains("whether ") || body.contains("must learn")
}

fn kept_or_piece_matches_refusal(state: &OrgState, draft: &ProjectDraft) -> bool {
    let terms = refusal_terms(state);
    if terms.is_empty() {
        return false;
    }
    let hits = |text: &str| {
        let lower = text.to_lowercase();
        terms.iter().any(|term| {
            lower
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|w| w == term)
        })
    };
    draft
        .options
        .iter()
        .any(|option| option.kept && (hits(&option.title) || hits(&option.mechanism)))
        || draft.plan.iter().any(|step| hits(&step.piece))
}

fn refusal_terms(state: &OrgState) -> Vec<String> {
    let Some(head) = state.direction.get("strategy") else {
        return Vec::new();
    };
    let mut terms = Vec::new();
    for line in &head.artifact.lines {
        if line.line_type != Some(StrategyLineType::Refusal) {
            continue;
        }
        let words: Vec<&str> = line
            .text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        let mut index = 0;
        while index < words.len() {
            let word = words[index].to_lowercase();
            if word == "not" || word == "no" {
                index += 1;
                while index < words.len()
                    && matches!(words[index].to_lowercase().as_str(), "a" | "an" | "the")
                {
                    index += 1;
                }
                if index < words.len() {
                    let term = words[index].to_lowercase();
                    if term.len() >= 4 {
                        terms.push(term);
                    }
                }
            }
            index += 1;
        }
    }
    terms
}

fn ungrounded_number(state: &OrgState, draft: &ProjectDraft, change: &PlanChange) -> bool {
    let card = card_numerals(state);
    let mut prose = vec![
        draft.why.clone(),
        draft.brief.clone(),
        change.from.clone(),
        change.to.clone(),
    ];
    prose.extend(change.done_when.clone());
    for option in &draft.options {
        prose.push(option.title.clone());
        prose.push(option.mechanism.clone());
    }
    numerals_in(&prose.join(" "))
        .into_iter()
        .any(|number| !card.contains(&number))
}

fn card_numerals(state: &OrgState) -> Vec<String> {
    let mut prose = String::new();
    for head in state.direction.values() {
        prose.push_str(&head.artifact.body);
        prose.push(' ');
        for line in &head.artifact.lines {
            prose.push_str(&line.text);
            prose.push(' ');
            if let Some(done) = &line.done_when {
                prose.push_str(done);
                prose.push(' ');
            }
            if let Some(date) = line.date {
                prose.push_str(&date.to_string());
                prose.push(' ');
            }
        }
    }
    numerals_in(&prose)
}

fn numerals_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn gap_is_open(state: &OrgState, gap: &str) -> bool {
    state.drafts.values().any(|draft| {
        draft.gap == gap
            && draft.kind == DraftKind::Project
            && match &draft.outcome {
                None => true,
                Some(outcome) => matches!(
                    outcome.status,
                    DraftOutcomeStatus::Open | DraftOutcomeStatus::Amended
                ),
            }
    })
}

/// The structured call for one uncovered objective.
pub fn plan_request(state: &OrgState, card: &ObjectiveCard) -> ModelRequest {
    let line_ref = format!("objectives@{}#{}", card.version, card.line_id);
    let snapshot = compile::compile(state);
    ModelRequest {
        tier: Tier::Draft,
        system: format!(
            "Write one change plan for {line_ref}. Verdict is {}. \
             First steps are gates when the situation has an open question. \
             Drop any option that repeats a strategy refusal. \
             Do not invent numbers.",
            verdict_name(card.verdict)
        ),
        messages: vec![crate::think::model::Message {
            role: "user".into(),
            content: snapshot.org.direction,
        }],
        schema: serde_json::json!({ "type": "object" }),
        tool_name: "emit_project".into(),
        tools: vec![],
        temperature: 0.2,
        max_output_tokens: 1200,
    }
}

impl<M: ModelClient, R: RelayIo> OrgAgent<M, R> {
    /// Finish waiting verdict jobs. A fresh objectives or strategy job
    /// drafts uncovered lines when move 1 is on.
    pub async fn run_jobs(&mut self) -> Result<usize, crate::error::AgentError> {
        let mut published = 0;
        while self.jobs.start() {
            let fresh = self.jobs.finish(&self.state).is_some();
            let key = self.jobs.log().last().and_then(|entry| match entry {
                JobLog::Verdict { key, .. } => Some(key.clone()),
                JobLog::Stale { .. } => None,
            });
            let Some(key) = key else {
                continue;
            };
            if !fresh {
                continue;
            }
            if key.starts_with("direction:") {
                published += self.draft_uncovered().await?;
            } else if let Some(item) = key
                .strip_prefix("holder:")
                .or_else(|| key.strip_prefix("done:"))
            {
                published += self.draft_ready_tickets(item)?;
            }
        }
        Ok(published)
    }

    /// Publish at most one project draft per uncovered objective, and at
    /// most four. A second open draft for the same gap is not published.
    /// When move 1 is off, this returns without calling the model.
    pub async fn draft_uncovered(&mut self) -> Result<usize, crate::error::AgentError> {
        if !self.move_1 {
            return Ok(0);
        }
        let snapshot = compile::compile(&self.state);
        let mut published = 0;
        for card in snapshot.objectives {
            if published >= MAX_PLANS_PER_SCAN {
                break;
            }
            if card.verdict != GapVerdict::Uncovered {
                continue;
            }
            let line_ref = format!("objectives@{}#{}", card.version, card.line_id);
            if gap_is_open(&self.state, &line_ref) {
                continue;
            }
            let output = self
                .model
                .structured(plan_request(&self.state, &card))
                .await?;
            let draft = match judge_change_plan(&self.state, &card, &output.value) {
                Ok(draft) => draft,
                Err(_) => continue,
            };
            self.publish_project_draft(&card, &draft)?;
            self.publish_shapers_line(&draft)?;
            published += 1;
        }
        Ok(published)
    }

    fn publish_project_draft(
        &mut self,
        card: &ObjectiveCard,
        draft: &ProjectDraft,
    ) -> Result<(), PublishError> {
        let line_ref = format!("objectives@{}#{}", card.version, card.line_id);
        let content =
            serde_json::to_string(draft).map_err(|e| PublishError::Sign(e.to_string()))?;
        let tags = vec![
            parsed_tag(&["n", "shaper"])?,
            parsed_tag(&["t", "project"])?,
            parsed_tag(&["move", "1"])?,
            parsed_tag(&["origin", "gap"])?,
            parsed_tag(&["gap", &line_ref])?,
            parsed_tag(&["e", &card.event_id, "", "receipt"])?,
        ];
        let event = sign(&self.keys, Permitted::Draft, &content, tags)?;
        self.state
            .apply(&event)
            .map_err(|e| PublishError::Rejected(e.to_string()))?;
        self.outbox.append(event.clone(), card_now())?;
        self.try_send(&event)
    }

    fn publish_shapers_line(&mut self, draft: &ProjectDraft) -> Result<(), PublishError> {
        let Some(room) = self
            .state
            .shapers
            .as_ref()
            .and_then(|shapers| shapers.room.clone())
        else {
            return Ok(());
        };
        let content = format!(
            "I drafted \"{}\" from an uncovered objective. Review it in My work.",
            draft.title
        );
        self.publish_permitted(
            Permitted::Chat,
            &content,
            vec![parsed_tag(&["h", &room])?],
            card_now(),
        )
    }

    /// Draft the plan steps that can start. Held steps stay in coverage.
    /// At most seven. A step already drafted or already a child is skipped.
    fn draft_ready_tickets(&mut self, item: &str) -> Result<usize, PublishError> {
        let Some(root_id) = root_id(&self.state, item) else {
            return Ok(0);
        };
        let Some(plan) = plan_for_root(&self.state, &root_id) else {
            return Ok(0);
        };
        let root = self.state.items.get(&root_id).cloned();
        let Some(root) = root else {
            return Ok(0);
        };
        let Some(dri) = root.dri.clone() else {
            return Ok(0);
        };
        let Some(receipt) = self.state.item_generations.get(&root_id).cloned() else {
            return Ok(0);
        };
        let mut published = 0;
        for step in &plan {
            if published >= MAX_STEPS {
                break;
            }
            if !step_can_start(&self.state, &root_id, step) {
                continue;
            }
            let after = sibling_ids(&self.state, &root_id, step);
            let draft = ticket_from_step(&root, &dri, &plan, step, after);
            self.publish_ticket_draft(&root_id, &dri, &receipt, &draft)?;
            published += 1;
        }
        Ok(published)
    }

    fn publish_ticket_draft(
        &mut self,
        root_id: &str,
        dri: &str,
        receipt: &str,
        draft: &TicketDraft,
    ) -> Result<(), PublishError> {
        let content =
            serde_json::to_string(draft).map_err(|e| PublishError::Sign(e.to_string()))?;
        let gap = format!("{root_id}#{}", draft.title);
        let mut tags = vec![
            parsed_tag(&["n", dri])?,
            parsed_tag(&["p", dri, "", "needs"])?,
            parsed_tag(&["t", "ticket"])?,
            parsed_tag(&["move", "2"])?,
            parsed_tag(&["origin", "gap"])?,
            parsed_tag(&["gap", &gap])?,
            parsed_tag(&["u", root_id])?,
            parsed_tag(&["e", receipt, "", "receipt"])?,
        ];
        if let Some(holder) = &draft.suggested_holder {
            tags.push(parsed_tag(&["p", holder, "", "suggested"])?);
        }
        let event = sign(&self.keys, Permitted::Draft, &content, tags)?;
        self.state
            .apply(&event)
            .map_err(|e| PublishError::Rejected(e.to_string()))?;
        self.outbox.append(event.clone(), card_now())?;
        self.try_send(&event)
    }
}

fn root_id(state: &OrgState, item: &str) -> Option<String> {
    let found = state.items.get(item)?;
    if found.parent.is_none() {
        Some(found.id.clone())
    } else {
        found.parent.clone()
    }
}

fn plan_for_root(state: &OrgState, root_id: &str) -> Option<Vec<PlanStep>> {
    let proposal = state.proposals.values().find(|proposal| {
        proposal.kind == ProposalKind::Project
            && proposal
                .executed
                .as_ref()
                .is_some_and(|executed| executed.id == root_id)
    })?;
    serde_json::from_value(proposal.payload.get("plan")?.clone()).ok()
}

fn step_can_start(state: &OrgState, root_id: &str, step: &PlanStep) -> bool {
    if already_named(state, root_id, &step.piece) {
        return false;
    }
    let deps_done = step.after.iter().all(|previous| {
        state.items.values().any(|item| {
            item.parent.as_deref() == Some(root_id)
                && item.title == *previous
                && item.state == WorkItemState::Done
        })
    });
    if step.held.is_some() {
        deps_done && !step.after.is_empty()
    } else {
        deps_done
    }
}

fn already_named(state: &OrgState, root_id: &str, piece: &str) -> bool {
    if state
        .items
        .values()
        .any(|item| item.parent.as_deref() == Some(root_id) && item.title == piece)
    {
        return true;
    }
    state.drafts.values().any(|draft| {
        draft.kind == DraftKind::Ticket
            && draft.gap == format!("{root_id}#{piece}")
            && match &draft.outcome {
                None => true,
                Some(outcome) => !matches!(
                    outcome.status,
                    DraftOutcomeStatus::Declined | DraftOutcomeStatus::Expired
                ),
            }
    })
}

fn sibling_ids(state: &OrgState, root_id: &str, step: &PlanStep) -> Vec<String> {
    step.after
        .iter()
        .filter_map(|previous| {
            state.items.values().find_map(|item| {
                if item.parent.as_deref() == Some(root_id) && item.title == *previous {
                    Some(item.id.clone())
                } else {
                    None
                }
            })
        })
        .collect()
}

fn ticket_from_step(
    root: &buzz_core::intelligent_org::WorkItem,
    dri: &str,
    plan: &[PlanStep],
    step: &PlanStep,
    after: Vec<String>,
) -> TicketDraft {
    TicketDraft {
        parent: root.id.clone(),
        title: step.piece.clone(),
        brief: step.piece.clone(),
        due_at: root.due_at,
        requires: step.requires.clone(),
        suggested_holder: Some(dri.to_string()),
        unfilled: None,
        covers: step.piece.clone(),
        after,
        gate: step.gate,
        coverage: plan
            .iter()
            .enumerate()
            .map(|(index, piece)| CoveragePiece {
                piece: piece.piece.clone(),
                covered_by: None,
                order: (index as u32) + 1,
                after: piece.after.clone(),
                held: piece.held.clone(),
            })
            .collect(),
        matched: None,
        done_when: step.produces.clone(),
        kind: Some(step.kind.clone()),
    }
}

fn card_now() -> u64 {
    1_700_000_000
}

fn parsed_tag(parts: &[&str]) -> Result<Tag, PublishError> {
    Tag::parse(parts.to_vec()).map_err(|error| PublishError::Sign(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile;
    use crate::fixtures::{fixtures_dir, load_events};
    use crate::state::OrgState;
    use buzz_core::intelligent_org::StrategyLineType;

    fn river() -> OrgState {
        let events = load_events(&fixtures_dir().join("orgs/river/seed.json")).expect("river");
        let mut state = OrgState::from_events(&events).expect("apply");
        let head = state.direction.get_mut("strategy").expect("strategy");
        for line in &mut head.artifact.lines {
            let text = line.text.to_lowercase();
            if text.contains("not a grant") || text.starts_with("no ") {
                line.line_type = Some(StrategyLineType::Refusal);
            }
        }
        state
    }

    fn weekday_card(state: &OrgState) -> ObjectiveCard {
        compile(state)
            .objectives
            .into_iter()
            .find(|card| card.line_id == "l_84c9b3c2")
            .expect("weekday line")
    }

    fn plan_json(kept_grant: bool, first_gate: bool, why: &str) -> serde_json::Value {
        serde_json::json!({
            "title": "Weekday hall trial",
            "brief": "Four weekday sessions, then a decision.",
            "objective_ref": "objectives@3#l_84c9b3c2",
            "due_at": 1785488400,
            "suggested_dri": null,
            "why": why,
            "gaps": [],
            "gap": { "ref": "objectives@3#l_84c9b3c2", "verdict": "uncovered" },
            "options": [
                {
                    "title": "Weekday hall trial",
                    "mechanism": "tests whether weekday buyers come",
                    "kept": true
                },
                {
                    "title": "Apply for a city grant",
                    "mechanism": "a grant round pays the hall",
                    "kept": kept_grant,
                    "why_not": if kept_grant { serde_json::Value::Null } else { serde_json::json!("strategy refusal: not a grant round") }
                }
            ],
            "change": {
                "from": "no weekday night and no evening licence",
                "to": "a trial has answered whether weekday buyers come",
                "done_when": ["four sessions held", "attendance counted"],
                "moves": ["objectives@3#l_84c9b3c2"]
            },
            "plan": [
                {
                    "piece": "Evening licence application",
                    "kind": "writing",
                    "gate": first_gate,
                    "produces": ["four sessions held"]
                },
                {
                    "piece": "Count attendance each session",
                    "kind": "ops",
                    "gate": true,
                    "produces": ["attendance counted"]
                },
                {
                    "piece": "Book four Tuesdays",
                    "kind": "ops",
                    "gate": false,
                    "held": "after Evening licence application",
                    "after": ["Evening licence application"]
                }
            ]
        })
    }

    #[test]
    fn river_weekday_hall_gates_first_and_drops_a_grant() {
        let state = river();
        let card = weekday_card(&state);
        assert_eq!(card.verdict, GapVerdict::Uncovered);
        let dropped = judge_change_plan(
            &state,
            &card,
            &plan_json(true, true, "the stall cannot fund a hall yet"),
        )
        .expect_err("grant kept");
        assert_eq!(dropped, PlanReject::Refusal);
        let ungated = judge_change_plan(
            &state,
            &card,
            &plan_json(false, false, "learn whether buyers come"),
        )
        .expect_err("no leading gate");
        assert_eq!(ungated, PlanReject::Ungated);
        let numbered = judge_change_plan(
            &state,
            &card,
            &plan_json(false, true, "rent is 400 a night"),
        )
        .expect_err("number");
        assert_eq!(numbered, PlanReject::UngroundedNumber);
        let draft = judge_change_plan(
            &state,
            &card,
            &plan_json(false, true, "learn whether weekday buyers come"),
        )
        .expect("trial");
        assert!(draft.plan[0].gate, "first step is a gate");
        assert!(draft.plan[1].gate, "the next step is a gate");
        assert!(draft.plan[2].held.is_some());
    }
}
