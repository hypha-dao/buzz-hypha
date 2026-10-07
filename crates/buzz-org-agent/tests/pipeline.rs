//! Pipeline proofs: the outbox drains after a disconnect, and a newer
//! generation mid-THINK is `stale`. Both go through [`OrgAgent::handle`]
//! / [`OrgAgent::publish_permitted`] (the production seam).

use buzz_core::intelligent_org::{
    ChildrenCounts, DirectionArtifact, DirectionLine, DirectionSlug, WorkItem, WorkItemState,
};
use buzz_core::kind::KIND_IO_WORK_ITEM;
use buzz_org_agent::jobs::JobLog;
use buzz_org_agent::judge::{Draft, JudgeReason};
use buzz_org_agent::pipeline::{HandleOutcome, OrgAgent};
use buzz_org_agent::relay::{FakeRelay, Permitted};
use buzz_org_agent::state::{DirectionHead, OrgState, Transition};
use buzz_org_agent::think::model::{ModelOutput, Usage};
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
        created_by: None,
        offered_by_member: None,
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

fn objectives_head(event_id: &str) -> DirectionHead {
    DirectionHead {
        artifact: DirectionArtifact {
            slug: DirectionSlug::Objectives,
            version: 1,
            body: "Outcomes.".into(),
            lines: vec![DirectionLine {
                n: 1,
                id: "line-a".into(),
                text: "A weekday hall".into(),
                date: Some(1_780_000_000),
                done_when: Some("one paid night".into()),
                line_type: None,
            }],
            confirmed_by: "aa".repeat(32),
            confirmed_at: 1,
            proposed_by: "bb".repeat(32),
            proposal: "11111111-1111-4111-8111-111111111111".into(),
            prev: None,
        },
        event_id: event_id.into(),
    }
}

fn confirm(generation: &str) -> Transition {
    Transition::DirectionConfirmed {
        slug: DirectionSlug::Objectives,
        version: 1,
        diff: String::new(),
        generation: generation.into(),
    }
}

#[test]
fn direction_confirmed_coalesces_and_a_late_finish_is_stale() {
    let mut state = OrgState::new();
    state
        .direction
        .insert("objectives".into(), objectives_head("v1"));
    let (mut agent, _dir) = agent(state);
    agent.link.connect(&Default::default()).expect("connect");

    let first = agent
        .handle(confirm("v1"), None, ContextBundle::default())
        .expect("first");
    assert_eq!(first, HandleOutcome::NoDraft);
    assert_eq!(agent.jobs.len(), 1);
    assert_eq!(agent.jobs.generation_of("direction:objectives"), Some("v1"));

    let second = agent
        .handle(confirm("v2"), None, ContextBundle::default())
        .expect("second");
    assert_eq!(second, HandleOutcome::NoDraft);
    assert_eq!(agent.jobs.len(), 1);
    assert_eq!(agent.jobs.generation_of("direction:objectives"), Some("v2"));

    let mission = agent
        .handle(
            Transition::DirectionConfirmed {
                slug: DirectionSlug::Mission,
                version: 1,
                diff: String::new(),
                generation: "mission-1".into(),
            },
            None,
            ContextBundle::default(),
        )
        .expect("mission");
    assert_eq!(mission, HandleOutcome::NoDraft);
    assert_eq!(agent.jobs.len(), 1);

    assert!(agent.jobs.start());
    agent
        .state
        .direction
        .get_mut("objectives")
        .expect("head")
        .event_id = "v3".into();
    assert!(agent.jobs.finish(&agent.state).is_none());
    assert!(agent.jobs.log().iter().any(|entry| matches!(
        entry,
        JobLog::Stale { key, generation }
            if key == "direction:objectives" && generation == "v2"
    )));
    assert!(agent.link.io().published().is_empty());
}

