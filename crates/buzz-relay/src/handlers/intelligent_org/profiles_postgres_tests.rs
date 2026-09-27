//! R-11 proofs at the production seam: every `50021` enters through
//! [`crate::handlers::ingest::ingest_event`], and NIP-43 removal flips
//! `io_profiles.active` without deleting the `39105`.

use buzz_core::kind::{KIND_IO_DRAFT, KIND_IO_PROFILE, KIND_IO_PROFILE_SET};
use buzz_db::intelligent_org as store;
use buzz_db::relay_members;
use nostr::{Event, Keys, Tag};

use super::postgres_tests::{harness, rejected, signed, tag, Harness};

fn profile_set(keys: &Keys, content: &str, extra: Vec<Tag>) -> Event {
    signed(keys, KIND_IO_PROFILE_SET, extra, content)
}

fn profile_draft(keys: &Keys, needs: &str, subject: &str, gap: &str, receipt: &str) -> Event {
    let mut tags = vec![
        tag(["n", needs]),
        tag(["t", "profile"]),
        tag(["move", "1"]),
        tag(["origin", "gap"]),
        tag(["gap", gap]),
        tag(["e", receipt, "", "receipt"]),
    ];
    if needs != "shaper" {
        tags.push(tag(["p", needs, "", "needs"]));
    }
    signed(
        keys,
        KIND_IO_DRAFT,
        tags,
        &format!(
            r#"{{"pubkey":"{subject}","about":"hi","skills":["rust"],"open_limit":2,"heard":[]}}"#
        ),
    )
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

async fn profile_row(h: &Harness, pubkey: &[u8]) -> Option<(bool, u32, Vec<String>)> {
    let mut conn = h.pool.acquire().await.expect("acquire");
    let row = store::get_profile(&mut conn, h.community(), pubkey)
        .await
        .expect("read io_profiles")?;
    Some((
        row.active,
        row.content.version,
        row.content.skills.iter().map(|s| s.slug.clone()).collect(),
    ))
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn profile_set_writes_39105_and_enforces_limits_and_self() {
    let h = harness().await;
    h.bootstrap().await;
    let me = h.owner.public_key().to_hex();
    let other = Keys::generate();
    let other_hex = other.public_key().to_hex();

    // Happy path: about + skills → 39105 with kebab `k` tags and version 1.
    h.ingest(
        &h.owner,
        profile_set(
            &h.owner,
            r#"{"about":"I write grants.","skills":["grant writing","Rust"],"open_limit":3}"#,
            vec![],
        ),
    )
    .await
    .expect("set profile");
    let live = h.live_state(KIND_IO_PROFILE).await;
    let mine = live
        .iter()
        .find(|(_, c, _)| c["pubkey"] == me)
        .expect("39105 for owner");
    assert_eq!(mine.1["version"], 1);
    assert_eq!(mine.1["about"], "I write grants.");
    assert_eq!(mine.1["open_limit"], 3);
    assert_eq!(
        mine.1["skills"],
        serde_json::json!([
            {"slug":"grant-writing","label":"grant writing"},
            {"slug":"rust","label":"Rust"}
        ])
    );
    // Wire tags: one `k` per skill (asserted via SQL — the harness's
    // `live_state` only surfaces `p`).
    let tags_json: serde_json::Value = sqlx::query_scalar(
        "SELECT tags FROM events \
         WHERE community_id = $1 AND kind = $2 AND d_tag = $3 AND deleted_at IS NULL \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(h.community().as_uuid())
    .bind(KIND_IO_PROFILE as i32)
    .bind(&me)
    .fetch_one(&h.pool)
    .await
    .expect("39105 tags");
    let mut k_tags: Vec<String> = tags_json
        .as_array()
        .expect("tags array")
        .iter()
        .filter_map(|t| {
            let arr = t.as_array()?;
            if arr.first()?.as_str()? != "k" {
                return None;
            }
            arr.get(1)?.as_str().map(str::to_owned)
        })
        .collect();
    k_tags.sort();
    assert_eq!(k_tags, vec!["grant-writing".to_owned(), "rust".to_owned()]);
    assert_eq!(
        profile_row(&h, &h.owner.public_key().to_bytes()).await,
        Some((true, 1, vec!["grant-writing".into(), "rust".into()]))
    );

    // Who can do X — `#k` pushdown returns the matching profile.
    let mut q = buzz_db::EventQuery::for_community(h.community());
    q.kinds = Some(vec![KIND_IO_PROFILE as i32]);
    q.custom_tags = vec![("k".into(), vec!["grant-writing".into()])];
    let by_skill = h.state.db.query_events(&q).await.expect("query by skill");
    assert_eq!(by_skill.len(), 1);
    assert!(by_skill[0].event.tags.iter().any(|t| {
        let p = t.as_slice();
        p.first().map(String::as_str) == Some("d")
            && p.get(1).map(String::as_str) == Some(me.as_str())
    }));

    // Replace whole: version bumps; empty profile is allowed.
    h.ingest(
        &h.owner,
        profile_set(&h.owner, r#"{"about":"","skills":[]}"#, vec![]),
    )
    .await
    .expect("clear profile");
    assert_eq!(
        profile_row(&h, &h.owner.public_key().to_bytes()).await,
        Some((true, 2, vec![]))
    );

    // Limits.
    let long_about = format!(r#"{{"about":"{}","skills":[]}}"#, "x".repeat(1001));
    assert_eq!(
        rejected(
            h.ingest(&h.owner, profile_set(&h.owner, &long_about, vec![]))
                .await
        ),
        "invalid: about over 1000 characters"
    );
    let too_many: Vec<String> = (0..21).map(|i| format!("skill {i}")).collect();
    let many = serde_json::json!({ "about": "", "skills": too_many }).to_string();
    assert_eq!(
        rejected(
            h.ingest(&h.owner, profile_set(&h.owner, &many, vec![]))
                .await
        ),
        "invalid: at most 20 skills"
    );
    let long_label = format!(r#"{{"about":"","skills":["{}"]}}"#, "y".repeat(41));
    assert_eq!(
        rejected(
            h.ingest(&h.owner, profile_set(&h.owner, &long_label, vec![]))
                .await
        ),
        "invalid: skill label over 40 characters"
    );
    assert_eq!(
        rejected(
            h.ingest(
                &h.owner,
                profile_set(
                    &h.owner,
                    r#"{"about":"","skills":[],"open_limit":0}"#,
                    vec![]
                ),
            )
            .await
        ),
        "invalid: open_limit must be 1–50 or null"
    );

    // Self-only: content pubkey / p tag for someone else.
    assert_eq!(
        rejected(
            h.ingest(
                &h.owner,
                profile_set(
                    &h.owner,
                    &format!(r#"{{"pubkey":"{other_hex}","about":"no","skills":[]}}"#),
                    vec![],
                ),
            )
            .await
        ),
        "invalid: cannot set another member's profile"
    );
    assert_eq!(
        rejected(
            h.ingest(
                &h.owner,
                profile_set(
                    &h.owner,
                    r#"{"about":"no","skills":[]}"#,
                    vec![tag(["p", &other_hex])],
                ),
            )
            .await
        ),
        "invalid: cannot set another member's profile"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn profile_draft_must_address_its_subject_and_removal_inactivates() {
    let h = harness().await;
    h.bootstrap().await;
    let me = h.owner.public_key().to_hex();
    let other = Keys::generate();
    let other_hex = other.public_key().to_hex();
    h.state
        .db
        .add_relay_member(h.community(), &other_hex, "member", Some(&me))
        .await
        .expect("seed other");

    let receipt = shapers_event_hex(&h).await;
    // needs ≠ subject → refused at ingest (§5.4a).
    assert_eq!(
        rejected(
            h.ingest(
                &h.agent,
                profile_draft(&h.agent, &other_hex, &me, "gap-profile-wrong", &receipt),
            )
            .await
        ),
        "invalid: profile draft not addressed to its subject"
    );
    // Matched needs = subject is accepted.
    h.ingest(
        &h.agent,
        profile_draft(&h.agent, &me, &me, "gap-profile-ok", &receipt),
    )
    .await
    .expect("profile draft");

    h.ingest(
        &h.owner,
        profile_set(
            &h.owner,
            r#"{"about":"active","skills":["hosting events"]}"#,
            vec![],
        ),
    )
    .await
    .expect("set before leave");
    assert_eq!(
        profile_row(&h, &h.owner.public_key().to_bytes())
            .await
            .map(|r| r.0),
        Some(true)
    );

    // NIP-43 removal leaves the 39105 and marks the projection inactive.
    // Owner cannot leave; remove a seeded member who has a profile instead.
    h.ingest(
        &other,
        profile_set(&other, r#"{"about":"guest","skills":["design"]}"#, vec![]),
    )
    .await
    .expect("other sets profile");
    assert_eq!(
        profile_row(&h, &other.public_key().to_bytes()).await,
        Some((true, 1, vec!["design".into()]))
    );
    assert_eq!(
        relay_members::remove_relay_member(&h.pool, h.community(), &other_hex)
            .await
            .expect("remove"),
        relay_members::RemoveResult::Removed
    );
    assert_eq!(
        profile_row(&h, &other.public_key().to_bytes()).await,
        Some((false, 1, vec!["design".into()]))
    );
    // Live 39105 is still there (wire event untouched).
    let still = h.live_state(KIND_IO_PROFILE).await;
    assert!(still.iter().any(|(_, c, _)| c["pubkey"] == other_hex));

    // Rejoin picks the profile back up.
    h.state
        .db
        .add_relay_member(h.community(), &other_hex, "member", Some(&me))
        .await
        .expect("re-add");
    assert_eq!(
        profile_row(&h, &other.public_key().to_bytes())
            .await
            .map(|r| r.0),
        Some(true)
    );
}
