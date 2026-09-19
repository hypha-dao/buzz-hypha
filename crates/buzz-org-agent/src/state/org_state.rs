//! [`OrgState`] + [`OrgState::apply`].

use std::collections::{BTreeMap, BTreeSet};

use buzz_core::intelligent_org::{
    DirectionArtifact, DirectionSlug, DraftKind, DraftOutcome, DraftPayload, HealthRead,
    OrgProfile, ProgressNote, Proposal, Shapers, WorkItem, WorkItemState,
};
use buzz_core::kind::{
    is_intelligent_org_command_kind, KIND_IO_DRAFT, KIND_IO_HEALTH, KIND_IO_PROGRESS, KIND_IO_VOTE,
    KIND_IO_WORK_ITEM, KIND_NIP43_MEMBERSHIP_LIST,
};
use buzz_core::Event;
use serde::{Deserialize, Serialize};

use super::rooms::{Room, RoomKind};
use super::transitions::{self, Transition};
use crate::inbound::{self, ApplyError, CommandContent, Decoded};

/// A REQ stream whose watermark STATE tracks (Org agent § 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stream {
    /// `39100–39105`.
    State,
    /// `50100`, `50101`.
    Drafts,
    /// `50102`.
    Progress,
    /// `50001–50021`.
    Commands,
    /// NIP-43 membership and `39002`.
    Membership,
    /// `kind:9` / `40002` (HEAR).
    Talk,
    /// `41010`, `41011`, `39000`.
    DmControl,
}

impl Stream {
    /// The stream an inbound kind belongs to.
    pub fn for_kind(kind: u32) -> Option<Self> {
        Some(match kind {
            39100..=39105 => Self::State,
            50100 | 50101 => Self::Drafts,
            50102 => Self::Progress,
            k if is_intelligent_org_command_kind(k) => Self::Commands,
            KIND_NIP43_MEMBERSHIP_LIST | 39002 => Self::Membership,
            9 | 40002 => Self::Talk,
            41010 | 41011 | 39000 => Self::DmControl,
            _ => return None,
        })
    }
}

/// Latest `39100` for one slug, plus the event that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectionHead {
    /// The artifact.
    pub artifact: DirectionArtifact,
    /// Event id (generation).
    pub event_id: String,
}

/// A `50100` STATE has seen.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DraftRecord {
    /// Event id.
    pub event_id: String,
    /// Draft kind.
    pub kind: DraftKind,
    /// Typed payload.
    pub payload: DraftPayload,
    /// `n` tag: pubkey or `shaper`.
    pub needs: String,
    /// Dedupe key.
    pub gap: String,
    /// Outcome, when a `39104` has arrived.
    pub outcome: Option<DraftOutcome>,
}

/// Latest `50101` for a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredHealth {
    /// The read.
    pub read: HealthRead,
    /// Event id.
    pub event_id: String,
}

/// A person-signed command, for the ledger view.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommandEvent {
    /// Event id.
    pub event_id: String,
    /// Kind `50001–50021`.
    pub kind: u32,
    /// Signer.
    pub pubkey: String,
    /// Created-at.
    pub created_at: u64,
    /// Typed content.
    pub content: CommandContent,
    /// `i` / `e` / `d` tags the transitions need.
    pub item: Option<String>,
    /// Proposal uuid from an `e` tag (`50003`, `50019`).
    pub proposal: Option<String>,
}

/// The agent's read model. Plain, serialisable, no I/O.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct OrgState {
    /// `39100` heads, keyed by slug (`mission` / `vision` / …).
    pub direction: BTreeMap<String, DirectionHead>,
    /// `39101` by uuid.
    pub items: BTreeMap<String, WorkItem>,
    /// Event id of the latest `39101` per item (generation).
    pub item_generations: BTreeMap<String, String>,
    /// `39102` by uuid.
    pub proposals: BTreeMap<String, Proposal>,
    /// `39103`.
    pub shapers: Option<Shapers>,
    /// `39104` by draft event id.
    pub outcomes: BTreeMap<String, DraftOutcome>,
    /// `39105` by pubkey.
    pub profiles: BTreeMap<String, OrgProfile>,
    /// `50100` by event id.
    pub drafts: BTreeMap<String, DraftRecord>,
    /// Latest `50101` per root.
    pub health: BTreeMap<String, StoredHealth>,
    /// `50102` per item, newest first.
    pub progress: BTreeMap<String, Vec<ProgressNote>>,
    /// `50001–50021`.
    pub commands: Vec<CommandEvent>,
    /// Decline reasons on `50003`, keyed by proposal uuid.
    pub vote_reasons: BTreeMap<String, Vec<String>>,
    /// NIP-43 members.
    pub members: BTreeSet<String>,
    /// Rooms index.
    pub rooms: BTreeMap<String, Room>,
    /// Per-stream watermarks (`created_at`).
    pub watermarks: BTreeMap<Stream, u64>,
    /// The agent's pubkey, when known (`39103.agent` or config).
    pub me: Option<String>,
    /// Event ids already applied (echo / reconnect overlap).
    pub seen: BTreeSet<String>,
    /// When false, [`apply`] updates the mirror and emits nothing.
    pub live: bool,
}

