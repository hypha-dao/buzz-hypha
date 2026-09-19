//! Org-agent membership — Protocol §6.8.
//!
//! The org agent is a real `channel_members` row in every conversation. These
//! helpers run on the caller's connection so channel create, DM open,
//! `39103` bootstrap, and a passed `shapers/agent` can put or move that row
//! in the same transaction as the command. Per-room joins at creation write
//! no ledger row; only the batch syncs (`why=bootstrap|agent_changed`) do.

use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::channel_members::acquire_channel_membership_lock_on;
use crate::error::{DbError, Result};
use crate::intelligent_org::{self as store, hex32};
use crate::{observability, Db};
use buzz_core::channel::MemberRole;
use buzz_core::CommunityId;
use buzz_datastore_tracing::datastore_span;

/// Ledger `why` for the bootstrap backfill of existing rooms.
pub const AGENT_MEMBERSHIP_WHY_BOOTSTRAP: &str = "bootstrap";
/// Ledger `why` for a passed `shapers/agent` membership move.
pub const AGENT_MEMBERSHIP_WHY_AGENT_CHANGED: &str = "agent_changed";

/// How many rooms a batch membership sync touched, split as Protocol §6.2
/// `agent_membership_synced` detail requires.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentMembershipCounts {
    /// Non-DM channels the sync added or moved the agent into.
    pub channels: u32,
    /// DM channels the sync added or moved the agent into.
    pub dms: u32,
}

/// The live `39103.agent` as 32 bytes, or `None` when the community has no
/// Shaper set or the set names no agent.
pub async fn live_agent_pubkey(
    conn: &mut PgConnection,
    community_id: CommunityId,
) -> Result<Option<Vec<u8>>> {
    let Some(row) = store::get_shapers(conn, community_id).await? else {
        return Ok(None);
    };
    row.content.agent.as_deref().map(hex32).transpose()
}

/// Whether `pubkey` is the live org agent. Missing or malformed `39103.agent`
/// is treated as no agent — the caller then keeps its non-org behaviour.
pub fn is_org_agent(pubkey: &[u8], agent: Option<&[u8]>) -> bool {
    agent.is_some_and(|agent| agent == pubkey)
}

/// Drop the org agent from a participant list so DM identity (hash, cap,
/// discovery `p` tags, the `41010` system message) counts humans only.
pub fn without_agent<'a>(pubkeys: &'a [&'a [u8]], agent: Option<&[u8]>) -> Vec<&'a [u8]> {
    pubkeys
        .iter()
        .copied()
        .filter(|pubkey| !is_org_agent(pubkey, agent))
        .collect()
}

/// Put `agent` in `channel_id` as [`MemberRole::Member`] if they are not
/// already an active member. Re-activates a soft-removed row. Returns
/// `true` when the roster changed.
pub async fn put_org_agent_member(
    conn: &mut PgConnection,
    community_id: CommunityId,
    channel_id: Uuid,
    agent: &[u8],
    invited_by: &[u8],
) -> Result<bool> {
    check_pubkey(agent)?;
    check_pubkey(invited_by)?;
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS(\
             SELECT 1 FROM channel_members \
             WHERE community_id = $1 AND channel_id = $2 AND pubkey = $3 AND removed_at IS NULL\
         )",
    )
    .bind(community_id.as_uuid())
    .bind(channel_id)
    .bind(agent)
    .fetch_one(&mut *conn)
    .await?;
    if already {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO channel_members (community_id, channel_id, pubkey, role, invited_by) \
         VALUES ($1, $2, $3, $4::member_role, $5) \
         ON CONFLICT (community_id, channel_id, pubkey) DO UPDATE SET \
            removed_at = NULL, removed_by = NULL, \
            role = EXCLUDED.role, invited_by = EXCLUDED.invited_by",
    )
    .bind(community_id.as_uuid())
    .bind(channel_id)
    .bind(agent)
    .bind(MemberRole::Member.as_str())
    .bind(invited_by)
    .execute(&mut *conn)
    .await?;
    Ok(true)
}

/// Put the live `39103.agent` in `channel_id` when one exists. No-op before
/// bootstrap or when the agent is already a member.
pub async fn ensure_live_agent_member(
    conn: &mut PgConnection,
    community_id: CommunityId,
    channel_id: Uuid,
    invited_by: &[u8],
) -> Result<Option<Vec<u8>>> {
    let Some(agent) = live_agent_pubkey(conn, community_id).await? else {
        return Ok(None);
    };
    put_org_agent_member(conn, community_id, channel_id, &agent, invited_by).await?;
    Ok(Some(agent))
}

/// Add `agent` to every existing undeleted room except `skip` (the `#shapers`
/// room created in the same bootstrap transaction — `apply` already syncs
/// that roster). Returns how many rooms actually gained a row.
pub async fn backfill_agent_membership(
    conn: &mut PgConnection,
    community_id: CommunityId,
    agent: &[u8],
    invited_by: &[u8],
    skip: Option<Uuid>,
) -> Result<AgentMembershipCounts> {
    check_pubkey(agent)?;
    check_pubkey(invited_by)?;
    let rooms = list_live_rooms(conn, community_id).await?;
    let mut counts = AgentMembershipCounts::default();
    for (channel_id, channel_type) in rooms {
        if skip == Some(channel_id) {
            continue;
        }
        acquire_channel_membership_lock_on(conn, community_id, channel_id).await?;
        if put_org_agent_member(conn, community_id, channel_id, agent, invited_by).await? {
            bump_count(&mut counts, &channel_type);
        }
    }
    Ok(counts)
}

