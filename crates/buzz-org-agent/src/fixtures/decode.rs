//! One fixture event → its `buzz_core::intelligent_org` type, with the
//! Protocol §4 tag set checked against the content it carries.

use buzz_core::intelligent_org::{
    DirectionArtifact, DirectionProposeContent, DraftKind, DraftOutcome, DraftPayload,
    EmptyContent, HealthRead, JoinProposeContent, MoneyProposeContent, MoneyReleasedContent,
    OrgProfile, ProfileSetContent, ProgressNote, ProjectProposeContent, Proposal, ProposalKind,
    RulesContent, Shapers, TicketCreateContent, VoteContent, WhyContent, WorkItem,
};
use buzz_core::kind::{
    KIND_DM_OPEN, KIND_IO_ACCEPT, KIND_IO_DECLINE, KIND_IO_DIRECTION, KIND_IO_DIRECTION_PROPOSE,
    KIND_IO_DONE, KIND_IO_DRAFT, KIND_IO_DRAFT_DECIDE, KIND_IO_DRAFT_OUTCOME, KIND_IO_DRI_PROPOSE,
    KIND_IO_HEALTH, KIND_IO_HEALTH_RATE, KIND_IO_JOIN_PROPOSE, KIND_IO_MONEY_PROPOSE,
    KIND_IO_MONEY_RELEASED, KIND_IO_OFFER, KIND_IO_PROFILE, KIND_IO_PROFILE_SET, KIND_IO_PROGRESS,
    KIND_IO_PROJECT_PROPOSE, KIND_IO_PROPOSAL, KIND_IO_RELEASE, KIND_IO_REOPEN, KIND_IO_SET_DUE,
    KIND_IO_SHAPERS, KIND_IO_SHAPERS_PROPOSE, KIND_IO_SHAPER_ACCEPT, KIND_IO_SHAPER_STEP_DOWN,
    KIND_IO_TICKET_CREATE, KIND_IO_VOTE, KIND_IO_WORK_ITEM, KIND_NIP29_GROUP_MEMBERS,
    KIND_NIP29_GROUP_METADATA, KIND_NIP43_MEMBERSHIP_LIST,
};
use buzz_core::Event;
use serde::Serialize;
use serde_json::Value;

use super::FixtureError;

/// NIP-29 chat message.
const KIND_CHAT: u32 = 9;

/// The content of a person-signed command (`50001`–`50021`), typed.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CommandContent {
    /// `{}`.
    Empty(EmptyContent),
    /// `{ why? }`.
    Why(WhyContent),
    /// `50001 op=rules`.
    Rules(RulesContent),
    /// `50002`.
    DirectionPropose(DirectionProposeContent),
    /// `50003`.
    Vote(VoteContent),
    /// `50004`.
    ProjectPropose(ProjectProposeContent),
    /// `50005`.
    TicketCreate(TicketCreateContent),
    /// `50013` (reserved).
    MoneyPropose(MoneyProposeContent),
    /// `50014` (reserved).
    MoneyReleased(MoneyReleasedContent),
    /// `50016` (reserved).
    JoinPropose(JoinProposeContent),
    /// `50021`.
    ProfileSet(ProfileSetContent),
}

/// A fixture event, decoded.
#[derive(Debug, Clone, PartialEq)]
pub enum Decoded {
    /// `39100`.
    Direction(DirectionArtifact),
    /// `39101`.
    WorkItem(WorkItem),
    /// `39102`.
    Proposal(Proposal),
    /// `39103`.
    Shapers(Shapers),
    /// `39104`.
    DraftOutcome(DraftOutcome),
    /// `39105`.
    Profile(OrgProfile),
    /// `50100`, typed by its `t` tag.
    Draft(DraftKind, DraftPayload),
    /// `50101`.
    Health(HealthRead),
    /// `50102`.
    Progress(ProgressNote),
    /// `50001`–`50021`.
    Command(u32, CommandContent),
    /// `kind:9` in a room.
    Message,
    /// NIP-43 `13534`.
    Membership,
    /// NIP-29 `39000`.
    ChannelMeta,
    /// NIP-29 `39002`.
    ChannelMembers,
    /// `41010`.
    DmOpen,
}

