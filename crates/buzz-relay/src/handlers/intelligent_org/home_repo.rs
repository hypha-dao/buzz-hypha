//! Project home repository — Protocol §6.7 (H-1).
//!
//! On a passed project, the same transaction that creates the room also
//! signs `30617` and `30621`, reserves the git name, and writes
//! `home.repo` / `home.project`. Context files are seeded after that
//! transaction commits. A failed seed leaves the `context_seed` ledger row;
//! `context_seeded` is written only after the three files are on `main`.
//! A relay with no object-storage endpoint keeps the room and skips the repo.

use std::path::Path;
use std::time::Duration;

use buzz_core::intelligent_org::ProjectHome;
use buzz_core::kind::{KIND_GIT_REPO_ANNOUNCEMENT, KIND_PROJECT};
use buzz_core::tenant::TenantContext;
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{self as store, LedgerEntry, LEDGER_ACTOR_RELAY};
use buzz_db::replaceable::{ParameterizedReplacePrecondition, ParameterizedReplaceStatus};
use buzz_db::Db;
use buzz_sdk::{build_project, build_repo_announcement_with_tags, ProjectMemberCoord};
use nostr::{Keys, Tag, Timestamp};
use sqlx::{Postgres, Row, Transaction};
use tracing::warn;
use uuid::Uuid;

use super::home::home_channel_only;
use super::{internal, object, Command};
use crate::api::git::cas_publish::{cas_publish, PublishLimits};
use crate::api::git::hydrate::{hydrate_for_read, hydrate_for_write, HydrationOptions};
use crate::handlers::ingest::IngestError;
use crate::state::AppState;

const CONTEXT_README: &str = "context/README.md";
const CONTEXT_DECISIONS: &str = "context/decisions.md";
const CONTEXT_LINKS: &str = "context/links.md";
const MAX_CONTEXT_BYTES: usize = 8 * 1024;
const SEED_TIMEOUT: Duration = Duration::from_secs(20);
const STDERR_CAP: usize = 2_048;
const PENDING_LIMIT: i64 = 8;

/// What the pass transaction asks the home announcer to create.
pub(super) struct AnnounceRequest<'a> {
    /// Project title. Becomes the repo name and the README heading.
    pub title: &'a str,
    /// Project brief. Becomes the repo description and the README lead.
    pub brief: &'a str,
    /// The room created in this same transaction.
    pub room: Uuid,
    /// The new root's id. A slug collision borrows its first eight hex digits.
    pub item_id: Uuid,
    /// Root holder, when the pass already assigned one.
    pub dri: Option<&'a str>,
    /// Raw `50004` content, including `change` and `plan` when the client sent them.
    pub payload: &'a serde_json::Value,
}

/// Relay identity the announcer signs and stores with.
pub(super) struct AnnounceCtx<'a> {
    /// Community the project belongs to.
    pub community: CommunityId,
    /// Host used in the clone URL.
    pub host: &'a str,
    /// Relay key. It signs `30617` and `30621`.
    pub relay: &'a Keys,
    /// Store for the addressable events.
    pub db: &'a Db,
    /// `created_at` for both announcements.
    pub created_at: u64,
}

/// The home coordinates, plus the files to commit after the pass commits.
pub(super) struct AnnouncedHome {
    /// Written onto `39101.home`.
    pub home: ProjectHome,
    /// Present when a repository was announced. Seeded after commit.
    pub seed: Option<ContextSeed>,
}

/// One attempt to put the three context files on `main`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContextSeed {
    /// Work item the seed belongs to.
    pub object_id: String,
    /// `d` tag of the `30617`.
    pub repo_id: String,
    /// Relay pubkey, hex. Owner of the announcement.
    pub owner: String,
    /// `context/README.md`.
    pub readme: String,
    /// `context/decisions.md`.
    pub decisions: String,
    /// `context/links.md`.
    pub links: String,
}

