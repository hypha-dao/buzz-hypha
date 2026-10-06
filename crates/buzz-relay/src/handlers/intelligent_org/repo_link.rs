//! Link a repository onto a project's `30621` (H-2).
//!
//! The root holder publishes a `30617` tagged `buzz-org` with the work item
//! id. This module appends that coordinate to the relay-signed project. A
//! public GitHub `clone` and `web` are stored as the `a`-tag hint. Any other
//! host, or a URL with credentials, is not linked. Private GitHub is out.

use std::sync::Arc;

use buzz_core::intelligent_org::WorkItemState;
use buzz_core::kind::{KIND_GIT_REPO_ANNOUNCEMENT, KIND_PROJECT};
use buzz_core::tenant::TenantContext;
use buzz_db::intelligent_org::{self as store};
use buzz_db::replaceable::{ParameterizedReplacePrecondition, ParameterizedReplaceStatus};
use buzz_sdk::build_project_with_tags;
use nostr::{Event, Tag, Timestamp};
use sqlx::Row;
use uuid::Uuid;

use super::wall_clock;
use crate::state::AppState;

const MAX_LINKED_REPOS: usize = 16;

/// A public `https://github.com/{owner}/{repo}` URL, without credentials.
pub(crate) fn public_github_repo(url: &str) -> Option<String> {
    let url = url.trim();
    if url.contains('@') || url.contains('?') || url.contains('#') {
        return None;
    }
    let rest = url.strip_prefix("https://github.com/")?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = rest.split('/');
    let owner = parts.next().unwrap_or("");
    let repo = parts.next().unwrap_or("");
    if parts.next().is_some() || owner.is_empty() || repo.is_empty() {
        return None;
    }
    let name = |value: &str| {
        value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            && !value.starts_with('.')
            && !value.ends_with('.')
    };
    if !name(owner) || !name(repo) {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}"))
}

