//! Relay-signed state events (`39100–39105`, Protocol §3.1, §4).
//!
//! One builder per state kind turns the canonical content type from
//! [`buzz_core::intelligent_org`] into an unsigned [`StateDraft`] carrying the
//! exact tag set the Protocol fixes for that kind — the tags the doors filter
//! on (`#d`, `#p`, `#s`, `#t`, `#k`). [`sign`] then stamps it with the
//! relay's key at a `created_at` the executor chose (see
//! [`super::apply`]: strictly after the previous head of the same coordinate).
//!
//! Builders are pure; they never touch the database. A builder fails only when
//! the content holds a string that cannot become a tag, which is a programming
//! error upstream, not user input.

use buzz_core::intelligent_org::{
    tag, DirectionArtifact, DraftOutcome, OrgProfile, Proposal, ProposalKind, Shapers, WorkItem,
};
use buzz_core::kind::{
    KIND_IO_DIRECTION, KIND_IO_DRAFT_OUTCOME, KIND_IO_PROFILE, KIND_IO_PROPOSAL, KIND_IO_SHAPERS,
    KIND_IO_WORK_ITEM,
};
use nostr::{Event, EventBuilder, Keys, Kind, Tag, Timestamp};

use crate::handlers::ingest::IngestError;

/// The `d` tag of the community's single `39103`.
pub const SHAPERS_D_TAG: &str = "shapers";

/// An unsigned state event: kind, addressable coordinate, tags, and content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDraft {
    /// One of `39100–39105`.
    pub kind: u32,
    /// The NIP-33 `d` tag — also the first tag in `tags`.
    pub d_tag: String,
    /// The full tag set, `d` first.
    pub tags: Vec<Tag>,
    /// Canonical §4 content, serialized.
    pub content: String,
}

fn tag_of<const N: usize>(parts: [&str; N]) -> Result<Tag, IngestError> {
    Tag::parse(parts).map_err(|e| IngestError::Internal(format!("error: state tag: {e}")))
}

fn content_of<T: serde::Serialize>(value: &T) -> Result<String, IngestError> {
    serde_json::to_string(value)
        .map_err(|e| IngestError::Internal(format!("error: state content: {e}")))
}

fn wire<T: serde::Serialize>(value: &T) -> Result<String, IngestError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        Ok(other) => Err(IngestError::Internal(format!(
            "error: state tag expects a string enum, got {other}"
        ))),
        Err(e) => Err(IngestError::Internal(format!("error: state tag: {e}"))),
    }
}

fn draft(kind: u32, d_tag: &str, mut tags: Vec<Tag>, content: String) -> StateDraft {
    let mut all = Vec::with_capacity(tags.len() + 1);
    all.push(Tag::identifier(d_tag));
    all.append(&mut tags);
    StateDraft {
        kind,
        d_tag: d_tag.to_owned(),
        tags: all,
        content,
    }
}

/// `kind:39100` — `["d", slug]`, `["version", n]`, `["p", confirmed_by]`,
/// `["receipt", proposal]` (§4.1).
pub fn direction(artifact: &DirectionArtifact) -> Result<StateDraft, IngestError> {
    let slug = wire(&artifact.slug)?;
    let tags = vec![
        tag_of(["version", &artifact.version.to_string()])?,
        tag_of(["p", &artifact.confirmed_by])?,
        tag_of(["receipt", &artifact.proposal])?,
    ];
    Ok(draft(KIND_IO_DIRECTION, &slug, tags, content_of(artifact)?))
}

