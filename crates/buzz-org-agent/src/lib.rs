//! The intelligent-organization **org agent** — Development plan slices A-*.
//!
//! This crate is the home of `OrgState`, the transition table, the jobs,
//! and the evaluation harness described in
//! `docs/intelligent-org/architecture/intelligent-org-agent.md`. Slice E-1
//! lands it with only the evaluation fixtures under `tests/eval/` and the
//! [`fixtures`] loader the harness and A-1's `OrgState::apply` tests read;
//! A-1 (skeleton and ruler) fills in the agent itself.

pub mod fixtures;