/// Append the announced repository to the project's `30621` when `buzz-org`
/// names a root the author holds. No tag is a no-op.
pub(crate) async fn link_announced_repo(
    state: &Arc<AppState>,
    tenant: &TenantContext,
    event: &Event,
) -> Result<(), String> {
    let Some(item_id) = super::tag_value(event, "buzz-org") else {
        return Ok(());
    };
    let item_id =
        Uuid::parse_str(item_id).map_err(|_| "buzz-org is not a work item id".to_owned())?;
    let author = event.pubkey.to_hex();
    let repo_id = super::tag_value(event, "d").ok_or_else(|| "30617 missing d".to_owned())?;
    let coord = format!("{KIND_GIT_REPO_ANNOUNCEMENT}:{author}:{repo_id}");
    let hint = link_hint(event)?;

    let mut tx = state
        .db
        .begin_event_write_transaction()
        .await
        .map_err(|e| format!("begin repo link: {e}"))?;
    let item = store::get_work_item(&mut tx, tenant.community(), item_id)
        .await
        .map_err(|e| format!("read work item: {e}"))?
        .ok_or_else(|| "work item not found".to_owned())?;
    if item.content.state != WorkItemState::Accepted {
        return Err("only the holder of an accepted project can link a repository".into());
    }
    if item
        .content
        .dri
        .as_deref()
        .is_none_or(|dri| !dri.eq_ignore_ascii_case(&author))
    {
        return Err("only the project holder can link a repository".into());
    }
    let project = item
        .content
        .home
        .as_ref()
        .and_then(|home| home.project.clone())
        .ok_or_else(|| "project has no home repository".to_owned())?;
    let (kind, owner, slug) =
        split_coord(&project).ok_or_else(|| "home.project is not a coordinate".to_owned())?;
    if kind != KIND_PROJECT {
        return Err("home.project is not a 30621".into());
    }
    let relay = state.relay_keypair.public_key().to_hex();
    if !owner.eq_ignore_ascii_case(&relay) {
        return Err("home.project is not signed by this relay".into());
    }
    let owner_bytes = hex::decode(&owner).map_err(|_| "project owner is not hex".to_owned())?;
    let row = sqlx::query(
        "SELECT created_at, tags FROM events \
         WHERE community_id = $1 AND kind = $2 AND pubkey = $3 AND d_tag = $4 AND deleted_at IS NULL \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant.community().as_uuid())
    .bind(KIND_PROJECT as i32)
    .bind(&owner_bytes)
    .bind(&slug)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("read 30621: {e}"))?;
    let Some(row) = row else {
        return Err("30621 is missing".into());
    };
    let created: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let tags_json: serde_json::Value = row.get("tags");
    let mut tags = tags_from_json(&tags_json)?;
    if tags
        .iter()
        .any(|tag| tag.len() >= 2 && tag[0] == "a" && tag[1] == coord)
    {
        return Ok(());
    }
    let members = tags
        .iter()
        .filter(|tag| tag.first().map(String::as_str) == Some("a"))
        .count();
    if members >= MAX_LINKED_REPOS {
        return Err("project already has the maximum number of repositories".into());
    }
    let mut member = vec!["a".to_owned(), coord];
    if let Some(hint) = hint {
        member.push(hint);
    }
    tags.push(member);
    let parsed = tags
        .iter()
        .map(|tag| {
            let parts: Vec<&str> = tag.iter().map(String::as_str).collect();
            Tag::parse(parts).map_err(|e| format!("project tag: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let created_at = wall_clock().max(created.timestamp().unsigned_abs().saturating_add(1));
    let signed = build_project_with_tags("", parsed)
        .map_err(|e| format!("rebuild 30621: {e}"))?
        .allow_self_tagging()
        .custom_created_at(Timestamp::from(created_at))
        .sign_with_keys(&state.relay_keypair)
        .map_err(|e| format!("sign 30621: {e}"))?;
    let stored = state
        .db
        .replace_parameterized_event_in_transaction(
            &mut tx,
            tenant.community(),
            &signed,
            &slug,
            None,
            ParameterizedReplacePrecondition::Unconditional,
        )
        .await
        .map_err(|e| format!("store 30621: {e}"))?;
    if stored.status != ParameterizedReplaceStatus::Inserted {
        return Err(format!("30621 was not replaced ({:?})", stored.status));
    }
    tx.commit()
        .await
        .map_err(|e| format!("commit repo link: {e}"))?;
    Ok(())
}

fn link_hint(event: &Event) -> Result<Option<String>, String> {
    let clone = super::tag_value(event, "clone");
    let web = super::tag_value(event, "web");
    match (clone, web) {
        (None, None) => Ok(None),
        (Some(clone), Some(web)) => {
            let clone = public_github_repo(clone)
                .ok_or_else(|| "clone must be a public https://github.com repository".to_owned())?;
            let web = public_github_repo(web)
                .ok_or_else(|| "web must be a public https://github.com repository".to_owned())?;
            if clone != web {
                return Err("clone and web must name the same GitHub repository".into());
            }
            Ok(Some(clone))
        }
        _ => Err("clone and web must both be set for a GitHub repository".into()),
    }
}

fn split_coord(coord: &str) -> Option<(u32, String, String)> {
    let mut parts = coord.splitn(3, ':');
    let kind = parts.next()?.parse().ok()?;
    let owner = parts.next()?.to_owned();
    let slug = parts.next()?.to_owned();
    if owner.is_empty() || slug.is_empty() {
        return None;
    }
    Some((kind, owner, slug))
}

fn tags_from_json(value: &serde_json::Value) -> Result<Vec<Vec<String>>, String> {
    let rows = value.as_array().ok_or("30621 tags are not an array")?;
    rows.iter()
        .map(|row| {
            let parts = row.as_array().ok_or("30621 tag is not an array")?;
            Ok(parts
                .iter()
                .filter_map(|part| part.as_str().map(str::to_owned))
                .collect())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_public_github_repository_links() {
        assert_eq!(
            public_github_repo("https://github.com/hypha/weekday"),
            Some("https://github.com/hypha/weekday".into())
        );
        assert_eq!(
            public_github_repo("https://github.com/hypha/weekday.git"),
            Some("https://github.com/hypha/weekday".into())
        );
        assert!(public_github_repo("https://user:token@github.com/hypha/weekday").is_none());
        assert!(public_github_repo("https://gitlab.com/hypha/weekday").is_none());
        assert!(public_github_repo("http://github.com/hypha/weekday").is_none());
        assert!(public_github_repo("https://github.com/hypha/weekday/settings").is_none());
    }
}