/// `kind:39101` — the §4.2 tag set; `receipt` is the command that produced
/// this version.
pub fn work_item(item: &WorkItem, receipt: &str) -> Result<StateDraft, IngestError> {
    let mut tags = vec![
        tag_of([tag::STATUS, &wire(&item.state)?])?,
        tag_of(["root", &item.root])?,
    ];
    if let Some(parent) = &item.parent {
        tags.push(tag_of([tag::PARENT, parent])?);
    }
    if let Some(dri) = &item.dri {
        tags.push(tag_of(["p", dri])?);
    }
    if let Some(offered_to) = &item.offered_to {
        tags.push(tag_of(["p", offered_to, "", tag::MARKER_OFFERED])?);
    }
    tags.push(tag_of(["due", &item.due_at.to_string()])?);
    tags.push(tag_of([
        tag::TYPE,
        if item.parent.is_none() {
            "project"
        } else {
            "ticket"
        },
    ])?);
    if let Some(objective_ref) = &item.objective_ref {
        tags.push(tag_of(["ref", objective_ref])?);
    }
    tags.push(tag_of(["receipt", receipt])?);
    Ok(draft(KIND_IO_WORK_ITEM, &item.id, tags, content_of(item)?))
}

/// `kind:39102` — the §4.4 tag set. `subject` is the named person / payee /
/// seat for `dri`, `join`, `money`, and `shapers` add/remove; `item` the
/// work item a `dri` or `money` proposal is about; `receipt` the opening
/// command.
pub fn proposal(
    p: &Proposal,
    subject: Option<&str>,
    item: Option<&str>,
    receipt: &str,
) -> Result<StateDraft, IngestError> {
    let mut tags = vec![
        tag_of([tag::TYPE, &wire(&p.kind)?])?,
        tag_of([tag::STATUS, &wire(&p.status)?])?,
        tag_of(["p", &p.opened_by])?,
    ];
    if let Some(subject) = subject {
        debug_assert!(matches!(
            p.kind,
            ProposalKind::Dri | ProposalKind::Join | ProposalKind::Money | ProposalKind::Shapers
        ));
        tags.push(tag_of(["p", subject, "", tag::MARKER_SUBJECT])?);
    }
    for eligible in &p.eligible {
        tags.push(tag_of(["p", eligible, "", tag::MARKER_ELIGIBLE])?);
    }
    if let Some(item) = item {
        tags.push(tag_of([tag::ITEM, item])?);
    }
    tags.push(tag_of(["receipt", receipt])?);
    Ok(draft(KIND_IO_PROPOSAL, &p.id, tags, content_of(p)?))
}

/// `kind:39103` — `["d", "shapers"]` and one `["p", pubkey]` per Shaper (§4.5).
pub fn shapers(s: &Shapers) -> Result<StateDraft, IngestError> {
    let tags = s
        .shapers
        .iter()
        .map(|p| tag_of(["p", p]))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(draft(KIND_IO_SHAPERS, SHAPERS_D_TAG, tags, content_of(s)?))
}

/// `kind:39104` — `["d", draft]`, `["s", status]`, `["p", decided_by]` (§4.6).
pub fn draft_outcome(outcome: &DraftOutcome) -> Result<StateDraft, IngestError> {
    let mut tags = vec![tag_of([tag::STATUS, &wire(&outcome.status)?])?];
    if let Some(decided_by) = &outcome.decided_by {
        tags.push(tag_of(["p", decided_by])?);
    }
    Ok(draft(
        KIND_IO_DRAFT_OUTCOME,
        &outcome.draft,
        tags,
        content_of(outcome)?,
    ))
}

/// `kind:39105` — `["d", pubkey]`, `["p", pubkey]`, one `["k", slug]` per
/// skill, `["version", n]` (§4.7a).
pub fn profile(p: &OrgProfile) -> Result<StateDraft, IngestError> {
    let mut tags = vec![tag_of(["p", &p.pubkey])?];
    for skill in &p.skills {
        tags.push(tag_of([tag::SKILL, &skill.slug])?);
    }
    tags.push(tag_of(["version", &p.version.to_string()])?);
    Ok(draft(KIND_IO_PROFILE, &p.pubkey, tags, content_of(p)?))
}

