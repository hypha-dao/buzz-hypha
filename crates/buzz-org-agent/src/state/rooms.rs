//! Rooms index, listening mode, DM identity (Org agent § 4.5, § 7.1).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// How a room is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    /// An ordinary channel.
    Channel,
    /// A 1:1 or small DM.
    Dm,
    /// `#shapers` (`39103.room`).
    Shapers,
    /// A live project's home (`39101.home.channel`).
    ProjectHome,
}

/// HEAR listening mode (Org agent § 7.1). A-1 records it; HEAR is later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListeningMode {
    /// Every batch that passes the pre-filter.
    Passive,
    /// Only a batch that tags the org agent.
    MentionOnly,
}

/// One room in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    /// NIP-29 group id, or a DM id.
    pub id: String,
    /// Display name, when known.
    pub name: Option<String>,
    /// Classification.
    pub kind: RoomKind,
    /// Current members (agent included when the relay lists it).
    pub members: BTreeSet<String>,
    /// HEAR mode.
    pub listening: ListeningMode,
}

impl Room {
    /// Participants minus the org agent — the relay's DM-identity rule.
    pub fn participants_minus_agent(&self, agent: Option<&str>) -> BTreeSet<String> {
        self.members
            .iter()
            .filter(|p| agent.is_none_or(|a| *p != a))
            .cloned()
            .collect()
    }

    /// Recompute [`RoomKind`] and [`ListeningMode`] from `shapers_room`,
    /// project-home channels, and the agent pubkey.
    pub fn classify(
        &mut self,
        shapers_room: Option<&str>,
        project_homes: &BTreeSet<String>,
        agent: Option<&str>,
    ) {
        if shapers_room.is_some_and(|r| r == self.id) {
            self.kind = RoomKind::Shapers;
            self.listening = ListeningMode::Passive;
            return;
        }
        if project_homes.contains(&self.id) {
            self.kind = RoomKind::ProjectHome;
            self.listening = ListeningMode::Passive;
            return;
        }
        if self.kind == RoomKind::Dm
            || (self.kind != RoomKind::Channel
                && self.participants_minus_agent(agent).len() == 1
                && self.members.iter().any(|p| agent.is_some_and(|a| p == a)))
        {
            self.kind = RoomKind::Dm;
            let minus = self.participants_minus_agent(agent);
            self.listening = if minus.len() == 1 {
                ListeningMode::Passive
            } else {
                ListeningMode::MentionOnly
            };
            return;
        }
        self.kind = RoomKind::Channel;
        self.listening = ListeningMode::MentionOnly;
    }
}