impl Decoded {
    /// The typed content re-serialized, for kinds whose content is typed.
    pub fn to_value(&self) -> Result<Option<Value>, serde_json::Error> {
        Ok(Some(match self {
            Self::Direction(v) => serde_json::to_value(v)?,
            Self::WorkItem(v) => serde_json::to_value(v)?,
            Self::Proposal(v) => serde_json::to_value(v)?,
            Self::Shapers(v) => serde_json::to_value(v)?,
            Self::DraftOutcome(v) => serde_json::to_value(v)?,
            Self::Profile(v) => serde_json::to_value(v)?,
            Self::Draft(_, v) => serde_json::to_value(v)?,
            Self::Health(v) => serde_json::to_value(v)?,
            Self::Progress(v) => serde_json::to_value(v)?,
            Self::Command(_, v) => serde_json::to_value(v)?,
            Self::Message
            | Self::Membership
            | Self::ChannelMeta
            | Self::ChannelMembers
            | Self::DmOpen => return Ok(None),
        }))
    }
}

struct Tags<'a> {
    event: &'a Event,
    kind: u32,
    rows: Vec<Vec<String>>,
}

impl<'a> Tags<'a> {
    fn new(event: &'a Event) -> Self {
        Self {
            event,
            kind: u32::from(event.kind.as_u16()),
            rows: event.tags.iter().map(|t| t.as_slice().to_vec()).collect(),
        }
    }

    fn err(&self, reason: impl Into<String>) -> FixtureError {
        FixtureError::Event {
            id: self.event.id.to_hex(),
            kind: self.kind,
            reason: reason.into(),
        }
    }

    /// Two-element tags named `name` (a `["p", x, "", marker]` is not one).
    fn plain(&self, name: &str) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| r.len() == 2 && r[0] == name)
            .map(|r| r[1].as_str())
            .collect()
    }

    fn first(&self, name: &str) -> Option<&str> {
        self.rows
            .iter()
            .find(|r| r.len() >= 2 && r[0] == name)
            .map(|r| r[1].as_str())
    }

    fn required(&self, name: &str) -> Result<&str, FixtureError> {
        self.first(name)
            .ok_or_else(|| self.err(format!("missing [\"{name}\", …] tag")))
    }

    /// Values of `["<name>", x, "", <marker>]` tags.
    fn marked(&self, name: &str, marker: &str) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| r.len() == 4 && r[0] == name && r[2].is_empty() && r[3] == marker)
            .map(|r| r[1].as_str())
            .collect()
    }

    fn expect(&self, name: &str, want: &str) -> Result<(), FixtureError> {
        let got = self.required(name)?;
        if got != want {
            return Err(self.err(format!("[\"{name}\"] is {got}, content says {want}")));
        }
        Ok(())
    }

    fn expect_opt(&self, name: &str, want: Option<&str>) -> Result<(), FixtureError> {
        match (self.plain(name).first().copied(), want) {
            (Some(got), Some(want)) if got == want => Ok(()),
            (None, None) => Ok(()),
            (got, want) => Err(self.err(format!("[\"{name}\"] is {got:?}, content says {want:?}"))),
        }
    }

    fn expect_marked(&self, name: &str, marker: &str, want: &[&str]) -> Result<(), FixtureError> {
        let got = self.marked(name, marker);
        if got != want {
            return Err(self.err(format!(
                "[\"{name}\", …, \"{marker}\"] tags are {got:?}, content says {want:?}"
            )));
        }
        Ok(())
    }

    fn content<T: serde::de::DeserializeOwned>(&self) -> Result<T, FixtureError> {
        serde_json::from_str(&self.event.content).map_err(|e| self.err(format!("content: {e}")))
    }
}