impl OrgState {
    /// An empty mirror. Apply the backfill, then [`mark_live`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Start emitting transitions for events newer than the backfill.
    pub fn mark_live(&mut self) {
        self.live = true;
    }

    /// Apply a fixture or relay event list as backfill (no transitions),
    /// then mark live.
    pub fn apply_backfill(&mut self, events: &[Event]) -> Result<(), ApplyError> {
        let was = self.live;
        self.live = false;
        for event in events {
            let _ = self.apply(event)?;
        }
        self.live = was;
        Ok(())
    }

    /// Load a complete seed: apply every event, then mark live.
    pub fn from_events(events: &[Event]) -> Result<Self, ApplyError> {
        let mut state = Self::new();
        state.apply_backfill(events)?;
        state.mark_live();
        Ok(state)
    }

    /// The generation currently stored for a transition's object.
    pub fn generation_of<'a>(&'a self, transition: &'a Transition) -> Option<&'a str> {
        match transition {
            Transition::DirectionConfirmed { slug, .. } => self
                .direction
                .get(slug_key(*slug))
                .map(|h| h.event_id.as_str()),
            Transition::RootOpenUnheld { item, .. }
            | Transition::OfferReturned { item, .. }
            | Transition::HolderSet { item, .. }
            | Transition::ItemDone { item, .. }
            | Transition::ProgressNoted { item, .. } => {
                self.item_generations.get(item).map(String::as_str)
            }
            Transition::LastChildDone { parent, .. }
            | Transition::ChildDoneBriefUnmet { parent, .. } => {
                self.item_generations.get(parent).map(String::as_str)
            }
            Transition::EnteredReview { root, .. }
            | Transition::RootClosed { root, .. }
            | Transition::RootLedgerChanged { root, .. } => {
                self.item_generations.get(root).map(String::as_str)
            }
            _ => Some(transition.generation()),
        }
    }

    /// The only mutator. Validates Protocol §4 tags, updates the mirror,
    /// and — once live — returns the § 5.2 transitions.
    pub fn apply(&mut self, event: &Event) -> Result<Vec<Transition>, ApplyError> {
        let id = event.id.to_hex();
        if !self.seen.insert(id.clone()) {
            return Ok(Vec::new());
        }
        let decoded = inbound::decode(event)?;
        let kind = u32::from(event.kind.as_u16());
        if let Some(stream) = Stream::for_kind(kind) {
            let ts = event.created_at.as_secs();
            let watermark = self.watermarks.entry(stream).or_insert(0);
            if ts > *watermark {
                *watermark = ts;
            }
        }

        let prev = self.previous_decoded(&decoded);
        let prev_members = self.members.clone();
        let tags: Vec<Vec<String>> = event.tags.iter().map(|t| t.as_slice().to_vec()).collect();
        self.ingest(&decoded, &id, event, &tags);

        if !self.live {
            return Ok(Vec::new());
        }

        let mut out = transitions::emit(
            self,
            prev.as_ref(),
            &decoded,
            &id,
            &event.pubkey.to_hex(),
            &tags,
        );
        out.extend(self.membership_joins(&decoded, &prev_members, &id));
        out.extend(self.ledger_touches(&decoded, event, &id));
        Ok(out)
    }

