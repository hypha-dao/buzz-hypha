//! One unit test per judge gate (Org agent § 9.1). Each draft fails
//! exactly that gate.

use buzz_core::intelligent_org::{
    CoveragePiece, DraftKind, DraftOutcome, DraftOutcomeStatus, DraftPayload, GroundedSentence,
    HealthBand, HealthFactor, HealthRead, HolderMatch, LineOp, ObjectivesDraft, WorkItemState,
};
use serde_json::json;

use crate::judge::{self, Draft, JudgeReason};
use crate::state::OrgState;
use crate::test_support::{item, passing_ticket, root_held, work_item_event};
use crate::think::context::{Candidate, ContextBundle};

fn state_with_root() -> OrgState {
    let mut state = OrgState::new();
    state.mark_live();
    let _ = state
        .apply(&work_item_event(&root_held("holder")))
        .expect("root");
    state
}

fn judge_err(state: &OrgState, bundle: &ContextBundle, draft: &Draft) -> JudgeReason {
    judge::judge(state, bundle, draft).expect_err("should fail")
}

#[test]
fn gate_schema() {
    let state = state_with_root();
    let (_, mut draft, bundle) = passing_ticket();
    draft.raw = json!("not-an-object");
    draft.payload = None;
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Schema);
}

#[test]
fn gate_receipt_outside_context() {
    let state = state_with_root();
    let (_, mut draft, bundle) = passing_ticket();
    draft.receipts = vec!["unknown".into()];
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::ReceiptOutsideContext
    );
}

#[test]
fn gate_unresolved_receipt() {
    let state = state_with_root();
    let (_, mut draft, mut bundle) = passing_ticket();
    bundle.ids.insert("ghost".into());
    draft.receipts = vec!["ghost".into()];
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::UnresolvedReceipt
    );
}

#[test]
fn gate_authority() {
    let state = state_with_root();
    let (_, mut draft, bundle) = passing_ticket();
    draft.needs = "stranger".into();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Authority);
}

#[test]
fn gate_date() {
    let state = state_with_root();
    let (mut ticket, mut draft, bundle) = passing_ticket();
    ticket.due_at = 1;
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Date);
}

#[test]
fn gate_duplicate() {
    let mut state = state_with_root();
    state.drafts.insert(
        "open1".into(),
        crate::state::DraftRecord {
            event_id: "open1".into(),
            kind: DraftKind::Ticket,
            payload: passing_ticket().1.payload.clone().unwrap(),
            needs: "holder".into(),
            gap: "ticket:covers".into(),
            outcome: Some(DraftOutcome {
                draft: "open1".into(),
                status: DraftOutcomeStatus::Open,
                decided_by: None,
                decided_at: None,
                reason: None,
                result: None,
            }),
        },
    );
    let (_, draft, bundle) = passing_ticket();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Duplicate);
}

#[test]
fn gate_nag() {
    let state = state_with_root();
    let (_, draft, mut bundle) = passing_ticket();
    bundle.fingerprint = Some("same".into());
    bundle.declined_fingerprint = Some("same".into());
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Nag);
}

#[test]
fn gate_self_contradiction() {
    let state = state_with_root();
    let (mut ticket, mut draft, bundle) = passing_ticket();
    ticket.coverage = vec![CoveragePiece {
        piece: "covers".into(),
        covered_by: Some("other".into()),
        order: 1,
        after: vec![],
        held: None,
    }];
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::SelfContradiction
    );
}

#[test]
fn gate_not_candidate() {
    let state = state_with_root();
    let (mut ticket, mut draft, bundle) = passing_ticket();
    ticket.suggested_holder = Some("cc".repeat(32));
    ticket.matched = Some(HolderMatch::default());
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::NotCandidate
    );
}

#[test]
fn gate_unmatched_skill() {
    let state = state_with_root();
    let (mut ticket, mut draft, mut bundle) = passing_ticket();
    let pk = "dd".repeat(32);
    ticket.suggested_holder = Some(pk.clone());
    ticket.matched = Some(HolderMatch {
        skills: vec!["welding".into()],
        about: None,
        items: vec![],
    });
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    bundle.candidates.push(Candidate {
        pubkey: pk,
        skills: vec!["carpentry".into()],
        items: vec![],
        open_count: 0,
        open_limit: None,
        is_member: true,
        profile_version: 1,
    });
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::UnmatchedSkill
    );
}

#[test]
fn gate_holder_at_limit() {
    let state = state_with_root();
    let (mut ticket, mut draft, mut bundle) = passing_ticket();
    let pk = "ee".repeat(32);
    ticket.suggested_holder = Some(pk.clone());
    ticket.matched = Some(HolderMatch::default());
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    bundle.candidates.push(Candidate {
        pubkey: pk,
        skills: vec![],
        items: vec![],
        open_count: 2,
        open_limit: Some(2),
        is_member: true,
        profile_version: 1,
    });
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::HolderAtLimit
    );
}

