//! End-to-end proofs for the intelligent organization (C-3), grown one relay
//! slice at a time. This file holds the R-3 lines — the executor spine and
//! the Shapers — and the R-12 lines — invites minted by Shapers, the
//! `member_joined` ledger row, the transparency notice — driven through the
//! relay's real HTTP door (`POST /events`, `POST /query`, `POST /api/invites`,
//! `GET /api/join-policy`, NIP-98) exactly as a client or the `buzz` CLI would.
//!
//! Every test gets its own community (a fresh `Host`) on the running relay,
//! because a `39103` bootstrap happens once per community and the relay
//! process is shared with the other e2e suites. The only rows a test seeds
//! directly are the ones an operator would: the community, its relay
//! members, and the hosted org-agent key in `io_hosted_agents`. One test also
//! seeds an *offered* seat, which a passed `shapers/add` will write in R-4a.
//!
//! # Running
//!
//! ```text
//! just relay   # in another shell
//! cargo test -p buzz-test-client --test e2e_intelligent_org -- --ignored
//! ```
//!
//! `RELAY_URL` (default `ws://localhost:3000`) and `DATABASE_URL` (default
//! the dev Postgres) point at the relay under test. `*.localhost` hosts are
//! sent in the `Host` header, so no DNS is needed.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use buzz_core::kind::{
    KIND_IO_PROPOSAL, KIND_IO_SHAPERS, KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT,
    KIND_IO_SHAPER_STEP_DOWN,
};
use nostr::{Event, EventBuilder, Keys, Kind, Tag, Timestamp};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

fn relay_http_url() -> String {
    std::env::var("RELAY_URL")
        .unwrap_or_else(|_| "ws://localhost:3000".to_string())
        .replace("wss://", "https://")
        .replace("ws://", "http://")
        .trim_end_matches('/')
        .to_string()
}

fn http_scheme() -> &'static str {
    if relay_http_url().starts_with("https://") {
        "https"
    } else {
        "http"
    }
}

async fn db_pool() -> sqlx::PgPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://buzz:buzz_dev@localhost:5432/buzz".to_string() // sadscan:disable np.postgres.1
    });
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to e2e Postgres")
}

fn nip98_header(keys: &Keys, url: &str, body: &str) -> String {
    let event = EventBuilder::new(Kind::Custom(27_235), "")
        .tags(vec![
            Tag::parse(["u", url]).expect("u tag"),
            Tag::parse(["method", "POST"]).expect("method tag"),
            Tag::parse(["payload", &hex::encode(Sha256::digest(body.as_bytes()))])
                .expect("payload tag"),
            Tag::parse(["nonce", &Uuid::new_v4().to_string()]).expect("nonce tag"),
        ])
        .sign_with_keys(keys)
        .expect("sign NIP-98 event");
    format!(
        "Nostr {}",
        BASE64.encode(serde_json::to_string(&event).expect("serialize NIP-98 event"))
    )
}

fn tag<const N: usize>(parts: [&str; N]) -> Tag {
    Tag::parse(parts).expect("tag")
}

/// Sign as a client would. `allow_self_tagging` matters: the bootstrap names
/// the sender in its own `p` tag, which nostr's builder drops by default.
fn signed(keys: &Keys, kind: u32, tags: Vec<Tag>, content: &str) -> Event {
    EventBuilder::new(Kind::Custom(kind as u16), content)
        .tags(tags)
        .allow_self_tagging()
        .custom_created_at(Timestamp::now())
        .sign_with_keys(keys)
        .expect("sign")
}

