//! One change plan per uncovered objective.
//!
//! The compiler decides the verdict. This module asks the model for a plan,
//! then judges it without another model call. Nothing is published unless
//! [`move_1_enabled`] is on (`IO_MOVE_1_ENABLED`).

use std::collections::HashSet;

use buzz_core::intelligent_org::{
    CoveragePiece, DraftKind, DraftOutcomeStatus, DraftPayload, PlanChange, PlanStep, ProjectDraft,
    ProposalKind, StrategyLineType, TicketDraft, WorkItem, WorkItemState,
};
use nostr::Tag;

use crate::compile::{self, GapVerdict, ObjectiveCard};
use crate::jobs::JobLog;
use crate::pipeline::OrgAgent;
use crate::prompt::{self, CodeDigest, DigestLimits, WorkPromptInput};
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

fn review_already_open(state: &OrgState, root_id: &str) -> bool {
    state.drafts.values().any(|draft| {
        draft.kind == DraftKind::Review
            && draft.gap == root_id
            && match &draft.outcome {
                None => true,
                Some(outcome) => matches!(
                    outcome.status,
                    DraftOutcomeStatus::Open | DraftOutcomeStatus::Amended
                ),
            }
    })
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
                published += self.draft_ready_tickets(item).await?;
            } else if let Some(item) = key.strip_prefix("prompt:") {
                published += self.publish_ticket_prompt(item)?;
            } else if let Some(root) = key.strip_prefix("review:") {
                published += self.publish_review(root).await?;
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

    /// Draft every plan step, including steps that wait. At most seven.
    /// A step already drafted or already a child is skipped.
    /// A held project whose plan is missing or not offerable asks the model
    /// for two to seven steps first. A plan already stored in memory is not
    /// asked for again.
    async fn draft_ready_tickets(&mut self, item: &str) -> Result<usize, crate::error::AgentError> {
        if let Some(root_id) = root_id(&self.state, item) {
            let root = self.state.items.get(&root_id).cloned();
            let (repo, commit, paths) = root
                .as_ref()
                .map(|root| self.planner_digest(root))
                .unwrap_or_default();
            if !paths.is_empty() {
                remember_digest(&mut self.state, &repo, &commit, &paths);
            }
            let recovered = plan_for_root(&self.state, &root_id);
            let needs_steps = match &recovered {
                None => true,
                Some(plan) => !plan_ready(plan) && !self.state.plans.contains_key(&root_id),
            };
            if needs_steps {
                if let Some(root) = root {
                    let context = digest_block(&repo, &commit, &paths);
                    let output = self
                        .model
                        .structured(steps_model_request(&root.title, &root.brief, &context))
                        .await?;
                    if let Some(steps) = steps_ready_in(&root.title, &output.value, &paths) {
                        self.state.plans.insert(root_id, steps);
                    }
                }
            }
        }
        let events = draft_ready(&mut self.state, &self.keys, item)?;
        let published = events.len();
        for event in &events {
            self.outbox.append(event.clone(), card_now())?;
            self.try_send(event)?;
        }
        Ok(published)
    }

    /// Publish one `50100` review. The compiler picks the next step. The
    /// model writes why. A dropped explanation publishes nothing.
    async fn publish_review(&mut self, root_id: &str) -> Result<usize, crate::error::AgentError> {
        if review_already_open(&self.state, root_id) {
            return Ok(0);
        }
        let Some(root) = self.state.items.get(root_id).cloned() else {
            return Ok(0);
        };
        if root.parent.is_some() || root.state != WorkItemState::InReview {
            return Ok(0);
        }
        let decisions = self.decisions_body(&root);
        let output = self
            .model
            .structured(crate::review::review_request(
                &self.state,
                root_id,
                decisions.as_deref(),
            ))
            .await?;
        let draft = match crate::review::judge_review(&self.state, root_id, &output.value) {
            Ok(draft) => draft,
            Err(_) => return Ok(0),
        };
        let Some(receipt) = self.state.item_generations.get(root_id).cloned() else {
            return Ok(0);
        };
        self.publish_review_draft(&draft, &receipt)?;
        self.publish_review_line(&root, &draft)?;
        Ok(1)
    }

    fn decisions_body(&self, root: &WorkItem) -> Option<String> {
        let repo = root.home.as_ref().and_then(|home| home.repo.clone())?;
        self.context_files
            .get(&repo)?
            .get("context/decisions.md")
            .cloned()
    }

    fn publish_review_draft(
        &mut self,
        draft: &buzz_core::intelligent_org::ReviewDraft,
        receipt: &str,
    ) -> Result<(), PublishError> {
        let content =
            serde_json::to_string(draft).map_err(|error| PublishError::Sign(error.to_string()))?;
        let tags = vec![
            parsed_tag(&["n", "shaper"])?,
            parsed_tag(&["t", "review"])?,
            parsed_tag(&["move", "3"])?,
            parsed_tag(&["origin", "gap"])?,
            parsed_tag(&["gap", &draft.item])?,
            parsed_tag(&["i", &draft.item])?,
            parsed_tag(&["e", receipt, "", "receipt"])?,
        ];
        let event = sign(&self.keys, Permitted::Draft, &content, tags)?;
        self.state
            .apply(&event)
            .map_err(|error| PublishError::Rejected(error.to_string()))?;
        self.outbox.append(event.clone(), card_now())?;
        self.try_send(&event)
    }

    fn publish_review_line(
        &mut self,
        root: &WorkItem,
        draft: &buzz_core::intelligent_org::ReviewDraft,
    ) -> Result<(), PublishError> {
        let Some(room) = self
            .state
            .shapers
            .as_ref()
            .and_then(|shapers| shapers.room.clone())
        else {
            return Ok(());
        };
        let next = match &draft.recommendation {
            buzz_core::intelligent_org::ReviewRecommendation::FollowUp { .. } => {
                "a follow-up project"
            }
            buzz_core::intelligent_org::ReviewRecommendation::ObjectivesRedraw { .. } => {
                "an objectives redraw"
            }
            buzz_core::intelligent_org::ReviewRecommendation::NoFurtherWork { .. } => "stop",
        };
        let content = format!(
            "I drafted a review of \"{}\". The next step is {next}. It is in My work.",
            root.title
        );
        self.publish_permitted(
            Permitted::Chat,
            &content,
            vec![parsed_tag(&["h", &room])?],
            card_now(),
        )
    }

    /// Publish one `50104` for an offered or accepted ticket. A code ticket
    /// whose digest fails is left without a prompt.
    fn publish_ticket_prompt(&mut self, item_id: &str) -> Result<usize, PublishError> {
        let Some(item) = self.state.items.get(item_id).cloned() else {
            return Ok(0);
        };
        if item.parent.is_none()
            || !matches!(item.state, WorkItemState::Offered | WorkItemState::Accepted)
        {
            return Ok(0);
        }
        let Some(based_on) = self.state.item_generations.get(item_id).cloned() else {
            return Ok(0);
        };
        if based_on.is_empty() {
            return Ok(0);
        }
        let root_id = item.root.clone();
        let root = self.state.items.get(&root_id).cloned();
        let step = plan_for_root(&self.state, &root_id)
            .and_then(|plan| plan.into_iter().find(|step| step.piece == item.title));
        let kind = step
            .as_ref()
            .map(|step| step.kind.clone())
            .unwrap_or_else(|| "writing".into());
        let done_when = step
            .as_ref()
            .map(|step| step.produces.clone())
            .unwrap_or_default();
        let (change_from, change_to) = change_ends(&self.state, &root_id);
        let objective = root
            .as_ref()
            .map(|root| objective_text(&self.state, root))
            .unwrap_or_default();
        let constraints = strategy_constraints(&self.state);
        let mut commit = None;
        let digest = if kind == "code" {
            match self.code_digest(&root) {
                Some(mut code) => {
                    commit = Some(code.commit.clone());
                    if let Some(step) = step.as_ref().filter(|step| !step.files.is_empty()) {
                        let allowed = self
                            .listings
                            .get(&code.repo)
                            .map(|listing| {
                                listing
                                    .files
                                    .iter()
                                    .map(|file| file.path.clone())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        code.paths = step
                            .files
                            .iter()
                            .filter(|file| allowed.iter().any(|path| path == *file))
                            .cloned()
                            .collect();
                    }
                    Ok(Some(code))
                }
                None => return Ok(0),
            }
        } else {
            Ok(None)
        };
        let how = step
            .as_ref()
            .map(|step| step.how.clone())
            .unwrap_or_default();
        let waits_on = step
            .as_ref()
            .map(|step| step.after.clone())
            .unwrap_or_default();
        let files = step
            .as_ref()
            .map(|step| step.files.clone())
            .unwrap_or_default();
        let input = WorkPromptInput {
            ticket_id: &item.id,
            title: &item.title,
            step: &item.brief,
            objective: &objective,
            change_from: &change_from,
            change_to: &change_to,
            done_when: &done_when,
            constraints: &constraints,
            kind: &kind,
            how: &how,
            waits_on: &waits_on,
            files: &files,
        };
        let Some(text) = prompt::prompt_text(&input, digest) else {
            return Ok(0);
        };
        if self.last_prompt.get(item_id) == Some(&text) {
            return Ok(0);
        }
        let tags = match commit.as_deref() {
            Some(commit) => vec![
                parsed_tag(&["i", item_id])?,
                parsed_tag(&["based_on", &based_on, commit])?,
            ],
            None => vec![
                parsed_tag(&["i", item_id])?,
                parsed_tag(&["based_on", &based_on])?,
            ],
        };
        let event = sign(&self.keys, Permitted::WorkPrompt, &text, tags)?;
        self.last_prompt.insert(item_id.to_owned(), text);
        self.outbox.append(event.clone(), card_now())?;
        self.try_send(&event)?;
        Ok(1)
    }

    fn code_digest(&mut self, root: &Option<WorkItem>) -> Option<CodeDigest> {
        let repo = root
            .as_ref()
            .and_then(|root| root.home.as_ref())
            .and_then(|home| home.repo.clone())?;
        let listing = self.listings.get(&repo).cloned()?;
        let mut cache = self.digest_cache.get(&repo).cloned();
        let paths = prompt::digest_paths(&mut cache, &listing, DigestLimits::default()).ok()?;
        if let Some(cached) = cache {
            self.digest_cache.insert(repo.clone(), cached);
        }
        Some(CodeDigest {
            repo,
            commit: listing.commit,
            paths,
        })
    }

    /// Paths a code step may name for this project's home repository.
    fn planner_digest(&self, root: &WorkItem) -> (String, String, Vec<String>) {
        let Some(repo) = root
            .home
            .as_ref()
            .and_then(|home| home.repo.clone())
            .filter(|repo| !repo.is_empty())
        else {
            return (String::new(), String::new(), Vec::new());
        };
        let Some(listing) = self.listings.get(&repo) else {
            return (repo, String::new(), Vec::new());
        };
        let paths = listing
            .files
            .iter()
            .map(|file| file.path.clone())
            .filter(|path| !path.is_empty() && !path.contains(".."))
            .take(80)
            .collect();
        (repo, listing.commit.clone(), paths)
    }
}

fn change_ends(state: &OrgState, root_id: &str) -> (String, String) {
    let Some(proposal) = state.proposals.values().find(|proposal| {
        proposal.kind == ProposalKind::Project
            && proposal
                .executed
                .as_ref()
                .is_some_and(|executed| executed.id == root_id)
    }) else {
        return (String::new(), String::new());
    };
    let change = proposal.payload.get("change");
    let from = change
        .and_then(|value| value.get("from"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned();
    let to = change
        .and_then(|value| value.get("to"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_owned();
    (from, to)
}

fn objective_text(state: &OrgState, root: &WorkItem) -> String {
    let Some(line_ref) = root.objective_ref.as_deref() else {
        return String::new();
    };
    let Some((_, line_id)) = line_ref.split_once('#') else {
        return String::new();
    };
    state
        .direction
        .get("objectives")
        .and_then(|head| head.artifact.lines.iter().find(|line| line.id == line_id))
        .map(|line| line.text.clone())
        .unwrap_or_default()
}

fn strategy_constraints(state: &OrgState) -> Vec<String> {
    let Some(head) = state.direction.get("strategy") else {
        return Vec::new();
    };
    head.artifact
        .lines
        .iter()
        .filter_map(|line| match line.line_type {
            Some(StrategyLineType::Refusal) => Some(format!("refusal: {}", line.text)),
            Some(StrategyLineType::Rule) => Some(format!("rule: {}", line.text)),
            _ => None,
        })
        .collect()
}

fn root_id(state: &OrgState, item: &str) -> Option<String> {
    let found = state.items.get(item)?;
    if found.parent.is_none() {
        Some(found.id.clone())
    } else {
        found.parent.clone()
    }
}

/// Ticket drafts for every step of a held project, including steps that
/// wait on another step. A project whose plan is not offerable drafts
/// nothing until [`steps_ready`] has stored one.
pub(crate) fn take_due_tickets(
    state: &mut OrgState,
    keys: &nostr::Keys,
) -> Result<Vec<buzz_core::Event>, PublishError> {
    let ids: Vec<String> = state.items.keys().cloned().collect();
    let mut events = Vec::new();
    for id in ids {
        let Some(item) = state.items.get(&id) else {
            continue;
        };
        let held_root =
            item.parent.is_none() && item.state == WorkItemState::Accepted && item.dri.is_some();
        let done_child = item.parent.is_some() && item.state == WorkItemState::Done;
        if held_root || done_child {
            events.extend(draft_ready(state, keys, &id)?);
        }
    }
    Ok(events)
}

/// Draft every plan step under `item`'s project, including steps that wait.
/// A waiting step names what it waits on. Each ticket is published with its
/// work prompt. A plan that is not yet offerable publishes nothing.
fn draft_ready(
    state: &mut OrgState,
    keys: &nostr::Keys,
    item: &str,
) -> Result<Vec<buzz_core::Event>, PublishError> {
    let Some(root_id) = root_id(state, item) else {
        return Ok(Vec::new());
    };
    let Some(root) = state.items.get(&root_id).cloned() else {
        return Ok(Vec::new());
    };
    let Some(plan) = plan_for_root(state, &root_id) else {
        return Ok(Vec::new());
    };
    let plan = ground_code_files(plan, &allowed_paths(state, &root));
    if !plan_ready(&plan) {
        return Ok(Vec::new());
    }
    let Some(dri) = root.dri.clone() else {
        return Ok(Vec::new());
    };
    let Some(receipt) = state.item_generations.get(&root_id).cloned() else {
        return Ok(Vec::new());
    };
    let (change_from, change_to) = change_ends(state, &root_id);
    let objective = objective_text(state, &root);
    let constraints = strategy_constraints(state);
    let repo = root
        .home
        .as_ref()
        .and_then(|home| home.repo.clone())
        .unwrap_or_else(|| "the project repository".into());
    let mut events = Vec::new();
    let mut tickets = 0usize;
    for step in &plan {
        if tickets >= MAX_STEPS {
            break;
        }
        if same_title(&step.piece, &root.title) || already_named(state, &root_id, &step.piece) {
            continue;
        }
        let after = sibling_ids(state, &root_id, step);
        let draft = ticket_from_step(state, &root, &dri, &plan, step, after);
        let ticket = stage_ticket_draft(state, keys, &root_id, &dri, &receipt, &draft)?;
        let draft_id = ticket.id.to_hex();
        events.push(ticket);
        tickets += 1;
        let commit = digest_commit(state, &root);
        if let Some(prompt) = stage_prompt(
            keys,
            &draft_id,
            &PromptFacts {
                repo: &repo,
                commit: &commit,
                objective: &objective,
                change_from: &change_from,
                change_to: &change_to,
                constraints: &constraints,
            },
            step,
        )? {
            events.push(prompt);
        }
    }
    Ok(events)
}

fn stage_ticket_draft(
    state: &mut OrgState,
    keys: &nostr::Keys,
    root_id: &str,
    dri: &str,
    receipt: &str,
    draft: &TicketDraft,
) -> Result<buzz_core::Event, PublishError> {
    let content = serde_json::to_string(draft).map_err(|e| PublishError::Sign(e.to_string()))?;
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
    let event = sign(keys, Permitted::Draft, &content, tags)?;
    state
        .apply(&event)
        .map_err(|e| PublishError::Rejected(e.to_string()))?;
    Ok(event)
}

fn plan_for_root(state: &OrgState, root_id: &str) -> Option<Vec<PlanStep>> {
    let title = state
        .items
        .get(root_id)
        .map(|item| item.title.clone())
        .unwrap_or_default();
    let allowed = state
        .items
        .get(root_id)
        .map(|root| allowed_paths(state, root))
        .unwrap_or_default();
    let proposal = proposal_plan(state, root_id).and_then(|plan| {
        without_title_clone(&title, plan).map(|plan| ground_code_files(plan, &allowed))
    });
    let memory = state.plans.get(root_id).and_then(|plan| {
        without_title_clone(&title, plan.clone()).map(|plan| ground_code_files(plan, &allowed))
    });
    let drafts =
        plan_from_drafts(state, root_id, &title).map(|plan| ground_code_files(plan, &allowed));
    for candidate in [&memory, &proposal, &drafts] {
        if candidate.as_ref().is_some_and(|plan| plan_ready(plan)) {
            return candidate.clone();
        }
    }
    memory.or(proposal).or(drafts)
}

/// Two to seven steps, each with a brief, how-lines, and a done-when that
/// is not the title. A code step names files.
fn plan_ready(plan: &[PlanStep]) -> bool {
    (2..=MAX_STEPS).contains(&plan.len()) && plan.iter().all(step_ready)
}

fn step_ready(step: &PlanStep) -> bool {
    if vacuous_title(&step.piece) {
        return false;
    }
    let brief = step.brief.trim();
    if brief.is_empty() || same_title(brief, &step.piece) {
        return false;
    }
    if !(2..=4).contains(&step.how.len()) {
        return false;
    }
    if step.how.iter().any(|line| {
        let line = line.trim();
        line.len() < 12
            || same_title(line, &step.piece)
            || step
                .produces
                .iter()
                .any(|produced| same_title(line, produced))
    }) {
        return false;
    }
    if !step
        .produces
        .iter()
        .any(|line| done_when_ok(&step.piece, line))
    {
        return false;
    }
    if step.kind == "code" && step.files.is_empty() {
        return false;
    }
    true
}

fn done_when_ok(title: &str, line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || same_title(line, title) {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    lower != "the step is done" && lower != "done" && !lower.ends_with(" is done")
}

fn proposal_plan(state: &OrgState, root_id: &str) -> Option<Vec<PlanStep>> {
    let proposal = state.proposals.values().find(|proposal| {
        proposal.kind == ProposalKind::Project
            && proposal
                .executed
                .as_ref()
                .is_some_and(|executed| executed.id == root_id)
    })?;
    let plan: Vec<PlanStep> = serde_json::from_value(proposal.payload.get("plan")?.clone()).ok()?;
    if plan.is_empty() {
        None
    } else {
        Some(plan)
    }
}

const STEP_KINDS: &[&str] = &["code", "research", "writing", "outreach", "design", "ops"];

/// The words the chat model and the hold path both use to break a project
/// into steps. `context` is direction text, and may be empty.
pub(crate) fn project_steps_prompt(title: &str, brief: &str, context: &str) -> String {
    format!(
        "Break this project into the steps one person would do.\n\
         Title: {title}\n\
         Brief: {brief}\n\
         {context}\n\
         Return JSON only, no markdown: {{\"steps\":[{{\"piece\":\"...\",\"brief\":\"...\",\"kind\":\"code\",\"gate\":false,\"after\":[],\"requires\":[],\"produces\":[\"...\"],\"how\":[\"...\",\"...\"],\"files\":[]}}]}}\n\
         Two to seven steps. piece is a short specific title, at most eight words, never the project title.\n\
         A title that could sit under any project is not a step: Do research, Make a plan, Kick-off, Implement the feature.\n\
         brief is one sentence of what to do. It is not the title and not a how-line.\n\
         kind is code, research, writing, outreach, design, or ops.\n\
         requires is the skill this step needs. Leave it empty when any member could do it. Do not name a person who is not in the org.\n\
         If a later step needs an answer, an earlier step is the one that gets that answer, and the later step names that piece in after.\n\
         produces is the check that the step is done. It is not the title and not \"the step is done\".\n\
         how is two to four imperative sentences. Do not copy produces into how.\n\
         A code step's files are paths from the Digest files list in the context. Do not invent a path. A code step with no file from that list is not a step.\n\
         Do not repeat the project as a step. Do not mention money."
    )
}

/// Held projects that still have no usable step list.
pub(crate) fn projects_without_steps(state: &OrgState) -> Vec<(String, String, String)> {
    state
        .items
        .values()
        .filter(|item| {
            item.parent.is_none()
                && item.state == WorkItemState::Accepted
                && item.dri.is_some()
                && plan_for_root(state, &item.id).is_none()
        })
        .map(|item| (item.id.clone(), item.title.clone(), item.brief.clone()))
        .collect()
}

/// Held projects whose step list has no how-to yet, and this process has
/// not already asked for one.
pub(crate) fn projects_needing_how(state: &OrgState) -> Vec<(String, String, String)> {
    state
        .items
        .values()
        .filter(|item| {
            item.parent.is_none()
                && item.state == WorkItemState::Accepted
                && item.dri.is_some()
                && !state.plans.contains_key(&item.id)
                && plan_for_root(state, &item.id).is_some_and(|plan| !plan_ready(&plan))
        })
        .map(|item| (item.id.clone(), item.title.clone(), item.brief.clone()))
        .collect()
}

/// Direction text for a step list, capped so the prompt stays small.
pub(crate) fn direction_excerpt(state: &OrgState) -> String {
    let mut parts = Vec::new();
    for slug in ["mission", "vision", "situation", "objectives", "strategy"] {
        let Some(head) = state.direction.get(slug) else {
            continue;
        };
        let body: String = head.artifact.body.chars().take(400).collect();
        if !body.trim().is_empty() {
            parts.push(format!("{slug}: {body}"));
        }
    }
    let text = parts.join("\n");
    text.chars().take(1500).collect()
}

fn steps_model_request(title: &str, brief: &str, context: &str) -> ModelRequest {
    ModelRequest {
        tier: Tier::Fast,
        system: project_steps_prompt(title, brief, context),
        messages: vec![crate::think::model::Message {
            role: "user".into(),
            content: format!("Title: {title}\nBrief: {brief}"),
        }],
        schema: serde_json::json!({ "type": "object" }),
        tool_name: "emit_steps".into(),
        tools: vec![],
        temperature: 0.2,
        max_output_tokens: 1400,
    }
}

/// Two to seven offerable steps that are not the project title. `raw` is
/// the model's JSON object, or the `plan` array on a project act.
pub fn steps_ready(title: &str, raw: &serde_json::Value) -> Option<Vec<PlanStep>> {
    steps_ready_in(title, raw, &[])
}

/// [`steps_ready`], and when `allowed` is non-empty every code file must be
/// in that digest. A code step whose files are all outside it is not offerable.
pub fn steps_ready_in(
    title: &str,
    raw: &serde_json::Value,
    allowed: &[String],
) -> Option<Vec<PlanStep>> {
    let parsed = parse_step_array(raw)?;
    let steps = without_title_clone(title, parsed)?;
    let steps = ground_code_files(steps, allowed);
    if plan_ready(&steps) {
        Some(steps)
    } else {
        None
    }
}

/// Record a digest the accept path can enforce. An empty list is not a digest.
/// `commit` is the revision the paths were read from. A code prompt names it.
pub(crate) fn remember_digest(state: &mut OrgState, repo: &str, commit: &str, paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    state.code_paths.insert(repo.to_string(), paths.to_vec());
    if !commit.is_empty() {
        state
            .code_commits
            .insert(repo.to_string(), commit.to_string());
    }
}

/// Store every `Digest files` block in `context`. Each repository keeps its
/// own commit. Paths are also merged under `""` for a project with no home yet.
pub(crate) fn remember_digests_in(state: &mut OrgState, context: &str) {
    let mut repo = String::new();
    let mut commit = String::new();
    let mut paths = Vec::new();
    let mut in_digest = false;
    for line in context.lines() {
        let line = line.trim();
        if let Some((next_repo, next_commit)) = digest_header(line) {
            flush_digest(state, &repo, &commit, &mut paths);
            repo = next_repo;
            commit = next_commit;
            in_digest = true;
            continue;
        }
        if !in_digest {
            continue;
        }
        let Some(path) = line.strip_prefix('-') else {
            if line.is_empty() {
                continue;
            }
            in_digest = false;
            continue;
        };
        let path = path.trim();
        if !path.is_empty() && !path.contains("..") {
            paths.push(path.to_string());
        }
    }
    flush_digest(state, &repo, &commit, &mut paths);
}

fn flush_digest(state: &mut OrgState, repo: &str, commit: &str, paths: &mut Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let kept = paths.clone();
    remember_digest(state, repo, commit, &kept);
    if !repo.is_empty() {
        let mut merged = state.code_paths.get("").cloned().unwrap_or_default();
        for path in &kept {
            if !merged.iter().any(|have| have == path) {
                merged.push(path.clone());
            }
        }
        state.code_paths.insert(String::new(), merged);
        if !commit.is_empty() && !state.code_commits.contains_key("") {
            state.code_commits.insert(String::new(), commit.to_string());
        }
    }
    paths.clear();
}

fn digest_header(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("Digest files (")?.strip_suffix("):")?;
    match rest.rsplit_once('@') {
        Some((repo, commit)) if !repo.is_empty() && !commit.is_empty() && !commit.contains(' ') => {
            Some((repo.to_string(), commit.to_string()))
        }
        _ => Some((rest.to_string(), String::new())),
    }
}

fn digest_commit(state: &OrgState, root: &WorkItem) -> String {
    if let Some(repo) = root.home.as_ref().and_then(|home| home.repo.as_deref()) {
        if let Some(commit) = state.code_commits.get(repo) {
            return commit.clone();
        }
    }
    state.code_commits.get("").cloned().unwrap_or_default()
}

fn allowed_paths(state: &OrgState, root: &WorkItem) -> Vec<String> {
    if let Some(repo) = root.home.as_ref().and_then(|home| home.repo.as_deref()) {
        if let Some(paths) = state.code_paths.get(repo) {
            return paths.clone();
        }
    }
    state.code_paths.get("").cloned().unwrap_or_default()
}

fn ground_code_files(mut steps: Vec<PlanStep>, allowed: &[String]) -> Vec<PlanStep> {
    if allowed.is_empty() {
        return steps;
    }
    let allow: HashSet<&str> = allowed.iter().map(String::as_str).collect();
    for step in &mut steps {
        if step.kind == "code" {
            step.files.retain(|path| allow.contains(path.as_str()));
        }
    }
    steps
}

/// `Digest files (repo@commit):` followed by `- path` lines. Empty when
/// there is nothing to name.
pub(crate) fn digest_block(repo: &str, commit: &str, paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let mut out = format!("Digest files ({repo}");
    if !commit.is_empty() {
        out.push('@');
        out.push_str(commit);
    }
    out.push_str("):\n");
    for path in paths.iter().take(80) {
        out.push_str("- ");
        out.push_str(path);
        out.push('\n');
    }
    out
}

/// Paths under a `Digest files` block. Anything else in the context is ignored.
pub(crate) fn digest_paths_in(context: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut in_digest = false;
    for line in context.lines() {
        let line = line.trim();
        if line.starts_with("Digest files") {
            in_digest = true;
            continue;
        }
        if !in_digest {
            continue;
        }
        let Some(path) = line.strip_prefix('-') else {
            if line.is_empty() {
                continue;
            }
            in_digest = false;
            continue;
        };
        let path = path.trim();
        if !path.is_empty() && !path.contains("..") {
            paths.push(path.to_string());
        }
    }
    paths
}

fn parse_step_array(raw: &serde_json::Value) -> Option<Vec<PlanStep>> {
    let array = raw
        .as_array()
        .or_else(|| raw.get("steps").and_then(|value| value.as_array()))
        .or_else(|| raw.get("plan").and_then(|value| value.as_array()))?;
    let mut steps = Vec::new();
    for value in array {
        let original = value
            .get("piece")
            .and_then(|item| item.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let piece = short_name(&original);
        if piece.is_empty() {
            continue;
        }
        let kind = value
            .get("kind")
            .and_then(|item| item.as_str())
            .unwrap_or("ops");
        let kind = if STEP_KINDS.contains(&kind) {
            kind.to_string()
        } else {
            "ops".to_string()
        };
        let produces: Vec<String> = value
            .get("produces")
            .and_then(|item| item.as_array())
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|line| line.as_str())
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .filter(|lines: &Vec<String>| !lines.is_empty())
            .unwrap_or_default();
        let how = value
            .get("how")
            .and_then(|item| item.as_array())
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|line| line.as_str())
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(|line| line.chars().take(240).collect::<String>())
                    .take(4)
                    .collect()
            })
            .unwrap_or_default();
        let brief = value
            .get("brief")
            .and_then(|item| item.as_str())
            .unwrap_or("")
            .trim()
            .chars()
            .take(280)
            .collect::<String>();
        let files = value
            .get("files")
            .and_then(|item| item.as_array())
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(|path| path.as_str())
                    .map(str::trim)
                    .filter(|path| !path.is_empty() && !path.contains(".."))
                    .map(str::to_string)
                    .take(12)
                    .collect()
            })
            .unwrap_or_default();
        let after = value
            .get("after")
            .and_then(|item| item.as_array())
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| name.as_str())
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(short_name)
                    .collect()
            })
            .unwrap_or_default();
        steps.push(PlanStep {
            piece,
            kind,
            gate: value
                .get("gate")
                .and_then(|item| item.as_bool())
                .unwrap_or(false),
            answers: None,
            requires: string_list(value, "requires"),
            after,
            held: value
                .get("held")
                .and_then(|item| item.as_str())
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string),
            size: None,
            produces,
            how,
            brief,
            files,
        });
    }
    if steps.is_empty() {
        None
    } else {
        Some(steps)
    }
}

