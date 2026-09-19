//! R-3, R-4a, R-4b, R-5a, and R-8 proofs at the production seam: every command
//! enters through [`crate::handlers::ingest::ingest_event`] exactly as a
//! WebSocket or HTTP client's would, and every assertion reads the tables
//! the relay serves from. Redis is deliberately unreachable — fan-out is
//! best-effort and must not affect what commits.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use buzz_auth::Scope;
use buzz_core::intelligent_org::{
    DirectionArtifact, DirectionSlug, Proposal, ProposalStatus, Shapers, VoteChoice, WorkItem,
    WorkItemState,
};
use buzz_core::kind::{
    KIND_DM_ADD_MEMBER, KIND_DM_OPEN, KIND_IO_ACCEPT, KIND_IO_DECLINE, KIND_IO_DIRECTION,
    KIND_IO_DIRECTION_PROPOSE, KIND_IO_DONE, KIND_IO_DRI_PROPOSE, KIND_IO_JOIN_PROPOSE,
    KIND_IO_MONEY_PROPOSE, KIND_IO_MONEY_RELEASED, KIND_IO_OFFER, KIND_IO_PROJECT_PROPOSE,
    KIND_IO_PROPOSAL, KIND_IO_RELEASE, KIND_IO_REOPEN, KIND_IO_SET_DUE, KIND_IO_SHAPERS,
    KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT, KIND_IO_SHAPER_STEP_DOWN,
    KIND_IO_TICKET_CREATE, KIND_IO_VOTE, KIND_IO_WORK_ITEM, KIND_NIP29_CREATE_GROUP,
    KIND_NIP29_GROUP_MEMBERS, KIND_NIP29_GROUP_METADATA,
};
use buzz_core::tenant::TenantContext;
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{self as store, HostedAgentRow, VoteRow};
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
            scopes: vec![Scope::MessagesWrite, Scope::ChannelsWrite],
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

    /// Active membership pubkeys of `channel`, hex-sorted.
    async fn member_pubkeys(&self, channel: Uuid) -> Vec<String> {
        let mut members = self.roster(channel).await;
        members.sort();
        members.into_iter().map(|(p, _)| p).collect()
    }

    /// `p` tag values of the live discovery event for `channel`.
    async fn discovery_p_tags(&self, kind: u32, channel: Uuid) -> Vec<String> {
        let rows = sqlx::query(
            "SELECT tags FROM events \
             WHERE community_id = $1 AND kind = $2 AND channel_id = $3 \
               AND deleted_at IS NULL \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(self.community().as_uuid())
        .bind(kind as i32)
        .bind(channel)
        .fetch_optional(&self.pool)
        .await
        .expect("read discovery event");
        let Some(row) = rows else {
            return Vec::new();
        };
        let tags: serde_json::Value = row.get("tags");
        let mut p: Vec<String> = tags
            .as_array()
            .expect("tags")
            .iter()
            .filter(|t| t[0] == "p")
            .map(|t| t[1].as_str().expect("p").to_owned())
            .collect();
        p.sort();
        p
    }

    /// `participants` of the `41010` system message (kind 40099) for `channel`.
    async fn dm_system_participants(&self, channel: Uuid) -> Vec<String> {
        let content: String = sqlx::query_scalar(
            "SELECT content FROM events \
             WHERE community_id = $1 AND kind = 40099 AND channel_id = $2 \
               AND deleted_at IS NULL \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(self.community().as_uuid())
        .bind(channel)
        .fetch_one(&self.pool)
        .await
        .expect("system message");
        let body: serde_json::Value = serde_json::from_str(&content).expect("system json");
        let mut p: Vec<String> = body["participants"]
            .as_array()
            .expect("participants")
            .iter()
            .map(|v| v.as_str().expect("hex").to_owned())
            .collect();
        p.sort();
        p
    }

    async fn open_dm(&self, keys: &Keys, others: &[&Keys]) -> Uuid {
        let tags = others
            .iter()
            .map(|k| tag(["p", &k.public_key().to_hex()]))
            .collect();
        let message = self
            .send(keys, KIND_DM_OPEN, tags, "")
            .await
            .expect("41010 accepted");
        let payload = message.strip_prefix("response:").expect("response: prefix");
        let reply: serde_json::Value = serde_json::from_str(payload).expect("dm reply");
        Uuid::parse_str(reply["channel_id"].as_str().expect("channel_id")).expect("uuid")
    }

    async fn create_stream(&self, keys: &Keys) -> Uuid {
        let id = Uuid::new_v4();
        self.send(
            keys,
            KIND_NIP29_CREATE_GROUP,
            vec![
                tag(["h", &id.to_string()]),
                tag(["name", &format!("room-{id}")]),
                tag(["channel_type", "stream"]),
                tag(["visibility", "open"]),
            ],
            "",
        )
        .await
        .expect("9007 accepted");
        id
    }

    async fn work_items(&self) -> Vec<WorkItem> {
        let rows = sqlx::query(
            "SELECT content FROM io_work_items \
             WHERE community_id = $1 ORDER BY created_at, id",
        )
        .bind(self.community().as_uuid())
        .fetch_all(&self.pool)
        .await
        .expect("list io_work_items");
        rows.into_iter()
            .map(|row| serde_json::from_value(row.get("content")).expect("work item content"))
            .collect()
    }

    /// Pass a `project` as `keys` (D1 opener-vote) and return the new root.
    async fn pass_project(&self, keys: &Keys, content: &str) -> (String, WorkItem) {
        let before: HashSet<String> = self.work_items().await.into_iter().map(|i| i.id).collect();
        let reply = self
            .project(keys, content, true)
            .await
            .expect("pass project");
        assert_eq!(reply["status"], "passed", "project must pass: {reply}");
        let proposal = reply["proposal"].as_str().expect("id").to_owned();
        let item = self
            .work_items()
            .await
            .into_iter()
            .find(|item| !before.contains(&item.id))
            .expect("passed project opens a root");
        (proposal, item)
    }

    async fn ticket(
        &self,
        keys: &Keys,
        parent: &str,
        offer_to: Option<&str>,
        content: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let mut tags = vec![tag(["u", parent])];
        if let Some(p) = offer_to {
            tags.push(tag(["p", p]));
        }
        let message = self
            .send(keys, KIND_IO_TICKET_CREATE, tags, content)
            .await?;
        Ok(serde_json::from_str(&message).expect("ticket reply is json"))
    }

    async fn offer_item(
        &self,
        keys: &Keys,
        item: &str,
        p: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(
                keys,
                KIND_IO_OFFER,
                vec![tag(["i", item]), tag(["p", p])],
                "{}",
            )
            .await?;
        Ok(serde_json::from_str(&message)
            .unwrap_or_else(|e| panic!("offer reply is json: {e}: {message:?}")))
    }

    async fn accept_item(&self, keys: &Keys, item: &str) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(keys, KIND_IO_ACCEPT, vec![tag(["i", item])], "{}")
            .await?;
        Ok(serde_json::from_str(&message).expect("accept reply is json"))
    }

    async fn decline_item(
        &self,
        keys: &Keys,
        item: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(keys, KIND_IO_DECLINE, vec![tag(["i", item])], "{}")
            .await?;
        Ok(serde_json::from_str(&message).expect("decline reply is json"))
    }

    async fn done_item(
        &self,
        keys: &Keys,
        item: &str,
        content: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(keys, KIND_IO_DONE, vec![tag(["i", item])], content)
            .await?;
        Ok(serde_json::from_str(&message).expect("done reply is json"))
    }

    async fn release_item(
        &self,
        keys: &Keys,
        item: &str,
        content: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(keys, KIND_IO_RELEASE, vec![tag(["i", item])], content)
            .await?;
        Ok(serde_json::from_str(&message).expect("release reply is json"))
    }

    async fn set_due_item(
        &self,
        keys: &Keys,
        item: &str,
        due_at: u64,
        content: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(
                keys,
                KIND_IO_SET_DUE,
                vec![tag(["i", item]), tag(["due", &due_at.to_string()])],
                content,
            )
            .await?;
        Ok(serde_json::from_str(&message).expect("set-due reply is json"))
    }

    async fn reopen_item(
        &self,
        keys: &Keys,
        item: &str,
        content: &str,
    ) -> Result<serde_json::Value, IngestError> {
        let message = self
            .send(keys, KIND_IO_REOPEN, vec![tag(["i", item])], content)
            .await?;
        Ok(serde_json::from_str(&message).expect("reopen reply is json"))
    }

    async fn live_work_count(&self) -> usize {
        self.live_state(KIND_IO_WORK_ITEM).await.len()
    }

    async fn events_of(&self, kind: u32) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM events \
             WHERE community_id = $1 AND kind = $2 AND deleted_at IS NULL",
        )
        .bind(self.community().as_uuid())
        .bind(kind as i32)
        .fetch_one(&self.pool)
        .await
        .expect("count events")
    }

    async fn work_done_at(&self, id: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        sqlx::query_scalar("SELECT done_at FROM io_work_items WHERE community_id = $1 AND id = $2")
            .bind(self.community().as_uuid())
            .bind(Uuid::parse_str(id).expect("item uuid"))
            .fetch_one(&self.pool)
            .await
            .expect("read done_at")
    }

    async fn backdate_done_at(&self, id: &str, days: i64) {
        sqlx::query(
            "UPDATE io_work_items SET done_at = now() - ($3 * interval '1 day') \
             WHERE community_id = $1 AND id = $2",
        )
        .bind(self.community().as_uuid())
        .bind(Uuid::parse_str(id).expect("item uuid"))
        .bind(days)
        .execute(&self.pool)
        .await
        .expect("backdate done_at");
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
/// `created_at` ticks so two identical commands in the same second (a
/// re-offer after decline) are not a NIP-01 replay.
fn signed(keys: &Keys, kind: u32, tags: Vec<Tag>, content: &str) -> Event {
    static TICK: AtomicU64 = AtomicU64::new(0);
    let created_at = Timestamp::from(
        Timestamp::now()
            .as_secs()
            .saturating_add(TICK.fetch_add(1, Ordering::Relaxed)),
    );
    EventBuilder::new(Kind::Custom(kind as u16), content)
        .tags(tags)
        .allow_self_tagging()
        .custom_created_at(created_at)
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
            "agent_membership_synced",
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
        Some("agent_membership_synced")
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
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;
    let stranger = Keys::generate();
    // Roots open under D1 (one Shaper). Seat the second Shaper after, so
    // `pass_project`'s opener-vote still passes.
    let (_, open_item) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#,
        )
        .await;
    let open_id = open_item.id.clone();
    let (_, offered_item) = h
        .pass_project(
            &h.owner,
            &format!(
                r#"{{"title":"Offered hall","brief":"Offer it","due_at":1800000000,"suggested_dri":"{member_hex}"}}"#
            ),
        )
        .await;
    let offered_id = offered_item.id.clone();
    assert_eq!(offered_item.state, WorkItemState::Offered);
    let (_, held_root) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Held hall","brief":"Hold it","due_at":1800000000}"#,
        )
        .await;
    h.offer_item(&h.owner, &held_root.id, &member_hex)
        .await
        .expect("offer held root");
    h.accept_item(&member, &held_root.id)
        .await
        .expect("accept held root");
    let held_id = held_root.id.clone();
    h.add_shaper(&h.owner, &[], &second).await;

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
    let accepted = live
        .iter()
        .find(|(_, content, _)| content["id"] == open_id)
        .expect("live 39101 for the named root");
    assert_eq!(accepted.1["state"], "accepted");
    assert_eq!(accepted.1["dri"], second_hex);
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
async fn a_passed_project_opens_a_root_in_open_or_offered() {
    let h = harness().await;
    h.bootstrap().await;
    let owner_hex = h.owner.public_key().to_hex();
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
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
    assert_eq!(
        rejected(
            h.project(
                &h.owner,
                r#"{"title":"t","brief":"b","due_at":1,"amount":"10"}"#,
                true
            )
            .await
        ),
        "invalid: money fields are not allowed"
    );
    assert_eq!(
        rejected(
            h.project(
                &h.owner,
                r#"{"title":"t","brief":"b","due_at":1,"objective_ref":"objectives@1#l_7f3a"}"#,
                true
            )
            .await
        ),
        "invalid: objective_ref not a live line"
    );

    h.direction(
        &h.owner,
        "objectives",
        0,
        r#"{"body":"the lines","lines":[{"id":"l_7f3a","text":"Weekday hall"}]}"#,
        true,
    )
    .await
    .expect("confirm objectives");

    let ledger_before = h.ledger_verbs().await.len();
    let opened = h
        .project(&member, content, false)
        .await
        .expect("member opens a project");
    assert_eq!(opened["status"], "open");
    let waiting = opened["proposal"].as_str().expect("id").to_owned();
    let p = h.proposal(&waiting).await;
    assert_eq!(p.kind, buzz_core::intelligent_org::ProposalKind::Project);
    assert_eq!(p.needed, 1);
    assert_eq!(p.eligible, vec![owner_hex.clone()]);
    assert_eq!(
        h.vote(&h.owner, &waiting, "agree", "{}")
            .await
            .expect("pass")["status"],
        "passed"
    );
    let roots: Vec<_> = h
        .work_items()
        .await
        .into_iter()
        .filter(|item| item.parent.is_none())
        .collect();
    assert_eq!(roots.len(), 1);
    let root = &roots[0];
    assert_eq!(root.state, WorkItemState::Open);
    assert_eq!(root.title, "Weekday hall");
    assert!(root.approved_at.is_some());
    assert!(root.home.is_none(), "project home is R-9a");
    assert!(root.dri.is_none());
    assert_eq!(root.path, Vec::<String>::new());
    let live = h.live_state(KIND_IO_WORK_ITEM).await;
    assert_eq!(live.len(), 1, "one live 39101 for the new root");
    assert_eq!(live[0].1["state"], "open");
    assert_eq!(
        h.ledger_verbs().await[ledger_before..],
        [
            "proposal_opened",
            "vote_cast",
            "proposal_passed",
            "item_created"
        ]
    );

    let (_, offered) = h
        .pass_project(
            &h.owner,
            &format!(
                r#"{{"title":"Offered hall","brief":"Offer it","due_at":1800000000,"suggested_dri":"{member_hex}","objective_ref":"objectives@1#l_7f3a"}}"#
            ),
        )
        .await;
    assert_eq!(offered.state, WorkItemState::Offered);
    assert_eq!(offered.offered_to.as_deref(), Some(member_hex.as_str()));
    assert_eq!(offered.offered_by.as_deref(), Some(owner_hex.as_str()));
    assert_eq!(
        offered.objective_ref.as_deref(),
        Some("objectives@1#l_7f3a")
    );
    assert!(offered.home.is_none());
    assert_eq!(h.live_state(KIND_IO_WORK_ITEM).await.len(), 2);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn only_the_holder_creates_children_and_after_must_be_a_sibling() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;
    let (_, root) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#,
        )
        .await;
    assert_eq!(
        rejected(
            h.ticket(
                &h.owner,
                &root.id,
                None,
                r#"{"title":"Permit","brief":"Get it","due_at":1800000001}"#,
            )
            .await
        ),
        "restricted: not the holder",
        "an open root has no holder"
    );

    h.offer_item(&h.owner, &root.id, &member_hex)
        .await
        .expect("offer root");
    h.accept_item(&member, &root.id)
        .await
        .expect("member holds the root");

    assert_eq!(
        rejected(
            h.ticket(
                &h.owner,
                &root.id,
                None,
                r#"{"title":"Permit","brief":"Get it","due_at":1800000001}"#,
            )
            .await
        ),
        "restricted: not the holder"
    );
    assert_eq!(
        rejected(
            h.ticket(
                &member,
                &root.id,
                None,
                r#"{"title":"x","brief":"y","due_at":1,"amount":"1"}"#,
            )
            .await
        ),
        "invalid: money fields are not allowed"
    );
    assert_eq!(
        rejected(
            h.ticket(
                &member,
                &root.id,
                None,
                r#"{"title":"Permit","brief":"Get it","due_at":1800000001,"after":["00000000-0000-4000-8000-000000000001"]}"#,
            )
            .await
        ),
        "invalid: after not a sibling"
    );

    let live_before = h.live_state(KIND_IO_WORK_ITEM).await.len();
    let ledger_before = h.ledger_verbs().await.len();
    let created = h
        .ticket(
            &member,
            &root.id,
            None,
            r#"{"title":"Permit","brief":"Get it","due_at":1800000001}"#,
        )
        .await
        .expect("holder creates");
    let permit = created["item"].as_str().expect("item").to_owned();
    let child = h.work_item(&permit).await.expect("child");
    assert_eq!(child.state, WorkItemState::Open);
    assert_eq!(child.parent.as_deref(), Some(root.id.as_str()));
    assert_eq!(child.root, root.id);
    assert_eq!(child.depth, 1);
    assert_eq!(child.path, vec![root.id.clone()]);
    assert!(child.branch.as_deref().unwrap_or("").starts_with("io/"));
    assert!(child.after.is_empty());
    let parent = h.work_item(&root.id).await.expect("parent");
    assert_eq!(parent.children.open, 1);
    assert_eq!(parent.children.offered, 0);
    assert_eq!(h.live_state(KIND_IO_WORK_ITEM).await.len(), live_before + 1);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_created"]);

    let blocked = h
        .ticket(
            &member,
            &root.id,
            Some(&h.owner.public_key().to_hex()),
            &format!(
                r#"{{"title":"Build","brief":"After the permit","due_at":1800000002,"after":["{permit}"]}}"#
            ),
        )
        .await
        .expect("after a live sibling");
    let build = blocked["item"].as_str().expect("item").to_owned();
    let child = h.work_item(&build).await.expect("build");
    assert_eq!(child.state, WorkItemState::Offered);
    assert_eq!(child.after, vec![permit.clone()]);
    let parent = h.work_item(&root.id).await.expect("parent");
    assert_eq!(parent.children.open, 1);
    assert_eq!(parent.children.offered, 1);

    // after is order, not a lock: accept succeeds while the earlier sibling is open.
    h.accept_item(&h.owner, &build)
        .await
        .expect("accept with open after");
    let child = h.work_item(&build).await.expect("accepted build");
    assert_eq!(child.state, WorkItemState::Accepted);
    assert_eq!(
        child.dri.as_deref(),
        Some(h.owner.public_key().to_hex().as_str())
    );
    let parent = h.work_item(&root.id).await.expect("parent");
    assert_eq!(parent.children.open, 1);
    assert_eq!(parent.children.offered, 0);
    assert_eq!(parent.children.accepted, 1);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn only_offered_to_accepts_or_declines_and_later_work_kinds_stay_refused() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;
    let other = Keys::generate();
    h.member(&other).await;
    let (_, root) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#,
        )
        .await;

    assert_eq!(
        rejected(h.offer_item(&member, &root.id, &member_hex).await),
        "restricted: not a Shaper"
    );
    let live_before = h.live_state(KIND_IO_WORK_ITEM).await.len();
    let ledger_before = h.ledger_verbs().await.len();
    h.offer_item(&h.owner, &root.id, &member_hex)
        .await
        .expect("shaper offers a root");
    let offered = h.work_item(&root.id).await.expect("offered");
    assert_eq!(offered.state, WorkItemState::Offered);
    assert_eq!(offered.offered_to.as_deref(), Some(member_hex.as_str()));
    assert_eq!(h.live_state(KIND_IO_WORK_ITEM).await.len(), live_before);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_offered"]);

    assert_eq!(
        rejected(h.accept_item(&other, &root.id).await),
        "restricted: not the offered person"
    );
    assert_eq!(
        rejected(h.decline_item(&other, &root.id).await),
        "restricted: not the offered person"
    );

    let ledger_before = h.ledger_verbs().await.len();
    h.decline_item(&member, &root.id)
        .await
        .expect("offered_to declines");
    let open = h.work_item(&root.id).await.expect("open again");
    assert_eq!(open.state, WorkItemState::Open);
    assert!(open.offered_to.is_none());
    assert!(open.dri.is_none());
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_declined"]);

    h.offer_item(&h.owner, &root.id, &member_hex)
        .await
        .expect("offer again");
    let ledger_before = h.ledger_verbs().await.len();
    h.accept_item(&member, &root.id)
        .await
        .expect("offered_to accepts");
    let held = h.work_item(&root.id).await.expect("accepted");
    assert_eq!(held.state, WorkItemState::Accepted);
    assert_eq!(held.dri.as_deref(), Some(member_hex.as_str()));
    assert!(held.offered_to.is_none());
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_accepted"]);
    assert_eq!(h.live_state(KIND_IO_WORK_ITEM).await.len(), 1);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn only_the_holder_marks_done_and_open_children_are_refused() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;
    let other = Keys::generate();
    h.member(&other).await;
    let (_, root) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#,
        )
        .await;
    h.offer_item(&h.owner, &root.id, &member_hex)
        .await
        .expect("offer root");
    h.accept_item(&member, &root.id)
        .await
        .expect("member holds the root");

    assert_eq!(
        rejected(h.done_item(&other, &root.id, "{}").await),
        "restricted: not the holder"
    );
    assert_eq!(
        rejected(h.done_item(&member, &root.id, r#"{"amount":"1"}"#).await),
        "invalid: money fields are not allowed"
    );

    let created = h
        .ticket(
            &member,
            &root.id,
            Some(&member_hex),
            r#"{"title":"Permit","brief":"Get it","due_at":1800000001}"#,
        )
        .await
        .expect("child");
    let permit = created["item"].as_str().expect("item").to_owned();
    assert_eq!(
        rejected(h.done_item(&member, &root.id, "{}").await),
        "invalid: open children"
    );
    h.accept_item(&member, &permit)
        .await
        .expect("member holds the child");
    assert_eq!(
        rejected(h.done_item(&member, &root.id, "{}").await),
        "invalid: open children"
    );

    let live_before = h.live_work_count().await;
    let ledger_before = h.ledger_verbs().await.len();
    let commands_before = h.events_of(KIND_IO_DONE).await;
    h.done_item(&member, &permit, "{}")
        .await
        .expect("holder marks the child done");
    let child = h.work_item(&permit).await.expect("done child");
    assert_eq!(child.state, WorkItemState::Done);
    assert_eq!(
        child.closed_by,
        Some(buzz_core::intelligent_org::ClosedBy::Dri)
    );
    assert!(child.done_receipt.is_some());
    assert_eq!(child.dri.as_deref(), Some(member_hex.as_str()));
    assert!(h.work_done_at(&permit).await.is_some());
    let parent = h.work_item(&root.id).await.expect("parent");
    assert_eq!(parent.children.open, 0);
    assert_eq!(parent.children.done, 1);
    assert_eq!(
        h.live_work_count().await,
        live_before,
        "parent 39101 replaced"
    );
    assert_eq!(h.events_of(KIND_IO_DONE).await, commands_before + 1);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_done"]);

    let live_before = h.live_work_count().await;
    let ledger_before = h.ledger_verbs().await.len();
    h.done_item(&member, &root.id, "{}")
        .await
        .expect("last child closed, root may done");
    let root_done = h.work_item(&root.id).await.expect("done root");
    assert_eq!(root_done.state, WorkItemState::Done);
    assert_eq!(h.live_work_count().await, live_before);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_done"]);

    assert_eq!(
        rejected(h.reopen_item(&other, &root.id, "{}").await),
        "restricted: not the holder"
    );
    h.backdate_done_at(&root.id, 8).await;
    assert_eq!(
        rejected(h.reopen_item(&member, &root.id, "{}").await),
        "invalid: the reopen window has closed"
    );
    h.backdate_done_at(&root.id, 1).await;
    let live_before = h.live_work_count().await;
    let ledger_before = h.ledger_verbs().await.len();
    h.reopen_item(&member, &root.id, r#"{"why":"too soon"}"#)
        .await
        .expect("reopen within 7 days");
    let reopened = h.work_item(&root.id).await.expect("reopened");
    assert_eq!(reopened.state, WorkItemState::Accepted);
    assert_eq!(reopened.dri.as_deref(), Some(member_hex.as_str()));
    assert!(reopened.closed_by.is_none());
    assert!(reopened.done_receipt.is_none());
    assert!(h.work_done_at(&root.id).await.is_none());
    assert_eq!(h.live_work_count().await, live_before);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_reopened"]);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn release_returns_children_and_set_due_follows_authority() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    let member_hex = member.public_key().to_hex();
    h.member(&member).await;
    let other = Keys::generate();
    let other_hex = other.public_key().to_hex();
    h.member(&other).await;
    let (_, root) = h
        .pass_project(
            &h.owner,
            r#"{"title":"Weekday hall","brief":"Book it","due_at":1800000000}"#,
        )
        .await;
    h.offer_item(&h.owner, &root.id, &member_hex)
        .await
        .expect("offer root");
    h.accept_item(&member, &root.id)
        .await
        .expect("member holds the root");

    let mid = h
        .ticket(
            &member,
            &root.id,
            Some(&member_hex),
            r#"{"title":"Covers","brief":"Split it","due_at":1800000001}"#,
        )
        .await
        .expect("mid")["item"]
        .as_str()
        .expect("item")
        .to_owned();
    h.accept_item(&member, &mid)
        .await
        .expect("member holds the mid ticket");
    let open_child = h
        .ticket(
            &member,
            &mid,
            None,
            r#"{"title":"Print","brief":"Open piece","due_at":1800000002}"#,
        )
        .await
        .expect("open child")["item"]
        .as_str()
        .expect("item")
        .to_owned();
    let held_child = h
        .ticket(
            &member,
            &mid,
            Some(&other_hex),
            r#"{"title":"Rota","brief":"Held piece","due_at":1800000003}"#,
        )
        .await
        .expect("held child")["item"]
        .as_str()
        .expect("item")
        .to_owned();
    h.accept_item(&other, &held_child)
        .await
        .expect("other holds the rota");

    let parent = h.work_item(&mid).await.expect("mid before release");
    assert_eq!(parent.children.open, 1);
    assert_eq!(parent.children.accepted, 1);

    assert_eq!(
        rejected(h.release_item(&other, &mid, "{}").await),
        "restricted: not the holder"
    );
    assert_eq!(
        rejected(h.release_item(&member, &mid, r#"{"budget":"1"}"#).await),
        "invalid: money fields are not allowed"
    );

    let live_before = h.live_work_count().await;
    let ledger_before = h.ledger_verbs().await.len();
    let commands_before = h.events_of(KIND_IO_RELEASE).await;
    h.release_item(&member, &mid, r#"{"why":"handing back"}"#)
        .await
        .expect("holder releases");
    let released = h.work_item(&mid).await.expect("released mid");
    assert_eq!(released.state, WorkItemState::Open);
    assert!(released.dri.is_none());
    assert_eq!(released.children.open, 0);
    assert_eq!(released.children.offered, 2);
    assert_eq!(released.children.accepted, 0);
    let print = h.work_item(&open_child).await.expect("print");
    assert_eq!(print.state, WorkItemState::Offered);
    assert_eq!(print.offered_to.as_deref(), Some(member_hex.as_str()));
    assert!(print.dri.is_none());
    let rota = h.work_item(&held_child).await.expect("rota");
    assert_eq!(rota.state, WorkItemState::Offered);
    assert_eq!(rota.offered_to.as_deref(), Some(member_hex.as_str()));
    assert!(rota.dri.is_none());
    let root_after = h.work_item(&root.id).await.expect("root counters");
    assert_eq!(root_after.children.accepted, 0);
    assert_eq!(root_after.children.open, 1);
    assert_eq!(
        h.live_work_count().await,
        live_before,
        "parent 39101 replaced"
    );
    assert_eq!(h.events_of(KIND_IO_RELEASE).await, commands_before + 1);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["item_released"]);

    assert_eq!(
        rejected(h.set_due_item(&other, &root.id, 1_900_000_000, "{}").await),
        "restricted: not a Shaper"
    );
    assert_eq!(
        rejected(h.set_due_item(&other, &mid, 1_900_000_000, "{}").await),
        "restricted: not the holder"
    );
    let live_before = h.live_work_count().await;
    let ledger_before = h.ledger_verbs().await.len();
    h.set_due_item(
        &h.owner,
        &root.id,
        1_900_000_000,
        r#"{"why":"keep it open"}"#,
    )
    .await
    .expect("shaper sets due on a root");
    let root_due = h.work_item(&root.id).await.expect("root due");
    assert_eq!(root_due.due_at, 1_900_000_000);
    assert_eq!(root_due.state, WorkItemState::Accepted);
    assert_eq!(h.live_work_count().await, live_before);
    assert_eq!(h.ledger_verbs().await[ledger_before..], ["due_changed"]);

    h.set_due_item(&member, &mid, 1_800_000_500, "{}")
        .await
        .expect("parent holder sets due on a child");
    assert_eq!(
        h.work_item(&mid).await.expect("mid due").due_at,
        1_800_000_500
    );
    assert_eq!(
        rejected(
            h.set_due_item(&other, &mid, 1, r#"{"currency":"EUR"}"#)
                .await
        ),
        "invalid: money fields are not allowed"
    );
}

// ── R-8: the agent everywhere ────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Postgres"]
async fn two_members_opening_our_dm_share_one_channel_that_hides_the_agent() {
    let h = harness().await;
    h.bootstrap().await;
    let alice = Keys::generate();
    let bob = Keys::generate();
    h.member(&alice).await;
    h.member(&bob).await;
    let alice_hex = alice.public_key().to_hex();
    let bob_hex = bob.public_key().to_hex();
    let agent_hex = h.agent.public_key().to_hex();

    let first = h.open_dm(&alice, &[&bob]).await;
    let second = h.open_dm(&bob, &[&alice]).await;
    assert_eq!(first, second, "two humans opening our DM find one channel");

    let mut humans = vec![alice_hex.clone(), bob_hex.clone()];
    humans.sort();
    let mut members = vec![alice_hex.clone(), bob_hex.clone(), agent_hex.clone()];
    members.sort();
    assert_eq!(
        h.member_pubkeys(first).await,
        members,
        "channel_members holds the two humans and the agent"
    );
    assert_eq!(
        h.discovery_p_tags(KIND_NIP29_GROUP_METADATA, first).await,
        humans,
        "39000 lists two p"
    );
    assert_eq!(
        h.discovery_p_tags(KIND_NIP29_GROUP_MEMBERS, first).await,
        humans,
        "39002 lists two p"
    );
    assert_eq!(
        h.dm_system_participants(first).await,
        humans,
        "the 41010 system message names two participants"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_nine_human_dm_holds_the_agent_as_a_tenth_row_and_41011_keeps_identity() {
    let h = harness().await;
    h.bootstrap().await;
    let mut people: Vec<Keys> = (0..9).map(|_| Keys::generate()).collect();
    for keys in &people {
        h.member(keys).await;
    }
    let opener = people.remove(0);
    let others: Vec<&Keys> = people.iter().collect();
    let dm = h.open_dm(&opener, &others).await;
    assert_eq!(
        h.member_pubkeys(dm).await.len(),
        10,
        "nine humans plus the agent"
    );
    let tenth = Keys::generate();
    h.member(&tenth).await;
    assert_eq!(
        rejected(
            h.send(
                &opener,
                KIND_DM_ADD_MEMBER,
                vec![
                    tag(["h", &dm.to_string()]),
                    tag(["p", &tenth.public_key().to_hex()]),
                ],
                "",
            )
            .await
        ),
        "invalid: DM supports at most 9 participants"
    );

    let extra = Keys::generate();
    h.member(&extra).await;
    // 41011 on a 2-human DM (fresh) then a 41010 by the humans finds that channel.
    let carol = Keys::generate();
    h.member(&carol).await;
    let pair = h.open_dm(&opener, &[&carol]).await;
    h.send(
        &opener,
        KIND_DM_ADD_MEMBER,
        vec![
            tag(["h", &pair.to_string()]),
            tag(["p", &extra.public_key().to_hex()]),
        ],
        "",
    )
    .await
    .expect("41011 accepted");
    let again = h.open_dm(&opener, &[&carol, &extra]).await;
    let via_carol = h.open_dm(&carol, &[&opener, &extra]).await;
    assert_eq!(again, via_carol);
    assert_ne!(again, pair, "41011 created a new identity set");
    assert_eq!(h.member_pubkeys(again).await.len(), 4, "3 humans + agent");
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn a_member_only_41010_opens_the_agent_dm_and_a_channel_lists_the_agent() {
    let h = harness().await;
    h.bootstrap().await;
    let member = Keys::generate();
    h.member(&member).await;
    let member_hex = member.public_key().to_hex();
    let agent_hex = h.agent.public_key().to_hex();

    let tagged_self = h.open_dm(&member, &[&member]).await;
    let tagged_agent = h.open_dm(&member, &[&h.agent]).await;
    assert_eq!(
        tagged_self, tagged_agent,
        "[member] and [member, agent] are the same DM"
    );
    assert_eq!(h.member_pubkeys(tagged_self).await, {
        let mut m = vec![member_hex.clone(), agent_hex.clone()];
        m.sort();
        m
    });
    assert_eq!(
        h.discovery_p_tags(KIND_NIP29_GROUP_METADATA, tagged_self)
            .await,
        vec![member_hex.clone()],
        "the agent DM's 39000 is {{member}}"
    );

    let room = h.create_stream(&member).await;
    let members = h.member_pubkeys(room).await;
    assert!(
        members.contains(&agent_hex),
        "a member-created channel has the agent in channel_members"
    );
    let p = h.discovery_p_tags(KIND_NIP29_GROUP_MEMBERS, room).await;
    assert!(
        p.contains(&agent_hex),
        "a channel created by a member has the agent in 39002"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn bootstrap_backfills_existing_rooms_and_shapers_agent_moves_every_row() {
    let h = harness().await;
    let alice = Keys::generate();
    h.member(&alice).await;
    let before_room = h.create_stream(&h.owner).await;
    let before_dm = h.open_dm(&h.owner, &[&alice]).await;
    assert!(
        !h.member_pubkeys(before_room)
            .await
            .contains(&h.agent.public_key().to_hex()),
        "no 39103 yet: the agent is not in the room"
    );

    let reply = h.bootstrap().await;
    let agent_hex = h.agent.public_key().to_hex();
    assert!(
        h.member_pubkeys(before_room).await.contains(&agent_hex),
        "bootstrap backfills existing channels"
    );
    assert!(
        h.member_pubkeys(before_dm).await.contains(&agent_hex),
        "bootstrap backfills existing DMs"
    );
    let verbs = h.ledger_verbs().await;
    assert!(
        verbs.contains(&"agent_membership_synced".to_owned()),
        "bootstrap writes the backfill ledger row"
    );

    let new_agent = Keys::generate();
    let new_hex = new_agent.public_key().to_hex();
    h.member(&new_agent).await;
    h.propose(
        &h.owner,
        "agent",
        Some(&new_hex),
        r#"{"why":"our own"}"#,
        true,
    )
    .await
    .expect("one Shaper: opener agree passes agent");

    for channel in [reply.room, before_room, before_dm] {
        let members = h.member_pubkeys(channel).await;
        assert!(members.contains(&new_hex), "the new key is in every room");
        assert!(!members.contains(&agent_hex), "the old key is in no room");
    }
}
