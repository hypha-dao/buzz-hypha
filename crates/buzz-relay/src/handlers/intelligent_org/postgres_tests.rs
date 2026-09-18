//! R-3 proofs at the production seam: every command enters through
//! [`crate::handlers::ingest::ingest_event`] exactly as a WebSocket or HTTP
//! client's would, and every assertion reads the tables the relay serves
//! from. Redis is deliberately unreachable — fan-out is best-effort and must
//! not affect what commits.

use std::sync::Arc;

use buzz_auth::Scope;
use buzz_core::intelligent_org::{OfferedSeat, Shapers};
use buzz_core::kind::{
    KIND_IO_PROPOSAL, KIND_IO_SHAPERS, KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT,
    KIND_IO_SHAPER_STEP_DOWN,
};
use buzz_core::tenant::TenantContext;
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{self as store, HostedAgentRow};
use buzz_db::relay_rooms;
use nostr::{Event, EventBuilder, Keys, Kind, Tag, Timestamp};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::apply::{AGENT_ROOM_ROLE, SHAPER_ROOM_ROLE};
use super::shapers::SHAPERS_ROOM_NAME;
use crate::handlers::ingest::{ingest_event, HttpAuthMethod, IngestAuth, IngestError};
use crate::state::AppState;

struct Harness {
    state: Arc<AppState>,
    pool: PgPool,
    tenant: TenantContext,
    owner: Keys,
    agent: Keys,
}

impl Harness {
    fn community(&self) -> CommunityId {
        self.tenant.community()
    }

    /// Sign `kind` with `tags`/`content` and push it through ingest as `keys`.
    async fn send(
        &self,
        keys: &Keys,
        kind: u32,
        tags: Vec<Tag>,
        content: &str,
    ) -> Result<String, IngestError> {
        let event = signed(keys, kind, tags, content);
        self.ingest(keys, event).await
    }

    async fn ingest(&self, keys: &Keys, event: Event) -> Result<String, IngestError> {
        let auth = IngestAuth::Http {
            pubkey: keys.public_key(),
            scopes: vec![Scope::MessagesWrite],
            auth_method: HttpAuthMethod::Nip98,
        };
        ingest_event(&self.state, &self.tenant, event, auth)
            .await
            .map(|r| {
                assert!(r.accepted, "an Ok ingest result must be accepted");
                r.message
            })
    }

    async fn bootstrap(&self) -> BootstrapReply {
        let owner_hex = self.owner.public_key().to_hex();
        let message = self
            .send(
                &self.owner,
                KIND_IO_SHAPERS_PROPOSE,
                vec![tag(["op", "add"]), tag(["p", &owner_hex])],
                r#"{"why":"first Shaper"}"#,
            )
            .await
            .expect("owner bootstrap is accepted");
        let reply: serde_json::Value = serde_json::from_str(&message).expect("reply is json");
        BootstrapReply {
            proposal: reply["proposal"].as_str().expect("proposal id").to_owned(),
            room: Uuid::parse_str(reply["room"].as_str().expect("room id")).expect("room uuid"),
        }
    }

    async fn shapers_content(&self) -> Option<Shapers> {
        let mut conn = self.pool.acquire().await.expect("acquire");
        store::get_shapers(&mut conn, self.community())
            .await
            .expect("read io_shapers")
            .map(|row| row.content)
    }

    /// The live `39103`: `(id hex, content, p tags)`.
    async fn live_state(&self, kind: u32) -> Vec<(String, serde_json::Value, Vec<String>)> {
        let rows = sqlx::query(
            "SELECT id, content, tags FROM events \
             WHERE community_id = $1 AND kind = $2 AND deleted_at IS NULL ORDER BY created_at",
        )
        .bind(self.community().as_uuid())
        .bind(kind as i32)
        .fetch_all(&self.pool)
        .await
        .expect("read state events");
        rows.into_iter()
            .map(|row| {
                let id: Vec<u8> = row.get("id");
                let content: String = row.get("content");
                let tags: serde_json::Value = row.get("tags");
                let p_tags = tags
                    .as_array()
                    .expect("tags array")
                    .iter()
                    .filter(|t| t[0] == "p")
                    .map(|t| t[1].as_str().expect("p value").to_owned())
                    .collect();
                (
                    hex::encode(id),
                    serde_json::from_str(&content).expect("state content json"),
                    p_tags,
                )
            })
            .collect()
    }

