//! `50024` opens a codebases proposal. Passing it replaces the org list
//! (`39106`, `d` = `codebases`).
//!
//! A seated Shaper publishes the whole list: each repository is a name, a
//! URL, and one line on what it is, and the landing page is one link on
//! that same list. The direction rule decides the proposal. Strategy's
//! version does not move.

use buzz_core::intelligent_org::ProposalKind;
use buzz_core::kind::KIND_IO_KNOWLEDGE;
use buzz_db::intelligent_org::LedgerEntry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};

use super::apply::Projection;
use super::{
    authorize, begin, current_shapers, internal, object, parse_content, Command, Persisted,
};
use crate::handlers::ingest::{IngestError, IngestResult};

/// What a passed codebases proposal writes, for the same `apply` as the `39102`.
pub(super) struct PassedList {
    pub projection: Projection,
    pub row: LedgerEntry,
}

const SLUG: &str = "codebases";
const MAX_ITEMS: usize = 20;
const NAME_MAX: usize = 80;
const ABOUT_MAX: usize = 280;
const URL_MAX: usize = 300;

#[derive(Debug, Default, Deserialize)]
struct SetContent {
    slug: String,
    items: Vec<ItemIn>,
}

#[derive(Debug, Clone, Deserialize)]
struct ItemIn {
    #[serde(default)]
    id: String,
    kind: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    about: String,
    /// Commit the file list was read from. Empty when the repo was not read.
    #[serde(default)]
    commit: String,
    /// Paths a code step may name. Empty when the repo was not read.
    #[serde(default)]
    files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredItem {
    id: String,
    kind: String,
    name: String,
    url: String,
    about: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    commit: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    files: Vec<String>,
}

fn invalid(reason: &str) -> IngestError {
    IngestError::Rejected(format!("invalid: {reason}"))
}

/// The list a Shaper may store. Empty input and a bad URL are refused.
fn normalize_items(items: &[ItemIn]) -> Result<Vec<StoredItem>, IngestError> {
    if items.is_empty() {
        return Err(invalid("codebases need at least one link"));
    }
    if items.len() > MAX_ITEMS {
        return Err(invalid(&format!("at most {MAX_ITEMS} codebases")));
    }
    let mut stored = Vec::with_capacity(items.len());
    let mut seen_url = std::collections::HashSet::new();
    let mut seen_id = std::collections::HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let kind = item.kind.trim();
        if kind != "repository" && kind != "site" {
            return Err(invalid("codebase kind must be repository or site"));
        }
        let name = item.name.trim();
        let url = item.url.trim();
        let about = item.about.trim();
        if name.chars().count() > NAME_MAX {
            return Err(invalid(&format!(
                "codebase name over {NAME_MAX} characters"
            )));
        }
        if about.chars().count() > ABOUT_MAX {
            return Err(invalid(&format!(
                "codebase about over {ABOUT_MAX} characters"
            )));
        }
        if !url.is_empty()
            && (!(url.starts_with("https://") || url.starts_with("http://"))
                || url.chars().count() > URL_MAX
                || url.chars().any(char::is_whitespace))
        {
            return Err(invalid("codebase url must be an http(s) link"));
        }
        if !url.is_empty() && !seen_url.insert(url.to_string()) {
            return Err(invalid("duplicate codebase url"));
        }
        let id = item_id(&item.id, kind, name, index, &mut seen_id);
        let (commit, files) = if kind == "repository" {
            digest_of(&item.commit, &item.files)?
        } else {
            (String::new(), Vec::new())
        };
        stored.push(StoredItem {
            id,
            kind: kind.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            about: about.to_string(),
            commit,
            files,
        });
    }
    Ok(stored)
}

const DIGEST_MAX_FILES: usize = 80;
const DIGEST_PATH_MAX: usize = 180;

