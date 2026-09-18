//! Relay-managed rooms — transaction-scoped channel and roster writes.
//!
//! Some channels have exactly one writer: the relay itself. The intelligent
//! organization's `#shapers` room (Protocol §6.4) and a project's home room
//! (§6.7) are created by the relay and their rosters are kept equal to a
//! relay-derived set — `shapers ∪ {agent}`, the work tree — in the **same
//! transaction** as the state event that defines that set. There is no
//! second writer, so there is no reconcile loop; a member removed by Buzz's
//! own channel administration is re-added on the next write.
//!
//! The pool-based [`crate::channel::create_channel`] and
//! [`crate::channel_members::add_member`] open their own transactions and
//! enforce inviter roles that do not apply when the relay is the actor. These
//! functions instead run on the caller's connection — the transaction
//! `persist_command_event` returns — and do only the writes: the room row,
//! the roster upsert, the soft removal. Authorization is the caller's.
//!
//! Every roster write here takes the same per-channel advisory lock as the
//! pool-based writers (`acquire_channel_membership_lock`), so a concurrent
//! `kind:9000`/`9001` on the same room serializes against it rather than
//! interleaving with a half-applied sync.

use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::channel_members::{acquire_channel_membership_lock_on, MemberRecord};
use crate::error::{DbError, Result};
use buzz_core::channel::{ChannelType, ChannelVisibility, MemberRole};
use buzz_core::CommunityId;

/// One desired roster entry for a relay-managed room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredMember {
    /// The member (32 bytes).
    pub pubkey: Vec<u8>,
    /// The role the relay wants them to hold.
    pub role: MemberRole,
}

/// One roster change [`sync_room_roster`] made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RosterChange {
    /// The member was added, or re-activated, or moved to `role`.
    Added {
        /// The member (32 bytes).
        pubkey: Vec<u8>,
        /// The role now held.
        role: MemberRole,
    },
    /// The member was soft-removed.
    Removed {
        /// The member (32 bytes).
        pubkey: Vec<u8>,
    },
}

impl RosterChange {
    /// The member this change concerns.
    pub fn pubkey(&self) -> &[u8] {
        match self {
            Self::Added { pubkey, .. } | Self::Removed { pubkey } => pubkey,
        }
    }
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

/// Create a relay-managed room with no members. Returns the new channel id.
///
/// `created_by` is recorded as the channel creator (the person whose command
/// caused the room, or the relay's own key); it grants no membership — call
/// [`sync_room_roster`] for the roster.
pub async fn create_room(
    conn: &mut PgConnection,
    community_id: CommunityId,
    name: &str,
    channel_type: ChannelType,
    visibility: ChannelVisibility,
    description: Option<&str>,
    created_by: &[u8],
) -> Result<Uuid> {
    check_pubkey(created_by)?;
    let name = buzz_core::channel::canonical_channel_name(name);
    if name.trim().is_empty() {
        return Err(DbError::InvalidData("channel name is required".into()));
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels \
            (id, community_id, name, channel_type, visibility, description, created_by) \
         VALUES ($1, $2, $3, $4::channel_type, $5::channel_visibility, $6, $7)",
    )
    .bind(id)
    .bind(community_id.as_uuid())
    .bind(name)
    .bind(channel_type.as_str())
    .bind(visibility.as_str())
    .bind(description)
    .bind(created_by)
    .execute(conn)
    .await?;
    Ok(id)
}

/// Active members of a room, oldest first, on the caller's connection.
pub async fn list_active_members(
    conn: &mut PgConnection,
    community_id: CommunityId,
    channel_id: Uuid,
) -> Result<Vec<MemberRecord>> {
    let rows = sqlx::query(
        "SELECT channel_id, pubkey, role::text AS role, joined_at, invited_by, removed_at \
         FROM channel_members \
         WHERE community_id = $1 AND channel_id = $2 AND removed_at IS NULL \
         ORDER BY joined_at ASC, pubkey ASC",
    )
    .bind(community_id.as_uuid())
    .bind(channel_id)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(MemberRecord {
                channel_id: row.try_get("channel_id")?,
                pubkey: row.try_get("pubkey")?,
                role: row.try_get("role")?,
                joined_at: row.try_get("joined_at")?,
                invited_by: row.try_get("invited_by")?,
                removed_at: row.try_get("removed_at")?,
            })
        })
        .collect()
}

