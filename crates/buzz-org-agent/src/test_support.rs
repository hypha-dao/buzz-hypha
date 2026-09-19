//! Shared test helpers (not compiled into the shipped agent).

use buzz_core::intelligent_org::{
    ChildrenCounts, ClosedBy, DraftKind, DraftPayload, TicketDraft, WorkItem, WorkItemState,
};
use buzz_core::kind::KIND_IO_WORK_ITEM;
use buzz_core::Event;
use nostr::{EventBuilder, Keys, Kind, Tag};

use crate::judge::Draft;
use crate::think::context::ContextBundle;

pub fn sign(kind: u32, content: &str, tags: Vec<Vec<&str>>) -> Event {
    let keys = Keys::generate();
    EventBuilder::new(Kind::Custom(kind as u16), content)
        .tags(tags.into_iter().map(|t| Tag::parse(t).expect("tag")))
        .allow_self_tagging()
        .sign_with_keys(&keys)
        .expect("sign")
}

pub fn item(id: &str, parent: Option<&str>, state: WorkItemState, dri: Option<&str>) -> WorkItem {
    WorkItem {
        id: id.to_owned(),
        parent: parent.map(str::to_owned),
        root: parent
            .map(|_| "root-1".to_owned())
            .unwrap_or_else(|| id.to_owned()),
        depth: if parent.is_some() { 1 } else { 0 },
        path: if parent.is_some() {
            vec!["root-1".into()]
        } else {
            vec![]
        },
        title: "t".into(),
        brief: "b".into(),
        state,
        dri: dri.map(str::to_owned),
        offered_to: None,
        offered_by: None,
        offered_at: None,
        due_at: 2_000_000_000,
        approved_at: None,
        objective_ref: None,
        created_from: "aa".repeat(32),
        draft: None,
        done_receipt: None,
        closed_by: if state == WorkItemState::Done {
            Some(ClosedBy::Dri)
        } else {
            None
        },
        children: ChildrenCounts::default(),
        home: None,
        branch: None,
        after: vec![],
        last_progress: None,
    }
}

pub fn work_item_event(item: &WorkItem) -> Event {
    let content = serde_json::to_string(item).expect("json");
    let mut tags: Vec<Vec<String>> = vec![
        vec!["d".into(), item.id.clone()],
        vec!["s".into(), wire_state(item.state).into()],
        vec!["root".into(), item.root.clone()],
        vec!["due".into(), item.due_at.to_string()],
        vec![
            "t".into(),
            if item.parent.is_none() {
                "project"
            } else {
                "ticket"
            }
            .into(),
        ],
        vec!["receipt".into(), item.created_from.clone()],
    ];
    if let Some(p) = &item.parent {
        tags.push(vec!["u".into(), p.clone()]);
    }
    if let Some(d) = &item.dri {
        tags.push(vec!["p".into(), d.clone()]);
    }
    if let Some(o) = &item.offered_to {
        tags.push(vec!["p".into(), o.clone(), String::new(), "offered".into()]);
    }
    if let Some(r) = &item.objective_ref {
        tags.push(vec!["ref".into(), r.clone()]);
    }
    let keys = Keys::generate();
    EventBuilder::new(Kind::Custom(KIND_IO_WORK_ITEM as u16), content)
        .tags(tags.iter().map(|t| Tag::parse(t).expect("tag")))
        .allow_self_tagging()
        .sign_with_keys(&keys)
        .expect("sign")
}

fn wire_state(s: WorkItemState) -> &'static str {
    match s {
        WorkItemState::Open => "open",
        WorkItemState::Offered => "offered",
        WorkItemState::Accepted => "accepted",
        WorkItemState::InReview => "in_review",
        WorkItemState::Done => "done",
    }
}

pub fn passing_ticket() -> (TicketDraft, Draft, ContextBundle) {
    let ticket = TicketDraft {
        parent: "root-1".into(),
        title: "Covers".into(),
        brief: "hang the covers".into(),
        due_at: 1_900_000_000,
        requires: vec![],
        suggested_holder: None,
        unfilled: None,
        covers: "covers".into(),
        after: vec![],
        gate: false,
        coverage: vec![],
        matched: None,
    };
    let raw = serde_json::to_value(&ticket).expect("json");
    let draft = Draft {
        kind: Some(DraftKind::Ticket),
        payload: Some(DraftPayload::Ticket(ticket.clone())),
        health: None,
        raw,
        needs: "holder".into(),
        gap: "ticket:covers".into(),
        receipts: vec!["rec1".into()],
        generation: "gen1".into(),
        parent: Some("root-1".into()),
        item: None,
    };
    let mut bundle = ContextBundle {
        generation: "gen1".into(),
        now: 1_700_000_000,
        ..ContextBundle::default()
    };
    bundle.ids.insert("rec1".into());
    bundle.resolved.insert("rec1".into());
    (ticket, draft, bundle)
}

pub fn root_held(dri: &str) -> WorkItem {
    item("root-1", None, WorkItemState::Accepted, Some(dri))
}
