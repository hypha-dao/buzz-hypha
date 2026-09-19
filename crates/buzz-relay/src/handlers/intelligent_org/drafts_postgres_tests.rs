//! R-7 proofs at the production seam: every `50100` / `50101` / `50103` /
//! `50012` / `50017` and every draft-tagged command enters through
//! [`crate::handlers::ingest::ingest_event`].

use buzz_core::intelligent_org::{DraftOutcomeStatus, OrgProfile, Skill};
use buzz_core::kind::{
    KIND_IO_AGENT_NOTE, KIND_IO_DRAFT, KIND_IO_DRAFT_DECIDE, KIND_IO_DRAFT_OUTCOME, KIND_IO_HEALTH,
    KIND_IO_HEALTH_RATE, KIND_IO_PROFILE, KIND_IO_PROJECT_PROPOSE,
};
use buzz_db::intelligent_org::{self as store, ProfileRow};
use nostr::{Event, EventId, Keys, Tag};
use uuid::Uuid;

use super::postgres_tests::{harness, rejected, signed, tag, Harness};
const PROJECT: &str = r#"{
  "title":"Hall","brief":"A weekday hall","objective_ref":null,
  "due_at":1800000000,"suggested_dri":null,"why":"one line","gaps":[]
}"#;

const PROJECT_EDITED: &str = r#"{
  "title":"Hall v2","brief":"A weekday hall","objective_ref":null,
  "due_at":1800000000,"suggested_dri":null,"why":"one line","gaps":[]
}"#;

fn draft_event(keys: &Keys, needs: &str, gap: &str, receipt: &str, content: &str) -> Event {
    let mut tags = vec![
        tag(["n", needs]),
        tag(["t", "project"]),
        tag(["move", "1"]),
        tag(["origin", "gap"]),
        tag(["gap", gap]),
        tag(["e", receipt, "", "receipt"]),
    ];
    if needs != "shaper" {
        tags.push(tag(["p", needs, "", "needs"]));
    }
    signed(keys, KIND_IO_DRAFT, tags, content)
}

fn draft_tag(id: &str) -> Tag {
    tag(["e", id, "", "draft"])
}

async fn shapers_event_hex(h: &Harness) -> String {
    let mut conn = h.pool.acquire().await.expect("acquire");
    hex::encode(
        store::get_shapers(&mut conn, h.community())
            .await
            .expect("read io_shapers")
            .expect("bootstrapped")
            .event_id,
    )
}

async fn draft_status(h: &Harness, id: &EventId) -> DraftOutcomeStatus {
    let mut conn = h.pool.acquire().await.expect("acquire");
    store::get_draft(&mut conn, h.community(), id.as_bytes())
        .await
        .expect("read io_drafts")
        .expect("draft row")
        .outcome
        .status
}

