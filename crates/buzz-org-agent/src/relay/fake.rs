//! In-memory relay the harness and pipeline tests drive.

use std::collections::VecDeque;

use buzz_core::Event;

use super::link::{RelayIo, StreamReq};

/// Result of a publish against the fake (or a live OK).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishAck {
    /// Event id.
    pub event_id: String,
    /// `OK` accepted.
    pub accepted: bool,
    /// Reason when refused or timed out.
    pub message: String,
}

/// An in-process relay: stores events, yields them to the subscriber,
/// and can simulate a disconnect.
#[derive(Debug, Default)]
pub struct FakeRelay {
    stored: Vec<Event>,
    inbox: VecDeque<Event>,
    published: Vec<Event>,
    connected: bool,
    /// When true, `publish` returns a timeout and does not store.
    pub fail_publish: bool,
    subscriptions: Vec<StreamReq>,
}

impl FakeRelay {
    /// Empty relay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed events the next subscribe will yield.
    pub fn seed(&mut self, events: impl IntoIterator<Item = Event>) {
        for e in events {
            self.stored.push(e.clone());
            if self.connected {
                self.inbox.push_back(e);
            }
        }
    }

    /// Events this agent published (accepted).
    pub fn published(&self) -> &[Event] {
        &self.published
    }

    /// Simulate a drop. The outbox keeps signed events.
    pub fn simulate_disconnect(&mut self) {
        self.connected = false;
        self.inbox.clear();
    }
}

impl RelayIo for FakeRelay {
    fn connect(&mut self) -> Result<(), String> {
        self.connected = true;
        Ok(())
    }

    fn subscribe(&mut self, reqs: &[StreamReq]) -> Result<(), String> {
        if !self.connected {
            return Err("disconnected".into());
        }
        self.subscriptions = reqs.to_vec();
        self.inbox.clear();
        for e in &self.stored {
            let kind = u32::from(e.kind.as_u16());
            let ts = e.created_at.as_secs();
            let wanted = reqs
                .iter()
                .any(|r| r.kinds.contains(&kind) && r.since.is_none_or(|s| ts >= s));
            if wanted {
                self.inbox.push_back(e.clone());
            }
        }
        Ok(())
    }

    fn next_event(&mut self) -> Result<Option<Event>, String> {
        if !self.connected {
            return Err("disconnected".into());
        }
        Ok(self.inbox.pop_front())
    }

    fn publish(&mut self, event: Event) -> Result<PublishAck, String> {
        if !self.connected || self.fail_publish {
            return Ok(PublishAck {
                event_id: event.id.to_hex(),
                accepted: false,
                message: "timeout".into(),
            });
        }
        let id = event.id.to_hex();
        self.stored.push(event.clone());
        self.published.push(event.clone());
        self.inbox.push_back(event);
        Ok(PublishAck {
            event_id: id,
            accepted: true,
            message: String::new(),
        })
    }

    fn disconnect(&mut self) {
        self.simulate_disconnect();
    }

    fn connected(&self) -> bool {
        self.connected
    }
}