    fn previous_decoded(&self, next: &Decoded) -> Option<Decoded> {
        match next {
            Decoded::Direction(a) => self
                .direction
                .get(slug_key(a.slug))
                .map(|h| Decoded::Direction(h.artifact.clone())),
            Decoded::WorkItem(i) => self.items.get(&i.id).cloned().map(Decoded::WorkItem),
            Decoded::Proposal(p) => self.proposals.get(&p.id).cloned().map(Decoded::Proposal),
            Decoded::Shapers(_) => self.shapers.clone().map(Decoded::Shapers),
            Decoded::DraftOutcome(o) => self
                .outcomes
                .get(&o.draft)
                .cloned()
                .map(Decoded::DraftOutcome),
            Decoded::Profile(p) => self.profiles.get(&p.pubkey).cloned().map(Decoded::Profile),
            _ => None,
        }
    }

    fn ingest(&mut self, decoded: &Decoded, event_id: &str, event: &Event, tags: &[Vec<String>]) {
        match decoded {
            Decoded::Direction(a) => {
                self.direction.insert(
                    slug_key(a.slug).to_owned(),
                    DirectionHead {
                        artifact: a.clone(),
                        event_id: event_id.to_owned(),
                    },
                );
            }
            Decoded::WorkItem(item) => {
                self.items.insert(item.id.clone(), item.clone());
                self.item_generations
                    .insert(item.id.clone(), event_id.to_owned());
                self.reclassify_rooms();
            }
            Decoded::Proposal(p) => {
                self.proposals.insert(p.id.clone(), p.clone());
            }
            Decoded::Shapers(s) => {
                if self.me.is_none() {
                    self.me = s.agent.clone();
                }
                self.shapers = Some(s.clone());
                self.reclassify_rooms();
            }
            Decoded::DraftOutcome(o) => {
                self.outcomes.insert(o.draft.clone(), o.clone());
                if let Some(d) = self.drafts.get_mut(&o.draft) {
                    d.outcome = Some(o.clone());
                }
            }
            Decoded::Profile(p) => {
                self.profiles.insert(p.pubkey.clone(), p.clone());
            }
            Decoded::Draft(kind, payload) => {
                let needs = first_tag(tags, "n").unwrap_or_else(|| "shaper".into());
                let gap = first_tag(tags, "gap").unwrap_or_default();
                self.drafts.insert(
                    event_id.to_owned(),
                    DraftRecord {
                        event_id: event_id.to_owned(),
                        kind: *kind,
                        payload: payload.clone(),
                        needs,
                        gap,
                        outcome: self.outcomes.get(event_id).cloned(),
                    },
                );
            }
            Decoded::Health(h) => {
                self.health.insert(
                    h.item.clone(),
                    StoredHealth {
                        read: h.clone(),
                        event_id: event_id.to_owned(),
                    },
                );
            }
            Decoded::Progress(n) => {
                let list = self.progress.entry(n.item.clone()).or_default();
                list.insert(0, n.clone());
            }
            Decoded::Command(kind, content) => {
                let item = first_tag(tags, "i").or_else(|| first_tag(tags, "u"));
                let proposal = if *kind == KIND_IO_VOTE || *kind == 50019 || *kind == 50003 {
                    first_tag(tags, "e")
                } else {
                    None
                };
                if *kind == KIND_IO_VOTE {
                    if let CommandContent::Vote(v) = content {
                        if let (Some(pid), Some(reason)) = (proposal.clone(), v.reason.clone()) {
                            self.vote_reasons.entry(pid).or_default().push(reason);
                        }
                    }
                }
                self.commands.push(CommandEvent {
                    event_id: event_id.to_owned(),
                    kind: *kind,
                    pubkey: event.pubkey.to_hex(),
                    created_at: event.created_at.as_secs(),
                    content: content.clone(),
                    item,
                    proposal,
                });
            }
            Decoded::Membership => {
                self.members = tags
                    .iter()
                    .filter(|t| t.first().map(String::as_str) == Some("member"))
                    .filter_map(|t| t.get(1).cloned())
                    .collect();
            }
            Decoded::ChannelMeta => {
                if let Some(id) = first_tag(tags, "d") {
                    let room = self.rooms.entry(id.clone()).or_insert_with(|| Room {
                        id,
                        name: first_tag(tags, "name"),
                        kind: RoomKind::Channel,
                        members: BTreeSet::new(),
                        listening: crate::state::rooms::ListeningMode::MentionOnly,
                    });
                    room.name = first_tag(tags, "name").or_else(|| room.name.clone());
                    self.reclassify_rooms();
                }
            }
            Decoded::ChannelMembers => {
                if let Some(id) = first_tag(tags, "d") {
                    let members = tags
                        .iter()
                        .filter(|t| t.first().map(String::as_str) == Some("p"))
                        .filter_map(|t| t.get(1).cloned())
                        .collect();
                    let room = self.rooms.entry(id.clone()).or_insert_with(|| Room {
                        id,
                        name: None,
                        kind: RoomKind::Channel,
                        members: BTreeSet::new(),
                        listening: crate::state::rooms::ListeningMode::MentionOnly,
                    });
                    room.members = members;
                    self.reclassify_rooms();
                }
            }
            Decoded::DmOpen => {
                let members: BTreeSet<String> = tags
                    .iter()
                    .filter(|t| t.first().map(String::as_str) == Some("p"))
                    .filter_map(|t| t.get(1).cloned())
                    .collect();
                let id = first_tag(tags, "d").unwrap_or_else(|| {
                    let mut v: Vec<_> = members.iter().cloned().collect();
                    v.sort();
                    format!("dm:{}", v.join("+"))
                });
                self.rooms.insert(
                    id.clone(),
                    Room {
                        id,
                        name: None,
                        kind: RoomKind::Dm,
                        members,
                        listening: crate::state::rooms::ListeningMode::Passive,
                    },
                );
                self.reclassify_rooms();
            }
            Decoded::Message | Decoded::AgentNote => {}
        }
        let _ = KIND_IO_DRAFT;
        let _ = KIND_IO_HEALTH;
        let _ = KIND_IO_PROGRESS;
        let _ = KIND_IO_WORK_ITEM;
    }

