//! The § 5.2 transition table.

use buzz_core::intelligent_org::{
    ClosedBy, DeclineReason, DirectionSlug, DraftOutcomeStatus, ProgressHint, ProposalKind,
    ProposalStatus, WorkItem, WorkItemState,
};
use serde::{Deserialize, Serialize};

use super::org_state::{DraftRecord, OrgState};
use crate::inbound::Decoded;

/// A state-change trigger. Each row of Org agent § 5.2 is one variant.
///
/// `generation` is the event id of the `39100`/`39101` (or the event that
/// produced the row) so JOBS can fence on it (§ 6.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transition {
    /// `39100` new version (any slug).
    DirectionConfirmed {
        /// Artifact slug.
        slug: DirectionSlug,
        /// New version.
        version: u32,
        /// Compact `v{prev}->v{next}` (empty on v1).
        diff: String,
        /// Producing event id.
        generation: String,
    },
    /// `39101` root appears `open`, `dri = null`.
    RootOpenUnheld {
        /// Item uuid.
        item: String,
        /// Producing event id.
        generation: String,
    },
    /// `39101` `offered → open` (decline or expiry), root.
    OfferReturned {
        /// Item uuid.
        item: String,
        /// Who it was offered to.
        to: Option<String>,
        /// Producing event id.
        generation: String,
    },
    /// `39101` `* → accepted`.
    HolderSet {
        /// Item uuid.
        item: String,
        /// The holder.
        dri: String,
        /// Producing event id.
        generation: String,
    },
    /// `39101` `* → done`, parent has no live children.
    LastChildDone {
        /// Parent uuid.
        parent: String,
        /// Producing event id.
        generation: String,
    },
    /// `39101` `* → done`, parent's last J2 coverage still has uncovered pieces.
    ChildDoneBriefUnmet {
        /// Parent uuid.
        parent: String,
        /// Producing event id.
        generation: String,
    },
    /// `39101` `* → done`, any.
    ItemDone {
        /// Item uuid.
        item: String,
        /// How it closed.
        closed_by: Option<ClosedBy>,
        /// Producing event id.
        generation: String,
    },
    /// `39101` root `→ in_review`.
    EnteredReview {
        /// Root uuid.
        root: String,
        /// Producing event id.
        generation: String,
    },
    /// `39101` root `→ done`.
    RootClosed {
        /// Root uuid.
        root: String,
        /// How it closed.
        closed_by: Option<ClosedBy>,
        /// Producing event id.
        generation: String,
    },
    /// `39101` `last_progress` changed.
    ProgressNoted {
        /// Item uuid.
        item: String,
        /// Hint from the newest `50102`, when STATE has it.
        hint: Option<ProgressHint>,
        /// `merged_into` from the note, when present.
        merged_into: Option<String>,
        /// Producing event id.
        generation: String,
    },
    /// `39102` `open → expired`.
    ProposalExpired {
        /// Proposal uuid.
        proposal: String,
        /// Who opened it.
        opened_by: String,
        /// Producing event id.
        generation: String,
    },
    /// `39102` `direction` `→ rejected` with a decline reason of `direction`.
    DirectionRejectedWithReason {
        /// Proposal uuid.
        proposal: String,
        /// The decline reason (`direction`).
        reason: String,
        /// Producing event id.
        generation: String,
    },
    /// `39103` gained a pubkey.
    ShaperAccepted {
        /// New Shaper.
        p: String,
        /// Producing event id.
        generation: String,
    },
    /// `39103` rules / windows changed.
    RulesChanged {
        /// Producing event id.
        generation: String,
    },
    /// `39103` `agent` changed.
    AgentChanged {
        /// Previous agent.
        from: Option<String>,
        /// New agent.
        to: Option<String>,
        /// Producing event id.
        generation: String,
    },
    /// `39104` `→ declined`.
    DraftDeclined {
        /// Gap key (from the `50100` when STATE has it).
        gap: String,
        /// Decline reason.
        reason: Option<DeclineReason>,
        /// Fingerprint at decline (empty until ROUTE fills it).
        fingerprint: String,
        /// Producing event id.
        generation: String,
    },
    /// `39105` new or changed.
    ProfileChanged {
        /// Member pubkey.
        pubkey: String,
        /// Producing event id.
        generation: String,
    },
    /// Any `39101` / command / `50102` touching a live root.
    RootLedgerChanged {
        /// Root uuid.
        root: String,
        /// Producing event id.
        generation: String,
    },
    /// Membership list gained a pubkey.
    MemberJoined {
        /// New member.
        p: String,
        /// How they joined (`invite` when known, else `membership`).
        via: String,
        /// Producing event id.
        generation: String,
    },
    /// `41010` with the agent in the participant set.
    DmOpened {
        /// Room / conversation id.
        room: String,
        /// Producing event id.
        generation: String,
    },
}