fn wire<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Decode a fixture event as the type for its kind and check its tag set
/// against Protocol §4.
pub fn decode(event: &Event) -> Result<Decoded, FixtureError> {
    let t = Tags::new(event);
    match t.kind {
        KIND_IO_DIRECTION => {
            let artifact: DirectionArtifact = t.content()?;
            t.expect("d", &wire(&artifact.slug))?;
            t.expect("version", &artifact.version.to_string())?;
            t.expect("p", &artifact.confirmed_by)?;
            t.expect("receipt", &artifact.proposal)?;
            Ok(Decoded::Direction(artifact))
        }
        KIND_IO_WORK_ITEM => {
            let item: WorkItem = t.content()?;
            t.expect("d", &item.id)?;
            t.expect("s", &wire(&item.state))?;
            t.expect("root", &item.root)?;
            t.expect_opt("u", item.parent.as_deref())?;
            t.expect_opt("p", item.dri.as_deref())?;
            t.expect_marked(
                "p",
                "offered",
                &item
                    .offered_to
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )?;
            t.expect("due", &item.due_at.to_string())?;
            t.expect(
                "t",
                if item.parent.is_none() {
                    "project"
                } else {
                    "ticket"
                },
            )?;
            t.expect_opt("ref", item.objective_ref.as_deref())?;
            t.required("receipt")?;
            if item.parent.is_none() != (item.depth == 0)
                || (item.parent.is_none() && item.root != item.id)
            {
                return Err(t.err("a root has depth 0 and is its own root"));
            }
            if item.path.len() as u32 != item.depth {
                return Err(t.err("path length is not depth"));
            }
            Ok(Decoded::WorkItem(item))
        }
        KIND_IO_PROPOSAL => {
            let p: Proposal = t.content()?;
            t.expect("d", &p.id)?;
            t.expect("t", &wire(&p.kind))?;
            t.expect("s", &wire(&p.status))?;
            t.expect("p", &p.opened_by)?;
            t.expect_marked(
                "p",
                "eligible",
                &p.eligible.iter().map(String::as_str).collect::<Vec<_>>(),
            )?;
            let subject = t.marked("p", "subject");
            match p.kind {
                ProposalKind::Dri | ProposalKind::Join | ProposalKind::Money
                    if subject.len() != 1 =>
                {
                    return Err(t.err("a dri/join/money proposal names one subject"));
                }
                // `shapers` add/remove name the seat; `rules`/`agent` may not.
                ProposalKind::Shapers if subject.len() > 1 => {
                    return Err(t.err("a shapers proposal names at most one subject"));
                }
                ProposalKind::Direction | ProposalKind::Project if !subject.is_empty() => {
                    return Err(t.err("a direction/project proposal has no subject"));
                }
                _ => {}
            }
            t.required("receipt")?;
            Ok(Decoded::Proposal(p))
        }
        KIND_IO_SHAPERS => {
            let s: Shapers = t.content()?;
            t.expect("d", "shapers")?;
            if t.plain("p") != s.shapers.iter().map(String::as_str).collect::<Vec<_>>() {
                return Err(t.err("[\"p\"] tags are not the Shaper set"));
            }
            Ok(Decoded::Shapers(s))
        }
        KIND_IO_DRAFT_OUTCOME => {
            let o: DraftOutcome = t.content()?;
            t.expect("d", &o.draft)?;
            t.expect("s", &wire(&o.status))?;
            t.expect_opt("p", o.decided_by.as_deref())?;
            Ok(Decoded::DraftOutcome(o))
        }
        KIND_IO_PROFILE => {
            let p: OrgProfile = t.content()?;
            t.expect("d", &p.pubkey)?;
            t.expect("p", &p.pubkey)?;
            if t.plain("k") != p.skills.iter().map(|s| s.slug.as_str()).collect::<Vec<_>>() {
                return Err(t.err("[\"k\"] tags are not the skill slugs"));
            }
            t.expect("version", &p.version.to_string())?;
            Ok(Decoded::Profile(p))
        }
        KIND_IO_DRAFT => decode_draft(&t),
        KIND_IO_HEALTH => {
            let h: HealthRead = t.content()?;
            t.expect("i", &h.item)?;
            t.expect("week", &h.week)?;
            t.expect("band", &wire(&h.band))?;
            Ok(Decoded::Health(h))
        }
        KIND_IO_PROGRESS => {
            let n: ProgressNote = t.content()?;
            t.expect("i", &n.item)?;
            t.expect("p", &n.dri)?;
            t.expect("ref", &n.git_ref)?;
            t.expect("hint", &wire(&n.hint))?;
            if t.plain("commit") != n.commits.iter().map(|c| c.sha.as_str()).collect::<Vec<_>>() {
                return Err(t.err("[\"commit\"] tags are not the commits"));
            }
            Ok(Decoded::Progress(n))
        }
        k if buzz_core::kind::is_intelligent_org_command_kind(k) => decode_command(&t),
        KIND_CHAT => {
            t.required("h")?;
            Ok(Decoded::Message)
        }
        KIND_NIP43_MEMBERSHIP_LIST => {
            if t.rows
                .iter()
                .filter(|r| r.len() == 3 && r[0] == "member")
                .count()
                == 0
            {
                return Err(t.err("no [\"member\", pubkey, role] tags"));
            }
            Ok(Decoded::Membership)
        }
        KIND_NIP29_GROUP_METADATA => {
            t.required("d")?;
            t.required("name")?;
            Ok(Decoded::ChannelMeta)
        }
        KIND_NIP29_GROUP_MEMBERS => {
            t.required("d")?;
            t.required("p")?;
            Ok(Decoded::ChannelMembers)
        }
        KIND_DM_OPEN => {
            t.required("p")?;
            Ok(Decoded::DmOpen)
        }
        other => Err(t.err(format!("kind {other} is not one the fixtures may carry"))),
    }
}

