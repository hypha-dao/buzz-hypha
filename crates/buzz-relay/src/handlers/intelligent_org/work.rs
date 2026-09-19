//! Work-tree commands (`50005`–`50008`) and `project` execution (Protocol
//! §4.2, §5.1, §5.3).
//!
//! A passed `project` opens a root in `open` (or `offered` to
//! `suggested_dri`). Home — room, `30617` — is R-9a and is not written here.
//! Children are created only by the parent's holder; only `offered_to`
//! accepts or declines. Each command stores one event, emits the item's
//! `39101` (and rewrites the parent so `children` stays current), and
//! appends one ledger row, all on the `persist_command_event` transaction.
//! `50009`–`50011` / `50018` stay refused until R-5b.

use buzz_core::intelligent_org::{
    ChildrenCounts, DirectionSlug, EmptyContent, Executed, ProjectProposeContent, Proposal,
    TicketCreateContent, WorkItem, WorkItemState,
};
use buzz_db::intelligent_org::{self as store, LedgerEntry};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::apply::{apply, ApplyContext, Projection};
use super::proposals::Execution;
use super::{
    authorize, begin, commit, content_value, current_shapers, internal, object, parse_content,
    pubkey_tag, uuid_tag, wall_clock, Command, Persisted,
};
use crate::handlers::ingest::{IngestError, IngestResult};

fn executed_work_item(id: &str) -> Executed {
    Executed {
        kind: object::WORK_ITEM.to_owned(),
        id: id.to_owned(),
    }
}

fn work_projection(item: WorkItem, receipt: String) -> Projection {
    Projection::WorkItem {
        item: Box::new(item),
        receipt,
    }
}

fn bump_children(
    counts: &mut ChildrenCounts,
    from: Option<WorkItemState>,
    to: Option<WorkItemState>,
) {
    if let Some(from) = from {
        match from {
            WorkItemState::Open => counts.open = counts.open.saturating_sub(1),
            WorkItemState::Offered => counts.offered = counts.offered.saturating_sub(1),
            WorkItemState::Accepted | WorkItemState::InReview => {
                counts.accepted = counts.accepted.saturating_sub(1);
            }
            WorkItemState::Done => counts.done = counts.done.saturating_sub(1),
        }
    }
    if let Some(to) = to {
        match to {
            WorkItemState::Open => counts.open += 1,
            WorkItemState::Offered => counts.offered += 1,
            WorkItemState::Accepted | WorkItemState::InReview => counts.accepted += 1,
            WorkItemState::Done => counts.done += 1,
        }
    }
}