/// How a seed attempt finished without publishing a retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeedStatus {
    Published,
    AlreadyPresent,
    Superseded,
}

/// The three context files for one project. No `context/drafts/`.
pub(super) struct ContextDocs {
    /// `context/README.md`.
    pub readme: String,
    /// `context/decisions.md`.
    pub decisions: String,
    /// `context/links.md`.
    pub links: String,
}

/// Object storage is configured when the S3 endpoint is non-empty.
///
/// An empty endpoint is a sovereign relay: the room still opens, and no
/// repository is announced. A configured endpoint that later fails to accept
/// the seed is a retry, not a skip.
pub(super) fn object_storage_enabled(endpoint: &str) -> bool {
    !endpoint.trim().is_empty()
}

/// Tags on the relay-signed `30617`, including `buzz-protect` on `main`.
pub(super) fn repo_announcement_tags(
    name: &str,
    description: &str,
    clone_url: &str,
    room: &str,
    dri: Option<&str>,
) -> Vec<Vec<String>> {
    let mut tags = vec![
        vec!["name".to_owned(), name.to_owned()],
        vec!["description".to_owned(), description.to_owned()],
        vec!["clone".to_owned(), clone_url.to_owned()],
        vec!["buzz-channel".to_owned(), room.to_owned()],
        vec![
            "buzz-protect".to_owned(),
            "refs/heads/main".to_owned(),
            "push:admin".to_owned(),
            "no-force-push".to_owned(),
            "no-delete".to_owned(),
        ],
    ];
    if let Some(dri) = dri {
        tags.push(vec!["maintainers".to_owned(), dri.to_owned()]);
    }
    tags
}

/// Markdown for `context/README.md`, `context/decisions.md`, and `context/links.md`.
pub(super) fn context_documents(
    title: &str,
    brief: &str,
    payload: &serde_json::Value,
) -> ContextDocs {
    let change = payload.get("change");
    let from = json_str(change, "from");
    let to = json_str(change, "to");
    let done_when = json_strings(change, "done_when");
    let steps = plan_steps(payload.get("plan"));

    let mut readme = format!("# {title}\n\n{brief}\n\n## Change\n\n");
    if from.is_empty() && to.is_empty() && done_when.is_empty() {
        readme.push_str("No change recorded yet.\n");
    } else {
        readme.push_str(&format!("From: {from}\n\nTo: {to}\n\nDone when:\n\n"));
        for item in &done_when {
            readme.push_str(&format!("- {item}\n"));
        }
    }
    readme.push_str("\n## Steps\n\n");
    if steps.is_empty() {
        readme.push_str("No steps recorded yet.\n");
    } else {
        for (piece, held) in &steps {
            if *held {
                readme.push_str(&format!("- {piece} — held\n"));
            } else {
                readme.push_str(&format!("- {piece}\n"));
            }
        }
    }

    let decisions = if from.is_empty() && to.is_empty() {
        "# Decisions\n\nNo decisions yet.\n".to_owned()
    } else {
        format!("# Decisions\n\n- From {from} to {to}.\n")
    };
    let links = "# Links\n\nNo links yet.\n".to_owned();
    ContextDocs {
        readme: cap_text(readme),
        decisions: cap_text(decisions),
        links: cap_text(links),
    }
}

