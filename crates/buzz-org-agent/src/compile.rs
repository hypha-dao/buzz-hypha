//! State cards and gap verdicts. No model call.
//!
//! A live root covers an objective only when its `objective_ref` cites that
//! line. Plan-piece coverage stays empty until a passed project stores a plan.

use std::collections::BTreeMap;

use buzz_core::intelligent_org::{DirectionSlug, WorkItemState};

use crate::state::OrgState;

/// How far a live project covers one objective line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapVerdict {
    /// No live root cites the line.
    Uncovered,
    /// A live root cites it, and none of those roots are accepted yet.
    Partly,
    /// An accepted or in-review root cites the line.
    Covered,
}

/// The organization card: one head per direction slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgCard {
    /// Slug to the `39100` event id, when that head exists.
    pub event_ids: BTreeMap<String, String>,
    /// Direction block the chat overview prints.
    pub direction: String,
}

/// One objective line and whether a live root cites it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveCard {
    /// Line id on the objectives head.
    pub line_id: String,
    /// Event id of the `39100` the line came from.
    pub event_id: String,
    /// The line text.
    pub text: String,
    /// Version of the head.
    pub version: u32,
    /// Cite verdict.
    pub verdict: GapVerdict,
}

/// One live root. Piece coverage is empty until a plan is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCard {
    /// Item id.
    pub id: String,
    /// Event id of the latest `39101`.
    pub event_id: String,
    /// Title.
    pub title: String,
    /// Plan pieces that have a live or done child. Empty until P-1.
    pub pieces: Vec<String>,
}

/// The picture `compile` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Organization card.
    pub org: OrgCard,
    /// One card per objective line.
    pub objectives: Vec<ObjectiveCard>,
    /// One card per live root.
    pub roots: Vec<RootCard>,
}

const SLUGS: [&str; 5] = ["mission", "vision", "situation", "objectives", "strategy"];

/// Compile the current picture. Verdicts come from cites, not from a model.
pub fn compile(state: &OrgState) -> Snapshot {
    let direction = render_direction(state);
    let mut event_ids = BTreeMap::new();
    for slug in SLUGS {
        if let Some(head) = state.direction.get(slug) {
            event_ids.insert(slug.to_string(), head.event_id.clone());
        }
    }
    Snapshot {
        org: OrgCard {
            event_ids,
            direction,
        },
        objectives: objective_cards(state),
        roots: root_cards(state),
    }
}

/// Direction lines in the overview, from the heads and nothing else.
pub fn render_direction(state: &OrgState) -> String {
    let heads = SLUGS.map(|slug| {
        state.direction.get(slug).and_then(|head| {
            let body = head.artifact.body.trim();
            if body.is_empty() {
                None
            } else {
                Some((head.artifact.version, body.to_string()))
            }
        })
    });
    render_board_direction(heads)
}

/// The direction block chat prints. `heads` is mission, vision, situation,
/// objectives, strategy — `None` is "not set".
pub fn render_board_direction(heads: [Option<(u32, String)>; 5]) -> String {
    let mut lines = vec!["Direction:".to_string()];
    for (slug, head) in SLUGS.into_iter().zip(heads) {
        match head {
            Some((version, body)) => lines.push(format!("- {slug} v{version}: {body}")),
            None => lines.push(format!("- {slug}: not set")),
        }
    }
    lines.join("\n")
}

fn objective_cards(state: &OrgState) -> Vec<ObjectiveCard> {
    let Some(head) = state.direction.get("objectives") else {
        return Vec::new();
    };
    if head.artifact.slug != DirectionSlug::Objectives {
        return Vec::new();
    }
    head.artifact
        .lines
        .iter()
        .map(|line| ObjectiveCard {
            line_id: line.id.clone(),
            event_id: head.event_id.clone(),
            text: line.text.clone(),
            version: head.artifact.version,
            verdict: verdict_for(state, head.artifact.version, &line.id),
        })
        .collect()
}