fn plan_value() -> serde_json::Value {
    serde_json::json!({
        "title": "Weekday hall trial",
        "brief": "A short trial of a weekday night.",
        "objective_ref": "objectives@1#line-a",
        "due_at": 1785488400,
        "suggested_dri": null,
        "why": "learn whether weekday buyers come",
        "gaps": [],
        "gap": { "ref": "objectives@1#line-a", "verdict": "uncovered" },
        "options": [
            { "title": "Weekday hall trial", "mechanism": "tests demand before a weekly booking", "kept": true },
            { "title": "Apply for a city grant", "mechanism": "a grant round pays the hall", "kept": false, "why_not": "strategy refusal" }
        ],
        "change": {
            "from": "no weekday night",
            "to": "a trial has answered whether buyers come",
            "done_when": ["sessions held"],
            "moves": ["objectives@1#line-a"]
        },
        "plan": [
            { "piece": "Evening licence application", "kind": "writing", "gate": true, "produces": ["sessions held"] },
            { "piece": "Book the nights", "kind": "ops", "gate": false, "held": "after the licence", "after": ["Evening licence application"] }
        ]
    })
}

#[tokio::test]
async fn direction_confirmed_publishes_one_project_draft() {
    let gen = "ab".repeat(32);
    let mut state = OrgState::new();
    let mut head = objectives_head(&gen);
    head.artifact.lines[0].done_when = Some("sessions held".into());
    state.direction.insert("objectives".into(), head);
    state.direction.insert(
        "situation".into(),
        DirectionHead {
            artifact: buzz_core::intelligent_org::DirectionArtifact {
                slug: DirectionSlug::Situation,
                version: 1,
                body: "What is stuck is whether weekday buyers will come.".into(),
                lines: vec![],
                confirmed_by: "aa".repeat(32),
                confirmed_at: 1,
                proposed_by: "bb".repeat(32),
                proposal: "11111111-1111-4111-8111-111111111111".into(),
                prev: None,
            },
            event_id: "cd".repeat(32),
        },
    );
    state.direction.insert(
        "strategy".into(),
        DirectionHead {
            artifact: buzz_core::intelligent_org::DirectionArtifact {
                slug: DirectionSlug::Strategy,
                version: 1,
                body: "How we get there.".into(),
                lines: vec![buzz_core::intelligent_org::DirectionLine {
                    n: 1,
                    id: "s1".into(),
                    text: "The stall funds the hall, not a grant round.".into(),
                    date: None,
                    done_when: None,
                    line_type: Some(buzz_core::intelligent_org::StrategyLineType::Refusal),
                }],
                confirmed_by: "aa".repeat(32),
                confirmed_at: 1,
                proposed_by: "bb".repeat(32),
                proposal: "22222222-2222-4222-8222-222222222222".into(),
                prev: None,
            },
            event_id: "ef".repeat(32),
        },
    );
    state.shapers = Some(buzz_core::intelligent_org::Shapers {
        founder: "aa".repeat(32),
        shapers: vec!["aa".repeat(32)],
        offered: vec![],
        room: Some("shapers-room".into()),
        agent: None,
        agent_hosted: false,
        rules: Default::default(),
        decision_window_secs: 60,
        offer_window_secs: 60,
        updated_at: 1,
        receipt: "11".repeat(32),
    });
    let (mut agent, _dir) = agent(state);
    agent.link.connect(&Default::default()).expect("connect");
    agent.move_1 = true;
    agent.model.push(ModelOutput {
        value: plan_value(),
        usage: Usage::default(),
        model: "taped".into(),
    });
    agent
        .handle(confirm(&gen), None, ContextBundle::default())
        .expect("enqueue");
    assert_eq!(agent.jobs.len(), 1);
    let published = agent.run_jobs().await.expect("jobs");
    assert_eq!(published, 1);
    let drafts: Vec<_> = agent
        .link
        .io()
        .published()
        .iter()
        .filter(|event| u32::from(event.kind.as_u16()) == 50100)
        .collect();
    assert_eq!(drafts.len(), 1);
    assert!(drafts[0].content.contains("\"gate\":true"));
    assert!(drafts[0].content.contains("Evening licence application"));
    assert!(agent
        .link
        .io()
        .published()
        .iter()
        .any(|event| event.kind.as_u16() == 9 && event.content.contains("My work")));

    agent.model.push(ModelOutput {
        value: plan_value(),
        usage: Usage::default(),
        model: "taped".into(),
    });
    agent
        .handle(confirm(&gen), None, ContextBundle::default())
        .expect("again");
    let again = agent.run_jobs().await.expect("second");
    assert_eq!(again, 0, "an open draft for the gap is not drafted again");
    assert_eq!(
        agent
            .link
            .io()
            .published()
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50100)
            .count(),
        1
    );
}