/// Announce the repository and project inside the caller's transaction.
///
/// `storage_enabled` false returns the room-only home and writes nothing
/// about a repository. The git name reservation and both events roll back
/// with the caller.
pub(super) async fn announce_home(
    ctx: &AnnounceCtx<'_>,
    tx: &mut Transaction<'static, Postgres>,
    req: &AnnounceRequest<'_>,
    storage_enabled: bool,
) -> Result<AnnouncedHome, IngestError> {
    if !storage_enabled {
        return Ok(AnnouncedHome {
            home: home_channel_only(req.room),
            seed: None,
        });
    }

    let owner = ctx.relay.public_key().to_hex();
    let slug = claim_repo_id(tx, ctx.community, req.title, req.item_id, &owner).await?;
    let name = truncate_bytes(req.title, 128);
    let description = truncate_bytes(req.brief, 1024);
    let clone_url = format!(
        "http://{}/git/{owner}/{slug}",
        ctx.host.trim().trim_end_matches('/')
    );
    let room = req.room.to_string();
    let docs = context_documents(req.title, req.brief, req.payload);
    let tag_rows = repo_announcement_tags(&name, &description, &clone_url, &room, req.dri);
    let repo_event = sign_announcement(ctx, &slug, &tag_rows, "")?;
    store_addressable(ctx, tx, &repo_event, &slug).await?;

    let repo_coord = format!("{KIND_GIT_REPO_ANNOUNCEMENT}:{owner}:{slug}");
    let member = ProjectMemberCoord::parse_full(&repo_coord)
        .map_err(|e| internal("project member coordinate", e))?;
    let project_builder = build_project(
        &slug,
        Some(name.as_str()),
        None,
        &[member],
        Some(room.as_str()),
        Some("listed"),
    )
    .map_err(|e| internal("build project announcement", e))?;
    let project_event = project_builder
        .allow_self_tagging()
        .custom_created_at(Timestamp::from(ctx.created_at))
        .sign_with_keys(ctx.relay)
        .map_err(|e| internal("sign project announcement", e))?;
    store_addressable(ctx, tx, &project_event, &slug).await?;

    let project_coord = format!("{KIND_PROJECT}:{owner}:{slug}");
    Ok(AnnouncedHome {
        home: ProjectHome {
            channel: room,
            repo: Some(repo_coord),
            project: Some(project_coord),
        },
        seed: Some(ContextSeed {
            object_id: req.item_id.to_string(),
            repo_id: slug,
            owner,
            readme: docs.readme,
            decisions: docs.decisions,
            links: docs.links,
        }),
    })
}

/// Ledger detail for a `context_seed` row. The item id is the ledger object.
pub(super) fn seed_detail(seed: &ContextSeed) -> serde_json::Value {
    serde_json::json!({
        "repo_id": seed.repo_id,
        "owner": seed.owner,
        "readme": seed.readme,
        "decisions": seed.decisions,
        "links": seed.links,
    })
}

/// The `context_seed` row carried on a just-committed pass, if this entry is one.
pub(super) fn seed_from_entry(entry: &LedgerEntry) -> Option<ContextSeed> {
    if entry.verb != "context_seed" {
        return None;
    }
    Some(ContextSeed {
        object_id: entry.object_id.clone(),
        repo_id: entry.detail.get("repo_id")?.as_str()?.to_owned(),
        owner: entry.detail.get("owner")?.as_str()?.to_owned(),
        readme: entry.detail.get("readme")?.as_str()?.to_owned(),
        decisions: entry.detail.get("decisions")?.as_str()?.to_owned(),
        links: entry.detail.get("links")?.as_str()?.to_owned(),
    })
}

/// Seed every `context_seed` row this command just committed.
///
/// A failure is logged and the row stays pending. It does not fail the pass.
pub(super) async fn seed_committed(cmd: &Command<'_>, rows: &[LedgerEntry]) {
    if !object_storage_enabled(&cmd.state.config.media.s3_endpoint) {
        return;
    }
    for row in rows {
        let Some(seed) = seed_from_entry(row) else {
            continue;
        };
        let id = match latest_seed_id(cmd.state, cmd.tenant, &seed.object_id).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                warn!(item = %seed.object_id, "context seed row missing after commit");
                continue;
            }
            Err(e) => {
                warn!(error = %e, item = %seed.object_id, "context seed id lookup failed");
                continue;
            }
        };
        seed_and_mark(cmd.state, cmd.tenant, id, &seed).await;
    }
}

