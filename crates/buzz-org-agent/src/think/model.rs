//! [`ModelClient`], [`BuzzAgentModel`], [`Recorded`] (Org agent § 3.1).

use std::pin::Pin;
use std::sync::Mutex;

use buzz_agent::llm::{CompleteOverrides, Llm};
use buzz_agent::types::{HistoryItem, ToolDef};
use serde_json::Value;
use thiserror::Error;

/// Draft vs fast tier (Org agent § 3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// THINK produce calls.
    Draft,
    /// HEAR classify / THINK-R plan / cheap answers.
    Fast,
}

/// One user or retrieval turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Role (`user` / `assistant` / `tool`).
    pub role: String,
    /// Text.
    pub content: String,
}

/// A structured-output request.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    /// Which tier.
    pub tier: Tier,
    /// Rendered prompt.
    pub system: String,
    /// Context + retrieval turns.
    pub messages: Vec<Message>,
    /// `emit_<move>` JSON Schema (as a value).
    pub schema: Value,
    /// Forced tool name (`emit_<move>`). Written as the raw `tool_choice`
    /// wire string (A-0 follow-up: not the provider forced-tool object).
    pub tool_name: String,
    /// THINK-R read tools; empty for THINK-0.
    pub tools: Vec<Value>,
    /// 0.2 drafts, 0.0 classify.
    pub temperature: f32,
    /// Cap.
    pub max_output_tokens: u32,
}

/// Token usage the provider reported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usage {
    /// Input tokens, when known.
    pub input_tokens: Option<u64>,
    /// Output tokens, when known.
    pub output_tokens: Option<u64>,
}

/// Structured model output.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelOutput {
    /// The `emit_<move>` arguments.
    pub value: Value,
    /// Usage.
    pub usage: Usage,
    /// Model id.
    pub model: String,
}

/// Why a model call failed.
#[derive(Debug, Error)]
pub enum ModelError {
    /// Provider / HTTP.
    #[error("model: {0}")]
    Provider(String),
    /// No tool call and no parseable JSON.
    #[error("schema")]
    Schema,
    /// The recorded tape ran out.
    #[error("recorded tape is empty")]
    TapeEmpty,
}

/// The model port. The harness replaces it with [`Recorded`].
pub trait ModelClient: Send + Sync {
    /// One structured call.
    fn structured<'a>(
        &'a self,
        req: ModelRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ModelOutput, ModelError>> + Send + 'a>>;
}

/// `buzz-agent`'s [`Llm`] — `complete` / `complete_with`, not `summarize`.
pub struct BuzzAgentModel {
    llm: Llm,
    cfg: buzz_agent::config::Config,
    draft_model: String,
    fast_model: String,
}

impl BuzzAgentModel {
    /// Build from buzz-agent's env config and the org-agent tier ids.
    pub fn new(
        cfg: buzz_agent::config::Config,
        draft_model: impl Into<String>,
        fast_model: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let llm = Llm::new(&cfg).map_err(|e| ModelError::Provider(e.to_string()))?;
        Ok(Self {
            llm,
            draft_model: draft_model.into(),
            fast_model: fast_model.into(),
            cfg,
        })
    }
}

impl ModelClient for BuzzAgentModel {
    fn structured<'a>(
        &'a self,
        req: ModelRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ModelOutput, ModelError>> + Send + 'a>>
    {
        Box::pin(async move {
            let model = match req.tier {
                Tier::Draft => self.draft_model.as_str(),
                Tier::Fast => self.fast_model.as_str(),
            };
            let history: Vec<HistoryItem> = req
                .messages
                .iter()
                .map(|m| HistoryItem::User(m.content.clone()))
                .collect();
            let tool = ToolDef {
                name: req.tool_name.clone(),
                description: "emit the move as structured output".into(),
                input_schema: req.schema.clone(),
            };
            // A-0 follow-up: `tool_choice: Some(s)` is the raw wire string.
            let overrides = CompleteOverrides {
                temperature: Some(req.temperature),
                tool_choice: Some(req.tool_name.clone()),
            };
            let _ = (req.tools, req.max_output_tokens);
            let resp = self
                .llm
                .complete_with(&self.cfg, &req.system, &history, &[tool], model, overrides)
                .await
                .map_err(|e| ModelError::Provider(e.to_string()))?;
            let value = resp
                .tool_calls
                .first()
                .map(|c| c.arguments.clone())
                .or_else(|| serde_json::from_str(&resp.text).ok())
                .ok_or(ModelError::Schema)?;
            Ok(ModelOutput {
                value,
                usage: Usage {
                    input_tokens: resp.input_tokens,
                    output_tokens: resp.output_tokens,
                },
                model: model.to_owned(),
            })
        })
    }
}

/// Harness client: replays taped responses; `EVAL_LIVE=1` is A-2+.
pub struct Recorded {
    tape: Mutex<Vec<ModelOutput>>,
}

impl Recorded {
    /// Responses in the order `structured` will return them.
    pub fn new(responses: Vec<ModelOutput>) -> Self {
        Self {
            tape: Mutex::new(responses),
        }
    }

    /// Push a response onto the end of the tape.
    pub fn push(&self, output: ModelOutput) {
        if let Ok(mut tape) = self.tape.lock() {
            tape.push(output);
        }
    }
}

impl ModelClient for Recorded {
    fn structured<'a>(
        &'a self,
        _req: ModelRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ModelOutput, ModelError>> + Send + 'a>>
    {
        Box::pin(async move {
            let mut tape = self
                .tape
                .lock()
                .map_err(|_| ModelError::Provider("recorded lock".into()))?;
            if tape.is_empty() {
                return Err(ModelError::TapeEmpty);
            }
            Ok(tape.remove(0))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recorded_replays_in_order() {
        let client = Recorded::new(vec![ModelOutput {
            value: serde_json::json!({"ok": true}),
            usage: Usage::default(),
            model: "taped".into(),
        }]);
        let out = client
            .structured(ModelRequest {
                tier: Tier::Draft,
                system: String::new(),
                messages: vec![],
                schema: serde_json::json!({}),
                tool_name: "emit_ticket".into(),
                tools: vec![],
                temperature: 0.2,
                max_output_tokens: 256,
            })
            .await
            .expect("tape");
        assert_eq!(out.model, "taped");
        assert_eq!(out.value["ok"], true);
    }

    #[test]
    fn tool_choice_override_is_the_raw_wire_string() {
        let overrides = CompleteOverrides {
            temperature: Some(0.2),
            tool_choice: Some("emit_ticket".into()),
        };
        assert_eq!(overrides.tool_choice.as_deref(), Some("emit_ticket"));
    }
}
