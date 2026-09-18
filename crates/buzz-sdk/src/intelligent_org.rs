//! Intelligent organization — `build_io_*` event builders (Development plan C-1).
//!
//! One builder per person-signed command (`50001–50021`, Protocol §4.8) and
//! per agent- or person-signed draft and read (`50100–50103`, Protocol
//! §4.3, §4.7, §4.7b, §4.7c). Every builder is typed over the payload types
//! in [`buzz_core::intelligent_org`]: the content is the serialized type,
//! the tags are laid out exactly as the Protocol tables list them, and
//! anything the tags must agree with (a draft's `u`/`i`, its suggested
//! holder, a note's `move`/`gap`/`week`) is derived from the payload so the
//! two cannot drift.
//!
//! There is deliberately **no** builder for `39100–39105`: those are
//! relay-signed state and a client `EVENT` of them is refused
//! `restricted: relay-only kind`. `io_state_kinds_have_no_builder` in the
//! tests pins that every builder here yields a command or read kind.
//!
//! Every builder sets [`nostr::EventBuilder::allow_self_tagging`]. nostr's
//! builder otherwise drops a `p` tag naming the signer at signing time, and
//! several `io` events legitimately name their own author — the owner's
//! bootstrap `io_shapers_propose op=add`, a Shaper offering a root to
//! themselves, a member proposing themselves as DRI, a person-signed draft
//! addressed to its author, a holder's own progress note. A silently dropped
//! `p` becomes `invalid: op=add and op=remove need a p tag` at the relay
//! (progress log follow-up), so the flag is set uniformly rather than per
//! case.
//!
//! Like the rest of the crate: the caller signs (`builder.sign_with_keys`),
//! no keys are held here, no network is touched.

use buzz_core::intelligent_org::tag as io_tag;
use buzz_core::intelligent_org::{
    AgentNote, DeclineReason, DirectionProposeContent, DirectionSlug, DraftDecision, DraftOrigin,
    DraftPayload, EmptyContent, HealthBand, HealthRead, HolderMatch, JoinProposeContent,
    MoneyProposeContent, MoneyReleasedContent, ProfileSetContent, ProgressNote,
    ProjectProposeContent, RulesContent, ShapersOp, TicketCreateContent, Timestamp, VoteChoice,
    VoteContent, WhyContent, PROGRESS_MAX_COMMITS, PROGRESS_SUMMARY_MAX_CHARS,
};
use buzz_core::kind::{
    KIND_IO_ACCEPT, KIND_IO_AGENT_NOTE, KIND_IO_DECLINE, KIND_IO_DIRECTION_PROPOSE, KIND_IO_DONE,
    KIND_IO_DRAFT, KIND_IO_DRAFT_DECIDE, KIND_IO_DRI_PROPOSE, KIND_IO_HEALTH, KIND_IO_HEALTH_RATE,
    KIND_IO_JOIN_PROPOSE, KIND_IO_MONEY_PROPOSE, KIND_IO_MONEY_RELEASED, KIND_IO_OFFER,
    KIND_IO_PROFILE, KIND_IO_PROFILE_SET, KIND_IO_PROGRESS, KIND_IO_PROJECT_PROPOSE,
    KIND_IO_RELEASE, KIND_IO_REOPEN, KIND_IO_SET_DUE, KIND_IO_SHAPERS_PROPOSE,
    KIND_IO_SHAPER_ACCEPT, KIND_IO_SHAPER_STEP_DOWN, KIND_IO_TICKET_CREATE, KIND_IO_VOTE,
};
use nostr::{EventBuilder, EventId, Kind, Tag};
use serde::Serialize;
use uuid::Uuid;

use crate::builders::{check_content, check_pubkey_hex, tag};
use crate::SdkError;

/// Content cap shared with the crate's other builders.
const MAX_CONTENT_BYTES: usize = 64 * 1024;

/// The lowest and highest agent move (`["move", "1".."4"]`).
const MOVE_RANGE: std::ops::RangeInclusive<u8> = 1..=4;

// ── shared helpers ───────────────────────────────────────────────────────────

/// Serialize a `#[serde(rename_all)]` unit enum to its wire string.
fn enum_str<T: Serialize>(value: &T, field: &str) -> Result<String, SdkError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        _ => Err(SdkError::InvalidInput(format!(
            "{field} did not serialize to a string"
        ))),
    }
}

/// Serialize a content type and cap it like every other builder's content.
fn content_json<T: Serialize>(content: &T) -> Result<String, SdkError> {
    let json = serde_json::to_string(content)
        .map_err(|e| SdkError::InvalidInput(format!("content did not serialize: {e}")))?;
    check_content(&json, MAX_CONTENT_BYTES)?;
    Ok(json)
}

/// Parse a payload field that the Protocol says is a UUID.
fn check_uuid(value: &str, field: &str) -> Result<Uuid, SdkError> {
    Uuid::parse_str(value)
        .map_err(|_| SdkError::InvalidInput(format!("{field} must be a lowercase RFC 4122 uuid")))
}

fn check_non_empty(value: &str, field: &str) -> Result<(), SdkError> {
    if value.trim().is_empty() {
        return Err(SdkError::InvalidInput(format!("{field} must not be empty")));
    }
    Ok(())
}

/// `YYYY-Www` — an ISO week such as `2026-W38`.
fn check_iso_week(week: &str) -> Result<(), SdkError> {
    let bytes = week.as_bytes();
    let well_formed = bytes.len() == 8
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && &bytes[4..6] == b"-W"
        && bytes[6..].iter().all(u8::is_ascii_digit)
        && matches!(week[6..].parse::<u8>(), Ok(1..=53));
    if !well_formed {
        return Err(SdkError::InvalidInput(format!(
            "week must be an ISO week like 2026-W38 (got {week:?})"
        )));
    }
    Ok(())
}

fn check_move(agent_move: u8) -> Result<String, SdkError> {
    if !MOVE_RANGE.contains(&agent_move) {
        return Err(SdkError::InvalidInput(format!(
            "move must be within 1..=4 (got {agent_move})"
        )));
    }
    Ok(agent_move.to_string())
}

/// `["e", <draft>, "", "draft"]` — the command settles that draft (§3.2).
fn draft_tag(draft: EventId) -> Result<Tag, SdkError> {
    tag(&["e", &draft.to_hex(), "", io_tag::MARKER_DRAFT])
}

/// `["vote", "agree"]` — the opener's own vote riding on the proposal (D1).
fn opener_vote_tag() -> Result<Tag, SdkError> {
    tag(&["vote", &enum_str(&VoteChoice::Agree, "vote")?])
}

fn item_tag(item: Uuid) -> Result<Tag, SdkError> {
    tag(&[io_tag::ITEM, &item.to_string()])
}

fn pubkey_tag(pubkey: &str, field: &str) -> Result<Tag, SdkError> {
    tag(&["p", &check_pubkey_hex(pubkey, field)?])
}

/// Every `io` builder ends here: the kind, the tags in Protocol order, the
/// content, and `allow_self_tagging` (module docs).
fn io_event(kind: u32, tags: Vec<Tag>, content: String) -> EventBuilder {
    EventBuilder::new(Kind::Custom(kind as u16), content)
        .tags(tags)
        .allow_self_tagging()
}

// ── §4.8 commands ────────────────────────────────────────────────────────────

/// What an `io_shapers_propose` (`50001`) proposes. One variant per `op`, so
/// the tag and the content type cannot disagree: `add`/`remove` carry a `p`
/// and `{ why? }`; `rules` carries `{ rules, … }` and no `p`; `agent` names
/// a self-run agent or, with `pubkey: None`, returns to the hosted default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapersProposal<'a> {
    /// `op=add`: offer a seat to `pubkey`. The community owner's bootstrap
    /// is this variant naming themselves (Protocol §6.4).
    Add {
        /// The person offered the seat.
        pubkey: &'a str,
        /// `{ why?: "" }`.
        why: WhyContent,
    },
    /// `op=remove`: remove `pubkey` from the Shaper set.
    Remove {
        /// The Shaper to remove.
        pubkey: &'a str,
        /// `{ why?: "" }`.
        why: WhyContent,
    },
    /// `op=rules`: replace the decision rules and windows; always passes under `all`.
    Rules(RulesContent),
    /// `op=agent`: name a self-run org agent, or `None` to return to the
    /// operator's hosted default; always passes under `all`.
    Agent {
        /// The new agent, which must already be a non-Shaper member; `None` = hosted.
        pubkey: Option<&'a str>,
        /// `{ why?: "" }`.
        why: WhyContent,
    },
}

impl ShapersProposal<'_> {
    /// The `op` tag value.
    pub fn op(&self) -> ShapersOp {
        match self {
            Self::Add { .. } => ShapersOp::Add,
            Self::Remove { .. } => ShapersOp::Remove,
            Self::Rules(_) => ShapersOp::Rules,
            Self::Agent { .. } => ShapersOp::Agent,
        }
    }
}

