//! THINK — context, model client. A-1 lands the client, not the prompts.

pub mod context;
pub mod model;

pub use context::{Candidate, ContextBundle, Receipt};
pub use model::{
    BuzzAgentModel, Message, ModelClient, ModelError, ModelOutput, ModelRequest, Recorded, Tier,
    Usage,
};