fn five_step_plan() -> serde_json::Value {
    serde_json::json!([
        { "piece": "Evening licence application", "kind": "writing", "gate": true, "after": [], "produces": ["licence filed"] },
        { "piece": "Hygiene certificate", "kind": "ops", "gate": true, "after": [], "produces": ["certificate held"] },
        { "piece": "Book four Tuesdays", "kind": "ops", "gate": false, "after": ["Evening licence application"], "held": "after Evening licence application" },
        { "piece": "Publicity", "kind": "outreach", "gate": false, "after": ["Book four Tuesdays"], "held": "after Book four Tuesdays" },
        { "piece": "Run four sessions", "kind": "ops", "gate": false, "after": ["Book four Tuesdays", "Hygiene certificate"], "held": "after both gates" }
    ])
}

#[tokio::test]
async fn a_held_project_without_a_plan_asks_for_steps() {
    let gen = "ab".repeat(32);
    let mut root = held_root("Continuous task flow");
    root.brief = "The AI suggests the next task.".into();
    let mut state = OrgState::new();
    state.item_generations.insert(root.id.clone(), gen.clone());
    state.items.insert(root.id.clone(), root);
    let (mut agent, _dir) = agent(state);
    agent.link.connect(&Default::default()).expect("connect");
    agent.model.push(ModelOutput {
        value: serde_json::json!({
            "steps": [
                { "piece": "Show the next task on My work", "kind": "code", "after": [], "produces": ["the next task is visible"] },
                { "piece": "Offer the next task when one is done", "kind": "code", "after": ["Show the next task on My work"], "produces": ["done work opens the next task"] }
            ]
        }),
        usage: Usage::default(),
        model: "taped".into(),
    });
    agent
        .handle(
            Transition::HolderSet {
                item: "root-1".into(),
                dri: "aa".repeat(32),
                generation: gen,
            },
            None,
            ContextBundle::default(),
        )
        .expect("hold");
    let published = agent.run_jobs().await.expect("tickets");
    assert_eq!(published, 1);
    let body: serde_json::Value = agent
        .link
        .io()
        .published()
        .iter()
        .find(|event| u32::from(event.kind.as_u16()) == 50100)
        .map(|event| serde_json::from_str(&event.content).expect("ticket json"))
        .expect("draft");
    assert_eq!(body["title"], "Show the next task on My work");
    assert_eq!(body["brief"], "the next task is visible");
    assert_ne!(body["title"], "Continuous task flow");
    let again = agent.run_jobs().await.expect("quiet");
    assert_eq!(again, 0);
}