/// `io_shapers_propose` (`50001`): `["op", …]`, `["p", <pubkey>]` for
/// add/remove and for agent when naming one, `["vote", "agree"]` when
/// `vote_agree` (D1). Content per [`ShapersProposal`].
pub fn build_io_shapers_propose(
    proposal: &ShapersProposal<'_>,
    vote_agree: bool,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![tag(&["op", &enum_str(&proposal.op(), "op")?])?];
    let content = match proposal {
        ShapersProposal::Add { pubkey, why } | ShapersProposal::Remove { pubkey, why } => {
            tags.push(pubkey_tag(pubkey, "p")?);
            content_json(why)?
        }
        ShapersProposal::Rules(rules) => content_json(rules)?,
        ShapersProposal::Agent { pubkey, why } => {
            if let Some(pubkey) = pubkey {
                tags.push(pubkey_tag(pubkey, "p")?);
            }
            content_json(why)?
        }
    };
    if vote_agree {
        tags.push(opener_vote_tag()?);
    }
    Ok(io_event(KIND_IO_SHAPERS_PROPOSE, tags, content))
}

/// `io_direction_propose` (`50002`): `["d", <slug>]`, `["base", "<version>"]`,
/// `["e", <draft>, "", "draft"]`?, `["vote", "agree"]`?. Content is the
/// whole new version.
pub fn build_io_direction_propose(
    slug: DirectionSlug,
    base_version: u32,
    content: &DirectionProposeContent,
    draft: Option<EventId>,
    vote_agree: bool,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![
        tag(&["d", &enum_str(&slug, "d")?])?,
        tag(&["base", &base_version.to_string()])?,
    ];
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    if vote_agree {
        tags.push(opener_vote_tag()?);
    }
    Ok(io_event(
        KIND_IO_DIRECTION_PROPOSE,
        tags,
        content_json(content)?,
    ))
}

/// `io_vote` (`50003`): `["e", <proposal-uuid>]`, `["vote", "agree"|"decline"]`.
pub fn build_io_vote(
    proposal: Uuid,
    vote: VoteChoice,
    content: &VoteContent,
) -> Result<EventBuilder, SdkError> {
    let tags = vec![
        tag(&["e", &proposal.to_string()])?,
        tag(&["vote", &enum_str(&vote, "vote")?])?,
    ];
    Ok(io_event(KIND_IO_VOTE, tags, content_json(content)?))
}