/// Retry pending context seeds for one community. One seed failure does not
/// fail the sweep; a ledger read failure is returned to the caller.
pub(super) async fn retry_pending(state: &AppState, tenant: &TenantContext) -> Result<(), String> {
    if !object_storage_enabled(&state.config.media.s3_endpoint) {
        return Ok(());
    }
    let pending = pending_seeds(state, tenant).await?;
    for (id, seed) in pending {
        seed_and_mark(state, tenant, id, &seed).await;
    }
    Ok(())
}

/// Read one file from the seeded repository. `Ok(None)` when the repo or the
/// path is absent. Test and compiler callers use this rather than a private helper.
pub(super) async fn read_seeded_file(
    state: &AppState,
    tenant: &TenantContext,
    owner: &str,
    repo_id: &str,
    path: &str,
) -> Result<Option<String>, String> {
    let Some(hydrated) = hydrate_read(state, tenant, owner, repo_id).await? else {
        return Ok(None);
    };
    let spec = format!("HEAD:{path}");
    let bytes = cat_file(hydrated.path(), &spec).await?;
    Ok(bytes.map(|b| String::from_utf8_lossy(&b).into_owned()))
}

/// Every path at `HEAD` of the seeded repository.
#[cfg(test)]
pub(super) async fn list_seeded_paths(
    state: &AppState,
    tenant: &TenantContext,
    owner: &str,
    repo_id: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(hydrated) = hydrate_read(state, tenant, owner, repo_id).await? else {
        return Ok(None);
    };
    let git_dir = hydrated
        .path()
        .to_str()
        .ok_or_else(|| "hydrated repo path is not utf-8".to_owned())?;
    let stdout = git_dir_args(git_dir, &["ls-tree", "-r", "--name-only", "HEAD"]).await?;
    let text = String::from_utf8_lossy(&stdout);
    Ok(Some(
        text.lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect(),
    ))
}

async fn seed_and_mark(state: &AppState, tenant: &TenantContext, seed_id: i64, seed: &ContextSeed) {
    match seed_context(state, tenant, seed_id, seed).await {
        Ok(_) => {
            if let Err(e) = mark_seeded(state, tenant, seed_id).await {
                warn!(error = %e, seed_id, "context seed succeeded but the retry row could not be closed");
            }
        }
        Err(e) => {
            warn!(
                error = %e,
                seed_id,
                item = %seed.object_id,
                "context seed failed; the project stands and the row stays pending"
            );
        }
    }
}

async fn seed_context(
    state: &AppState,
    tenant: &TenantContext,
    seed_id: i64,
    seed: &ContextSeed,
) -> Result<SeedStatus, String> {
    if seed.repo_id.is_empty() || seed.owner.len() != 64 {
        return Err("context seed is missing a repository coordinate".into());
    }
    if newest_is_newer(state, tenant, seed, seed_id).await? {
        return Ok(SeedStatus::Superseded);
    }
    if files_match(state, tenant, seed).await? {
        return Ok(SeedStatus::AlreadyPresent);
    }
    if newest_is_newer(state, tenant, seed, seed_id).await? {
        return Ok(SeedStatus::Superseded);
    }
    publish_context(state, tenant, seed).await?;
    Ok(SeedStatus::Published)
}