    async fn event_stored(&self, id: &nostr::EventId) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM events WHERE community_id = $1 AND id = $2)",
        )
        .bind(self.community().as_uuid())
        .bind(id.as_bytes().to_vec())
        .fetch_one(&self.pool)
        .await
        .expect("event existence")
    }

    async fn ledger_verbs(&self) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT verb FROM io_ledger WHERE community_id = $1 ORDER BY id",
        )
        .bind(self.community().as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("ledger verbs")
    }

    /// Active `#shapers` roster as `(pubkey hex, role)`, sorted by pubkey.
    async fn roster(&self, room: Uuid) -> Vec<(String, String)> {
        let mut conn = self.pool.acquire().await.expect("acquire");
        let mut members: Vec<(String, String)> =
            relay_rooms::list_active_members(&mut conn, self.community(), room)
                .await
                .expect("list roster")
                .into_iter()
                .map(|m| (hex::encode(m.pubkey), m.role))
                .collect();
        members.sort();
        members
    }

    /// `shapers ∪ {agent}` with the roles `apply` promises (§6.4).
    fn expected_roster(&self, shapers: &Shapers) -> Vec<(String, String)> {
        let mut expected: Vec<(String, String)> = shapers
            .shapers
            .iter()
            .map(|p| (p.clone(), SHAPER_ROOM_ROLE.as_str().to_owned()))
            .collect();
        if let Some(agent) = &shapers.agent {
            if !shapers.shapers.contains(agent) {
                expected.push((agent.clone(), AGENT_ROOM_ROLE.as_str().to_owned()));
            }
        }
        expected.sort();
        expected
    }

    /// Seed an offered seat for `p` the way a passed `shapers/add` will in
    /// R-4a, so `io_shaper_accept` has something to take. Test setup only:
    /// the seam under test is the accept command, not the offer.
    async fn offer_seat(&self, p: &Keys, proposal: &str, at: u64) {
        let mut conn = self.pool.acquire().await.expect("acquire");
        let mut row = store::get_shapers(&mut conn, self.community())
            .await
            .expect("read io_shapers")
            .expect("39103 exists");
        row.content.offered.push(OfferedSeat {
            p: p.public_key().to_hex(),
            proposal: proposal.to_owned(),
            at,
        });
        store::upsert_shapers(&mut conn, self.community(), &row)
            .await
            .expect("seed offered seat");
    }
}

struct BootstrapReply {
    proposal: String,
    room: Uuid,
}

fn tag<const N: usize>(parts: [&str; N]) -> Tag {
    Tag::parse(parts).expect("tag")
}

/// Sign as a client would. `allow_self_tagging` matters: the bootstrap names
/// the sender in its own `p` tag, and nostr's builder drops that by default.
fn signed(keys: &Keys, kind: u32, tags: Vec<Tag>, content: &str) -> Event {
    EventBuilder::new(Kind::Custom(kind as u16), content)
        .tags(tags)
        .allow_self_tagging()
        .custom_created_at(Timestamp::now())
        .sign_with_keys(keys)
        .expect("sign")
}