/// Drop a step that only repeats the project. One real step is enough to
/// draft from a plan that was already stored. The model path still requires two.
fn without_title_clone(title: &str, steps: Vec<PlanStep>) -> Option<Vec<PlanStep>> {
    let mut steps: Vec<PlanStep> = steps
        .into_iter()
        .filter(|step| {
            let piece = step.piece.trim();
            !piece.is_empty() && !same_title(piece, title)
        })
        .take(MAX_STEPS)
        .collect();
    if steps.is_empty() {
        return None;
    }
    let names: HashSet<String> = steps.iter().map(|step| step.piece.clone()).collect();
    for step in &mut steps {
        step.after
            .retain(|previous| names.contains(previous) && previous != &step.piece);
        if step.after.is_empty() {
            step.held = None;
        }
    }
    if steps.iter().all(|step| !step.after.is_empty()) {
        steps[0].after.clear();
        steps[0].held = None;
    }
    Some(steps)
}

fn same_title(piece: &str, title: &str) -> bool {
    piece.trim().eq_ignore_ascii_case(title.trim())
}

/// Card titles stay a short name. The check for the step stays in `produces`.
fn short_name(piece: &str) -> String {
    let clause = piece.split([',', ';', ':']).next().unwrap_or(piece).trim();
    let mut words: Vec<&str> = clause.split_whitespace().collect();
    if words.len() > 8 {
        words.truncate(8);
    }
    while words.last().is_some_and(|word| {
        matches!(
            word.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_ascii_lowercase()
                .as_str(),
            "a" | "an"
                | "the"
                | "and"
                | "or"
                | "with"
                | "from"
                | "to"
                | "of"
                | "for"
                | "including"
                | "that"
                | "which"
        )
    }) {
        words.pop();
    }
    if words.len() < 2 {
        words = piece.split_whitespace().take(6).collect();
    }
    words.join(" ")
}