/// Sign `draft` with the relay key at `created_at`.
///
/// `allow_self_tagging` keeps a `p` tag equal to the relay's own pubkey (the
/// default builder strips it) so a state event's tag set is exactly what the
/// builder produced.
pub fn sign(draft: &StateDraft, keys: &Keys, created_at: u64) -> Result<Event, IngestError> {
    let kind = u16::try_from(draft.kind).map_err(|_| {
        IngestError::Internal(format!("error: state kind {} overflows", draft.kind))
    })?;
    EventBuilder::new(Kind::Custom(kind), draft.content.clone())
        .tags(draft.tags.clone())
        .allow_self_tagging()
        .custom_created_at(Timestamp::from(created_at))
        .sign_with_keys(keys)
        .map_err(|e| IngestError::Internal(format!("error: sign state event: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::intelligent_org::{
        ChildrenCounts, DecisionRule, DecisionRules, DirectionSlug, DraftOutcomeStatus,
        ProposalStatus, Skill, Vote, VoteChoice, WorkItemState, DEFAULT_DECISION_WINDOW_SECS,
        DEFAULT_OFFER_WINDOW_SECS,
    };

    fn pk(seed: u8) -> String {
        hex::encode([seed; 32])
    }

    fn tag_vecs(draft: &StateDraft) -> Vec<Vec<String>> {
        draft.tags.iter().map(|t| t.as_slice().to_vec()).collect()
    }

    fn shapers_content() -> Shapers {
        Shapers {
            founder: pk(1),
            shapers: vec![pk(1), pk(2)],
            offered: vec![],
            room: Some(uuid::Uuid::nil().to_string()),
            agent: Some(pk(0xA0)),
            agent_hosted: true,
            rules: DecisionRules::default(),
            decision_window_secs: DEFAULT_DECISION_WINDOW_SECS,
            offer_window_secs: DEFAULT_OFFER_WINDOW_SECS,
            updated_at: 1_700_000_000,
            receipt: pk(9),
        }
    }

    #[test]
    fn shapers_carries_d_and_one_p_per_shaper_and_round_trips_content() {
        let s = shapers_content();
        let d = shapers(&s).unwrap();
        assert_eq!(d.kind, KIND_IO_SHAPERS);
        assert_eq!(d.d_tag, "shapers");
        assert_eq!(
            tag_vecs(&d),
            vec![
                vec!["d".to_owned(), "shapers".to_owned()],
                vec!["p".to_owned(), pk(1)],
                vec!["p".to_owned(), pk(2)],
            ]
        );
        let back: Shapers = serde_json::from_str(&d.content).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn proposal_carries_the_section_4_4_tag_set() {
        let id = uuid::Uuid::new_v4().to_string();
        let p = Proposal {
            id: id.clone(),
            kind: ProposalKind::Shapers,
            status: ProposalStatus::Passed,
            opened_by: pk(1),
            opened_at: 1,
            expires_at: 2,
            draft: None,
            payload: serde_json::json!({}),
            rule: DecisionRule::MAJORITY,
            needed: 1,
            eligible: vec![pk(1), pk(2)],
            votes: vec![Vote {
                p: pk(1),
                vote: VoteChoice::Agree,
                at: 1,
                receipt: pk(9),
            }],
            decided_at: Some(1),
            executed: None,
            settlement: None,
        };
        let d = proposal(&p, Some(&pk(3)), Some("item-uuid"), &pk(9)).unwrap();
        assert_eq!(d.kind, KIND_IO_PROPOSAL);
        assert_eq!(d.d_tag, id);
        assert_eq!(
            tag_vecs(&d),
            vec![
                vec!["d".to_owned(), id],
                vec!["t".to_owned(), "shapers".to_owned()],
                vec!["s".to_owned(), "passed".to_owned()],
                vec!["p".to_owned(), pk(1)],
                vec!["p".to_owned(), pk(3), String::new(), "subject".to_owned()],
                vec!["p".to_owned(), pk(1), String::new(), "eligible".to_owned()],
                vec!["p".to_owned(), pk(2), String::new(), "eligible".to_owned()],
                vec!["i".to_owned(), "item-uuid".to_owned()],
                vec!["receipt".to_owned(), pk(9)],
            ]
        );
    }

    #[test]
    fn direction_work_item_outcome_and_profile_use_their_fixed_tags() {
        let artifact = DirectionArtifact {
            slug: DirectionSlug::Objectives,
            version: 3,
            body: "b".into(),
            lines: vec![],
            confirmed_by: pk(1),
            confirmed_at: 1,
            proposed_by: pk(2),
            proposal: "prop".into(),
            prev: None,
        };
        let d = direction(&artifact).unwrap();
        assert_eq!(
            (d.kind, d.d_tag.as_str()),
            (KIND_IO_DIRECTION, "objectives")
        );
        assert_eq!(
            tag_vecs(&d)[1..],
            [
                vec!["version".to_owned(), "3".to_owned()],
                vec!["p".to_owned(), pk(1)],
                vec!["receipt".to_owned(), "prop".to_owned()],
            ]
        );

        let item = WorkItem {
            id: "item".into(),
            parent: Some("parent".into()),
            root: "root".into(),
            depth: 1,
            path: vec!["root".into()],
            title: "t".into(),
            brief: "b".into(),
            state: WorkItemState::Offered,
            dri: None,
            offered_to: Some(pk(4)),
            offered_by: None,
            offered_at: Some(5),
            due_at: 99,
            approved_at: None,
            objective_ref: Some("objectives@1#l_1".into()),
            created_from: pk(8),
            draft: None,
            done_receipt: None,
            closed_by: None,
            children: ChildrenCounts::default(),
            home: None,
            branch: None,
            after: vec![],
            last_progress: None,
        };
        let d = work_item(&item, &pk(8)).unwrap();
        assert_eq!((d.kind, d.d_tag.as_str()), (KIND_IO_WORK_ITEM, "item"));
        assert_eq!(
            tag_vecs(&d)[1..],
            [
                vec!["s".to_owned(), "offered".to_owned()],
                vec!["root".to_owned(), "root".to_owned()],
                vec!["u".to_owned(), "parent".to_owned()],
                vec!["p".to_owned(), pk(4), String::new(), "offered".to_owned()],
                vec!["due".to_owned(), "99".to_owned()],
                vec!["t".to_owned(), "ticket".to_owned()],
                vec!["ref".to_owned(), "objectives@1#l_1".to_owned()],
                vec!["receipt".to_owned(), pk(8)],
            ]
        );

        let outcome = DraftOutcome {
            draft: pk(7),
            status: DraftOutcomeStatus::Accepted,
            decided_by: Some(pk(1)),
            decided_at: Some(1),
            reason: None,
            result: Some(pk(6)),
        };
        let d = draft_outcome(&outcome).unwrap();
        assert_eq!(
            (d.kind, d.d_tag.as_str()),
            (KIND_IO_DRAFT_OUTCOME, pk(7).as_str())
        );
        assert_eq!(
            tag_vecs(&d)[1..],
            [
                vec!["s".to_owned(), "accepted".to_owned()],
                vec!["p".to_owned(), pk(1)],
            ]
        );

        let p = OrgProfile {
            pubkey: pk(5),
            version: 2,
            about: "a".into(),
            skills: vec![
                Skill {
                    slug: "rust".into(),
                    label: "Rust".into(),
                },
                Skill {
                    slug: "ops".into(),
                    label: "Ops".into(),
                },
            ],
            open_limit: None,
            updated_at: 1,
            receipt: pk(9),
        };
        let d = profile(&p).unwrap();
        assert_eq!(
            (d.kind, d.d_tag.as_str()),
            (KIND_IO_PROFILE, pk(5).as_str())
        );
        assert_eq!(
            tag_vecs(&d)[1..],
            [
                vec!["p".to_owned(), pk(5)],
                vec!["k".to_owned(), "rust".to_owned()],
                vec!["k".to_owned(), "ops".to_owned()],
                vec!["version".to_owned(), "2".to_owned()],
            ]
        );
    }

    #[test]
    fn sign_keeps_every_tag_and_the_requested_created_at() {
        let keys = Keys::generate();
        let d = shapers(&shapers_content()).unwrap();
        let event = sign(&d, &keys, 1_700_000_123).unwrap();
        assert_eq!(event.kind.as_u16() as u32, KIND_IO_SHAPERS);
        assert_eq!(event.created_at.as_secs(), 1_700_000_123);
        assert_eq!(event.pubkey, keys.public_key());
        assert_eq!(event.tags.len(), d.tags.len());
        assert!(event.verify().is_ok());
    }
}