async fn publish_context(
    state: &AppState,
    tenant: &TenantContext,
    seed: &ContextSeed,
) -> Result<(), String> {
    let options = hydration_options(state)?;
    let (hydrated, parent) = hydrate_for_write(
        &state.git_store,
        tenant,
        &seed.owner,
        &seed.repo_id,
        options,
    )
    .await
    .map_err(|e| format!("hydrate home repo for write: {e}"))?;
    let work = tempfile::TempDir::new_in(options.scratch_dir)
        .map_err(|e| format!("context worktree: {e}"))?;
    write_context_tree(work.path(), seed)?;
    let publish = async {
        git(work.path(), &["init", "-b", "main"]).await?;
        git(
            work.path(),
            &["add", CONTEXT_README, CONTEXT_DECISIONS, CONTEXT_LINKS],
        )
        .await?;
        git_commit(work.path()).await?;
        let bare = hydrated
            .path()
            .to_str()
            .ok_or_else(|| "hydrated repo path is not utf-8".to_owned())?;
        git(work.path(), &["push", bare, "HEAD:refs/heads/main"]).await?;
        let limits = PublishLimits {
            parent_hydrated_bytes: hydrated.hydrated_bytes(),
            max_pack_bytes: state.config.git_max_pack_bytes,
            max_repo_bytes: state.config.git_max_repo_bytes,
        };
        cas_publish(
            &state.git_store,
            tenant,
            hydrated.path(),
            &seed.owner,
            &seed.repo_id,
            &parent,
            limits,
        )
        .await
        .map_err(|e| format!("publish home context: {e}"))?;
        Ok(())
    };
    tokio::time::timeout(SEED_TIMEOUT, publish)
        .await
        .map_err(|_| "context seed timed out".to_owned())?
}

fn write_context_tree(root: &Path, seed: &ContextSeed) -> Result<(), String> {
    let dir = root.join("context");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir context: {e}"))?;
    std::fs::write(dir.join("README.md"), seed.readme.as_bytes())
        .map_err(|e| format!("write README: {e}"))?;
    std::fs::write(dir.join("decisions.md"), seed.decisions.as_bytes())
        .map_err(|e| format!("write decisions: {e}"))?;
    std::fs::write(dir.join("links.md"), seed.links.as_bytes())
        .map_err(|e| format!("write links: {e}"))?;
    Ok(())
}

async fn files_match(
    state: &AppState,
    tenant: &TenantContext,
    seed: &ContextSeed,
) -> Result<bool, String> {
    let readme =
        read_seeded_file(state, tenant, &seed.owner, &seed.repo_id, CONTEXT_README).await?;
    let decisions =
        read_seeded_file(state, tenant, &seed.owner, &seed.repo_id, CONTEXT_DECISIONS).await?;
    let links = read_seeded_file(state, tenant, &seed.owner, &seed.repo_id, CONTEXT_LINKS).await?;
    Ok(readme.as_deref() == Some(seed.readme.as_str())
        && decisions.as_deref() == Some(seed.decisions.as_str())
        && links.as_deref() == Some(seed.links.as_str()))
}

async fn hydrate_read(
    state: &AppState,
    tenant: &TenantContext,
    owner: &str,
    repo_id: &str,
) -> Result<Option<crate::api::git::hydrate::HydratedRepo>, String> {
    let options = hydration_options(state)?;
    hydrate_for_read(&state.git_store, tenant, owner, repo_id, options)
        .await
        .map_err(|e| format!("hydrate home repo for read: {e}"))
}

fn hydration_options(state: &AppState) -> Result<HydrationOptions<'_>, String> {
    std::fs::create_dir_all(&state.config.git_repo_path)
        .map_err(|e| format!("git scratch dir: {e}"))?;
    Ok(HydrationOptions {
        pack_cache: &state.git_pack_cache,
        scratch_dir: state.config.git_repo_path.as_path(),
        max_pack_bytes: state.config.git_max_pack_bytes,
        max_repo_bytes: state.config.git_max_repo_bytes,
    })
}

async fn claim_repo_id(
    tx: &mut Transaction<'static, Postgres>,
    community: CommunityId,
    title: &str,
    item_id: Uuid,
    owner: &str,
) -> Result<String, IngestError> {
    let base = truncate_bytes(&super::home::room_slug(title), 64);
    let id8 = item_id.simple().to_string();
    let id8 = &id8[..8];
    let suffixed = format!("{}-{id8}", truncate_bytes(&base, 55));
    for slug in [base, suffixed] {
        let inserted = sqlx::query(
            "INSERT INTO git_repo_names (community_id, repo_id, owner_pubkey) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (community_id, repo_id) DO NOTHING \
             RETURNING repo_id",
        )
        .bind(community.as_uuid())
        .bind(&slug)
        .bind(owner)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| internal("reserve project repo name", e))?;
        if inserted.is_some() {
            return Ok(slug);
        }
    }
    Err(IngestError::Internal(
        "error: claim project repo name: both slug candidates are taken".into(),
    ))
}

