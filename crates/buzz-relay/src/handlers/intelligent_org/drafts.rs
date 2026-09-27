//! Drafts, reads, and settlement (Protocol §3.2, §3.3, §4.3, §4.6–4.7c,
//! §5.4, §5.4a, §6.1).
//!
//! `50100` / `50101` / `50103` are regular events: stored through
//! [`super::begin`], then projected on the same transaction. Settlement
//! rides on any command carrying `["e", id, "", "draft"]` and is attached
//! by [`attach_settlement`] before [`super::apply::apply`]. `50012` is the
//! standalone decide (no draft marker). `50017` is a Shaper's blind band.

use buzz_core::intelligent_org::{
    tag, AgentNote, DeclineReason, DirectionSlug, DraftDecision, DraftKind, DraftOrigin,
    DraftOutcome, DraftOutcomeStatus, DraftPayload, HealthBand, HealthRead, OrgProfile,
};
use buzz_core::kind::{KIND_IO_DRAFT_DECIDE, KIND_IO_PROFILE, KIND_IO_WORK_ITEM};
use buzz_db::intelligent_org::{self as store, DraftRow, HealthRatingRow, HealthRow, LedgerEntry};
use nostr::Event;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::apply::Projection;
use super::{
    authorize, begin, content_value, current_shapers, hex32_tag, internal, object, parse_content,
    persist_write, tag_value, uuid_tag, Command, Persisted,
};
use crate::handlers::ingest::{IngestError, IngestResult};

fn invalid(reason: &str) -> IngestError {
    IngestError::Rejected(format!("invalid: {reason}"))
}

fn parse_hex32(value: &str, what: &str) -> Result<Vec<u8>, IngestError> {
    let is_hex = value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !is_hex {
        return Err(invalid(&format!(
            "{what} must be a 64-char lowercase hex id"
        )));
    }
    hex::decode(value).map_err(|_| invalid(&format!("{what} must be a 64-char lowercase hex id")))
}

fn draft_marker(event: &Event) -> Result<Option<Vec<u8>>, IngestError> {
    for t in event.tags.iter() {
        let parts = t.as_slice();
        if parts.len() >= 4 && parts[0] == "e" && parts[3] == tag::MARKER_DRAFT {
            return Ok(Some(parse_hex32(&parts[1], "draft")?));
        }
    }
    Ok(None)
}

enum Receipt {
    Event(Vec<u8>),
    Coord {
        kind: u32,
        pubkey: Vec<u8>,
        d_tag: String,
    },
    Line(String),
}

fn parse_coord(value: &str) -> Result<Receipt, IngestError> {
    let (kind_s, rest) = value
        .split_once(':')
        .ok_or_else(|| invalid("draft shape"))?;
    let kind: u32 = kind_s.parse().map_err(|_| invalid("unresolved receipt"))?;
    if rest.len() < 64 {
        return Err(invalid("unresolved receipt"));
    }
    let (pk, d_tag) = rest.split_at(64);
    let d_tag = d_tag
        .strip_prefix(':')
        .ok_or_else(|| invalid("unresolved receipt"))?;
    Ok(Receipt::Coord {
        kind,
        pubkey: parse_hex32(pk, "receipt")?,
        d_tag: d_tag.to_owned(),
    })
}

