//! Pipeline proofs: the outbox drains after a disconnect, and a newer
//! generation mid-THINK is `stale`. Both go through [`OrgAgent::handle`]
//! / [`OrgAgent::publish_permitted`] (the production seam).

use buzz_core::intelligent_org::{ChildrenCounts, WorkItem, WorkItemState};
use buzz_core::kind::KIND_IO_WORK_ITEM;
use buzz_org_agent::judge::{Draft, JudgeReason};
use buzz_org_agent::pipeline::{HandleOutcome, OrgAgent};
use buzz_org_agent::relay::{FakeRelay, Permitted};
use buzz_org_agent::state::{OrgState, Transition};
use buzz_org_agent::think::{ContextBundle, Recorded};
use nostr::{EventBuilder, Keys, Kind, Tag};

fn work_item_event(item: &WorkItem) -> buzz_core::Event {
    let content = serde_json::to_string(item).expect("json");
    let mut tags: Vec<Vec<String>> = vec![
        vec!["d".into(), item.id.clone()],
        vec!["s".into(), "accepted".into()],
        vec!["root".into(), item.root.clone()],
        vec!["due".into(), item.due_at.to_string()],
        vec!["t".into(), "project".into()],
        vec!["receipt".into(), item.created_from.clone()],
        vec!["p".into(), item.dri.clone().expect("dri")],
    ];
    if let Some(p) = &item.parent {
        tags.push(vec!["u".into(), p.clone()]);
    }
    EventBuilder::new(Kind::Custom(KIND_IO_WORK_ITEM as u16), content)
        .tags(tags.iter().map(|t| Tag::parse(t).expect("tag")))
        .allow_self_tagging()
        .sign_with_keys(&Keys::generate())
        .expect("sign")
}

fn held_root(title: &str) -> WorkItem {
    WorkItem {
        id: "root-1".into(),
        parent: None,
        root: "root-1".into(),
        depth: 0,
        path: vec![],
        title: title.into(),
        brief: "b".into(),
        state: WorkItemState::Accepted,
        dri: Some("aa".repeat(32)),
        offered_to: None,
        offered_by: None,
        offered_at: None,
        due_at: 2_000_000_000,
        approved_at: None,
        objective_ref: None,
        created_from: "bb".repeat(32),
        draft: None,
        done_receipt: None,
        closed_by: None,
        children: ChildrenCounts::default(),
        home: None,
        branch: None,
        after: vec![],
        last_progress: None,
    }
}

fn agent(state: OrgState) -> (OrgAgent<Recorded, FakeRelay>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tmp");
    let agent = OrgAgent::new(
        state,
        Recorded::new(vec![]),
        FakeRelay::new(),
        false,
        dir.path().to_path_buf(),
        Keys::generate(),
    )
    .expect("agent");
    (agent, dir)
}

#[test]
fn outbox_drains_after_simulated_disconnect() {
    let (mut agent, _dir) = agent(OrgState::new());
    agent.link.connect(&Default::default()).expect("connect");
    agent
        .publish_permitted(Permitted::AgentNote, "{}", vec![], 1)
        .expect("first");
    assert!(agent.outbox.pending().is_empty());
    assert_eq!(agent.link.io().published().len(), 1);

    agent.link.io().simulate_disconnect();
    agent
        .publish_permitted(Permitted::AgentNote, "{\"n\":2}", vec![], 2)
        .expect("queued");
    assert_eq!(agent.outbox.pending().len(), 1);
    assert_eq!(agent.link.io().published().len(), 1);

    agent.link.connect(&Default::default()).expect("reconnect");
    agent.drain_outbox().expect("drain");
    assert!(agent.outbox.pending().is_empty());
    assert_eq!(agent.link.io().published().len(), 2);
}

#[test]
fn newer_generation_mid_think_is_stale() {
    let first = work_item_event(&held_root("one"));
    let snapped = first.id.to_hex();
    let mut state = OrgState::new();
    state.mark_live();
    let _ = state.apply(&first).expect("first");

    let second = work_item_event(&held_root("two"));
    let (mut agent, _dir) = agent(state);
    agent.mid_think = Some(Box::new(move |s| {
        let _ = s.apply(&second).expect("newer");
    }));

    let mut bundle = ContextBundle {
        generation: snapped.clone(),
        now: 1_700_000_000,
        ..ContextBundle::default()
    };
    bundle.ids.insert("rec1".into());
    bundle.resolved.insert("rec1".into());
    let draft = Draft {
        kind: None,
        payload: None,
        health: Some(buzz_core::intelligent_org::HealthRead {
            item: "root-1".into(),
            week: "2026-W12".into(),
            pct: 0.5,
            band: buzz_core::intelligent_org::HealthBand::Healthy,
            factors: vec![],
            sentences: vec![buzz_core::intelligent_org::GroundedSentence {
                text: "fine".into(),
                rows: vec!["rec1".into()],
            }],
            formula: "health-weights@1".into(),
        }),
        raw: serde_json::json!({}),
        needs: "shaper".into(),
        gap: String::new(),
        receipts: vec!["rec1".into()],
        generation: snapped.clone(),
        parent: None,
        item: Some("root-1".into()),
    };
    let transition = Transition::HolderSet {
        item: "root-1".into(),
        dri: "aa".repeat(32),
        generation: snapped,
    };
    let outcome = agent
        .handle(transition, Some(draft), bundle)
        .expect("handle");
    assert_eq!(
        outcome,
        HandleOutcome::Dropped {
            reason: JudgeReason::Stale
        }
    );
}
