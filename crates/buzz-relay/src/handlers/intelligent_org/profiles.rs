//! Org profile commands (`50021` → `39105` + `io_profiles`, Protocol §4.7a,
//! §5.4a).
//!
//! A member replaces their own profile whole: about, skills (relay-derived
//! kebab slugs), and an optional `open_limit`. The signer is the subject —
//! never a content field or a `p` tag naming someone else. Limits beyond
//! §4.7a are `invalid:`; a non-member is `restricted:`. Settlement of a
//! `profile` draft rides [`super::drafts::attach_settlement`] through
//! [`persist_write`] like every other command.

use buzz_core::intelligent_org::{
    skill_slug, OrgProfile, ProfileSetContent, Skill, PROFILE_ABOUT_MAX_CHARS, PROFILE_MAX_SKILLS,
    PROFILE_OPEN_LIMIT_RANGE, PROFILE_SKILL_LABEL_MAX_CHARS,
};
use buzz_db::intelligent_org as store;
use sqlx::{Postgres, Transaction};

use super::apply::Projection;
use super::{
    authorize, begin, content_value, internal, object, parse_content, persist_write, pubkey_tag,
    Command, Persisted,
};
use crate::handlers::ingest::{IngestError, IngestResult};

fn invalid(reason: &str) -> IngestError {
    IngestError::Rejected(format!("invalid: {reason}"))
}

/// Skills as the person wrote them → §4.7a `Skill` rows, kebab-slugged and
/// deduplicated by slug (first label wins).
fn skills_from_labels(labels: &[String]) -> Result<Vec<Skill>, IngestError> {
    if labels.len() > PROFILE_MAX_SKILLS {
        return Err(invalid(&format!("at most {PROFILE_MAX_SKILLS} skills")));
    }
    let mut skills = Vec::with_capacity(labels.len());
    let mut seen = std::collections::HashSet::new();
    for label in labels {
        if label.chars().count() > PROFILE_SKILL_LABEL_MAX_CHARS {
            return Err(invalid(&format!(
                "skill label over {PROFILE_SKILL_LABEL_MAX_CHARS} characters"
            )));
        }
        let slug = skill_slug(label);
        if slug.is_empty() {
            return Err(invalid("skill label must yield a slug"));
        }
        if !seen.insert(slug.clone()) {
            continue;
        }
        skills.push(Skill {
            slug,
            label: label.clone(),
        });
    }
    Ok(skills)
}

fn enforce_limits(content: &ProfileSetContent) -> Result<Vec<Skill>, IngestError> {
    if content.about.chars().count() > PROFILE_ABOUT_MAX_CHARS {
        return Err(invalid(&format!(
            "about over {PROFILE_ABOUT_MAX_CHARS} characters"
        )));
    }
    if let Some(limit) = content.open_limit {
        if !PROFILE_OPEN_LIMIT_RANGE.contains(&limit) {
            return Err(invalid("open_limit must be 1–50 or null"));
        }
    }
    skills_from_labels(&content.skills)
}

/// §5.4a / §4.8: the subject is the signer. A content `pubkey` or a `p` tag
/// naming anyone else is refused so a client cannot smuggle another person's
/// profile through this command.
fn refuse_foreign_subject(cmd: &Command<'_>) -> Result<(), IngestError> {
    let raw = content_value(cmd.event)?;
    if let Some(named) = raw.get("pubkey").and_then(|v| v.as_str()) {
        if named != cmd.actor_hex {
            return Err(invalid("cannot set another member's profile"));
        }
    }
    if let Some(p) = pubkey_tag(cmd.event, "p")? {
        if p != cmd.actor_hex {
            return Err(invalid("cannot set another member's profile"));
        }
    }
    Ok(())
}

async fn next_version(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
) -> Result<u32, IngestError> {
    let row = store::get_profile(tx, cmd.tenant.community(), &cmd.actor_bytes)
        .await
        .map_err(|e| internal("read io_profiles", e))?;
    Ok(row
        .map(|r| r.content.version.saturating_add(1))
        .unwrap_or(1))
}

/// `50021` — replace the signer's org profile.
pub async fn set(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    authorize::require_member(cmd.is_member(&cmd.actor_hex).await?)?;
    refuse_foreign_subject(cmd)?;
    let content: ProfileSetContent = parse_content(cmd.event)?;
    let skills = enforce_limits(&content)?;
    let version = next_version(&mut tx, cmd).await?;
    let profile = OrgProfile {
        pubkey: cmd.actor_hex.clone(),
        version,
        about: content.about,
        skills,
        open_limit: content.open_limit,
        updated_at: cmd.at,
        receipt: cmd.receipt_hex(),
    };
    let rows = vec![cmd.ledger(
        "profile_set",
        object::PROFILE,
        &cmd.actor_hex,
        serde_json::json!({ "version": version }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::Profile(profile)],
        rows,
        serde_json::json!({ "pubkey": cmd.actor_hex, "version": version }).to_string(),
        None,
    )
    .await
}