fn rejected(result: Result<String, IngestError>) -> String {
    match result {
        Err(IngestError::Rejected(message)) => message,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

fn internal(result: Result<String, IngestError>) -> String {
    match result {
        Err(IngestError::Internal(message)) => message,
        other => panic!("expected an internal failure, got {other:?}"),
    }
}

/// A relay whose Postgres is real and whose Redis is unreachable, with a
/// fresh community that has an owner and a live hosted agent key.
async fn harness() -> Harness {
    let mut config = crate::config::Config::from_env().expect("default config loads");
    config.require_relay_membership = false;
    config.redis_url = "redis://127.0.0.1:1".to_string();
    config.database_url = crate::test_support::database_url();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("connect test database");
    let db = buzz_db::Db::from_pool(pool.clone());
    if std::env::var("BUZZ_TEST_SCHEMA_MODE").as_deref() != Ok("desired") {
        db.migrate().await.expect("migrate test database");
    }
    let redis_pool = deadpool_redis::Config::from_url(&config.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("redis pool");
    let pubsub = Arc::new(
        buzz_pubsub::PubSubManager::new(&config.redis_url, redis_pool.clone())
            .await
            .expect("pubsub manager"),
    );
    let audit = buzz_audit::AuditService::new(pool.clone());
    let auth = buzz_auth::AuthService::new(config.auth.clone());
    let search = buzz_search::SearchService::new(pool.clone());
    let workflow_engine = Arc::new(buzz_workflow::WorkflowEngine::new(
        db.clone(),
        buzz_workflow::WorkflowConfig::default(),
    ));
    let media_storage = buzz_media::MediaStorage::new(&config.media).expect("media storage");
    let (state, _audit_shutdown) = AppState::new(
        config,
        db.clone(),
        redis_pool,
        audit,
        pubsub,
        auth,
        search,
        workflow_engine,
        Keys::generate(),
        media_storage,
    );

    let host = format!("io-r3-{}.example", Uuid::new_v4().simple());
    let community = db
        .ensure_configured_community(&host)
        .await
        .expect("create community")
        .id;
    let owner = Keys::generate();
    db.add_relay_member(community, &owner.public_key().to_hex(), "owner", None)
        .await
        .expect("seed owner");
    let agent = Keys::generate();
    let mut conn = pool.acquire().await.expect("acquire");
    store::insert_hosted_agent(
        &mut conn,
        community,
        &HostedAgentRow {
            pubkey: agent.public_key().to_bytes().to_vec(),
            provisioned_at: chrono::Utc::now(),
            budget: None,
            retired_at: None,
        },
    )
    .await
    .expect("seed hosted agent");

    Harness {
        state: Arc::new(state),
        pool,
        tenant: TenantContext::resolved(community, host),
        owner,
        agent,
    }
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn bootstrap_writes_39103_with_the_hosted_agent_and_the_room_roster() {
    let h = harness().await;
    let owner_hex = h.owner.public_key().to_hex();
    let agent_hex = h.agent.public_key().to_hex();

    let reply = h.bootstrap().await;

    let shapers = h.shapers_content().await.expect("io_shapers row");
    assert_eq!(shapers.founder, owner_hex);
    assert_eq!(shapers.shapers, vec![owner_hex.clone()]);
    assert_eq!(shapers.agent.as_deref(), Some(agent_hex.as_str()));
    assert!(shapers.agent_hosted, "the hosted key is the default agent");
    assert_eq!(
        shapers.room.as_deref(),
        Some(reply.room.to_string().as_str())
    );

    let live = h.live_state(KIND_IO_SHAPERS).await;
    assert_eq!(live.len(), 1, "exactly one live 39103");
    let (_, content, p_tags) = &live[0];
    assert_eq!(content["agent"], agent_hex);
    assert_eq!(content["agent_hosted"], true);
    assert_eq!(content["shapers"], serde_json::json!([owner_hex]));
    assert_eq!(p_tags, &vec![owner_hex.clone()]);

    let proposals = h.live_state(KIND_IO_PROPOSAL).await;
    assert_eq!(
        proposals.len(),
        1,
        "the bootstrap proposal is stored passed"
    );
    assert_eq!(proposals[0].1["id"], reply.proposal);
    assert_eq!(proposals[0].1["status"], "passed");
    assert_eq!(proposals[0].1["kind"], "shapers");

    let room_row = sqlx::query(
        "SELECT name, visibility::text AS visibility, deleted_at FROM channels \
         WHERE community_id = $1 AND id = $2",
    )
    .bind(h.community().as_uuid())
    .bind(reply.room)
    .fetch_one(&h.pool)
    .await
    .expect("room row");
    assert_eq!(room_row.get::<String, _>("name"), SHAPERS_ROOM_NAME);
    assert_eq!(room_row.get::<String, _>("visibility"), "private");

    assert_eq!(h.roster(reply.room).await, h.expected_roster(&shapers));
    assert_eq!(
        h.ledger_verbs().await,
        vec![
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "shaper_added",
            "agent_changed",
        ]
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_client_event_of_39103_is_rejected_restricted() {
    let h = harness().await;
    let owner_hex = h.owner.public_key().to_hex();
    let forged = signed(
        &h.owner,
        KIND_IO_SHAPERS,
        vec![tag(["d", "shapers"]), tag(["p", &owner_hex])],
        r#"{"founder":"x"}"#,
    );
    let id = forged.id;
    let message = rejected(h.ingest(&h.owner, forged).await);
    assert_eq!(message, "restricted: relay-only kind");
    assert!(
        !h.event_stored(&id).await,
        "a rejected 39103 is never stored"
    );
    assert!(h.shapers_content().await.is_none());
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_non_owner_cannot_bootstrap_and_leaves_nothing_behind() {
    let h = harness().await;
    let stranger = Keys::generate();
    let stranger_hex = stranger.public_key().to_hex();
    let command = signed(
        &stranger,
        KIND_IO_SHAPERS_PROPOSE,
        vec![tag(["op", "add"]), tag(["p", &stranger_hex])],
        "{}",
    );
    let id = command.id;

    let message = rejected(h.ingest(&stranger, command).await);
    assert_eq!(
        message,
        "restricted: only the community owner may bootstrap"
    );

    assert!(
        !h.event_stored(&id).await,
        "the refused command must not be stored (§3.2)"
    );
    assert!(h.shapers_content().await.is_none(), "no io_shapers row");
    assert!(h.live_state(KIND_IO_SHAPERS).await.is_empty(), "no 39103");
    assert!(h.ledger_verbs().await.is_empty(), "no ledger rows");
    let rooms: i64 =
        sqlx::query_scalar("SELECT count(*) FROM channels WHERE community_id = $1 AND name = $2")
            .bind(h.community().as_uuid())
            .bind(SHAPERS_ROOM_NAME)
            .fetch_one(&h.pool)
            .await
            .expect("count rooms");
    assert_eq!(rooms, 0, "no #shapers room survives the rollback");

    // The owner naming someone else is not a bootstrap either.
    let message = rejected(
        h.send(
            &h.owner,
            KIND_IO_SHAPERS_PROPOSE,
            vec![tag(["op", "add"]), tag(["p", &stranger_hex])],
            "{}",
        )
        .await,
    );
    assert_eq!(message, "invalid: bootstrap must name the owner themselves");
    assert!(h.shapers_content().await.is_none());
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn the_last_shaper_cannot_step_down() {
    let h = harness().await;
    let reply = h.bootstrap().await;
    let before = h.live_state(KIND_IO_SHAPERS).await;

    let command = signed(
        &h.owner,
        KIND_IO_SHAPER_STEP_DOWN,
        vec![],
        r#"{"why":"leaving"}"#,
    );
    let id = command.id;
    let message = rejected(h.ingest(&h.owner, command).await);
    assert_eq!(message, "restricted: the last Shaper cannot step down");

    assert!(
        !h.event_stored(&id).await,
        "the refused command is not stored"
    );
    assert_eq!(
        h.live_state(KIND_IO_SHAPERS).await,
        before,
        "39103 unchanged"
    );
    let shapers = h.shapers_content().await.expect("io_shapers row");
    assert_eq!(shapers.shapers, vec![h.owner.public_key().to_hex()]);
    assert_eq!(h.roster(reply.room).await, h.expected_roster(&shapers));

    // A non-Shaper gets the role refusal, not the last-Shaper one.
    let stranger = Keys::generate();
    let message = rejected(
        h.send(&stranger, KIND_IO_SHAPER_STEP_DOWN, vec![], "{}")
            .await,
    );
    assert_eq!(message, "restricted: not a Shaper");
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn the_room_roster_equals_shapers_and_agent_after_each_change() {
    let h = harness().await;
    let reply = h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    let second_hex = second.public_key().to_hex();

    let now = Timestamp::now().as_secs();
    h.offer_seat(&second, &reply.proposal, now).await;

    // Accepting on the wrong proposal is refused and changes nothing.
    let wrong = Uuid::new_v4().to_string();
    let message = rejected(
        h.send(
            &second,
            KIND_IO_SHAPER_ACCEPT,
            vec![tag(["e", &wrong])],
            "{}",
        )
        .await,
    );
    assert_eq!(
        message,
        "restricted: no seat is offered to you on that proposal"
    );
    let shapers = h.shapers_content().await.expect("io_shapers row");
    assert_eq!(shapers.shapers, vec![owner_hex.clone()]);

    // Accept: the roster grows to two Shapers plus the agent.
    let accept = signed(
        &second,
        KIND_IO_SHAPER_ACCEPT,
        vec![tag(["e", &reply.proposal])],
        "{}",
    );
    let accept_id = accept.id;
    h.ingest(&second, accept.clone())
        .await
        .expect("accept the seat");
    let shapers = h.shapers_content().await.expect("io_shapers row");
    assert_eq!(shapers.shapers, vec![owner_hex.clone(), second_hex.clone()]);
    assert!(
        shapers.offered.is_empty(),
        "the taken seat leaves `offered`"
    );
    assert_eq!(shapers.receipt, accept_id.to_hex());
    assert_eq!(h.roster(reply.room).await, h.expected_roster(&shapers));
    assert_eq!(h.roster(reply.room).await.len(), 3);
    let live = h.live_state(KIND_IO_SHAPERS).await;
    assert_eq!(live.len(), 1, "NIP-33: one live 39103 after the rewrite");
    assert_eq!(live[0].2, vec![owner_hex.clone(), second_hex.clone()]);

    // Replaying the accepted command is idempotent.
    let replay = h.ingest(&second, accept).await.expect("replay is accepted");
    assert_eq!(replay, "duplicate: already processed");
    assert_eq!(h.roster(reply.room).await.len(), 3);
    assert_eq!(
        h.ledger_verbs()
            .await
            .iter()
            .filter(|v| *v == "shaper_added")
            .count(),
        2,
        "bootstrap + one accept; the replay adds no ledger row"
    );

    // Accepting twice with a fresh command (distinct id: empty content reads
    // as `{}` but hashes differently) is refused: already a Shaper.
    let message = rejected(
        h.send(
            &second,
            KIND_IO_SHAPER_ACCEPT,
            vec![tag(["e", &reply.proposal])],
            "",
        )
        .await,
    );
    assert_eq!(message, "invalid: already a Shaper");

    // Step down: the founder leaves, the roster shrinks to the second + agent.
    h.send(
        &h.owner,
        KIND_IO_SHAPER_STEP_DOWN,
        vec![],
        r#"{"why":"handing over"}"#,
    )
    .await
    .expect("founder steps down while another Shaper remains");
    let shapers = h.shapers_content().await.expect("io_shapers row");
    assert_eq!(shapers.shapers, vec![second_hex.clone()]);
    assert_eq!(shapers.founder, owner_hex, "founder is history, not a seat");
    let roster = h.roster(reply.room).await;
    assert_eq!(roster, h.expected_roster(&shapers));
    assert!(
        !roster.iter().any(|(p, _)| p == &owner_hex),
        "the stepped-down Shaper is removed from #shapers"
    );
    assert_eq!(h.live_state(KIND_IO_SHAPERS).await[0].2, vec![second_hex]);
    assert_eq!(
        h.ledger_verbs().await.last().map(String::as_str),
        Some("shaper_stepped_down")
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_failure_after_the_projection_write_rolls_everything_back() {
    let h = harness().await;
    let reply = h.bootstrap().await;
    let second = Keys::generate();
    h.offer_seat(&second, &reply.proposal, Timestamp::now().as_secs())
        .await;

    let before_state = h.live_state(KIND_IO_SHAPERS).await;
    let before_shapers = h.shapers_content().await.expect("io_shapers row");
    let before_ledger = h.ledger_verbs().await;
    let before_roster = h.roster(reply.room).await;

    // Break the step that runs *after* `io_shapers` is written: the roster
    // sync finds no live room and fails, so the whole command must unwind.
    sqlx::query("UPDATE channels SET deleted_at = now() WHERE community_id = $1 AND id = $2")
        .bind(h.community().as_uuid())
        .bind(reply.room)
        .execute(&h.pool)
        .await
        .expect("soft-delete #shapers");

    let accept = signed(
        &second,
        KIND_IO_SHAPER_ACCEPT,
        vec![tag(["e", &reply.proposal])],
        "{}",
    );
    let id = accept.id;
    let message = internal(h.ingest(&second, accept).await);
    assert!(
        message.contains("#shapers room") && message.contains("is missing"),
        "unexpected failure text: {message}"
    );

    assert!(!h.event_stored(&id).await, "no command event");
    assert_eq!(h.ledger_verbs().await, before_ledger, "no ledger row");
    assert_eq!(
        h.live_state(KIND_IO_SHAPERS).await,
        before_state,
        "no new 39103"
    );
    assert_eq!(
        h.shapers_content().await.expect("io_shapers row"),
        before_shapers,
        "the projection write rolled back with the rest"
    );
    assert_eq!(h.roster(reply.room).await, before_roster);

    // Restoring the room lets the same seat be taken; the command is fresh
    // because the failed one was never stored.
    sqlx::query("UPDATE channels SET deleted_at = NULL WHERE community_id = $1 AND id = $2")
        .bind(h.community().as_uuid())
        .bind(reply.room)
        .execute(&h.pool)
        .await
        .expect("restore #shapers");
    h.send(
        &second,
        KIND_IO_SHAPER_ACCEPT,
        vec![tag(["e", &reply.proposal])],
        "{}",
    )
    .await
    .expect("accept succeeds once the room is back");
    assert_eq!(h.roster(reply.room).await.len(), 3);
}