#[tokio::test]
async fn holder_set_drafts_gate_tickets_and_done_unblocks_one() {
    use buzz_core::intelligent_org::{
        ClosedBy, DecisionRule, Executed, Proposal, ProposalKind, ProposalStatus,
    };

    let gen = "ab".repeat(32);
    let mut root = held_root("Weekday hall trial");
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
                "brief": "A trial.",
                "due_at": 1785488400,
                "plan": five_step_plan(),
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
    let (mut agent, _dir) = agent(state);
    agent.link.connect(&Default::default()).expect("connect");
    agent
        .handle(
            Transition::HolderSet {
                item: "root-1".into(),
                dri: "aa".repeat(32),
                generation: gen.clone(),
            },
            None,
            ContextBundle::default(),
        )
        .expect("hold");
    let published = agent.run_jobs().await.expect("tickets");
    assert_eq!(published, 2);
    let titles: Vec<String> = agent
        .link
        .io()
        .published()
        .iter()
        .filter(|event| u32::from(event.kind.as_u16()) == 50100)
        .map(|event| {
            serde_json::from_str::<serde_json::Value>(&event.content).expect("ticket json")["title"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(
        titles,
        vec![
            "Evening licence application".to_string(),
            "Hygiene certificate".to_string()
        ]
    );
    assert!(agent.link.io().published().iter().all(|event| {
        serde_json::from_str::<serde_json::Value>(&event.content)
            .ok()
            .and_then(|value| value.get("gate").and_then(|gate| gate.as_bool()))
            .unwrap_or(true)
    }));

    let mut gate = held_root("Evening licence application");
    gate.id = "gate-1".into();
    gate.parent = Some("root-1".into());
    gate.root = "root-1".into();
    gate.depth = 1;
    gate.state = buzz_core::intelligent_org::WorkItemState::Done;
    gate.closed_by = Some(ClosedBy::Dri);
    let child_gen = "cd".repeat(32);
    agent
        .state
        .item_generations
        .insert(gate.id.clone(), child_gen.clone());
    agent.state.items.insert(gate.id.clone(), gate);
    agent
        .handle(
            Transition::ItemDone {
                item: "gate-1".into(),
                closed_by: Some(ClosedBy::Dri),
                generation: child_gen,
            },
            None,
            ContextBundle::default(),
        )
        .expect("done");
    let unblocked = agent.run_jobs().await.expect("unblocked");
    assert_eq!(unblocked, 1);
    let tickets: Vec<_> = agent
        .link
        .io()
        .published()
        .iter()
        .filter(|event| u32::from(event.kind.as_u16()) == 50100)
        .collect();
    assert_eq!(tickets.len(), 3);
    assert!(tickets
        .last()
        .expect("ticket")
        .content
        .contains("Book four Tuesdays"));
    let again = agent.run_jobs().await.expect("quiet");
    assert_eq!(again, 0);
    assert_eq!(
        agent
            .link
            .io()
            .published()
            .iter()
            .filter(|event| u32::from(event.kind.as_u16()) == 50100)
            .count(),
        3
    );
}

#[tokio::test]
async fn an_offered_ticket_publishes_a_prompt_and_a_failed_digest_does_not() {
    use std::time::Duration;

    use buzz_core::intelligent_org::{
        DecisionRule, Executed, ProjectHome, Proposal, ProposalKind, ProposalStatus,
    };
    use buzz_org_agent::prompt::{RepoFile, RepoListing, DIGEST_MAX_FILES};

    let gen = "ab".repeat(32);
    let mut root = held_root("Weekday hall");
    root.home = Some(ProjectHome {
        channel: "room-1".into(),
        repo: Some("30617:aa:weekday".into()),
        project: Some("30621:aa:weekday".into()),
    });
    let writing = WorkItem {
        id: "ticket-write".into(),
        parent: Some("root-1".into()),
        root: "root-1".into(),
        depth: 1,
        path: vec!["root-1".into()],
        title: "Write the note".into(),
        brief: "Write the note".into(),
        state: WorkItemState::Offered,
        dri: None,
        offered_to: Some("cc".repeat(32)),
        offered_by: Some("aa".repeat(32)),
        offered_at: Some(1),
        branch: Some("io/tick-write-the-note".into()),
        ..held_root("unused")
    };
    let code = WorkItem {
        id: "ticket-code".into(),
        title: "Patch the door".into(),
        brief: "Patch the door".into(),
        ..writing.clone()
    };
    let mut state = OrgState::new();
    state.item_generations.insert(root.id.clone(), gen.clone());
    state
        .item_generations
        .insert(writing.id.clone(), gen.clone());
    state.item_generations.insert(code.id.clone(), gen.clone());
    state.items.insert(root.id.clone(), root);
    state.items.insert(writing.id.clone(), writing.clone());
    state.items.insert(code.id.clone(), code.clone());
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
                "title": "Weekday hall",
                "brief": "A trial.",
                "due_at": 1785488400,
                "plan": [
                    { "piece": "Write the note", "kind": "writing", "produces": ["note filed", "reader can find it"] },
                    { "piece": "Patch the door", "kind": "code", "produces": ["door opens"] }
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
    let (mut agent, _dir) = agent(state);
    agent.listings.insert(
        "30617:aa:weekday".into(),
        RepoListing {
            commit: "abc1234".into(),
            files: vec![
                RepoFile {
                    path: "README.md".into(),
                    bytes: 1,
                };
                DIGEST_MAX_FILES + 1
            ],
            elapsed: Duration::ZERO,
        },
    );
    agent.link.connect(&Default::default()).expect("connect");
    agent
        .handle(
            Transition::TicketPrompt {
                item: writing.id.clone(),
                generation: gen.clone(),
            },
            None,
            ContextBundle::default(),
        )
        .expect("writing job");
    agent
        .handle(
            Transition::TicketPrompt {
                item: code.id,
                generation: gen,
            },
            None,
            ContextBundle::default(),
        )
        .expect("code job");
    let published = agent.run_jobs().await.expect("prompts");
    assert_eq!(published, 1);
    let prompts: Vec<_> = agent
        .link
        .io()
        .published()
        .iter()
        .filter(|event| u32::from(event.kind.as_u16()) == 50104)
        .collect();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].content.contains("note filed"));
    assert!(prompts[0].content.contains("reader can find it"));
    assert!(!prompts[0].content.contains("Paths"));
    assert!(!prompts[0].content.contains("README.md"));
    assert!(prompts
        .iter()
        .all(|event| !event.content.contains("Patch the door")));
}

fn trial_root(done: bool) -> OrgState {
    use buzz_core::intelligent_org::{
        DecisionRule, Executed, Proposal, ProposalKind, ProposalStatus,
    };

    let gen = "ab".repeat(32);
    let mut root = held_root("Weekday hall trial");
    root.state = WorkItemState::InReview;
    root.objective_ref = Some("objectives@1#line-a".into());
    let mut state = OrgState::new();
    state
        .direction
        .insert("objectives".into(), objectives_head(&gen));
    state.item_generations.insert(root.id.clone(), gen);
    let child = WorkItem {
        id: "step-1".into(),
        parent: Some(root.id.clone()),
        root: root.id.clone(),
        depth: 1,
        path: vec![root.id.clone()],
        title: "Evening licence application".into(),
        brief: "File it.".into(),
        state: if done {
            WorkItemState::Done
        } else {
            WorkItemState::Accepted
        },
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
    };
    let root_id = root.id.clone();
    state.items.insert(child.id.clone(), child);
    state.items.insert(root_id.clone(), root);
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
                "change": {
                    "from": "no weekday night",
                    "to": "a trial has answered whether buyers come",
                    "done_when": ["sessions held"],
                    "moves": ["objectives@1#line-a"]
                },
                "plan": [
                    { "piece": "Evening licence application", "kind": "writing", "gate": true, "produces": ["sessions held"] }
                ]
            }),
            rule: DecisionRule::MAJORITY,
            needed: 1,
            eligible: vec![],
            votes: vec![],
            decided_at: Some(2),
            executed: Some(Executed {
                kind: "work_item".into(),
                id: root_id,
            }),
            settlement: None,
        },
    );
    state.shapers = Some(buzz_core::intelligent_org::Shapers {
        founder: "aa".repeat(32),
        shapers: vec!["aa".repeat(32)],
        offered: vec![],
        room: Some("shapers-room".into()),
        agent: None,
        agent_hosted: false,
        rules: Default::default(),
        decision_window_secs: 60,
        offer_window_secs: 60,
        updated_at: 1,
        receipt: "11".repeat(32),
    });
    state
}