/// Convention branch `io/<first 4 of the uuid>-<slug>` (§4.2).
pub(super) fn branch_for(id: Uuid, title: &str) -> String {
    let prefix: String = id.as_hyphenated().to_string().chars().take(4).collect();
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in title.chars() {
        let mapped = if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else if c.is_whitespace() || c == '-' || c == '_' {
            Some('-')
        } else {
            None
        };
        match mapped {
            Some('-') if prev_dash || slug.is_empty() => {}
            Some('-') => {
                slug.push('-');
                prev_dash = true;
            }
            Some(ch) => {
                slug.push(ch);
                prev_dash = false;
            }
            None => {}
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        format!("io/{prefix}")
    } else {
        format!("io/{prefix}-{slug}")
    }
}

fn with_child_delta(
    mut parent: WorkItem,
    from: Option<WorkItemState>,
    to: Option<WorkItemState>,
) -> WorkItem {
    bump_children(&mut parent.children, from, to);
    parent
}

async fn persist(
    cmd: &Command<'_>,
    mut tx: Transaction<'static, Postgres>,
    projections: Vec<Projection>,
    rows: Vec<LedgerEntry>,
    message: String,
) -> Result<IngestResult, IngestError> {
    let ctx = ApplyContext {
        community: cmd.tenant.community(),
        relay: &cmd.state.relay_keypair,
        actor: &cmd.actor_bytes,
        now: wall_clock(),
    };
    let applied = apply(&cmd.state.db, &mut tx, &ctx, &projections, &rows).await?;
    commit(tx).await?;
    super::finish(cmd, applied, None).await;
    Ok(cmd.accepted(message))
}

async fn load_item(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    id: Uuid,
    what: &str,
) -> Result<WorkItem, IngestError> {
    Ok(store::get_work_item(tx, cmd.tenant.community(), id)
        .await
        .map_err(|e| internal("read io_work_items", e))?
        .ok_or_else(|| IngestError::Rejected(format!("invalid: unknown {what}")))?
        .content)
}

/// §5.3 `project`: create the root in `open`, or `offered` to
/// `suggested_dri`. `approved_at` is the passing vote; `home` waits for R-9a.
/// `opening_receipt` is the `50004` that opened the proposal — `created_from`
/// on the root — supplied by `settle` because a D1 opener-pass has not
/// written the `39102` yet.
pub(super) async fn execute_project(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    proposal: &Proposal,
    opening_receipt: &str,
) -> Result<Execution, IngestError> {
    authorize::money_fields(&proposal.payload)?;
    let payload: ProjectProposeContent = serde_json::from_value(proposal.payload.clone())
        .map_err(|e| internal("project proposal payload", e))?;
    authorize::work_copy(&payload.title, &payload.brief)?;
    if let Some(reference) = &payload.objective_ref {
        let head = store::get_direction_head(tx, cmd.tenant.community(), DirectionSlug::Objectives)
            .await
            .map_err(|e| internal("read io_direction", e))?;
        authorize::objective_ref(reference, head.as_ref().map(|row| &row.content))?;
    }
    // `settle` already holds the opening receipt. On a D1 opener-pass the
    // 39102 is not written until after execute, so looking it up here
    // would miss the head.
    let created_from = opening_receipt.to_owned();
    let id = Uuid::new_v4();
    let offered_to = payload.suggested_dri.clone();
    let state = if offered_to.is_some() {
        WorkItemState::Offered
    } else {
        WorkItemState::Open
    };
    let item = WorkItem {
        id: id.to_string(),
        parent: None,
        root: id.to_string(),
        depth: 0,
        path: vec![],
        title: payload.title,
        brief: payload.brief,
        state,
        dri: None,
        offered_to: offered_to.clone(),
        offered_by: offered_to.as_ref().map(|_| proposal.opened_by.clone()),
        offered_at: offered_to.as_ref().map(|_| cmd.at),
        due_at: payload.due_at,
        approved_at: Some(cmd.at),
        objective_ref: payload.objective_ref,
        created_from,
        draft: None,
        done_receipt: None,
        closed_by: None,
        children: ChildrenCounts::default(),
        home: None,
        branch: None,
        after: vec![],
        last_progress: None,
    };
    Ok(Execution {
        executed: executed_work_item(&item.id),
        projections: vec![work_projection(item.clone(), cmd.receipt_hex())],
        rows: vec![cmd.ledger(
            "item_created",
            object::WORK_ITEM,
            &item.id,
            serde_json::json!({
                "proposal": proposal.id,
                "state": state,
                "offered_to": offered_to,
            }),
        )?],
    })
}

/// `io_ticket_create` (`50005`): a child under the held parent, `open` or
/// `offered` when `p` is set. `after` is stored, never a lock.
pub(super) async fn create(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let parent_id = uuid_tag(cmd.event, "u")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing u tag (parent)".into()))?;
    let offer_to = pubkey_tag(cmd.event, "p")?;
    let raw = content_value(cmd.event)?;
    authorize::money_fields(&raw)?;
    let content: TicketCreateContent = serde_json::from_value(raw)
        .map_err(|e| IngestError::Rejected(format!("invalid: command content: {e}")))?;
    authorize::work_copy(&content.title, &content.brief)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let parent = load_item(&mut tx, cmd, parent_id, "parent").await?;
    authorize::create_child(&parent, &cmd.actor_hex)?;
    let siblings = store::list_children(&mut tx, cmd.tenant.community(), parent_id)
        .await
        .map_err(|e| internal("list children", e))?;
    let sibling_items: Vec<WorkItem> = siblings.into_iter().map(|row| row.content).collect();
    authorize::after_siblings(&content.after, &parent.id, &sibling_items)?;

    let id = Uuid::new_v4();
    let state = if offer_to.is_some() {
        WorkItemState::Offered
    } else {
        WorkItemState::Open
    };
    let mut path = parent.path.clone();
    path.push(parent.id.clone());
    let child = WorkItem {
        id: id.to_string(),
        parent: Some(parent.id.clone()),
        root: parent.root.clone(),
        depth: parent.depth.saturating_add(1),
        path,
        title: content.title.clone(),
        brief: content.brief,
        state,
        dri: None,
        offered_to: offer_to.clone(),
        offered_by: offer_to.as_ref().map(|_| cmd.actor_hex.clone()),
        offered_at: offer_to.as_ref().map(|_| cmd.at),
        due_at: content.due_at,
        approved_at: None,
        objective_ref: None,
        created_from: cmd.receipt_hex(),
        draft: None,
        done_receipt: None,
        closed_by: None,
        children: ChildrenCounts::default(),
        home: None,
        branch: Some(branch_for(id, &content.title)),
        after: content.after,
        last_progress: None,
    };
    let parent = with_child_delta(parent, None, Some(state));
    let rows = vec![cmd.ledger(
        "item_created",
        object::WORK_ITEM,
        &child.id,
        serde_json::json!({
            "parent": parent_id,
            "state": state,
            "offered_to": offer_to,
        }),
    )?];
    persist(
        cmd,
        tx,
        vec![
            work_projection(child.clone(), cmd.receipt_hex()),
            work_projection(parent, cmd.receipt_hex()),
        ],
        rows,
        serde_json::json!({ "item": child.id }).to_string(),
    )
    .await
}

/// `io_offer` (`50006`): item → `offered` to `p`.
pub(super) async fn offer(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item_id = uuid_tag(cmd.event, "i")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing i tag (item)".into()))?;
    let to = pubkey_tag(cmd.event, "p")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing p tag".into()))?;
    parse_content::<EmptyContent>(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("invalid: no Shapers yet".into()))?;
    let item = load_item(&mut tx, cmd, item_id, "item").await?;
    let parent = match item.parent.as_deref() {
        Some(parent) => {
            let parent_id = Uuid::parse_str(parent).map_err(|e| internal("parent id", e))?;
            Some(load_item(&mut tx, cmd, parent_id, "parent").await?)
        }
        None => None,
    };
    authorize::offer(&item, parent.as_ref(), &shapers, &cmd.actor_hex)?;

    let from = item.state;
    let mut next = item;
    next.state = WorkItemState::Offered;
    next.offered_to = Some(to.clone());
    next.offered_by = Some(cmd.actor_hex.clone());
    next.offered_at = Some(cmd.at);
    let mut projections = vec![work_projection(next.clone(), cmd.receipt_hex())];
    if let Some(parent) = parent {
        projections.push(work_projection(
            with_child_delta(parent, Some(from), Some(WorkItemState::Offered)),
            cmd.receipt_hex(),
        ));
    }
    let rows = vec![cmd.ledger(
        "item_offered",
        object::WORK_ITEM,
        &next.id,
        serde_json::json!({ "p": to }),
    )?];
    persist(
        cmd,
        tx,
        projections,
        rows,
        serde_json::json!({ "item": next.id }).to_string(),
    )
    .await
}

/// `io_accept` (`50007`): item → `accepted`; `dri` = `offered_to`.
pub(super) async fn accept(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    settle_offer(cmd, true).await
}

/// `io_decline` (`50008`): item → `open`; the offer is cleared.
pub(super) async fn decline(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    settle_offer(cmd, false).await
}

async fn settle_offer(cmd: &Command<'_>, accept: bool) -> Result<IngestResult, IngestError> {
    let item_id = uuid_tag(cmd.event, "i")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing i tag (item)".into()))?;
    parse_content::<EmptyContent>(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let item = load_item(&mut tx, cmd, item_id, "item").await?;
    authorize::accept_or_decline(&item, &cmd.actor_hex)?;
    let parent = match item.parent.as_deref() {
        Some(parent) => {
            let parent_id = Uuid::parse_str(parent).map_err(|e| internal("parent id", e))?;
            Some(load_item(&mut tx, cmd, parent_id, "parent").await?)
        }
        None => None,
    };

    let from = item.state;
    let to = if accept {
        WorkItemState::Accepted
    } else {
        WorkItemState::Open
    };
    let mut next = item;
    if accept {
        next.state = WorkItemState::Accepted;
        next.dri = next.offered_to.clone();
    } else {
        next.state = WorkItemState::Open;
        next.dri = None;
    }
    next.offered_to = None;
    next.offered_by = None;
    next.offered_at = None;
    let mut projections = vec![work_projection(next.clone(), cmd.receipt_hex())];
    if let Some(parent) = parent {
        projections.push(work_projection(
            with_child_delta(parent, Some(from), Some(to)),
            cmd.receipt_hex(),
        ));
    }
    let verb = if accept {
        "item_accepted"
    } else {
        "item_declined"
    };
    let rows = vec![cmd.ledger(
        verb,
        object::WORK_ITEM,
        &next.id,
        serde_json::json!({ "why": if accept { "accept" } else { "decline" } }),
    )?];
    persist(
        cmd,
        tx,
        projections,
        rows,
        serde_json::json!({ "item": next.id }).to_string(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_for_uses_the_first_four_of_the_uuid_and_a_slug() {
        let id = Uuid::parse_str("7f3a0000-0000-4000-8000-000000000000").unwrap();
        assert_eq!(branch_for(id, "Weekday hall"), "io/7f3a-weekday-hall");
        assert_eq!(branch_for(id, "  --  "), "io/7f3a");
    }

    #[test]
    fn bump_children_moves_a_count_between_buckets() {
        let mut counts = ChildrenCounts {
            open: 1,
            offered: 0,
            accepted: 0,
            done: 0,
        };
        bump_children(
            &mut counts,
            Some(WorkItemState::Open),
            Some(WorkItemState::Offered),
        );
        assert_eq!(
            counts,
            ChildrenCounts {
                open: 0,
                offered: 1,
                accepted: 0,
                done: 0,
            }
        );
        bump_children(&mut counts, None, Some(WorkItemState::Open));
        assert_eq!(counts.open, 1);
        bump_children(&mut counts, Some(WorkItemState::Offered), None);
        assert_eq!(counts.offered, 0);
        bump_children(
            &mut counts,
            Some(WorkItemState::InReview),
            Some(WorkItemState::Done),
        );
        assert_eq!(counts.accepted, 0);
        assert_eq!(counts.done, 1);
    }
}
