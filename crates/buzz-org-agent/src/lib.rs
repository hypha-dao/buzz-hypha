//! The intelligent-organization **org agent** — Development plan slices A-*.
//!
//! A-1 lands the skeleton and ruler: [`OrgState`] and [`apply`](OrgState::apply)
//! with the § 5.2 table, [`RelayLink`], the outbox, [`ModelClient`],
//! [`judge`](judge::judge), [`route`], [`publish`](relay::publish),
//! [`FakeRelay`], the harness loader over E-1, and `run` / `dry-run` /
//! `replay` / `doctor`. Nothing drafts.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod inbound;
pub mod jobs_impl;
pub mod judge;
pub mod pipeline;
pub mod relay;
pub mod route;
pub mod state;
pub mod think;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

pub use config::Config;
pub use error::AgentError;
pub use inbound::{decode, ApplyError, Decoded};
pub use judge::{judge, Draft, JudgeReason};
pub use pipeline::{HandleOutcome, OrgAgent};
pub use relay::{FakeRelay, Outbox, Permitted, RelayLink};
pub use state::{OrgState, Transition};
pub use think::{BuzzAgentModel, ModelClient, Recorded};

#[cfg(test)]
mod judge_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod transition_tests;

#[cfg(test)]
#[path = "../tests/eval_fixtures.rs"]
mod eval_fixtures;

#[cfg(test)]
#[path = "../tests/eval_cases.rs"]
mod eval_cases;
