//! Project home — the room half of Protocol §6.7.
//!
//! On a passed `project` the relay creates an open NIP-29 room named from the
//! title slug, puts `39103.agent` in it (via [`relay_rooms::create_room`]),
//! and writes `home.channel` on the root `39101`. The repository half
//! (`30617` / `30621` / context files) lives in [`super::home_repo`]. Accept,
//! release, and a passed `dri` keep the roster equal to the tree: root holder
//! → admin, child holders → member, NIP-OA-attested agents → bot; talk
//! joiners are never removed.

use buzz_core::channel::{ChannelType, ChannelVisibility, MemberRole};
use buzz_core::intelligent_org::{ProjectHome, WorkItem};
use buzz_core::CommunityId;
use buzz_db::intelligent_org::{self as store, LedgerEntry};
use buzz_db::relay_rooms::{self, DesiredMember, RosterChange};
use buzz_db::DbError;
use sqlx::PgConnection;
use uuid::Uuid;

use super::apply::{ApplyContext, AGENT_ROOM_ROLE};
use super::{internal, object};
use crate::handlers::ingest::IngestError;

/// Channel-name slug from a project title (Protocol §6.7 `name <slug>`).
pub(super) fn room_slug(title: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in title.chars() {
        let mapped = if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else if c.is_whitespace() || c == '-' || c == '_' {
            Some('-')
        } else {
            None
        };
        match mapped {
            Some('-') if prev_dash || slug.is_empty() => {}
            Some('-') => {
                slug.push('-');
                prev_dash = true;
            }
            Some(ch) => {
                slug.push(ch);
                prev_dash = false;
            }
            None => {}
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "project".into()
    } else {
        slug
    }
}

/// Create the project's open room. The org agent is put as `member` by
/// [`relay_rooms::create_room`]; holders land on accept / `dri`.
pub(super) async fn create_project_room(
    conn: &mut PgConnection,
    community: CommunityId,
    title: &str,
    created_by: &[u8],
) -> Result<Uuid, IngestError> {
    let name = room_slug(title);
    relay_rooms::create_room(
        conn,
        community,
        &name,
        ChannelType::Stream,
        ChannelVisibility::Open,
        Some(title),
        created_by,
    )
    .await
    .map_err(|e| internal("create project home room", e))
}

/// Archive a project's home room. Already archived, or a room that is gone,
/// is a no-op so a retry of the same removal does not fail.
pub(super) async fn archive_home_channel(
    conn: &mut PgConnection,
    community: CommunityId,
    channel: Uuid,
) -> Result<(), IngestError> {
    sqlx::query(
        "UPDATE channels SET archived_at = NOW() \
         WHERE community_id = $1 AND id = $2 AND deleted_at IS NULL AND archived_at IS NULL",
    )
    .bind(community.as_uuid())
    .bind(channel)
    .execute(conn)
    .await
    .map_err(|e| internal("archive project home room", e))?;
    Ok(())
}

/// `39101.home` when the relay has no object storage: the room, and no repository.
pub(super) fn home_channel_only(channel: Uuid) -> ProjectHome {
    ProjectHome {
        channel: channel.to_string(),
        repo: None,
        project: None,
    }
}

/// Make the project room's holder roles match the live tree under `root`.
///
/// Talk joiners stay. Returns roster changes for post-commit fan-out, and
/// appends `home_member_synced` ledger rows for each change.
pub(super) async fn sync_home_roster(
    conn: &mut PgConnection,
    ctx: &ApplyContext<'_>,
    root: &WorkItem,
    receipt_event_id: &[u8],
    ledger: &mut Vec<LedgerEntry>,
) -> Result<Vec<(Uuid, RosterChange)>, IngestError> {
    let Some(home) = root.home.as_ref() else {
        return Ok(vec![]);
    };
    let room = Uuid::parse_str(&home.channel).map_err(|e| internal("home.channel", e))?;
    let root_id = Uuid::parse_str(&root.id).map_err(|e| internal("root id", e))?;

    let desired = desired_home_roster(conn, ctx.community, root).await?;
    let changes = relay_rooms::upsert_home_roles(conn, ctx.community, room, &desired, ctx.actor)
        .await
        .map_err(|e| match e {
            DbError::ChannelNotFound(id) => IngestError::Internal(format!(
                "error: the project home room {id} is missing; restore it before changing holders"
            )),
            other => internal("sync project home roster", other),
        })?;

    let actor_hex = hex::encode(ctx.actor);
    let at = store::ts(ctx.now).map_err(|e| internal("ledger time", e))?;
    for change in &changes {
        let (pubkey, role, action, why) = match change {
            RosterChange::Added { pubkey, role } => {
                let why = match role {
                    MemberRole::Admin | MemberRole::Member => "holder",
                    MemberRole::Bot => "attested_agent",
                    _ => "holder",
                };
                // A lower from admin/bot to member is still an Added in the
                // upsert API; tag it release when the member is no longer a
                // desired holder/bot at that role.
                let still_desired = desired
                    .iter()
                    .any(|d| d.pubkey == *pubkey && d.role == *role);
                let why = if *role == MemberRole::Member && !still_desired {
                    "release"
                } else {
                    why
                };
                (pubkey, role.as_str(), "added", why)
            }
            RosterChange::Removed { pubkey } => (pubkey, "member", "removed", "release"),
        };
        ledger.push(LedgerEntry {
            at,
            actor: actor_hex.clone(),
            verb: "home_member_synced".into(),
            object_type: object::WORK_ITEM.to_owned(),
            object_id: root.id.clone(),
            receipt_event_id: Some(receipt_event_id.to_vec()),
            detail: serde_json::json!({
                "p": hex::encode(pubkey),
                "role": role,
                "action": action,
                "why": why,
                "channel": room,
                "root": root_id,
            }),
        });
    }

    Ok(changes.into_iter().map(|c| (room, c)).collect())
}

async fn desired_home_roster(
    conn: &mut PgConnection,
    community: CommunityId,
    root: &WorkItem,
) -> Result<Vec<DesiredMember>, IngestError> {
    let mut desired = Vec::new();
    let mut holders: Vec<Vec<u8>> = Vec::new();

    if let Some(dri) = root.dri.as_deref() {
        let pubkey = store::hex32(dri).map_err(|e| internal("root dri", e))?;
        holders.push(pubkey.clone());
        desired.push(DesiredMember {
            pubkey,
            role: MemberRole::Admin,
        });
    }

    let root_id = Uuid::parse_str(&root.id).map_err(|e| internal("root id", e))?;
    let held = store::list_dri_under_root(conn, community, root_id)
        .await
        .map_err(|e| internal("list dri under root", e))?;
    for child in held {
        if child.content.parent.is_none() {
            continue; // root already handled
        }
        let Some(dri) = child.content.dri.as_deref() else {
            continue;
        };
        let pubkey = store::hex32(dri).map_err(|e| internal("child dri", e))?;
        if desired.iter().any(|d| d.pubkey == pubkey) {
            // Already the root admin — keep admin, do not demote to member.
            continue;
        }
        holders.push(pubkey.clone());
        desired.push(DesiredMember {
            pubkey,
            role: MemberRole::Member,
        });
    }

    if !holders.is_empty() {
        let agents = list_attested_agents(conn, community, &holders).await?;
        for agent in agents {
            if desired.iter().any(|d| d.pubkey == agent) {
                continue;
            }
            desired.push(DesiredMember {
                pubkey: agent,
                role: MemberRole::Bot,
            });
        }
    }

    if let Some(agent) = store::get_shapers(conn, community)
        .await
        .map_err(|e| internal("read io_shapers for home roster", e))?
        .and_then(|row| row.content.agent)
    {
        let pubkey = store::hex32(&agent).map_err(|e| internal("org agent", e))?;
        if !desired.iter().any(|d| d.pubkey == pubkey) {
            desired.push(DesiredMember {
                pubkey,
                role: AGENT_ROOM_ROLE,
            });
        }
    }

    Ok(desired)
}

async fn list_attested_agents(
    conn: &mut PgConnection,
    community: CommunityId,
    owners: &[Vec<u8>],
) -> Result<Vec<Vec<u8>>, IngestError> {
    if owners.is_empty() {
        return Ok(vec![]);
    }
    let rows: Vec<(Vec<u8>,)> = sqlx::query_as(
        "SELECT pubkey FROM users \
         WHERE community_id = $1 AND agent_owner_pubkey = ANY($2)",
    )
    .bind(community.as_uuid())
    .bind(owners)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| internal("list attested agents", e))?;
    Ok(rows.into_iter().map(|(pk,)| pk).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_slug_kebab_cases_the_title() {
        assert_eq!(room_slug("Weekday hall"), "weekday-hall");
        assert_eq!(room_slug("  --  "), "project");
        assert_eq!(room_slug("Hall #3"), "hall-3");
    }
}
