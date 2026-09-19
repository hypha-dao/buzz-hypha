//! Crate-level error types.

use thiserror::Error;

use crate::inbound::ApplyError;
use crate::relay::publish::PublishError;
use crate::think::model::ModelError;

/// An error from the org-agent process.
#[derive(Debug, Error)]
pub enum AgentError {
    /// STATE refused an event.
    #[error(transparent)]
    Apply(#[from] ApplyError),
    /// Signing or the outbox failed.
    #[error(transparent)]
    Publish(#[from] PublishError),
    /// The model client failed.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// Configuration or environment.
    #[error("{0}")]
    Config(String),
    /// Local I/O (snapshot, outbox, state dir).
    #[error("{0}")]
    Io(String),
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