impl Transition {
    /// The generation this job fences on.
    pub fn generation(&self) -> &str {
        match self {
            Self::DirectionConfirmed { generation, .. }
            | Self::RootOpenUnheld { generation, .. }
            | Self::OfferReturned { generation, .. }
            | Self::HolderSet { generation, .. }
            | Self::LastChildDone { generation, .. }
            | Self::ChildDoneBriefUnmet { generation, .. }
            | Self::ItemDone { generation, .. }
            | Self::EnteredReview { generation, .. }
            | Self::RootClosed { generation, .. }
            | Self::ProgressNoted { generation, .. }
            | Self::ProposalExpired { generation, .. }
            | Self::DirectionRejectedWithReason { generation, .. }
            | Self::ShaperAccepted { generation, .. }
            | Self::RulesChanged { generation }
            | Self::AgentChanged { generation, .. }
            | Self::DraftDeclined { generation, .. }
            | Self::ProfileChanged { generation, .. }
            | Self::RootLedgerChanged { generation, .. }
            | Self::MemberJoined { generation, .. }
            | Self::DmOpened { generation, .. } => generation,
        }
    }

    /// Object key for JOBS (item, slug, room, or `community`).
    pub fn job_key(&self) -> String {
        match self {
            Self::DirectionConfirmed { slug, .. } => format!("direction:{slug:?}"),
            Self::RootOpenUnheld { item, .. }
            | Self::OfferReturned { item, .. }
            | Self::HolderSet { item, .. }
            | Self::ItemDone { item, .. }
            | Self::ProgressNoted { item, .. } => item.clone(),
            Self::LastChildDone { parent, .. } | Self::ChildDoneBriefUnmet { parent, .. } => {
                parent.clone()
            }
            Self::EnteredReview { root, .. }
            | Self::RootClosed { root, .. }
            | Self::RootLedgerChanged { root, .. } => root.clone(),
            Self::ProposalExpired { proposal, .. }
            | Self::DirectionRejectedWithReason { proposal, .. } => proposal.clone(),
            Self::ShaperAccepted { .. } | Self::RulesChanged { .. } | Self::AgentChanged { .. } => {
                "community".into()
            }
            Self::DraftDeclined { gap, .. } => gap.clone(),
            Self::ProfileChanged { pubkey, .. } => pubkey.clone(),
            Self::MemberJoined { p, .. } => p.clone(),
            Self::DmOpened { room, .. } => room.clone(),
        }
    }
}

