//! One review when a root enters `in_review`.
//!
//! The compiler picks the next step. The model writes the why. A follow-up
//! names the same objective. Stop and an objectives redraw carry no project.

use buzz_core::intelligent_org::{
    GroundedSentence, ObjectivesDraft, PlanStep, ProjectDraft, ProposalKind, ReviewDraft,
    ReviewRecommendation, WorkItemState,
};
use serde_json::Value;

use crate::compile::{self, ObjectiveCard};
use crate::plan::{judge_change_plan, PlanReject};
use crate::state::OrgState;
use crate::think::model::{ModelRequest, Tier};

/// The one next step. The compiler chooses it. The model does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewNext {
    /// Every promised check is done. Open another project on the same line.
    FollowUp,
    /// Some promised checks are done. Redraw the objective lines.
    Redraw,
    /// None of the promised checks are done.
    Stop,
}

/// Why a review was not published. No model call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewReject {
    /// The model output is not one recommendation with a why.
    Shape,
    /// The recommendation is not the compiler's next step.
    WrongNext,
    /// Stop or a redraw included a project payload.
    ProjectOnStop,
    /// The follow-up names a different objective.
    Objective,
    /// The follow-up plan failed the change-plan judge.
    Plan(PlanReject),
}

/// Which step the compiler allows for this root.
pub fn review_next(state: &OrgState, root_id: &str) -> ReviewNext {
    let Some(payload) = project_payload(state, root_id) else {
        return ReviewNext::Stop;
    };
    let promised = promised_lines(payload);
    if promised.is_empty() {
        return ReviewNext::Stop;
    }
    let steps = plan_steps(payload);
    let met = promised
        .iter()
        .filter(|line| line_is_met(state, root_id, &steps, line))
        .count();
    if met == promised.len() {
        ReviewNext::FollowUp
    } else if met == 0 {
        ReviewNext::Stop
    } else {
        ReviewNext::Redraw
    }
}

/// Judge the model's explanation. The next step stays the compiler's.
pub fn judge_review(
    state: &OrgState,
    root_id: &str,
    raw: &Value,
) -> Result<ReviewDraft, ReviewReject> {
    let next = review_next(state, root_id);
    let asked = recommendation_name(raw).ok_or(ReviewReject::Shape)?;
    if asked != next {
        return Err(ReviewReject::WrongNext);
    }
    let why = raw
        .get("why")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_owned();
    if why.is_empty() {
        return Err(ReviewReject::Shape);
    }
    let recommendation = match next {
        ReviewNext::Stop => {
            if raw.get("project").is_some() || raw.get("objectives").is_some() {
                return Err(ReviewReject::ProjectOnStop);
            }
            ReviewRecommendation::NoFurtherWork { why: why.clone() }
        }
        ReviewNext::Redraw => {
            if raw.get("project").is_some() {
                return Err(ReviewReject::ProjectOnStop);
            }
            let objectives: ObjectivesDraft = raw
                .get("objectives")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok())
                .ok_or(ReviewReject::Shape)?;
            if objectives.ops.is_empty() {
                return Err(ReviewReject::Shape);
            }
            let version = state
                .direction
                .get("objectives")
                .map(|head| head.artifact.version)
                .unwrap_or(0);
            if objectives.base_version != version {
                return Err(ReviewReject::Shape);
            }
            ReviewRecommendation::ObjectivesRedraw {
                objectives: Box::new(objectives),
            }
        }
        ReviewNext::FollowUp => {
            if raw.get("objectives").is_some() {
                return Err(ReviewReject::Shape);
            }
            let project_value = raw.get("project").cloned().ok_or(ReviewReject::Shape)?;
            let line_ref = state
                .items
                .get(root_id)
                .and_then(|item| item.objective_ref.clone())
                .ok_or(ReviewReject::Objective)?;
            let project: ProjectDraft =
                serde_json::from_value(project_value.clone()).map_err(|_| ReviewReject::Shape)?;
            if project.objective_ref.as_deref() != Some(line_ref.as_str()) {
                return Err(ReviewReject::Objective);
            }
            let card = objective_card(state, &line_ref).ok_or(ReviewReject::Objective)?;
            judge_change_plan(state, &card, &project_value).map_err(ReviewReject::Plan)?;
            ReviewRecommendation::FollowUp {
                project: Box::new(project),
            }
        }
    };
    Ok(ReviewDraft {
        item: root_id.to_owned(),
        brief: brief_for(state, root_id, &why),
        recommendation,
    })
}

