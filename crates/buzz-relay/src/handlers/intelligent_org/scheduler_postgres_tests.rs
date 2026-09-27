//! R-6 proofs at the production seam: every transition goes through
//! [`super::scheduler::sweep_community`] with a DB claim, then asserts on
//! the live `39101`/`39102`/`39103`/`39104` and `io_ledger` the relay serves.

use std::sync::Arc;

use buzz_core::intelligent_org::{ClosedBy, DraftOutcomeStatus, ProposalStatus, WorkItemState};
use buzz_core::kind::{
    KIND_IO_DRAFT, KIND_IO_DRAFT_OUTCOME, KIND_IO_PROPOSAL, KIND_IO_SHAPERS, KIND_IO_WORK_ITEM,
};
use buzz_db::intelligent_org::{self as store, LEDGER_ACTOR_RELAY};
use chrono::{Duration, Utc};
use nostr::Keys;
use uuid::Uuid;

use super::postgres_tests::{harness, signed, tag, Harness};
use super::scheduler;

impl Harness {
    async fn backdate_offer(&self, id: &str, offered_at: i64) {
        let uuid = Uuid::parse_str(id).expect("uuid");
        sqlx::query(
            "UPDATE io_work_items SET offered_at = to_timestamp($3), \
                content = jsonb_set(content, '{offered_at}', to_jsonb($3::bigint)) \
             WHERE community_id = $1 AND id = $2",
        )
        .bind(self.community().as_uuid())
        .bind(uuid)
        .bind(offered_at)
        .execute(&self.pool)
        .await
        .expect("backdate offer");
    }

    async fn backdate_root_dates(&self, id: &str, approved_at: i64, due_at: i64) {
        let uuid = Uuid::parse_str(id).expect("uuid");
        sqlx::query(
            "UPDATE io_work_items SET \
                approved_at = to_timestamp($3), due_at = to_timestamp($4), \
                content = jsonb_set( \
                    jsonb_set(content, '{approved_at}', to_jsonb($3::bigint)), \
                    '{due_at}', to_jsonb($4::bigint)) \
             WHERE community_id = $1 AND id = $2",
        )
        .bind(self.community().as_uuid())
        .bind(uuid)
        .bind(approved_at)
        .bind(due_at)
        .execute(&self.pool)
        .await
        .expect("backdate root dates");
    }

    async fn set_due_sql(&self, id: &str, due_at: i64) {
        let uuid = Uuid::parse_str(id).expect("uuid");
        sqlx::query(
            "UPDATE io_work_items SET due_at = to_timestamp($3), \
                content = jsonb_set(content, '{due_at}', to_jsonb($3::bigint)) \
             WHERE community_id = $1 AND id = $2",
        )
        .bind(self.community().as_uuid())
        .bind(uuid)
        .bind(due_at)
        .execute(&self.pool)
        .await
        .expect("move due_at");
    }

    async fn ledger_rows_for(&self, object_id: &str) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT verb, actor FROM io_ledger \
             WHERE community_id = $1 AND object_id = $2 ORDER BY id",
        )
        .bind(self.community().as_uuid())
        .bind(object_id)
        .fetch_all(&self.pool)
        .await
        .expect("ledger rows")
    }

    async fn claim_count(&self, kind: &str, object_id: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM io_scheduler_claims \
             WHERE community_id = $1 AND kind = $2 AND object_id = $3",
        )
        .bind(self.community().as_uuid())
        .bind(kind)
        .bind(object_id)
        .fetch_one(&self.pool)
        .await
        .expect("claim count")
    }
}