pub(crate) fn emit(
    state: &OrgState,
    prev: Option<&Decoded>,
    next: &Decoded,
    event_id: &str,
    event_pubkey: &str,
    tags: &[Vec<String>],
) -> Vec<Transition> {
    let mut out = Vec::new();
    match (prev, next) {
        (prev, Decoded::Direction(artifact)) => {
            let prev_v = match prev {
                Some(Decoded::Direction(p)) => Some(p.version),
                _ => None,
            };
            if prev_v != Some(artifact.version) {
                let diff = prev_v
                    .map(|v| format!("v{v}->v{}", artifact.version))
                    .unwrap_or_default();
                out.push(Transition::DirectionConfirmed {
                    slug: artifact.slug,
                    version: artifact.version,
                    diff,
                    generation: event_id.to_owned(),
                });
            }
        }
        (prev, Decoded::WorkItem(item)) => {
            out.extend(for_work_item(
                match prev {
                    Some(Decoded::WorkItem(p)) => Some(p),
                    _ => None,
                },
                item,
                event_id,
                &state.items,
                &state.drafts,
            ));
        }
        (prev, Decoded::Proposal(proposal)) => {
            let prev_status = match prev {
                Some(Decoded::Proposal(p)) => Some(p.status),
                _ => None,
            };
            if prev_status == Some(ProposalStatus::Open)
                && proposal.status == ProposalStatus::Expired
            {
                out.push(Transition::ProposalExpired {
                    proposal: proposal.id.clone(),
                    opened_by: proposal.opened_by.clone(),
                    generation: event_id.to_owned(),
                });
            }
            if proposal.kind == ProposalKind::Direction
                && proposal.status == ProposalStatus::Rejected
                && prev_status != Some(ProposalStatus::Rejected)
                && state
                    .vote_reasons
                    .get(&proposal.id)
                    .is_some_and(|rs| rs.iter().any(|r| r == "direction"))
            {
                out.push(Transition::DirectionRejectedWithReason {
                    proposal: proposal.id.clone(),
                    reason: "direction".into(),
                    generation: event_id.to_owned(),
                });
            }
        }
        (prev, Decoded::Shapers(next_s)) => {
            let prev_s = match prev {
                Some(Decoded::Shapers(s)) => Some(s),
                _ => None,
            };
            let prev_set: std::collections::BTreeSet<&str> = prev_s
                .map(|s| s.shapers.iter().map(String::as_str).collect())
                .unwrap_or_default();
            for p in &next_s.shapers {
                if !prev_set.contains(p.as_str()) {
                    out.push(Transition::ShaperAccepted {
                        p: p.clone(),
                        generation: event_id.to_owned(),
                    });
                }
            }
            if let Some(prev_s) = prev_s {
                if prev_s.rules != next_s.rules
                    || prev_s.decision_window_secs != next_s.decision_window_secs
                    || prev_s.offer_window_secs != next_s.offer_window_secs
                {
                    out.push(Transition::RulesChanged {
                        generation: event_id.to_owned(),
                    });
                }
                if prev_s.agent != next_s.agent {
                    out.push(Transition::AgentChanged {
                        from: prev_s.agent.clone(),
                        to: next_s.agent.clone(),
                        generation: event_id.to_owned(),
                    });
                }
            }
        }
        (prev, Decoded::DraftOutcome(outcome)) => {
            let prev_status = match prev {
                Some(Decoded::DraftOutcome(o)) => Some(o.status),
                _ => None,
            };
            if outcome.status == DraftOutcomeStatus::Declined
                && prev_status != Some(DraftOutcomeStatus::Declined)
            {
                let gap = state
                    .drafts
                    .get(&outcome.draft)
                    .map(|d| d.gap.clone())
                    .unwrap_or_default();
                out.push(Transition::DraftDeclined {
                    gap,
                    reason: outcome.reason,
                    fingerprint: String::new(),
                    generation: event_id.to_owned(),
                });
            }
        }
        (prev, Decoded::Profile(profile)) => {
            let changed = match prev {
                Some(Decoded::Profile(p)) => p != profile,
                _ => true,
            };
            if changed {
                out.push(Transition::ProfileChanged {
                    pubkey: profile.pubkey.clone(),
                    generation: event_id.to_owned(),
                });
            }
        }
        (_, Decoded::Membership) => {
            // MemberJoined is emitted by OrgState after it diffs the set.
        }
        (_, Decoded::DmOpen) => {
            let members: Vec<&str> = tags
                .iter()
                .filter(|t| t.first().map(String::as_str) == Some("p"))
                .filter_map(|t| t.get(1).map(String::as_str))
                .collect();
            if state.me.as_deref().is_some_and(|me| members.contains(&me)) {
                let room = tags
                    .iter()
                    .find(|t| t.first().map(String::as_str) == Some("d"))
                    .and_then(|t| t.get(1))
                    .cloned()
                    .unwrap_or_else(|| format!("dm:{}", members.join("+")));
                out.push(Transition::DmOpened {
                    room,
                    generation: event_id.to_owned(),
                });
            }
        }
        _ => {}
    }
    let _ = (event_pubkey, tags);
    out
}

