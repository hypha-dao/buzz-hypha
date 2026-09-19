//! Pipeline entry the harness and the live agent share (Org agent § 18).
//! A-1: handle a transition through JUDGE (and optional THINK hook).
//! Nothing drafts.

use std::path::PathBuf;

use crate::judge::{self, Draft, JudgeReason};
use crate::relay::link::{RelayIo, RelayLink};
use crate::relay::outbox::Outbox;
use crate::relay::publish::{sign, Permitted, PublishError};
use crate::state::{OrgState, Transition};
use crate::think::context::ContextBundle;
use crate::think::model::ModelClient;
use nostr::Keys;

/// What [`OrgAgent::handle`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleOutcome {
    /// Judge passed. A-1 does not publish a card.
    Accepted,
    /// Judge (or fence) dropped the result.
    Dropped {
        /// Reason code.
        reason: JudgeReason,
    },
    /// No draft was produced (A-1 default).
    NoDraft,
}

/// The process the harness and `run` share.
pub struct OrgAgent<M, R> {
    /// Read model.
    pub state: OrgState,
    /// Model port.
    pub model: M,
    /// Relay.
    pub link: RelayLink<R>,
    /// Durable outbox.
    pub outbox: Outbox,
    /// Agent key.
    pub keys: Keys,
    /// Hook the fencing test uses: run after the job snaps generation
    /// and before JUDGE, so a newer event can land mid-THINK.
    pub mid_think: Option<MidThinkHook>,
}

/// Test hook: mutate STATE after a job snaps generation.
pub type MidThinkHook = Box<dyn FnMut(&mut OrgState) + Send>;

impl<M: ModelClient, R: RelayIo> OrgAgent<M, R> {
    /// Build around an existing mirror, model, and transport.
    pub fn new(
        state: OrgState,
        model: M,
        io: R,
        hear: bool,
        state_dir: PathBuf,
        keys: Keys,
    ) -> Result<Self, PublishError> {
        Ok(Self {
            state,
            model,
            link: RelayLink::new(io, hear),
            outbox: Outbox::open(&state_dir)?,
            keys,
            mid_think: None,
        })
    }

    /// Production seam (Review-Proven rule 3): transitions enter here.
    pub fn handle(
        &mut self,
        transition: Transition,
        draft: Option<Draft>,
        mut bundle: ContextBundle,
    ) -> Result<HandleOutcome, crate::error::AgentError> {
        let snapped = transition.generation().to_owned();
        if let Some(mut hook) = self.mid_think.take() {
            hook(&mut self.state);
            self.mid_think = Some(hook);
        }
        let current = self
            .state
            .generation_of(&transition)
            .unwrap_or(transition.generation())
            .to_owned();
        bundle.generation = current.clone();
        let Some(mut draft) = draft else {
            return Ok(HandleOutcome::NoDraft);
        };
        draft.generation = snapped;
        match judge::judge(&self.state, &bundle, &draft) {
            Ok(()) => Ok(HandleOutcome::Accepted),
            Err(reason) => Ok(HandleOutcome::Dropped { reason }),
        }
    }

    /// Sign, append to the outbox, try to send. On timeout/disconnect the
    /// item stays; [`drain_outbox`] sends it after reconnect.
    pub fn publish_permitted(
        &mut self,
        kind: Permitted,
        content: &str,
        tags: Vec<nostr::Tag>,
        now: u64,
    ) -> Result<(), PublishError> {
        let event = sign(&self.keys, kind, content, tags)?;
        self.outbox.append(event.clone(), now)?;
        self.try_send(&event)
    }

    /// Drain the outbox after reconnect.
    pub fn drain_outbox(&mut self) -> Result<(), PublishError> {
        let ids: Vec<buzz_core::Event> = self
            .outbox
            .pending()
            .iter()
            .map(|i| i.event.clone())
            .collect();
        for event in ids {
            self.try_send(&event)?;
        }
        Ok(())
    }

    fn try_send(&mut self, event: &buzz_core::Event) -> Result<(), PublishError> {
        let id = event.id.to_hex();
        match self.link.io().publish(event.clone()) {
            Ok(ack) if ack.accepted => self.outbox.ack(&id),
            Ok(ack) if ack.message == "timeout" => {
                self.outbox.bump_attempt(&id)?;
                Ok(())
            }
            Ok(ack) => {
                self.outbox.drop_id(&id)?;
                Err(PublishError::Rejected(ack.message))
            }
            Err(_) => {
                self.outbox.bump_attempt(&id)?;
                Ok(())
            }
        }
    }

    /// Pull inbound events from the link and apply them.
    pub fn ingest_available(&mut self) -> Result<Vec<Transition>, crate::error::AgentError> {
        let mut out = Vec::new();
        loop {
            match self.link.io().next_event() {
                Ok(Some(event)) => out.extend(self.state.apply(&event)?),
                Ok(None) => break,
                Err(_) => break,
            }
        }
        Ok(out)
    }
}