/// Make the room's active roster exactly `desired` — add or re-activate what
/// is missing, move anyone holding the wrong role, soft-remove everyone else —
/// and return what changed. `actor` is recorded as `invited_by` / `removed_by`.
///
/// The room must exist in `community_id`; a missing room is
/// [`DbError::ChannelNotFound`]. Idempotent: a roster already equal to
/// `desired` returns no changes.
pub async fn sync_room_roster(
    conn: &mut PgConnection,
    community_id: CommunityId,
    channel_id: Uuid,
    desired: &[DesiredMember],
    actor: &[u8],
) -> Result<Vec<RosterChange>> {
    check_pubkey(actor)?;
    for member in desired {
        check_pubkey(&member.pubkey)?;
    }
    acquire_channel_membership_lock_on(conn, community_id, channel_id).await?;

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM channels \
         WHERE community_id = $1 AND id = $2 AND deleted_at IS NULL)",
    )
    .bind(community_id.as_uuid())
    .bind(channel_id)
    .fetch_one(&mut *conn)
    .await?;
    if !exists {
        return Err(DbError::ChannelNotFound(channel_id));
    }

    let current = list_active_members(conn, community_id, channel_id).await?;
    let mut changes = Vec::new();

    for wanted in desired {
        let held = current
            .iter()
            .find(|m| m.pubkey == wanted.pubkey)
            .map(|m| m.role.as_str());
        if held == Some(wanted.role.as_str()) {
            continue;
        }
        sqlx::query(
            "INSERT INTO channel_members (community_id, channel_id, pubkey, role, invited_by) \
             VALUES ($1, $2, $3, $4::member_role, $5) \
             ON CONFLICT (community_id, channel_id, pubkey) DO UPDATE SET \
                removed_at = NULL, removed_by = NULL, role = EXCLUDED.role, \
                invited_by = EXCLUDED.invited_by",
        )
        .bind(community_id.as_uuid())
        .bind(channel_id)
        .bind(&wanted.pubkey)
        .bind(wanted.role.as_str())
        .bind(actor)
        .execute(&mut *conn)
        .await?;
        changes.push(RosterChange::Added {
            pubkey: wanted.pubkey.clone(),
            role: wanted.role,
        });
    }

    for extra in current
        .iter()
        .filter(|m| !desired.iter().any(|d| d.pubkey == m.pubkey))
    {
        sqlx::query(
            "UPDATE channel_members SET removed_at = NOW(), removed_by = $1 \
             WHERE community_id = $2 AND channel_id = $3 AND pubkey = $4 AND removed_at IS NULL",
        )
        .bind(actor)
        .bind(community_id.as_uuid())
        .bind(channel_id)
        .bind(&extra.pubkey)
        .execute(&mut *conn)
        .await?;
        changes.push(RosterChange::Removed {
            pubkey: extra.pubkey.clone(),
        });
    }

    Ok(changes)
}

#[cfg(test)]
mod postgres_tests {
    use super::*;
    use sqlx::PgPool;

    async fn context() -> (PgPool, CommunityId) {
        let pool = PgPool::connect(&crate::test_support::database_url())
            .await
            .expect("connect relay-rooms test database");
        let db = crate::Db::from_pool(pool.clone());
        if std::env::var("BUZZ_TEST_SCHEMA_MODE").as_deref() != Ok("desired") {
            db.migrate().await.expect("migrate test database");
        }
        let host = format!("relay-rooms-{}.example", Uuid::new_v4().simple());
        let community = db
            .ensure_configured_community(&host)
            .await
            .expect("create test community")
            .id;
        (pool, community)
    }

    fn member(seed: u8, role: MemberRole) -> DesiredMember {
        DesiredMember {
            pubkey: vec![seed; 32],
            role,
        }
    }