/// Move every active membership row from `from` to `to` (same role), or
/// remove `from` when `to` is `None`. Rooms where `to` is already a member
/// keep that row's role and only drop `from`.
pub async fn move_agent_membership(
    conn: &mut PgConnection,
    community_id: CommunityId,
    from: &[u8],
    to: Option<&[u8]>,
    actor: &[u8],
) -> Result<AgentMembershipCounts> {
    check_pubkey(from)?;
    check_pubkey(actor)?;
    if let Some(to) = to {
        check_pubkey(to)?;
        if to == from {
            return Ok(AgentMembershipCounts::default());
        }
    }
    let rooms = rooms_holding(conn, community_id, from).await?;
    let mut counts = AgentMembershipCounts::default();
    for (channel_id, channel_type, role) in rooms {
        acquire_channel_membership_lock_on(conn, community_id, channel_id).await?;
        if let Some(to) = to {
            let already: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                     SELECT 1 FROM channel_members \
                     WHERE community_id = $1 AND channel_id = $2 \
                       AND pubkey = $3 AND removed_at IS NULL\
                 )",
            )
            .bind(community_id.as_uuid())
            .bind(channel_id)
            .bind(to)
            .fetch_one(&mut *conn)
            .await?;
            if !already {
                sqlx::query(
                    "INSERT INTO channel_members \
                        (community_id, channel_id, pubkey, role, invited_by) \
                     VALUES ($1, $2, $3, $4::member_role, $5) \
                     ON CONFLICT (community_id, channel_id, pubkey) DO UPDATE SET \
                        removed_at = NULL, removed_by = NULL, \
                        role = EXCLUDED.role, invited_by = EXCLUDED.invited_by",
                )
                .bind(community_id.as_uuid())
                .bind(channel_id)
                .bind(to)
                .bind(&role)
                .bind(actor)
                .execute(&mut *conn)
                .await?;
            }
        }
        sqlx::query(
            "UPDATE channel_members \
             SET removed_at = NOW(), removed_by = $1 \
             WHERE community_id = $2 AND channel_id = $3 AND pubkey = $4 \
               AND removed_at IS NULL",
        )
        .bind(actor)
        .bind(community_id.as_uuid())
        .bind(channel_id)
        .bind(from)
        .execute(&mut *conn)
        .await?;
        bump_count(&mut counts, &channel_type);
    }
    Ok(counts)
}

fn check_pubkey(pubkey: &[u8]) -> Result<()> {
    if pubkey.len() != 32 {
        return Err(DbError::InvalidData(format!(
            "pubkey must be 32 bytes, got {}",
            pubkey.len()
        )));
    }
    Ok(())
}

fn bump_count(counts: &mut AgentMembershipCounts, channel_type: &str) {
    if channel_type == "dm" {
        counts.dms = counts.dms.saturating_add(1);
    } else {
        counts.channels = counts.channels.saturating_add(1);
    }
}

async fn list_live_rooms(
    conn: &mut PgConnection,
    community_id: CommunityId,
) -> Result<Vec<(Uuid, String)>> {
    let rows = sqlx::query(
        "SELECT id, channel_type::text AS channel_type FROM channels \
         WHERE community_id = $1 AND deleted_at IS NULL \
         ORDER BY created_at, id",
    )
    .bind(community_id.as_uuid())
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("id")?,
                row.try_get::<String, _>("channel_type")?,
            ))
        })
        .collect()
}

async fn rooms_holding(
    conn: &mut PgConnection,
    community_id: CommunityId,
    pubkey: &[u8],
) -> Result<Vec<(Uuid, String, String)>> {
    let rows = sqlx::query(
        "SELECT c.id, c.channel_type::text AS channel_type, cm.role::text AS role \
         FROM channel_members cm \
         JOIN channels c \
           ON c.community_id = cm.community_id AND c.id = cm.channel_id \
         WHERE cm.community_id = $1 AND cm.pubkey = $2 AND cm.removed_at IS NULL \
           AND c.deleted_at IS NULL \
         ORDER BY c.created_at, c.id",
    )
    .bind(community_id.as_uuid())
    .bind(pubkey)
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("id")?,
                row.try_get::<String, _>("channel_type")?,
                row.try_get::<String, _>("role")?,
            ))
        })
        .collect()
}

impl Db {
    /// The live `39103.agent` as 32 bytes, or `None` when the community has
    /// no Shaper set or the set names no agent. Used by DM open / discovery
    /// — one pool-scoped read, no cache (the set changes on a Shaper vote).
    #[datastore_span(name = "org_agent_pubkey", system = "postgresql")]
    pub async fn org_agent_pubkey(&self, community: CommunityId) -> Result<Option<Vec<u8>>> {
        let mut conn = observability::acquire_writer(
            &self.pool,
            observability::WriterOperation::Authorization,
        )
        .await?;
        live_agent_pubkey(&mut conn, community).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_agent_drops_only_the_named_key() {
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let agent = [3u8; 32];
        let humans = without_agent(&[&alice, &bob, &agent], Some(&agent));
        assert_eq!(humans, vec![&alice[..], &bob[..]]);
        assert_eq!(
            without_agent(&[&alice, &bob], None),
            vec![&alice[..], &bob[..]],
            "no agent means the set is unchanged"
        );
    }

    #[test]
    fn is_org_agent_is_false_when_the_community_has_none() {
        assert!(!is_org_agent(&[1u8; 32], None));
        assert!(is_org_agent(&[1u8; 32], Some(&[1u8; 32])));
    }
}