async fn short_offer_window(h: &Harness) {
    // One Shaper: opener agree passes. Shrink the offer window so half/full
    // cuts are easy to backdate against.
    h.propose(
        &h.owner,
        "rules",
        None,
        r#"{"rules":{"shapers":1},"offer_window_secs":100}"#,
        true,
    )
    .await
    .expect("shrink offer window");
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn offer_renotify_and_expire_once_under_concurrent_sweeps() {
    let h = harness().await;
    h.bootstrap().await;
    short_offer_window(&h).await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;

    let (_, root) = h
        .pass_project(
            &h.owner,
            &format!(
                r#"{{"title":"Hall","brief":"Book it","due_at":1800000000,"suggested_dri":"{member_hex}"}}"#
            ),
        )
        .await;
    assert_eq!(root.state, WorkItemState::Offered);

    let now = Utc::now();
    let now_secs = now.timestamp();
    // Past half-window (50s of 100), still inside the full window.
    h.backdate_offer(&root.id, now_secs - 60).await;

    let state = Arc::clone(&h.state);
    let tenant = h.tenant.clone();
    let a = tokio::spawn({
        let state = Arc::clone(&state);
        let tenant = tenant.clone();
        async move { scheduler::sweep_community(&state, &tenant, now).await }
    });
    let b = tokio::spawn({
        let state = Arc::clone(&state);
        let tenant = tenant.clone();
        async move { scheduler::sweep_community(&state, &tenant, now).await }
    });
    a.await.expect("join a").expect("sweep a");
    b.await.expect("join b").expect("sweep b");

    let item = h.work_item(&root.id).await.expect("item");
    assert_eq!(item.state, WorkItemState::Offered, "renotify keeps offered");
    let rows = h.ledger_rows_for(&root.id).await;
    let renotifies: Vec<_> = rows
        .iter()
        .filter(|(v, _)| v == "offer_renotified")
        .collect();
    assert_eq!(renotifies.len(), 1, "exactly one renotify: {rows:?}");
    assert_eq!(renotifies[0].1, LEDGER_ACTOR_RELAY);
    assert_eq!(h.claim_count("offer_renotify", &root.id).await, 1);
    // Parameterized replace retires the prior head; the live count stays one.
    let live = h.live_state(KIND_IO_WORK_ITEM).await;
    assert!(
        live.iter()
            .any(|(_, c, _)| c["id"] == root.id && c["state"] == "offered"),
        "live 39101 still offered after renotify"
    );

    // Past the full window → expire once, even under two sweeps.
    let later = now + Duration::seconds(50);
    h.backdate_offer(&root.id, later.timestamp() - 100).await;
    let a = tokio::spawn({
        let state = Arc::clone(&state);
        let tenant = tenant.clone();
        async move { scheduler::sweep_community(&state, &tenant, later).await }
    });
    let b = tokio::spawn({
        let state = Arc::clone(&state);
        let tenant = tenant.clone();
        async move { scheduler::sweep_community(&state, &tenant, later).await }
    });
    a.await.expect("join a").expect("sweep a");
    b.await.expect("join b").expect("sweep b");

    let item = h.work_item(&root.id).await.expect("item");
    assert_eq!(item.state, WorkItemState::Open);
    assert!(item.offered_to.is_none());
    let rows = h.ledger_rows_for(&root.id).await;
    assert_eq!(
        rows.iter().filter(|(v, _)| v == "offer_expired").count(),
        1,
        "exactly one expire: {rows:?}"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn root_enters_review_and_closes_with_orphaned_children_moved_due_skips() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;

    let (_, root) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Hall","brief":"Book it","due_at":1800000000}"#,
        )
        .await;
    h.offer_item(&h.owner, &root.id, &member_hex)
        .await
        .expect("offer");
    h.accept_item(&member, &root.id).await.expect("accept");

    // Child under the held root.
    let child_reply = h
        .ticket(
            &member,
            &root.id,
            None,
            r#"{"title":"Permit","brief":"Get it","due_at":1800000001}"#,
        )
        .await
        .expect("create child");
    let child_id = child_reply["item"].as_str().expect("child id").to_owned();

    let now = Utc::now();
    let now_secs = now.timestamp();
    // 10-day span; review floor = 2 days → review at due − 2d.
    let approved = now_secs - 8 * 86_400;
    let due = now_secs + 2 * 86_400;
    h.backdate_root_dates(&root.id, approved, due).await;

    scheduler::sweep_community(&h.state, &h.tenant, now)
        .await
        .expect("enter review");
    let item = h.work_item(&root.id).await.expect("item");
    assert_eq!(item.state, WorkItemState::InReview);
    let rows = h.ledger_rows_for(&root.id).await;
    assert!(
        rows.iter()
            .any(|(v, a)| v == "item_entered_review" && a == LEDGER_ACTOR_RELAY),
        "review ledger: {rows:?}"
    );

    // Move due_at into the future before close would fire — must not close.
    let moved_due = now_secs + 30 * 86_400;
    h.set_due_sql(&root.id, moved_due).await;
    // Also put the old due into the past on a claim epoch that would have
    // closed: sweep at `due` must no-op because live due_at moved.
    scheduler::sweep_community(
        &h.state,
        &h.tenant,
        now + Duration::seconds(due - now_secs + 1),
    )
    .await
    .expect("sweep after moved due");
    let item = h.work_item(&root.id).await.expect("item");
    assert_eq!(
        item.state,
        WorkItemState::InReview,
        "moved due_at is not closed"
    );
    assert_eq!(item.due_at, moved_due as u64);

    // Close on the live due_at: backdate due to now, sweep.
    h.set_due_sql(&root.id, now_secs - 1).await;
    // Re-enter accepted/in_review path — already in_review.
    scheduler::sweep_community(&h.state, &h.tenant, now)
        .await
        .expect("close");
    let item = h.work_item(&root.id).await.expect("item");
    assert_eq!(item.state, WorkItemState::Done);
    assert_eq!(item.closed_by, Some(ClosedBy::Rule));
    let child = h.work_item(&child_id).await.expect("child");
    assert_eq!(child.state, WorkItemState::Open);
    let child_rows = h.ledger_rows_for(&child_id).await;
    assert!(
        child_rows
            .iter()
            .any(|(v, a)| v == "orphaned_by_close" && a == LEDGER_ACTOR_RELAY),
        "orphan ledger: {child_rows:?}"
    );
    let root_rows = h.ledger_rows_for(&root.id).await;
    assert!(
        root_rows
            .iter()
            .any(|(v, a)| v == "item_closed_by_rule" && a == LEDGER_ACTOR_RELAY),
        "close ledger: {root_rows:?}"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn draft_and_proposal_expire_and_seat_lapses_by_proposal() {
    let h = harness().await;
    h.bootstrap().await;
    short_offer_window(&h).await;

    // Open a direction proposal with a short decision window by SQL-backdating
    // expires_at after opening under the default window.
    let opened = h
        .direction(
            &h.owner,
            "mission",
            0,
            r#"{"body":"Ship the hall","lines":[]}"#,
            false,
        )
        .await
        .expect("open direction");
    let proposal_id = opened["proposal"].as_str().expect("id").to_owned();
    let now = Utc::now();
    let expired_at = now - Duration::seconds(10);
    sqlx::query(
        "UPDATE io_proposals SET expires_at = $3, \
            content = jsonb_set(content, '{expires_at}', to_jsonb($4::bigint)) \
         WHERE community_id = $1 AND id = $2",
    )
    .bind(h.community().as_uuid())
    .bind(Uuid::parse_str(&proposal_id).unwrap())
    .bind(expired_at)
    .bind(expired_at.timestamp())
    .execute(&h.pool)
    .await
    .expect("backdate proposal");

    // Seed an open draft past expiration via the projection row; the
    // scheduler writes the expired `39104` through `apply`.
    let draft_event = signed(
        &h.agent,
        KIND_IO_DRAFT,
        vec![
            tag(["n", &h.owner.public_key().to_hex()]),
            tag(["t", "project"]),
            tag(["move", "1"]),
            tag(["origin", "gap"]),
            tag(["gap", "test-gap-r6"]),
            tag(["expiration", &(expired_at.timestamp() as u64).to_string()]),
        ],
        r#"{"title":"t","brief":"b","objective_ref":null,"due_at":1,"suggested_dri":null,"why":"w","gaps":[]}"#,
    );
    let mut conn = h.pool.acquire().await.expect("acquire");
    store::insert_draft(
        &mut conn,
        h.community(),
        &store::DraftRow {
            event_id: draft_event.id.to_bytes().to_vec(),
            author: h.agent.public_key().to_bytes().to_vec(),
            draft_kind: buzz_core::intelligent_org::DraftKind::Project,
            needs: h.owner.public_key().to_hex(),
            gap: "test-gap-r6".into(),
            r#move: Some(1),
            origin: Some(buzz_core::intelligent_org::DraftOrigin::Gap),
            item_id: None,
            parent_id: None,
            shadow: false,
            expires_at: Some(expired_at),
            payload: serde_json::json!({
                "title":"t","brief":"b","objective_ref":null,"due_at":1,
                "suggested_dri":null,"why":"w","gaps":[]
            }),
            outcome: buzz_core::intelligent_org::DraftOutcome {
                draft: draft_event.id.to_hex(),
                status: DraftOutcomeStatus::Open,
                decided_by: None,
                decided_at: None,
                reason: None,
                result: None,
            },
            outcome_event_id: None,
            created_at: expired_at,
        },
    )
    .await
    .expect("seed draft");
    drop(conn);

    // Offer a seat, then backdate it past the 100s window.
    let invitee = Keys::generate();
    h.member(&invitee).await;
    let seat_proposal = h.offer_seat(&invitee).await;
    let now_secs = now.timestamp();
    // Backdate the seat's `at` inside 39103 content.
    let mut shapers = h.shapers_content().await.expect("shapers");
    for seat in &mut shapers.offered {
        if seat.proposal == seat_proposal {
            seat.at = (now_secs - 200) as u64;
        }
    }
    // Rewrite via SQL content + leave event_id; scheduler reads content.offered.
    sqlx::query("UPDATE io_shapers SET content = $2 WHERE community_id = $1")
        .bind(h.community().as_uuid())
        .bind(serde_json::to_value(&shapers).unwrap())
        .execute(&h.pool)
        .await
        .expect("backdate seat");

    scheduler::sweep_community(&h.state, &h.tenant, now)
        .await
        .expect("expire sweep");

    let prop = h.proposal(&proposal_id).await;
    assert_eq!(prop.status, ProposalStatus::Expired);
    let prop_rows = h.ledger_rows_for(&proposal_id).await;
    assert!(
        prop_rows
            .iter()
            .any(|(v, a)| v == "proposal_expired" && a == LEDGER_ACTOR_RELAY),
        "{prop_rows:?}"
    );
    let live_prop = h.live_state(KIND_IO_PROPOSAL).await;
    assert!(
        live_prop
            .iter()
            .any(|(_, c, _)| c["id"] == proposal_id && c["status"] == "expired"),
        "39102 expired"
    );

    let draft_hex = draft_event.id.to_hex();
    let mut conn = h.pool.acquire().await.expect("acquire");
    let draft = store::get_draft(&mut conn, h.community(), draft_event.id.as_bytes())
        .await
        .expect("read draft")
        .expect("draft row");
    assert_eq!(draft.outcome.status, DraftOutcomeStatus::Expired);
    drop(conn);
    let live_draft = h.live_state(KIND_IO_DRAFT_OUTCOME).await;
    assert!(
        live_draft
            .iter()
            .any(|(_, c, _)| c["draft"] == draft_hex && c["status"] == "expired"),
        "39104 expired"
    );

    let shapers = h.shapers_content().await.expect("shapers");
    assert!(
        !shapers.offered.iter().any(|s| s.proposal == seat_proposal),
        "lapsed seat dropped"
    );
    assert!(
        h.ledger_verbs()
            .await
            .iter()
            .any(|v| v == "shaper_offer_lapsed"),
        "seat lapse ledger"
    );
    let _ = (KIND_IO_SHAPERS, KIND_IO_WORK_ITEM);
}
