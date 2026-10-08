//! Jobs beside the chat loop. A transition becomes one fenced job.
//!
//! [`OrgAgent::handle`](crate::pipeline::OrgAgent::handle) enqueues.
//! [`JobQueue::finish`] compiles a verdict and does not call a model.
//! `dm_chat::serve` answers talk and does not hold [`OrgState`].

use buzz_core::intelligent_org::DirectionSlug;

use crate::compile::{self, Snapshot};
use crate::state::{OrgState, Transition};

/// What a finished job recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobLog {
    /// The generation still matched. The verdict is the compile.
    Verdict {
        /// Object key.
        key: String,
        /// Generation that matched.
        generation: String,
    },
    /// A newer version landed before this job finished. Nothing is published.
    Stale {
        /// Object key.
        key: String,
        /// Generation the job started with.
        generation: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Queued {
    key: String,
    generation: String,
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Direction,
    Holder,
    Review,
    Done,
    Prompt,
}

/// Coalesced queue. One pending job per object, one job in flight.
#[derive(Debug, Default)]
pub struct JobQueue {
    pending: Vec<Queued>,
    inflight: Option<Queued>,
    log: Vec<JobLog>,
}

impl JobQueue {
    /// Empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many jobs are waiting.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// No job is waiting.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Generation of the pending job for `key`, if one is waiting.
    pub fn generation_of(&self, key: &str) -> Option<&str> {
        self.pending
            .iter()
            .find(|job| job.key == key)
            .map(|job| job.generation.as_str())
    }

    /// Recorded finishes, oldest first.
    pub fn log(&self) -> &[JobLog] {
        &self.log
    }

    /// Enqueue a watched transition, and a prompt job when a ticket or a
    /// cited direction head changed. A second job for the same object
    /// replaces the one still waiting. Other transitions are ignored.
    pub fn enqueue(&mut self, transition: &Transition, state: &OrgState) -> bool {
        let mut enqueued = false;
        if let Some(job) = watched(transition, state) {
            self.upsert(job);
            enqueued = true;
        }
        for job in prompt_jobs(transition, state) {
            self.upsert(job);
            enqueued = true;
        }
        enqueued
    }

    fn upsert(&mut self, job: Queued) {
        self.pending.retain(|existing| existing.key != job.key);
        self.pending.push(job);
    }

    /// Take the oldest waiting job into flight. A job already in flight
    /// stays there until [`finish`](Self::finish).
    pub fn start(&mut self) -> bool {
        if self.inflight.is_some() || self.pending.is_empty() {
            return false;
        }
        self.inflight = Some(self.pending.remove(0));
        true
    }

    /// Compile if the in-flight job is still the newest generation. A stale
    /// job records `stale` and returns no snapshot, so nothing can be published.
    pub fn finish(&mut self, state: &OrgState) -> Option<Snapshot> {
        let job = self.inflight.take()?;
        let current = current_generation(state, &job);
        if current.as_deref() != Some(job.generation.as_str()) {
            self.log.push(JobLog::Stale {
                key: job.key,
                generation: job.generation,
            });
            return None;
        }
        let snapshot = compile::compile(state);
        self.log.push(JobLog::Verdict {
            key: job.key,
            generation: job.generation,
        });
        Some(snapshot)
    }

    /// Recompute verdicts for objectives, strategy, held roots, and roots
    /// already in review. Covers a transition missed while the process was down.
    pub fn monday_scan(&mut self, state: &OrgState) {
        for slug in ["objectives", "strategy"] {
            let Some(head) = state.direction.get(slug) else {
                continue;
            };
            let transition = Transition::DirectionConfirmed {
                slug: if slug == "objectives" {
                    DirectionSlug::Objectives
                } else {
                    DirectionSlug::Strategy
                },
                version: head.artifact.version,
                diff: String::new(),
                generation: head.event_id.clone(),
            };
            self.enqueue(&transition, state);
        }
        for item in state.items.values() {
            if item.parent.is_some() {
                continue;
            }
            let generation = state
                .item_generations
                .get(&item.id)
                .cloned()
                .unwrap_or_default();
            if generation.is_empty() {
                continue;
            }
            if matches!(
                item.state,
                buzz_core::intelligent_org::WorkItemState::InReview
            ) {
                self.enqueue(
                    &Transition::EnteredReview {
                        root: item.id.clone(),
                        generation,
                    },
                    state,
                );
            } else if let Some(dri) = item.dri.clone() {
                self.enqueue(
                    &Transition::HolderSet {
                        item: item.id.clone(),
                        dri,
                        generation,
                    },
                    state,
                );
            }
        }
        for item in state.items.values() {
            if item.parent.is_none() {
                continue;
            }
            if !matches!(
                item.state,
                buzz_core::intelligent_org::WorkItemState::Offered
                    | buzz_core::intelligent_org::WorkItemState::Accepted
            ) {
                continue;
            }
            let Some(generation) = state.item_generations.get(&item.id).cloned() else {
                continue;
            };
            if generation.is_empty() {
                continue;
            }
            self.enqueue(
                &Transition::TicketPrompt {
                    item: item.id.clone(),
                    generation,
                },
                state,
            );
        }
    }
}

fn watched(transition: &Transition, state: &OrgState) -> Option<Queued> {
    match transition {
        Transition::DirectionConfirmed {
            slug: DirectionSlug::Objectives,
            generation,
            ..
        } => Some(queued("direction:objectives", generation, Kind::Direction)),
        Transition::DirectionConfirmed {
            slug: DirectionSlug::Strategy,
            generation,
            ..
        } => Some(queued("direction:strategy", generation, Kind::Direction)),
        Transition::HolderSet {
            item, generation, ..
        } => {
            if state
                .items
                .get(item)
                .is_some_and(|found| found.parent.is_some())
            {
                return None;
            }
            Some(queued(&format!("holder:{item}"), generation, Kind::Holder))
        }
        Transition::EnteredReview { root, generation } => {
            Some(queued(&format!("review:{root}"), generation, Kind::Review))
        }
        Transition::ItemDone {
            item, generation, ..
        } => Some(queued(&format!("done:{item}"), generation, Kind::Done)),
        _ => None,
    }
}

fn prompt_jobs(transition: &Transition, state: &OrgState) -> Vec<Queued> {
    match transition {
        Transition::TicketPrompt { item, generation } => vec![queued(
            &format!("prompt:{item}"),
            &prompt_stamp(state, generation),
            Kind::Prompt,
        )],
        Transition::DirectionConfirmed {
            slug: DirectionSlug::Objectives | DirectionSlug::Strategy,
            ..
        } => state
            .items
            .values()
            .filter(|item| {
                item.parent.is_some()
                    && matches!(
                        item.state,
                        buzz_core::intelligent_org::WorkItemState::Offered
                            | buzz_core::intelligent_org::WorkItemState::Accepted
                    )
            })
            .filter_map(|item| {
                let generation = state.item_generations.get(&item.id)?;
                Some(queued(
                    &format!("prompt:{}", item.id),
                    &prompt_stamp(state, generation),
                    Kind::Prompt,
                ))
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn prompt_stamp(state: &OrgState, ticket_generation: &str) -> String {
    let objectives = state
        .direction
        .get("objectives")
        .map(|head| head.event_id.as_str())
        .unwrap_or("");
    let strategy = state
        .direction
        .get("strategy")
        .map(|head| head.event_id.as_str())
        .unwrap_or("");
    format!("{ticket_generation}|{objectives}|{strategy}")
}

fn queued(key: &str, generation: &str, kind: Kind) -> Queued {
    Queued {
        key: key.to_string(),
        generation: generation.to_string(),
        kind,
    }
}

fn current_generation(state: &OrgState, job: &Queued) -> Option<String> {
    match job.kind {
        Kind::Direction => {
            let slug = job.key.strip_prefix("direction:")?;
            state.direction.get(slug).map(|head| head.event_id.clone())
        }
        Kind::Holder => {
            let item = job.key.strip_prefix("holder:")?;
            state.item_generations.get(item).cloned()
        }
        Kind::Review => {
            let item = job.key.strip_prefix("review:")?;
            state.item_generations.get(item).cloned()
        }
        Kind::Done => {
            let item = job.key.strip_prefix("done:")?;
            state.item_generations.get(item).cloned()
        }
        Kind::Prompt => {
            let item = job.key.strip_prefix("prompt:")?;
            let ticket = state.item_generations.get(item)?;
            Some(prompt_stamp(state, ticket))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::DirectionHead;
    use crate::test_support::item;
    use buzz_core::intelligent_org::{
        DirectionArtifact, DirectionLine, DirectionSlug, WorkItemState,
    };

    fn head(event_id: &str) -> DirectionHead {
        DirectionHead {
            artifact: DirectionArtifact {
                slug: DirectionSlug::Objectives,
                version: 1,
                body: "Outcomes.".into(),
                lines: vec![DirectionLine {
                    n: 1,
                    id: "line-a".into(),
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
            event_id: event_id.to_string(),
        }
    }

    fn objectives_state(event_id: &str) -> OrgState {
        let mut state = OrgState::new();
        state.direction.insert("objectives".into(), head(event_id));
        state
    }

    #[test]
    fn monday_scan_recomputes_objectives_and_a_held_root() {
        let mut state = objectives_state("v1");
        let root = item("root-1", None, WorkItemState::Accepted, Some("dd"));
        state
            .item_generations
            .insert(root.id.clone(), "gen-root".into());
        state.items.insert(root.id.clone(), root);
        let child = item(
            "child-1",
            Some("root-1"),
            WorkItemState::Accepted,
            Some("ee"),
        );
        state
            .item_generations
            .insert(child.id.clone(), "gen-child".into());
        state.items.insert(child.id.clone(), child);
        let review = item("root-2", None, WorkItemState::InReview, Some("ff"));
        state
            .item_generations
            .insert(review.id.clone(), "gen-review".into());
        state.items.insert(review.id.clone(), review);
        let mut queue = JobQueue::new();
        queue.monday_scan(&state);
        assert_eq!(queue.generation_of("direction:objectives"), Some("v1"));
        assert_eq!(queue.generation_of("holder:root-1"), Some("gen-root"));
        assert_eq!(queue.generation_of("review:root-2"), Some("gen-review"));
        assert_eq!(queue.generation_of("prompt:child-1"), Some("gen-child|v1|"));
        assert!(queue.generation_of("holder:root-2").is_none());
        assert!(queue.generation_of("holder:child-1").is_none());
        assert!(queue.generation_of("prompt:root-1").is_none());
        assert!(queue.generation_of("direction:strategy").is_none());
    }

    #[test]
    fn a_matching_finish_records_the_verdict_and_a_child_holder_is_ignored() {
        let state = objectives_state("v1");
        let mut queue = JobQueue::new();
        assert!(queue.enqueue(
            &Transition::DirectionConfirmed {
                slug: DirectionSlug::Objectives,
                version: 1,
                diff: String::new(),
                generation: "v1".into(),
            },
            &state,
        ));
        assert!(queue.start());
        let snapshot = queue.finish(&state).expect("verdict");
        assert_eq!(
            snapshot.objectives[0].verdict,
            crate::compile::GapVerdict::Uncovered
        );
        assert!(matches!(
            queue.log()[0],
            JobLog::Verdict { ref generation, .. } if generation == "v1"
        ));

        let child = item(
            "child-1",
            Some("root-1"),
            WorkItemState::Accepted,
            Some("ee"),
        );
        let mut held = state;
        held.item_generations
            .insert(child.id.clone(), "gen-child".into());
        held.items.insert(child.id.clone(), child);
        assert!(!queue.enqueue(
            &Transition::HolderSet {
                item: "child-1".into(),
                dri: "ee".into(),
                generation: "gen-child".into(),
            },
            &held,
        ));
    }
}