fn decode_draft(t: &Tags<'_>) -> Result<Decoded, FixtureError> {
    let kind_tag = t.required("t")?;
    let kind: DraftKind = serde_json::from_value(Value::String(kind_tag.to_owned()))
        .map_err(|_| t.err(format!("[\"t\", {kind_tag}] is not a draft kind")))?;
    let payload = DraftPayload::parse(kind, &t.event.content)
        .map_err(|e| t.err(format!("content as {kind_tag} draft: {e}")))?;
    let needs = t.required("n")?;
    if needs != "shaper" {
        t.expect_marked("p", "needs", &[needs])?;
    } else if !t.marked("p", "needs").is_empty() {
        return Err(t.err("a needs=shaper draft carries no [\"p\", …, \"needs\"]"));
    }
    let mv: u32 = t
        .required("move")?
        .parse()
        .map_err(|_| t.err("[\"move\"] is not a number"))?;
    if !(1..=4).contains(&mv) {
        return Err(t.err("[\"move\"] is not 1..4"));
    }
    let origin = t.required("origin")?;
    if origin != "talk" && origin != "gap" {
        return Err(t.err("[\"origin\"] is not talk|gap"));
    }
    t.required("gap")?;
    let receipts =
        t.marked("e", "receipt").len() + t.marked("a", "receipt").len() + t.plain("ref").len();
    if receipts == 0 {
        return Err(t.err("a draft carries at least one receipt"));
    }
    match (&payload, kind) {
        (DraftPayload::Ticket(d), _) => t.expect("u", &d.parent)?,
        (DraftPayload::Done(d), _) => t.expect("u", &d.item)?,
        (DraftPayload::Dri(d), _) => t.expect("i", &d.item)?,
        (DraftPayload::Review(d), _) => t.expect("i", &d.item)?,
        (DraftPayload::Money(d), _) => t.expect("i", &d.item)?,
        _ => {}
    }
    let suggested = t.marked("p", "suggested");
    let suggested_in_payload = match &payload {
        DraftPayload::Ticket(d) => d.suggested_holder.clone(),
        DraftPayload::Project(d) => d.suggested_dri.clone(),
        DraftPayload::Dri(d) => Some(d.suggested.clone()),
        _ => None,
    };
    match (suggested.first(), suggested_in_payload.as_deref()) {
        (Some(tag), Some(pk)) if *tag == pk => {}
        (None, None) => {}
        (tag, pk) => {
            return Err(t.err(format!(
                "[\"p\", …, \"suggested\"] is {tag:?}, payload says {pk:?}"
            )))
        }
    }
    Ok(Decoded::Draft(kind, payload))
}