fn p_tags(event: &Value) -> Vec<String> {
    event["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .filter(|t| t[0] == "p")
        .map(|t| t[1].as_str().expect("p value").to_owned())
        .collect()
}

fn content(event: &Value) -> Value {
    serde_json::from_str(event["content"].as_str().expect("content string")).expect("content json")
}

/// One fresh community on the shared relay, addressed by `Host`.
struct Community {
    host: String,
    id: Uuid,
    pool: sqlx::PgPool,
    http: reqwest::Client,
    owner: Keys,
    agent: Keys,
}

impl Community {
    /// A community with a provisioned hosted org agent — what an operator
    /// leaves behind before the owner bootstraps.
    async fn fresh() -> Self {
        Self::fresh_with(true).await
    }

    /// A community no operator has touched: no `io_hosted_agents` row and no
    /// `39103`. Not an intelligent organization, so the invite page owes it
    /// no notice.
    async fn fresh_plain() -> Self {
        Self::fresh_with(false).await
    }

    async fn fresh_with(hosted_agent: bool) -> Self {
        let pool = db_pool().await;
        let host = format!("io-r3-{}.localhost", Uuid::new_v4().simple());
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO communities (id, host) VALUES ($1, $2)")
            .bind(id)
            .bind(&host)
            .execute(&pool)
            .await
            .expect("seed community");
        let community = Self {
            host,
            id,
            pool,
            http: reqwest::Client::new(),
            owner: Keys::generate(),
            agent: Keys::generate(),
        };
        community
            .seed_member(&community.owner.clone(), "owner")
            .await;
        if hosted_agent {
            sqlx::query(
                "INSERT INTO io_hosted_agents (community_id, pubkey, provisioned_at) \
                 VALUES ($1, $2, now())",
            )
            .bind(id)
            .bind(community.agent.public_key().to_bytes().to_vec())
            .execute(&community.pool)
            .await
            .expect("seed hosted agent");
        }
        community
    }

    /// The relay-level role (`relay_members.role`) of `keys`, if a member.
    /// This is what `POST /api/invites` checked before R-12 — distinct from
    /// the room role the executor gives Shapers in `#shapers`.
    async fn relay_role(&self, keys: &Keys) -> Option<String> {
        sqlx::query_scalar("SELECT role FROM relay_members WHERE community_id = $1 AND pubkey = $2")
            .bind(self.id)
            .bind(keys.public_key().to_hex())
            .fetch_optional(&self.pool)
            .await
            .expect("read relay role")
    }

    /// Unauthenticated `GET` on this community's host, as a browser landing
    /// on `/invite/<code>` performs it.
    async fn get(&self, path: &str) -> (StatusCode, Value) {
        let response = self
            .http
            .get(format!("{}{path}", relay_http_url()))
            .header(reqwest::header::HOST, &self.host)
            .send()
            .await
            .unwrap_or_else(|e| panic!("GET {path} failed: {e}"));
        let status = response.status();
        let body: Value = response.json().await.expect("json body");
        (status, body)
    }

    /// `POST /api/invites` as `keys`, with an empty request (relay defaults).
    async fn mint_invite(&self, keys: &Keys) -> (StatusCode, Value) {
        self.post(keys, "/api/invites", "{}".to_owned()).await
    }

    /// `POST /api/invites/claim` as `keys`.
    async fn claim_invite(&self, keys: &Keys, code: &str) -> (StatusCode, Value) {
        self.post(
            keys,
            "/api/invites/claim",
            json!({ "code": code }).to_string(),
        )
        .await
    }

    /// Every `member_joined` ledger row as `(actor, detail)`, oldest first.
    async fn member_joined_rows(&self) -> Vec<(String, Value)> {
        sqlx::query(
            "SELECT actor, object_type, object_id, receipt_event_id, detail FROM io_ledger \
             WHERE community_id = $1 AND verb = 'member_joined' ORDER BY id",
        )
        .bind(self.id)
        .fetch_all(&self.pool)
        .await
        .expect("read member_joined rows")
        .into_iter()
        .map(|r| {
            let actor: String = r.get("actor");
            assert_eq!(r.get::<String, _>("object_type"), "member");
            assert_eq!(r.get::<String, _>("object_id"), actor);
            assert_eq!(
                r.get::<Option<Vec<u8>>, _>("receipt_event_id"),
                None,
                "an HTTP claim has no event to cite"
            );
            (actor, r.get("detail"))
        })
        .collect()
    }

    /// Give `keys` a relay membership so the door admits them whatever
    /// `REQUIRE_RELAY_MEMBERSHIP` says; only `role = owner` may bootstrap.
    async fn seed_member(&self, keys: &Keys, role: &str) {
        sqlx::query(
            "INSERT INTO relay_members (community_id, pubkey, role, added_by) \
             VALUES ($1, $2, $3, NULL) \
             ON CONFLICT (community_id, pubkey) DO UPDATE SET role = $3, updated_at = now()",
        )
        .bind(self.id)
        .bind(keys.public_key().to_hex())
        .bind(role)
        .execute(&self.pool)
        .await
        .expect("seed relay member");
    }

    async fn post(&self, keys: &Keys, path: &str, body: String) -> (StatusCode, Value) {
        let signed_url = format!("{}://{}{path}", http_scheme(), self.host);
        let response = self
            .http
            .post(format!("{}{path}", relay_http_url()))
            .header(reqwest::header::HOST, &self.host)
            .header("Authorization", nip98_header(keys, &signed_url, &body))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("POST {path} failed: {e}"));
        let status = response.status();
        let body: Value = response.json().await.expect("json body");
        (status, body)
    }

    /// `POST /events`; returns the status and body verbatim.
    async fn submit(&self, keys: &Keys, event: &Event) -> (StatusCode, Value) {
        self.post(
            keys,
            "/events",
            serde_json::to_string(event).expect("serialize event"),
        )
        .await
    }

    /// `POST /events` that must be accepted; returns the `message`.
    async fn submit_ok(&self, keys: &Keys, event: &Event) -> String {
        let (status, body) = self.submit(keys, event).await;
        assert_eq!(status, StatusCode::OK, "expected acceptance, got {body}");
        assert_eq!(body["accepted"], true, "{body}");
        body["message"].as_str().expect("message").to_owned()
    }

    /// `POST /events` that must be refused `400` with exactly `reason`.
    async fn submit_rejected(&self, keys: &Keys, event: &Event, reason: &str) {
        let (status, body) = self.submit(keys, event).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "expected rejection, got {body}"
        );
        assert_eq!(body["error"], reason, "{body}");
    }

    /// `POST /query` with one filter, as `keys`.
    async fn query(&self, keys: &Keys, filter: Value) -> Vec<Value> {
        let (status, body) = self.post(keys, "/query", json!([filter]).to_string()).await;
        assert_eq!(status, StatusCode::OK, "query failed: {body}");
        body.as_array().cloned().expect("query returns an array")
    }

    /// The live `39103`, if any, as the owner sees it.
    async fn shapers_state(&self) -> Option<Value> {
        let mut events = self
            .query(
                &self.owner,
                json!({ "kinds": [KIND_IO_SHAPERS], "#d": ["shapers"] }),
            )
            .await;
        assert!(
            events.len() <= 1,
            "NIP-33 keeps one 39103, got {}",
            events.len()
        );
        events.pop()
    }

    async fn bootstrap(&self) -> (String, Uuid) {
        let owner_hex = self.owner.public_key().to_hex();
        let message = self
            .submit_ok(
                &self.owner,
                &signed(
                    &self.owner,
                    KIND_IO_SHAPERS_PROPOSE,
                    vec![tag(["op", "add"]), tag(["p", &owner_hex])],
                    r#"{"why":"first Shaper"}"#,
                ),
            )
            .await;
        let reply: Value = serde_json::from_str(&message).expect("bootstrap reply json");
        (
            reply["proposal"].as_str().expect("proposal id").to_owned(),
            Uuid::parse_str(reply["room"].as_str().expect("room id")).expect("room uuid"),
        )
    }

    /// Active `#shapers` roster as `(pubkey hex, role)`, sorted — the rows the
    /// relay serves membership from.
    async fn roster(&self, room: Uuid) -> Vec<(String, String)> {
        let rows = sqlx::query(
            "SELECT pubkey, role::text AS role FROM channel_members \
             WHERE community_id = $1 AND channel_id = $2 AND removed_at IS NULL",
        )
        .bind(self.id)
        .bind(room)
        .fetch_all(&self.pool)
        .await
        .expect("read roster");
        let mut roster: Vec<(String, String)> = rows
            .into_iter()
            .map(|r| (hex::encode(r.get::<Vec<u8>, _>("pubkey")), r.get("role")))
            .collect();
        roster.sort();
        roster
    }

    /// `shapers ∪ {agent}` as the relay must keep it: Shapers are admins,
    /// the agent a member (Design § Shapers).
    fn expected_roster(state: &Value) -> Vec<(String, String)> {
        let body = content(state);
        let shapers: Vec<String> = body["shapers"]
            .as_array()
            .expect("shapers")
            .iter()
            .map(|p| p.as_str().expect("pubkey").to_owned())
            .collect();
        let mut expected: Vec<(String, String)> = shapers
            .iter()
            .map(|p| (p.clone(), "admin".to_owned()))
            .collect();
        if let Some(agent) = body["agent"].as_str() {
            if !shapers.iter().any(|p| p == agent) {
                expected.push((agent.to_owned(), "member".to_owned()));
            }
        }
        expected.sort();
        expected
    }

    async fn ledger_verbs(&self) -> Vec<String> {
        sqlx::query_scalar("SELECT verb FROM io_ledger WHERE community_id = $1 ORDER BY id")
            .bind(self.id)
            .fetch_all(&self.pool)
            .await
            .expect("ledger verbs")
    }

    /// Seed the offered seat a passed `shapers/add` writes (R-4a), so
    /// `io_shaper_accept` has a seat to take.
    async fn offer_seat(&self, p: &Keys, proposal: &str) {
        let seat = json!([{
            "p": p.public_key().to_hex(),
            "proposal": proposal,
            "at": Timestamp::now().as_secs(),
        }]);
        let updated = sqlx::query(
            "UPDATE io_shapers SET content = jsonb_set(content, '{offered}', $2) \
             WHERE community_id = $1",
        )
        .bind(self.id)
        .bind(seat)
        .execute(&self.pool)
        .await
        .expect("seed offered seat")
        .rows_affected();
        assert_eq!(updated, 1, "io_shapers row exists after bootstrap");
    }

    async fn set_room_deleted(&self, room: Uuid, deleted: bool) {
        let sql = if deleted {
            "UPDATE channels SET deleted_at = now() WHERE community_id = $1 AND id = $2"
        } else {
            "UPDATE channels SET deleted_at = NULL WHERE community_id = $1 AND id = $2"
        };
        sqlx::query(sql)
            .bind(self.id)
            .bind(room)
            .execute(&self.pool)
            .await
            .expect("toggle room deletion");
    }
}

