//! Needs table, downward bias, Personal Assistant map (Org agent § 10).
//! A-1 lands the resolver. Cards and SAY land in A-2.

use buzz_core::intelligent_org::DraftKind;

use crate::judge::Draft;
use crate::state::OrgState;

/// Who can make the draft real.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Needs {
    /// Any Shaper.
    Shaper,
    /// A specific pubkey.
    Pubkey(String),
}

impl Needs {
    /// Wire `n` tag.
    pub fn as_tag(&self) -> &str {
        match self {
            Self::Shaper => "shaper",
            Self::Pubkey(p) => p,
        }
    }
}

/// Where a talk-derived draft came from, for downward bias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalkOrigin {
    /// Speaker pubkey.
    pub speaker: String,
    /// The item the talk was about, when known.
    pub about: Option<String>,
}

/// Resolve `needs` from the draft kind and STATE. A table, not a judgment.
pub fn needs_for(state: &OrgState, draft: &Draft, talk: Option<&TalkOrigin>) -> Needs {
    let Some(kind) = draft.kind else {
        return Needs::Shaper;
    };
    match kind {
        DraftKind::Project => {
            if let Some(t) = talk {
                return Needs::Pubkey(t.speaker.clone());
            }
            Needs::Shaper
        }
        DraftKind::Dri | DraftKind::Objectives | DraftKind::Direction | DraftKind::Review => {
            Needs::Shaper
        }
        DraftKind::Ticket => draft
            .parent
            .as_ref()
            .and_then(|p| state.items.get(p))
            .and_then(|i| i.dri.clone())
            .map(Needs::Pubkey)
            .unwrap_or(Needs::Shaper),
        DraftKind::Done => draft
            .item
            .as_ref()
            .and_then(|i| state.items.get(i))
            .and_then(|i| i.dri.clone())
            .map(Needs::Pubkey)
            .unwrap_or(Needs::Shaper),
        DraftKind::Profile => draft
            .payload
            .as_ref()
            .and_then(|p| match p {
                buzz_core::intelligent_org::DraftPayload::Profile(d) => {
                    Some(Needs::Pubkey(d.pubkey.clone()))
                }
                _ => None,
            })
            .unwrap_or(Needs::Shaper),
        DraftKind::Money => talk
            .map(|t| Needs::Pubkey(t.speaker.clone()))
            .unwrap_or(Needs::Shaper),
    }
}

/// Fingerprint that lifts a nag (Org agent § 10.3).
pub fn fingerprint(
    direction_slug: &str,
    direction_version: u32,
    subtree_hash: &str,
    candidate_profile_versions: &[u32],
) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(direction_slug.as_bytes());
    h.update(direction_version.to_le_bytes());
    h.update(subtree_hash.as_bytes());
    for v in candidate_profile_versions {
        h.update(v.to_le_bytes());
    }
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::OrgState;
    use serde_json::json;

    fn draft(kind: DraftKind) -> Draft {
        Draft {
            kind: Some(kind),
            payload: None,
            health: None,
            raw: json!({}),
            needs: String::new(),
            gap: String::new(),
            receipts: vec![],
            generation: String::new(),
            parent: None,
            item: None,
        }
    }

    #[test]
    fn project_from_talk_needs_the_asker() {
        let state = OrgState::new();
        let d = draft(DraftKind::Project);
        let talk = TalkOrigin {
            speaker: "aa".repeat(32),
            about: None,
        };
        assert_eq!(
            needs_for(&state, &d, Some(&talk)),
            Needs::Pubkey("aa".repeat(32))
        );
        assert_eq!(needs_for(&state, &d, None), Needs::Shaper);
    }

    #[test]
    fn fingerprint_changes_when_a_profile_version_moves() {
        let a = fingerprint("objectives", 4, "tree", &[1, 2]);
        let b = fingerprint("objectives", 4, "tree", &[1, 3]);
        assert_ne!(a, b);
    }
}