async fn live_outcome(h: &Harness, draft: &str) -> serde_json::Value {
    let events = h.live_state(KIND_IO_DRAFT_OUTCOME).await;
    events
        .into_iter()
        .find(|(id, content, _)| content["draft"] == draft || *id == draft)
        .map(|(_, content, _)| content)
        .unwrap_or_else(|| panic!("no 39104 for {draft}"))
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn unresolved_receipt_is_rejected() {
    let h = harness().await;
    h.bootstrap().await;
    let fake = "ab".repeat(32);
    let event = draft_event(
        &h.agent,
        &h.owner.public_key().to_hex(),
        "gap-missing",
        &fake,
        PROJECT,
    );
    assert_eq!(
        rejected(h.ingest(&h.agent, event).await),
        "invalid: unresolved receipt"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn second_open_draft_per_gap_is_rejected() {
    let h = harness().await;
    h.bootstrap().await;
    let receipt = shapers_event_hex(&h).await;
    let needs = h.owner.public_key().to_hex();
    let first = draft_event(&h.agent, &needs, "gap-once", &receipt, PROJECT);
    h.ingest(&h.agent, first).await.expect("first draft");
    let second = draft_event(&h.agent, &needs, "gap-once", &receipt, PROJECT);
    assert_eq!(
        rejected(h.ingest(&h.agent, second).await),
        "invalid: an open draft already exists for this gap"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn agent_note_from_a_non_agent_is_rejected() {
    let h = harness().await;
    h.bootstrap().await;
    let event = signed(
        &h.owner,
        KIND_IO_AGENT_NOTE,
        vec![
            tag(["t", "draft_dropped"]),
            tag(["move", "1"]),
            tag(["gap", "x"]),
        ],
        r#"{"note":"draft_dropped","move":1,"gap":"x","reason":"nag","kind":"ticket","needs":"shaper","trace":null}"#,
    );
    assert_eq!(
        rejected(h.ingest(&h.owner, event).await),
        "restricted: not the org agent"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn health_read_from_a_non_agent_is_rejected() {
    let h = harness().await;
    h.bootstrap().await;
    let (_, item) = h
        .pass_project(&h.owner, r#"{"title":"H","brief":"b","due_at":1800000000}"#)
        .await;
    let event = signed(
        &h.owner,
        KIND_IO_HEALTH,
        vec![
            tag(["i", &item.id]),
            tag(["week", "2026-W38"]),
            tag(["band", "healthy"]),
        ],
        &format!(
            r#"{{"item":"{}","week":"2026-W38","pct":0.9,"band":"healthy","factors":[],"sentences":[],"formula":"health-weights@1"}}"#,
            item.id
        ),
    );
    assert_eq!(
        rejected(h.ingest(&h.owner, event).await),
        "restricted: not the org agent"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn draft_tag_from_the_wrong_needs_party_leaves_the_draft_untouched() {
    let h = harness().await;
    h.bootstrap().await;
    let stranger = Keys::generate();
    h.member(&stranger).await;
    let receipt = shapers_event_hex(&h).await;
    let needs = h.owner.public_key().to_hex();
    let draft = draft_event(&h.agent, &needs, "gap-needs", &receipt, PROJECT);
    h.ingest(&h.agent, draft.clone())
        .await
        .expect("draft stored");
    let command = signed(
        &stranger,
        KIND_IO_PROJECT_PROPOSE,
        vec![tag(["vote", "agree"]), draft_tag(&draft.id.to_hex())],
        PROJECT,
    );
    assert_eq!(
        rejected(h.ingest(&stranger, command.clone()).await),
        "restricted: not the draft's needs party"
    );
    assert_eq!(draft_status(&h, &draft.id).await, DraftOutcomeStatus::Open);
    assert!(
        !h.event_stored(&command.id).await,
        "the refused command is not stored"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn equal_payload_accepts_and_a_different_payload_amends() {
    let h = harness().await;
    h.bootstrap().await;
    let receipt = shapers_event_hex(&h).await;
    let needs = h.owner.public_key().to_hex();

    let equal = draft_event(&h.agent, &needs, "gap-eq", &receipt, PROJECT);
    h.ingest(&h.agent, equal.clone())
        .await
        .expect("equal draft");
    h.send(
        &h.owner,
        KIND_IO_PROJECT_PROPOSE,
        vec![tag(["vote", "agree"]), draft_tag(&equal.id.to_hex())],
        PROJECT,
    )
    .await
    .expect("agree as written");
    assert_eq!(
        draft_status(&h, &equal.id).await,
        DraftOutcomeStatus::Accepted
    );
    assert_eq!(
        live_outcome(&h, &equal.id.to_hex()).await["status"],
        "accepted"
    );

    let different = draft_event(&h.agent, &needs, "gap-diff", &receipt, PROJECT);
    h.ingest(&h.agent, different.clone())
        .await
        .expect("different draft");
    h.send(
        &h.owner,
        KIND_IO_PROJECT_PROPOSE,
        vec![tag(["vote", "agree"]), draft_tag(&different.id.to_hex())],
        PROJECT_EDITED,
    )
    .await
    .expect("agree with edits");
    assert_eq!(
        draft_status(&h, &different.id).await,
        DraftOutcomeStatus::Amended
    );
    assert_eq!(
        live_outcome(&h, &different.id.to_hex()).await["status"],
        "amended"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn draft_decide_and_health_rate_follow_the_protocol() {
    let h = harness().await;
    h.bootstrap().await;
    let receipt = shapers_event_hex(&h).await;
    let needs = h.owner.public_key().to_hex();
    let member = Keys::generate();
    h.member(&member).await;

    let decline = draft_event(&h.agent, &needs, "gap-dec", &receipt, PROJECT);
    h.ingest(&h.agent, decline.clone())
        .await
        .expect("decline target");
    assert_eq!(
        rejected(
            h.send(
                &h.owner,
                KIND_IO_DRAFT_DECIDE,
                vec![
                    tag(["e", &decline.id.to_hex()]),
                    tag(["outcome", "decline"])
                ],
                "{}",
            )
            .await
        ),
        "invalid: decline needs a reason"
    );
    assert_eq!(
        draft_status(&h, &decline.id).await,
        DraftOutcomeStatus::Open
    );
    assert_eq!(
        rejected(
            h.send(
                &member,
                KIND_IO_DRAFT_DECIDE,
                vec![
                    tag(["e", &decline.id.to_hex()]),
                    tag(["outcome", "decline"]),
                    tag(["reason", "not_now"]),
                ],
                "{}",
            )
            .await
        ),
        "restricted: not the draft's needs party"
    );
    h.send(
        &h.owner,
        KIND_IO_DRAFT_DECIDE,
        vec![
            tag(["e", &decline.id.to_hex()]),
            tag(["outcome", "decline"]),
            tag(["reason", "not_now"]),
        ],
        "{}",
    )
    .await
    .expect("decline");
    assert_eq!(
        draft_status(&h, &decline.id).await,
        DraftOutcomeStatus::Declined
    );

    let accept = draft_event(&h.agent, &needs, "gap-acc", &receipt, PROJECT);
    h.ingest(&h.agent, accept.clone())
        .await
        .expect("accept target");
    h.send(
        &h.owner,
        KIND_IO_DRAFT_DECIDE,
        vec![tag(["e", &accept.id.to_hex()]), tag(["outcome", "accept"])],
        "{}",
    )
    .await
    .expect("standalone accept");
    assert_eq!(
        draft_status(&h, &accept.id).await,
        DraftOutcomeStatus::Accepted
    );

    let (_, item) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Rate me","brief":"b","due_at":1800000000}"#,
        )
        .await;
    assert_eq!(
        rejected(
            h.send(
                &member,
                KIND_IO_HEALTH_RATE,
                vec![
                    tag(["i", &item.id]),
                    tag(["week", "2026-W38"]),
                    tag(["band", "wobbly"]),
                ],
                "{}",
            )
            .await
        ),
        "restricted: not a Shaper"
    );
    h.send(
        &h.owner,
        KIND_IO_HEALTH_RATE,
        vec![
            tag(["i", &item.id]),
            tag(["week", "2026-W38"]),
            tag(["band", "wobbly"]),
        ],
        "{}",
    )
    .await
    .expect("shaper rates");
    let verbs = h.ledger_verbs().await;
    assert!(verbs.contains(&"draft_decided".to_owned()));
    assert!(verbs.contains(&"health_rated".to_owned()));

    let note = signed(
        &h.agent,
        KIND_IO_AGENT_NOTE,
        vec![tag(["t", "tally"]), tag(["week", "2026-W38"])],
        r#"{"note":"tally","week":"2026-W38","window_weeks":4,"moves":{},"health":{"reads":0,"rated":0,"agreed":0},"open_older_than_5d":0,"receipt_rejected":0}"#,
    );
    h.ingest(&h.agent, note).await.expect("agent tally");
    let verbs = h.ledger_verbs().await;
    assert!(verbs.contains(&"tally".to_owned()));
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn holder_receipt_skill_and_open_limit_and_profile_needs() {
    let h = harness().await;
    h.bootstrap().await;
    let holder = Keys::generate();
    h.member(&holder).await;
    let holder_hex = holder.public_key().to_hex();
    let receipt = shapers_event_hex(&h).await;

    let (_, item) = h
        .pass_project(
            &h.owner,
            &format!(
                r#"{{"title":"Held","brief":"b","due_at":1800000000,"suggested_dri":"{holder_hex}"}}"#
            ),
        )
        .await;
    h.accept_item(&holder, &item.id)
        .await
        .expect("holder accepts");
    let item_event = {
        let mut conn = h.pool.acquire().await.expect("acquire");
        hex::encode(
            store::get_work_item(
                &mut conn,
                h.community(),
                Uuid::parse_str(&item.id).expect("uuid"),
            )
            .await
            .expect("read item")
            .expect("item")
            .event_id,
        )
    };

    let profile = OrgProfile {
        pubkey: holder_hex.clone(),
        version: 1,
        about: "grants".into(),
        skills: vec![Skill {
            slug: "grant-writing".into(),
            label: "grant writing".into(),
        }],
        open_limit: None,
        updated_at: 1,
        receipt: receipt.clone(),
    };
    let mut conn = h.pool.acquire().await.expect("acquire");
    store::upsert_profile(
        &mut conn,
        h.community(),
        &ProfileRow {
            content: profile.clone(),
            active: true,
            event_id: vec![0x11u8; 32],
            updated_at: chrono::Utc::now(),
        },
    )
    .await
    .expect("seed profile");
    sqlx::query(
        "INSERT INTO events (community_id, id, pubkey, created_at, kind, tags, content, sig, d_tag) \
         VALUES ($1, $2, $3, now(), $4, '[]'::jsonb, $5, $6, $7)",
    )
    .bind(h.community().as_uuid())
    .bind(vec![0x11u8; 32])
    .bind(h.state.relay_keypair.public_key().to_bytes().to_vec())
    .bind(KIND_IO_PROFILE as i32)
    .bind(serde_json::to_string(&profile).expect("profile json"))
    .bind(vec![0u8; 64])
    .bind(&holder_hex)
    .execute(&mut *conn)
    .await
    .expect("seed 39105");
    drop(conn);

    let ok = signed(
        &h.agent,
        KIND_IO_DRAFT,
        vec![
            tag(["n", "shaper"]),
            tag(["t", "dri"]),
            tag(["move", "1"]),
            tag(["origin", "gap"]),
            tag(["gap", &format!("{}#dri", item.id)]),
            tag(["i", &item.id]),
            tag(["p", &holder_hex, "", "suggested"]),
            tag(["e", &item_event, "", "receipt"]),
            tag(["k", "grant-writing"]),
        ],
        &format!(
            r#"{{"item":"{}","suggested":"{holder_hex}","why":"fit","matched":{{"skills":["grant-writing"],"about":null,"items":[]}},"evidence":[]}}"#,
            item.id
        ),
    );
    h.ingest(&h.agent, ok)
        .await
        .expect("holder evidence via 39101");

    let bad_skill = signed(
        &h.agent,
        KIND_IO_DRAFT,
        vec![
            tag(["n", "shaper"]),
            tag(["t", "dri"]),
            tag(["move", "1"]),
            tag(["origin", "gap"]),
            tag(["gap", &format!("{}#skill", item.id)]),
            tag(["i", &item.id]),
            tag(["p", &holder_hex, "", "suggested"]),
            tag(["e", &item_event, "", "receipt"]),
            tag(["k", "rust"]),
        ],
        &format!(
            r#"{{"item":"{}","suggested":"{holder_hex}","why":"fit","matched":{{"skills":["rust"],"about":null,"items":[]}},"evidence":[]}}"#,
            item.id
        ),
    );
    assert_eq!(
        rejected(h.ingest(&h.agent, bad_skill).await),
        "invalid: skill not on the holder's profile"
    );

    {
        let mut limited = profile.clone();
        limited.open_limit = Some(1);
        let mut conn = h.pool.acquire().await.expect("acquire");
        store::upsert_profile(
            &mut conn,
            h.community(),
            &ProfileRow {
                content: limited,
                active: true,
                event_id: vec![0x11u8; 32],
                updated_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("set open_limit");
    }

    let at_limit = signed(
        &h.agent,
        KIND_IO_DRAFT,
        vec![
            tag(["n", "shaper"]),
            tag(["t", "project"]),
            tag(["move", "1"]),
            tag(["origin", "gap"]),
            tag(["gap", "gap-limit"]),
            tag(["p", &holder_hex, "", "suggested"]),
            tag(["e", &item_event, "", "receipt"]),
        ],
        &format!(
            r#"{{"title":"Next","brief":"b","objective_ref":null,"due_at":1800000001,"suggested_dri":"{holder_hex}","why":"more","gaps":[]}}"#
        ),
    );
    assert_eq!(
        rejected(h.ingest(&h.agent, at_limit).await),
        "invalid: holder at limit"
    );

    let profile_draft = signed(
        &h.agent,
        KIND_IO_DRAFT,
        vec![
            tag(["n", &needs_other(&h.owner)]),
            tag(["t", "profile"]),
            tag(["move", "1"]),
            tag(["origin", "talk"]),
            tag(["gap", "profile-wrong"]),
            tag(["p", &needs_other(&h.owner), "", "needs"]),
            tag(["e", &receipt, "", "receipt"]),
        ],
        &format!(
            r#"{{"pubkey":"{holder_hex}","about":"x","skills":[],"open_limit":null,"heard":[]}}"#
        ),
    );
    assert_eq!(
        rejected(h.ingest(&h.agent, profile_draft).await),
        "invalid: profile draft not addressed to its subject"
    );
}

fn needs_other(keys: &Keys) -> String {
    keys.public_key().to_hex()
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn health_read_stores_when_the_item_and_rows_resolve() {
    let h = harness().await;
    h.bootstrap().await;
    let receipt = shapers_event_hex(&h).await;
    let (_, item) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Health","brief":"b","due_at":1800000000}"#,
        )
        .await;
    let event = signed(
        &h.agent,
        KIND_IO_HEALTH,
        vec![
            tag(["i", &item.id]),
            tag(["week", "2026-W38"]),
            tag(["band", "wobbly"]),
        ],
        &format!(
            r#"{{"item":"{}","week":"2026-W38","pct":0.5,"band":"wobbly","factors":[{{"name":"overdue","value":1,"weight":0.25,"rows":["{receipt}"]}}],"sentences":[{{"text":"One piece is late.","rows":["{receipt}"]}}],"formula":"health-weights@1"}}"#,
            item.id
        ),
    );
    h.ingest(&h.agent, event).await.expect("health read");
    assert!(h.ledger_verbs().await.contains(&"health_read".to_owned()));
}
