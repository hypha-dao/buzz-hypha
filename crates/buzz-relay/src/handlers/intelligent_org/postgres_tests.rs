//! R-3, R-4a, and R-4b proofs at the production seam: every command enters
//! through [`crate::handlers::ingest::ingest_event`] exactly as a WebSocket
//! or HTTP client's would, and every assertion reads the tables the relay
//! serves from. Redis is deliberately unreachable — fan-out is best-effort
//! and must not affect what commits.

use std::sync::Arc;

use buzz_auth::Scope;
use buzz_core::intelligent_org::{
    ChildrenCounts, DirectionArtifact, DirectionSlug, Proposal, ProposalStatus, Shapers,
    VoteChoice, WorkItem, WorkItemState,
};
use buzz_core::kind::{
    KIND_IO_DIRECTION, KIND_IO_DIRECTION_PROPOSE, KIND_IO_DRI_PROPOSE, KIND_IO_JOIN_PROPOSE,
    KIND_IO_MONEY_PROPOSE, KIND_IO_MONEY_RELEASED, KIND_IO_PROJECT_PROPOSE, KIND_IO_PROPOSAL,
    KIND_IO_SHAPERS, KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT, KIND_IO_SHAPER_STEP_DOWN,
    KIND_IO_VOTE, KIND_IO_WORK_ITEM,
};
use buzz_core::tenant::TenantContext;
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{self as store, HostedAgentRow, VoteRow, WorkItemRow};
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

    /// Give `keys` a plain relay membership (NIP-43 member, not owner).
    async fn member(&self, keys: &Keys) {
        self.state
            .db
            .add_relay_member(
                self.community(),
                &keys.public_key().to_hex(),
                "member",
                None,
            )
            .await
            .expect("seed member");
    }

    /// `io_shapers_propose` as `keys`: `op`, an optional `p`, `content`, and
    /// the D1 `["vote", "agree"]` when `agree`. Returns the reply JSON.
    async fn propose(
        &self,
        keys: &Keys,
        op: &str,
        p: Option<&str>,
        content: &str,
        agree: bool,
    ) -> Result<serde_json::Value, IngestError> {
        let mut tags = vec![tag(["op", op])];
        if let Some(p) = p {
            tags.push(tag(["p", p]));
        }
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        let message = self
            .send(keys, KIND_IO_SHAPERS_PROPOSE, tags, content)
            .await?;
        Ok(serde_json::from_str(&message)
            .unwrap_or_else(|e| panic!("propose reply is json: {e}: {message:?}")))
    }

    /// `io_vote` as `keys` on `proposal`.
    async fn vote(
        &self,
        keys: &Keys,
        proposal: &str,
        choice: &str,
        content: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(
                keys,
                KIND_IO_VOTE,
                vec![tag(["e", proposal]), tag(["vote", choice])],
                content,
            )
            .await?;
        Ok(serde_json::from_str(&message).expect("vote reply is json"))
    }

    /// Open `shapers/add` for `p` as `opener` with the opener's agree, then
    /// have every other `voter` agree. Returns the proposal id; the reply of
    /// the last command must say `passed`.
    async fn pass_add(&self, opener: &Keys, voters: &[&Keys], p: &Keys) -> String {
        let reply = self
            .propose(
                opener,
                "add",
                Some(&p.public_key().to_hex()),
                r#"{"why":"knows the domain"}"#,
                true,
            )
            .await
            .expect("open add");
        let proposal = reply["proposal"].as_str().expect("proposal id").to_owned();
        let mut status = reply["status"].clone();
        for voter in voters {
            status = self
                .vote(voter, &proposal, "agree", "{}")
                .await
                .expect("agree")["status"]
                .clone();
        }
        assert_eq!(status, "passed", "the add must pass to offer the seat");
        proposal
    }

    /// The real path to an offered seat: a passed `shapers/add`. With one
    /// Shaper the opener's agree passes it in one act.
    async fn offer_seat(&self, p: &Keys) -> String {
        self.pass_add(&self.owner, &[], p).await
    }

    /// Pass an add and accept it: `p` becomes a Shaper.
    async fn add_shaper(&self, opener: &Keys, voters: &[&Keys], p: &Keys) -> String {
        let proposal = self.pass_add(opener, voters, p).await;
        self.send(p, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &proposal])], "{}")
            .await
            .expect("accept the seat");
        proposal
    }

    async fn proposal(&self, id: &str) -> Proposal {
        let mut conn = self.pool.acquire().await.expect("acquire");
        store::get_proposal(
            &mut conn,
            self.community(),
            Uuid::parse_str(id).expect("proposal uuid"),
        )
        .await
        .expect("read io_proposals")
        .expect("proposal row")
        .content
    }

    async fn votes(&self, id: &str) -> Vec<VoteRow> {
        let mut conn = self.pool.acquire().await.expect("acquire");
        store::list_votes(
            &mut conn,
            self.community(),
            Uuid::parse_str(id).expect("proposal uuid"),
        )
        .await
        .expect("read io_votes")
    }

    /// `io_direction_propose` as `keys`.
    async fn direction(
        &self,
        keys: &Keys,
        slug: &str,
        base: u32,
        content: &str,
        agree: bool,
    ) -> Result<serde_json::Value, IngestError> {
        let mut tags = vec![tag(["d", slug]), tag(["base", &base.to_string()])];
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        let message = self
            .send(keys, KIND_IO_DIRECTION_PROPOSE, tags, content)
            .await?;
        Ok(serde_json::from_str(&message).expect("direction reply is json"))
    }

    /// `io_dri_propose` as `keys`.
    async fn dri(
        &self,
        keys: &Keys,
        item: &str,
        p: &str,
        content: &str,
        agree: bool,
    ) -> Result<serde_json::Value, IngestError> {
        let mut tags = vec![tag(["i", item]), tag(["p", p])];
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        let message = self.send(keys, KIND_IO_DRI_PROPOSE, tags, content).await?;
        Ok(serde_json::from_str(&message).expect("dri reply is json"))
    }

    /// `io_project_propose` as `keys`.
    async fn project(
        &self,
        keys: &Keys,
        content: &str,
        agree: bool,
    ) -> Result<serde_json::Value, IngestError> {
        let mut tags = Vec::new();
        if agree {
            tags.push(tag(["vote", "agree"]));
        }
        let message = self
            .send(keys, KIND_IO_PROJECT_PROPOSE, tags, content)
            .await?;
        Ok(serde_json::from_str(&message).expect("project reply is json"))
    }

    /// Seed a work item the way R-5a will write it — R-4b cannot create one
    /// through a command, so DRI proofs start from a projection row.
    async fn seed_item(&self, item: &WorkItem) {
        let mut conn = self.pool.acquire().await.expect("acquire");
        let now = chrono::Utc::now();
        store::upsert_work_item(
            &mut conn,
            self.community(),
            &WorkItemRow {
                content: item.clone(),
                event_id: vec![0x11; 32],
                last_progress_at: None,
                done_at: None,
                created_at: now,
                updated_at: now,
            },
        )
        .await
        .expect("seed work item");
    }

    async fn work_item(&self, id: &str) -> Option<WorkItem> {
        let mut conn = self.pool.acquire().await.expect("acquire");
        store::get_work_item(
            &mut conn,
            self.community(),
            Uuid::parse_str(id).expect("item uuid"),
        )
        .await
        .expect("read io_work_items")
        .map(|row| row.content)
    }

    async fn direction_head(&self, slug: DirectionSlug) -> Option<DirectionArtifact> {
        let mut conn = self.pool.acquire().await.expect("acquire");
        store::get_direction_head(&mut conn, self.community(), slug)
            .await
            .expect("read io_direction")
            .map(|row| row.content)
    }

    /// The live `39102` of `id` as `(content, tags)`.
    async fn live_proposal(&self, id: &str) -> (serde_json::Value, Vec<Vec<String>>) {
        let row = sqlx::query(
            "SELECT content, tags FROM events \
             WHERE community_id = $1 AND kind = $2 AND d_tag = $3 AND deleted_at IS NULL",
        )
        .bind(self.community().as_uuid())
        .bind(KIND_IO_PROPOSAL as i32)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .expect("exactly one live 39102");
        let content: String = row.get("content");
        let tags: serde_json::Value = row.get("tags");
        let tags = tags
            .as_array()
            .expect("tags array")
            .iter()
            .map(|t| {
                t.as_array()
                    .expect("tag array")
                    .iter()
                    .map(|v| v.as_str().expect("tag string").to_owned())
                    .collect()
            })
            .collect();
        (
            serde_json::from_str(&content).expect("39102 content json"),
            tags,
        )
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

fn rejected<T: std::fmt::Debug>(result: Result<T, IngestError>) -> String {
    match result {
        Err(IngestError::Rejected(message)) => message,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

fn internal<T: std::fmt::Debug>(result: Result<T, IngestError>) -> String {
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

    let add = h.offer_seat(&second).await;

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
    let accept = signed(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}");
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
        h.send(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "")
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
    let add = h.offer_seat(&second).await;

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

    let accept = signed(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}");
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
    h.send(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}")
        .await
        .expect("accept succeeds once the room is back");
    assert_eq!(h.roster(reply.room).await.len(), 3);
}

// ── R-4a: Shapers proposals and votes ─────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_passed_add_offers_a_seat_that_is_live_only_on_accept() {
    let h = harness().await;
    let reply = h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    let second_hex = second.public_key().to_hex();
    let roster_before = h.roster(reply.room).await;
    let ledger_before = h.ledger_verbs().await.len();

    // Without the D1 tag the proposal opens and waits, even for one Shaper.
    let opened = h
        .propose(&h.owner, "add", Some(&second_hex), "{}", false)
        .await
        .expect("open add");
    assert_eq!(opened["status"], "open");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&waiting).await;
    assert_eq!(p.status, ProposalStatus::Open);
    assert_eq!(p.needed, 1);
    assert_eq!(p.eligible, vec![owner_hex.clone()]);
    assert!(p.votes.is_empty(), "opening is not an agree");
    assert!(h.votes(&waiting).await.is_empty());
    assert_eq!(
        p.payload,
        serde_json::json!({ "op": "add", "p": second_hex }),
        "the payload carries the op and p its tags named"
    );
    assert_eq!(
        h.shapers_content().await.expect("39103").offered.len(),
        0,
        "an open add offers nothing"
    );
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["proposal_opened"]);

    // With it, a one-Shaper community passes its own add in one act: open,
    // vote, pass, and offer are one command and one transaction.
    let command = signed(
        &h.owner,
        KIND_IO_SHAPERS_PROPOSE,
        vec![
            tag(["op", "add"]),
            tag(["p", &second_hex]),
            tag(["vote", "agree"]),
        ],
        r#"{"why":"knows the domain"}"#,
    );
    let command_id = command.id.to_hex();
    let message = h.ingest(&h.owner, command).await.expect("add passes");
    let reply2: serde_json::Value = serde_json::from_str(&message).expect("json");
    assert_eq!(reply2["status"], "passed");
    let add = reply2["proposal"].as_str().expect("id").to_owned();
    assert_eq!(
        h.ledger_verbs().await[ledger_before + 1..],
        [
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "shaper_offered"
        ]
    );
    let p = h.proposal(&add).await;
    assert_eq!(p.status, ProposalStatus::Passed);
    assert_eq!(p.decided_at, Some(p.opened_at));
    assert_eq!(p.votes.len(), 1);
    assert_eq!(p.votes[0].p, owner_hex);
    assert_eq!(
        p.votes[0].receipt, command_id,
        "the opener's agree cites the opening command"
    );
    assert_eq!(
        p.executed
            .as_ref()
            .map(|e| (e.kind.as_str(), e.id.as_str())),
        Some(("shapers", "shapers"))
    );
    let votes = h.votes(&add).await;
    assert_eq!(votes.len(), 1);
    assert_eq!(hex::encode(&votes[0].voter), owner_hex);
    assert_eq!(votes[0].vote, VoteChoice::Agree);
    assert_eq!(hex::encode(&votes[0].receipt_event_id), command_id);

    let (content, tags) = h.live_proposal(&add).await;
    assert_eq!(content["status"], "passed");
    assert_eq!(
        tags,
        vec![
            vec!["d".to_owned(), add.clone()],
            vec!["t".to_owned(), "shapers".to_owned()],
            vec!["s".to_owned(), "passed".to_owned()],
            vec!["p".to_owned(), owner_hex.clone()],
            vec![
                "p".to_owned(),
                second_hex.clone(),
                String::new(),
                "subject".to_owned()
            ],
            vec![
                "p".to_owned(),
                owner_hex.clone(),
                String::new(),
                "eligible".to_owned()
            ],
            vec!["receipt".to_owned(), command_id.clone()],
        ]
    );

    // The seat is offered, not live: 39103.shapers and the roster are as before.
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.shapers, vec![owner_hex.clone()]);
    assert_eq!(shapers.offered.len(), 1);
    assert_eq!(shapers.offered[0].p, second_hex);
    assert_eq!(shapers.offered[0].proposal, add);
    assert_eq!(
        shapers.receipt, command_id,
        "the 39103 cites the passing command"
    );
    assert_eq!(h.roster(reply.room).await, roster_before);
    assert_eq!(
        h.live_state(KIND_IO_SHAPERS).await[0].2,
        vec![owner_hex.clone()],
        "no p tag for an offered seat"
    );

    // A second add for the same person while the seat is live is refused.
    assert_eq!(
        rejected(
            h.propose(&h.owner, "add", Some(&second_hex), "{}", true)
                .await
        ),
        "invalid: a seat is already offered to p"
    );

    // Accept makes it live and the roster follows.
    h.send(&second, KIND_IO_SHAPER_ACCEPT, vec![tag(["e", &add])], "{}")
        .await
        .expect("accept");
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.shapers, vec![owner_hex.clone(), second_hex.clone()]);
    assert!(shapers.offered.is_empty());
    assert_eq!(h.roster(reply.room).await, h.expected_roster(&shapers));

    // Adding a Shaper again is refused at opening; so is a non-Shaper opening.
    // (Distinct content from the first open, which would otherwise be the
    // same event id within the second and replay as a duplicate.)
    assert_eq!(
        rejected(
            h.propose(
                &h.owner,
                "add",
                Some(&second_hex),
                r#"{"why":"again"}"#,
                false
            )
            .await
        ),
        "invalid: already a Shaper"
    );
    let member = Keys::generate();
    h.member(&member).await;
    let refused = signed(
        &member,
        KIND_IO_SHAPERS_PROPOSE,
        vec![
            tag(["op", "add"]),
            tag(["p", &member.public_key().to_hex()]),
        ],
        "{}",
    );
    let refused_id = refused.id;
    assert_eq!(
        rejected(h.ingest(&member, refused).await),
        "restricted: not a Shaper"
    );
    assert!(
        !h.event_stored(&refused_id).await,
        "a refused open is not stored"
    );

    // The earlier open add still waits; the owner's 50003 passes it now. Its
    // p is already seated by the other add, so passing offers nothing and
    // rewrites no 39103 — the seat happened by another path.
    let state_before = h.live_state(KIND_IO_SHAPERS).await;
    let ledger_before = h.ledger_verbs().await.len();
    let voted = h
        .vote(
            &h.owner,
            &waiting,
            "agree",
            r#"{"reason":"still want them"}"#,
        )
        .await
        .expect("vote");
    assert_eq!(voted["status"], "passed");
    let votes = h.votes(&waiting).await;
    assert_eq!(votes.len(), 1);
    assert_eq!(votes[0].reason.as_deref(), Some("still want them"));
    assert_eq!(h.proposal(&waiting).await.status, ProposalStatus::Passed);
    assert!(h.shapers_content().await.expect("39103").offered.is_empty());
    assert_eq!(h.live_state(KIND_IO_SHAPERS).await, state_before);
    assert_eq!(
        h.ledger_verbs().await[ledger_before..],
        ["vote_cast", "proposal_passed"]
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_vote_from_a_non_eligible_pubkey_or_the_subject_is_rejected() {
    let h = harness().await;
    let reply = h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    let second_hex = second.public_key().to_hex();
    h.add_shaper(&h.owner, &[], &second).await;

    // Remove the second: eligible is the owner alone; needed 1.
    let opened = h
        .propose(
            &h.owner,
            "remove",
            Some(&second_hex),
            r#"{"why":"inactive"}"#,
            false,
        )
        .await
        .expect("open remove");
    let remove = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&remove).await;
    assert_eq!(
        p.eligible,
        vec![owner_hex.clone()],
        "the subject is not eligible"
    );
    assert_eq!(p.needed, 1);
    let (_, tags) = h.live_proposal(&remove).await;
    assert!(tags.contains(&vec![
        "p".to_owned(),
        second_hex.clone(),
        String::new(),
        "subject".to_owned()
    ]));

    let stranger = Keys::generate();
    let member = Keys::generate();
    h.member(&member).await;
    for (who, label) in [
        (&stranger, "a stranger"),
        (&member, "a member"),
        (&second, "the subject"),
    ] {
        let command = signed(
            who,
            KIND_IO_VOTE,
            vec![tag(["e", &remove]), tag(["vote", "agree"])],
            "{}",
        );
        let id = command.id;
        assert_eq!(
            rejected(h.ingest(who, command).await),
            "restricted: not eligible to vote on this proposal",
            "{label}"
        );
        assert!(
            !h.event_stored(&id).await,
            "{label}: the refused vote is not stored"
        );
    }
    let p = h.proposal(&remove).await;
    assert_eq!(p.status, ProposalStatus::Open);
    assert!(p.votes.is_empty());
    assert!(h.votes(&remove).await.is_empty());

    // A vote on a proposal that does not exist, and a malformed one.
    assert_eq!(
        rejected(
            h.vote(&h.owner, &Uuid::new_v4().to_string(), "agree", "{}")
                .await
        ),
        "invalid: unknown proposal"
    );
    assert_eq!(
        rejected(h.vote(&h.owner, &remove, "yes", "{}").await),
        "invalid: unknown vote \"yes\"; expected agree or decline"
    );
    assert_eq!(
        rejected(
            h.send(&h.owner, KIND_IO_VOTE, vec![tag(["e", &remove])], "{}")
                .await
        ),
        "invalid: missing vote tag"
    );

    // The one eligible Shaper agrees: passed, executed, roster follows.
    let voted = h
        .vote(&h.owner, &remove, "agree", "{}")
        .await
        .expect("vote");
    assert_eq!(voted["status"], "passed");
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.shapers, vec![owner_hex.clone()]);
    assert_eq!(h.roster(reply.room).await, h.expected_roster(&shapers));
    assert!(
        !h.roster(reply.room)
            .await
            .iter()
            .any(|(p, _)| p == &second_hex),
        "the removed Shaper leaves #shapers in the same transaction"
    );
    assert_eq!(
        h.ledger_verbs()
            .await
            .iter()
            .rev()
            .take(3)
            .rev()
            .cloned()
            .collect::<Vec<_>>(),
        ["vote_cast", "proposal_passed", "shaper_removed"]
    );

    // Decided proposals take no more votes.
    assert_eq!(
        rejected(h.vote(&h.owner, &remove, "decline", "{}").await),
        "invalid: proposal is not open"
    );

    // And the last Shaper cannot be removed, not even by themselves.
    assert_eq!(
        rejected(
            h.propose(&h.owner, "remove", Some(&owner_hex), "{}", true)
                .await
        ),
        "invalid: last shaper"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn needed_is_fixed_at_opening_while_a_shaper_is_added_mid_vote() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    let second_hex = second.public_key().to_hex();
    h.add_shaper(&h.owner, &[], &second).await;

    // Two Shapers, majority: needed 2, eligible both.
    let third = Keys::generate();
    let opened = h
        .propose(
            &h.owner,
            "add",
            Some(&third.public_key().to_hex()),
            "{}",
            false,
        )
        .await
        .expect("open");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let before = h.proposal(&waiting).await;
    assert_eq!(before.needed, 2);
    assert_eq!(before.eligible, vec![owner_hex.clone(), second_hex.clone()]);

    // Meanwhile a fourth Shaper is added and seated by another proposal.
    let fourth = Keys::generate();
    h.add_shaper(&h.owner, &[&second], &fourth).await;
    assert_eq!(h.shapers_content().await.expect("39103").shapers.len(), 3);

    // The bar of the open proposal has not moved, and the newcomer is not on it.
    let after = h.proposal(&waiting).await;
    assert_eq!(after.needed, 2);
    assert_eq!(after.eligible, before.eligible);
    assert_eq!(
        rejected(h.vote(&fourth, &waiting, "agree", "{}").await),
        "restricted: not eligible to vote on this proposal"
    );
    let one = h
        .vote(&h.owner, &waiting, "agree", "{}")
        .await
        .expect("first agree");
    assert_eq!(one["status"], "open", "one of two is not a majority of two");
    let two = h
        .vote(&second, &waiting, "agree", "{}")
        .await
        .expect("second agree");
    assert_eq!(two["status"], "passed");
    let p = h.proposal(&waiting).await;
    assert_eq!(p.needed, 2);
    assert_eq!(p.votes.len(), 2);
    assert_eq!(h.votes(&waiting).await.len(), 2);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_changed_vote_replaces_the_first_and_enough_declines_reject() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    let third = Keys::generate();
    h.add_shaper(&h.owner, &[], &second).await;
    h.add_shaper(&h.owner, &[&second], &third).await;

    // Three Shapers, majority → needed 2. The owner declines, then changes
    // their mind: one vote, one io_votes row, ledger vote_changed.
    let fourth = Keys::generate();
    let opened = h
        .propose(
            &h.owner,
            "add",
            Some(&fourth.public_key().to_hex()),
            "{}",
            false,
        )
        .await
        .expect("open");
    let add = opened["proposal"].as_str().expect("id").to_owned();
    let declined = h
        .vote(&h.owner, &add, "decline", r#"{"reason":"too soon"}"#)
        .await
        .expect("decline");
    assert_eq!(declined["status"], "open");
    let changed_command = signed(
        &h.owner,
        KIND_IO_VOTE,
        vec![tag(["e", &add]), tag(["vote", "agree"])],
        "{}",
    );
    let changed_id = changed_command.id.to_hex();
    h.ingest(&h.owner, changed_command).await.expect("change");
    let p = h.proposal(&add).await;
    assert_eq!(p.votes.len(), 1);
    assert_eq!(p.votes[0].vote, VoteChoice::Agree);
    assert_eq!(p.votes[0].receipt, changed_id);
    let votes = h.votes(&add).await;
    assert_eq!(votes.len(), 1);
    assert_eq!(votes[0].vote, VoteChoice::Agree);
    assert_eq!(
        votes[0].reason, None,
        "the changed vote's reason replaces the first"
    );
    assert_eq!(hex::encode(&votes[0].receipt_event_id), changed_id);
    assert_eq!(
        h.ledger_verbs()
            .await
            .iter()
            .rev()
            .take(2)
            .rev()
            .cloned()
            .collect::<Vec<_>>(),
        ["vote_cast", "vote_changed"]
    );
    let passed = h
        .vote(&second, &add, "agree", "{}")
        .await
        .expect("second agree");
    assert_eq!(passed["status"], "passed");

    // Remove the third: eligible {owner, second}, majority of 2 → needed 2,
    // so a single decline makes passing impossible.
    let opened = h
        .propose(
            &h.owner,
            "remove",
            Some(&third.public_key().to_hex()),
            "{}",
            true,
        )
        .await
        .expect("open remove with the opener's agree");
    assert_eq!(opened["status"], "open");
    let remove = opened["proposal"].as_str().expect("id").to_owned();
    let decided = h
        .vote(
            &second,
            &remove,
            "decline",
            r#"{"reason":"they do good work"}"#,
        )
        .await
        .expect("decline");
    assert_eq!(decided["status"], "rejected");
    let p = h.proposal(&remove).await;
    assert_eq!(p.status, ProposalStatus::Rejected);
    assert!(p.decided_at.is_some());
    assert!(p.executed.is_none());
    let (content, tags) = h.live_proposal(&remove).await;
    assert_eq!(content["status"], "rejected");
    assert!(tags.contains(&vec!["s".to_owned(), "rejected".to_owned()]));
    assert_eq!(
        h.ledger_verbs().await.last().map(String::as_str),
        Some("proposal_rejected")
    );
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.shapers.len(), 3, "a rejected remove removes nobody");
    assert!(shapers.shapers.contains(&third.public_key().to_hex()));
    assert_eq!(shapers.shapers[0], owner_hex);
    assert_eq!(
        rejected(h.vote(&h.owner, &remove, "agree", "{}").await),
        "invalid: proposal is not open"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn rules_and_agent_need_every_shaper_and_the_agent_must_be_a_non_shaper_member() {
    let h = harness().await;
    let reply = h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let hosted_hex = h.agent.public_key().to_hex();
    let second = Keys::generate();
    let second_hex = second.public_key().to_hex();
    h.member(&second).await;
    h.add_shaper(&h.owner, &[], &second).await;

    // rules: content is checked before anything is stored.
    assert_eq!(
        rejected(h.propose(&h.owner, "rules", None, "", false).await),
        "invalid: command content: missing field `rules`"
    );
    assert_eq!(
        rejected(
            h.propose(&h.owner, "rules", None, r#"{"rules":{"shapers":0}}"#, false)
                .await
        ),
        "invalid: rules.shapers must be at least 1"
    );

    // rules.shapers → 1 needs both Shapers although rules.shapers is majority
    // (which would also be 2 here); the proof is the next add.
    let opened = h
        .propose(
            &h.owner,
            "rules",
            None,
            r#"{"rules":{"shapers":1},"offer_window_secs":3600}"#,
            true,
        )
        .await
        .expect("open rules");
    assert_eq!(opened["status"], "open", "all of two is two");
    let rules = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&rules).await;
    assert_eq!(p.rule, buzz_core::intelligent_org::DecisionRule::ALL);
    assert_eq!(p.needed, 2);
    let passed = h.vote(&second, &rules, "agree", "{}").await.expect("agree");
    assert_eq!(passed["status"], "passed");
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(
        shapers.rules.shapers,
        buzz_core::intelligent_org::DecisionRule::AtLeast(1)
    );
    assert_eq!(shapers.offer_window_secs, 3600);
    assert_eq!(
        shapers.decision_window_secs,
        buzz_core::intelligent_org::DEFAULT_DECISION_WINDOW_SECS,
        "a window not given is kept"
    );
    assert_eq!(
        h.ledger_verbs().await.last().map(String::as_str),
        Some("rules_changed")
    );

    // Under rules.shapers = 1 an add passes on the opener's agree alone …
    let third = Keys::generate();
    let one_vote = h
        .propose(
            &h.owner,
            "add",
            Some(&third.public_key().to_hex()),
            "{}",
            true,
        )
        .await
        .expect("open add");
    assert_eq!(one_vote["status"], "passed");
    assert_eq!(
        h.proposal(one_vote["proposal"].as_str().expect("id"))
            .await
            .needed,
        1
    );

    // … but agent still needs all: the rule is ignored for rules and agent.
    let new_agent = Keys::generate();
    let new_agent_hex = new_agent.public_key().to_hex();
    assert_eq!(
        rejected(
            h.propose(&h.owner, "agent", Some(&new_agent_hex), "{}", true)
                .await
        ),
        "invalid: agent not a member"
    );
    assert_eq!(
        rejected(
            h.propose(&h.owner, "agent", Some(&second_hex), "{}", true)
                .await
        ),
        "invalid: agent is a shaper"
    );
    h.member(&new_agent).await;
    let opened = h
        .propose(
            &h.owner,
            "agent",
            Some(&new_agent_hex),
            r#"{"why":"our own"}"#,
            true,
        )
        .await
        .expect("open agent");
    assert_eq!(opened["status"], "open");
    let agent = opened["proposal"].as_str().expect("id").to_owned();
    assert_eq!(h.proposal(&agent).await.needed, 2);
    let (_, tags) = h.live_proposal(&agent).await;
    assert!(
        !tags.iter().any(|t| t.len() == 4 && t[3] == "subject"),
        "an agent's p is not a §4.4 subject"
    );
    let passed = h.vote(&second, &agent, "agree", "{}").await.expect("agree");
    assert_eq!(passed["status"], "passed");
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.agent.as_deref(), Some(new_agent_hex.as_str()));
    assert!(!shapers.agent_hosted);
    let roster = h.roster(reply.room).await;
    assert_eq!(roster, h.expected_roster(&shapers));
    assert!(roster
        .iter()
        .any(|(p, role)| p == &new_agent_hex && role == "member"));
    assert!(
        !roster.iter().any(|(p, _)| p == &hosted_hex),
        "the hosted key leaves #shapers with the swap"
    );
    assert_eq!(
        h.ledger_verbs().await.last().map(String::as_str),
        Some("agent_changed")
    );

    // No p: back to the hosted default from io_hosted_agents.
    let opened = h
        .propose(&h.owner, "agent", None, "{}", true)
        .await
        .expect("open agent (hosted)");
    let back = opened["proposal"].as_str().expect("id").to_owned();
    h.vote(&second, &back, "agree", "{}").await.expect("agree");
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.agent.as_deref(), Some(hosted_hex.as_str()));
    assert!(shapers.agent_hosted);
    assert_eq!(shapers.shapers, vec![owner_hex.clone(), second_hex.clone()]);
    let roster = h.roster(reply.room).await;
    assert_eq!(roster, h.expected_roster(&shapers));
    assert!(!roster.iter().any(|(p, _)| p == &new_agent_hex));
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn an_opener_vote_from_a_non_eligible_opener_is_ignored_and_decline_is_refused() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    h.add_shaper(&h.owner, &[], &second).await;

    // The owner proposes their own removal with an agree: not eligible, so
    // the tag is ignored and the proposal simply waits for the second.
    let opened = h
        .propose(&h.owner, "remove", Some(&owner_hex), "{}", true)
        .await
        .expect("open self-removal");
    assert_eq!(opened["status"], "open");
    let remove = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&remove).await;
    assert!(
        p.votes.is_empty(),
        "the non-eligible opener's agree is not recorded"
    );
    assert!(h.votes(&remove).await.is_empty());
    assert_eq!(p.eligible, vec![second.public_key().to_hex()]);

    // vote=decline on an opening command is malformed.
    let command = signed(
        &h.owner,
        KIND_IO_SHAPERS_PROPOSE,
        vec![tag(["op", "rules"]), tag(["vote", "decline"])],
        r#"{"rules":{}}"#,
    );
    let id = command.id;
    assert_eq!(
        rejected(h.ingest(&h.owner, command).await),
        "invalid: an opening command may only carry vote=agree"
    );
    assert!(!h.event_stored(&id).await);

    // The second agrees: the founder leaves and the second is the last Shaper.
    h.vote(&second, &remove, "agree", "{}")
        .await
        .expect("agree");
    let shapers = h.shapers_content().await.expect("39103");
    assert_eq!(shapers.shapers, vec![second.public_key().to_hex()]);
    assert_eq!(shapers.founder, owner_hex, "founder is history, not a seat");
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn money_and_join_commands_are_rejected_with_the_fixed_reasons() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
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
        let command = signed(&h.owner, kind, tags, content);
        let id = command.id;
        assert_eq!(
            rejected(h.ingest(&h.owner, command).await),
            reason,
            "kind {kind}"
        );
        assert!(!h.event_stored(&id).await, "kind {kind} is not stored");
    }
    assert!(
        h.live_state(KIND_IO_PROPOSAL).await.len() == 1,
        "only the bootstrap proposal exists"
    );
}