fn verdict_for(state: &OrgState, version: u32, line_id: &str) -> GapVerdict {
    let cite = format!("objectives@{version}#{line_id}");
    let mut live = false;
    let mut accepted = false;
    for item in state.items.values() {
        if item.parent.is_some() || item.objective_ref.as_deref() != Some(cite.as_str()) {
            continue;
        }
        if !is_live(item.state) {
            continue;
        }
        live = true;
        if matches!(
            item.state,
            WorkItemState::Accepted | WorkItemState::InReview
        ) {
            accepted = true;
        }
    }
    if accepted {
        GapVerdict::Covered
    } else if live {
        GapVerdict::Partly
    } else {
        GapVerdict::Uncovered
    }
}

fn is_live(state: WorkItemState) -> bool {
    !matches!(state, WorkItemState::Withdrawn | WorkItemState::Done)
}

fn root_cards(state: &OrgState) -> Vec<RootCard> {
    let mut roots: Vec<RootCard> = state
        .items
        .values()
        .filter(|item| item.parent.is_none() && is_live(item.state))
        .map(|item| RootCard {
            id: item.id.clone(),
            event_id: state
                .item_generations
                .get(&item.id)
                .cloned()
                .unwrap_or_default(),
            title: item.title.clone(),
            // P-1 stores the plan. Until then no piece can be covered.
            pieces: Vec::new(),
        })
        .collect();
    roots.sort_by(|left, right| left.id.cmp(&right.id));
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{fixtures_dir, load_events};
    use crate::state::OrgState;
    use crate::test_support::item;
    use buzz_core::intelligent_org::{
        DirectionArtifact, DirectionLine, DirectionSlug, WorkItemState,
    };

    use crate::state::DirectionHead;

    fn objectives_head(version: u32, line_id: &str) -> DirectionHead {
        DirectionHead {
            artifact: DirectionArtifact {
                slug: DirectionSlug::Objectives,
                version,
                body: "Three outcomes.".into(),
                lines: vec![DirectionLine {
                    n: 1,
                    id: line_id.to_string(),
                    text: "A weekday hall".into(),
                    date: Some(1_780_000_000),
                    done_when: Some("one paid night".into()),
                    line_type: None,
                }],
                confirmed_by: "aa".repeat(32),
                confirmed_at: 1,
                proposed_by: "bb".repeat(32),
                proposal: "11111111-1111-4111-8111-111111111111".into(),
                prev: None,
            },
            event_id: "cc".repeat(32),
        }
    }

    #[test]
    fn river_seed_compiles_and_a_cite_covers_the_line() {
        let events = load_events(&fixtures_dir().join("orgs/river/seed.json")).expect("river");
        let river = OrgState::from_events(&events).expect("apply");
        let snapshot = compile(&river);
        assert!(snapshot.org.direction.contains("- mission"));
        assert!(snapshot.org.direction.contains("situation"));
        assert!(
            !snapshot.objectives.is_empty() || snapshot.org.event_ids.contains_key("objectives")
        );
        for card in &snapshot.objectives {
            assert!(!card.event_id.is_empty());
        }
        for root in &snapshot.roots {
            assert!(root.pieces.is_empty());
        }

        let mut bare = OrgState::default();
        bare.direction
            .insert("objectives".into(), objectives_head(2, "line-a"));
        let uncovered = compile(&bare);
        assert_eq!(
            uncovered.objectives[0].verdict,
            GapVerdict::Uncovered,
            "no citing root"
        );

        let mut cited = bare.clone();
        let mut root = item("root-1", None, WorkItemState::Accepted, Some("dd"));
        root.objective_ref = Some("objectives@2#line-a".into());
        cited.items.insert(root.id.clone(), root);
        let covered = compile(&cited);
        assert_eq!(covered.objectives[0].verdict, GapVerdict::Covered);
        assert_eq!(covered.objectives[0].event_id, "cc".repeat(32));

        let mut open = bare;
        let mut waiting = item("root-2", None, WorkItemState::Open, None);
        waiting.objective_ref = Some("objectives@2#line-a".into());
        open.items.insert(waiting.id.clone(), waiting);
        assert_eq!(compile(&open).objectives[0].verdict, GapVerdict::Partly);
    }
}
