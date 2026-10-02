//! Work-tree commands (`50005`–`50011`, `50018`) and `project` execution
//! (Protocol §4.2, §5.1, §5.3).
//!
//! A passed `project` opens a root in `open`, `offered` to `suggested_dri`,
//! or `accepted` when that person already agreed, and creates its home room
//! (R-9a; `home.repo` /
//! `home.project` wait for R-9b). Children are created only by the parent's
//! holder; only `offered_to` accepts or declines; only the holder marks done
//! or releases; a Shaper (root) or the parent holder (child) sets due; the
//! `dri` of a `done` item may reopen within seven days. Each command stores
//! one event, emits the item's `39101` (and rewrites the parent so `children`
//! stays current), and appends one ledger row, all on the
//! `persist_command_event` transaction.

use buzz_core::intelligent_org::{
    tag, ChildrenCounts, ClosedBy, DirectionSlug, EmptyContent, Executed, ProjectProposeContent,
    Proposal, TicketCreateContent, VoteChoice, WhyContent, WorkItem, WorkItemState,
};
use buzz_db::intelligent_org::{self as store, LedgerEntry, WorkItemRow};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::apply::Projection;
use super::home::{self, home_channel_only};
use super::proposals::Execution;
use super::{
    authorize, begin, content_value, current_shapers, internal, object, parse_content,
    persist_write, pubkey_tag, timestamp_tag, uuid_tag, Command, Persisted,
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
            WorkItemState::Withdrawn => {}
        }
    }
    if let Some(to) = to {
        match to {
            WorkItemState::Open => counts.open += 1,
            WorkItemState::Offered => counts.offered += 1,
            WorkItemState::Accepted | WorkItemState::InReview => counts.accepted += 1,
            WorkItemState::Done => counts.done += 1,
            WorkItemState::Withdrawn => {}
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

/// §5.1 rule 5 helper for the scheduler: every live child of a
/// rule-closed root is returned to the Shapers as `open`. `done`
/// children are left alone. Already-`open` children still get a rewrite
/// so the ledger `orphaned_by_close` has a receipt `39101`.
pub(super) fn orphan_child_for_closed_root(
    mut child: WorkItem,
) -> Option<(WorkItemState, WorkItem)> {
    if matches!(child.state, WorkItemState::Done | WorkItemState::Withdrawn) {
        return None;
    }
    let from = child.state;
    child.state = WorkItemState::Open;
    child.dri = None;
    child.offered_to = None;
    child.offered_by = None;
    child.offered_at = None;
    Some((from, child))
}

/// Build a work-item projection the scheduler (and commands) write through
/// `apply`. `receipt` is a prior state-event id for rule-driven rewrites.
pub(super) fn work_item_projection(item: WorkItem, receipt: String) -> Projection {
    work_projection(item, receipt)
}

/// Apply a children-counter delta to a parent the same way commands do.
pub(super) fn parent_with_child_delta(
    parent: WorkItem,
    from: Option<WorkItemState>,
    to: Option<WorkItemState>,
) -> WorkItem {
    with_child_delta(parent, from, to)
}

async fn persist(
    cmd: &Command<'_>,
    tx: Transaction<'static, Postgres>,
    projections: Vec<Projection>,
    rows: Vec<LedgerEntry>,
    message: String,
) -> Result<IngestResult, IngestError> {
    persist_write(cmd, tx, projections, rows, message, None).await
}

async fn load_row(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    id: Uuid,
    what: &str,
) -> Result<WorkItemRow, IngestError> {
    store::get_work_item(tx, cmd.tenant.community(), id)
        .await
        .map_err(|e| internal("read io_work_items", e))?
        .ok_or_else(|| IngestError::Rejected(format!("invalid: unknown {what}")))
}

async fn load_item(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    id: Uuid,
    what: &str,
) -> Result<WorkItem, IngestError> {
    Ok(load_row(tx, cmd, id, what).await?.content)
}

async fn load_parent(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    item: &WorkItem,
) -> Result<Option<WorkItem>, IngestError> {
    match item.parent.as_deref() {
        Some(parent) => {
            let parent_id = Uuid::parse_str(parent).map_err(|e| internal("parent id", e))?;
            Ok(Some(load_item(tx, cmd, parent_id, "parent").await?))
        }
        None => Ok(None),
    }
}

async fn load_children(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    id: &str,
) -> Result<Vec<WorkItem>, IngestError> {
    let parent_id = Uuid::parse_str(id).map_err(|e| internal("item id", e))?;
    Ok(store::list_children(tx, cmd.tenant.community(), parent_id)
        .await
        .map_err(|e| internal("list children", e))?
        .into_iter()
        .map(|row| row.content)
        .collect())
}

fn item_id(cmd: &Command<'_>) -> Result<Uuid, IngestError> {
    uuid_tag(cmd.event, "i")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing i tag (item)".into()))
}

/// `["e", <id>, "", "receipt"]` on `io_done` — the chat message a done was
/// relayed from (§5.5). Phase 0 stores it; the §5.5 author check is later.
fn receipt_marker(event: &nostr::Event) -> Result<Option<String>, IngestError> {
    let Some(value) = event.tags.iter().find_map(|t| {
        let parts = t.as_slice();
        (parts.len() >= 4 && parts[0] == "e" && parts[3] == tag::MARKER_RECEIPT)
            .then(|| parts[1].as_str())
    }) else {
        return Ok(None);
    };
    let is_hex = value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !is_hex {
        return Err(IngestError::Rejected(
            "invalid: receipt tag must be a 64-char lowercase hex event id".into(),
        ));
    }
    Ok(Some(value.to_owned()))
}

/// §5.1 rule 4: an open child of a released item is offered to the parent
/// holder, or returned to `open` (the Shapers) when the released item is a
/// root. A child already held by that authority is left accepted.
fn return_open_child(
    mut child: WorkItem,
    authority: Option<&str>,
    actor: &str,
    at: u64,
) -> Option<(WorkItemState, WorkItem)> {
    if matches!(child.state, WorkItemState::Done | WorkItemState::Withdrawn) {
        return None;
    }
    let from = child.state;
    match authority {
        Some(p)
            if child.dri.as_deref() == Some(p)
                && matches!(from, WorkItemState::Accepted | WorkItemState::InReview) =>
        {
            if child.offered_to.is_none() && child.offered_by.is_none() {
                return None;
            }
            child.offered_to = None;
            child.offered_by = None;
            child.offered_at = None;
        }
        Some(p) => {
            if from == WorkItemState::Offered
                && child.offered_to.as_deref() == Some(p)
                && child.offered_by.as_deref() == Some(actor)
            {
                return None;
            }
            child.state = WorkItemState::Offered;
            child.dri = None;
            child.offered_to = Some(p.to_owned());
            child.offered_by = Some(actor.to_owned());
            child.offered_by_member = Some(actor.to_owned());
            child.offered_at = Some(at);
        }
        None => {
            if from == WorkItemState::Open && child.dri.is_none() && child.offered_to.is_none() {
                return None;
            }
            child.state = WorkItemState::Open;
            child.dri = None;
            child.offered_to = None;
            child.offered_by = None;
            child.offered_at = None;
        }
    }
    Some((from, child))
}

/// The suggested holder already agreed, so the root is theirs.
fn dri_already_agreed(proposal: &Proposal, dri: &str) -> bool {
    proposal
        .votes
        .iter()
        .any(|vote| vote.vote == VoteChoice::Agree && vote.p.eq_ignore_ascii_case(dri))
}

/// §5.3 `project`: create the root in `open`, or `offered` to
/// `suggested_dri`. A suggested holder who already agreed holds it
/// (`accepted`) in the same transaction — the chat yes is not a second
/// Accept on My Work. `approved_at` is the passing vote. R-9a creates the
/// home room in the same transaction and writes `home.channel`; `repo` /
/// `project` wait for R-9b. `opening_receipt` is the `50004` that opened
/// the proposal — `created_from` on the root — supplied by `settle`
/// because a D1 opener-pass has not written the `39102` yet.
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
    let named = payload.suggested_dri.clone();
    let holds = named
        .as_deref()
        .is_some_and(|dri| dri_already_agreed(proposal, dri));
    let state = if holds {
        WorkItemState::Accepted
    } else if named.is_some() {
        WorkItemState::Offered
    } else {
        WorkItemState::Open
    };
    let dri = if holds { named.clone() } else { None };
    let offered_to = if holds { None } else { named };
    let room =
        home::create_project_room(tx, cmd.tenant.community(), &payload.title, &cmd.actor_bytes)
            .await?;
    let item = WorkItem {
        id: id.to_string(),
        parent: None,
        root: id.to_string(),
        depth: 0,
        path: vec![],
        title: payload.title.clone(),
        brief: payload.brief,
        state,
        dri: dri.clone(),
        offered_to: offered_to.clone(),
        offered_by: offered_to.as_ref().map(|_| proposal.opened_by.clone()),
        offered_at: offered_to.as_ref().map(|_| cmd.at),
        created_by: None,
        offered_by_member: offered_to.as_ref().map(|_| proposal.opened_by.clone()),
        due_at: payload.due_at,
        approved_at: Some(cmd.at),
        objective_ref: payload.objective_ref,
        created_from,
        draft: None,
        done_receipt: None,
        closed_by: None,
        children: ChildrenCounts::default(),
        home: Some(home_channel_only(room)),
        branch: None,
        after: vec![],
        last_progress: None,
    };
    Ok(Execution {
        executed: executed_work_item(&item.id),
        projections: vec![work_projection(item.clone(), cmd.receipt_hex())],
        rows: vec![
            cmd.ledger(
                "item_created",
                object::WORK_ITEM,
                &item.id,
                serde_json::json!({
                    "proposal": proposal.id,
                    "state": state,
                    "offered_to": offered_to,
                    "dri": dri,
                }),
            )?,
            cmd.ledger(
                "home_created",
                object::WORK_ITEM,
                &item.id,
                serde_json::json!({
                    "channel": room,
                    "repo": null,
                    "project": null,
                }),
            )?,
        ],
        room_created: Some(room),
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
        created_by: Some(cmd.actor_hex.clone()),
        offered_by_member: offer_to.as_ref().map(|_| cmd.actor_hex.clone()),
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
    next.offered_by_member = Some(cmd.actor_hex.clone());
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

/// `io_done` (`50009`): item → `done`, `closed_by=dri`. Refused while any
/// child is live. The optional receipt tag is stored as `done_receipt`;
/// without one the command itself is the receipt.
pub(super) async fn done(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item_id = item_id(cmd)?;
    let raw = content_value(cmd.event)?;
    authorize::money_fields(&raw)?;
    parse_content::<EmptyContent>(cmd.event)?;
    let done_receipt = receipt_marker(cmd.event)?.unwrap_or_else(|| cmd.receipt_hex());

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let item = load_item(&mut tx, cmd, item_id, "item").await?;
    let children = load_children(&mut tx, cmd, &item.id).await?;
    authorize::done(&item, &cmd.actor_hex, &children)?;
    let parent = load_parent(&mut tx, cmd, &item).await?;

    let from = item.state;
    let mut next = item;
    next.state = WorkItemState::Done;
    next.closed_by = Some(ClosedBy::Dri);
    next.done_receipt = Some(done_receipt.clone());
    let mut projections = vec![work_projection(next.clone(), cmd.receipt_hex())];
    if let Some(parent) = parent {
        projections.push(work_projection(
            with_child_delta(parent, Some(from), Some(WorkItemState::Done)),
            cmd.receipt_hex(),
        ));
    }
    let rows = vec![cmd.ledger(
        "item_done",
        object::WORK_ITEM,
        &next.id,
        serde_json::json!({
            "closed_by": "dri",
            "done_receipt": done_receipt,
        }),
    )?];
    // Index only when the command cites a distinct message (§5.5 / §6.8).
    // When the command itself is the receipt there is no room message to unlock.
    if let Some(cited_hex) = receipt_marker(cmd.event)? {
        let cited = hex::decode(&cited_hex).map_err(|e| internal("decode done receipt", e))?;
        store::insert_receipts(
            &mut tx,
            cmd.tenant.community(),
            &cmd.receipt_bytes(),
            &[cited],
            store::receipt_source::DONE_RECEIPT,
        )
        .await
        .map_err(|e| internal("write io_receipts", e))?;
    }
    persist(
        cmd,
        tx,
        projections,
        rows,
        serde_json::json!({ "item": next.id }).to_string(),
    )
    .await
}

/// `io_release` (`50010`): item → `open`, holder cleared. Live children
/// are offered to the parent holder, or returned to `open` when the
/// released item is a root (§5.1 rule 4).
pub(super) async fn release(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item_id = item_id(cmd)?;
    let raw = content_value(cmd.event)?;
    authorize::money_fields(&raw)?;
    let content: WhyContent = parse_content(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let item = load_item(&mut tx, cmd, item_id, "item").await?;
    authorize::release(&item, &cmd.actor_hex)?;
    let parent = load_parent(&mut tx, cmd, &item).await?;
    let children = load_children(&mut tx, cmd, &item.id).await?;

    let from = item.state;
    let authority = parent.as_ref().and_then(|p| p.dri.clone());
    let mut next = item;
    next.state = WorkItemState::Open;
    next.dri = None;
    next.offered_to = None;
    next.offered_by = None;
    next.offered_at = None;
    let mut returned = Vec::new();
    let mut returned_detail = Vec::new();
    for child in children {
        let Some((from_child, child)) =
            return_open_child(child, authority.as_deref(), &cmd.actor_hex, cmd.at)
        else {
            continue;
        };
        bump_children(&mut next.children, Some(from_child), Some(child.state));
        returned_detail.push(serde_json::json!({
            "id": child.id,
            "from": from_child,
            "to": child.state,
            "offered_to": child.offered_to,
        }));
        returned.push(child);
    }
    let mut projections = vec![work_projection(next.clone(), cmd.receipt_hex())];
    for child in returned {
        projections.push(work_projection(child, cmd.receipt_hex()));
    }
    if let Some(parent) = parent {
        projections.push(work_projection(
            with_child_delta(parent, Some(from), Some(WorkItemState::Open)),
            cmd.receipt_hex(),
        ));
    }
    let rows = vec![cmd.ledger(
        "item_released",
        object::WORK_ITEM,
        &next.id,
        serde_json::json!({
            "why": content.why,
            "returned": returned_detail,
            "authority": authority.as_deref().unwrap_or("shapers"),
        }),
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

/// `io_set_due` (`50011`): write a new `due_at`. A later `due_at` on a
/// root is what cancels the scheduled close (R-6 reads the live date).
pub(super) async fn set_due(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item_id = item_id(cmd)?;
    let due_at = timestamp_tag(cmd.event, "due")?
        .ok_or_else(|| IngestError::Rejected("invalid: missing due tag".into()))?;
    let raw = content_value(cmd.event)?;
    authorize::money_fields(&raw)?;
    let content: WhyContent = parse_content(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("invalid: no Shapers yet".into()))?;
    let item = load_item(&mut tx, cmd, item_id, "item").await?;
    let parent = load_parent(&mut tx, cmd, &item).await?;
    authorize::set_due(&item, parent.as_ref(), &shapers, &cmd.actor_hex)?;

    let mut next = item;
    next.due_at = due_at;
    let rows = vec![cmd.ledger(
        "due_changed",
        object::WORK_ITEM,
        &next.id,
        serde_json::json!({ "due_at": due_at, "why": content.why }),
    )?];
    persist(
        cmd,
        tx,
        vec![work_projection(next.clone(), cmd.receipt_hex())],
        rows,
        serde_json::json!({ "item": next.id }).to_string(),
    )
    .await
}

/// `io_reopen` (`50018`): a `done` item → `accepted` within seven days of
/// `done_at`. The `dri` stays; `closed_by` and `done_receipt` are cleared.
pub(super) async fn reopen(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item_id = item_id(cmd)?;
    let raw = content_value(cmd.event)?;
    authorize::money_fields(&raw)?;
    let content: WhyContent = parse_content(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let row = load_row(&mut tx, cmd, item_id, "item").await?;
    let done_at = row.done_at.map(|t| t.timestamp().unsigned_abs());
    let item = row.content;
    authorize::reopen(&item, &cmd.actor_hex, done_at, cmd.at)?;
    let parent = load_parent(&mut tx, cmd, &item).await?;

    let from = item.state;
    let mut next = item;
    next.state = WorkItemState::Accepted;
    next.closed_by = None;
    next.done_receipt = None;
    let mut projections = vec![work_projection(next.clone(), cmd.receipt_hex())];
    if let Some(parent) = parent {
        projections.push(work_projection(
            with_child_delta(parent, Some(from), Some(WorkItemState::Accepted)),
            cmd.receipt_hex(),
        ));
    }
    let rows = vec![cmd.ledger(
        "item_reopened",
        object::WORK_ITEM,
        &next.id,
        serde_json::json!({ "why": content.why }),
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

/// How many nodes one removal may rewrite. A larger tree is refused
/// rather than walked without a bound.
const MAX_WITHDRAW_NODES: usize = 200;

/// `io_withdraw` (`50022`). A sole Shaper removes a project. The ticket's
/// creator, or the member who offered it, removes a ticket. Descendants
/// leave the live board with it.
pub(super) async fn withdraw(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item_id = item_id(cmd)?;
    let raw = content_value(cmd.event)?;
    authorize::money_fields(&raw)?;
    let content: WhyContent = parse_content(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("invalid: no Shapers yet".into()))?;
    let item = load_item(&mut tx, cmd, item_id, "item").await?;
    if item.parent.is_none() {
        authorize::withdraw_project(&item, &shapers, &cmd.actor_hex)?;
    } else {
        let creator = if item.created_by.is_none() {
            creating_author(&mut tx, cmd, &item.created_from).await?
        } else {
            None
        };
        authorize::withdraw_ticket(&item, &cmd.actor_hex, creator.as_deref())?;
    }
    let execution = withdraw_loaded(cmd, &mut tx, item, content.why).await?;
    persist(
        cmd,
        tx,
        execution.projections,
        execution.rows,
        serde_json::json!({ "removed": true }).to_string(),
    )
    .await
}

/// A passed `withdraw` proposal. Refuses if the project is already gone,
/// so the vote stays open rather than passing with no effect.
pub(super) async fn execute_withdraw_proposal(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    proposal: &Proposal,
) -> Result<Execution, IngestError> {
    let item_id = proposal
        .payload
        .get("i")
        .and_then(|value| value.as_str())
        .ok_or_else(|| IngestError::Rejected("invalid: withdraw proposal has no item".into()))?;
    let id = Uuid::parse_str(item_id).map_err(|error| internal("withdraw item id", error))?;
    let item = load_item(tx, cmd, id, "item").await?;
    if item.parent.is_some() {
        return Err(IngestError::Rejected(
            "invalid: a ticket is removed by its creator or the person who offered it".into(),
        ));
    }
    if item.state == WorkItemState::Withdrawn {
        return Err(IngestError::Rejected("invalid: already removed".into()));
    }
    let why = proposal
        .payload
        .get("why")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    withdraw_loaded(cmd, tx, item, why).await
}

async fn withdraw_loaded(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    item: WorkItem,
    why: Option<String>,
) -> Result<Execution, IngestError> {
    let parent = load_parent(tx, cmd, &item).await?;
    let from = item.state;
    let nodes = collect_subtree(tx, cmd, item).await?;
    let channels = project_home_channels(&nodes);
    for channel in &channels {
        home::archive_home_channel(&mut **tx, cmd.tenant.community(), *channel).await?;
    }
    let ids: Vec<String> = nodes.iter().map(|node| node.id.clone()).collect();
    let root_id = ids.first().cloned().unwrap_or_default();
    let mut projections = Vec::with_capacity(nodes.len().saturating_add(1));
    for mut node in nodes {
        node.state = WorkItemState::Withdrawn;
        node.children = ChildrenCounts::default();
        projections.push(work_projection(node, cmd.receipt_hex()));
    }
    if let Some(parent) = parent {
        projections.push(work_projection(
            with_child_delta(parent, Some(from), Some(WorkItemState::Withdrawn)),
            cmd.receipt_hex(),
        ));
    }
    let rows = vec![cmd.ledger(
        "item_withdrawn",
        object::WORK_ITEM,
        &root_id,
        serde_json::json!({
            "ids": ids,
            "why": why,
            "channels": channels.iter().map(ToString::to_string).collect::<Vec<_>>(),
        }),
    )?];
    Ok(Execution {
        executed: executed_work_item(&root_id),
        projections,
        rows,
        room_created: None,
    })
}

async fn collect_subtree(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    root: WorkItem,
) -> Result<Vec<WorkItem>, IngestError> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        if nodes.len() >= MAX_WITHDRAW_NODES {
            return Err(IngestError::Rejected(
                "invalid: too many pieces to remove at once".into(),
            ));
        }
        let children = load_children(tx, cmd, &node.id).await?;
        for child in children {
            if child.state != WorkItemState::Withdrawn {
                pending.push(child);
            }
        }
        nodes.push(node);
    }
    Ok(nodes)
}

/// Home rooms of the projects in a withdrawn tree. Tickets have no room of
/// their own, so a ticket removal leaves the project's channel up.
fn project_home_channels(nodes: &[WorkItem]) -> Vec<Uuid> {
    let mut channels = Vec::new();
    for node in nodes {
        if node.parent.is_some() {
            continue;
        }
        let Some(channel) = node.home.as_ref().map(|home| home.channel.as_str()) else {
            continue;
        };
        let Ok(id) = Uuid::parse_str(channel) else {
            continue;
        };
        if !channels.contains(&id) {
            channels.push(id);
        }
    }
    channels
}

async fn creating_author(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    created_from: &str,
) -> Result<Option<String>, IngestError> {
    let Ok(id) = hex::decode(created_from) else {
        return Ok(None);
    };
    if id.len() != 32 {
        return Ok(None);
    }
    let row = sqlx::query(
        "SELECT pubkey FROM events WHERE community_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(cmd.tenant.community().as_uuid())
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| internal("read creating command", error))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let pubkey: Vec<u8> = row
        .try_get("pubkey")
        .map_err(|error| internal("creating command pubkey", error))?;
    if pubkey.len() != 32 {
        return Ok(None);
    }
    Ok(Some(hex::encode(pubkey)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdrawing_a_project_names_its_home_room_and_a_ticket_does_not() {
        let room = Uuid::parse_str("0532e77b-04c5-4443-8225-331c7d7ae833").unwrap();
        let mut project = child(WorkItemState::Open, None);
        project.parent = None;
        project.home = Some(home_channel_only(room));
        let ticket = child(WorkItemState::Accepted, None);
        assert_eq!(project_home_channels(&[project, ticket]), vec![room]);

        let mut bare = child(WorkItemState::Open, None);
        bare.parent = None;
        assert!(project_home_channels(&[bare]).is_empty());
    }

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

    fn child(state: WorkItemState, dri: Option<&str>) -> WorkItem {
        WorkItem {
            id: "child".into(),
            parent: Some("mid".into()),
            root: "root".into(),
            depth: 2,
            path: vec!["root".into(), "mid".into()],
            title: "t".into(),
            brief: "b".into(),
            state,
            dri: dri.map(str::to_owned),
            offered_to: None,
            offered_by: None,
            offered_at: None,
            created_by: None,
            offered_by_member: None,
            due_at: 1,
            approved_at: None,
            objective_ref: None,
            created_from: "aa".repeat(32),
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

    #[test]
    fn return_open_child_offers_to_the_parent_holder_or_opens_for_shapers() {
        let holder = "11".repeat(32);
        let other = "22".repeat(32);
        let actor = "33".repeat(32);

        let done = child(WorkItemState::Done, Some(&other));
        assert!(return_open_child(done, Some(&holder), &actor, 10).is_none());

        let already = child(WorkItemState::Accepted, Some(&holder));
        assert!(return_open_child(already, Some(&holder), &actor, 10).is_none());

        let held = child(WorkItemState::Accepted, Some(&other));
        let (from, next) = return_open_child(held, Some(&holder), &actor, 10).expect("returned");
        assert_eq!(from, WorkItemState::Accepted);
        assert_eq!(next.state, WorkItemState::Offered);
        assert_eq!(next.offered_to.as_deref(), Some(holder.as_str()));
        assert_eq!(next.offered_by.as_deref(), Some(actor.as_str()));
        assert!(next.dri.is_none());

        let held_under_root = child(WorkItemState::Accepted, Some(&other));
        let (from, next) = return_open_child(held_under_root, None, &actor, 10).expect("shapers");
        assert_eq!(from, WorkItemState::Accepted);
        assert_eq!(next.state, WorkItemState::Open);
        assert!(next.offered_to.is_none());
        assert!(next.dri.is_none());
    }
}