fn collect_receipts(event: &Event) -> Result<Vec<Receipt>, IngestError> {
    let mut out = Vec::new();
    for t in event.tags.iter() {
        let parts = t.as_slice();
        match parts.first().map(String::as_str) {
            Some("e") if parts.len() >= 4 && parts[3] == tag::MARKER_RECEIPT => {
                out.push(Receipt::Event(parse_hex32(&parts[1], "receipt")?));
            }
            Some("a") if parts.len() >= 4 && parts[3] == tag::MARKER_RECEIPT => {
                out.push(parse_coord(&parts[1])?);
            }
            Some("ref") if parts.get(1).is_some_and(|v| !v.is_empty()) => {
                out.push(Receipt::Line(parts[1].clone()));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_line_ref(value: &str) -> Option<(DirectionSlug, u32, &str)> {
    let (slug_s, rest) = value.split_once('@')?;
    let slug = match slug_s {
        "objectives" => DirectionSlug::Objectives,
        "strategy" => DirectionSlug::Strategy,
        _ => return None,
    };
    let (version, line) = rest.split_once('#')?;
    let version = version.parse().ok()?;
    if line.is_empty() {
        return None;
    }
    Some((slug, version, line))
}

async fn event_exists(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    id: &[u8],
) -> Result<bool, IngestError> {
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM events \
         WHERE community_id = $1 AND id = $2 AND deleted_at IS NULL)",
    )
    .bind(cmd.tenant.community().as_uuid())
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| internal("resolve receipt", e))?;
    Ok(found)
}

async fn coord_exists(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    kind: u32,
    pubkey: &[u8],
    d_tag: &str,
) -> Result<bool, IngestError> {
    let kind = i32::try_from(kind).map_err(|e| internal("receipt kind", e))?;
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM events \
         WHERE community_id = $1 AND kind = $2 AND pubkey = $3 AND d_tag = $4 \
           AND deleted_at IS NULL)",
    )
    .bind(cmd.tenant.community().as_uuid())
    .bind(kind)
    .bind(pubkey)
    .bind(d_tag)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| internal("resolve receipt", e))?;
    Ok(found)
}

async fn resolve_receipts(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    receipts: &[Receipt],
) -> Result<(), IngestError> {
    for receipt in receipts {
        let ok = match receipt {
            Receipt::Event(id) => event_exists(tx, cmd, id).await?,
            Receipt::Coord {
                kind,
                pubkey,
                d_tag,
            } => coord_exists(tx, cmd, *kind, pubkey, d_tag).await?,
            Receipt::Line(value) => {
                let Some((slug, version, line_id)) = parse_line_ref(value) else {
                    return Err(invalid("unresolved receipt"));
                };
                let head = store::get_direction_head(tx, cmd.tenant.community(), slug)
                    .await
                    .map_err(|e| internal("read io_direction", e))?;
                head.is_some_and(|row| {
                    row.content.slug == slug
                        && row.content.version == version
                        && row.content.lines.iter().any(|line| line.id == line_id)
                })
            }
        };
        if !ok {
            return Err(invalid("unresolved receipt"));
        }
    }
    Ok(())
}

fn suggested_holder(event: &Event, payload: &DraftPayload) -> Option<String> {
    for t in event.tags.iter() {
        let parts = t.as_slice();
        if parts.len() >= 4 && parts[0] == "p" && parts[3] == tag::MARKER_SUGGESTED {
            return Some(parts[1].clone());
        }
    }
    match payload {
        DraftPayload::Project(p) => p.suggested_dri.clone(),
        DraftPayload::Ticket(t) => t.suggested_holder.clone(),
        DraftPayload::Dri(d) => Some(d.suggested.clone()),
        _ => None,
    }
}

fn skill_slugs(event: &Event) -> Vec<String> {
    event
        .tags
        .iter()
        .filter_map(|t| {
            let parts = t.as_slice();
            (parts.first().map(String::as_str) == Some(tag::SKILL))
                .then(|| parts.get(1).cloned())
                .flatten()
        })
        .collect()
}

async fn event_kind_d(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    id: &[u8],
) -> Result<Option<(u32, Option<String>)>, IngestError> {
    let row: Option<(i32, Option<String>)> = sqlx::query_as(
        "SELECT kind, d_tag FROM events \
         WHERE community_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(cmd.tenant.community().as_uuid())
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| internal("read receipt event", e))?;
    Ok(row.and_then(|(kind, d)| u32::try_from(kind).ok().map(|k| (k, d))))
}

