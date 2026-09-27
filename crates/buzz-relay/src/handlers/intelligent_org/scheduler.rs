//! `io_scheduler` — Protocol §6.3 date rules (R-6, V9, D6).
//!
//! One tick walks every community with a live `39103` and attempts each
//! transition as its own transaction: claim a row in `io_scheduler_claims`
//! (INSERT … ON CONFLICT DO NOTHING), re-verify the candidate still matches,
//! then write through [`super::apply`] with `actor = "relay"` ledger rows.
//! A pod that loses the claim does nothing. The claim epoch is the rule's
//! timestamp (`offered_at`, `due_at`, `expires_at`, seat `at`), so a moved
//! `due_at` or a fresh offer/seat is a new claim.
//!
//! Transitions:
//! 1. Offers past half-window → renotify (ledger `offer_renotified`, re-emit
//!    unchanged `39101`); past the window → return to `open`.
//! 2. Roots in the last fifth of `[approved_at, due_at]` (floor two days) →
//!    `in_review`.
//! 3. Roots past `due_at` → `done` / `closed_by=rule`; live children →
//!    `open` with ledger `orphaned_by_close`.
//! 4. Drafts past `expiration` → `39104 expired`.
//! 5. Open proposals past `expires_at` → `expired`; seats in
//!    `39103.offered` past `offer_window_secs` → dropped
//!    (`shaper_offer_lapsed`).

use std::sync::Arc;
use std::time::Duration;

use buzz_core::intelligent_org::{
    ClosedBy, DraftOutcome, DraftOutcomeStatus, ProposalStatus, Shapers, WorkItemState,
};
use buzz_core::tenant::TenantContext;
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{
    self as store, LedgerEntry, SchedulerClaimKind, WorkItemRow, LEDGER_ACTOR_RELAY,
};
use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::apply::{self, ApplyContext, Projection};
use super::object;
use super::proposals;
use super::work;
use crate::state::AppState;

/// Default tick interval — Protocol §6.3 "every five minutes".
const DEFAULT_INTERVAL_SECS: u64 = 300;

/// Floor for the review window: two days in seconds (§5.1 / §6.3).
const REVIEW_FLOOR_SECS: u64 = 172_800;

/// Wall clock for a tick. Tests and short-window configs may set
/// `IO_SCHEDULER_NOW_OVERRIDE` to a unix-seconds string (V9).
pub fn scheduler_now() -> DateTime<Utc> {
    if let Ok(raw) = std::env::var("IO_SCHEDULER_NOW_OVERRIDE") {
        if let Ok(secs) = raw.parse::<i64>() {
            if let Some(ts) = DateTime::from_timestamp(secs, 0) {
                return ts;
            }
        }
    }
    Utc::now()
}

fn secs(dt: DateTime<Utc>) -> u64 {
    dt.timestamp().unsigned_abs()
}

