//! End-to-end proofs for the intelligent organization (C-3), grown one relay
//! slice at a time. This file holds the R-3 lines — the executor spine and
//! the Shapers — the R-4a lines — `shapers` proposals, votes, and their
//! execution — the R-4b lines — `50002` / `50004`-opening / `50015`,
//! `50003` on those kinds, and `direction` / `dri` execution — the R-8
//! lines — bootstrap backfill of `39103.agent` and the `shapers/agent`
//! membership move — and the R-12 lines — invites minted by Shapers, the
//! `member_joined` ledger row, the transparency notice — driven through
//! the relay's real HTTP door (`POST /events`, `POST /query`,
//! `POST /api/invites`, `GET /api/join-policy`, NIP-98) exactly as a
//! client or the `buzz` CLI would.
//!
//! Every test gets its own community (a fresh `Host`) on the running relay,
//! because a `39103` bootstrap happens once per community and the relay
//! process is shared with the other e2e suites. The only rows a test seeds
//! directly are the ones an operator would: the community, its relay
//! members, and the hosted org-agent key in `io_hosted_agents`. Everything
//! else — including an offered seat — is reached by real commands.
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
    KIND_DM_OPEN, KIND_IO_DIRECTION, KIND_IO_DIRECTION_PROPOSE, KIND_IO_DRI_PROPOSE,
    KIND_IO_JOIN_PROPOSE, KIND_IO_MONEY_PROPOSE, KIND_IO_MONEY_RELEASED, KIND_IO_PROJECT_PROPOSE,
    KIND_IO_PROPOSAL, KIND_IO_SHAPERS, KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT,
    KIND_IO_SHAPER_STEP_DOWN, KIND_IO_VOTE, KIND_IO_WORK_ITEM, KIND_NIP29_CREATE_GROUP,
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

    /// An `io_shapers_propose` as `keys`: `op`, an optional `p`, `content`,
    /// and the D1 `["vote", "agree"]` when `agree`.
    fn propose_event(keys: &Keys, op: &str, p: Option<&str>, content: &str, agree: bool) -> Event {
        let mut tags = vec![tag(["op", op])];
        if let Some(p) = p {
            tags.push(tag(["p", p]));
        }
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        signed(keys, KIND_IO_SHAPERS_PROPOSE, tags, content)
    }

    fn direction_event(keys: &Keys, slug: &str, base: u32, content: &str, agree: bool) -> Event {
        let mut tags = vec![tag(["d", slug]), tag(["base", &base.to_string()])];
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        signed(keys, KIND_IO_DIRECTION_PROPOSE, tags, content)
    }

    fn dri_event(keys: &Keys, item: &str, p: &str, content: &str, agree: bool) -> Event {
        let mut tags = vec![tag(["i", item]), tag(["p", p])];
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        signed(keys, KIND_IO_DRI_PROPOSE, tags, content)
    }

    fn project_event(keys: &Keys, content: &str, agree: bool) -> Event {
        let mut tags = Vec::new();
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        signed(keys, KIND_IO_PROJECT_PROPOSE, tags, content)
    }

    async fn direction_ok(
        &self,
        keys: &Keys,
        slug: &str,
        base: u32,
        content: &str,
        agree: bool,
    ) -> Value {
        let message = self
            .submit_ok(
                keys,
                &Self::direction_event(keys, slug, base, content, agree),
            )
            .await;
        serde_json::from_str(&message).expect("direction reply json")
    }

    async fn dri_ok(&self, keys: &Keys, item: &str, p: &str, content: &str, agree: bool) -> Value {
        let message = self
            .submit_ok(keys, &Self::dri_event(keys, item, p, content, agree))
            .await;
        serde_json::from_str(&message).expect("dri reply json")
    }

    async fn project_ok(&self, keys: &Keys, content: &str, agree: bool) -> Value {
        let message = self
            .submit_ok(keys, &Self::project_event(keys, content, agree))
            .await;
        serde_json::from_str(&message).expect("project reply json")
    }

    /// Seed a work item by SQL — R-5a is the command path; DRI proofs need
    /// an unheld item before that lands.
    async fn seed_item(
        &self,
        id: Uuid,
        state: &str,
        dri: Option<&Keys>,
        offered_to: Option<&Keys>,
    ) {
        let dri_bytes = dri.map(|k| k.public_key().to_bytes().to_vec());
        let offered_bytes = offered_to.map(|k| k.public_key().to_bytes().to_vec());
        let content = json!({
            "id": id,
            "parent": null,
            "root": id,
            "depth": 0,
            "path": [id],
            "title": "Weekday hall",
            "brief": "Book the hall",
            "state": state,
            "dri": dri.map(|k| k.public_key().to_hex()),
            "offered_to": offered_to.map(|k| k.public_key().to_hex()),
            "offered_by": offered_to.map(|_| "agent"),
            "offered_at": offered_to.map(|_| 1_700_000_000u64),
            "due_at": 1_800_000_000u64,
            "created_from": hex::encode([0x22; 32]),
            "draft": null,
            "done_receipt": null,
            "closed_by": null,
            "children": { "open": 0, "offered": 0, "accepted": 0, "done": 0 },
        });
        sqlx::query(
            "INSERT INTO io_work_items \
                (community_id, id, root_id, parent_id, depth, kind, state, dri, offered_to, \
                 offered_at, due_at, content, event_id) \
             VALUES ($1, $2, $2, NULL, 0, 'project', $3, $4, $5, $6, to_timestamp(1800000000), \
                     $7, $8)",
        )
        .bind(self.id)
        .bind(id)
        .bind(state)
        .bind(dri_bytes)
        .bind(offered_bytes)
        .bind(offered_to.and_then(|_| chrono::DateTime::from_timestamp(1_700_000_000, 0)))
        .bind(content)
        .bind(vec![0x11u8; 32])
        .execute(&self.pool)
        .await
        .expect("seed work item");
    }

    async fn item_row(&self, id: Uuid) -> Option<(String, Option<String>, Option<String>)> {
        sqlx::query(
            "SELECT state, dri, offered_to FROM io_work_items \
             WHERE community_id = $1 AND id = $2",
        )
        .bind(self.id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .expect("read io_work_items")
        .map(|r| {
            (
                r.get("state"),
                r.get::<Option<Vec<u8>>, _>("dri").map(hex::encode),
                r.get::<Option<Vec<u8>>, _>("offered_to").map(hex::encode),
            )
        })
    }

    /// The live `39100` for `slug`, as the owner sees it.
    async fn direction_state(&self, slug: &str) -> Option<Value> {
        let mut events = self
            .query(
                &self.owner,
                json!({ "kinds": [KIND_IO_DIRECTION], "#d": [slug] }),
            )
            .await;
        assert!(
            events.len() <= 1,
            "NIP-33 keeps one 39100 per slug, got {}",
            events.len()
        );
        events.pop()
    }

    /// An `io_vote` as `keys` on `proposal`.
    fn vote_event(keys: &Keys, proposal: &str, choice: &str, content: &str) -> Event {
        signed(
            keys,
            KIND_IO_VOTE,
            vec![tag(["e", proposal]), tag(["vote", choice])],
            content,
        )
    }

    /// Submit a proposal that must be accepted; returns the reply JSON
    /// (`{proposal, status}`).
    async fn propose_ok(
        &self,
        keys: &Keys,
        op: &str,
        p: Option<&str>,
        content: &str,
        agree: bool,
    ) -> Value {
        let message = self
            .submit_ok(keys, &Self::propose_event(keys, op, p, content, agree))
            .await;
        serde_json::from_str(&message).expect("propose reply json")
    }

    /// Submit a vote that must be accepted; returns the reply JSON.
    async fn vote_ok(&self, keys: &Keys, proposal: &str, choice: &str, content: &str) -> Value {
        let message = self
            .submit_ok(keys, &Self::vote_event(keys, proposal, choice, content))
            .await;
        serde_json::from_str(&message).expect("vote reply json")
    }

    /// Open `shapers/add` for `p` as `opener` with the opener's agree, then
    /// have every `voter` agree; the last reply must say `passed`. Returns
    /// the proposal id. This is the real path to an offered seat.
    async fn pass_add(&self, opener: &Keys, voters: &[&Keys], p: &Keys) -> String {
        let reply = self
            .propose_ok(
                opener,
                "add",
                Some(&p.public_key().to_hex()),
                r#"{"why":"knows the domain"}"#,
                true,
            )
            .await;
        let proposal = reply["proposal"].as_str().expect("proposal id").to_owned();
        let mut status = reply["status"].clone();
        for voter in voters {
            status = self.vote_ok(voter, &proposal, "agree", "{}").await["status"].clone();
        }
        assert_eq!(status, "passed", "the add must pass to offer the seat");
        proposal
    }

    /// Offer `p` a seat: a passed `shapers/add` by the owner alone (one
    /// Shaper, so the opener's agree passes it).
    async fn offer_seat(&self, p: &Keys) -> String {
        self.pass_add(&self.owner, &[], p).await
    }

    /// Seat `p`: pass an add and accept it. `p` must already be a relay member.
    async fn add_shaper(&self, opener: &Keys, voters: &[&Keys], p: &Keys) -> String {
        let proposal = self.pass_add(opener, voters, p).await;
        self.submit_ok(
            p,
            &signed(p, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &proposal])], "{}"),
        )
        .await;
        proposal
    }

    /// The live `39102` for `id`, as the owner sees it.
    async fn proposal_state(&self, id: &str) -> Value {
        let mut events = self
            .query(
                &self.owner,
                json!({ "kinds": [KIND_IO_PROPOSAL], "#d": [id] }),
            )
            .await;
        assert_eq!(events.len(), 1, "one live 39102 for {id}");
        events.pop().expect("one event")
    }

    /// `io_votes` rows for `id` as `(voter hex, vote, reason)`, in vote order.
    async fn votes(&self, id: &str) -> Vec<(String, String, Option<String>)> {
        sqlx::query(
            "SELECT voter, vote, reason FROM io_votes \
             WHERE community_id = $1 AND proposal_id = $2 ORDER BY cast_at, voter",
        )
        .bind(self.id)
        .bind(Uuid::parse_str(id).expect("proposal uuid"))
        .fetch_all(&self.pool)
        .await
        .expect("read io_votes")
        .into_iter()
        .map(|r| {
            (
                hex::encode(r.get::<Vec<u8>, _>("voter")),
                r.get("vote"),
                r.get("reason"),
            )
        })
        .collect()
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

    /// Active membership pubkeys of `channel`, hex-sorted.
    async fn member_pubkeys(&self, channel: Uuid) -> Vec<String> {
        self.roster(channel)
            .await
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    }

    async fn open_dm(&self, keys: &Keys, others: &[&Keys]) -> Uuid {
        let tags = others
            .iter()
            .map(|k| tag(["p", &k.public_key().to_hex()]))
            .collect();
        let message = self
            .submit_ok(keys, &signed(keys, KIND_DM_OPEN, tags, ""))
            .await;
        let payload = message.strip_prefix("response:").expect("response: prefix");
        let reply: Value = serde_json::from_str(payload).expect("dm reply");
        Uuid::parse_str(reply["channel_id"].as_str().expect("channel_id")).expect("uuid")
    }

    async fn create_stream(&self, keys: &Keys) -> Uuid {
        let id = Uuid::new_v4();
        self.submit_ok(
            keys,
            &signed(
                keys,
                KIND_NIP29_CREATE_GROUP,
                vec![
                    tag(["h", &id.to_string()]),
                    tag(["name", &format!("room-{id}")]),
                    tag(["channel_type", "stream"]),
                    tag(["visibility", "open"]),
                ],
                "",
            ),
        )
        .await;
        id
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
            "agent_membership_synced",
        ]
    );

    // The bootstrap is one-shot: a second, distinct self-add is an ordinary
    // proposal now, and one that names a sitting Shaper is refused.
    c.submit_rejected(
        &c.owner,
        &signed(
            &c.owner,
            KIND_IO_SHAPERS_PROPOSE,
            vec![tag(["op", "add"]), tag(["p", &owner_hex])],
            r#"{"why":"again"}"#,
        ),
        "invalid: already a Shaper",
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

    // Accept an offered seat — offered by a passed shapers/add, the real
    // path: 39103 is rewritten and the roster grows.
    let add = c.offer_seat(&second).await;
    let accept = signed(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}");
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
    let (_, room) = c.bootstrap().await;
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    let add = c.offer_seat(&second).await;

    let before = c.shapers_state().await.expect("39103");
    let ledger_before = c.ledger_verbs().await;
    let roster_before = c.roster(room).await;

    // Break the step after `io_shapers` is written: the roster sync finds no
    // live room. The relay reports an internal error and unwinds everything.
    c.set_room_deleted(room, true).await;
    let accept = signed(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}");
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
    let (_, room) = c.bootstrap().await;
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

    // Take a seat offered by a passed shapers/add: the 39103 now names them,
    // and nothing else about them changes — the relay role stays `member`.
    let add = c.offer_seat(&shaper).await;
    c.submit_ok(
        &shaper,
        &signed(&shaper, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}"),
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

// ── R-4a: shapers proposals, votes, and execution ────────────────────────────

#[tokio::test]
#[ignore]
async fn a_passed_add_offers_a_seat_and_the_opener_vote_is_one_atomic_act() {
    let c = Community::fresh().await;
    let (_, room) = c.bootstrap().await;
    let owner_hex = c.owner.public_key().to_hex();
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    let roster_before = c.roster(room).await;
    let ledger_before = c.ledger_verbs().await.len();

    // Without the D1 tag the proposal opens and waits, even for one Shaper.
    let opened = c
        .propose_ok(&c.owner, "add", Some(&second_hex), "{}", false)
        .await;
    assert_eq!(opened["status"], "open");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let state = c.proposal_state(&waiting).await;
    assert_ne!(state["pubkey"], owner_hex, "39102 is relay-signed");
    let body = content(&state);
    assert_eq!(body["status"], "open");
    assert_eq!(body["needed"], 1);
    assert_eq!(body["eligible"], json!([owner_hex]));
    assert_eq!(body["votes"], json!([]), "opening is not an agree");
    assert_eq!(body["payload"], json!({ "op": "add", "p": second_hex }));
    assert!(c.votes(&waiting).await.is_empty());
    assert_eq!(
        content(&c.shapers_state().await.expect("39103"))["offered"],
        json!([]),
        "an open add offers nothing"
    );
    assert_eq!(
        &c.ledger_verbs().await[ledger_before..],
        ["proposal_opened"]
    );

    // With it, one Shaper passes their own add in one command: open, vote,
    // pass, and offer are one transaction and the vote cites the opener.
    let command = Community::propose_event(
        &c.owner,
        "add",
        Some(&second_hex),
        r#"{"why":"knows the domain"}"#,
        true,
    );
    let command_id = command.id.to_hex();
    let reply: Value =
        serde_json::from_str(&c.submit_ok(&c.owner, &command).await).expect("reply json");
    assert_eq!(reply["status"], "passed");
    let add = reply["proposal"].as_str().expect("id").to_owned();
    assert_eq!(
        &c.ledger_verbs().await[ledger_before + 1..],
        [
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "shaper_offered"
        ]
    );
    let state = c.proposal_state(&add).await;
    let body = content(&state);
    assert_eq!(body["status"], "passed");
    assert_eq!(body["decided_at"], body["opened_at"]);
    assert_eq!(body["votes"].as_array().expect("votes").len(), 1);
    assert_eq!(body["votes"][0]["p"], owner_hex);
    assert_eq!(body["votes"][0]["vote"], "agree");
    assert_eq!(
        body["votes"][0]["receipt"], command_id,
        "the opener's agree cites the opening command"
    );
    assert_eq!(
        body["executed"],
        json!({ "kind": "shapers", "id": "shapers" })
    );
    let tags = state["tags"].as_array().expect("tags");
    assert!(tags.contains(&json!(["s", "passed"])));
    assert!(tags.contains(&json!(["p", second_hex, "", "subject"])));
    assert!(tags.contains(&json!(["p", owner_hex, "", "eligible"])));
    assert!(tags.contains(&json!(["receipt", command_id])));
    assert_eq!(
        c.votes(&add).await,
        vec![(owner_hex.clone(), "agree".to_owned(), None)]
    );

    // The seat is offered, not live: shapers and the roster are as before.
    let shapers = content(&c.shapers_state().await.expect("39103"));
    assert_eq!(shapers["shapers"], json!([owner_hex]));
    assert_eq!(shapers["offered"].as_array().expect("offered").len(), 1);
    assert_eq!(shapers["offered"][0]["p"], second_hex);
    assert_eq!(shapers["offered"][0]["proposal"], add);
    assert_eq!(shapers["receipt"], command_id);
    assert_eq!(c.roster(room).await, roster_before);

    // A second add for the same person while the seat is live is refused, as
    // is any open from a member who is not a Shaper.
    c.submit_rejected(
        &c.owner,
        &Community::propose_event(&c.owner, "add", Some(&second_hex), "{}", true),
        "invalid: a seat is already offered to p",
    )
    .await;
    c.submit_rejected(
        &second,
        &Community::propose_event(&second, "add", Some(&second_hex), "{}", false),
        "restricted: not a Shaper",
    )
    .await;

    // Accept makes it live and the roster follows.
    c.submit_ok(
        &second,
        &signed(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}"),
    )
    .await;
    let state = c.shapers_state().await.expect("39103");
    assert_eq!(content(&state)["shapers"], json!([owner_hex, second_hex]));
    assert_eq!(content(&state)["offered"], json!([]));
    assert_eq!(c.roster(room).await, Community::expected_roster(&state));

    // The earlier open add still waits; the owner's 50003 passes it, and
    // since its p is already seated, passing offers nothing new.
    let voted = c
        .vote_ok(
            &c.owner,
            &waiting,
            "agree",
            r#"{"reason":"still want them"}"#,
        )
        .await;
    assert_eq!(voted["status"], "passed");
    assert_eq!(
        c.votes(&waiting).await,
        vec![(
            owner_hex.clone(),
            "agree".to_owned(),
            Some("still want them".to_owned())
        )]
    );
    assert_eq!(
        content(&c.proposal_state(&waiting).await)["status"],
        "passed"
    );
    assert_eq!(
        content(&c.shapers_state().await.expect("39103"))["offered"],
        json!([])
    );
}

#[tokio::test]
#[ignore]
async fn a_non_eligible_vote_and_the_subjects_own_vote_are_rejected() {
    let c = Community::fresh().await;
    let (_, room) = c.bootstrap().await;
    let owner_hex = c.owner.public_key().to_hex();
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    c.add_shaper(&c.owner, &[], &second).await;
    let member = Keys::generate();
    c.seed_member(&member, "member").await;

    // Remove the second: the subject is left out of eligible; needed 1.
    let opened = c
        .propose_ok(
            &c.owner,
            "remove",
            Some(&second_hex),
            r#"{"why":"inactive"}"#,
            false,
        )
        .await;
    assert_eq!(opened["status"], "open");
    let remove = opened["proposal"].as_str().expect("id").to_owned();
    let state = c.proposal_state(&remove).await;
    assert_eq!(content(&state)["eligible"], json!([owner_hex]));
    assert_eq!(content(&state)["needed"], 1);
    assert!(state["tags"]
        .as_array()
        .expect("tags")
        .contains(&json!(["p", second_hex, "", "subject"])));

    // A member who is not a Shaper, and the subject: refused, not stored.
    for (who, label) in [(&member, "a member"), (&second, "the subject")] {
        let vote = Community::vote_event(who, &remove, "agree", "{}");
        c.submit_rejected(
            who,
            &vote,
            "restricted: not eligible to vote on this proposal",
        )
        .await;
        assert!(
            c.query(
                who,
                json!({ "kinds": [KIND_IO_VOTE], "authors": [who.public_key().to_hex()] })
            )
            .await
            .is_empty(),
            "{label}: the refused vote is not stored"
        );
    }
    assert_eq!(
        content(&c.proposal_state(&remove).await)["votes"],
        json!([])
    );
    assert!(c.votes(&remove).await.is_empty());

    // Malformed votes are refused with their own reasons.
    c.submit_rejected(
        &c.owner,
        &Community::vote_event(&c.owner, &Uuid::new_v4().to_string(), "agree", "{}"),
        "invalid: unknown proposal",
    )
    .await;
    c.submit_rejected(
        &c.owner,
        &Community::vote_event(&c.owner, &remove, "yes", "{}"),
        "invalid: unknown vote \"yes\"; expected agree or decline",
    )
    .await;

    // The one eligible Shaper agrees: passed, executed, roster follows.
    let voted = c.vote_ok(&c.owner, &remove, "agree", "{}").await;
    assert_eq!(voted["status"], "passed");
    let state = c.shapers_state().await.expect("39103");
    assert_eq!(content(&state)["shapers"], json!([owner_hex]));
    assert_eq!(p_tags(&state), vec![owner_hex.clone()]);
    let roster = c.roster(room).await;
    assert_eq!(roster, Community::expected_roster(&state));
    assert!(
        !roster.iter().any(|(p, _)| p == &second_hex),
        "the removed Shaper leaves #shapers in the same transaction"
    );
    let verbs = c.ledger_verbs().await;
    assert_eq!(
        &verbs[verbs.len() - 3..],
        ["vote_cast", "proposal_passed", "shaper_removed"]
    );

    // Decided proposals take no more votes; the last Shaper cannot be removed.
    c.submit_rejected(
        &c.owner,
        &Community::vote_event(&c.owner, &remove, "decline", "{}"),
        "invalid: proposal is not open",
    )
    .await;
    c.submit_rejected(
        &c.owner,
        &Community::propose_event(&c.owner, "remove", Some(&owner_hex), "{}", true),
        "invalid: last shaper",
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn needed_is_fixed_at_opening_while_a_shaper_is_added_mid_vote() {
    let c = Community::fresh().await;
    c.bootstrap().await;
    let owner_hex = c.owner.public_key().to_hex();
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    c.add_shaper(&c.owner, &[], &second).await;

    // Two Shapers, majority: needed 2, eligible both.
    let third = Keys::generate();
    c.seed_member(&third, "member").await;
    let opened = c
        .propose_ok(
            &c.owner,
            "add",
            Some(&third.public_key().to_hex()),
            "{}",
            false,
        )
        .await;
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let before = content(&c.proposal_state(&waiting).await);
    assert_eq!(before["needed"], 2);
    assert_eq!(before["eligible"], json!([owner_hex, second_hex]));

    // Meanwhile a fourth Shaper is added and seated by another proposal.
    let fourth = Keys::generate();
    c.seed_member(&fourth, "member").await;
    c.add_shaper(&c.owner, &[&second], &fourth).await;
    assert_eq!(
        content(&c.shapers_state().await.expect("39103"))["shapers"]
            .as_array()
            .expect("shapers")
            .len(),
        3
    );

    // The bar of the open proposal has not moved, and the newcomer is not on it.
    let after = content(&c.proposal_state(&waiting).await);
    assert_eq!(after["needed"], 2);
    assert_eq!(after["eligible"], before["eligible"]);
    c.submit_rejected(
        &fourth,
        &Community::vote_event(&fourth, &waiting, "agree", "{}"),
        "restricted: not eligible to vote on this proposal",
    )
    .await;
    let one = c.vote_ok(&c.owner, &waiting, "agree", "{}").await;
    assert_eq!(one["status"], "open", "one of two is not a majority of two");
    let two = c.vote_ok(&second, &waiting, "agree", "{}").await;
    assert_eq!(two["status"], "passed");
    let decided = content(&c.proposal_state(&waiting).await);
    assert_eq!(decided["needed"], 2);
    assert_eq!(decided["votes"].as_array().expect("votes").len(), 2);
    assert_eq!(c.votes(&waiting).await.len(), 2);
}

#[tokio::test]
#[ignore]
async fn rules_and_agent_need_all_and_the_agent_must_be_a_non_shaper_member() {
    let c = Community::fresh().await;
    let (_, room) = c.bootstrap().await;
    let hosted_hex = c.agent.public_key().to_hex();
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    c.add_shaper(&c.owner, &[], &second).await;

    // rules: content is checked before anything is stored.
    c.submit_rejected(
        &c.owner,
        &Community::propose_event(&c.owner, "rules", None, r#"{"rules":{"shapers":0}}"#, false),
        "invalid: rules.shapers must be at least 1",
    )
    .await;

    // rules.shapers → 1 still needs both Shapers: rules always pass under all.
    let opened = c
        .propose_ok(
            &c.owner,
            "rules",
            None,
            r#"{"rules":{"shapers":1},"offer_window_secs":3600}"#,
            true,
        )
        .await;
    assert_eq!(opened["status"], "open", "all of two is two");
    let rules = opened["proposal"].as_str().expect("id").to_owned();
    let body = content(&c.proposal_state(&rules).await);
    assert_eq!(body["rule"], "all");
    assert_eq!(body["needed"], 2);
    let passed = c.vote_ok(&second, &rules, "agree", "{}").await;
    assert_eq!(passed["status"], "passed");
    let shapers = content(&c.shapers_state().await.expect("39103"));
    assert_eq!(shapers["rules"]["shapers"], 1);
    assert_eq!(shapers["offer_window_secs"], 3600);
    assert_eq!(
        c.ledger_verbs().await.last().map(String::as_str),
        Some("rules_changed")
    );

    // Under rules.shapers = 1 an add passes on the opener's agree alone …
    let third = Keys::generate();
    let one_vote = c
        .propose_ok(
            &c.owner,
            "add",
            Some(&third.public_key().to_hex()),
            "{}",
            true,
        )
        .await;
    assert_eq!(one_vote["status"], "passed");

    // … but agent still needs all, and its p must be a non-Shaper member.
    let new_agent = Keys::generate();
    let new_agent_hex = new_agent.public_key().to_hex();
    c.submit_rejected(
        &c.owner,
        &Community::propose_event(&c.owner, "agent", Some(&new_agent_hex), "{}", true),
        "invalid: agent not a member",
    )
    .await;
    c.submit_rejected(
        &c.owner,
        &Community::propose_event(&c.owner, "agent", Some(&second_hex), "{}", true),
        "invalid: agent is a shaper",
    )
    .await;
    c.seed_member(&new_agent, "member").await;
    let opened = c
        .propose_ok(
            &c.owner,
            "agent",
            Some(&new_agent_hex),
            r#"{"why":"our own"}"#,
            true,
        )
        .await;
    assert_eq!(opened["status"], "open");
    let agent = opened["proposal"].as_str().expect("id").to_owned();
    let state = c.proposal_state(&agent).await;
    assert_eq!(content(&state)["needed"], 2);
    assert!(
        !state["tags"].as_array().expect("tags").iter().any(|t| t
            .as_array()
            .is_some_and(|t| t.len() == 4 && t[3] == "subject")),
        "an agent's p is not a §4.4 subject"
    );
    let passed = c.vote_ok(&second, &agent, "agree", "{}").await;
    assert_eq!(passed["status"], "passed");
    let state = c.shapers_state().await.expect("39103");
    let shapers = content(&state);
    assert_eq!(shapers["agent"], new_agent_hex);
    assert_eq!(shapers["agent_hosted"], false);
    let roster = c.roster(room).await;
    assert_eq!(roster, Community::expected_roster(&state));
    assert!(roster
        .iter()
        .any(|(p, role)| p == &new_agent_hex && role == "member"));
    assert!(
        !roster.iter().any(|(p, _)| p == &hosted_hex),
        "the hosted key left #shapers"
    );
    assert_eq!(
        c.ledger_verbs().await.last().map(String::as_str),
        Some("agent_membership_synced")
    );

    // op=agent with no p returns to the hosted default.
    let back = c
        .propose_ok(&c.owner, "agent", None, r#"{"why":"back to hosted"}"#, true)
        .await;
    let back_id = back["proposal"].as_str().expect("id").to_owned();
    c.vote_ok(&second, &back_id, "agree", "{}").await;
    let shapers = content(&c.shapers_state().await.expect("39103"));
    assert_eq!(shapers["agent"], hosted_hex);
    assert_eq!(shapers["agent_hosted"], true);
}

#[tokio::test]
#[ignore]
async fn money_and_join_kinds_are_rejected_with_the_fixed_reasons() {
    let c = Community::fresh().await;
    c.bootstrap().await;
    let owner_hex = c.owner.public_key().to_hex();
    let item = Uuid::new_v4().to_string();
    for (kind, tags, content, reason) in [
        (
            KIND_IO_MONEY_PROPOSE,
            vec![tag(["i", &item]), tag(["p", &owner_hex])],
            r#"{"amount":"1","currency":"USD"}"#,
            "restricted: money not enabled",
        ),
        (
            KIND_IO_MONEY_RELEASED,
            vec![tag(["e", &item]), tag(["tx", "0xabc"])],
            r#"{"chain":"x","contract":"y","amount":"1","currency":"USD"}"#,
            "restricted: money not enabled",
        ),
        (
            KIND_IO_JOIN_PROPOSE,
            vec![tag(["p", &owner_hex])],
            "{}",
            "restricted: join not enabled",
        ),
    ] {
        c.submit_rejected(&c.owner, &signed(&c.owner, kind, tags, content), reason)
            .await;
        assert!(
            c.query(&c.owner, json!({ "kinds": [kind], "authors": [owner_hex] }))
                .await
                .is_empty(),
            "kind {kind} is not stored"
        );
    }
    assert_eq!(
        c.query(&c.owner, json!({ "kinds": [KIND_IO_PROPOSAL] }))
            .await
            .len(),
        1,
        "only the bootstrap proposal exists"
    );
}

// ── R-4b: direction / dri / project-opening ─────────────────────────────────

#[tokio::test]
#[ignore]
async fn a_passed_direction_writes_39100_and_stale_base_is_rejected() {
    let c = Community::fresh().await;
    c.bootstrap().await;
    let owner_hex = c.owner.public_key().to_hex();
    let member = Keys::generate();
    c.seed_member(&member, "member").await;
    let ledger_before = c.ledger_verbs().await.len();

    c.submit_rejected(
        &member,
        &Community::direction_event(&member, "mission", 0, r#"{"body":"we exist"}"#, true),
        "restricted: not a Shaper",
    )
    .await;
    c.submit_rejected(
        &c.owner,
        &Community::direction_event(&c.owner, "mission", 1, r#"{"body":"we exist"}"#, true),
        "invalid: stale base",
    )
    .await;

    let command = Community::direction_event(
        &c.owner,
        "mission",
        0,
        r#"{"body":"we exist to host"}"#,
        true,
    );
    let command_id = command.id.to_hex();
    let reply: Value =
        serde_json::from_str(&c.submit_ok(&c.owner, &command).await).expect("reply json");
    assert_eq!(reply["status"], "passed");
    let mission = reply["proposal"].as_str().expect("id").to_owned();
    assert_eq!(
        &c.ledger_verbs().await[ledger_before..],
        [
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "direction_confirmed"
        ]
    );
    let body = content(&c.proposal_state(&mission).await);
    assert_eq!(body["status"], "passed");
    assert_eq!(body["votes"][0]["receipt"], command_id);
    assert_eq!(
        body["executed"],
        json!({ "kind": "direction", "id": "mission" })
    );
    let state = c.direction_state("mission").await.expect("39100");
    assert_ne!(state["pubkey"], owner_hex, "39100 is relay-signed");
    let artifact = content(&state);
    assert_eq!(artifact["version"], 1);
    assert_eq!(artifact["body"], "we exist to host");
    assert_eq!(artifact["confirmed_by"], owner_hex);
    assert_eq!(artifact["proposal"], mission);
    assert!(artifact.get("prev").is_none());
    c.submit_rejected(
        &c.owner,
        &Community::direction_event(&c.owner, "mission", 0, r#"{"body":"stale"}"#, true),
        "invalid: stale base",
    )
    .await;

    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    c.add_shaper(&c.owner, &[], &second).await;
    let opened = c
        .direction_ok(
            &c.owner,
            "objectives",
            0,
            r#"{"body":"the lines","lines":[{"id":"l_7f3a","text":"Weekday hall"}]}"#,
            false,
        )
        .await;
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    assert_eq!(content(&c.proposal_state(&waiting).await)["needed"], 2);
    let third = Keys::generate();
    c.seed_member(&third, "member").await;
    c.add_shaper(&c.owner, &[&second], &third).await;
    assert_eq!(content(&c.proposal_state(&waiting).await)["needed"], 2);
    c.submit_rejected(
        &third,
        &Community::vote_event(&third, &waiting, "agree", "{}"),
        "restricted: not eligible to vote on this proposal",
    )
    .await;
    assert_eq!(
        c.vote_ok(&c.owner, &waiting, "agree", "{}").await["status"],
        "open"
    );
    assert_eq!(
        c.vote_ok(&second, &waiting, "agree", "{}").await["status"],
        "passed"
    );
    let objectives = content(&c.direction_state("objectives").await.expect("objectives"));
    assert_eq!(objectives["version"], 1);
    assert_eq!(objectives["lines"][0]["id"], "l_7f3a");
    assert_eq!(objectives["confirmed_by"], second.public_key().to_hex());
}

#[tokio::test]
#[ignore]
async fn a_passed_dri_sets_the_holder_and_the_subject_cannot_vote() {
    let c = Community::fresh().await;
    c.bootstrap().await;
    let second = Keys::generate();
    c.seed_member(&second, "member").await;
    let second_hex = second.public_key().to_hex();
    c.add_shaper(&c.owner, &[], &second).await;
    let member = Keys::generate();
    c.seed_member(&member, "member").await;
    let member_hex = member.public_key().to_hex();
    let open_id = Uuid::new_v4();
    let offered_id = Uuid::new_v4();
    let held_id = Uuid::new_v4();
    c.seed_item(open_id, "open", None, None).await;
    c.seed_item(offered_id, "offered", None, Some(&member))
        .await;
    c.seed_item(held_id, "accepted", Some(&member), None).await;

    c.submit_rejected(
        &c.owner,
        &Community::dri_event(&c.owner, &held_id.to_string(), &second_hex, "{}", false),
        "invalid: item already has a holder",
    )
    .await;
    let stranger = Keys::generate();
    c.submit_rejected(
        &stranger,
        &Community::dri_event(&stranger, &open_id.to_string(), &member_hex, "{}", false),
        "restricted: not a member",
    )
    .await;

    let opened = c
        .dri_ok(
            &c.owner,
            &open_id.to_string(),
            &second_hex,
            r#"{"why":"they know the hall"}"#,
            false,
        )
        .await;
    let dri = opened["proposal"].as_str().expect("id").to_owned();
    let body = content(&c.proposal_state(&dri).await);
    assert_eq!(body["kind"], "dri");
    assert_eq!(body["needed"], 1);
    assert_eq!(body["eligible"], json!([c.owner.public_key().to_hex()]));
    let tags = c.proposal_state(&dri).await["tags"]
        .as_array()
        .expect("tags")
        .clone();
    assert!(tags.contains(&json!(["p", second_hex, "", "subject"])));
    assert!(tags.contains(&json!(["i", open_id.to_string()])));
    for who in [&stranger, &member, &second] {
        c.submit_rejected(
            who,
            &Community::vote_event(who, &dri, "agree", "{}"),
            "restricted: not eligible to vote on this proposal",
        )
        .await;
    }
    assert_eq!(
        c.vote_ok(&c.owner, &dri, "agree", "{}").await["status"],
        "passed"
    );
    assert_eq!(
        c.item_row(open_id).await,
        Some(("accepted".into(), Some(second_hex.clone()), None))
    );
    let work = c
        .query(
            &c.owner,
            json!({ "kinds": [KIND_IO_WORK_ITEM], "#d": [open_id.to_string()] }),
        )
        .await;
    assert_eq!(work.len(), 1);
    assert_eq!(content(&work[0])["dri"], second_hex);
    assert_eq!(content(&work[0])["state"], "accepted");

    let opened = c
        .dri_ok(
            &member,
            &offered_id.to_string(),
            &member_hex,
            r#"{"why":"I'll take it"}"#,
            false,
        )
        .await;
    let offered = opened["proposal"].as_str().expect("id").to_owned();
    c.vote_ok(&c.owner, &offered, "agree", "{}").await;
    assert_eq!(
        c.vote_ok(&second, &offered, "agree", "{}").await["status"],
        "passed"
    );
    assert_eq!(
        c.item_row(offered_id).await,
        Some(("accepted".into(), Some(member_hex), None))
    );
}

#[tokio::test]
#[ignore]
async fn a_passing_project_vote_is_refused_until_r5a() {
    let c = Community::fresh().await;
    c.bootstrap().await;
    let member = Keys::generate();
    c.seed_member(&member, "member").await;
    let stranger = Keys::generate();
    let payload = r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#;

    c.submit_rejected(
        &stranger,
        &Community::project_event(&stranger, payload, true),
        "restricted: not a member",
    )
    .await;
    let opened = c.project_ok(&member, payload, false).await;
    assert_eq!(opened["status"], "open");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    assert_eq!(
        content(&c.proposal_state(&waiting).await)["kind"],
        "project"
    );
    c.submit_rejected(
        &c.owner,
        &Community::project_event(
            &c.owner,
            r#"{"title":"Weekday hall","brief":"Book it now","due_at":1800000000}"#,
            true,
        ),
        "invalid: execution of project proposals is not implemented yet",
    )
    .await;
    c.submit_rejected(
        &c.owner,
        &Community::vote_event(&c.owner, &waiting, "agree", "{}"),
        "invalid: execution of project proposals is not implemented yet",
    )
    .await;
    assert_eq!(content(&c.proposal_state(&waiting).await)["status"], "open");
    assert!(c
        .query(&c.owner, json!({ "kinds": [KIND_IO_WORK_ITEM] }))
        .await
        .is_empty());
}

// ── R-8: bootstrap backfill and shapers/agent membership move ────────────────

#[tokio::test]
#[ignore]
async fn bootstrap_backfills_existing_rooms_and_shapers_agent_moves_every_row() {
    let c = Community::fresh().await;
    let alice = Keys::generate();
    c.seed_member(&alice, "member").await;
    let before_room = c.create_stream(&c.owner).await;
    let before_dm = c.open_dm(&c.owner, &[&alice]).await;
    let hosted = c.agent.public_key().to_hex();
    assert!(
        !c.member_pubkeys(before_room).await.contains(&hosted),
        "no 39103 yet: the agent is not in the room"
    );

    let (_, room) = c.bootstrap().await;
    assert!(
        c.member_pubkeys(before_room).await.contains(&hosted),
        "bootstrap backfills existing channels"
    );
    assert!(
        c.member_pubkeys(before_dm).await.contains(&hosted),
        "bootstrap backfills existing DMs"
    );
    assert!(
        c.ledger_verbs()
            .await
            .contains(&"agent_membership_synced".to_owned()),
        "bootstrap writes the backfill ledger row"
    );

    let new_agent = Keys::generate();
    let new_hex = new_agent.public_key().to_hex();
    c.seed_member(&new_agent, "member").await;
    c.propose_ok(
        &c.owner,
        "agent",
        Some(&new_hex),
        r#"{"why":"our own"}"#,
        true,
    )
    .await;

    for channel in [room, before_room, before_dm] {
        let members = c.member_pubkeys(channel).await;
        assert!(members.contains(&new_hex), "the new key is in every room");
        assert!(!members.contains(&hosted), "the old key is in no room");
    }
}