/// The structured call. `decisions` is `context/decisions.md` when the repo has it.
pub fn review_request(state: &OrgState, root_id: &str, decisions: Option<&str>) -> ModelRequest {
    let next = match review_next(state, root_id) {
        ReviewNext::FollowUp => "follow_up",
        ReviewNext::Redraw => "objectives_redraw",
        ReviewNext::Stop => "stop",
    };
    let card = brief_for(state, root_id, "");
    let promised = card
        .first()
        .map(|sentence| sentence.text.as_str())
        .unwrap_or("Promised: nothing checkable");
    let happened = card
        .get(1)
        .map(|sentence| sentence.text.as_str())
        .unwrap_or("Happened: No steps were on the plan.");
    let mut content = format!(
        "Root {root_id}. The next step is {next}. \
         {promised}. {happened}. \
         Write why in one line. Do not choose a different step. \
         A stop returns recommendation \"stop\" and no project. \
         An objectives redraw returns recommendation \"objectives_redraw\", \
         objectives with base_version and ops, and no project. \
         A follow-up returns recommendation \"follow_up\" and a project \
         that names this root's objective."
    );
    if let Some(body) = decisions {
        content.push('\n');
        content.push_str(&compile::fence_untrusted("context/decisions.md", body));
    }
    ModelRequest {
        tier: Tier::Draft,
        system: "Explain the review. The next step is already decided.".into(),
        messages: vec![crate::think::model::Message {
            role: "user".into(),
            content,
        }],
        schema: serde_json::json!({ "type": "object" }),
        tool_name: "emit_review".into(),
        tools: vec![],
        temperature: 0.2,
        max_output_tokens: 1200,
    }
}

fn brief_for(state: &OrgState, root_id: &str, why: &str) -> Vec<GroundedSentence> {
    let payload = project_payload(state, root_id);
    let promised = payload.map(promised_lines).unwrap_or_default();
    let steps = payload.map(plan_steps).unwrap_or_default();
    let happened = if steps.is_empty() {
        "No steps were on the plan.".to_owned()
    } else {
        steps
            .iter()
            .map(|step| {
                let status = if step_is_done(state, root_id, &step.piece) {
                    "done"
                } else {
                    "not done"
                };
                format!("{} ({status})", step.piece)
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let next = match review_next(state, root_id) {
        ReviewNext::FollowUp => "follow-up project",
        ReviewNext::Redraw => "objectives redraw",
        ReviewNext::Stop => "stop",
    };
    let promised_text = if promised.is_empty() {
        "nothing checkable".to_owned()
    } else {
        promised.join("; ")
    };
    vec![
        GroundedSentence {
            text: format!("Promised: {promised_text}"),
            rows: vec![],
        },
        GroundedSentence {
            text: format!("Happened: {happened}"),
            rows: vec![],
        },
        GroundedSentence {
            text: format!("Next: {next}. {why}"),
            rows: vec![],
        },
    ]
}

fn recommendation_name(raw: &Value) -> Option<ReviewNext> {
    let name = raw
        .get("recommendation")
        .and_then(Value::as_str)
        .or_else(|| {
            raw.get("recommendation")
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str)
        })?;
    match name {
        "follow_up" => Some(ReviewNext::FollowUp),
        "objectives_redraw" => Some(ReviewNext::Redraw),
        "stop" | "no_further_work" => Some(ReviewNext::Stop),
        _ => None,
    }
}

fn project_payload<'a>(state: &'a OrgState, root_id: &str) -> Option<&'a Value> {
    state
        .proposals
        .values()
        .find(|proposal| {
            proposal.kind == ProposalKind::Project
                && proposal
                    .executed
                    .as_ref()
                    .is_some_and(|executed| executed.id == root_id)
        })
        .map(|proposal| &proposal.payload)
}