async fn holder_profile(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    holder: &str,
) -> Result<Option<OrgProfile>, IngestError> {
    let bytes = store::hex32(holder).map_err(|e| internal("holder pubkey", e))?;
    if let Some(row) = store::get_profile(tx, cmd.tenant.community(), &bytes)
        .await
        .map_err(|e| internal("read io_profiles", e))?
    {
        return Ok(Some(row.content));
    }
    let content: Option<String> = sqlx::query_scalar(
        "SELECT content FROM events \
         WHERE community_id = $1 AND kind = $2 AND d_tag = $3 AND deleted_at IS NULL \
         ORDER BY created_at DESC, id ASC LIMIT 1",
    )
    .bind(cmd.tenant.community().as_uuid())
    .bind(KIND_IO_PROFILE as i32)
    .bind(holder)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| internal("read 39105", e))?;
    content
        .map(|c| serde_json::from_str(&c).map_err(|e| internal("parse 39105", e)))
        .transpose()
}

async fn holder_rules(
    tx: &mut Transaction<'static, Postgres>,
    cmd: &Command<'_>,
    holder: &str,
    receipts: &[Receipt],
    slugs: &[String],
) -> Result<(), IngestError> {
    let mut evidenced = false;
    for receipt in receipts {
        match receipt {
            Receipt::Coord { kind, d_tag, .. } if *kind == KIND_IO_PROFILE && d_tag == holder => {
                evidenced = true;
            }
            Receipt::Event(id) => {
                if let Some((kind, d_tag)) = event_kind_d(tx, cmd, id).await? {
                    if kind == KIND_IO_WORK_ITEM {
                        let Some(d_tag) = d_tag else {
                            continue;
                        };
                        let Ok(item_id) = Uuid::parse_str(&d_tag) else {
                            continue;
                        };
                        let item = store::get_work_item(tx, cmd.tenant.community(), item_id)
                            .await
                            .map_err(|e| internal("read io_work_items", e))?;
                        if item.is_some_and(|row| row.content.dri.as_deref() == Some(holder)) {
                            evidenced = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if !evidenced {
        return Err(invalid("unresolved receipt"));
    }

    let profile = holder_profile(tx, cmd, holder).await?;
    if !slugs.is_empty() {
        let skills = profile
            .as_ref()
            .map(|p| p.skills.iter().map(|s| s.slug.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        if slugs.iter().any(|slug| !skills.contains(&slug.as_str())) {
            return Err(invalid("skill not on the holder's profile"));
        }
    }
    if let Some(limit) = profile.as_ref().and_then(|p| p.open_limit) {
        let dri = store::hex32(holder).map_err(|e| internal("holder pubkey", e))?;
        let held = store::count_held_items(tx, cmd.tenant.community(), &dri)
            .await
            .map_err(|e| internal("count held items", e))?;
        if held >= i64::from(limit) {
            return Err(invalid("holder at limit"));
        }
    }
    Ok(())
}

fn draft_kind(event: &Event) -> Result<DraftKind, IngestError> {
    let value = tag_value(event, tag::TYPE).ok_or_else(|| invalid("draft shape"))?;
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| invalid("draft shape"))
}

fn origin_tag(event: &Event) -> Result<Option<DraftOrigin>, IngestError> {
    tag_value(event, "origin")
        .map(|v| {
            serde_json::from_value(serde_json::Value::String(v.to_owned()))
                .map_err(|_| invalid("draft shape"))
        })
        .transpose()
}

fn move_tag(event: &Event) -> Result<Option<i16>, IngestError> {
    tag_value(event, "move")
        .map(|v| {
            let n: i16 = v.parse().map_err(|_| invalid("draft shape"))?;
            if (1..=4).contains(&n) {
                Ok(n)
            } else {
                Err(invalid("draft shape"))
            }
        })
        .transpose()
}

fn needs_value(event: &Event) -> Result<String, IngestError> {
    let needs = tag_value(event, tag::NEEDS).ok_or_else(|| invalid("draft shape"))?;
    if needs == tag::NEEDS_SHAPER {
        return Ok(needs.to_owned());
    }
    let _ = parse_hex32(needs, "needs")?;
    let has_p = event.tags.iter().any(|t| {
        let parts = t.as_slice();
        parts.len() >= 4 && parts[0] == "p" && parts[1] == needs && parts[3] == tag::MARKER_NEEDS
    });
    if !has_p {
        return Err(invalid("draft shape"));
    }
    Ok(needs.to_owned())
}

fn required_item_tag(
    event: &Event,
    kind: DraftKind,
) -> Result<(Option<Uuid>, Option<Uuid>), IngestError> {
    match kind {
        DraftKind::Ticket | DraftKind::Done => {
            let id = uuid_tag(event, tag::PARENT)?.ok_or_else(|| invalid("draft shape"))?;
            Ok((None, Some(id)))
        }
        DraftKind::Dri | DraftKind::Review | DraftKind::Money => {
            let id = uuid_tag(event, tag::ITEM)?.ok_or_else(|| invalid("draft shape"))?;
            Ok((Some(id), None))
        }
        _ => Ok((None, None)),
    }
}

/// Store a `50100` and emit `39104` `open` (or `shadow` from birth).
pub(super) async fn ingest_draft(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd).await?;
    let agent = shapers.as_ref().and_then(|s| s.agent.as_deref());
    authorize::require_member_or_agent(
        cmd.is_member(&cmd.actor_hex).await?,
        agent,
        &cmd.actor_hex,
    )?;

    let kind = draft_kind(cmd.event)?;
    let needs = needs_value(cmd.event)?;
    let gap = tag_value(cmd.event, "gap")
        .filter(|g| !g.is_empty())
        .ok_or_else(|| invalid("draft shape"))?
        .to_owned();
    let receipts = collect_receipts(cmd.event)?;
    if receipts.is_empty() {
        return Err(invalid("draft shape"));
    }
    let payload =
        DraftPayload::parse(kind, &cmd.event.content).map_err(|_| invalid("draft shape"))?;
    if let DraftPayload::Profile(profile) = &payload {
        authorize::profile_draft_needs(&needs, &profile.pubkey)?;
    }
    let (item_id, parent_id) = required_item_tag(cmd.event, kind)?;
    let shadow = tag_value(cmd.event, "shadow") == Some("true");
    let expires_at = tag_value(cmd.event, "expiration")
        .map(|v| {
            let secs: u64 = v.parse().map_err(|_| invalid("draft shape"))?;
            store::ts(secs).map_err(|e| internal("draft expiration", e))
        })
        .transpose()?;

    resolve_receipts(&mut tx, cmd, &receipts).await?;
    if let Some(holder) = suggested_holder(cmd.event, &payload) {
        holder_rules(&mut tx, cmd, &holder, &receipts, &skill_slugs(cmd.event)).await?;
    }
    if !shadow
        && store::find_open_draft_for_gap(&mut tx, cmd.tenant.community(), &gap)
            .await
            .map_err(|e| internal("read io_drafts", e))?
            .is_some()
    {
        return Err(invalid("an open draft already exists for this gap"));
    }

    let status = if shadow {
        DraftOutcomeStatus::Shadow
    } else {
        DraftOutcomeStatus::Open
    };
    let draft_hex = cmd.receipt_hex();
    let outcome = DraftOutcome {
        draft: draft_hex.clone(),
        status,
        decided_by: None,
        decided_at: None,
        reason: None,
        result: None,
    };
    let payload_value = serde_json::to_value(&payload).map_err(|e| internal("draft payload", e))?;
    let row = DraftRow {
        event_id: cmd.receipt_bytes(),
        author: cmd.actor_bytes.clone(),
        draft_kind: kind,
        needs,
        gap: gap.clone(),
        r#move: Some(move_tag(cmd.event)?.ok_or_else(|| invalid("draft shape"))?),
        origin: Some(origin_tag(cmd.event)?.ok_or_else(|| invalid("draft shape"))?),
        item_id,
        parent_id,
        shadow,
        expires_at,
        payload: payload_value,
        outcome: outcome.clone(),
        outcome_event_id: None,
        created_at: store::ts(cmd.at).map_err(|e| internal("draft time", e))?,
    };
    let rows = vec![cmd.ledger(
        "draft_stored",
        object::DRAFT,
        &draft_hex,
        serde_json::json!({ "gap": gap, "kind": kind, "shadow": shadow }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::Draft {
            outcome,
            insert: Some(Box::new(row)),
        }],
        rows,
        serde_json::json!({ "draft": draft_hex, "status": status }).to_string(),
        None,
    )
    .await
}

/// Store a `50101` after the item and every `rows` id resolve.
pub(super) async fn ingest_health(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("restricted: not the org agent".into()))?;
    authorize::require_agent(shapers.agent.as_deref(), &cmd.actor_hex)?;

    let read: HealthRead = serde_json::from_str(&cmd.event.content)
        .map_err(|e| IngestError::Rejected(format!("invalid: command content: {e}")))?;
    authorize::iso_week(&read.week)?;
    let item = uuid_tag(cmd.event, tag::ITEM)?.ok_or_else(|| invalid("missing i tag (item)"))?;
    if item.to_string() != read.item {
        return Err(invalid("item tag does not match content"));
    }
    if store::get_work_item(&mut tx, cmd.tenant.community(), item)
        .await
        .map_err(|e| internal("read io_work_items", e))?
        .is_none()
    {
        return Err(invalid("unknown item"));
    }

    let mut row_ids = Vec::new();
    for factor in &read.factors {
        row_ids.extend(factor.rows.iter().cloned());
    }
    for sentence in &read.sentences {
        row_ids.extend(sentence.rows.iter().cloned());
    }
    for id in &row_ids {
        let bytes = parse_hex32(id, "receipt")?;
        if !event_exists(&mut tx, cmd, &bytes).await? {
            return Err(invalid("unresolved receipt"));
        }
    }

    let row = HealthRow {
        event_id: cmd.receipt_bytes(),
        content: read.clone(),
        read_at: store::ts(cmd.at).map_err(|e| internal("health time", e))?,
    };
    let rows = vec![cmd.ledger(
        "health_read",
        object::WORK_ITEM,
        &read.item,
        serde_json::json!({ "week": read.week, "band": read.band }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::Health(row)],
        rows,
        serde_json::json!({ "item": read.item, "week": read.week }).to_string(),
        None,
    )
    .await
}

/// Store a `50103` from `39103.agent` with no receipt check, plus its ledger.
pub(super) async fn ingest_note(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("restricted: not the org agent".into()))?;
    authorize::require_agent(shapers.agent.as_deref(), &cmd.actor_hex)?;

    let note: AgentNote = serde_json::from_str(&cmd.event.content)
        .map_err(|e| IngestError::Rejected(format!("invalid: command content: {e}")))?;
    let t = tag_value(cmd.event, tag::TYPE).ok_or_else(|| invalid("draft shape"))?;
    let expected = serde_json::to_value(note.kind())
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned));
    if expected.as_deref() != Some(t) {
        return Err(invalid("draft shape"));
    }
    if let Some(gap) = tag_value(cmd.event, "gap") {
        if gap.is_empty() {
            return Err(invalid("draft shape"));
        }
    }
    if tag_value(cmd.event, tag::ITEM).is_some() {
        uuid_tag(cmd.event, tag::ITEM)?;
    }

    let (verb, object_id, detail) = match &note {
        AgentNote::DraftDropped {
            agent_move,
            gap,
            reason,
            ..
        } => (
            "draft_dropped",
            gap.clone(),
            serde_json::json!({ "move": agent_move, "gap": gap, "reason": reason }),
        ),
        AgentNote::TriggerSkipped {
            trigger,
            object,
            reason,
            ..
        } => (
            "trigger_skipped",
            object.clone(),
            serde_json::json!({ "trigger": trigger, "object": object, "reason": reason }),
        ),
        AgentNote::BudgetExhausted { budget, until } => (
            "budget_exhausted",
            budget.clone(),
            serde_json::json!({ "budget": budget, "until": until }),
        ),
        AgentNote::Tally { week, .. } => {
            ("tally", week.clone(), serde_json::json!({ "week": week }))
        }
    };
    let rows = vec![cmd.ledger(verb, object::AGENT, &object_id, detail)?];
    persist_write(
        cmd,
        tx,
        vec![],
        rows,
        serde_json::json!({ "note": verb }).to_string(),
        None,
    )
    .await
}

/// `io_draft_decide` (`50012`): standalone accept or decline. `e` is the
/// 64-hex draft id, read as a plain hex value (C-2 leftover).
pub(super) async fn decide(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let draft_id = hex32_tag(cmd.event, "e")?.ok_or_else(|| invalid("missing e tag (draft)"))?;
    let outcome = tag_value(cmd.event, "outcome").ok_or_else(|| invalid("missing outcome tag"))?;
    let outcome: DraftDecision =
        serde_json::from_value(serde_json::Value::String(outcome.to_owned()))
            .map_err(|_| invalid("outcome must be accept or decline"))?;
    let reason = tag_value(cmd.event, "reason")
        .map(|v| {
            serde_json::from_value::<DeclineReason>(serde_json::Value::String(v.to_owned()))
                .map_err(|_| invalid("unknown decline reason"))
        })
        .transpose()?;
    parse_content::<buzz_core::intelligent_org::EmptyContent>(cmd.event)?;
    authorize::draft_decide(outcome, reason)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let row = store::get_draft(&mut tx, cmd.tenant.community(), &draft_id)
        .await
        .map_err(|e| internal("read io_drafts", e))?
        .ok_or_else(|| invalid("unknown draft"))?;
    if row.outcome.status != DraftOutcomeStatus::Open {
        return Err(invalid("draft is not open"));
    }
    let shapers = current_shapers(&mut tx, cmd).await?;
    authorize::draft_needs_party(
        &row.needs,
        &cmd.actor_hex,
        shapers
            .as_ref()
            .is_some_and(|s| authorize::is_shaper(s, &cmd.actor_hex)),
    )?;

    let status = match outcome {
        DraftDecision::Accept => DraftOutcomeStatus::Accepted,
        DraftDecision::Decline => DraftOutcomeStatus::Declined,
    };
    let draft_hex = hex::encode(&draft_id);
    let decided = DraftOutcome {
        draft: draft_hex.clone(),
        status,
        decided_by: Some(cmd.actor_hex.clone()),
        decided_at: Some(cmd.at),
        reason,
        result: Some(cmd.receipt_hex()),
    };
    let rows = vec![cmd.ledger(
        "draft_decided",
        object::DRAFT,
        &draft_hex,
        serde_json::json!({ "status": status, "reason": reason }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::Draft {
            outcome: decided,
            insert: None,
        }],
        rows,
        serde_json::json!({ "draft": draft_hex, "status": status }).to_string(),
        None,
    )
    .await
}

/// `io_health_rate` (`50017`): a Shaper's blind band for an item and week.
pub(super) async fn health_rate(cmd: &Command<'_>) -> Result<IngestResult, IngestError> {
    let item = uuid_tag(cmd.event, tag::ITEM)?.ok_or_else(|| invalid("missing i tag (item)"))?;
    let week = tag_value(cmd.event, "week")
        .ok_or_else(|| invalid("missing week tag"))?
        .to_owned();
    authorize::iso_week(&week)?;
    let band = tag_value(cmd.event, "band").ok_or_else(|| invalid("missing band tag"))?;
    let band: HealthBand = serde_json::from_value(serde_json::Value::String(band.to_owned()))
        .map_err(|_| invalid("unknown band"))?;
    parse_content::<buzz_core::intelligent_org::EmptyContent>(cmd.event)?;

    let mut tx = match begin(cmd).await? {
        Persisted::Replay(result) => return Ok(result),
        Persisted::Open(tx) => tx,
    };
    let shapers = current_shapers(&mut tx, cmd)
        .await?
        .ok_or_else(|| IngestError::Rejected("restricted: not a Shaper".into()))?;
    authorize::require_shaper(&shapers, &cmd.actor_hex)?;
    if store::get_work_item(&mut tx, cmd.tenant.community(), item)
        .await
        .map_err(|e| internal("read io_work_items", e))?
        .is_none()
    {
        return Err(invalid("unknown item"));
    }
    let row = HealthRatingRow {
        item_id: item,
        week: week.clone(),
        rater: cmd.actor_bytes.clone(),
        band,
        receipt_event_id: cmd.receipt_bytes(),
        rated_at: store::ts(cmd.at).map_err(|e| internal("rating time", e))?,
    };
    let rows = vec![cmd.ledger(
        "health_rated",
        object::WORK_ITEM,
        &item.to_string(),
        serde_json::json!({ "week": week, "band": band }),
    )?];
    persist_write(
        cmd,
        tx,
        vec![Projection::HealthRating(row)],
        rows,
        serde_json::json!({ "item": item, "week": week }).to_string(),
        None,
    )
    .await
}

/// If the command carries `["e", id, "", "draft"]`, authorize the author as
/// the `needs` party and append the `39104` + `draft_decided` row. Equal
/// JSON → `accepted`; different → `amended`. A refusal leaves the draft
/// untouched because the caller's transaction has not committed.
pub(super) async fn attach_settlement(
    cmd: &Command<'_>,
    tx: &mut Transaction<'static, Postgres>,
    projections: &mut Vec<Projection>,
    rows: &mut Vec<LedgerEntry>,
) -> Result<(), IngestError> {
    if cmd.event.kind.as_u16() as u32 == KIND_IO_DRAFT_DECIDE {
        return Ok(());
    }
    let Some(draft_id) = draft_marker(cmd.event)? else {
        return Ok(());
    };
    let row = store::get_draft(tx, cmd.tenant.community(), &draft_id)
        .await
        .map_err(|e| internal("read io_drafts", e))?
        .ok_or_else(|| invalid("unknown draft"))?;
    if row.outcome.status != DraftOutcomeStatus::Open {
        return Err(invalid("draft is not open"));
    }
    let shapers = current_shapers(tx, cmd).await?;
    authorize::draft_needs_party(
        &row.needs,
        &cmd.actor_hex,
        shapers
            .as_ref()
            .is_some_and(|s| authorize::is_shaper(s, &cmd.actor_hex)),
    )?;
    let command = content_value(cmd.event)?;
    let status = if command == row.payload {
        DraftOutcomeStatus::Accepted
    } else {
        DraftOutcomeStatus::Amended
    };
    let draft_hex = hex::encode(&draft_id);
    let outcome = DraftOutcome {
        draft: draft_hex.clone(),
        status,
        decided_by: Some(cmd.actor_hex.clone()),
        decided_at: Some(cmd.at),
        reason: None,
        result: Some(cmd.receipt_hex()),
    };
    rows.push(cmd.ledger(
        "draft_decided",
        object::DRAFT,
        &draft_hex,
        serde_json::json!({ "status": status }),
    )?);
    projections.push(Projection::Draft {
        outcome,
        insert: None,
    });
    Ok(())
}
