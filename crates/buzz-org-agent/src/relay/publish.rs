//! Sign chokepoint — the only place the crate builds an [`Event`].

use buzz_core::kind::{
    KIND_AGENT_ENGRAM, KIND_AUTH, KIND_DM_OPEN, KIND_IO_AGENT_NOTE, KIND_IO_DONE, KIND_IO_DRAFT,
    KIND_IO_HEALTH, KIND_IO_WORK_PROMPT,
};
use nostr::{Event, EventBuilder, Keys, Kind, Tag};
use thiserror::Error;

/// Kinds the agent may sign (Org agent § 4.6). `50009` is not on this
/// list — it is constructible only from [`crate::jobs_impl::done_from_talk`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permitted {
    /// `50100`.
    Draft,
    /// `50101`.
    Health,
    /// `50103`.
    AgentNote,
    /// `50104`. A work prompt. It does not change state.
    WorkPrompt,
    /// `kind:9`.
    Chat,
    /// `41010`.
    DmOpen,
    /// `30174`.
    Engram,
    /// `22242`.
    Auth,
    /// `kind:0`.
    Profile,
}

impl Permitted {
    /// Kind integer.
    pub fn kind(self) -> u32 {
        match self {
            Self::Draft => KIND_IO_DRAFT,
            Self::Health => KIND_IO_HEALTH,
            Self::AgentNote => KIND_IO_AGENT_NOTE,
            Self::WorkPrompt => KIND_IO_WORK_PROMPT,
            Self::Chat => 9,
            Self::DmOpen => KIND_DM_OPEN,
            Self::Engram => KIND_AGENT_ENGRAM,
            Self::Auth => KIND_AUTH,
            Self::Profile => 0,
        }
    }

    /// Every kind this enum can produce — the allow-list.
    pub fn all_kinds() -> &'static [u32] {
        &[
            KIND_IO_DRAFT,
            KIND_IO_HEALTH,
            KIND_IO_AGENT_NOTE,
            KIND_IO_WORK_PROMPT,
            9,
            KIND_DM_OPEN,
            KIND_AGENT_ENGRAM,
            KIND_AUTH,
            0,
        ]
    }
}

/// Why signing failed.
#[derive(Debug, Error)]
pub enum PublishError {
    /// nostr refused to sign.
    #[error("sign: {0}")]
    Sign(String),
    /// `50009` requested outside `done_from_talk`.
    #[error("kind 50009 is not permitted outside done_from_talk")]
    DoneNotPermitted,
    /// Outbox I/O.
    #[error("outbox: {0}")]
    Outbox(String),
    /// The relay's OK was false.
    #[error("rejected: {0}")]
    Rejected(String),
}

/// The one `sign()` chokepoint. Tests deny any other `EventBuilder` in this
/// crate except `done_from_talk`.
pub fn sign(
    keys: &Keys,
    kind: Permitted,
    content: &str,
    tags: Vec<Tag>,
) -> Result<Event, PublishError> {
    sign_at(keys, kind, content, tags, None)
}

/// `sign` at a chosen second. The mission handoff uses the next second so a
/// client that orders equal timestamps by event id cannot show it first.
pub fn sign_at(
    keys: &Keys,
    kind: Permitted,
    content: &str,
    tags: Vec<Tag>,
    created_at: Option<u64>,
) -> Result<Event, PublishError> {
    let mut builder = EventBuilder::new(Kind::Custom(kind.kind() as u16), content)
        .tags(tags)
        .allow_self_tagging();
    if let Some(secs) = created_at {
        builder = builder.custom_created_at(nostr::Timestamp::from(secs));
    }
    builder
        .sign_with_keys(keys)
        .map_err(|e| PublishError::Sign(e.to_string()))
}

/// Internal: `50009` from [`crate::jobs_impl::done_from_talk`] only.
pub(crate) fn sign_done_from_talk(
    keys: &Keys,
    content: &str,
    tags: Vec<Tag>,
) -> Result<Event, PublishError> {
    EventBuilder::new(Kind::Custom(KIND_IO_DONE as u16), content)
        .tags(tags)
        .allow_self_tagging()
        .sign_with_keys(keys)
        .map_err(|e| PublishError::Sign(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permitted_kinds_do_not_include_io_done() {
        assert!(!Permitted::all_kinds().contains(&KIND_IO_DONE));
        assert!(!Permitted::all_kinds().contains(&50009));
    }

    #[test]
    fn sign_produces_the_permitted_kind() {
        let keys = Keys::generate();
        let event = sign(&keys, Permitted::AgentNote, "{}", vec![]).expect("sign");
        assert_eq!(u32::from(event.kind.as_u16()), KIND_IO_AGENT_NOTE);
    }

    #[test]
    fn sign_at_stamps_the_given_second() {
        let keys = Keys::generate();
        let event = sign_at(&keys, Permitted::Chat, "hi", vec![], Some(50)).expect("sign");
        assert_eq!(event.created_at.as_secs(), 50);
    }
}