fn promised_lines(payload: &Value) -> Vec<String> {
    payload
        .get("change")
        .and_then(|change| change.get("done_when"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn plan_steps(payload: &Value) -> Vec<PlanStep> {
    payload
        .get("plan")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn line_is_met(state: &OrgState, root_id: &str, steps: &[PlanStep], line: &str) -> bool {
    let want = line.trim();
    steps.iter().any(|step| {
        step.produces.iter().any(|produced| produced.trim() == want)
            && step_is_done(state, root_id, &step.piece)
    })
}

fn step_is_done(state: &OrgState, root_id: &str, piece: &str) -> bool {
    state.items.values().any(|item| {
        item.parent.as_deref() == Some(root_id)
            && item.title == piece
            && item.state == WorkItemState::Done
    })
}

fn objective_card(state: &OrgState, line_ref: &str) -> Option<ObjectiveCard> {
    let rest = line_ref.strip_prefix("objectives@")?;
    let (version, line_id) = rest.split_once('#')?;
    let version: u32 = version.parse().ok()?;
    compile::compile(state)
        .objectives
        .into_iter()
        .find(|card| card.version == version && card.line_id == line_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{fixtures_dir, load_events};
    use crate::state::OrgState;
    use buzz_core::intelligent_org::{
        ChildrenCounts, Executed, Proposal, ProposalStatus, StrategyLineType, WorkItem,
    };

    fn river_trial(done: &[&str]) -> (OrgState, String) {
        let events = load_events(&fixtures_dir().join("orgs/river/seed.json")).expect("river");
        let mut state = OrgState::from_events(&events).expect("apply");
        let head = state.direction.get_mut("strategy").expect("strategy");
        for line in &mut head.artifact.lines {
            let text = line.text.to_lowercase();
            if text.contains("not a grant") || text.starts_with("no ") {
                line.line_type = Some(StrategyLineType::Refusal);
            }
        }
        let version = state
            .direction
            .get("objectives")
            .expect("objectives")
            .artifact
            .version;
        let line_ref = format!("objectives@{version}#l_84c9b3c2");
        let root_id = "weekday-trial".to_owned();
        state.items.insert(
            root_id.clone(),
            WorkItem {
                id: root_id.clone(),
                parent: None,
                root: root_id.clone(),
                depth: 0,
                path: vec![],
                title: "Weekday hall trial".into(),
                brief: "Four sessions, then a decision.".into(),
                state: WorkItemState::InReview,
                dri: Some("aa".repeat(32)),
                offered_to: None,
                offered_by: None,
                offered_at: None,
                created_by: None,
                offered_by_member: None,
                due_at: 1_785_488_400,
                approved_at: Some(1_700_000_000),
                objective_ref: Some(line_ref.clone()),
                created_from: "bb".repeat(32),
                draft: None,
                done_receipt: None,
                closed_by: None,
                children: ChildrenCounts::default(),
                home: None,
                branch: None,
                after: vec![],
                last_progress: None,
            },
        );
        for (index, title) in ["Evening licence application", "Count attendance"]
            .iter()
            .enumerate()
        {
            let id = format!("step-{index}");
            let state_name = if done.contains(title) {
                WorkItemState::Done
            } else {
                WorkItemState::Accepted
            };
            state.items.insert(
                id.clone(),
                WorkItem {
                    id: id.clone(),
                    parent: Some(root_id.clone()),
                    root: root_id.clone(),
                    depth: 1,
                    path: vec![root_id.clone()],
                    title: (*title).to_owned(),
                    brief: (*title).to_owned(),
                    state: state_name,
                    dri: Some("aa".repeat(32)),
                    offered_to: None,
                    offered_by: None,
                    offered_at: None,
                    created_by: None,
                    offered_by_member: None,
                    due_at: 1_785_488_400,
                    approved_at: None,
                    objective_ref: None,
                    created_from: "cc".repeat(32),
                    draft: None,
                    done_receipt: None,
                    closed_by: None,
                    children: ChildrenCounts::default(),
                    home: None,
                    branch: None,
                    after: vec![],
                    last_progress: None,
                },
            );
        }
        state.proposals.insert(
            "prop-trial".into(),
            Proposal {
                id: "prop-trial".into(),
                kind: ProposalKind::Project,
                status: ProposalStatus::Passed,
                opened_by: "aa".repeat(32),
                opened_at: 1,
                expires_at: 2,
                draft: None,
                payload: serde_json::json!({
                    "title": "Weekday hall trial",
                    "change": {
                        "from": "no weekday night",
                        "to": "a trial has answered whether weekday buyers come",
                        "done_when": ["four sessions held", "attendance counted"],
                        "moves": [line_ref]
                    },
                    "plan": [
                        { "piece": "Evening licence application", "kind": "writing", "gate": true, "produces": ["four sessions held"] },
                        { "piece": "Count attendance", "kind": "ops", "gate": true, "produces": ["attendance counted"] }
                    ]
                }),
                rule: Default::default(),
                needed: 1,
                eligible: vec!["aa".repeat(32)],
                votes: vec![],
                decided_at: Some(1),
                executed: Some(Executed {
                    kind: "work_item".into(),
                    id: root_id.clone(),
                }),
                settlement: None,
            },
        );
        (state, line_ref)
    }

    fn follow_up(line_ref: &str, verdict: &str) -> Value {
        serde_json::json!({
            "recommendation": "follow_up",
            "why": "the trial answered whether weekday buyers come",
            "project": {
                "title": "Second weekday night",
                "brief": "A second night now that the trial answered the question.",
                "objective_ref": line_ref,
                "due_at": 1785488400,
                "suggested_dri": null,
                "why": "the trial answered whether weekday buyers come",
                "gaps": [],
                "gap": { "ref": line_ref, "verdict": verdict },
                "options": [
                    { "title": "Second weekday night", "mechanism": "keeps the night the trial filled", "kept": true },
                    { "title": "Apply for a city grant", "mechanism": "a grant round pays the hall", "kept": false, "why_not": "strategy refusal" }
                ],
                "change": {
                    "from": "one trial and no second night",
                    "to": "a second weekday night is open",
                    "done_when": ["four sessions held"],
                    "moves": [line_ref]
                },
                "plan": [
                    { "piece": "Book the second night", "kind": "ops", "gate": true, "produces": ["four sessions held"] }
                ]
            }
        })
    }

    #[test]
    fn a_trial_that_answered_recommends_the_next_project() {
        let (state, line_ref) = river_trial(&["Evening licence application", "Count attendance"]);
        assert_eq!(review_next(&state, "weekday-trial"), ReviewNext::FollowUp);
        let card = objective_card(&state, &line_ref).expect("line");
        let verdict = match card.verdict {
            compile::GapVerdict::Covered => "covered",
            compile::GapVerdict::Partly => "partly",
            compile::GapVerdict::Uncovered => "uncovered",
        };
        let draft = judge_review(&state, "weekday-trial", &follow_up(&line_ref, verdict))
            .expect("follow-up");
        match draft.recommendation {
            ReviewRecommendation::FollowUp { project } => {
                assert_eq!(project.objective_ref.as_deref(), Some(line_ref.as_str()));
            }
            other => panic!("expected a follow-up, got {other:?}"),
        }
        assert!(draft
            .brief
            .iter()
            .any(|sentence| sentence.text.starts_with("Promised:")));
        assert!(draft
            .brief
            .iter()
            .any(|sentence| sentence.text.starts_with("Happened:")));
        let stopped = judge_review(
            &state,
            "weekday-trial",
            &serde_json::json!({ "recommendation": "stop", "why": "nothing more" }),
        );
        assert_eq!(stopped, Err(ReviewReject::WrongNext));
    }

    #[test]
    fn a_trial_that_missed_recommends_stop_with_no_project() {
        let (state, line_ref) = river_trial(&[]);
        assert_eq!(review_next(&state, "weekday-trial"), ReviewNext::Stop);
        let draft = judge_review(
            &state,
            "weekday-trial",
            &serde_json::json!({ "recommendation": "stop", "why": "the trial did not answer the question" }),
        )
        .expect("stop");
        assert!(matches!(
            draft.recommendation,
            ReviewRecommendation::NoFurtherWork { .. }
        ));
        let published = serde_json::to_value(&draft).expect("json");
        assert!(published
            .get("recommendation")
            .and_then(|value| value.get("project"))
            .is_none());
        assert!(draft
            .brief
            .iter()
            .any(|sentence| sentence.text.contains("stop")));
        let followed = judge_review(&state, "weekday-trial", &follow_up(&line_ref, "covered"));
        assert_eq!(followed, Err(ReviewReject::WrongNext));
        let smuggled = judge_review(
            &state,
            "weekday-trial",
            &serde_json::json!({
                "recommendation": "stop",
                "why": "stop",
                "project": { "title": "Keep going" }
            }),
        );
        assert_eq!(smuggled, Err(ReviewReject::ProjectOnStop));
    }

    #[test]
    fn a_partial_trial_redraws_objectives_and_names_no_project() {
        let (state, _) = river_trial(&["Evening licence application"]);
        assert_eq!(review_next(&state, "weekday-trial"), ReviewNext::Redraw);
        let version = state
            .direction
            .get("objectives")
            .expect("objectives")
            .artifact
            .version;
        let draft = judge_review(
            &state,
            "weekday-trial",
            &serde_json::json!({
                "recommendation": "objectives_redraw",
                "why": "one check is met and one is not",
                "objectives": {
                    "base_version": version,
                    "ops": [{ "op": "move", "id": "l_84c9b3c2", "date": 1790000000, "why": "the trial is only half done" }]
                }
            }),
        )
        .expect("redraw");
        assert!(matches!(
            draft.recommendation,
            ReviewRecommendation::ObjectivesRedraw { .. }
        ));
        let published = serde_json::to_value(&draft).expect("json");
        assert!(published.pointer("/recommendation/project").is_none());
    }

    #[test]
    fn decisions_are_fenced_and_a_long_file_is_capped() {
        let (state, _) = river_trial(&[]);
        let bare = review_request(&state, "weekday-trial", None);
        assert!(!bare.messages[0].content.contains("untrusted"));
        assert!(bare.messages[0].content.contains("Promised:"));
        assert!(bare.messages[0]
            .content
            .contains("four sessions held; attendance counted"));
        assert!(bare.messages[0].content.contains("Happened:"));
        assert!(bare.messages[0].content.contains("not done"));
        let long = "x".repeat(20_000);
        let fenced = review_request(&state, "weekday-trial", Some(&long));
        assert!(fenced.messages[0]
            .content
            .contains("<untrusted source=\"context/decisions.md\">"));
        assert!(fenced.messages[0].content.len() < long.len());
    }
}