/// `io_shaper_accept` (`50019`): `["e", <proposal-uuid>]`, content `{}` —
/// take the seat the passed `shapers/add` offered.
pub fn build_io_shaper_accept(proposal: Uuid) -> Result<EventBuilder, SdkError> {
    let tags = vec![tag(&["e", &proposal.to_string()])?];
    Ok(io_event(
        KIND_IO_SHAPER_ACCEPT,
        tags,
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_shaper_step_down` (`50020`): no tags, `{ why?: "" }`.
pub fn build_io_shaper_step_down(content: &WhyContent) -> Result<EventBuilder, SdkError> {
    Ok(io_event(
        KIND_IO_SHAPER_STEP_DOWN,
        Vec::new(),
        content_json(content)?,
    ))
}

/// `io_project_propose` (`50004`): `["e", <draft>, "", "draft"]`?,
/// `["vote", "agree"]`?. Content `{ title, brief, due_at, objective_ref?, suggested_dri? }`.
pub fn build_io_project_propose(
    content: &ProjectProposeContent,
    draft: Option<EventId>,
    vote_agree: bool,
) -> Result<EventBuilder, SdkError> {
    check_non_empty(&content.title, "title")?;
    if let Some(dri) = &content.suggested_dri {
        check_pubkey_hex(dri, "suggested_dri")?;
    }
    let mut tags = Vec::new();
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    if vote_agree {
        tags.push(opener_vote_tag()?);
    }
    Ok(io_event(
        KIND_IO_PROJECT_PROPOSE,
        tags,
        content_json(content)?,
    ))
}

/// `io_ticket_create` (`50005`): `["u", <parent-uuid>]`, `["p", <offer_to>]`?,
/// `["e", <draft>, "", "draft"]`?. Content `{ title, brief, due_at, after? }`.
/// With `offer_to` the child opens `offered`; without, `open`.
pub fn build_io_ticket_create(
    parent: Uuid,
    offer_to: Option<&str>,
    content: &TicketCreateContent,
    draft: Option<EventId>,
) -> Result<EventBuilder, SdkError> {
    check_non_empty(&content.title, "title")?;
    for sibling in &content.after {
        check_uuid(sibling, "after[]")?;
    }
    let mut tags = vec![tag(&[io_tag::PARENT, &parent.to_string()])?];
    if let Some(offer_to) = offer_to {
        tags.push(pubkey_tag(offer_to, "p")?);
    }
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    Ok(io_event(
        KIND_IO_TICKET_CREATE,
        tags,
        content_json(content)?,
    ))
}

/// `io_offer` (`50006`): `["i", <uuid>]`, `["p", <pubkey>]`,
/// `["e", <draft>, "", "draft"]`?, content `{}`.
pub fn build_io_offer(
    item: Uuid,
    to: &str,
    draft: Option<EventId>,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![item_tag(item)?, pubkey_tag(to, "p")?];
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    Ok(io_event(
        KIND_IO_OFFER,
        tags,
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_accept` (`50007`): `["i", <uuid>]`, content `{}`.
pub fn build_io_accept(item: Uuid) -> Result<EventBuilder, SdkError> {
    Ok(io_event(
        KIND_IO_ACCEPT,
        vec![item_tag(item)?],
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_decline` (`50008`): `["i", <uuid>]`, content `{}` — no reason is
/// recorded; a decline is private to the person.
pub fn build_io_decline(item: Uuid) -> Result<EventBuilder, SdkError> {
    Ok(io_event(
        KIND_IO_DECLINE,
        vec![item_tag(item)?],
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_done` (`50009`): `["i", <uuid>]`, `["e", <message-id>, "", "receipt"]`?,
/// `["e", <draft>, "", "draft"]`?, content `{}`. `receipt` is the chat
/// message a done was relayed from (§5.5).
pub fn build_io_done(
    item: Uuid,
    receipt: Option<EventId>,
    draft: Option<EventId>,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![item_tag(item)?];
    if let Some(receipt) = receipt {
        tags.push(tag(&["e", &receipt.to_hex(), "", io_tag::MARKER_RECEIPT])?);
    }
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    Ok(io_event(
        KIND_IO_DONE,
        tags,
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_release` (`50010`): `["i", <uuid>]`, `{ why?: "" }`.
pub fn build_io_release(item: Uuid, content: &WhyContent) -> Result<EventBuilder, SdkError> {
    Ok(io_event(
        KIND_IO_RELEASE,
        vec![item_tag(item)?],
        content_json(content)?,
    ))
}

/// `io_set_due` (`50011`): `["i", <uuid>]`, `["due", "<ts>"]`, `{ why?: "" }`.
pub fn build_io_set_due(
    item: Uuid,
    due_at: Timestamp,
    content: &WhyContent,
) -> Result<EventBuilder, SdkError> {
    let tags = vec![item_tag(item)?, tag(&["due", &due_at.to_string()])?];
    Ok(io_event(KIND_IO_SET_DUE, tags, content_json(content)?))
}

/// `io_reopen` (`50018`): `["i", <uuid>]`, `{ why?: "" }` — undo a done
/// within 7 days.
pub fn build_io_reopen(item: Uuid, content: &WhyContent) -> Result<EventBuilder, SdkError> {
    Ok(io_event(
        KIND_IO_REOPEN,
        vec![item_tag(item)?],
        content_json(content)?,
    ))
}

/// `io_draft_decide` (`50012`): `["e", <draft>]`, `["outcome", "accept"|"decline"]`,
/// `["reason", <reason>]`?, content `{}`. The `e` tag here is plain — this
/// command *is* the decision, it does not settle the draft through another
/// command, so it carries no `draft` marker.
pub fn build_io_draft_decide(
    draft: EventId,
    outcome: DraftDecision,
    reason: Option<DeclineReason>,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![
        tag(&["e", &draft.to_hex()])?,
        tag(&["outcome", &enum_str(&outcome, "outcome")?])?,
    ];
    if let Some(reason) = reason {
        tags.push(tag(&["reason", &enum_str(&reason, "reason")?])?);
    }
    Ok(io_event(
        KIND_IO_DRAFT_DECIDE,
        tags,
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_money_propose` (`50013`, reserved — the relay refuses it
/// `restricted: money not enabled`): `["i", <uuid>]`, `["p", <payee>]`,
/// `["e", <draft>, "", "draft"]`?. Content `{ amount, currency, note?, agreed? }`.
pub fn build_io_money_propose(
    item: Uuid,
    payee: &str,
    content: &MoneyProposeContent,
    draft: Option<EventId>,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![item_tag(item)?, pubkey_tag(payee, "p")?];
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    Ok(io_event(
        KIND_IO_MONEY_PROPOSE,
        tags,
        content_json(content)?,
    ))
}

/// `io_money_released` (`50014`, reserved — signed by the treasury bridge
/// key, never a CLI verb): `["e", <proposal-uuid>]`, `["tx", <chain-tx-id>]`.
/// Content `{ chain, contract, amount, currency }`.
pub fn build_io_money_released(
    proposal: Uuid,
    tx: &str,
    content: &MoneyReleasedContent,
) -> Result<EventBuilder, SdkError> {
    check_non_empty(tx, "tx")?;
    let tags = vec![tag(&["e", &proposal.to_string()])?, tag(&["tx", tx])?];
    Ok(io_event(
        KIND_IO_MONEY_RELEASED,
        tags,
        content_json(content)?,
    ))
}

/// `io_dri_propose` (`50015`): `["i", <uuid>]`, `["p", <pubkey>]`,
/// `["e", <draft>, "", "draft"]`?, `["vote", "agree"]`?, `{ why?: "" }`.
pub fn build_io_dri_propose(
    item: Uuid,
    pubkey: &str,
    content: &WhyContent,
    draft: Option<EventId>,
    vote_agree: bool,
) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![item_tag(item)?, pubkey_tag(pubkey, "p")?];
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    if vote_agree {
        tags.push(opener_vote_tag()?);
    }
    Ok(io_event(KIND_IO_DRI_PROPOSE, tags, content_json(content)?))
}

/// `io_join_propose` (`50016`, reserved — the relay refuses it
/// `restricted: join not enabled`): `["p", <pubkey>]`, `{ note?: "" }`.
pub fn build_io_join_propose(
    pubkey: &str,
    content: &JoinProposeContent,
) -> Result<EventBuilder, SdkError> {
    let tags = vec![pubkey_tag(pubkey, "p")?];
    Ok(io_event(KIND_IO_JOIN_PROPOSE, tags, content_json(content)?))
}

/// `io_health_rate` (`50017`): `["i", <uuid>]`, `["week", "<iso-week>"]`,
/// `["band", <band>]`, content `{}` — a Shaper's blind band for one week.
pub fn build_io_health_rate(
    item: Uuid,
    week: &str,
    band: HealthBand,
) -> Result<EventBuilder, SdkError> {
    check_iso_week(week)?;
    let tags = vec![
        item_tag(item)?,
        tag(&["week", week])?,
        tag(&["band", &enum_str(&band, "band")?])?,
    ];
    Ok(io_event(
        KIND_IO_HEALTH_RATE,
        tags,
        content_json(&EmptyContent::default())?,
    ))
}

/// `io_profile_set` (`50021`): `["e", <draft>, "", "draft"]`?. Content is
/// the whole profile `{ about, skills, open_limit? }`; the subject is the
/// signer and is never a tag.
pub fn build_io_profile_set(
    content: &ProfileSetContent,
    draft: Option<EventId>,
) -> Result<EventBuilder, SdkError> {
    let mut tags = Vec::new();
    if let Some(draft) = draft {
        tags.push(draft_tag(draft)?);
    }
    Ok(io_event(KIND_IO_PROFILE_SET, tags, content_json(content)?))
}

// ── §4.3 kind:50100 — draft ──────────────────────────────────────────────────

/// The one party that may act on a draft: the `n` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftNeeds<'a> {
    /// `["n", "shaper"]` — any Shaper; no `p` tag, the inbox resolves Shapers from `39103`.
    Shaper,
    /// `["n", <pubkey>]` plus `["p", <pubkey>, "", "needs"]` so the mention index files it.
    Member(&'a str),
}

/// One receipt on a draft (§4.3: at least one is required).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftReceipt<'a> {
    /// `["e", <event-id>, "", "receipt"]` — a message, command, or state event.
    Event(EventId),
    /// `["a", <coord>, "", "receipt"]` — an addressable coordinate, e.g. a
    /// `39105` profile (see [`profile_coordinate`]).
    Coordinate(&'a str),
    /// `["ref", <line-ref>]` — an objective line, `objectives@3#l_7f3a`.
    LineRef(&'a str),
}

/// Optional provenance tags on `50100` and `50101` (Org agent § 8.4, § 16):
/// the prompt version and model that produced it, and the agent's job id.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Provenance<'a> {
    /// `["prompt", "<job>@<version>"]`.
    pub prompt: Option<&'a str>,
    /// `["model", "<id>"]`.
    pub model: Option<&'a str>,
    /// `["trace", "<id>"]`.
    pub trace: Option<&'a str>,
}

impl Provenance<'_> {
    fn push_tags(&self, tags: &mut Vec<Tag>) -> Result<(), SdkError> {
        for (name, value) in [
            ("prompt", self.prompt),
            ("model", self.model),
            ("trace", self.trace),
        ] {
            if let Some(value) = value {
                check_non_empty(value, name)?;
                tags.push(tag(&[name, value])?);
            }
        }
        Ok(())
    }
}

/// Everything a `50100` carries besides its payload.
///
/// The `t`, `u`/`i`, suggested-holder `p`, and matched-skill `k` tags are
/// derived from `payload`, so the tags always describe the content that
/// travels with them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoDraft<'a> {
    /// Who may act.
    pub needs: DraftNeeds<'a>,
    /// The typed payload; its variant is the `t` tag.
    pub payload: &'a DraftPayload,
    /// Which agent move produced it, `1..=4`.
    pub agent_move: u8,
    /// Heard in a room, or drafted from a gap.
    pub origin: DraftOrigin,
    /// Dedupe key: an objective line ref, an item uuid, or `<item-uuid>#<covers-slug>`.
    pub gap: &'a str,
    /// At least one.
    pub receipts: &'a [DraftReceipt<'a>],
    /// Shown to nobody; recorded for evaluation only.
    pub shadow: bool,
    /// NIP-40 cleanup for drafts nobody decides.
    pub expiration: Option<Timestamp>,
    /// Prompt, model, trace.
    pub provenance: Provenance<'a>,
}

/// The `a` coordinate of a member's `39105` org profile, for a
/// [`DraftReceipt::Coordinate`] on a draft that suggests them as holder.
pub fn profile_coordinate(relay_pubkey: &str, member_pubkey: &str) -> Result<String, SdkError> {
    Ok(format!(
        "{}:{}:{}",
        KIND_IO_PROFILE,
        check_pubkey_hex(relay_pubkey, "relay_pubkey")?,
        check_pubkey_hex(member_pubkey, "member_pubkey")?
    ))
}

/// The suggested holder a payload names, with the fit it cites, if any.
fn suggested_holder(payload: &DraftPayload) -> (Option<&str>, Option<&HolderMatch>) {
    match payload {
        DraftPayload::Project(p) => (p.suggested_dri.as_deref(), p.matched.as_ref()),
        DraftPayload::Ticket(t) => (t.suggested_holder.as_deref(), t.matched.as_ref()),
        DraftPayload::Dri(d) => (Some(d.suggested.as_str()), Some(&d.matched)),
        _ => (None, None),
    }
}

/// `io_draft` (`50100`): the §4.3 tag set in table order — `n`, `t`, `move`,
/// `origin`, `gap`, `u` (ticket/done) or `i` (dri/review/money), `p … needs`,
/// `p … suggested`, receipts, matched `k` slugs, `shadow`, `expiration`,
/// provenance. Content is the payload for its `t`.
pub fn build_io_draft(draft: &IoDraft<'_>) -> Result<EventBuilder, SdkError> {
    let kind = draft.payload.kind();
    check_non_empty(draft.gap, "gap")?;
    if draft.receipts.is_empty() {
        return Err(SdkError::InvalidInput(
            "a draft needs at least one receipt".into(),
        ));
    }

    let mut tags = Vec::new();
    let needs_pubkey = match draft.needs {
        DraftNeeds::Shaper => {
            tags.push(tag(&[io_tag::NEEDS, io_tag::NEEDS_SHAPER])?);
            None
        }
        DraftNeeds::Member(pubkey) => {
            let pubkey = check_pubkey_hex(pubkey, "needs")?;
            tags.push(tag(&[io_tag::NEEDS, &pubkey])?);
            Some(pubkey)
        }
    };
    if let (DraftPayload::Profile(profile), Some(needs)) = (draft.payload, &needs_pubkey) {
        if !profile.pubkey.eq_ignore_ascii_case(needs) {
            return Err(SdkError::InvalidInput(
                "a profile draft must be addressed to its subject".into(),
            ));
        }
    }
    tags.push(tag(&[io_tag::TYPE, &enum_str(&kind, "t")?])?);
    tags.push(tag(&["move", &check_move(draft.agent_move)?])?);
    tags.push(tag(&["origin", &enum_str(&draft.origin, "origin")?])?);
    tags.push(tag(&["gap", draft.gap])?);

    match draft.payload {
        DraftPayload::Ticket(t) => tags.push(tag(&[
            io_tag::PARENT,
            &check_uuid(&t.parent, "parent")?.to_string(),
        ])?),
        DraftPayload::Done(d) => tags.push(tag(&[
            io_tag::PARENT,
            &check_uuid(&d.item, "item")?.to_string(),
        ])?),
        DraftPayload::Dri(d) => tags.push(item_tag(check_uuid(&d.item, "item")?)?),
        DraftPayload::Review(r) => tags.push(item_tag(check_uuid(&r.item, "item")?)?),
        DraftPayload::Money(m) => tags.push(item_tag(check_uuid(&m.item, "item")?)?),
        DraftPayload::Project(_)
        | DraftPayload::Objectives(_)
        | DraftPayload::Direction(_)
        | DraftPayload::Profile(_) => {}
    }

    if let Some(pubkey) = &needs_pubkey {
        tags.push(tag(&["p", pubkey, "", io_tag::MARKER_NEEDS])?);
    }
    let (suggested, matched) = suggested_holder(draft.payload);
    if let Some(suggested) = suggested {
        tags.push(tag(&[
            "p",
            &check_pubkey_hex(suggested, "suggested holder")?,
            "",
            io_tag::MARKER_SUGGESTED,
        ])?);
    }

    for receipt in draft.receipts {
        tags.push(match receipt {
            DraftReceipt::Event(id) => tag(&["e", &id.to_hex(), "", io_tag::MARKER_RECEIPT])?,
            DraftReceipt::Coordinate(coord) => {
                check_non_empty(coord, "receipt coordinate")?;
                tag(&["a", coord, "", io_tag::MARKER_RECEIPT])?
            }
            DraftReceipt::LineRef(line_ref) => {
                check_non_empty(line_ref, "receipt ref")?;
                tag(&["ref", line_ref])?
            }
        });
    }
    if suggested.is_some() {
        for slug in matched.map(|m| m.skills.as_slice()).unwrap_or_default() {
            check_non_empty(slug, "matched skill")?;
            tags.push(tag(&[io_tag::SKILL, slug])?);
        }
    }

    if draft.shadow {
        tags.push(tag(&["shadow", "true"])?);
    }
    if let Some(expiration) = draft.expiration {
        tags.push(tag(&["expiration", &expiration.to_string()])?);
    }
    draft.provenance.push_tags(&mut tags)?;

    Ok(io_event(KIND_IO_DRAFT, tags, content_json(draft.payload)?))
}

// ── §4.7 kind:50101 — health read ────────────────────────────────────────────

/// `io_health` (`50101`): `["i", <uuid>]`, `["week", "<iso-week>"]`,
/// `["band", <band>]`, then provenance. Every tag is read from `read`.
pub fn build_io_health(
    read: &HealthRead,
    provenance: Provenance<'_>,
) -> Result<EventBuilder, SdkError> {
    check_iso_week(&read.week)?;
    let mut tags = vec![
        item_tag(check_uuid(&read.item, "item")?)?,
        tag(&["week", &read.week])?,
        tag(&["band", &enum_str(&read.band, "band")?])?,
    ];
    provenance.push_tags(&mut tags)?;
    Ok(io_event(KIND_IO_HEALTH, tags, content_json(read)?))
}

// ── §4.7b kind:50102 — progress note ─────────────────────────────────────────

/// `io_progress` (`50102`): `["i", <uuid>]`, `["p", <dri>]`, `["ref", <git-ref>]`,
/// one `["commit", <sha>]` per commit in `note.commits` (newest first, at
/// most [`PROGRESS_MAX_COMMITS`]), `["hint", <hint>]`, and
/// `["e", <previous 50102>, "", "prev"]` when `prev` is given.
///
/// `head_verified` and `merged_into` are relay-set; a note that carries
/// either is refused here rather than silently stripped.
pub fn build_io_progress(
    note: &ProgressNote,
    prev: Option<EventId>,
) -> Result<EventBuilder, SdkError> {
    if note.head_verified.is_some() || note.merged_into.is_some() {
        return Err(SdkError::InvalidInput(
            "head_verified and merged_into are relay-set; a client note must not carry them".into(),
        ));
    }
    if note.commits.len() > PROGRESS_MAX_COMMITS {
        return Err(SdkError::InvalidInput(format!(
            "a progress note covers at most {PROGRESS_MAX_COMMITS} commits (got {})",
            note.commits.len()
        )));
    }
    let summary_chars = note.summary.chars().count();
    if summary_chars > PROGRESS_SUMMARY_MAX_CHARS {
        return Err(SdkError::InvalidInput(format!(
            "summary exceeds {PROGRESS_SUMMARY_MAX_CHARS} chars (got {summary_chars})"
        )));
    }
    check_non_empty(&note.git_ref, "ref")?;
    check_non_empty(&note.head, "head")?;

    let mut tags = vec![
        item_tag(check_uuid(&note.item, "item")?)?,
        pubkey_tag(&note.dri, "dri")?,
        tag(&["ref", &note.git_ref])?,
    ];
    for commit in &note.commits {
        check_non_empty(&commit.sha, "commit sha")?;
        tags.push(tag(&["commit", &commit.sha])?);
    }
    tags.push(tag(&["hint", &enum_str(&note.hint, "hint")?])?);
    if let Some(prev) = prev {
        tags.push(tag(&["e", &prev.to_hex(), "", "prev"])?);
    }
    Ok(io_event(KIND_IO_PROGRESS, tags, content_json(note)?))
}

// ── §4.7c kind:50103 — agent note ────────────────────────────────────────────

/// `io_agent_note` (`50103`): `["t", <note>]`, then per variant `["move", n]`,
/// `["gap", <gap-key>]`, `["week", <iso-week>]`, `["trace", <id>]`, all read
/// from `note`; `["i", <uuid>]` when the note is about an item.
pub fn build_io_agent_note(note: &AgentNote, item: Option<Uuid>) -> Result<EventBuilder, SdkError> {
    let mut tags = vec![tag(&[io_tag::TYPE, &enum_str(&note.kind(), "t")?])?];
    let trace = match note {
        AgentNote::DraftDropped {
            agent_move,
            gap,
            trace,
            ..
        } => {
            tags.push(tag(&["move", &check_move(*agent_move)?])?);
            check_non_empty(gap, "gap")?;
            tags.push(tag(&["gap", gap])?);
            trace.as_deref()
        }
        AgentNote::TriggerSkipped {
            agent_move, trace, ..
        } => {
            tags.push(tag(&["move", &check_move(*agent_move)?])?);
            trace.as_deref()
        }
        AgentNote::BudgetExhausted { .. } => None,
        AgentNote::Tally { week, .. } => {
            check_iso_week(week)?;
            None
        }
    };
    if let Some(item) = item {
        tags.push(item_tag(item)?);
    }
    if let AgentNote::Tally { week, .. } = note {
        tags.push(tag(&["week", week])?);
    }
    if let Some(trace) = trace {
        check_non_empty(trace, "trace")?;
        tags.push(tag(&["trace", trace])?);
    }
    Ok(io_event(KIND_IO_AGENT_NOTE, tags, content_json(note)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::intelligent_org::{
        AgreedAmount, CommitRef, CoveragePiece, DecisionRule, DecisionRules, DirectionLineInput,
        DoneDraft, DraftKind, DriDraft, HealthFactor, HealthTally, MoneyDraft, ObjectivesDraft,
        ProfileDraft, ProgressHint, ProjectDraft, ReviewDraft, ReviewRecommendation, TicketDraft,
        VoteContent,
    };
    use buzz_core::kind::{
        is_command_kind, is_intelligent_org_command_kind, is_intelligent_org_read_kind,
        is_intelligent_org_state_kind, is_relay_only_kind, IO_STATE_KIND_MAX, IO_STATE_KIND_MIN,
    };
    use nostr::{Event, Keys};
    use serde_json::{json, Value};
    use std::collections::BTreeSet;

    const PK: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const PK2: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const RELAY: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const EV: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const EV2: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const ID: &str = "7f3a0000-0000-4000-8000-000000000001";
    const ID2: &str = "7f3a0000-0000-4000-8000-000000000002";

    fn ev(hex: &str) -> EventId {
        EventId::from_hex(hex).expect("event id")
    }

    fn id(s: &str) -> Uuid {
        Uuid::parse_str(s).expect("uuid")
    }

    fn sign(builder: EventBuilder) -> Event {
        builder.sign_with_keys(&Keys::generate()).expect("sign")
    }

    fn tags(event: &Event) -> Vec<Vec<String>> {
        event.tags.iter().map(|t| t.as_slice().to_vec()).collect()
    }

    fn strs(parts: &[&[&str]]) -> Vec<Vec<String>> {
        parts
            .iter()
            .map(|p| p.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    fn content(event: &Event) -> Value {
        serde_json::from_str(&event.content).expect("content is json")
    }

    fn why(s: &str) -> WhyContent {
        WhyContent {
            why: Some(s.into()),
        }
    }

    fn project_content() -> ProjectProposeContent {
        ProjectProposeContent {
            title: "Weekday hall".into(),
            brief: "md".into(),
            due_at: 1_785_000_000,
            objective_ref: Some("objectives@3#l_7f3a".into()),
            suggested_dri: Some(PK.into()),
        }
    }

    fn ticket_content() -> TicketCreateContent {
        TicketCreateContent {
            title: "Permit".into(),
            brief: "md".into(),
            due_at: 1_780_000_000,
            after: vec![ID2.into()],
        }
    }

    fn ticket_draft() -> DraftPayload {
        DraftPayload::Ticket(TicketDraft {
            parent: ID.into(),
            title: "Permit".into(),
            brief: "md".into(),
            due_at: 1,
            requires: vec!["grant-writing".into()],
            suggested_holder: Some(PK2.into()),
            unfilled: None,
            covers: "get the permit".into(),
            after: vec![],
            gate: true,
            coverage: vec![CoveragePiece {
                piece: "permit".into(),
                covered_by: None,
                order: 1,
                after: vec![],
                held: None,
            }],
            matched: Some(HolderMatch {
                skills: vec!["grant-writing".into(), "spanish".into()],
                about: None,
                items: vec![],
            }),
        })
    }

    fn health_read() -> HealthRead {
        HealthRead {
            item: ID.into(),
            week: "2026-W38".into(),
            pct: 0.62,
            band: HealthBand::Wobbly,
            factors: vec![HealthFactor {
                name: "overdue".into(),
                value: 2.0,
                weight: 0.25,
                rows: vec![EV.into()],
            }],
            sentences: vec![],
            formula: "health-weights@1".into(),
        }
    }

    fn progress_note(dri: &str) -> ProgressNote {
        ProgressNote {
            item: ID.into(),
            dri: dri.into(),
            from: 1,
            to: 2,
            summary: "Booking form landed.".into(),
            hint: ProgressHint::Ready,
            git_ref: "refs/heads/io/7f3a-weekday-hall".into(),
            head: "abc123".into(),
            commits: vec![
                CommitRef {
                    sha: "abc123".into(),
                    title: "Add booking form".into(),
                },
                CommitRef {
                    sha: "def456".into(),
                    title: "Scaffold".into(),
                },
            ],
            files_changed: 7,
            uncommitted: None,
            head_verified: None,
            merged_into: None,
        }
    }

    fn receipts() -> Vec<DraftReceipt<'static>> {
        vec![DraftReceipt::Event(ev(EV))]
    }

    fn draft<'a>(
        needs: DraftNeeds<'a>,
        payload: &'a DraftPayload,
        receipts: &'a [DraftReceipt<'a>],
    ) -> IoDraft<'a> {
        IoDraft {
            needs,
            payload,
            agent_move: 2,
            origin: DraftOrigin::Gap,
            gap: "7f3a0000-0000-4000-8000-000000000001#permit",
            receipts,
            shadow: false,
            expiration: None,
            provenance: Provenance::default(),
        }
    }

    /// One sample of every builder in this module — the enumeration the
    /// kind-coverage test binds to. Adding a builder means adding a row.
    fn every_builder() -> Vec<(&'static str, EventBuilder)> {
        let item = id(ID);
        let proposal = id(ID2);
        let d = Some(ev(EV));
        let payload = DraftPayload::Project(ProjectDraft {
            title: "t".into(),
            brief: "b".into(),
            objective_ref: None,
            due_at: 0,
            suggested_dri: None,
            why: "w".into(),
            gaps: vec![],
            matched: None,
        });
        let rcpts = receipts();
        let note = AgentNote::BudgetExhausted {
            budget: "calls_per_hour".into(),
            until: 1,
        };
        vec![
            (
                "shapers_propose",
                build_io_shapers_propose(
                    &ShapersProposal::Add {
                        pubkey: PK,
                        why: WhyContent::default(),
                    },
                    false,
                )
                .unwrap(),
            ),
            (
                "direction_propose",
                build_io_direction_propose(
                    DirectionSlug::Mission,
                    1,
                    &DirectionProposeContent {
                        body: "b".into(),
                        lines: None,
                        why: None,
                    },
                    None,
                    false,
                )
                .unwrap(),
            ),
            (
                "vote",
                build_io_vote(proposal, VoteChoice::Agree, &VoteContent::default()).unwrap(),
            ),
            ("shaper_accept", build_io_shaper_accept(proposal).unwrap()),
            (
                "shaper_step_down",
                build_io_shaper_step_down(&WhyContent::default()).unwrap(),
            ),
            (
                "project_propose",
                build_io_project_propose(&project_content(), None, false).unwrap(),
            ),
            (
                "ticket_create",
                build_io_ticket_create(item, None, &ticket_content(), None).unwrap(),
            ),
            ("offer", build_io_offer(item, PK, None).unwrap()),
            ("accept", build_io_accept(item).unwrap()),
            ("decline", build_io_decline(item).unwrap()),
            ("done", build_io_done(item, None, None).unwrap()),
            (
                "release",
                build_io_release(item, &WhyContent::default()).unwrap(),
            ),
            (
                "set_due",
                build_io_set_due(item, 1, &WhyContent::default()).unwrap(),
            ),
            (
                "reopen",
                build_io_reopen(item, &WhyContent::default()).unwrap(),
            ),
            (
                "draft_decide",
                build_io_draft_decide(ev(EV), DraftDecision::Accept, None).unwrap(),
            ),
            (
                "money_propose",
                build_io_money_propose(
                    item,
                    PK,
                    &MoneyProposeContent {
                        amount: "1".into(),
                        currency: "USDC".into(),
                        note: None,
                        agreed: None,
                    },
                    d,
                )
                .unwrap(),
            ),
            (
                "money_released",
                build_io_money_released(
                    proposal,
                    "0xabc",
                    &MoneyReleasedContent {
                        chain: "base".into(),
                        contract: "0x1".into(),
                        amount: "1".into(),
                        currency: "USDC".into(),
                    },
                )
                .unwrap(),
            ),
            (
                "dri_propose",
                build_io_dri_propose(item, PK, &WhyContent::default(), None, false).unwrap(),
            ),
            (
                "join_propose",
                build_io_join_propose(PK, &JoinProposeContent::default()).unwrap(),
            ),
            (
                "health_rate",
                build_io_health_rate(item, "2026-W38", HealthBand::Healthy).unwrap(),
            ),
            (
                "profile_set",
                build_io_profile_set(&ProfileSetContent::default(), None).unwrap(),
            ),
            (
                "draft",
                build_io_draft(&draft(DraftNeeds::Shaper, &payload, &rcpts)).unwrap(),
            ),
            (
                "health",
                build_io_health(&health_read(), Provenance::default()).unwrap(),
            ),
            (
                "progress",
                build_io_progress(&progress_note(PK), None).unwrap(),
            ),
            ("agent_note", build_io_agent_note(&note, None).unwrap()),
        ]
    }

    // ── Proves: a builder cannot produce a 39100–39105 ──────────────────────

    #[test]
    fn io_state_kinds_have_no_builder() {
        let mut kinds = BTreeSet::new();
        for (name, builder) in every_builder() {
            let kind = u32::from(sign(builder).kind.as_u16());
            assert!(
                !is_intelligent_org_state_kind(kind) && !is_relay_only_kind(kind),
                "{name} produced relay-only kind {kind}"
            );
            assert!(
                !(IO_STATE_KIND_MIN..=IO_STATE_KIND_MAX).contains(&kind),
                "{name} produced a kind in the reserved state range: {kind}"
            );
            assert!(
                is_intelligent_org_command_kind(kind) || is_intelligent_org_read_kind(kind),
                "{name} produced {kind}, neither a command nor a read"
            );
            if is_intelligent_org_command_kind(kind) {
                assert!(
                    is_command_kind(kind),
                    "{name}: {kind} must route to the executor"
                );
            }
            assert!(kinds.insert(kind), "{name} duplicates kind {kind}");
        }
        // Every command kind and every read kind has exactly one builder.
        let expected: BTreeSet<u32> = (KIND_IO_SHAPERS_PROPOSE..=KIND_IO_PROFILE_SET)
            .chain(KIND_IO_DRAFT..=KIND_IO_AGENT_NOTE)
            .collect();
        assert_eq!(kinds, expected);
        assert_eq!(kinds.len(), 21 + 4);
    }

    #[test]
    fn every_io_builder_allows_self_tagging() {
        for (name, builder) in every_builder() {
            assert!(builder.allow_self_tagging, "{name} would drop a self `p`");
        }
    }

    // ── Proves: tag sets per command (Protocol §4.8) ────────────────────────

    #[test]
    fn shapers_propose_tag_sets() {
        let add = sign(
            build_io_shapers_propose(
                &ShapersProposal::Add {
                    pubkey: PK,
                    why: why("founder"),
                },
                false,
            )
            .unwrap(),
        );
        assert_eq!(add.kind.as_u16(), 50001);
        assert_eq!(tags(&add), strs(&[&["op", "add"], &["p", PK]]));
        assert_eq!(content(&add), json!({ "why": "founder" }));
        // The relay reads it back as WhyContent.
        let parsed: WhyContent = serde_json::from_str(&add.content).unwrap();
        assert_eq!(parsed.why.as_deref(), Some("founder"));

        let remove = sign(
            build_io_shapers_propose(
                &ShapersProposal::Remove {
                    pubkey: PK2,
                    why: WhyContent::default(),
                },
                true,
            )
            .unwrap(),
        );
        assert_eq!(
            tags(&remove),
            strs(&[&["op", "remove"], &["p", PK2], &["vote", "agree"]])
        );
        assert_eq!(content(&remove), json!({}));

        let rules = sign(
            build_io_shapers_propose(
                &ShapersProposal::Rules(RulesContent {
                    rules: DecisionRules {
                        shapers: DecisionRule::ALL,
                        dri: DecisionRule::AtLeast(2),
                        ..DecisionRules::default()
                    },
                    decision_window_secs: Some(86_400),
                    offer_window_secs: None,
                }),
                false,
            )
            .unwrap(),
        );
        assert_eq!(tags(&rules), strs(&[&["op", "rules"]]));
        let parsed: RulesContent = serde_json::from_str(&rules.content).unwrap();
        assert_eq!(parsed.rules.shapers, DecisionRule::ALL);
        assert_eq!(parsed.rules.dri, DecisionRule::AtLeast(2));
        assert_eq!(parsed.decision_window_secs, Some(86_400));
        assert!(content(&rules).get("offer_window_secs").is_none());

        let agent = sign(
            build_io_shapers_propose(
                &ShapersProposal::Agent {
                    pubkey: Some(PK2),
                    why: WhyContent::default(),
                },
                false,
            )
            .unwrap(),
        );
        assert_eq!(tags(&agent), strs(&[&["op", "agent"], &["p", PK2]]));
        let hosted = sign(
            build_io_shapers_propose(
                &ShapersProposal::Agent {
                    pubkey: None,
                    why: why("back to hosted"),
                },
                false,
            )
            .unwrap(),
        );
        assert_eq!(tags(&hosted), strs(&[&["op", "agent"]]));

        // A malformed p never leaves the builder.
        assert!(matches!(
            build_io_shapers_propose(
                &ShapersProposal::Add {
                    pubkey: "not-a-pubkey",
                    why: WhyContent::default()
                },
                false
            ),
            Err(SdkError::InvalidInput(_))
        ));
    }

    #[test]
    fn shapers_propose_lowercases_p_for_the_relay() {
        // handlers/intelligent_org/mod.rs::pubkey_tag accepts lowercase hex only.
        let upper = PK.to_ascii_uppercase();
        let e = sign(
            build_io_shapers_propose(
                &ShapersProposal::Add {
                    pubkey: &upper,
                    why: WhyContent::default(),
                },
                false,
            )
            .unwrap(),
        );
        assert_eq!(tags(&e)[1], vec!["p", PK]);
    }

    #[test]
    fn direction_propose_tag_set() {
        let c = DirectionProposeContent {
            body: "Weekday hall booked".into(),
            lines: Some(vec![
                DirectionLineInput {
                    id: Some("l_7f3a".into()),
                    text: "kept".into(),
                    date: None,
                },
                DirectionLineInput {
                    id: None,
                    text: "new".into(),
                    date: Some(1),
                },
            ]),
            why: Some("w".into()),
        };
        let e = sign(
            build_io_direction_propose(DirectionSlug::Objectives, 3, &c, Some(ev(EV)), true)
                .unwrap(),
        );
        assert_eq!(e.kind.as_u16(), 50002);
        assert_eq!(
            tags(&e),
            strs(&[
                &["d", "objectives"],
                &["base", "3"],
                &["e", EV, "", "draft"],
                &["vote", "agree"],
            ])
        );
        let parsed: DirectionProposeContent = serde_json::from_str(&e.content).unwrap();
        assert_eq!(parsed, c);

        let bare = sign(
            build_io_direction_propose(
                DirectionSlug::Mission,
                1,
                &DirectionProposeContent {
                    body: "b".into(),
                    lines: None,
                    why: None,
                },
                None,
                false,
            )
            .unwrap(),
        );
        assert_eq!(tags(&bare), strs(&[&["d", "mission"], &["base", "1"]]));
        assert_eq!(content(&bare), json!({ "body": "b" }));
    }

    #[test]
    fn vote_accept_and_step_down_tag_sets() {
        let vote = sign(
            build_io_vote(
                id(ID2),
                VoteChoice::Decline,
                &VoteContent {
                    reason: Some("too vague".into()),
                },
            )
            .unwrap(),
        );
        assert_eq!(vote.kind.as_u16(), 50003);
        assert_eq!(tags(&vote), strs(&[&["e", ID2], &["vote", "decline"]]));
        assert_eq!(content(&vote), json!({ "reason": "too vague" }));

        let accept = sign(build_io_shaper_accept(id(ID2)).unwrap());
        assert_eq!(accept.kind.as_u16(), 50019);
        assert_eq!(tags(&accept), strs(&[&["e", ID2]]));
        assert_eq!(accept.content, "{}");
        // handlers/intelligent_org/shapers.rs::accept reads the e tag with uuid_tag.
        assert_eq!(Uuid::parse_str(&tags(&accept)[0][1]).unwrap(), id(ID2));

        let down = sign(build_io_shaper_step_down(&why("moving on")).unwrap());
        assert_eq!(down.kind.as_u16(), 50020);
        assert!(tags(&down).is_empty());
        assert_eq!(content(&down), json!({ "why": "moving on" }));
        let silent = sign(build_io_shaper_step_down(&WhyContent::default()).unwrap());
        assert_eq!(silent.content, "{}");
    }

    #[test]
    fn project_and_ticket_tag_sets() {
        let plain = sign(build_io_project_propose(&project_content(), None, false).unwrap());
        assert_eq!(plain.kind.as_u16(), 50004);
        assert!(tags(&plain).is_empty());
        assert_eq!(
            content(&plain),
            json!({ "title": "Weekday hall", "brief": "md", "due_at": 1785000000,
                    "objective_ref": "objectives@3#l_7f3a", "suggested_dri": PK })
        );
        let settled =
            sign(build_io_project_propose(&project_content(), Some(ev(EV)), true).unwrap());
        assert_eq!(
            tags(&settled),
            strs(&[&["e", EV, "", "draft"], &["vote", "agree"]])
        );
        let mut bad = project_content();
        bad.suggested_dri = Some("nope".into());
        assert!(build_io_project_propose(&bad, None, false).is_err());
        let mut untitled = project_content();
        untitled.title = "  ".into();
        assert!(build_io_project_propose(&untitled, None, false).is_err());

        let open = sign(build_io_ticket_create(id(ID), None, &ticket_content(), None).unwrap());
        assert_eq!(open.kind.as_u16(), 50005);
        assert_eq!(tags(&open), strs(&[&["u", ID]]));
        assert_eq!(
            content(&open),
            json!({ "title": "Permit", "brief": "md", "due_at": 1780000000, "after": [ID2] })
        );
        let offered = sign(
            build_io_ticket_create(id(ID), Some(PK2), &ticket_content(), Some(ev(EV))).unwrap(),
        );
        assert_eq!(
            tags(&offered),
            strs(&[&["u", ID], &["p", PK2], &["e", EV, "", "draft"]])
        );
        let mut bad_after = ticket_content();
        bad_after.after = vec!["not-a-uuid".into()];
        assert!(build_io_ticket_create(id(ID), None, &bad_after, None).is_err());
    }

    #[test]
    fn work_item_command_tag_sets() {
        let item = id(ID);
        let offer = sign(build_io_offer(item, PK2, Some(ev(EV))).unwrap());
        assert_eq!(offer.kind.as_u16(), 50006);
        assert_eq!(
            tags(&offer),
            strs(&[&["i", ID], &["p", PK2], &["e", EV, "", "draft"]])
        );
        assert_eq!(offer.content, "{}");

        for (kind, e) in [
            (50007, sign(build_io_accept(item).unwrap())),
            (50008, sign(build_io_decline(item).unwrap())),
            (50009, sign(build_io_done(item, None, None).unwrap())),
        ] {
            assert_eq!(e.kind.as_u16(), kind);
            assert_eq!(tags(&e), strs(&[&["i", ID]]), "kind {kind}");
            assert_eq!(e.content, "{}", "kind {kind}");
        }

        let relayed = sign(build_io_done(item, Some(ev(EV2)), Some(ev(EV))).unwrap());
        assert_eq!(
            tags(&relayed),
            strs(&[
                &["i", ID],
                &["e", EV2, "", "receipt"],
                &["e", EV, "", "draft"],
            ])
        );

        for (kind, e) in [
            (50010, sign(build_io_release(item, &why("w")).unwrap())),
            (50018, sign(build_io_reopen(item, &why("w")).unwrap())),
        ] {
            assert_eq!(e.kind.as_u16(), kind);
            assert_eq!(tags(&e), strs(&[&["i", ID]]), "kind {kind}");
            assert_eq!(content(&e), json!({ "why": "w" }), "kind {kind}");
        }

        let due = sign(build_io_set_due(item, 1_785_000_000, &WhyContent::default()).unwrap());
        assert_eq!(due.kind.as_u16(), 50011);
        assert_eq!(tags(&due), strs(&[&["i", ID], &["due", "1785000000"]]));
        assert_eq!(due.content, "{}");
    }

    #[test]
    fn draft_decide_tag_set() {
        let decline = sign(
            build_io_draft_decide(ev(EV), DraftDecision::Decline, Some(DeclineReason::NotNow))
                .unwrap(),
        );
        assert_eq!(decline.kind.as_u16(), 50012);
        assert_eq!(
            tags(&decline),
            strs(&[&["e", EV], &["outcome", "decline"], &["reason", "not_now"]])
        );
        assert_eq!(decline.content, "{}");
        // The e tag is plain: this command does not settle the draft through
        // another command, so mod.rs::has_draft_tag must not fire on it.
        assert!(tags(&decline).iter().all(|t| t.len() < 4));

        let accept = sign(build_io_draft_decide(ev(EV), DraftDecision::Accept, None).unwrap());
        assert_eq!(tags(&accept), strs(&[&["e", EV], &["outcome", "accept"]]));
        let wrong_holder = sign(
            build_io_draft_decide(
                ev(EV),
                DraftDecision::Decline,
                Some(DeclineReason::NotWhatTheLineMeant),
            )
            .unwrap(),
        );
        assert_eq!(
            tags(&wrong_holder)[2],
            vec!["reason", "not_what_the_line_meant"]
        );
    }

    #[test]
    fn reserved_money_and_join_tag_sets() {
        let propose = sign(
            build_io_money_propose(
                id(ID),
                PK2,
                &MoneyProposeContent {
                    amount: "150".into(),
                    currency: "USDC".into(),
                    note: None,
                    agreed: Some(AgreedAmount {
                        amount: "150".into(),
                        heard: EV2.into(),
                    }),
                },
                Some(ev(EV)),
            )
            .unwrap(),
        );
        assert_eq!(propose.kind.as_u16(), 50013);
        assert_eq!(
            tags(&propose),
            strs(&[&["i", ID], &["p", PK2], &["e", EV, "", "draft"]])
        );
        assert_eq!(
            content(&propose),
            json!({ "amount": "150", "currency": "USDC", "agreed": { "amount": "150", "heard": EV2 } })
        );

        let released = sign(
            build_io_money_released(
                id(ID2),
                "0xdeadbeef",
                &MoneyReleasedContent {
                    chain: "base".into(),
                    contract: "0x1".into(),
                    amount: "150".into(),
                    currency: "USDC".into(),
                },
            )
            .unwrap(),
        );
        assert_eq!(released.kind.as_u16(), 50014);
        assert_eq!(tags(&released), strs(&[&["e", ID2], &["tx", "0xdeadbeef"]]));
        assert_eq!(
            content(&released),
            json!({ "chain": "base", "contract": "0x1", "amount": "150", "currency": "USDC" })
        );
        assert!(build_io_money_released(
            id(ID2),
            "",
            &MoneyReleasedContent {
                chain: "b".into(),
                contract: "c".into(),
                amount: "1".into(),
                currency: "U".into(),
            },
        )
        .is_err());

        let join = sign(
            build_io_join_propose(
                PK2,
                &JoinProposeContent {
                    note: Some("met at the stall".into()),
                },
            )
            .unwrap(),
        );
        assert_eq!(join.kind.as_u16(), 50016);
        assert_eq!(tags(&join), strs(&[&["p", PK2]]));
        assert_eq!(content(&join), json!({ "note": "met at the stall" }));
    }

    #[test]
    fn dri_propose_health_rate_and_profile_set_tag_sets() {
        let dri = sign(
            build_io_dri_propose(
                id(ID),
                PK2,
                &why("she ran it last year"),
                Some(ev(EV)),
                true,
            )
            .unwrap(),
        );
        assert_eq!(dri.kind.as_u16(), 50015);
        assert_eq!(
            tags(&dri),
            strs(&[
                &["i", ID],
                &["p", PK2],
                &["e", EV, "", "draft"],
                &["vote", "agree"],
            ])
        );
        assert_eq!(content(&dri), json!({ "why": "she ran it last year" }));

        let rate = sign(build_io_health_rate(id(ID), "2026-W38", HealthBand::Struggling).unwrap());
        assert_eq!(rate.kind.as_u16(), 50017);
        assert_eq!(
            tags(&rate),
            strs(&[&["i", ID], &["week", "2026-W38"], &["band", "struggling"]])
        );
        assert_eq!(rate.content, "{}");
        for bad in ["2026-38", "2026-W54", "2026-W00", "26-W38", "2026-W3"] {
            assert!(
                build_io_health_rate(id(ID), bad, HealthBand::Healthy).is_err(),
                "{bad} should be rejected"
            );
        }

        let profile = ProfileSetContent {
            about: "I run the Tuesday kitchen.".into(),
            skills: vec!["hosting events".into(), "spanish".into()],
            open_limit: Some(3),
        };
        let set = sign(build_io_profile_set(&profile, Some(ev(EV))).unwrap());
        assert_eq!(set.kind.as_u16(), 50021);
        assert_eq!(tags(&set), strs(&[&["e", EV, "", "draft"]]));
        assert_eq!(
            content(&set),
            json!({ "about": "I run the Tuesday kitchen.", "skills": ["hosting events", "spanish"], "open_limit": 3 })
        );
        // `pubkey` is the signer, never a tag or a field.
        let plain = sign(build_io_profile_set(&profile, None).unwrap());
        assert!(tags(&plain).is_empty());
        assert!(!plain.content.contains("pubkey"));
    }

    // ── §4.3 draft ──────────────────────────────────────────────────────────

    #[test]
    fn draft_tag_set_in_protocol_order_for_a_ticket_with_a_suggested_holder() {
        let payload = ticket_draft();
        let coord = profile_coordinate(RELAY, PK2).unwrap();
        assert_eq!(coord, format!("39105:{RELAY}:{PK2}"));
        let rcpts = vec![
            DraftReceipt::Event(ev(EV)),
            DraftReceipt::Coordinate(&coord),
            DraftReceipt::LineRef("objectives@3#l_7f3a"),
        ];
        let mut d = draft(DraftNeeds::Member(PK), &payload, &rcpts);
        d.shadow = true;
        d.expiration = Some(1_790_000_000);
        d.provenance = Provenance {
            prompt: Some("2-project-to-tickets@3"),
            model: Some("m"),
            trace: Some("job-1"),
        };
        let e = sign(build_io_draft(&d).unwrap());
        assert_eq!(e.kind.as_u16(), 50100);
        assert_eq!(
            tags(&e),
            strs(&[
                &["n", PK],
                &["t", "ticket"],
                &["move", "2"],
                &["origin", "gap"],
                &["gap", "7f3a0000-0000-4000-8000-000000000001#permit"],
                &["u", ID],
                &["p", PK, "", "needs"],
                &["p", PK2, "", "suggested"],
                &["e", EV, "", "receipt"],
                &["a", &coord, "", "receipt"],
                &["ref", "objectives@3#l_7f3a"],
                &["k", "grant-writing"],
                &["k", "spanish"],
                &["shadow", "true"],
                &["expiration", "1790000000"],
                &["prompt", "2-project-to-tickets@3"],
                &["model", "m"],
                &["trace", "job-1"],
            ])
        );
        // Content is the payload for the `t`, and parses back through it.
        let parsed = DraftPayload::parse(DraftKind::Ticket, &e.content).unwrap();
        assert_eq!(&parsed, &payload);
    }

    #[test]
    fn draft_u_and_i_tags_follow_the_payload_kind() {
        let rcpts = receipts();
        let dri = DraftPayload::Dri(DriDraft {
            item: ID.into(),
            suggested: PK2.into(),
            why: "w".into(),
            matched: HolderMatch {
                skills: vec!["rust".into()],
                about: None,
                items: vec![],
            },
            evidence: vec![],
        });
        let e = sign(build_io_draft(&draft(DraftNeeds::Shaper, &dri, &rcpts)).unwrap());
        assert_eq!(
            tags(&e),
            strs(&[
                &["n", "shaper"],
                &["t", "dri"],
                &["move", "2"],
                &["origin", "gap"],
                &["gap", "7f3a0000-0000-4000-8000-000000000001#permit"],
                &["i", ID],
                &["p", PK2, "", "suggested"],
                &["e", EV, "", "receipt"],
                &["k", "rust"],
            ])
        );

        let done = DraftPayload::Done(DoneDraft {
            item: ID.into(),
            why: "last child closed".into(),
            heard: None,
        });
        let e = sign(build_io_draft(&draft(DraftNeeds::Member(PK), &done, &rcpts)).unwrap());
        assert_eq!(tags(&e)[1], vec!["t", "done"]);
        assert_eq!(tags(&e)[5], vec!["u", ID]);
        assert_eq!(tags(&e)[6], vec!["p", PK, "", "needs"]);
        assert!(!tags(&e).iter().any(|t| t[0] == "i"));

        let review = DraftPayload::Review(ReviewDraft {
            item: ID.into(),
            brief: vec![],
            recommendation: ReviewRecommendation::NoFurtherWork { why: "done".into() },
        });
        let e = sign(build_io_draft(&draft(DraftNeeds::Shaper, &review, &rcpts)).unwrap());
        assert_eq!(tags(&e)[5], vec!["i", ID]);

        let money = DraftPayload::Money(MoneyDraft {
            item: ID.into(),
            payee: PK.into(),
            amount: "1".into(),
            currency: "USDC".into(),
            agreed: None,
        });
        let e = sign(build_io_draft(&draft(DraftNeeds::Shaper, &money, &rcpts)).unwrap());
        assert_eq!(tags(&e)[5], vec!["i", ID]);

        // Kinds about no item carry neither, and a shaper draft has no `p`.
        let objectives = DraftPayload::Objectives(ObjectivesDraft {
            base_version: 3,
            ops: vec![],
        });
        let e = sign(build_io_draft(&draft(DraftNeeds::Shaper, &objectives, &rcpts)).unwrap());
        assert_eq!(
            tags(&e),
            strs(&[
                &["n", "shaper"],
                &["t", "objectives"],
                &["move", "2"],
                &["origin", "gap"],
                &["gap", "7f3a0000-0000-4000-8000-000000000001#permit"],
                &["e", EV, "", "receipt"],
            ])
        );
    }

    #[test]
    fn draft_rejects_shapes_the_relay_would_refuse() {
        let payload = ticket_draft();
        let rcpts = receipts();
        let no_receipt = draft(DraftNeeds::Shaper, &payload, &[]);
        assert!(matches!(
            build_io_draft(&no_receipt),
            Err(SdkError::InvalidInput(_))
        ));
        let mut bad_move = draft(DraftNeeds::Shaper, &payload, &rcpts);
        bad_move.agent_move = 5;
        assert!(build_io_draft(&bad_move).is_err());
        let mut no_gap = draft(DraftNeeds::Shaper, &payload, &rcpts);
        no_gap.gap = "";
        assert!(build_io_draft(&no_gap).is_err());
        let bad_needs = draft(DraftNeeds::Member("me"), &payload, &rcpts);
        assert!(build_io_draft(&bad_needs).is_err());

        let profile = DraftPayload::Profile(ProfileDraft {
            pubkey: PK.into(),
            about: "a".into(),
            skills: vec![],
            open_limit: None,
            heard: vec![],
        });
        assert!(build_io_draft(&draft(DraftNeeds::Member(PK2), &profile, &rcpts)).is_err());
        let addressed =
            sign(build_io_draft(&draft(DraftNeeds::Member(PK), &profile, &rcpts)).unwrap());
        assert_eq!(tags(&addressed)[1], vec!["t", "profile"]);
        assert_eq!(tags(&addressed)[5], vec!["p", PK, "", "needs"]);
    }

    // ── §4.7, §4.7b, §4.7c reads ────────────────────────────────────────────

    #[test]
    fn health_read_tag_set_comes_from_the_read() {
        let read = health_read();
        let e = sign(
            build_io_health(
                &read,
                Provenance {
                    prompt: Some("4-health@1"),
                    model: None,
                    trace: Some("t"),
                },
            )
            .unwrap(),
        );
        assert_eq!(e.kind.as_u16(), 50101);
        assert_eq!(
            tags(&e),
            strs(&[
                &["i", ID],
                &["week", "2026-W38"],
                &["band", "wobbly"],
                &["prompt", "4-health@1"],
                &["trace", "t"],
            ])
        );
        let parsed: HealthRead = serde_json::from_str(&e.content).unwrap();
        assert_eq!(parsed, read);
        let mut bad = health_read();
        bad.week = "week 38".into();
        assert!(build_io_health(&bad, Provenance::default()).is_err());
        let mut bad_item = health_read();
        bad_item.item = "x".into();
        assert!(build_io_health(&bad_item, Provenance::default()).is_err());
    }

    #[test]
    fn progress_note_tag_set_and_client_field_guard() {
        let note = progress_note(PK2);
        let e = sign(build_io_progress(&note, Some(ev(EV2))).unwrap());
        assert_eq!(e.kind.as_u16(), 50102);
        assert_eq!(
            tags(&e),
            strs(&[
                &["i", ID],
                &["p", PK2],
                &["ref", "refs/heads/io/7f3a-weekday-hall"],
                &["commit", "abc123"],
                &["commit", "def456"],
                &["hint", "ready"],
                &["e", EV2, "", "prev"],
            ])
        );
        let parsed: ProgressNote = serde_json::from_str(&e.content).unwrap();
        assert_eq!(parsed, note);
        assert!(!e.content.contains("head_verified"));

        let mut relay_set = progress_note(PK2);
        relay_set.head_verified = Some(true);
        assert!(build_io_progress(&relay_set, None).is_err());
        let mut merged = progress_note(PK2);
        merged.merged_into = Some("main".into());
        assert!(build_io_progress(&merged, None).is_err());
        let mut too_many = progress_note(PK2);
        too_many.commits = (0..=PROGRESS_MAX_COMMITS)
            .map(|i| CommitRef {
                sha: format!("{i:040x}"),
                title: String::new(),
            })
            .collect();
        assert!(build_io_progress(&too_many, None).is_err());
        let mut long = progress_note(PK2);
        long.summary = "é".repeat(PROGRESS_SUMMARY_MAX_CHARS + 1);
        assert!(build_io_progress(&long, None).is_err());
        long.summary = "é".repeat(PROGRESS_SUMMARY_MAX_CHARS);
        assert!(
            build_io_progress(&long, None).is_ok(),
            "limit is chars, not bytes"
        );
    }

    #[test]
    fn agent_note_tag_set_per_variant() {
        let dropped = AgentNote::DraftDropped {
            agent_move: 2,
            gap: "7f3a#rota".into(),
            reason: "unmatched_skill".into(),
            kind: DraftKind::Ticket,
            needs: PK.into(),
            trace: Some("job-9".into()),
        };
        let e = sign(build_io_agent_note(&dropped, Some(id(ID))).unwrap());
        assert_eq!(e.kind.as_u16(), 50103);
        assert_eq!(
            tags(&e),
            strs(&[
                &["t", "draft_dropped"],
                &["move", "2"],
                &["gap", "7f3a#rota"],
                &["i", ID],
                &["trace", "job-9"],
            ])
        );
        assert_eq!(content(&e)["note"], json!("draft_dropped"));
        assert_eq!(content(&e)["move"], json!(2));
        let parsed: AgentNote = serde_json::from_str(&e.content).unwrap();
        assert_eq!(parsed, dropped);

        let skipped = AgentNote::TriggerSkipped {
            agent_move: 1,
            trigger: "direction_confirmed".into(),
            object: "objectives@4".into(),
            reason: "model_timeout".into(),
            trace: None,
        };
        let e = sign(build_io_agent_note(&skipped, None).unwrap());
        assert_eq!(tags(&e), strs(&[&["t", "trigger_skipped"], &["move", "1"]]));

        let budget = AgentNote::BudgetExhausted {
            budget: "tokens_per_day".into(),
            until: 1,
        };
        let e = sign(build_io_agent_note(&budget, None).unwrap());
        assert_eq!(tags(&e), strs(&[&["t", "budget_exhausted"]]));

        let tally = AgentNote::Tally {
            week: "2026-W38".into(),
            window_weeks: 4,
            moves: Default::default(),
            health: HealthTally::default(),
            open_older_than_5d: 0,
            receipt_rejected: 0,
        };
        let e = sign(build_io_agent_note(&tally, None).unwrap());
        assert_eq!(tags(&e), strs(&[&["t", "tally"], &["week", "2026-W38"]]));

        let bad_week = AgentNote::Tally {
            week: "38".into(),
            window_weeks: 4,
            moves: Default::default(),
            health: HealthTally::default(),
            open_older_than_5d: 0,
            receipt_rejected: 0,
        };
        assert!(build_io_agent_note(&bad_week, None).is_err());
        let bad_move = AgentNote::TriggerSkipped {
            agent_move: 0,
            trigger: "t".into(),
            object: "o".into(),
            reason: "r".into(),
            trace: None,
        };
        assert!(build_io_agent_note(&bad_move, None).is_err());
    }

    // ── Follow-up: a `p` naming the signer survives signing ────────────────

    #[test]
    fn self_add_bootstrap_keeps_its_p_tag() {
        // The owner's bootstrap io_shapers_propose op=add names the owner.
        // nostr's builder drops a self `p` by default, which the relay then
        // refuses as `invalid: op=add and op=remove need a p tag`.
        let owner = Keys::generate();
        let me = owner.public_key().to_hex();
        let e = build_io_shapers_propose(
            &ShapersProposal::Add {
                pubkey: &me,
                why: WhyContent::default(),
            },
            false,
        )
        .unwrap()
        .sign_with_keys(&owner)
        .unwrap();
        assert_eq!(tags(&e), strs(&[&["op", "add"], &["p", &me]]));

        // Control: the same tags through nostr's default builder lose the p,
        // which is exactly the failure the flag prevents.
        let stripped = EventBuilder::new(Kind::Custom(50001), "{}")
            .tags([tag(&["op", "add"]).unwrap(), tag(&["p", &me]).unwrap()])
            .sign_with_keys(&owner)
            .unwrap();
        assert_eq!(tags(&stripped), strs(&[&["op", "add"]]));
    }

    #[test]
    fn every_builder_whose_p_may_be_the_signer_keeps_it() {
        let keys = Keys::generate();
        let me = keys.public_key().to_hex();
        let item = id(ID);
        let self_draft_payload = DraftPayload::Done(DoneDraft {
            item: ID.into(),
            why: "heard on a call".into(),
            heard: None,
        });
        let rcpts = receipts();
        let self_draft = draft(DraftNeeds::Member(&me), &self_draft_payload, &rcpts);
        let cases: Vec<(&str, EventBuilder, Vec<&str>)> = vec![
            (
                "50001 remove",
                build_io_shapers_propose(
                    &ShapersProposal::Remove {
                        pubkey: &me,
                        why: WhyContent::default(),
                    },
                    false,
                )
                .unwrap(),
                vec!["p", &me],
            ),
            (
                "50001 agent",
                build_io_shapers_propose(
                    &ShapersProposal::Agent {
                        pubkey: Some(&me),
                        why: WhyContent::default(),
                    },
                    false,
                )
                .unwrap(),
                vec!["p", &me],
            ),
            (
                "50005",
                build_io_ticket_create(item, Some(&me), &ticket_content(), None).unwrap(),
                vec!["p", &me],
            ),
            (
                "50006",
                build_io_offer(item, &me, None).unwrap(),
                vec!["p", &me],
            ),
            (
                "50013",
                build_io_money_propose(
                    item,
                    &me,
                    &MoneyProposeContent {
                        amount: "1".into(),
                        currency: "USDC".into(),
                        note: None,
                        agreed: None,
                    },
                    None,
                )
                .unwrap(),
                vec!["p", &me],
            ),
            (
                "50015",
                build_io_dri_propose(item, &me, &WhyContent::default(), None, false).unwrap(),
                vec!["p", &me],
            ),
            (
                "50016",
                build_io_join_propose(&me, &JoinProposeContent::default()).unwrap(),
                vec!["p", &me],
            ),
            (
                "50100 needs=me",
                build_io_draft(&self_draft).unwrap(),
                vec!["p", &me, "", "needs"],
            ),
            (
                "50102 dri=me",
                build_io_progress(&progress_note(&me), None).unwrap(),
                vec!["p", &me],
            ),
        ];
        for (name, builder, expected) in cases {
            let e = builder.sign_with_keys(&keys).unwrap();
            let expected: Vec<String> = expected.into_iter().map(String::from).collect();
            assert!(
                tags(&e).contains(&expected),
                "{name}: self p tag {expected:?} was dropped; tags = {:?}",
                tags(&e)
            );
        }
    }
}
