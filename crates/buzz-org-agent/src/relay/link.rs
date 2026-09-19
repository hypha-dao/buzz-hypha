//! [`RelayLink`]: reconnect, watermarks, REQ set (Org agent § 4.1–4.2).
//!
//! `buzz-ws-client` has connect / NIP-42 / REQ / publish and no reconnect
//! (V12). This wraps it. Tests drive [`RelayIo`] through [`FakeRelay`].

use std::collections::BTreeMap;
use std::time::Duration;

use buzz_core::Event;
use serde_json::{json, Value};

use super::fake::PublishAck;
use crate::state::Stream;

/// Seconds subtracted from each watermark on reconnect (skew).
pub const WATERMARK_SKEW_SECS: u64 = 60;

/// Backoff: 1 s → 60 s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    current: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            current: Duration::from_secs(1),
        }
    }
}

impl Backoff {
    /// Current delay.
    pub fn current(self) -> Duration {
        self.current
    }

    /// Next delay, capped at 60 s.
    pub fn next(self) -> Self {
        Self {
            current: (self.current * 2).min(Duration::from_secs(60)),
        }
    }

    /// Reset after a successful connect.
    pub fn reset(self) -> Self {
        Self::default()
    }
}

/// One live REQ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamReq {
    /// Subscription id (`state`, `drafts`, …).
    pub id: String,
    /// Filter kinds.
    pub kinds: Vec<u32>,
    /// `since` = watermark − skew, when known.
    pub since: Option<u64>,
}

impl StreamReq {
    /// The § 4.2 REQ set for the current watermarks and HEAR flag.
    pub fn set(watermarks: &BTreeMap<Stream, u64>, hear: bool) -> Vec<Self> {
        let since = |s: Stream| {
            watermarks
                .get(&s)
                .copied()
                .map(|w| w.saturating_sub(WATERMARK_SKEW_SECS))
        };
        let mut reqs = vec![
            StreamReq {
                id: "state".into(),
                kinds: vec![39100, 39101, 39102, 39103, 39104, 39105],
                since: since(Stream::State),
            },
            StreamReq {
                id: "drafts".into(),
                kinds: vec![50100, 50101],
                since: since(Stream::Drafts),
            },
            StreamReq {
                id: "progress".into(),
                kinds: vec![50102],
                since: since(Stream::Progress),
            },
            StreamReq {
                id: "commands".into(),
                kinds: (50001..=50021).collect(),
                since: since(Stream::Commands),
            },
            StreamReq {
                id: "membership".into(),
                kinds: vec![13534, 39002],
                since: since(Stream::Membership),
            },
            StreamReq {
                id: "dm_control".into(),
                kinds: vec![41010, 41011, 39000],
                since: since(Stream::DmControl),
            },
        ];
        if hear {
            reqs.push(StreamReq {
                id: "talk".into(),
                kinds: vec![9, 40002],
                since: since(Stream::Talk),
            });
        }
        reqs
    }

    /// NIP-01 REQ JSON.
    pub fn to_req_json(&self) -> Value {
        let mut filter = json!({ "kinds": self.kinds });
        if let Some(since) = self.since {
            filter["since"] = json!(since);
        }
        json!(["REQ", self.id, filter])
    }
}

/// Transport the link owns. [`crate::relay::FakeRelay`] implements it;
/// the live path wraps `buzz_ws_client::NostrWsConnection`.
pub trait RelayIo {
    /// Open (or reopen) the socket.
    fn connect(&mut self) -> Result<(), String>;
    /// (Re)open the REQ set.
    fn subscribe(&mut self, reqs: &[StreamReq]) -> Result<(), String>;
    /// Next inbound event, if any.
    fn next_event(&mut self) -> Result<Option<Event>, String>;
    /// Publish one signed event.
    fn publish(&mut self, event: Event) -> Result<PublishAck, String>;
    /// Tear the socket down. Pending publishes stay in the outbox.
    fn disconnect(&mut self);
    /// Whether the socket is up.
    fn connected(&self) -> bool;
}

/// Reconnect + REQ owner.
pub struct RelayLink<T> {
    io: T,
    backoff: Backoff,
    hear: bool,
}

impl<T: RelayIo> RelayLink<T> {
    /// Wrap a transport.
    pub fn new(io: T, hear: bool) -> Self {
        Self {
            io,
            backoff: Backoff::default(),
            hear,
        }
    }

    /// Connect and open the REQ set from `watermarks`.
    pub fn connect(&mut self, watermarks: &BTreeMap<Stream, u64>) -> Result<(), String> {
        self.io.connect()?;
        self.io.subscribe(&StreamReq::set(watermarks, self.hear))?;
        self.backoff = self.backoff.reset();
        Ok(())
    }

    /// Reconnect after a drop: backoff, then REQ with `since = watermark − 60`.
    pub fn reconnect(&mut self, watermarks: &BTreeMap<Stream, u64>) -> Result<Duration, String> {
        self.io.disconnect();
        let wait = self.backoff.current();
        self.backoff = self.backoff.next();
        self.connect(watermarks)?;
        Ok(wait)
    }

    /// Borrow the transport.
    pub fn io(&mut self) -> &mut T {
        &mut self.io
    }

    /// Current backoff.
    pub fn backoff(&self) -> Backoff {
        self.backoff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_to_sixty() {
        let mut b = Backoff::default();
        assert_eq!(b.current(), Duration::from_secs(1));
        b = b.next();
        assert_eq!(b.current(), Duration::from_secs(2));
        for _ in 0..10 {
            b = b.next();
        }
        assert_eq!(b.current(), Duration::from_secs(60));
    }

    #[test]
    fn talk_req_is_absent_when_hear_is_off() {
        let reqs = StreamReq::set(&BTreeMap::new(), false);
        assert!(reqs.iter().all(|r| r.id != "talk"));
        let reqs = StreamReq::set(&BTreeMap::new(), true);
        assert!(reqs.iter().any(|r| r.id == "talk"));
    }

    #[test]
    fn reconnect_since_subtracts_skew() {
        let mut wm = BTreeMap::new();
        wm.insert(Stream::State, 1_000);
        let reqs = StreamReq::set(&wm, false);
        let state = reqs.iter().find(|r| r.id == "state").expect("state");
        assert_eq!(state.since, Some(1_000 - WATERMARK_SKEW_SECS));
    }
}