fn hex_id(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Run the scheduler forever. Intended for `tokio::spawn`.
pub async fn run(state: Arc<AppState>) {
    let interval_secs: u64 = std::env::var("BUZZ_IO_SCHEDULER_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS);
    info!(interval_secs, "io_scheduler started");
    loop {
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        let now = scheduler_now();
        if let Err(e) = sweep_all(&state, now).await {
            error!("io_scheduler tick failed: {e}");
        }
    }
}

/// One tick across every org community. Public for the Postgres lane.
pub async fn sweep_all(state: &Arc<AppState>, now: DateTime<Utc>) -> Result<(), buzz_db::DbError> {
    let mut tx = state.db.begin_event_write_transaction().await?;
    let communities = store::list_org_communities(&mut tx).await?;
    tx.commit().await?;
    for community in communities {
        let host = match state.db.lookup_community_host(community).await? {
            Some(host) => host,
            None => {
                warn!(%community, "io_scheduler: community host missing; skipping");
                continue;
            }
        };
        let tenant = TenantContext::resolved(community, host);
        if let Err(e) = sweep_community(state, &tenant, now).await {
            error!(%community, "io_scheduler community sweep failed: {e}");
        }
    }
    Ok(())
}

/// Run every §6.3 step for one community. Public for tests.
pub async fn sweep_community(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    sweep_offers(state, tenant, now).await?;
    sweep_enter_review(state, tenant, now).await?;
    sweep_close_due(state, tenant, now).await?;
    sweep_drafts(state, tenant, now).await?;
    sweep_proposals(state, tenant, now).await?;
    sweep_seat_lapses(state, tenant, now).await?;
    Ok(())
}

async fn load_shapers(
    tx: &mut Transaction<'static, Postgres>,
    community: CommunityId,
) -> Result<Option<Shapers>, String> {
    store::get_shapers(tx, community)
        .await
        .map(|row| row.map(|r| r.content))
        .map_err(|e| format!("read io_shapers: {e}"))
}

async fn begin_transition(
    state: &AppState,
    community: CommunityId,
) -> Result<Transaction<'static, Postgres>, String> {
    let mut tx = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin tx: {e}"))?;
    store::lock_executor(&mut tx, community)
        .await
        .map_err(|e| format!("lock executor: {e}"))?;
    Ok(tx)
}

struct PendingLedger {
    verb: &'static str,
    object_type: &'static str,
    object_id: String,
    detail: serde_json::Value,
    /// Index into `applied.state_events` whose id becomes `receipt_event_id`.
    state_index: usize,
}

async fn commit_transition(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    mut tx: Transaction<'static, Postgres>,
    projections: Vec<Projection>,
    pending: Vec<PendingLedger>,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let relay_bytes = state.relay_keypair.public_key().to_bytes();
    let ctx = ApplyContext {
        community: tenant.community(),
        relay: &state.relay_keypair,
        actor: &relay_bytes,
        now: secs(now),
    };
    let applied = apply::apply(&state.db, &mut tx, &ctx, &projections, &[])
        .await
        .map_err(|e| format!("apply: {e:?}"))?;
    for entry in pending {
        let receipt = applied
            .state_events
            .get(entry.state_index)
            .map(|s| s.event.id.to_bytes().to_vec());
        let row = LedgerEntry {
            at: now,
            actor: LEDGER_ACTOR_RELAY.to_owned(),
            verb: entry.verb.to_owned(),
            object_type: entry.object_type.to_owned(),
            object_id: entry.object_id,
            receipt_event_id: receipt,
            detail: entry.detail,
        };
        store::insert_ledger(&mut tx, tenant.community(), &row)
            .await
            .map_err(|e| format!("ledger: {e}"))?;
    }
    tx.commit().await.map_err(|e| format!("commit: {e}"))?;
    finish_scheduler(state, tenant, applied).await;
    Ok(())
}

async fn finish_scheduler(state: &Arc<AppState>, tenant: &TenantContext, applied: apply::Applied) {
    use buzz_core::kind::{KIND_MEMBER_ADDED_NOTIFICATION, KIND_MEMBER_REMOVED_NOTIFICATION};
    use buzz_db::relay_rooms::RosterChange;

    let relay_hex = state.relay_keypair.public_key().to_hex();
    for stored in &applied.state_events {
        super::super::event::dispatch_persistent_event(
            tenant,
            state,
            stored,
            stored.event.kind.as_u16() as u32,
            &relay_hex,
            None,
        )
        .await;
    }
    let mut rooms: Vec<Uuid> = Vec::new();
    for (room, change) in &applied.roster {
        if !rooms.contains(room) {
            rooms.push(*room);
        }
        state.invalidate_membership(tenant, *room, change.pubkey());
        let notification = match change {
            RosterChange::Added { .. } => KIND_MEMBER_ADDED_NOTIFICATION,
            RosterChange::Removed { .. } => KIND_MEMBER_REMOVED_NOTIFICATION,
        };
        let actor = state.relay_keypair.public_key().to_bytes();
        if let Err(e) = super::super::side_effects::emit_membership_notification(
            tenant,
            state,
            *room,
            change.pubkey(),
            &actor,
            notification,
        )
        .await
        {
            warn!(room = %room, error = %e, "io_scheduler: membership notification failed");
        }
    }
    for room in rooms {
        if let Err(e) =
            super::super::side_effects::emit_group_discovery_events(tenant, state, room).await
        {
            warn!(room = %room, error = %e, "io_scheduler: discovery update failed");
        }
    }
}

fn prior_receipt(row: &WorkItemRow) -> String {
    hex_id(&row.event_id)
}

// ── Offers ────────────────────────────────────────────────────────────────────

async fn sweep_offers(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut probe = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin probe: {e}"))?;
    let shapers = store::get_shapers(&mut probe, tenant.community())
        .await
        .map_err(|e| format!("read shapers: {e}"))?
        .map(|r| r.content);
    let Some(shapers) = shapers else {
        probe.commit().await.ok();
        return Ok(());
    };
    let window = shapers.offer_window_secs;
    let half = window / 2;
    // Candidates past the half-window cut; renotify vs expire decided below.
    let half_cut = now - chrono::Duration::seconds(half as i64);
    let candidates = store::list_offered_items_past(&mut probe, tenant.community(), half_cut)
        .await
        .map_err(|e| format!("list offered: {e}"))?;
    probe.commit().await.ok();

    for row in candidates {
        let Some(offered_at) = row.content.offered_at else {
            continue;
        };
        let age = secs(now).saturating_sub(offered_at);
        if age >= window {
            try_offer_expire(state, tenant, &row, now).await?;
        } else if age >= half {
            try_offer_renotify(state, tenant, &row, now).await?;
        }
    }
    Ok(())
}

async fn try_offer_renotify(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    row: &WorkItemRow,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let Some(offered_at) = row.content.offered_at else {
        return Ok(());
    };
    let epoch = store::ts(offered_at).map_err(|e| format!("offered_at: {e}"))?;
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::OfferRenotify,
        &row.content.id,
        epoch,
    )
    .await
    .map_err(|e| format!("claim renotify: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    // Re-verify still offered with the same offered_at.
    let live = store::get_work_item(&mut tx, tenant.community(), {
        Uuid::parse_str(&row.content.id).map_err(|e| format!("item id: {e}"))?
    })
    .await
    .map_err(|e| format!("re-read item: {e}"))?;
    let Some(live) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    if live.content.state != WorkItemState::Offered || live.content.offered_at != Some(offered_at) {
        tx.rollback().await.ok();
        return Ok(());
    }
    let receipt = prior_receipt(&live);
    let pending = vec![PendingLedger {
        verb: "offer_renotified",
        object_type: object::WORK_ITEM,
        object_id: live.content.id.clone(),
        detail: serde_json::json!({ "offered_to": live.content.offered_to }),
        state_index: 0,
    }];
    // Unchanged content; new created_at via apply's head ratchet.
    commit_transition(
        state,
        tenant,
        tx,
        vec![work::work_item_projection(live.content, receipt)],
        pending,
        now,
    )
    .await
}

async fn try_offer_expire(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    row: &WorkItemRow,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let Some(offered_at) = row.content.offered_at else {
        return Ok(());
    };
    let epoch = store::ts(offered_at).map_err(|e| format!("offered_at: {e}"))?;
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::OfferExpire,
        &row.content.id,
        epoch,
    )
    .await
    .map_err(|e| format!("claim expire: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let id = Uuid::parse_str(&row.content.id).map_err(|e| format!("item id: {e}"))?;
    let live = store::get_work_item(&mut tx, tenant.community(), id)
        .await
        .map_err(|e| format!("re-read item: {e}"))?;
    let Some(live) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    if live.content.state != WorkItemState::Offered || live.content.offered_at != Some(offered_at) {
        tx.rollback().await.ok();
        return Ok(());
    }
    let receipt = prior_receipt(&live);
    let from = live.content.state;
    let mut next = live.content;
    next.state = WorkItemState::Open;
    next.dri = None;
    next.offered_to = None;
    next.offered_by = None;
    next.offered_at = None;
    let mut projections = vec![work::work_item_projection(next.clone(), receipt.clone())];
    if let Some(parent_id) = next.parent.as_deref() {
        let parent_uuid = Uuid::parse_str(parent_id).map_err(|e| format!("parent id: {e}"))?;
        if let Some(parent) = store::get_work_item(&mut tx, tenant.community(), parent_uuid)
            .await
            .map_err(|e| format!("read parent: {e}"))?
        {
            projections.push(work::work_item_projection(
                work::parent_with_child_delta(
                    parent.content,
                    Some(from),
                    Some(WorkItemState::Open),
                ),
                hex_id(&parent.event_id),
            ));
        }
    }
    let pending = vec![PendingLedger {
        verb: "offer_expired",
        object_type: object::WORK_ITEM,
        object_id: next.id,
        detail: serde_json::json!({}),
        state_index: 0,
    }];
    commit_transition(state, tenant, tx, projections, pending, now).await
}

// ── Review window ──────────────────────────────────────────────────────────────

async fn sweep_enter_review(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut probe = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin probe: {e}"))?;
    let candidates = store::list_roots_entering_review(&mut probe, tenant.community(), now)
        .await
        .map_err(|e| format!("list review: {e}"))?;
    probe.commit().await.ok();
    for row in candidates {
        try_enter_review(state, tenant, &row, now).await?;
    }
    Ok(())
}

fn review_threshold(approved_at: u64, due_at: u64) -> u64 {
    let span = due_at.saturating_sub(approved_at);
    let fifth = ((span as f64) * 0.2).floor() as u64;
    due_at.saturating_sub(fifth.max(REVIEW_FLOOR_SECS))
}

async fn try_enter_review(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    row: &WorkItemRow,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let Some(approved_at) = row.content.approved_at else {
        return Ok(());
    };
    let epoch = store::ts(row.content.due_at).map_err(|e| format!("due_at: {e}"))?;
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::EnterReview,
        &row.content.id,
        epoch,
    )
    .await
    .map_err(|e| format!("claim review: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let id = Uuid::parse_str(&row.content.id).map_err(|e| format!("item id: {e}"))?;
    let live = store::get_work_item(&mut tx, tenant.community(), id)
        .await
        .map_err(|e| format!("re-read item: {e}"))?;
    let Some(live) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    let item = &live.content;
    if item.state != WorkItemState::Accepted
        || item.parent.is_some()
        || item.approved_at != Some(approved_at)
        || item.due_at != row.content.due_at
        || secs(now) < review_threshold(approved_at, item.due_at)
        || secs(now) >= item.due_at
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let receipt = prior_receipt(&live);
    let mut next = live.content;
    next.state = WorkItemState::InReview;
    let pending = vec![PendingLedger {
        verb: "item_entered_review",
        object_type: object::WORK_ITEM,
        object_id: next.id.clone(),
        detail: serde_json::json!({}),
        state_index: 0,
    }];
    commit_transition(
        state,
        tenant,
        tx,
        vec![work::work_item_projection(next, receipt)],
        pending,
        now,
    )
    .await
}

// ── Close on due_at ───────────────────────────────────────────────────────────

async fn sweep_close_due(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut probe = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin probe: {e}"))?;
    let candidates = store::list_roots_due_for_close(&mut probe, tenant.community(), now)
        .await
        .map_err(|e| format!("list close: {e}"))?;
    probe.commit().await.ok();
    for row in candidates {
        try_close_due(state, tenant, &row, now).await?;
    }
    Ok(())
}

async fn try_close_due(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    row: &WorkItemRow,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let epoch = store::ts(row.content.due_at).map_err(|e| format!("due_at: {e}"))?;
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::CloseDue,
        &row.content.id,
        epoch,
    )
    .await
    .map_err(|e| format!("claim close: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let id = Uuid::parse_str(&row.content.id).map_err(|e| format!("item id: {e}"))?;
    let live = store::get_work_item(&mut tx, tenant.community(), id)
        .await
        .map_err(|e| format!("re-read item: {e}"))?;
    let Some(live) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    let item = &live.content;
    if item.parent.is_some()
        || item.due_at != row.content.due_at
        || secs(now) < item.due_at
        || !matches!(
            item.state,
            WorkItemState::Accepted | WorkItemState::InReview
        )
    {
        // Moved due_at or already closed — claim stands, transition skipped.
        tx.rollback().await.ok();
        return Ok(());
    }

    let children = store::list_children(&mut tx, tenant.community(), id)
        .await
        .map_err(|e| format!("list children: {e}"))?;
    let mut root = live.content.clone();
    let mut projections = Vec::new();
    let mut pending = Vec::new();

    for child_row in &children {
        let Some((from, orphaned)) = work::orphan_child_for_closed_root(child_row.content.clone())
        else {
            continue;
        };
        root = work::parent_with_child_delta(root, Some(from), Some(WorkItemState::Open));
        let child_index = projections.len();
        projections.push(work::work_item_projection(
            orphaned.clone(),
            hex_id(&child_row.event_id),
        ));
        pending.push(PendingLedger {
            verb: "orphaned_by_close",
            object_type: object::WORK_ITEM,
            object_id: orphaned.id,
            detail: serde_json::json!({ "root": root.id, "from": from }),
            state_index: child_index,
        });
    }

    root.state = WorkItemState::Done;
    root.closed_by = Some(ClosedBy::Rule);
    let root_index = projections.len();
    projections.push(work::work_item_projection(
        root.clone(),
        prior_receipt(&live),
    ));
    pending.push(PendingLedger {
        verb: "item_closed_by_rule",
        object_type: object::WORK_ITEM,
        object_id: root.id,
        detail: serde_json::json!({ "due_at": live.content.due_at }),
        state_index: root_index,
    });

    commit_transition(state, tenant, tx, projections, pending, now).await
}

// ── Drafts ────────────────────────────────────────────────────────────────────

async fn sweep_drafts(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut probe = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin probe: {e}"))?;
    let candidates = store::list_drafts_expiring(
        &mut probe,
        tenant.community(),
        DraftOutcomeStatus::Open,
        now,
    )
    .await
    .map_err(|e| format!("list drafts: {e}"))?;
    probe.commit().await.ok();
    for row in candidates {
        try_draft_expire(state, tenant, &row, now).await?;
    }
    Ok(())
}

async fn try_draft_expire(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    row: &store::DraftRow,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let Some(expires_at) = row.expires_at else {
        return Ok(());
    };
    let object_id = hex_id(&row.event_id);
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::DraftExpire,
        &object_id,
        expires_at,
    )
    .await
    .map_err(|e| format!("claim draft: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let live = store::get_draft(&mut tx, tenant.community(), &row.event_id)
        .await
        .map_err(|e| format!("re-read draft: {e}"))?;
    let Some(live) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    if live.outcome.status != DraftOutcomeStatus::Open
        || live.expires_at != Some(expires_at)
        || expires_at > now
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let outcome = DraftOutcome {
        draft: object_id.clone(),
        status: DraftOutcomeStatus::Expired,
        // §4.6 / table CHECK: decided_at is required only for
        // accepted/amended/declined; expired keeps both fields null.
        decided_by: None,
        decided_at: None,
        reason: None,
        result: None,
    };
    let pending = vec![PendingLedger {
        verb: "draft_decided",
        object_type: object::DRAFT,
        object_id: object_id.clone(),
        detail: serde_json::json!({ "status": "expired" }),
        state_index: 0,
    }];
    commit_transition(
        state,
        tenant,
        tx,
        vec![Projection::Draft {
            outcome,
            insert: None,
        }],
        pending,
        now,
    )
    .await
}

// ── Proposals ─────────────────────────────────────────────────────────────────

async fn sweep_proposals(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut probe = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin probe: {e}"))?;
    let candidates =
        store::list_proposals_expiring(&mut probe, tenant.community(), ProposalStatus::Open, now)
            .await
            .map_err(|e| format!("list proposals: {e}"))?;
    probe.commit().await.ok();
    for row in candidates {
        try_proposal_expire(state, tenant, &row, now).await?;
    }
    Ok(())
}

async fn try_proposal_expire(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    row: &store::ProposalRow,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&row.content.id).map_err(|e| format!("proposal id: {e}"))?;
    let epoch = store::ts(row.content.expires_at).map_err(|e| format!("expires_at: {e}"))?;
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::ProposalExpire,
        &row.content.id,
        epoch,
    )
    .await
    .map_err(|e| format!("claim proposal: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let live = store::get_proposal(&mut tx, tenant.community(), id)
        .await
        .map_err(|e| format!("re-read proposal: {e}"))?;
    let Some(live) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    if live.content.status != ProposalStatus::Open
        || live.content.expires_at != row.content.expires_at
        || secs(now) < live.content.expires_at
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let opening_receipt = store::get_proposal_opening_receipt(&mut tx, tenant.community(), id)
        .await
        .map_err(|e| format!("opening receipt: {e}"))?
        .unwrap_or_else(|| hex_id(&live.event_id));
    let (subject, item) = proposals::markers(&live.content);
    let mut proposal = live.content;
    let opened_by = proposal.opened_by.clone();
    proposal.status = ProposalStatus::Expired;
    proposal.decided_at = Some(secs(now));
    let pending = vec![PendingLedger {
        verb: "proposal_expired",
        object_type: object::PROPOSAL,
        object_id: proposal.id.clone(),
        detail: serde_json::json!({ "opened_by": opened_by }),
        state_index: 0,
    }];
    commit_transition(
        state,
        tenant,
        tx,
        vec![Projection::Proposal {
            proposal: Box::new(proposal),
            subject,
            item,
            receipt: opening_receipt,
            cast: vec![],
        }],
        pending,
        now,
    )
    .await
}

// ── Seat lapse ────────────────────────────────────────────────────────────────

async fn sweep_seat_lapses(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut probe = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin probe: {e}"))?;
    let shapers = store::get_shapers(&mut probe, tenant.community())
        .await
        .map_err(|e| format!("read shapers: {e}"))?;
    probe.commit().await.ok();
    let Some(row) = shapers else {
        return Ok(());
    };
    let window = row.content.offer_window_secs;
    let lapsed: Vec<_> = row
        .content
        .offered
        .iter()
        .filter(|seat| secs(now).saturating_sub(seat.at) >= window)
        .cloned()
        .collect();
    for seat in lapsed {
        try_seat_lapse(state, tenant, &seat, now).await?;
    }
    Ok(())
}

async fn try_seat_lapse(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    seat: &buzz_core::intelligent_org::OfferedSeat,
    now: DateTime<Utc>,
) -> Result<(), String> {
    // Drop by (p, proposal), not by p — a fresh seat for the same p must survive.
    let object_id = format!("{}:{}", seat.p, seat.proposal);
    let epoch = store::ts(seat.at).map_err(|e| format!("seat.at: {e}"))?;
    let mut tx = begin_transition(state, tenant.community()).await?;
    if !store::claim_scheduler_transition(
        &mut tx,
        tenant.community(),
        SchedulerClaimKind::SeatLapse,
        &object_id,
        epoch,
    )
    .await
    .map_err(|e| format!("claim seat: {e}"))?
    {
        tx.rollback().await.ok();
        return Ok(());
    }
    let live = load_shapers(&mut tx, tenant.community()).await?;
    let Some(mut shapers) = live else {
        tx.rollback().await.ok();
        return Ok(());
    };
    let before = shapers.offered.len();
    shapers
        .offered
        .retain(|s| !(s.p == seat.p && s.proposal == seat.proposal));
    if shapers.offered.len() == before {
        // Already gone (accepted, or replaced) — claim stands.
        tx.rollback().await.ok();
        return Ok(());
    }
    // Chain the version's receipt to the previous 39103 head.
    let prev = store::get_shapers(&mut tx, tenant.community())
        .await
        .map_err(|e| format!("re-read shapers row: {e}"))?
        .map(|r| hex_id(&r.event_id))
        .unwrap_or_else(|| shapers.receipt.clone());
    shapers.receipt = prev;
    shapers.updated_at = secs(now);
    let pending = vec![PendingLedger {
        verb: "shaper_offer_lapsed",
        object_type: object::SHAPERS,
        object_id: seat.p.clone(),
        detail: serde_json::json!({ "proposal": seat.proposal, "at": seat.at }),
        state_index: 0,
    }];
    commit_transition(
        state,
        tenant,
        tx,
        vec![Projection::Shapers(shapers)],
        pending,
        now,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_threshold_uses_last_fifth_with_two_day_floor() {
        // 10-day span → fifth = 2 days; floor also 2 days → review at due − 2d.
        let approved = 1_000_000;
        let due = approved + 10 * 86_400;
        assert_eq!(review_threshold(approved, due), due - REVIEW_FLOOR_SECS);
        // 50-day span → fifth = 10 days > floor.
        let due_long = approved + 50 * 86_400;
        assert_eq!(review_threshold(approved, due_long), due_long - 10 * 86_400);
    }
}