fn for_work_item(
    prev: Option<&WorkItem>,
    item: &WorkItem,
    event_id: &str,
    items: &std::collections::BTreeMap<String, WorkItem>,
    drafts: &std::collections::BTreeMap<String, DraftRecord>,
) -> Vec<Transition> {
    let mut out = Vec::new();
    let gen = event_id.to_owned();
    let became = |want: WorkItemState| item.state == want && prev.is_none_or(|p| p.state != want);

    if item.parent.is_none()
        && item.state == WorkItemState::Open
        && item.dri.is_none()
        && prev.is_none_or(|p| p.state != WorkItemState::Open || p.dri.is_some())
    {
        out.push(Transition::RootOpenUnheld {
            item: item.id.clone(),
            generation: gen.clone(),
        });
    }
    if item.parent.is_none()
        && item.state == WorkItemState::Open
        && prev.is_some_and(|p| p.state == WorkItemState::Offered)
    {
        out.push(Transition::OfferReturned {
            item: item.id.clone(),
            to: prev.and_then(|p| p.offered_to.clone()),
            generation: gen.clone(),
        });
    }
    if became(WorkItemState::Accepted) {
        if let Some(dri) = item.dri.clone() {
            out.push(Transition::HolderSet {
                item: item.id.clone(),
                dri,
                generation: gen.clone(),
            });
        }
    }
    if became(WorkItemState::Done) {
        out.push(Transition::ItemDone {
            item: item.id.clone(),
            closed_by: item.closed_by,
            generation: gen.clone(),
        });
        if item.parent.is_none() {
            out.push(Transition::RootClosed {
                root: item.id.clone(),
                closed_by: item.closed_by,
                generation: gen.clone(),
            });
        } else if let Some(parent) = item.parent.as_deref() {
            if !has_live_children(items, parent, &item.id) {
                out.push(Transition::LastChildDone {
                    parent: parent.to_owned(),
                    generation: gen.clone(),
                });
            }
            if parent_brief_unmet(drafts, parent) {
                out.push(Transition::ChildDoneBriefUnmet {
                    parent: parent.to_owned(),
                    generation: gen.clone(),
                });
            }
        }
    }
    if item.parent.is_none() && became(WorkItemState::InReview) {
        out.push(Transition::EnteredReview {
            root: item.id.clone(),
            generation: gen.clone(),
        });
    }
    let prev_progress = prev.and_then(|p| p.last_progress.as_deref());
    if item.last_progress.as_deref() != prev_progress && item.last_progress.is_some() {
        out.push(Transition::ProgressNoted {
            item: item.id.clone(),
            hint: None,
            merged_into: None,
            generation: gen,
        });
    }
    out
}

fn has_live_children(
    items: &std::collections::BTreeMap<String, WorkItem>,
    parent: &str,
    just_done: &str,
) -> bool {
    items.values().any(|i| {
        i.id != just_done
            && i.parent.as_deref() == Some(parent)
            && matches!(
                i.state,
                WorkItemState::Open
                    | WorkItemState::Offered
                    | WorkItemState::Accepted
                    | WorkItemState::InReview
            )
    })
}

fn parent_brief_unmet(
    drafts: &std::collections::BTreeMap<String, DraftRecord>,
    parent: &str,
) -> bool {
    use buzz_core::intelligent_org::DraftPayload;
    drafts.values().any(|d| match &d.payload {
        DraftPayload::Ticket(t) if t.parent == parent => {
            t.coverage.iter().any(|c| c.covered_by.is_none())
        }
        _ => false,
    })
}