/// A repository digest the planner may name. A path with `..` is refused.
fn digest_of(commit: &str, files: &[String]) -> Result<(String, Vec<String>), IngestError> {
    let commit = commit.trim();
    if commit.chars().count() > 80 || commit.chars().any(char::is_whitespace) {
        return Err(invalid("codebase commit is not a single token"));
    }
    if files.len() > DIGEST_MAX_FILES {
        return Err(invalid(&format!("at most {DIGEST_MAX_FILES} digest files")));
    }
    let mut kept = Vec::with_capacity(files.len());
    for file in files {
        let path = file.trim();
        if path.is_empty() {
            continue;
        }
        if path.chars().count() > DIGEST_PATH_MAX
            || path.contains("..")
            || path.starts_with('/')
            || path.chars().any(char::is_whitespace)
        {
            return Err(invalid("digest path is not a file in the repository"));
        }
        if !kept.iter().any(|seen: &String| seen == path) {
            kept.push(path.to_string());
        }
    }
    Ok((commit.to_string(), kept))
}

fn item_id(
    given: &str,
    kind: &str,
    name: &str,
    index: usize,
    seen: &mut std::collections::HashSet<String>,
) -> String {
    let source = if given.trim().is_empty() {
        if name.is_empty() {
            kind
        } else {
            name
        }
    } else {
        given.trim()
    };
    let mut id = String::new();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-' | '_' | '/') && !id.ends_with('-') && !id.is_empty() {
            id.push('-');
        }
    }
    let id = id.trim_matches('-');
    let base = if id.is_empty() {
        format!("{kind}-{}", index + 1)
    } else {
        id.to_string()
    };
    let mut candidate = base.clone();
    let mut n = 2u32;
    while !seen.insert(candidate.clone()) {
        candidate = format!("{base}-{n}");
        n = n.saturating_add(1);
    }
    candidate
}

async fn previous_version(
    cmd: &Command<'_>,
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
) -> Result<u32, IngestError> {
    let relay = cmd.state.relay_keypair.public_key().to_bytes();
    let row: Option<String> = sqlx::query_scalar(
        "SELECT content FROM events \
         WHERE community_id = $1 AND kind = $2 AND pubkey = $3 AND d_tag = $4 \
           AND deleted_at IS NULL \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(cmd.tenant.community().as_uuid())
    .bind(KIND_IO_KNOWLEDGE as i32)
    .bind(relay.as_slice())
    .bind(SLUG)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| internal("read codebases", e))?;
    let version = row
        .as_deref()
        .and_then(|content| serde_json::from_str::<Value>(content).ok())
        .and_then(|value| value.get("version").and_then(Value::as_u64))
        .unwrap_or(0);
    Ok(u32::try_from(version).unwrap_or(0).saturating_add(1))
}

