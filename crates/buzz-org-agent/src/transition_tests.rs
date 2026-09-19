//! One test per row of Org agent § 5.2. Each applies through
//! [`OrgState::apply`] (the production seam) and names the transition.

use buzz_core::intelligent_org::{
    ClosedBy, CoveragePiece, DecisionRule, DecisionRules, DeclineReason, DirectionArtifact,
    DirectionSlug, DraftKind, DraftOutcome, DraftOutcomeStatus, DraftPayload, OrgProfile, Proposal,
    ProposalKind, ProposalStatus, Shapers, TicketDraft, VoteContent, WorkItem, WorkItemState,
};
use buzz_core::kind::{
    KIND_DM_OPEN, KIND_IO_DIRECTION, KIND_IO_DRAFT_OUTCOME, KIND_IO_PROFILE, KIND_IO_PROPOSAL,
    KIND_IO_SHAPERS, KIND_IO_VOTE, KIND_NIP43_MEMBERSHIP_LIST,
};
use buzz_core::Event;
use nostr::{EventBuilder, Keys, Kind, Tag};
use serde_json::json;

use crate::state::{DraftRecord, OrgState, Transition};
use crate::test_support::{item, sign, work_item_event};

fn apply_last(events: &[Event]) -> (OrgState, Vec<Transition>) {
    let mut state = OrgState::new();
    if events.len() > 1 {
        state
            .apply_backfill(&events[..events.len() - 1])
            .expect("backfill");
    }
    state.mark_live();
    let out = state
        .apply(events.last().expect("last"))
        .expect("live apply");
    (state, out)
}

fn named<F>(ts: &[Transition], pred: F) -> &Transition
where
    F: Fn(&Transition) -> bool,
{
    ts.iter()
        .find(|t| pred(t))
        .unwrap_or_else(|| panic!("missing transition in {ts:?}"))
}

fn pk(n: u8) -> String {
    format!("{n:02x}").repeat(32)
}

fn direction(slug: DirectionSlug, version: u32) -> Event {
    let artifact = DirectionArtifact {
        slug,
        version,
        body: "body".into(),
        lines: vec![],
        confirmed_by: pk(1),
        confirmed_at: 1,
        proposed_by: pk(1),
        proposal: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
        prev: None,
    };
    let content = serde_json::to_string(&artifact).expect("json");
    let slug_s = serde_json::to_value(slug)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    sign(
        KIND_IO_DIRECTION,
        &content,
        vec![
            vec!["d", &slug_s],
            vec!["version", &version.to_string()],
            vec!["p", &pk(1)],
            vec!["receipt", &artifact.proposal],
        ],
    )
}

fn proposal(status: ProposalStatus, kind: ProposalKind) -> (String, Event) {
    let id = "11111111-2222-3333-4444-555555555555".to_owned();
    let opened_by = pk(2);
    let p = Proposal {
        id: id.clone(),
        kind,
        status,
        opened_by: opened_by.clone(),
        opened_at: 1,
        expires_at: 2,
        draft: None,
        payload: json!({}),
        rule: DecisionRule::MAJORITY,
        needed: 1,
        eligible: vec![opened_by.clone()],
        votes: vec![],
        decided_at: None,
        executed: None,
        settlement: None,
    };
    let content = serde_json::to_string(&p).expect("json");
    let kind_s = serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    let status_s = serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    let receipt = "bb".repeat(32);
    let event = EventBuilder::new(Kind::Custom(KIND_IO_PROPOSAL as u16), content)
        .tags([
            Tag::parse(["d", &id]).unwrap(),
            Tag::parse(["t", &kind_s]).unwrap(),
            Tag::parse(["s", &status_s]).unwrap(),
            Tag::parse(["p", &opened_by]).unwrap(),
            Tag::parse(["p", &opened_by, "", "eligible"]).unwrap(),
            Tag::parse(["receipt", &receipt]).unwrap(),
        ])
        .allow_self_tagging()
        .sign_with_keys(&Keys::generate())
        .expect("sign");
    (id, event)
}

fn shapers(members: &[&str], agent: Option<&str>, window: u64) -> Event {
    let s = Shapers {
        founder: members[0].to_owned(),
        shapers: members.iter().map(|p| (*p).to_owned()).collect(),
        offered: vec![],
        room: Some("shapers-room".into()),
        agent: agent.map(str::to_owned),
        agent_hosted: true,
        rules: DecisionRules::default(),
        decision_window_secs: window,
        offer_window_secs: 259_200,
        updated_at: 1,
        receipt: "cc".repeat(32),
    };
    let content = serde_json::to_string(&s).expect("json");
    let mut tags: Vec<Vec<&str>> = vec![vec!["d", "shapers"]];
    for p in members {
        tags.push(vec!["p", p]);
    }
    sign(KIND_IO_SHAPERS, &content, tags)
}