fn sign_announcement(
    ctx: &AnnounceCtx<'_>,
    slug: &str,
    tag_rows: &[Vec<String>],
    content: &str,
) -> Result<nostr::Event, IngestError> {
    let tags = tag_rows
        .iter()
        .map(|row| {
            let parts: Vec<&str> = row.iter().map(String::as_str).collect();
            Tag::parse(parts).map_err(|e| internal("project home tag", e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let builder = build_repo_announcement_with_tags(slug, content, tags)
        .map_err(|e| internal("build repo announcement", e))?;
    builder
        .allow_self_tagging()
        .custom_created_at(Timestamp::from(ctx.created_at))
        .sign_with_keys(ctx.relay)
        .map_err(|e| internal("sign repo announcement", e))
}

async fn store_addressable(
    ctx: &AnnounceCtx<'_>,
    tx: &mut Transaction<'static, Postgres>,
    event: &nostr::Event,
    slug: &str,
) -> Result<(), IngestError> {
    let stored = ctx
        .db
        .replace_parameterized_event_in_transaction(
            tx,
            ctx.community,
            event,
            slug,
            None,
            ParameterizedReplacePrecondition::Unconditional,
        )
        .await
        .map_err(|e| internal("store project home event", e))?;
    if stored.status != ParameterizedReplaceStatus::Inserted {
        return Err(IngestError::Internal(format!(
            "error: project home event {} was not accepted as the live head ({:?})",
            event.id.to_hex(),
            stored.status
        )));
    }
    Ok(())
}

async fn pending_seeds(
    state: &AppState,
    tenant: &TenantContext,
) -> Result<Vec<(i64, ContextSeed)>, String> {
    let mut tx = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin context seed scan: {e}"))?;
    let rows = sqlx::query(
        "SELECT l.id, l.object_id, l.detail \
         FROM io_ledger l \
         WHERE l.community_id = $1 \
           AND l.verb = 'context_seed' \
           AND l.object_type = $2 \
           AND NOT EXISTS ( \
             SELECT 1 FROM io_ledger s \
             WHERE s.community_id = l.community_id \
               AND s.object_id = l.object_id \
               AND s.verb = 'context_seeded' \
               AND s.detail->>'seed_ledger_id' = l.id::text \
           ) \
         ORDER BY l.id \
         LIMIT $3",
    )
    .bind(tenant.community().as_uuid())
    .bind(object::WORK_ITEM)
    .bind(PENDING_LIMIT)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("list pending context seeds: {e}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("commit context seed scan: {e}"))?;
    let mut pending = Vec::new();
    for row in rows {
        let id: i64 = row.get("id");
        let object_id: String = row.get("object_id");
        let detail: serde_json::Value = row.get("detail");
        let entry = LedgerEntry {
            at: chrono::Utc::now(),
            actor: LEDGER_ACTOR_RELAY.to_owned(),
            verb: "context_seed".into(),
            object_type: object::WORK_ITEM.to_owned(),
            object_id,
            receipt_event_id: None,
            detail,
        };
        if let Some(seed) = seed_from_entry(&entry) {
            pending.push((id, seed));
        }
    }
    Ok(pending)
}

async fn latest_seed_id(
    state: &AppState,
    tenant: &TenantContext,
    object_id: &str,
) -> Result<Option<i64>, String> {
    let mut tx = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin context seed lookup: {e}"))?;
    let id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM io_ledger \
         WHERE community_id = $1 AND object_type = $2 AND object_id = $3 AND verb = 'context_seed' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(tenant.community().as_uuid())
    .bind(object::WORK_ITEM)
    .bind(object_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("read context seed id: {e}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("commit context seed lookup: {e}"))?;
    Ok(id)
}

async fn newest_is_newer(
    state: &AppState,
    tenant: &TenantContext,
    seed: &ContextSeed,
    seed_id: i64,
) -> Result<bool, String> {
    let Some(newest) = latest_seed_id(state, tenant, &seed.object_id).await? else {
        return Ok(false);
    };
    Ok(newest > seed_id)
}

async fn mark_seeded(state: &AppState, tenant: &TenantContext, seed_id: i64) -> Result<(), String> {
    let mut tx = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin context seeded: {e}"))?;
    let entry = LedgerEntry {
        at: chrono::Utc::now(),
        actor: LEDGER_ACTOR_RELAY.to_owned(),
        verb: "context_seeded".into(),
        object_type: object::WORK_ITEM.to_owned(),
        object_id: seed_object_id(&mut tx, tenant, seed_id).await?,
        receipt_event_id: None,
        detail: serde_json::json!({ "seed_ledger_id": seed_id }),
    };
    store::insert_ledger(&mut tx, tenant.community(), &entry)
        .await
        .map_err(|e| format!("write context_seeded: {e}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("commit context_seeded: {e}"))?;
    Ok(())
}

async fn seed_object_id(
    tx: &mut Transaction<'static, Postgres>,
    tenant: &TenantContext,
    seed_id: i64,
) -> Result<String, String> {
    sqlx::query_scalar("SELECT object_id FROM io_ledger WHERE community_id = $1 AND id = $2")
        .bind(tenant.community().as_uuid())
        .bind(seed_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| format!("read context seed object: {e}"))
}

fn json_str(value: Option<&serde_json::Value>, key: &str) -> String {
    value
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned()
}

fn json_strings(value: Option<&serde_json::Value>, key: &str) -> Vec<String> {
    value
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn plan_steps(plan: Option<&serde_json::Value>) -> Vec<(String, bool)> {
    let Some(steps) = plan.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    steps
        .iter()
        .filter_map(|step| {
            let piece = step.get("piece")?.as_str()?.to_owned();
            let held = step
                .get("held")
                .and_then(|v| v.as_str())
                .is_some_and(|text| !text.is_empty());
            Some((piece, held))
        })
        .collect()
}

fn cap_text(text: String) -> String {
    if text.len() <= MAX_CONTEXT_BYTES {
        return text;
    }
    let mut end = MAX_CONTEXT_BYTES.saturating_sub(16);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = text[..end].to_owned();
    out.push_str("\n\n[truncated]\n");
    out
}

fn truncate_bytes(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

async fn git_commit(cwd: &Path) -> Result<Vec<u8>, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.current_dir(cwd)
        .args([
            "-c",
            "user.email=relay@localhost",
            "-c",
            "user.name=Buzz relay",
            "commit",
            "-m",
            "Seed project context",
        ])
        .kill_on_drop(true)
        .env("GIT_AUTHOR_NAME", "Buzz relay")
        .env("GIT_AUTHOR_EMAIL", "relay@localhost")
        .env("GIT_COMMITTER_NAME", "Buzz relay")
        .env("GIT_COMMITTER_EMAIL", "relay@localhost");
    git_output(cmd, &["commit"]).await
}

async fn git(cwd: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.current_dir(cwd).args(args).kill_on_drop(true);
    git_output(cmd, args).await
}

#[cfg(test)]
async fn git_dir_args(git_dir: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("--git-dir")
        .arg(git_dir)
        .args(args)
        .kill_on_drop(true);
    let mut labeled = vec!["--git-dir", git_dir];
    labeled.extend_from_slice(args);
    git_output(cmd, &labeled).await
}

async fn git_output(mut cmd: tokio::process::Command, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = tokio::time::timeout(SEED_TIMEOUT, cmd.output())
        .await
        .map_err(|_| format!("git {args:?} timed out"))?
        .map_err(|e| format!("spawn git {args:?}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr: String = stderr.chars().take(STDERR_CAP).collect();
        return Err(format!("git {args:?} exited {}: {stderr}", output.status));
    }
    Ok(output.stdout)
}

async fn cat_file(git_dir: &Path, spec: &str) -> Result<Option<Vec<u8>>, String> {
    let git_dir = git_dir
        .to_str()
        .ok_or_else(|| "git dir is not utf-8".to_owned())?;
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(["--git-dir", git_dir, "cat-file", "-p", spec])
        .kill_on_drop(true);
    let output = tokio::time::timeout(SEED_TIMEOUT, cmd.output())
        .await
        .map_err(|_| format!("git cat-file {spec} timed out"))?
        .map_err(|e| format!("spawn git cat-file: {e}"))?;
    if output.status.success() {
        return Ok(Some(output.stdout));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Not a valid object name") || stderr.contains("does not exist") {
        return Ok(None);
    }
    Err(format!(
        "git cat-file {spec} exited {}: {}",
        output.status,
        stderr.chars().take(STDERR_CAP).collect::<String>()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::channel::MemberRole;
    use buzz_core::git_perms::{evaluate_ref_update, parse_protection_tags, RefUpdate, UpdateKind};

    #[test]
    fn empty_object_storage_is_a_sovereign_relay() {
        assert!(!object_storage_enabled(""));
        assert!(!object_storage_enabled("   "));
        assert!(object_storage_enabled("http://localhost:9000"));
    }

    #[test]
    fn a_child_holder_cannot_push_main() {
        let tags = repo_announcement_tags(
            "Weekday hall",
            "Book it",
            "http://localhost/git/aa/weekday-hall",
            "11111111-1111-4111-8111-111111111111",
            None,
        );
        let parsed = parse_protection_tags(&tags).expect("protection tags parse");
        assert!(
            parsed.unknown_rules.is_empty(),
            "unexpected rules: {:?}",
            parsed.unknown_rules
        );
        let update = RefUpdate {
            ref_name: "refs/heads/main".to_owned(),
            kind: UpdateKind::FastForward,
            old_oid: "a".repeat(40),
            new_oid: "b".repeat(40),
        };
        assert!(
            evaluate_ref_update(&update, MemberRole::Member, &parsed.rules).is_err(),
            "a child holder is a room member and must not push main"
        );
        assert!(
            evaluate_ref_update(&update, MemberRole::Admin, &parsed.rules).is_ok(),
            "the root holder is a room admin and may push main"
        );
    }

    #[test]
    fn context_files_carry_the_change_and_skip_drafts() {
        let payload = serde_json::json!({
            "change": {
                "from": "no hall",
                "to": "a weekday hall",
                "done_when": ["a night happened"]
            },
            "plan": [
                {"piece": "Confirm the room", "gate": true},
                {"piece": "Book the band", "held": "after the licence"}
            ]
        });
        let docs = context_documents("Weekday hall", "Book it", &payload);
        assert!(docs.readme.contains("# Weekday hall"));
        assert!(docs.readme.contains("Book it"));
        assert!(docs.readme.contains("From: no hall"));
        assert!(docs.readme.contains("To: a weekday hall"));
        assert!(docs.readme.contains("- a night happened"));
        assert!(docs.readme.contains("- Confirm the room"));
        assert!(docs.readme.contains("- Book the band — held"));
        assert!(!docs.readme.contains("context/drafts"));
        assert!(!docs.decisions.contains("context/drafts"));
        assert!(docs.decisions.contains("From no hall to a weekday hall."));
        assert!(docs.links.contains("No links yet."));
        assert!(!docs.links.contains("context/drafts"));
    }

    #[test]
    fn a_project_without_a_change_still_has_three_files() {
        let docs = context_documents("Weekday hall", "Book it", &serde_json::json!({}));
        assert!(docs.readme.contains("No change recorded yet."));
        assert!(docs.decisions.contains("No decisions yet."));
        assert!(docs.links.contains("No links yet."));
        assert!(!docs.readme.contains("context/drafts"));
    }
}