    fn roster(members: &[MemberRecord]) -> Vec<(Vec<u8>, String)> {
        let mut out: Vec<_> = members
            .iter()
            .map(|m| (m.pubkey.clone(), m.role.clone()))
            .collect();
        out.sort();
        out
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn sync_makes_the_roster_equal_to_desired_in_both_directions() {
        let (pool, community) = context().await;
        let mut conn = pool.acquire().await.expect("acquire");
        let room = create_room(
            &mut conn,
            community,
            "#shapers",
            ChannelType::Stream,
            ChannelVisibility::Private,
            Some("Direction talk"),
            &[1; 32],
        )
        .await
        .expect("create room");
        let name: String =
            sqlx::query_scalar("SELECT name FROM channels WHERE community_id = $1 AND id = $2")
                .bind(community.as_uuid())
                .bind(room)
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(name, "shapers", "the leading # is canonicalised away");
        assert!(list_active_members(&mut conn, community, room)
            .await
            .unwrap()
            .is_empty());

        let desired = [
            member(1, MemberRole::Owner),
            member(0xA0, MemberRole::Member),
        ];
        let changes = sync_room_roster(&mut conn, community, room, &desired, &[1; 32])
            .await
            .unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(
            roster(
                &list_active_members(&mut conn, community, room)
                    .await
                    .unwrap()
            ),
            vec![
                (vec![1; 32], "owner".to_owned()),
                (vec![0xA0; 32], "member".to_owned()),
            ]
        );
        assert!(
            sync_room_roster(&mut conn, community, room, &desired, &[1; 32])
                .await
                .unwrap()
                .is_empty(),
            "an equal roster is a no-op"
        );

        // Buzz's own channel administration drifts the roster: one Shaper is
        // removed, a stranger is added. The next sync repairs both.
        sqlx::query(
            "UPDATE channel_members SET removed_at = NOW() \
             WHERE community_id = $1 AND channel_id = $2 AND pubkey = $3",
        )
        .bind(community.as_uuid())
        .bind(room)
        .bind(vec![1u8; 32])
        .execute(&mut *conn)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO channel_members (community_id, channel_id, pubkey, role) \
             VALUES ($1, $2, $3, 'member')",
        )
        .bind(community.as_uuid())
        .bind(room)
        .bind(vec![0xEEu8; 32])
        .execute(&mut *conn)
        .await
        .unwrap();
        let desired = [
            member(1, MemberRole::Owner),
            member(2, MemberRole::Owner),
            member(0xA0, MemberRole::Member),
        ];
        let mut changes = sync_room_roster(&mut conn, community, room, &desired, &[2; 32])
            .await
            .unwrap();
        changes.sort_by(|a, b| a.pubkey().cmp(b.pubkey()));
        assert_eq!(
            changes,
            vec![
                RosterChange::Added {
                    pubkey: vec![1; 32],
                    role: MemberRole::Owner,
                },
                RosterChange::Added {
                    pubkey: vec![2; 32],
                    role: MemberRole::Owner,
                },
                RosterChange::Removed {
                    pubkey: vec![0xEE; 32],
                },
            ]
        );
        assert_eq!(
            roster(
                &list_active_members(&mut conn, community, room)
                    .await
                    .unwrap()
            ),
            vec![
                (vec![1; 32], "owner".to_owned()),
                (vec![2; 32], "owner".to_owned()),
                (vec![0xA0; 32], "member".to_owned()),
            ]
        );

        // A role move is one change, not a remove plus an add.
        let desired = [
            member(1, MemberRole::Owner),
            member(2, MemberRole::Member),
            member(0xA0, MemberRole::Member),
        ];
        let changes = sync_room_roster(&mut conn, community, room, &desired, &[1; 32])
            .await
            .unwrap();
        assert_eq!(
            changes,
            vec![RosterChange::Added {
                pubkey: vec![2; 32],
                role: MemberRole::Member,
            }]
        );

        assert!(matches!(
            sync_room_roster(&mut conn, community, Uuid::new_v4(), &desired, &[1; 32]).await,
            Err(DbError::ChannelNotFound(_))
        ));
    }

    #[tokio::test]
    #[ignore = "requires Postgres"]
    async fn room_and_roster_roll_back_with_the_callers_transaction() {
        let (pool, community) = context().await;
        let mut tx = pool.begin().await.expect("begin");
        let room = create_room(
            &mut tx,
            community,
            "shapers",
            ChannelType::Stream,
            ChannelVisibility::Private,
            None,
            &[1; 32],
        )
        .await
        .unwrap();
        sync_room_roster(
            &mut tx,
            community,
            room,
            &[member(1, MemberRole::Owner)],
            &[1; 32],
        )
        .await
        .unwrap();
        tx.rollback().await.expect("roll back");

        let mut conn = pool.acquire().await.expect("acquire");
        let rooms: i64 =
            sqlx::query_scalar("SELECT count(*) FROM channels WHERE community_id = $1 AND id = $2")
                .bind(community.as_uuid())
                .bind(room)
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(rooms, 0, "the room leaves with the transaction");
        assert!(list_active_members(&mut conn, community, room)
            .await
            .unwrap()
            .is_empty());
    }
}