// ── R-3: executor spine and Shapers ──────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn bootstrap_emits_39103_with_the_hosted_pubkey_and_agent_hosted() {
    let c = Community::fresh().await;
    let owner_hex = c.owner.public_key().to_hex();
    let agent_hex = c.agent.public_key().to_hex();

    let (proposal_id, room) = c.bootstrap().await;

    let state = c.shapers_state().await.expect("39103 is live");
    assert_ne!(
        state["pubkey"], owner_hex,
        "39103 is relay-signed, not the owner's"
    );
    let body = content(&state);
    assert_eq!(body["founder"], owner_hex);
    assert_eq!(body["shapers"], json!([owner_hex]));
    assert_eq!(body["agent"], agent_hex, "the hosted key is the agent");
    assert_eq!(body["agent_hosted"], true);
    assert_eq!(body["room"], room.to_string());
    assert_eq!(p_tags(&state), vec![owner_hex.clone()]);

    let proposals = c
        .query(
            &c.owner,
            json!({ "kinds": [KIND_IO_PROPOSAL], "#d": [proposal_id] }),
        )
        .await;
    assert_eq!(proposals.len(), 1, "the bootstrap proposal is stored");
    let proposal = content(&proposals[0]);
    assert_eq!(proposal["kind"], "shapers");
    assert_eq!(proposal["status"], "passed");
    assert_eq!(
        proposal["executed"],
        json!({ "kind": "shapers", "id": "shapers" })
    );

    assert_eq!(c.roster(room).await, Community::expected_roster(&state));
    assert_eq!(c.roster(room).await.len(), 2, "owner + agent");
    assert_eq!(
        c.ledger_verbs().await,
        vec![
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "shaper_added",
            "agent_changed",
        ]
    );

    // The bootstrap is one-shot: a second, distinct self-add is not one.
    c.submit_rejected(
        &c.owner,
        &signed(
            &c.owner,
            KIND_IO_SHAPERS_PROPOSE,
            vec![tag(["op", "add"]), tag(["p", &owner_hex])],
            r#"{"why":"again"}"#,
        ),
        "invalid: shapers op=add proposals are not implemented yet",
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn a_client_event_of_39103_is_rejected_restricted() {
    let c = Community::fresh().await;
    let owner_hex = c.owner.public_key().to_hex();
    let forged = signed(
        &c.owner,
        KIND_IO_SHAPERS,
        vec![tag(["d", "shapers"]), tag(["p", &owner_hex])],
        r#"{"founder":"forged"}"#,
    );
    c.submit_rejected(&c.owner, &forged, "restricted: relay-only kind")
        .await;
    assert!(c.shapers_state().await.is_none(), "nothing was stored");
}

#[tokio::test]
#[ignore]
async fn a_non_owner_cannot_bootstrap() {
    let c = Community::fresh().await;
    let member = Keys::generate();
    c.seed_member(&member, "member").await;
    let member_hex = member.public_key().to_hex();

    c.submit_rejected(
        &member,
        &signed(
            &member,
            KIND_IO_SHAPERS_PROPOSE,
            vec![tag(["op", "add"]), tag(["p", &member_hex])],
            "{}",
        ),
        "restricted: only the community owner may bootstrap",
    )
    .await;

    assert!(c.shapers_state().await.is_none(), "no 39103");
    assert!(
        c.query(&c.owner, json!({ "kinds": [KIND_IO_PROPOSAL] }))
            .await
            .is_empty(),
        "no 39102"
    );
    assert!(
        c.query(
            &member,
            json!({ "kinds": [KIND_IO_SHAPERS_PROPOSE], "authors": [member_hex] })
        )
        .await
        .is_empty(),
        "the refused command itself is not stored (§3.2)"
    );
    assert!(c.ledger_verbs().await.is_empty());
    let rooms: i64 = sqlx::query_scalar("SELECT count(*) FROM channels WHERE community_id = $1")
        .bind(c.id)
        .fetch_one(&c.pool)
        .await
        .expect("count rooms");
    assert_eq!(rooms, 0, "no #shapers room");
}

#[tokio::test]
#[ignore]
async fn the_last_shaper_cannot_step_down() {
    let c = Community::fresh().await;
    let (_, room) = c.bootstrap().await;
    let before = c.shapers_state().await.expect("39103");

    c.submit_rejected(
        &c.owner,
        &signed(
            &c.owner,
            KIND_IO_SHAPER_STEP_DOWN,
            vec![],
            r#"{"why":"leaving"}"#,
        ),
        "restricted: the last Shaper cannot step down",
    )
    .await;

    let after = c.shapers_state().await.expect("39103");
    assert_eq!(after["id"], before["id"], "39103 is untouched");
    assert_eq!(c.roster(room).await, Community::expected_roster(&after));
}

#[tokio::test]
#[ignore]
async fn the_room_roster_equals_shapers_and_agent_after_each_change() {
    let c = Community::fresh().await;
    let (proposal_id, room) = c.bootstrap().await;
    let owner_hex = c.owner.public_key().to_hex();
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();

    let state = c.shapers_state().await.expect("39103 after bootstrap");
    assert_eq!(c.roster(room).await, Community::expected_roster(&state));

    // Accept before any offer: refused, and nothing moves.
    c.submit_rejected(
        &second,
        &signed(
            &second,
            KIND_IO_SHAPER_ACCEPT,
            vec![tag(["e", &proposal_id])],
            "{}",
        ),
        "restricted: no seat is offered to you on that proposal",
    )
    .await;
    assert_eq!(c.roster(room).await.len(), 2);

    // Accept an offered seat: 39103 is rewritten and the roster grows.
    c.offer_seat(&second, &proposal_id).await;
    let accept = signed(
        &second,
        KIND_IO_SHAPER_ACCEPT,
        vec![tag(["e", &proposal_id])],
        "{}",
    );
    c.submit_ok(&second, &accept).await;
    let state = c.shapers_state().await.expect("39103 after accept");
    assert_eq!(content(&state)["shapers"], json!([owner_hex, second_hex]));
    assert_eq!(content(&state)["receipt"], accept.id.to_hex());
    assert_eq!(p_tags(&state), vec![owner_hex.clone(), second_hex.clone()]);
    assert_eq!(c.roster(room).await, Community::expected_roster(&state));
    assert_eq!(c.roster(room).await.len(), 3, "two Shapers + the agent");

    // A replay of the same command is idempotent.
    let replay = c.submit_ok(&second, &accept).await;
    assert_eq!(replay, "duplicate: already processed");
    assert_eq!(c.roster(room).await.len(), 3);

    // The founder steps down: the roster follows.
    c.submit_ok(
        &c.owner,
        &signed(
            &c.owner,
            KIND_IO_SHAPER_STEP_DOWN,
            vec![],
            r#"{"why":"handing over"}"#,
        ),
    )
    .await;
    let state = c.shapers_state().await.expect("39103 after step down");
    assert_eq!(content(&state)["shapers"], json!([second_hex]));
    assert_eq!(content(&state)["founder"], owner_hex, "founder is history");
    assert_eq!(p_tags(&state), vec![second_hex.clone()]);
    let roster = c.roster(room).await;
    assert_eq!(roster, Community::expected_roster(&state));
    assert!(!roster.iter().any(|(p, _)| p == &owner_hex));
    assert_eq!(
        c.ledger_verbs().await.last().map(String::as_str),
        Some("shaper_stepped_down")
    );
}

#[tokio::test]
#[ignore]
async fn a_failure_after_the_projection_write_leaves_nothing_behind() {
    let c = Community::fresh().await;
    let (proposal_id, room) = c.bootstrap().await;
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    c.offer_seat(&second, &proposal_id).await;

    let before = c.shapers_state().await.expect("39103");
    let ledger_before = c.ledger_verbs().await;
    let roster_before = c.roster(room).await;

    // Break the step after `io_shapers` is written: the roster sync finds no
    // live room. The relay reports an internal error and unwinds everything.
    c.set_room_deleted(room, true).await;
    let accept = signed(
        &second,
        KIND_IO_SHAPER_ACCEPT,
        vec![tag(["e", &proposal_id])],
        "{}",
    );
    let (status, body) = c.submit(&second, &accept).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    c.set_room_deleted(room, false).await;

    let after = c.shapers_state().await.expect("39103");
    assert_eq!(after["id"], before["id"], "no new 39103");
    assert_eq!(
        content(&after)["shapers"],
        json!([c.owner.public_key().to_hex()])
    );
    assert_eq!(c.ledger_verbs().await, ledger_before, "no ledger row");
    assert_eq!(c.roster(room).await, roster_before, "no roster change");
    assert!(
        c.query(
            &second,
            json!({ "kinds": [KIND_IO_SHAPER_ACCEPT], "authors": [second_hex] })
        )
        .await
        .is_empty(),
        "the failed command was not stored"
    );

    // With the room back, the very same command goes through — it was never
    // recorded, so it is not a replay.
    c.submit_ok(&second, &accept).await;
    assert_eq!(c.roster(room).await.len(), 3);
}

// ── R-12: invites ────────────────────────────────────────────────────────────

/// Protocol §6.6, Features 6a: any Shaper can create an invite link; a plain
/// member cannot. The Shaper here is a `member` at the relay level — never
/// owner or admin — so only the live `39103.shapers` (read from `io_shapers`
/// at request time) can be what admits the mint. The claim of that code
/// lands `member_joined` with `minted_by` = the Shaper (V7).
#[tokio::test]
#[ignore]
async fn a_shaper_who_is_not_owner_or_admin_mints_and_a_plain_member_cannot() {
    let c = Community::fresh().await;
    let (proposal_id, room) = c.bootstrap().await;
    let shaper = Keys::generate();
    let plain = Keys::generate();
    let joiner = Keys::generate();
    c.seed_member(&shaper, "member").await;
    c.seed_member(&plain, "member").await;
    let shaper_hex = shaper.public_key().to_hex();
    let joiner_hex = joiner.public_key().to_hex();

    // Not yet a Shaper: a relay `member` is refused exactly as before R-12.
    let (status, body) = c.mint_invite(&shaper).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // Take the offered seat: the 39103 now names them, and nothing else about
    // them changes — the relay role stays `member`.
    c.offer_seat(&shaper, &proposal_id).await;
    c.submit_ok(
        &shaper,
        &signed(
            &shaper,
            KIND_IO_SHAPER_ACCEPT,
            vec![tag(["e", &proposal_id])],
            "{}",
        ),
    )
    .await;
    let state = c.shapers_state().await.expect("39103 after accept");
    assert!(content(&state)["shapers"]
        .as_array()
        .expect("shapers")
        .iter()
        .any(|p| p == &shaper_hex));
    assert_eq!(c.relay_role(&shaper).await.as_deref(), Some("member"));
    assert_eq!(c.roster(room).await.len(), 3, "two Shapers + the agent");

    // The Shaper mints; the plain member is told why not.
    let (status, body) = c.mint_invite(&shaper).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let code = body["code"].as_str().expect("code").to_owned();
    assert!(code.starts_with("v2."), "{code}");
    assert_eq!(
        body["url"].as_str(),
        Some(format!("{}://{}/invite/{code}", http_scheme(), c.host).as_str()),
        "the shareable landing URL is on the community's own host"
    );

    let (status, body) = c.mint_invite(&plain).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(
        body["error"],
        "only relay owners, admins, and Shapers can create invites"
    );
    assert!(
        c.member_joined_rows().await.is_empty(),
        "nothing joined yet"
    );

    // A stranger claims the Shaper's code and is a member: the ledger says
    // who let them in.
    let (status, body) = c.claim_invite(&joiner, &code).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "joined");
    assert_eq!(body["role"], "member");
    assert_eq!(c.relay_role(&joiner).await.as_deref(), Some("member"));
    assert_eq!(
        c.member_joined_rows().await,
        vec![(
            joiner_hex.clone(),
            json!({ "via": "invite", "minted_by": shaper_hex })
        )]
    );
    assert_eq!(
        c.ledger_verbs().await.last().map(String::as_str),
        Some("member_joined")
    );

    // A repeat claim is idempotent and writes no second row.
    let (status, body) = c.claim_invite(&joiner, &code).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "already_member");
    assert_eq!(c.member_joined_rows().await.len(), 1);

    // The new member is a member, not a Shaper: they cannot mint either.
    let (status, body) = c.mint_invite(&joiner).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // The Shaper steps down. The very next mint is refused — the authority is
    // the live set, read at request time, not a cached grant.
    c.submit_ok(
        &shaper,
        &signed(
            &shaper,
            KIND_IO_SHAPER_STEP_DOWN,
            vec![],
            r#"{"why":"done here"}"#,
        ),
    )
    .await;
    let state = c.shapers_state().await.expect("39103 after step down");
    assert_eq!(
        content(&state)["shapers"],
        json!([c.owner.public_key().to_hex()])
    );
    let (status, body) = c.mint_invite(&shaper).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // The owner never needed the Shaper set to mint.
    let (status, body) = c.mint_invite(&c.owner).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Readiness D7, Protocol §6.6: the `/invite/<code>` landing page fetches