fn seed_work_item(
    id: &str,
    state: WorkItemState,
    dri: Option<String>,
    offered_to: Option<String>,
) -> WorkItem {
    WorkItem {
        id: id.to_owned(),
        parent: None,
        root: id.to_owned(),
        depth: 0,
        path: vec![id.to_owned()],
        title: "Weekday hall".into(),
        brief: "Book the hall".into(),
        state,
        dri,
        offered_to: offered_to.clone(),
        offered_by: offered_to.as_ref().map(|_| "agent".to_owned()),
        offered_at: offered_to.as_ref().map(|_| 1_700_000_000),
        due_at: 1_800_000_000,
        approved_at: None,
        objective_ref: None,
        created_from: hex::encode([0x22; 32]),
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

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_passed_direction_writes_39100_and_stale_base_is_rejected() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let member = Keys::generate();
    h.member(&member).await;
    let ledger_before = h.ledger_verbs().await.len();

    assert_eq!(
        rejected(
            h.direction(&member, "mission", 0, r#"{"body":"we exist"}"#, true)
                .await
        ),
        "restricted: not a Shaper"
    );
    assert_eq!(
        rejected(
            h.send(
                &h.owner,
                KIND_IO_DIRECTION_PROPOSE,
                vec![tag(["d", "mission"])],
                r#"{"body":"we exist"}"#,
            )
            .await
        ),
        "invalid: missing base tag"
    );
    assert_eq!(
        rejected(
            h.direction(&h.owner, "charter", 0, r#"{"body":"no"}"#, false)
                .await
        ),
        "invalid: unknown direction slug \"charter\"; expected mission, vision, objectives, or strategy"
    );
    assert_eq!(
        rejected(
            h.direction(
                &h.owner,
                "mission",
                0,
                r#"{"body":"we exist","lines":[{"text":"no"}]}"#,
                true
            )
            .await
        ),
        "invalid: mission and vision have no lines"
    );
    assert_eq!(
        rejected(
            h.direction(&h.owner, "mission", 1, r#"{"body":"we exist"}"#, true)
                .await
        ),
        "invalid: stale base"
    );

    // D1: one Shaper confirms mission in one command.
    let command = signed(
        &h.owner,
        KIND_IO_DIRECTION_PROPOSE,
        vec![
            tag(["d", "mission"]),
            tag(["base", "0"]),
            tag(["vote", "agree"]),
        ],
        r#"{"body":"we exist to host","why":"v1"}"#,
    );
    let command_id = command.id.to_hex();
    let reply: serde_json::Value =
        serde_json::from_str(&h.ingest(&h.owner, command).await.expect("open mission"))
            .expect("reply json");
    assert_eq!(reply["status"], "passed");
    let mission = reply["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&mission).await;
    assert_eq!(p.status, ProposalStatus::Passed);
    assert_eq!(p.needed, 1);
    assert_eq!(p.eligible, vec![owner_hex.clone()]);
    assert_eq!(p.votes.len(), 1);
    assert_eq!(p.votes[0].receipt, command_id);
    assert_eq!(
        p.executed,
        Some(buzz_core::intelligent_org::Executed {
            kind: "direction".into(),
            id: "mission".into(),
        })
    );
    assert_eq!(
        p.payload["slug"], "mission",
        "the payload carries the slug and base its tags named"
    );
    assert_eq!(p.payload["base"], 0);
    let artifact = h
        .direction_head(DirectionSlug::Mission)
        .await
        .expect("39100");
    assert_eq!(artifact.version, 1);
    assert_eq!(artifact.body, "we exist to host");
    assert!(artifact.lines.is_empty());
    assert_eq!(artifact.confirmed_by, owner_hex);
    assert_eq!(artifact.proposed_by, owner_hex);
    assert_eq!(artifact.proposal, mission);
    assert!(artifact.prev.is_none());
    let live = h.live_state(KIND_IO_DIRECTION).await;
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].1["slug"], "mission");
    assert_eq!(live[0].1["version"], 1);
    assert_eq!(
        h.ledger_verbs().await[ledger_before..],
        [
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "direction_confirmed"
        ]
    );

    assert_eq!(
        rejected(
            h.direction(&h.owner, "mission", 0, r#"{"body":"stale"}"#, true)
                .await
        ),
        "invalid: stale base"
    );

    // Two Shapers: needed is frozen at opening while a third is seated.
    let second = Keys::generate();
    h.add_shaper(&h.owner, &[], &second).await;
    let opened = h
        .direction(
            &h.owner,
            "objectives",
            0,
            r#"{"body":"the lines","lines":[{"id":"l_7f3a","text":"Weekday hall"}]}"#,
            false,
        )
        .await
        .expect("open objectives");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let before = h.proposal(&waiting).await;
    assert_eq!(before.needed, 2);
    assert_eq!(before.eligible.len(), 2);
    let third = Keys::generate();
    h.add_shaper(&h.owner, &[&second], &third).await;
    assert_eq!(h.proposal(&waiting).await.needed, 2);
    assert_eq!(h.proposal(&waiting).await.eligible, before.eligible);
    assert_eq!(
        rejected(h.vote(&third, &waiting, "agree", "{}").await),
        "restricted: not eligible to vote on this proposal"
    );
    assert_eq!(
        h.vote(&h.owner, &waiting, "agree", "{}")
            .await
            .expect("first")["status"],
        "open"
    );
    assert_eq!(
        h.vote(&second, &waiting, "agree", "{}")
            .await
            .expect("second")["status"],
        "passed"
    );
    let objectives = h
        .direction_head(DirectionSlug::Objectives)
        .await
        .expect("objectives");
    assert_eq!(objectives.version, 1);
    assert_eq!(objectives.lines.len(), 1);
    assert_eq!(objectives.lines[0].id, "l_7f3a");
    assert_eq!(objectives.lines[0].n, 1);
    assert_eq!(objectives.confirmed_by, second.public_key().to_hex());

    // A second confirm of the same slug: base must be 1; prev is the v1 id.
    let v1_id = h
        .live_state(KIND_IO_DIRECTION)
        .await
        .into_iter()
        .find(|(_, content, _)| content["slug"] == "objectives")
        .expect("objectives 39100")
        .0;
    let opened = h
        .direction(
            &h.owner,
            "objectives",
            1,
            r#"{"body":"the lines, updated","lines":[{"id":"l_7f3a","text":"Weekday hall booked"},{"text":"Stall running"}]}"#,
            true,
        )
        .await
        .expect("open v2");
    let v2 = opened["proposal"].as_str().expect("id").to_owned();
    assert_eq!(opened["status"], "open");
    assert_eq!(
        h.vote(&second, &v2, "agree", "{}").await.expect("pass v2")["status"],
        "passed"
    );
    let next = h
        .direction_head(DirectionSlug::Objectives)
        .await
        .expect("v2");
    assert_eq!(next.version, 2);
    assert_eq!(next.prev.as_deref(), Some(v1_id.as_str()));
    assert_eq!(next.lines.len(), 2);
    assert_eq!(next.lines[0].id, "l_7f3a");
    assert!(next.lines[1].id.starts_with("l_"));
    assert_ne!(next.lines[1].id, "l_7f3a");

    // Two proposals on the same base: the second passing vote is stale.
    let first = h
        .direction(&h.owner, "vision", 0, r#"{"body":"first"}"#, true)
        .await
        .expect("open vision a");
    let second_open = h
        .direction(&h.owner, "vision", 0, r#"{"body":"second"}"#, true)
        .await
        .expect("open vision b");
    let a = first["proposal"].as_str().expect("id").to_owned();
    let b = second_open["proposal"].as_str().expect("id").to_owned();
    assert_eq!(
        h.vote(&second, &a, "agree", "{}").await.expect("pass a")["status"],
        "passed"
    );
    assert_eq!(
        rejected(h.vote(&second, &b, "agree", "{}").await),
        "invalid: stale base"
    );
    assert_eq!(h.proposal(&b).await.status, ProposalStatus::Open);
    assert_eq!(
        h.direction_head(DirectionSlug::Vision)
            .await
            .expect("vision")
            .body,
        "first"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_passed_dri_sets_the_holder_and_the_subject_cannot_vote() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let second = Keys::generate();
    let second_hex = second.public_key().to_hex();
    h.add_shaper(&h.owner, &[], &second).await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;
    let stranger = Keys::generate();
    let open_id = Uuid::new_v4().to_string();
    let offered_id = Uuid::new_v4().to_string();
    let held_id = Uuid::new_v4().to_string();
    h.seed_item(&seed_work_item(&open_id, WorkItemState::Open, None, None))
        .await;
    h.seed_item(&seed_work_item(
        &offered_id,
        WorkItemState::Offered,
        None,
        Some(member_hex.clone()),
    ))
    .await;
    h.seed_item(&seed_work_item(
        &held_id,
        WorkItemState::Accepted,
        Some(member_hex.clone()),
        None,
    ))
    .await;

    assert_eq!(
        rejected(
            h.dri(
                &h.owner,
                &Uuid::new_v4().to_string(),
                &member_hex,
                "{}",
                false
            )
            .await
        ),
        "invalid: unknown item"
    );
    assert_eq!(
        rejected(h.dri(&h.owner, &held_id, &second_hex, "{}", false).await),
        "invalid: item already has a holder"
    );
    assert_eq!(
        rejected(
            h.send(
                &h.owner,
                KIND_IO_DRI_PROPOSE,
                vec![tag(["i", &open_id])],
                "{}",
            )
            .await
        ),
        "invalid: missing p tag"
    );
    assert_eq!(
        rejected(h.dri(&stranger, &open_id, &member_hex, "{}", false).await),
        "restricted: not a member"
    );

    // Name the second Shaper: they are the subject and cannot vote.
    let opened = h
        .dri(
            &h.owner,
            &open_id,
            &second_hex,
            r#"{"why":"they know the hall"}"#,
            false,
        )
        .await
        .expect("open dri");
    let dri = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&dri).await;
    assert_eq!(p.kind, buzz_core::intelligent_org::ProposalKind::Dri);
    assert_eq!(p.needed, 1);
    assert_eq!(p.eligible, vec![owner_hex.clone()]);
    assert_eq!(p.payload["i"], open_id);
    assert_eq!(p.payload["p"], second_hex);
    let (_, tags) = h.live_proposal(&dri).await;
    assert!(tags.contains(&vec![
        "p".to_owned(),
        second_hex.clone(),
        String::new(),
        "subject".to_owned()
    ]));
    assert!(tags.contains(&vec!["i".to_owned(), open_id.clone()]));
    for (who, label) in [
        (&stranger, "a stranger"),
        (&member, "a member"),
        (&second, "the subject"),
    ] {
        assert_eq!(
            rejected(h.vote(who, &dri, "agree", "{}").await),
            "restricted: not eligible to vote on this proposal",
            "{label}"
        );
    }
    let ledger_before = h.ledger_verbs().await.len();
    let voted = h
        .vote(&h.owner, &dri, "agree", "{}")
        .await
        .expect("pass dri");
    assert_eq!(voted["status"], "passed");
    let item = h.work_item(&open_id).await.expect("item");
    assert_eq!(item.state, WorkItemState::Accepted);
    assert_eq!(item.dri.as_deref(), Some(second_hex.as_str()));
    assert!(item.offered_to.is_none());
    let live = h.live_state(KIND_IO_WORK_ITEM).await;
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].1["state"], "accepted");
    assert_eq!(live[0].1["dri"], second_hex);
    assert_eq!(
        h.ledger_verbs().await[ledger_before..],
        ["vote_cast", "proposal_passed", "item_accepted"]
    );
    assert_eq!(
        rejected(h.dri(&h.owner, &open_id, &member_hex, "{}", true).await),
        "invalid: item already has a holder"
    );

    // An offered item: passing withdraws the foreign offer.
    let opened = h
        .dri(
            &member,
            &offered_id,
            &member_hex,
            r#"{"why":"I'll take it"}"#,
            false,
        )
        .await
        .expect("member may open a dri");
    let offered = opened["proposal"].as_str().expect("id").to_owned();
    assert_eq!(opened["status"], "open");
    assert_eq!(
        h.proposal(&offered).await.eligible,
        vec![owner_hex.clone(), second_hex.clone()],
        "a non-Shaper subject is not subtracted from anyone"
    );
    h.vote(&h.owner, &offered, "agree", "{}")
        .await
        .expect("first");
    assert_eq!(
        h.vote(&second, &offered, "agree", "{}")
            .await
            .expect("second")["status"],
        "passed"
    );
    let item = h.work_item(&offered_id).await.expect("offered item");
    assert_eq!(item.state, WorkItemState::Accepted);
    assert_eq!(item.dri.as_deref(), Some(member_hex.as_str()));
    assert!(item.offered_to.is_none());
    assert!(item.offered_by.is_none());
    assert!(item.offered_at.is_none());
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_passing_project_vote_is_refused_until_r5a() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    h.member(&member).await;
    let stranger = Keys::generate();
    let content = r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#;

    assert_eq!(
        rejected(h.project(&stranger, content, true).await),
        "restricted: not a member"
    );
    assert_eq!(
        rejected(h.project(&h.owner, r#"{"title":"no"}"#, true).await),
        "invalid: command content: missing field `brief`"
    );

    // A plain member can open; a one-Shaper opener-vote would pass, but
    // project execution is R-5a.
    let opened = h
        .project(&member, content, false)
        .await
        .expect("member opens a project");
    assert_eq!(opened["status"], "open");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&waiting).await;
    assert_eq!(p.kind, buzz_core::intelligent_org::ProposalKind::Project);
    assert_eq!(p.needed, 1);
    assert_eq!(p.eligible, vec![h.owner.public_key().to_hex()]);
    assert_eq!(
        rejected(
            h.project(
                &h.owner,
                r#"{"title":"Weekday hall","brief":"Book it now","due_at":1800000000}"#,
                true
            )
            .await
        ),
        "invalid: execution of project proposals is not implemented yet"
    );
    assert_eq!(
        rejected(h.vote(&h.owner, &waiting, "agree", "{}").await),
        "invalid: execution of project proposals is not implemented yet"
    );
    assert_eq!(h.proposal(&waiting).await.status, ProposalStatus::Open);
    assert!(h.proposal(&waiting).await.votes.is_empty());
    assert!(h.live_state(KIND_IO_WORK_ITEM).await.is_empty());
}