fn summary(items: &[StoredItem]) -> (String, String) {
    let title = items
        .iter()
        .find(|item| !item.url.is_empty())
        .map(|item| {
            if item.name.is_empty() {
                item.url.clone()
            } else {
                item.name.clone()
            }
        })
        .unwrap_or_else(|| {
            if items.iter().any(|item| item.kind == "site") {
                "No landing page yet".to_string()
            } else {
                "No repository yet".to_string()
            }
        });
    let brief = items
        .iter()
        .map(|item| {
            let label = if item.kind == "site" {
                "Landing page"
            } else {
                "Repository"
            };
            let name = if item.name.is_empty() {
                label
            } else {
                item.name.as_str()
            };
            if item.url.is_empty() {
                let about = if item.about.is_empty() {
                    "not set"
                } else {
                    item.about.as_str()
                };
                format!("{name} — {about}")
            } else if item.about.is_empty() {
                format!("{name} — {}", item.url)
            } else {
                format!("{name} — {} — {}", item.url, item.about)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    (title, brief)
}

/// `50024` — a Shaper opens a codebases proposal. Passing writes `39106`.
pub async fn set(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let content: SetContent = parse_content(cmd.event)?;
    if content.slug != SLUG {
        return Err(invalid("knowledge slug must be codebases"));
    }
    let items = normalize_items(&content.items)?;
    let (title, brief) = summary(&items);
    let payload = serde_json::json!({
        "slug": SLUG,
        "items": items,
        "title": title,
        "brief": brief,
    });

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("restricted: not a Shaper".into()))?;
    authorize::require_shaper(&shapers, &cmd.actor_hex)?;
    let rule = shapers.rules.direction;
    let opening = super::proposals::Opening {
        kind: ProposalKind::Codebases,
        rule,
        subject: None,
        item: None,
        payload,
        detail: serde_json::json!({ "kind": "codebases", "rule": rule }),
    };
    super::proposals::open_and_settle(cmd, tx, shapers, opening).await
}

/// The list a passed codebases proposal stores. Version is the live head plus one.
pub(super) async fn write_passed(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    payload: &Value,
) -> Result<PassedList, IngestError> {
    let items = payload
        .get("items")
        .cloned()
        .ok_or_else(|| invalid("codebases proposal has no list"))?;
    let parsed: Vec<ItemIn> = serde_json::from_value(items)
        .map_err(|error| IngestError::Rejected(format!("invalid: command content: {error}")))?;
    let items = normalize_items(&parsed)?;
    let version = previous_version(cmd, tx).await?;
    let body = serde_json::json!({
        "slug": SLUG,
        "version": version,
        "items": items,
        "updated_at": cmd.at,
        "receipt": cmd.receipt_hex(),
    });
    let row = cmd.ledger(
        "knowledge_set",
        object::KNOWLEDGE,
        SLUG,
        serde_json::json!({ "version": version, "items": items.len() }),
    )?;
    Ok(PassedList {
        projection: Projection::Knowledge {
            d_tag: SLUG.to_string(),
            version,
            content: body.to_string(),
        },
        row,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::handlers::ingest::IngestError;

    fn item(kind: &str, name: &str, url: &str, about: &str) -> ItemIn {
        ItemIn {
            id: String::new(),
            kind: kind.into(),
            name: name.into(),
            url: url.into(),
            about: about.into(),
            commit: String::new(),
            files: Vec::new(),
        }
    }

    #[test]
    fn a_repository_and_a_landing_page_keep_name_url_and_about() {
        let items = normalize_items(&[
            item(
                "repository",
                "buzz-hypha",
                "https://github.com/hypha-dao/buzz-hypha",
                "The product",
            ),
            item("site", "Hypha", "https://hypha.earth", "Public site"),
        ])
        .expect("list");
        assert_eq!(items[0].id, "buzz-hypha");
        assert_eq!(items[0].kind, "repository");
        assert_eq!(items[0].about, "The product");
        assert_eq!(items[1].kind, "site");
        assert_eq!(items[1].url, "https://hypha.earth");
    }

    fn reason(err: IngestError) -> String {
        match err {
            IngestError::Rejected(message) => message,
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn a_blank_list_and_a_bad_url_are_refused() {
        assert!(reason(normalize_items(&[]).expect_err("empty")).contains("at least one"));
        let bad = normalize_items(&[item("repository", "x", "ftp://files.example", "no")]);
        assert!(reason(bad.expect_err("scheme")).contains("http(s)"));
        let wrong = normalize_items(&[item("strategy", "x", "https://example.com", "no")]);
        assert!(reason(wrong.expect_err("kind")).contains("repository or site"));
    }

    #[test]
    fn a_repository_digest_is_kept_and_a_parent_path_is_refused() {
        let mut repo = item(
            "repository",
            "buzz",
            "https://github.com/hypha-dao/buzz-hypha",
            "The product",
        );
        repo.commit = "abc1234".into();
        repo.files = vec!["crates/buzz-org-agent/src/plan.rs".into()];
        let items = normalize_items(&[repo]).expect("digest");
        assert_eq!(items[0].commit, "abc1234");
        assert_eq!(items[0].files, vec!["crates/buzz-org-agent/src/plan.rs"]);
        let mut bad = item(
            "repository",
            "buzz",
            "https://example.com/buzz",
            "The product",
        );
        bad.files = vec!["../secrets.env".into()];
        assert!(reason(normalize_items(&[bad]).expect_err("parent")).contains("digest path"));
    }
}