/// `GET /api/join-policy` on the community's host, and for an intelligent
/// organization that reply carries the relay operator's fixed transparency
/// notice. A community no operator has provisioned gets none. The notice is
/// present whether or not an owner has bootstrapped a `39103` — the hosted
/// agent row alone makes the community an org.
#[tokio::test]
#[ignore]
async fn the_landing_page_carries_the_notice_for_an_org_community() {
    let org = Community::fresh().await;
    let plain = Community::fresh_plain().await;

    let (status, body) = org.get("/api/join-policy").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let notice = body["org"]["transparency_notice"]
        .as_str()
        .unwrap_or_else(|| panic!("an org community carries the notice: {body}"));
    for must_say in [
        "intelligent organization",
        "every channel and every direct message",
        "cite what it finds to any member",
        "every decision is a person's",
        "cannot be turned off by the community",
    ] {
        assert!(
            notice.contains(must_say),
            "notice lacks {must_say:?}: {notice}"
        );
    }
    assert!(
        notice.split("\n\n").count() >= 3,
        "the notice is more than one paragraph: {notice}"
    );

    // Bootstrapping does not change what the page says — the notice is the
    // operator's, not the Shapers'.
    org.bootstrap().await;
    let (status, after) = org.get("/api/join-policy").await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["org"]["transparency_notice"], notice);

    // A plain community on the same relay: no `org` half at all, and its
    // invites still work exactly as before.
    let (status, body) = plain.get("/api/join-policy").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.get("org").is_none(), "{body}");
    let (status, body) = plain.mint_invite(&plain.owner).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let code = body["code"].as_str().expect("code").to_owned();
    let joiner = Keys::generate();
    let (status, body) = plain.claim_invite(&joiner, &code).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "joined");
    assert_eq!(
        plain.member_joined_rows().await,
        vec![(
            joiner.public_key().to_hex(),
            json!({ "via": "invite", "minted_by": plain.owner.public_key().to_hex() })
        )],
        "the ledger fact is written for every community, org or not"
    );
}