#[tokio::test]
async fn a_root_in_review_publishes_one_review_draft() {
    let gen = "ab".repeat(32);
    let follow = serde_json::json!({
        "recommendation": "follow_up",
        "why": "the trial answered whether buyers come",
        "project": {
            "title": "Second weekday night",
            "brief": "Keep the night the trial filled.",
            "objective_ref": "objectives@1#line-a",
            "due_at": 1785488400,
            "suggested_dri": null,
            "why": "the trial answered whether buyers come",
            "gaps": [],
            "gap": { "ref": "objectives@1#line-a", "verdict": "covered" },
            "options": [
                { "title": "Second weekday night", "mechanism": "keeps the night the trial filled", "kept": true },
                { "title": "Wait a season", "mechanism": "leave the hall dark", "kept": false, "why_not": "the question is answered" }
            ],
            "change": {
                "from": "one trial and no second night",
                "to": "a second weekday night is open",
                "done_when": ["sessions held"],
                "moves": ["objectives@1#line-a"]
            },
            "plan": [
                { "piece": "Book the second night", "kind": "ops", "gate": true, "produces": ["sessions held"] }
            ]
        }
    });
    let (mut following, _dir) = agent(trial_root(true));
    following
        .link
        .connect(&Default::default())
        .expect("connect");
    following.model.push(ModelOutput {
        value: follow,
        usage: Usage::default(),
        model: "taped".into(),
    });
    following
        .handle(
            Transition::EnteredReview {
                root: "root-1".into(),
                generation: gen.clone(),
            },
            None,
            ContextBundle::default(),
        )
        .expect("review");
    let published = following.run_jobs().await.expect("jobs");
    assert_eq!(published, 1);
    let drafts: Vec<_> = following
        .link
        .io()
        .published()
        .iter()
        .filter(|event| u32::from(event.kind.as_u16()) == 50100)
        .collect();
    assert_eq!(drafts.len(), 1);
    assert!(drafts[0].content.contains("objectives@1#line-a"));
    assert!(drafts[0].content.contains("Promised:"));
    assert!(drafts[0].content.contains("Happened:"));
    assert!(drafts[0].tags.iter().any(|tag| {
        tag.as_slice().first().is_some_and(|name| name == "t")
            && tag.as_slice().get(1).is_some_and(|value| value == "review")
    }));
    following
        .handle(
            Transition::EnteredReview {
                root: "root-1".into(),
                generation: gen.clone(),
            },
            None,
            ContextBundle::default(),
        )
        .expect("review again");
    assert_eq!(following.run_jobs().await.expect("second"), 0);

    let (mut stopped, _dir) = agent(trial_root(false));
    stopped.link.connect(&Default::default()).expect("connect");
    stopped.model.push(ModelOutput {
        value: serde_json::json!({
            "recommendation": "stop",
            "why": "the trial did not answer the question"
        }),
        usage: Usage::default(),
        model: "taped".into(),
    });
    stopped
        .handle(
            Transition::EnteredReview {
                root: "root-1".into(),
                generation: gen,
            },
            None,
            ContextBundle::default(),
        )
        .expect("stop");
    assert_eq!(stopped.run_jobs().await.expect("stop jobs"), 1);
    let stop_drafts: Vec<_> = stopped
        .link
        .io()
        .published()
        .iter()
        .filter(|event| u32::from(event.kind.as_u16()) == 50100)
        .collect();
    assert_eq!(stop_drafts.len(), 1);
    assert!(!stop_drafts[0].content.contains("\"project\""));
    assert!(stop_drafts[0].content.contains("no_further_work"));
}