#[test]
fn gate_forbidden_field() {
    let state = state_with_root();
    let (_, mut draft, bundle) = passing_ticket();
    draft.raw["dri"] = json!("someone");
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::ForbiddenField
    );
}

#[test]
fn gate_open_children() {
    let mut state = state_with_root();
    let child = item(
        "child-1",
        Some("root-1"),
        WorkItemState::Accepted,
        Some("holder"),
    );
    let _ = state.apply(&work_item_event(&child)).expect("child");
    let (_, mut draft, bundle) = passing_ticket();
    draft.kind = Some(DraftKind::Done);
    draft.item = Some("root-1".into());
    draft.payload = Some(DraftPayload::Done(buzz_core::intelligent_org::DoneDraft {
        item: "root-1".into(),
        why: "last child".into(),
        heard: None,
    }));
    draft.raw = json!({"item":"root-1","why":"last child","heard":null});
    draft.needs = "holder".into();
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::OpenChildren
    );
}

#[test]
fn gate_health_grounding() {
    let state = state_with_root();
    let health = HealthRead {
        item: "root-1".into(),
        week: "2026-W12".into(),
        pct: 0.5,
        band: HealthBand::Healthy,
        factors: vec![HealthFactor {
            name: "overdue".into(),
            value: 1.0,
            weight: 1.0,
            rows: vec!["rec1".into()],
        }],
        sentences: vec![GroundedSentence {
            text: "It is fine".into(),
            rows: vec![],
        }],
        formula: "health-weights@1".into(),
    };
    let draft = Draft {
        kind: None,
        payload: None,
        health: Some(health),
        raw: json!({}),
        needs: "shaper".into(),
        gap: String::new(),
        receipts: vec!["rec1".into()],
        generation: "gen1".into(),
        parent: None,
        item: Some("root-1".into()),
    };
    let mut bundle = ContextBundle {
        generation: "gen1".into(),
        expected_band: Some(HealthBand::Healthy),
        ..ContextBundle::default()
    };
    bundle.ids.insert("rec1".into());
    bundle.resolved.insert("rec1".into());
    assert_eq!(
        judge_err(&state, &bundle, &draft),
        JudgeReason::HealthGrounding
    );
}

#[test]
fn gate_redraw_shape() {
    let mut state = state_with_root();
    state.direction.insert(
        "objectives".into(),
        crate::state::DirectionHead {
            artifact: buzz_core::intelligent_org::DirectionArtifact {
                slug: buzz_core::intelligent_org::DirectionSlug::Objectives,
                version: 2,
                body: "b".into(),
                lines: vec![buzz_core::intelligent_org::DirectionLine {
                    n: 1,
                    id: "l1".into(),
                    text: "line".into(),
                    date: None,
                }],
                confirmed_by: "aa".repeat(32),
                confirmed_at: 1,
                proposed_by: "aa".repeat(32),
                proposal: "p1".into(),
                prev: None,
            },
            event_id: "dir1".into(),
        },
    );
    let ops = ObjectivesDraft {
        base_version: 2,
        ops: vec![LineOp::Strike {
            id: "missing".into(),
            why: "no".into(),
        }],
    };
    let draft = Draft {
        kind: Some(DraftKind::Objectives),
        payload: Some(DraftPayload::Objectives(ops.clone())),
        health: None,
        raw: serde_json::to_value(&ops).unwrap(),
        needs: "shaper".into(),
        gap: "obj".into(),
        receipts: vec!["rec1".into()],
        generation: "gen1".into(),
        parent: None,
        item: None,
    };
    let (_, _, bundle) = passing_ticket();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::RedrawShape);
}

#[test]
fn gate_stale() {
    let state = state_with_root();
    let (_, draft, mut bundle) = passing_ticket();
    bundle.generation = "newer".into();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Stale);
}

#[test]
fn gate_too_long() {
    let state = state_with_root();
    let (mut ticket, mut draft, bundle) = passing_ticket();
    ticket.brief = (0..50)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::TooLong);
}

#[test]
fn gate_sequence() {
    let state = state_with_root();
    let (mut ticket, mut draft, bundle) = passing_ticket();
    ticket.after = vec!["not-a-sibling".into()];
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Sequence);
}

#[test]
fn gate_unfilled() {
    let state = state_with_root();
    let (mut ticket, mut draft, bundle) = passing_ticket();
    ticket.requires = vec!["spanish".into()];
    ticket.suggested_holder = None;
    ticket.unfilled = None;
    draft.payload = Some(DraftPayload::Ticket(ticket.clone()));
    draft.raw = serde_json::to_value(&ticket).unwrap();
    assert_eq!(judge_err(&state, &bundle, &draft), JudgeReason::Unfilled);
}

#[test]
fn passing_ticket_clears_every_gate() {
    let state = state_with_root();
    let (_, draft, bundle) = passing_ticket();
    judge::judge(&state, &bundle, &draft).expect("pass");
}