fn decode_command(t: &Tags<'_>) -> Result<Decoded, FixtureError> {
    let content = match t.kind {
        KIND_IO_SHAPERS_PROPOSE => {
            let op = t.required("op")?;
            match op {
                "add" | "remove" => {
                    t.required("p")?;
                    CommandContent::Why(t.content()?)
                }
                "agent" => CommandContent::Why(t.content()?),
                "rules" => CommandContent::Rules(t.content()?),
                other => {
                    return Err(t.err(format!("[\"op\", {other}] is not add|remove|rules|agent")))
                }
            }
        }
        KIND_IO_DIRECTION_PROPOSE => {
            t.required("d")?;
            t.required("base")?;
            CommandContent::DirectionPropose(t.content()?)
        }
        KIND_IO_VOTE => {
            t.required("e")?;
            let vote = t.required("vote")?;
            if vote != "agree" && vote != "decline" {
                return Err(t.err("[\"vote\"] is not agree|decline"));
            }
            CommandContent::Vote(t.content()?)
        }
        KIND_IO_SHAPER_ACCEPT => {
            t.required("e")?;
            CommandContent::Empty(t.content()?)
        }
        KIND_IO_SHAPER_STEP_DOWN => CommandContent::Why(t.content()?),
        KIND_IO_PROJECT_PROPOSE => CommandContent::ProjectPropose(t.content()?),
        KIND_IO_TICKET_CREATE => {
            t.required("u")?;
            CommandContent::TicketCreate(t.content()?)
        }
        KIND_IO_OFFER => {
            t.required("i")?;
            t.required("p")?;
            CommandContent::Empty(t.content()?)
        }
        KIND_IO_ACCEPT | KIND_IO_DECLINE | KIND_IO_DONE => {
            t.required("i")?;
            CommandContent::Empty(t.content()?)
        }
        KIND_IO_RELEASE | KIND_IO_REOPEN => {
            t.required("i")?;
            CommandContent::Why(t.content()?)
        }
        KIND_IO_SET_DUE => {
            t.required("i")?;
            t.required("due")?;
            CommandContent::Why(t.content()?)
        }
        KIND_IO_DRAFT_DECIDE => {
            t.required("e")?;
            let outcome = t.required("outcome")?;
            if outcome != "accept" && outcome != "decline" {
                return Err(t.err("[\"outcome\"] is not accept|decline"));
            }
            CommandContent::Empty(t.content()?)
        }
        KIND_IO_MONEY_PROPOSE => {
            t.required("i")?;
            t.required("p")?;
            CommandContent::MoneyPropose(t.content()?)
        }
        KIND_IO_MONEY_RELEASED => {
            t.required("e")?;
            t.required("tx")?;
            CommandContent::MoneyReleased(t.content()?)
        }
        KIND_IO_DRI_PROPOSE => {
            t.required("i")?;
            t.required("p")?;
            CommandContent::Why(t.content()?)
        }
        KIND_IO_JOIN_PROPOSE => {
            t.required("p")?;
            CommandContent::JoinPropose(t.content()?)
        }
        KIND_IO_HEALTH_RATE => {
            t.required("i")?;
            t.required("week")?;
            t.required("band")?;
            CommandContent::Empty(t.content()?)
        }
        KIND_IO_PROFILE_SET => CommandContent::ProfileSet(t.content()?),
        other => return Err(t.err(format!("command kind {other} has no content type"))),
    };
    Ok(Decoded::Command(t.kind, content))
}