fn plan_from_drafts(state: &OrgState, root_id: &str, title: &str) -> Option<Vec<PlanStep>> {
    let coverage = state.drafts.values().find_map(|draft| {
        let DraftPayload::Ticket(ticket) = &draft.payload else {
            return None;
        };
        if ticket.parent != root_id || ticket.coverage.len() < 2 {
            return None;
        }
        if ticket
            .coverage
            .iter()
            .all(|piece| same_title(&piece.piece, title))
        {
            return None;
        }
        Some(ticket.coverage.clone())
    })?;
    let steps = coverage
        .into_iter()
        .map(|piece| {
            let sibling = state.drafts.values().find_map(|draft| {
                let DraftPayload::Ticket(ticket) = &draft.payload else {
                    return None;
                };
                (ticket.parent == root_id && ticket.title == piece.piece).then_some(ticket)
            });
            PlanStep {
                piece: piece.piece.clone(),
                kind: sibling
                    .and_then(|ticket| ticket.kind.clone())
                    .unwrap_or_else(|| "ops".into()),
                gate: sibling.is_some_and(|ticket| ticket.gate),
                answers: None,
                requires: sibling
                    .map(|ticket| ticket.requires.clone())
                    .unwrap_or_default(),
                after: piece.after,
                held: piece.held,
                size: None,
                produces: sibling
                    .map(|ticket| ticket.done_when.clone())
                    .filter(|lines| !lines.is_empty())
                    .unwrap_or_default(),
                how: sibling.map(|ticket| ticket.how.clone()).unwrap_or_default(),
                brief: sibling
                    .map(|ticket| ticket.brief.clone())
                    .unwrap_or_default(),
                files: sibling
                    .map(|ticket| ticket.files.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();
    without_title_clone(title, steps)
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
    state: &OrgState,
    root: &buzz_core::intelligent_org::WorkItem,
    dri: &str,
    plan: &[PlanStep],
    step: &PlanStep,
    after: Vec<String>,
) -> TicketDraft {
    let (suggested_holder, unfilled) = holder_for(state, dri, &step.requires);
    TicketDraft {
        parent: root.id.clone(),
        title: step.piece.clone(),
        brief: step.brief.clone(),
        due_at: root.due_at,
        requires: step.requires.clone(),
        suggested_holder,
        unfilled,
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
                held: if piece.after.is_empty() {
                    None
                } else {
                    Some(format!("after {}", piece.after.join(", ")))
                },
            })
            .collect(),
        matched: None,
        done_when: step.produces.clone(),
        kind: Some(step.kind.clone()),
        how: step.how.clone(),
        waits_on: step.after.clone(),
        files: step.files.clone(),
    }
}

struct PromptFacts<'a> {
    repo: &'a str,
    commit: &'a str,
    objective: &'a str,
    change_from: &'a str,
    change_to: &'a str,
    constraints: &'a [String],
}

fn stage_prompt(
    keys: &nostr::Keys,
    draft_id: &str,
    facts: &PromptFacts<'_>,
    step: &PlanStep,
) -> Result<Option<buzz_core::Event>, PublishError> {
    let digest = if step.kind == "code" {
        if step.files.is_empty() {
            return Ok(None);
        }
        Ok(Some(CodeDigest {
            repo: facts.repo.to_string(),
            commit: facts.commit.to_string(),
            paths: step.files.clone(),
        }))
    } else {
        Ok(None)
    };
    let input = WorkPromptInput {
        ticket_id: draft_id,
        title: &step.piece,
        step: &step.brief,
        objective: facts.objective,
        change_from: facts.change_from,
        change_to: facts.change_to,
        done_when: &step.produces,
        constraints: facts.constraints,
        kind: &step.kind,
        how: &step.how,
        waits_on: &step.after,
        files: &step.files,
    };
    let Some(text) = prompt::prompt_text(&input, digest) else {
        return Ok(None);
    };
    let tags = if step.kind == "code" && !facts.commit.is_empty() {
        vec![
            parsed_tag(&["i", draft_id])?,
            parsed_tag(&["based_on", draft_id, facts.commit])?,
        ]
    } else {
        vec![
            parsed_tag(&["i", draft_id])?,
            parsed_tag(&["based_on", draft_id])?,
        ]
    };
    Ok(Some(sign(keys, Permitted::WorkPrompt, &text, tags)?))
}

/// Titles that could sit under any project. The eval list is the same set.
const VACUOUS_TITLES: &[&str] = &[
    "Make a plan",
    "Make a plan for this",
    "Do research",
    "Research",
    "Kick-off",
    "Kickoff",
    "Kick off",
    "Get started",
    "Start the work",
    "Follow up",
    "Follow-up",
    "Next steps",
    "Move this forward",
    "Coordinate",
    "Coordination",
    "Sync",
    "Alignment",
    "Set up a meeting",
    "Send an update",
    "Write a brief",
    "Draft a proposal",
    "Improve things",
    "Be more visible",
    "General outreach",
    "Stakeholder engagement",
    "Capacity building",
    "Project management",
    "Admin",
    "Miscellaneous",
    "TBD",
    "TODO",
    "Phase 1",
    "Workstream",
    "Initiative",
    "Implement the feature",
];

fn vacuous_title(title: &str) -> bool {
    let title = title.trim();
    VACUOUS_TITLES
        .iter()
        .any(|bad| title.eq_ignore_ascii_case(bad))
}

fn string_list(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.chars().take(80).collect::<String>())
                .take(6)
                .collect()
        })
        .unwrap_or_default()
}