fn vote_direction(proposal_id: &str) -> Event {
    let content = serde_json::to_string(&VoteContent {
        reason: Some("direction".into()),
    })
    .expect("json");
    sign(
        KIND_IO_VOTE,
        &content,
        vec![vec!["e", proposal_id], vec!["vote", "decline"]],
    )
}

#[test]
fn direction_confirmed_on_new_objectives_version() {
    let v1 = direction(DirectionSlug::Objectives, 1);
    let v2 = direction(DirectionSlug::Objectives, 2);
    let (_, ts) = apply_last(&[v1, v2]);
    match named(&ts, |t| matches!(t, Transition::DirectionConfirmed { .. })) {
        Transition::DirectionConfirmed {
            slug,
            version,
            diff,
            ..
        } => {
            assert_eq!(*slug, DirectionSlug::Objectives);
            assert_eq!(*version, 2);
            assert_eq!(diff, "v1->v2");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn direction_confirmed_on_mission_is_the_negative_harness_case() {
    let (_, ts) = apply_last(&[direction(DirectionSlug::Mission, 1)]);
    match named(&ts, |t| matches!(t, Transition::DirectionConfirmed { .. })) {
        Transition::DirectionConfirmed { slug, version, .. } => {
            assert_eq!(*slug, DirectionSlug::Mission);
            assert_eq!(*version, 1);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn root_open_unheld() {
    let root = item("root-1", None, WorkItemState::Open, None);
    let (_, ts) = apply_last(&[work_item_event(&root)]);
    named(
        &ts,
        |t| matches!(t, Transition::RootOpenUnheld { item, .. } if item == "root-1"),
    );
}

#[test]
fn offer_returned() {
    let mut offered = item("root-1", None, WorkItemState::Offered, None);
    offered.offered_to = Some(pk(3));
    let mut open = offered.clone();
    open.state = WorkItemState::Open;
    open.offered_to = None;
    let (_, ts) = apply_last(&[work_item_event(&offered), work_item_event(&open)]);
    match named(&ts, |t| matches!(t, Transition::OfferReturned { .. })) {
        Transition::OfferReturned { item, to, .. } => {
            assert_eq!(item, "root-1");
            assert_eq!(to.as_deref(), Some(pk(3).as_str()));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn holder_set() {
    let open = item("root-1", None, WorkItemState::Open, None);
    let held = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let (_, ts) = apply_last(&[work_item_event(&open), work_item_event(&held)]);
    match named(&ts, |t| matches!(t, Transition::HolderSet { .. })) {
        Transition::HolderSet { item, dri, .. } => {
            assert_eq!(item, "root-1");
            assert_eq!(dri, &pk(4));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn last_child_done() {
    let parent = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let child = item(
        "child-1",
        Some("root-1"),
        WorkItemState::Accepted,
        Some(&pk(4)),
    );
    let mut done = child.clone();
    done.state = WorkItemState::Done;
    done.closed_by = Some(ClosedBy::Dri);
    let (_, ts) = apply_last(&[
        work_item_event(&parent),
        work_item_event(&child),
        work_item_event(&done),
    ]);
    named(
        &ts,
        |t| matches!(t, Transition::LastChildDone { parent, .. } if parent == "root-1"),
    );
}

#[test]
fn child_done_brief_unmet() {
    let parent = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let child = item(
        "child-1",
        Some("root-1"),
        WorkItemState::Accepted,
        Some(&pk(4)),
    );
    let mut done = child.clone();
    done.state = WorkItemState::Done;
    done.closed_by = Some(ClosedBy::Dri);
    let mut state = OrgState::new();
    state
        .apply_backfill(&[work_item_event(&parent), work_item_event(&child)])
        .expect("backfill");
    state.drafts.insert(
        "draft1".into(),
        DraftRecord {
            event_id: "draft1".into(),
            kind: DraftKind::Ticket,
            payload: DraftPayload::Ticket(TicketDraft {
                parent: "root-1".into(),
                title: "t".into(),
                brief: "b".into(),
                due_at: 2_000_000_000,
                requires: vec![],
                suggested_holder: None,
                unfilled: None,
                covers: "covers".into(),
                after: vec![],
                gate: false,
                coverage: vec![CoveragePiece {
                    piece: "covers".into(),
                    covered_by: None,
                    order: 1,
                    after: vec![],
                    held: None,
                }],
                matched: None,
            }),
            needs: pk(4),
            gap: "ticket:covers".into(),
            outcome: None,
        },
    );
    state.mark_live();
    let ts = state.apply(&work_item_event(&done)).expect("apply");
    named(
        &ts,
        |t| matches!(t, Transition::ChildDoneBriefUnmet { parent, .. } if parent == "root-1"),
    );
}

#[test]
fn item_done() {
    let open = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let mut done = open.clone();
    done.state = WorkItemState::Done;
    done.closed_by = Some(ClosedBy::Dri);
    let (_, ts) = apply_last(&[work_item_event(&open), work_item_event(&done)]);
    match named(&ts, |t| matches!(t, Transition::ItemDone { .. })) {
        Transition::ItemDone {
            item, closed_by, ..
        } => {
            assert_eq!(item, "root-1");
            assert_eq!(*closed_by, Some(ClosedBy::Dri));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn entered_review() {
    let held = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let mut review = held.clone();
    review.state = WorkItemState::InReview;
    let (_, ts) = apply_last(&[work_item_event(&held), work_item_event(&review)]);
    named(
        &ts,
        |t| matches!(t, Transition::EnteredReview { root, .. } if root == "root-1"),
    );
}

#[test]
fn root_closed() {
    let held = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let mut done = held.clone();
    done.state = WorkItemState::Done;
    done.closed_by = Some(ClosedBy::Rule);
    let (_, ts) = apply_last(&[work_item_event(&held), work_item_event(&done)]);
    match named(&ts, |t| matches!(t, Transition::RootClosed { .. })) {
        Transition::RootClosed {
            root, closed_by, ..
        } => {
            assert_eq!(root, "root-1");
            assert_eq!(*closed_by, Some(ClosedBy::Rule));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn progress_noted() {
    let mut first = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    first.last_progress = None;
    let mut second = first.clone();
    second.last_progress = Some("dd".repeat(32));
    let (_, ts) = apply_last(&[work_item_event(&first), work_item_event(&second)]);
    named(
        &ts,
        |t| matches!(t, Transition::ProgressNoted { item, .. } if item == "root-1"),
    );
}

#[test]
fn proposal_expired() {
    let (_, open) = proposal(ProposalStatus::Open, ProposalKind::Project);
    let (_, expired) = proposal(ProposalStatus::Expired, ProposalKind::Project);
    let (_, ts) = apply_last(&[open, expired]);
    named(
        &ts,
        |t| matches!(t, Transition::ProposalExpired { proposal, .. } if proposal == "11111111-2222-3333-4444-555555555555"),
    );
}

#[test]
fn direction_rejected_with_reason() {
    let (id, open) = proposal(ProposalStatus::Open, ProposalKind::Direction);
    let vote = vote_direction(&id);
    let (_, rejected) = proposal(ProposalStatus::Rejected, ProposalKind::Direction);
    let mut state = OrgState::new();
    state.apply_backfill(&[open]).expect("backfill");
    state.mark_live();
    let _ = state.apply(&vote).expect("vote");
    let ts = state.apply(&rejected).expect("rejected");
    match named(&ts, |t| {
        matches!(t, Transition::DirectionRejectedWithReason { .. })
    }) {
        Transition::DirectionRejectedWithReason {
            proposal, reason, ..
        } => {
            assert_eq!(proposal, &id);
            assert_eq!(reason, "direction");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn shaper_accepted() {
    let first = shapers(&[&pk(1)], Some(&pk(9)), 604_800);
    let second = shapers(&[&pk(1), &pk(2)], Some(&pk(9)), 604_800);
    let (_, ts) = apply_last(&[first, second]);
    named(
        &ts,
        |t| matches!(t, Transition::ShaperAccepted { p, .. } if *p == pk(2)),
    );
}

#[test]
fn rules_changed() {
    let first = shapers(&[&pk(1)], Some(&pk(9)), 604_800);
    let second = shapers(&[&pk(1)], Some(&pk(9)), 86_400);
    let (_, ts) = apply_last(&[first, second]);
    named(&ts, |t| matches!(t, Transition::RulesChanged { .. }));
}

#[test]
fn agent_changed() {
    let first = shapers(&[&pk(1)], Some(&pk(9)), 604_800);
    let second = shapers(&[&pk(1)], Some(&pk(8)), 604_800);
    let (_, ts) = apply_last(&[first, second]);
    match named(&ts, |t| matches!(t, Transition::AgentChanged { .. })) {
        Transition::AgentChanged { from, to, .. } => {
            assert_eq!(from.as_deref(), Some(pk(9).as_str()));
            assert_eq!(to.as_deref(), Some(pk(8).as_str()));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn draft_declined() {
    let outcome = DraftOutcome {
        draft: "ee".repeat(32),
        status: DraftOutcomeStatus::Declined,
        decided_by: Some(pk(1)),
        decided_at: Some(1),
        reason: Some(DeclineReason::WrongHolder),
        result: None,
    };
    let content = serde_json::to_string(&outcome).expect("json");
    let event = sign(
        KIND_IO_DRAFT_OUTCOME,
        &content,
        vec![
            vec!["d", &outcome.draft],
            vec!["s", "declined"],
            vec!["p", &pk(1)],
        ],
    );
    let (_, ts) = apply_last(&[event]);
    match named(&ts, |t| matches!(t, Transition::DraftDeclined { .. })) {
        Transition::DraftDeclined { reason, .. } => {
            assert_eq!(*reason, Some(DeclineReason::WrongHolder));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn profile_changed() {
    let profile = OrgProfile {
        pubkey: pk(5),
        version: 1,
        about: "hi".into(),
        skills: vec![],
        open_limit: None,
        updated_at: 1,
        receipt: "ff".repeat(32),
    };
    let content = serde_json::to_string(&profile).expect("json");
    let event = sign(
        KIND_IO_PROFILE,
        &content,
        vec![vec!["d", &pk(5)], vec!["p", &pk(5)], vec!["version", "1"]],
    );
    let (_, ts) = apply_last(&[event]);
    named(
        &ts,
        |t| matches!(t, Transition::ProfileChanged { pubkey, .. } if *pubkey == pk(5)),
    );
}

#[test]
fn root_ledger_changed() {
    let root = item("root-1", None, WorkItemState::Accepted, Some(&pk(4)));
    let (_, ts) = apply_last(&[work_item_event(&root)]);
    named(
        &ts,
        |t| matches!(t, Transition::RootLedgerChanged { root, .. } if root == "root-1"),
    );
}

#[test]
fn member_joined() {
    let first = EventBuilder::new(Kind::Custom(KIND_NIP43_MEMBERSHIP_LIST as u16), "")
        .tags([Tag::parse(["member", &pk(1), "member"]).unwrap()])
        .sign_with_keys(&Keys::generate())
        .expect("sign");
    let second = EventBuilder::new(Kind::Custom(KIND_NIP43_MEMBERSHIP_LIST as u16), "")
        .tags([
            Tag::parse(["member", &pk(1), "member"]).unwrap(),
            Tag::parse(["member", &pk(2), "member"]).unwrap(),
        ])
        .sign_with_keys(&Keys::generate())
        .expect("sign");
    let (_, ts) = apply_last(&[first, second]);
    named(
        &ts,
        |t| matches!(t, Transition::MemberJoined { p, .. } if *p == pk(2)),
    );
}

#[test]
fn dm_opened() {
    let me = pk(9);
    let other = pk(7);
    let shap = shapers(&[&pk(1)], Some(&me), 604_800);
    let dm = EventBuilder::new(Kind::Custom(KIND_DM_OPEN as u16), "")
        .tags([
            Tag::parse(["d", "dm-1"]).unwrap(),
            Tag::parse(["p", &me]).unwrap(),
            Tag::parse(["p", &other]).unwrap(),
        ])
        .allow_self_tagging()
        .sign_with_keys(&Keys::generate())
        .expect("sign");
    let (_, ts) = apply_last(&[shap, dm]);
    named(
        &ts,
        |t| matches!(t, Transition::DmOpened { room, .. } if room == "dm-1"),
    );
}

#[test]
fn backfill_emits_no_transitions() {
    let mut state = OrgState::new();
    let out = state
        .apply(&work_item_event(&item(
            "root-1",
            None,
            WorkItemState::Open,
            None,
        )))
        .expect("apply");
    assert!(out.is_empty());
}

#[test]
fn apply_refuses_a_work_item_missing_the_root_tag() {
    let item: WorkItem = item("root-1", None, WorkItemState::Open, None);
    let content = serde_json::to_string(&item).expect("json");
    let event = sign(
        buzz_core::kind::KIND_IO_WORK_ITEM,
        &content,
        vec![
            vec!["d", &item.id],
            vec!["s", "open"],
            vec!["due", &item.due_at.to_string()],
            vec!["t", "project"],
            vec!["receipt", &item.created_from],
        ],
    );
    let mut state = OrgState::new();
    let err = state.apply(&event).expect_err("missing root");
    assert!(err.to_string().contains("root"), "{err}");
}
