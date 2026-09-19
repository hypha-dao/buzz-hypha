//! Fixture-facing wrappers around [`crate::inbound`].
//!
//! Protocol §4 tag checks live in `inbound` so `OrgState::apply` is the one
//! validation path. This module keeps the E-1 names (`decode`, `roundtrip`,
//! `Decoded`) so existing fixture tests do not change.

use buzz_core::Event;

use super::FixtureError;
use crate::inbound::{self, ApplyError};

pub use crate::inbound::{CommandContent, Decoded};

pub(crate) use crate::inbound::Tags;

/// Decode a fixture event as the type for its kind and check its tag set
/// against Protocol §4.
pub fn decode(event: &Event) -> Result<Decoded, FixtureError> {
    inbound::decode(event).map_err(into_fixture)
}

/// Decode `event` and check that the typed content serializes back to the
/// content the event carries.
pub fn roundtrip(event: &Event) -> Result<Decoded, FixtureError> {
    inbound::roundtrip(event).map_err(into_fixture)
}

fn into_fixture(err: ApplyError) -> FixtureError {
    match err {
        ApplyError::Event { id, kind, reason } => FixtureError::Event { id, kind, reason },
    }
}
#[cfg(test)]
mod tests {
    //! Each Protocol §4 tag rule is falsifiable on its own: take a fixture
    //! event of the kind, re-sign it with one tag or field changed, and the
    //! decoder must name what is wrong. Signatures are not what these tests
    //! check — `verify_event` does that in `tests/eval_fixtures.rs`.

    use super::*;
    use crate::fixtures::{fixtures_dir, load_events};
    use buzz_core::kind::{KIND_IO_DRAFT, KIND_IO_OFFER, KIND_IO_PROPOSAL, KIND_IO_WORK_ITEM};
    use buzz_core::Event;
    use nostr::{EventBuilder, Keys, Kind, Tag};
    use serde_json::Value;

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