/// The member whose skills cover every requirement, or nobody.
/// An empty `requires` keeps the project holder.
fn holder_for(
    state: &OrgState,
    dri: &str,
    requires: &[String],
) -> (Option<String>, Option<String>) {
    if requires.is_empty() {
        return (Some(dri.to_string()), None);
    }
    if state
        .profiles
        .get(dri)
        .is_some_and(|profile| covers_requires(profile, requires))
    {
        return (Some(dri.to_string()), None);
    }
    if let Some(pubkey) = state
        .profiles
        .iter()
        .find(|(_, profile)| covers_requires(profile, requires))
        .map(|(pubkey, _)| pubkey.clone())
    {
        return (Some(pubkey), None);
    }
    (
        None,
        Some(format!("nobody here can hold it: {}", requires.join(", "))),
    )
}

fn covers_requires(profile: &buzz_core::intelligent_org::OrgProfile, requires: &[String]) -> bool {
    requires.iter().all(|need| {
        profile
            .skills
            .iter()
            .any(|skill| same_skill(&skill.slug, need) || same_skill(&skill.label, need))
    })
}

fn same_skill(have: &str, need: &str) -> bool {
    let have = normalize_skill(have);
    let need = normalize_skill(need);
    !need.is_empty() && have == need
}

fn normalize_skill(value: &str) -> String {
    value
        .split(['-', ' ', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
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

    #[test]
    fn a_step_named_the_project_is_not_a_plan() {
        let raw = serde_json::json!({
            "steps": [
                { "piece": "Continuous task flow", "kind": "code", "produces": ["the project is done"] }
            ]
        });
        assert!(steps_ready("Continuous task flow", &raw).is_none());
    }

    #[test]
    fn two_real_steps_are_a_project_plan() {
        let raw = serde_json::json!({
            "steps": [
                {
                    "piece": "Show the next task on My work",
                    "brief": "Put the next offered task on the My work column.",
                    "kind": "code",
                    "after": [],
                    "files": ["desktop/src/features/org/work/model.ts"],
                    "produces": ["the next task is visible on My work"],
                    "how": ["Open the My work column component.", "Render the next offered task under You hold."]
                },
                {
                    "piece": "Offer the next task when one is done",
                    "brief": "When a held task is marked done, offer the task that was waiting on it.",
                    "kind": "code",
                    "after": ["Show the next task on My work"],
                    "files": ["crates/buzz-org-agent/src/plan.rs"],
                    "produces": ["done work opens the next task"],
                    "how": ["Read the done event for the held task.", "Publish the ticket whose after names that task."]
                }
            ]
        });
        let steps = steps_ready("Continuous task flow", &raw).expect("steps");
        assert_eq!(steps.len(), 2);
        assert!(steps[0].after.is_empty());
        assert_eq!(steps[1].after, vec!["Show the next task on My work"]);
        let allowed = vec![
            "desktop/src/features/org/work/model.ts".into(),
            "crates/buzz-org-agent/src/plan.rs".into(),
        ];
        assert!(steps_ready_in("Continuous task flow", &raw, &allowed).is_some());
        let missing = vec!["desktop/src/features/org/work/model.ts".into()];
        assert!(
            steps_ready_in("Continuous task flow", &raw, &missing).is_none(),
            "a code file outside the digest is not a plan"
        );
    }

    #[test]
    fn a_long_step_name_stays_short_and_keeps_its_order() {
        let raw = serde_json::json!({
            "steps": [
                {
                    "piece": "Define how the AI picks the next task from the direction and the work just finished",
                    "brief": "Write the rule that chooses the next task from direction and the work just finished.",
                    "kind": "writing",
                    "produces": ["the selection rule is written down"],
                    "how": ["Read the direction texts.", "Write the rule in one paragraph."]
                },
                {
                    "piece": "Build the suggestion that appears when a task is done",
                    "brief": "Show that suggestion on the ticket that was just marked done.",
                    "kind": "code",
                    "files": ["desktop/src/features/org/ui/work/WorkItemView.tsx"],
                    "after": ["Define how the AI picks the next task from the direction and the work just finished"],
                    "produces": ["the suggestion is visible on the done ticket"],
                    "how": ["Subscribe to the done event.", "Render the suggestion under the ticket."]
                }
            ]
        });
        let steps = steps_ready("Continuous task flow", &raw).expect("steps");
        assert!(steps[0].piece.split_whitespace().count() <= 8);
        assert_eq!(steps[1].after, vec![steps[0].piece.clone()]);
    }

    #[test]
    fn a_step_keeps_how_to_do_it() {
        let raw = serde_json::json!({
            "steps": [
                {
                    "piece": "Write the selection rule",
                    "brief": "Write the rule that chooses which task is next.",
                    "kind": "writing",
                    "produces": ["the rule is written"],
                    "how": ["Write the rule in one paragraph.", "Name what the screen shows."]
                },
                {
                    "piece": "Show the suggestion",
                    "brief": "Show the suggestion on My work when a task is done.",
                    "kind": "code",
                    "files": ["desktop/src/features/org/ui/work/WorkItemView.tsx"],
                    "after": ["Write the selection rule"],
                    "produces": ["the suggestion is visible"],
                    "how": ["Read the selection rule.", "Render it under the done task."]
                }
            ]
        });
        let steps = steps_ready("Continuous task flow", &raw).expect("steps");
        assert_eq!(
            steps[0].how,
            vec![
                "Write the rule in one paragraph.",
                "Name what the screen shows."
            ]
        );
        assert_eq!(steps[1].how.len(), 2);
    }

    fn offerable(piece: &str, kind: &str, after: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "piece": piece,
            "brief": format!("Do the {piece} work for this project and record the result."),
            "kind": kind,
            "gate": after.is_empty(),
            "after": after,
            "produces": [format!("{piece} has a recorded result")],
            "how": [
                format!("Open the notes for {piece}."),
                format!("Write down what {piece} found.")
            ]
        })
    }

    #[test]
    fn accept_publishes_every_step_including_ones_that_wait() {
        use buzz_core::intelligent_org::{
            DecisionRule, Executed, Proposal, ProposalKind, ProposalStatus,
        };

        let gen = "ab".repeat(32);
        let mut root = crate::test_support::root_held(&"aa".repeat(32));
        root.title = "Weekday hall trial".into();
        root.due_at = 1_785_488_400;
        let mut state = OrgState::new();
        state.item_generations.insert(root.id.clone(), gen.clone());
        state.items.insert(root.id.clone(), root);
        state.proposals.insert(
            "prop-1".into(),
            Proposal {
                id: "prop-1".into(),
                kind: ProposalKind::Project,
                status: ProposalStatus::Passed,
                opened_by: "aa".repeat(32),
                opened_at: 1,
                expires_at: 2,
                draft: None,
                payload: serde_json::json!({
                    "title": "Weekday hall trial",
                    "brief": "A four-week trial.",
                    "due_at": 1785488400,
                    "plan": [
                        offerable("Evening licence application", "writing", &[]),
                        offerable("Hygiene certificate", "ops", &[]),
                        offerable("Book four Tuesdays", "ops", &["Evening licence application"])
                    ]
                }),
                rule: DecisionRule::MAJORITY,
                needed: 1,
                eligible: vec![],
                votes: vec![],
                decided_at: Some(2),
                executed: Some(Executed {
                    kind: "work_item".into(),
                    id: "root-1".into(),
                }),
                settlement: None,
            },
        );
        let keys = nostr::Keys::generate();
        let events = take_due_tickets(&mut state, &keys).expect("tickets");
        let tickets: Vec<serde_json::Value> = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50100)
            .map(|event| serde_json::from_str(&event.content).expect("json"))
            .collect();
        assert_eq!(tickets.len(), 3);
        assert!(tickets
            .iter()
            .any(|ticket| ticket["title"] == "Book four Tuesdays"));
        let waiting = tickets
            .iter()
            .find(|ticket| ticket["title"] == "Book four Tuesdays")
            .expect("waiting");
        assert_eq!(waiting["waits_on"][0], "Evening licence application");
        assert!(waiting["how"]
            .as_array()
            .is_some_and(|lines| lines.len() >= 2));
        assert_ne!(waiting["done_when"][0], waiting["title"]);
        let prompts = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50104)
            .count();
        assert_eq!(prompts, 3);
        state.plans.clear();
        let proposal = state.proposals.get_mut("prop-1").expect("proposal");
        proposal
            .payload
            .as_object_mut()
            .expect("object")
            .remove("plan");
        let recovered = plan_for_root(&state, "root-1").expect("recovered from drafts");
        assert!(plan_ready(&recovered));
        assert!(recovered
            .iter()
            .any(|step| step.piece == "Book four Tuesdays"));
        let waiting = recovered
            .iter()
            .find(|step| step.piece == "Book four Tuesdays")
            .expect("waiting step");
        assert!(waiting.how.len() >= 2);
        assert_eq!(
            waiting.after,
            vec!["Evening licence application".to_string()]
        );
        let again = take_due_tickets(&mut state, &keys).expect("quiet");
        assert!(
            again.is_empty(),
            "a second pass does not draft the same steps"
        );
    }

    #[test]
    fn accept_drops_a_code_file_outside_the_digest() {
        use buzz_core::intelligent_org::{
            DecisionRule, Executed, ProjectHome, Proposal, ProposalKind, ProposalStatus,
        };

        let gen = "ab".repeat(32);
        let mut root = crate::test_support::root_held(&"aa".repeat(32));
        root.title = "Show the next task".into();
        root.home = Some(ProjectHome {
            channel: "room-1".into(),
            repo: Some("30617:aa:buzz".into()),
            project: Some("30621:aa:buzz".into()),
        });
        let mut state = OrgState::new();
        state.item_generations.insert(root.id.clone(), gen);
        state.items.insert(root.id.clone(), root);
        remember_digest(
            &mut state,
            "30617:aa:buzz",
            "abc1234",
            &[
                "desktop/src/features/org/work/model.ts".into(),
                "crates/buzz-org-agent/src/plan.rs".into(),
            ],
        );
        state.proposals.insert(
            "prop-1".into(),
            Proposal {
                id: "prop-1".into(),
                kind: ProposalKind::Project,
                status: ProposalStatus::Passed,
                opened_by: "aa".repeat(32),
                opened_at: 1,
                expires_at: 2,
                draft: None,
                payload: serde_json::json!({
                    "title": "Show the next task",
                    "brief": "The next offered task is visible on My work.",
                    "plan": [
                        {
                            "piece": "Render the next task",
                            "brief": "Render the next offered task in the My work column.",
                            "kind": "code",
                            "files": ["desktop/src/features/org/work/model.ts"],
                            "produces": ["the next task is visible on My work"],
                            "how": ["Open the My work column.", "Render the next offered task there."]
                        },
                        {
                            "piece": "Offer the waiting task",
                            "brief": "Offer the task that was waiting when the first one is done.",
                            "kind": "code",
                            "after": ["Render the next task"],
                            "files": ["src/missing.rs"],
                            "produces": ["done work opens the next task"],
                            "how": ["Read the done event.", "Offer the task that names it in after."]
                        }
                    ]
                }),
                rule: DecisionRule::MAJORITY,
                needed: 1,
                eligible: vec![],
                votes: vec![],
                decided_at: Some(2),
                executed: Some(Executed {
                    kind: "work_item".into(),
                    id: "root-1".into(),
                }),
                settlement: None,
            },
        );
        let keys = nostr::Keys::generate();
        let events = take_due_tickets(&mut state, &keys).expect("tickets");
        assert!(
            events.is_empty(),
            "a code file outside the digest publishes no ticket"
        );
    }

    #[test]
    fn the_buzz_repo_digest_is_the_allow_list() {
        let digest: serde_json::Value =
            serde_json::from_str(include_str!("../tests/eval/digests/buzz.json")).expect("digest");
        let files: Vec<String> = digest["files"]
            .as_array()
            .expect("files")
            .iter()
            .filter_map(|path| path.as_str().map(str::to_string))
            .collect();
        assert!(files.len() <= 80);
        assert!(files.iter().any(|path| path.ends_with("plan.rs")));
        assert!(!digest["commit"].as_str().unwrap_or("").is_empty());
        let raw = serde_json::json!({
            "steps": [
                {
                    "piece": "Ground code steps in the digest",
                    "brief": "Keep a code step only when its file is in the digest.",
                    "kind": "code",
                    "files": ["crates/buzz-org-agent/src/plan.rs"],
                    "produces": ["a code step outside the digest is not a ticket"],
                    "how": ["Read the digest file list.", "Drop a path the list does not contain."]
                },
                {
                    "piece": "Show the waiting step",
                    "brief": "Show a step that waits on the digest check.",
                    "kind": "code",
                    "after": ["Ground code steps in the digest"],
                    "files": ["desktop/src/features/org/work/model.ts"],
                    "produces": ["the waiting step is on the project"],
                    "how": ["Open the project page.", "Render the step that names its wait."]
                }
            ]
        });
        assert!(steps_ready_in("Digest allow list", &raw, &files).is_some());
        let mut missing = raw;
        missing["steps"][1]["files"] = serde_json::json!(["src/missing.rs"]);
        assert!(steps_ready_in("Digest allow list", &missing, &files).is_none());
    }

    #[test]
    fn a_vacuous_title_publishes_nothing() {
        let listed: Vec<String> =
            serde_json::from_str(include_str!("../tests/eval/cases/vacuous-titles.json"))
                .expect("vacuous titles");
        for title in &listed {
            assert!(vacuous_title(title), "{title}");
        }
        let raw = serde_json::json!({
            "steps": [
                offerable("Do research", "research", &[]),
                offerable("Count the weekday buyers", "ops", &["Do research"])
            ]
        });
        assert!(steps_ready("Weekday hall trial", &raw).is_none());
        let mut state = held_root("Weekday hall trial");
        state.proposals.insert(
            "prop-1".into(),
            passed_plan(
                "Weekday hall trial",
                &serde_json::json!([
                    offerable("Do research", "research", &[]),
                    offerable("Count the weekday buyers", "ops", &["Do research"])
                ]),
            ),
        );
        let keys = nostr::Keys::generate();
        let events = take_due_tickets(&mut state, &keys).expect("tickets");
        assert!(events.is_empty(), "a generic title publishes no ticket");
    }

    #[test]
    fn accept_names_the_digest_commit_on_the_code_prompt() {
        let paths = vec![
            "desktop/src/features/org/work/model.ts".to_string(),
            "crates/buzz-org-agent/src/plan.rs".to_string(),
        ];
        let mut state = held_root("Show the next task");
        state.items.get_mut("root-1").expect("root").home =
            Some(buzz_core::intelligent_org::ProjectHome {
                channel: "room-1".into(),
                repo: Some("30617:aa:buzz".into()),
                project: Some("30621:aa:buzz".into()),
            });
        remember_digests_in(
            &mut state,
            &digest_block("30617:aa:buzz", "abc1234", &paths),
        );
        state.proposals.insert(
            "prop-1".into(),
            passed_plan(
                "Show the next task",
                &serde_json::json!([
                    {
                        "piece": "Render the next task",
                        "brief": "Render the next offered task in the My work column.",
                        "kind": "code",
                        "files": ["desktop/src/features/org/work/model.ts"],
                        "produces": ["the next task is visible on My work"],
                        "how": ["Open the My work column.", "Render the next offered task there."]
                    },
                    {
                        "piece": "Offer the waiting task",
                        "brief": "Offer the task that was waiting when the first one is done.",
                        "kind": "code",
                        "after": ["Render the next task"],
                        "files": ["crates/buzz-org-agent/src/plan.rs"],
                        "produces": ["done work opens the next task"],
                        "how": ["Read the done event.", "Offer the task that names it in after."]
                    }
                ]),
            ),
        );
        let keys = nostr::Keys::generate();
        let events = take_due_tickets(&mut state, &keys).expect("tickets");
        let prompts: Vec<_> = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50104)
            .collect();
        assert_eq!(prompts.len(), 2);
        for prompt in prompts {
            assert!(
                prompt.content.contains("30617:aa:buzz@abc1234"),
                "{}",
                prompt.content
            );
            assert!(
                prompt
                    .content
                    .contains("desktop/src/features/org/work/model.ts")
                    || prompt.content.contains("crates/buzz-org-agent/src/plan.rs"),
                "{}",
                prompt.content
            );
            assert!(prompt.tags.iter().any(|tag| {
                let row = tag.as_slice();
                row.first().map(String::as_str) == Some("based_on")
                    && row.get(2).map(String::as_str) == Some("abc1234")
            }));
        }
    }

    #[test]
    fn a_missing_skill_says_nobody_here_can_hold_it() {
        let mut state = held_root("Certify the hall electrics");
        state.proposals.insert(
            "prop-1".into(),
            passed_plan(
                "Certify the hall electrics",
                &serde_json::json!([
                    {
                        "piece": "Book the certified electrician",
                        "brief": "Find who can sign the hall's electrical certificate.",
                        "kind": "outreach",
                        "requires": ["electrical-certification"],
                        "produces": ["a named electrician has agreed to visit"],
                        "how": ["Ask the members who can certify electrics.", "Write down who agreed and when they can come."]
                    },
                    {
                        "piece": "Walk the hall with them",
                        "brief": "Walk the circuits the certificate will cover.",
                        "kind": "ops",
                        "after": ["Book the certified electrician"],
                        "requires": ["electrical-certification"],
                        "produces": ["the certificate lists every circuit"],
                        "how": ["Open each distribution board.", "Mark the circuits the certificate covers."]
                    }
                ]),
            ),
        );
        let keys = nostr::Keys::generate();
        let events = take_due_tickets(&mut state, &keys).expect("tickets");
        let tickets: Vec<serde_json::Value> = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50100)
            .map(|event| serde_json::from_str(&event.content).expect("json"))
            .collect();
        assert_eq!(tickets.len(), 2);
        for ticket in &tickets {
            assert!(ticket["suggested_holder"].is_null(), "{ticket}");
            let unfilled = ticket["unfilled"].as_str().unwrap_or("");
            assert!(unfilled.contains("nobody here can hold it"), "{unfilled}");
            assert!(unfilled.contains("electrical-certification"), "{unfilled}");
        }
    }

    #[test]
    fn a_member_with_the_skill_is_the_suggested_holder() {
        let electrician = "cc".repeat(32);
        let mut state = held_root("Certify the hall electrics");
        state.profiles.insert(
            electrician.clone(),
            buzz_core::intelligent_org::OrgProfile {
                pubkey: electrician.clone(),
                version: 1,
                about: "I certify hall electrics.".into(),
                skills: vec![buzz_core::intelligent_org::Skill {
                    slug: "electrical-certification".into(),
                    label: "electrical certification".into(),
                }],
                socials: vec![],
                open_limit: Some(2),
                updated_at: 1,
                receipt: "dd".repeat(32),
            },
        );
        state.proposals.insert(
            "prop-1".into(),
            passed_plan(
                "Certify the hall electrics",
                &serde_json::json!([
                    {
                        "piece": "Book the certified electrician",
                        "brief": "Find who can sign the hall's electrical certificate.",
                        "kind": "outreach",
                        "requires": ["electrical certification"],
                        "produces": ["a named electrician has agreed to visit"],
                        "how": ["Ask the members who can certify electrics.", "Write down who agreed and when they can come."]
                    },
                    {
                        "piece": "Walk the hall with them",
                        "brief": "Walk the circuits the certificate will cover.",
                        "kind": "ops",
                        "after": ["Book the certified electrician"],
                        "produces": ["the certificate lists every circuit"],
                        "how": ["Open each distribution board.", "Mark the circuits the certificate covers."]
                    }
                ]),
            ),
        );
        let keys = nostr::Keys::generate();
        let events = take_due_tickets(&mut state, &keys).expect("tickets");
        let tickets: Vec<serde_json::Value> = events
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50100)
            .map(|event| serde_json::from_str(&event.content).expect("json"))
            .collect();
        let booked = tickets
            .iter()
            .find(|ticket| ticket["title"] == "Book the certified electrician")
            .expect("booked");
        assert_eq!(booked["suggested_holder"], electrician);
        assert!(booked["unfilled"].is_null());
        let walk = tickets
            .iter()
            .find(|ticket| ticket["title"] == "Walk the hall with them")
            .expect("walk");
        assert_eq!(walk["suggested_holder"], "aa".repeat(32));
    }

    fn held_root(title: &str) -> OrgState {
        let mut root = crate::test_support::root_held(&"aa".repeat(32));
        root.title = title.into();
        root.due_at = 1_785_488_400;
        let mut state = OrgState::new();
        state
            .item_generations
            .insert(root.id.clone(), "ab".repeat(32));
        state.items.insert(root.id.clone(), root);
        state
    }

    fn passed_plan(title: &str, plan: &serde_json::Value) -> buzz_core::intelligent_org::Proposal {
        use buzz_core::intelligent_org::{
            DecisionRule, Executed, Proposal, ProposalKind, ProposalStatus,
        };
        Proposal {
            id: "prop-1".into(),
            kind: ProposalKind::Project,
            status: ProposalStatus::Passed,
            opened_by: "aa".repeat(32),
            opened_at: 1,
            expires_at: 2,
            draft: None,
            payload: serde_json::json!({
                "title": title,
                "brief": "The work this project actually does.",
                "plan": plan
            }),
            rule: DecisionRule::MAJORITY,
            needed: 1,
            eligible: vec![],
            votes: vec![],
            decided_at: Some(2),
            executed: Some(Executed {
                kind: "work_item".into(),
                id: "root-1".into(),
            }),
            settlement: None,
        }
    }
}