    fn reclassify_rooms(&mut self) {
        let shapers_room = self.shapers.as_ref().and_then(|s| s.room.as_deref());
        let homes: BTreeSet<String> = self
            .items
            .values()
            .filter(|i| i.parent.is_none() && i.state != WorkItemState::Done)
            .filter_map(|i| i.home.as_ref().map(|h| h.channel.clone()))
            .collect();
        let agent = self
            .me
            .clone()
            .or_else(|| self.shapers.as_ref().and_then(|s| s.agent.clone()));
        for room in self.rooms.values_mut() {
            room.classify(shapers_room, &homes, agent.as_deref());
        }
    }

    fn membership_joins(
        &self,
        decoded: &Decoded,
        prev_members: &BTreeSet<String>,
        event_id: &str,
    ) -> Vec<Transition> {
        if !matches!(decoded, Decoded::Membership) {
            return Vec::new();
        }
        self.members
            .iter()
            .filter(|p| !prev_members.contains(*p))
            .map(|p| Transition::MemberJoined {
                p: p.clone(),
                via: "membership".into(),
                generation: event_id.to_owned(),
            })
            .collect()
    }

    fn ledger_touches(&self, decoded: &Decoded, event: &Event, event_id: &str) -> Vec<Transition> {
        let root = match decoded {
            Decoded::WorkItem(i) => Some(i.root.clone()),
            Decoded::Progress(n) => self.items.get(&n.item).map(|i| i.root.clone()),
            Decoded::Command(_, _) => {
                let tags: Vec<Vec<String>> =
                    event.tags.iter().map(|t| t.as_slice().to_vec()).collect();
                first_tag(&tags, "i")
                    .or_else(|| first_tag(&tags, "u"))
                    .and_then(|id| self.items.get(&id).map(|i| i.root.clone()))
            }
            _ => None,
        };
        let Some(root) = root else {
            return Vec::new();
        };
        let live = self
            .items
            .get(&root)
            .is_some_and(|i| i.state != WorkItemState::Done)
            || matches!(
                decoded,
                Decoded::WorkItem(i) if i.parent.is_none() && i.state != WorkItemState::Done
            );
        if live {
            vec![Transition::RootLedgerChanged {
                root,
                generation: event_id.to_owned(),
            }]
        } else {
            Vec::new()
        }
    }
}

fn slug_key(slug: DirectionSlug) -> &'static str {
    match slug {
        DirectionSlug::Mission => "mission",
        DirectionSlug::Vision => "vision",
        DirectionSlug::Objectives => "objectives",
        DirectionSlug::Strategy => "strategy",
    }
}

fn first_tag(tags: &[Vec<String>], name: &str) -> Option<String> {
    tags.iter()
        .find(|t| t.first().map(String::as_str) == Some(name))
        .and_then(|t| t.get(1).cloned())
}