/// Decode `event` and check that the typed content serializes back to the
/// content the event carries — so a fixture field the type drops, or a
/// default the type adds, fails the fixture rather than passing silently.
pub fn roundtrip(event: &Event) -> Result<Decoded, FixtureError> {
    let decoded = decode(event)?;
    let err = |reason: String| FixtureError::Event {
        id: event.id.to_hex(),
        kind: u32::from(event.kind.as_u16()),
        reason,
    };
    if let Some(again) = decoded
        .to_value()
        .map_err(|e| err(format!("re-serialize: {e}")))?
    {
        let original: Value =
            serde_json::from_str(&event.content).map_err(|e| err(format!("content: {e}")))?;
        if again != original {
            return Err(err(format!(
                "content does not round-trip through its type:\n  fixture: {original}\n  typed:   {again}"
            )));
        }
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    //! Each Protocol §4 tag rule is falsifiable on its own: take a fixture
    //! event of the kind, re-sign it with one tag or field changed, and the
    //! decoder must name what is wrong. Signatures are not what these tests
    //! check — `verify_event` does that in `tests/eval_fixtures.rs`.

    use super::*;
    use crate::fixtures::{fixtures_dir, load_events};
    use nostr::{EventBuilder, Keys, Kind, Tag};

    fn river() -> Vec<Event> {
        load_events(&fixtures_dir().join("orgs/river/seed.json")).expect("river seed")
    }

    fn first(kind: u32, pick: impl Fn(&Event) -> bool) -> Event {
        river()
            .into_iter()
            .find(|e| u32::from(e.kind.as_u16()) == kind && pick(e))
            .unwrap_or_else(|| panic!("a kind {kind} event in the River seed"))
    }

    /// `event` with its tags and content replaced, signed by a throwaway key.
    fn resigned(event: &Event, tags: Vec<Vec<String>>, content: &str) -> Event {
        let keys = Keys::generate();
        EventBuilder::new(Kind::Custom(event.kind.as_u16()), content)
            .tags(tags.iter().map(|t| Tag::parse(t).expect("tag")))
            .sign_with_keys(&keys)
            .expect("sign")
    }

    fn rows(event: &Event) -> Vec<Vec<String>> {
        event.tags.iter().map(|t| t.as_slice().to_vec()).collect()
    }

    fn reason(result: Result<Decoded, FixtureError>) -> String {
        match result {
            Err(FixtureError::Event { reason, .. }) => reason,
            other => panic!("expected a tag error, got {other:?}"),
        }
    }

    #[test]
    fn a_fixture_event_decodes_unchanged() {
        let e = first(KIND_IO_WORK_ITEM, |e| Tags::new(e).first("u").is_some());
        assert!(matches!(roundtrip(&e), Ok(Decoded::WorkItem(_))));
        let again = resigned(&e, rows(&e), &e.content);
        assert!(
            matches!(roundtrip(&again), Ok(Decoded::WorkItem(_))),
            "tags and content, not the signer, decide"
        );
    }

    #[test]
    fn work_item_tags_must_match_content() {
        let e = first(KIND_IO_WORK_ITEM, |e| Tags::new(e).first("u").is_some());
        let without = |name: &str| -> Vec<Vec<String>> {
            rows(&e).into_iter().filter(|t| t[0] != name).collect()
        };
        assert!(reason(decode(&resigned(&e, without("root"), &e.content))).contains("root"));
        assert!(reason(decode(&resigned(&e, without("u"), &e.content))).contains("\"u\""));
        assert!(reason(decode(&resigned(&e, without("receipt"), &e.content))).contains("receipt"));
        let mut wrong_state = rows(&e);
        for t in &mut wrong_state {
            if t[0] == "s" {
                t[1] = "done".to_owned();
            }
        }
        assert!(reason(decode(&resigned(&e, wrong_state, &e.content))).contains("\"s\""));
        let mut wrong_type = rows(&e);
        for t in &mut wrong_type {
            if t[0] == "t" {
                t[1] = "project".to_owned();
            }
        }
        assert!(reason(decode(&resigned(&e, wrong_type, &e.content))).contains("\"t\""));
    }

    #[test]
    fn a_dropped_or_added_content_field_fails_the_round_trip() {
        let e = first(KIND_IO_WORK_ITEM, |_| true);
        let mut v: Value = serde_json::from_str(&e.content).expect("json");
        v["extra"] = Value::Bool(true);
        assert!(reason(roundtrip(&resigned(&e, rows(&e), &v.to_string()))).contains("round-trip"));
        let mut v: Value = serde_json::from_str(&e.content).expect("json");
        v.as_object_mut().expect("object").remove("path");
        // `path` has a serde default, so the type accepts the content and then
        // serializes the default back — which is exactly what round-trip catches.
        assert!(reason(roundtrip(&resigned(&e, rows(&e), &v.to_string()))).contains("round-trip"));
    }

    #[test]
    fn draft_tags_are_checked_against_the_payload() {
        let e = first(KIND_IO_DRAFT, |e| Tags::new(e).first("t") == Some("ticket"));
        let without = |name: &str| -> Vec<Vec<String>> {
            rows(&e).into_iter().filter(|t| t[0] != name).collect()
        };
        assert!(reason(decode(&resigned(&e, without("n"), &e.content))).contains("\"n\""));
        assert!(reason(decode(&resigned(&e, without("move"), &e.content))).contains("move"));
        assert!(reason(decode(&resigned(&e, without("u"), &e.content))).contains("\"u\""));
        let no_receipt: Vec<Vec<String>> = rows(&e)
            .into_iter()
            .filter(|t| !(t.len() == 4 && t[3] == "receipt") && t[0] != "ref")
            .collect();
        assert!(reason(decode(&resigned(&e, no_receipt, &e.content))).contains("receipt"));
        let no_suggested: Vec<Vec<String>> = rows(&e)
            .into_iter()
            .filter(|t| !(t.len() == 4 && t[3] == "suggested"))
            .collect();
        if !Tags::new(&e).marked("p", "suggested").is_empty() {
            assert!(reason(decode(&resigned(&e, no_suggested, &e.content))).contains("suggested"));
        }
    }

    #[test]
    fn proposal_subjects_follow_their_kind() {
        let e = first(KIND_IO_PROPOSAL, |e| {
            Tags::new(e).first("t") == Some("project")
        });
        let mut with_subject = rows(&e);
        let opener = Tags::new(&e).required("p").expect("p").to_owned();
        with_subject.push(vec!["p".into(), opener, String::new(), "subject".into()]);
        assert!(reason(decode(&resigned(&e, with_subject, &e.content))).contains("no subject"));
        let no_eligible: Vec<Vec<String>> = rows(&e)
            .into_iter()
            .filter(|t| !(t.len() == 4 && t[3] == "eligible"))
            .collect();
        assert!(reason(decode(&resigned(&e, no_eligible, &e.content))).contains("eligible"));
    }

    #[test]
    fn command_kinds_need_their_tags_and_unknown_kinds_are_refused() {
        let e = first(KIND_IO_OFFER, |_| true);
        let without = |name: &str| -> Vec<Vec<String>> {
            rows(&e).into_iter().filter(|t| t[0] != name).collect()
        };
        assert!(reason(decode(&resigned(&e, without("i"), &e.content))).contains("\"i\""));
        assert!(reason(decode(&resigned(&e, without("p"), &e.content))).contains("\"p\""));
        let stranger = EventBuilder::new(Kind::Custom(50050), "{}")
            .sign_with_keys(&Keys::generate())
            .expect("sign");
        assert!(reason(decode(&stranger)).contains("50050"));
    }
}
