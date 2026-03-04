//! Provider-agnostic LLM backend.
//!
//! Uses an enum dispatcher rather than `dyn Trait` because `send_structured<T>`
//! is generic, which Rust cannot put behind a vtable. The enum is `Clone` so it
//! can be cheaply shared across async tasks.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use crate::anthropic::AnthropicClient;
pub use crate::openai::OpenAIClient;

// ============================================================================
// Shared request / error types
// ============================================================================

/// Errors returned by any LLM provider.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error (status {status}): {body}")]
    Api { status: u16, body: String },

    #[error("Deserialisation error: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("No structured output in response")]
    NoStructuredOutput,

    #[error("No text content in response")]
    NoText,
}

/// A tool/function definition for structured output via tool-use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// "user" or "assistant"
    pub role: String,
    pub content: String,
}

/// A provider-agnostic LLM request.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub system: Option<String>,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: u32,
    /// If set, the model must call this tool and return structured JSON.
    pub tool: Option<LlmTool>,
}

// ============================================================================
// Enum dispatcher
// ============================================================================

/// Runtime-selected LLM backend. Add a variant here to support a new provider.
#[derive(Clone)]
pub enum LlmBackend {
    Anthropic(AnthropicClient),
    OpenAI(OpenAIClient),
}

impl LlmBackend {
    /// Send a request and deserialise the structured tool-use output as `T`.
    pub async fn send_structured<T>(&self, req: LlmRequest) -> Result<T, LlmError>
    where
        T: DeserializeOwned + Send,
    {
        match self {
            LlmBackend::Anthropic(c) => c.send_structured(req).await,
            LlmBackend::OpenAI(c) => c.send_structured(req).await,
        }
    }

    /// Send a request and return the first text response.
    pub async fn send_text(&self, req: LlmRequest) -> Result<String, LlmError> {
        match self {
            LlmBackend::Anthropic(c) => c.send_text(req).await,
            LlmBackend::OpenAI(c) => c.send_text(req).await,
        }
    }

    /// The model name in use (for logging).
    pub fn model(&self) -> &str {
        match self {
            LlmBackend::Anthropic(c) => c.model(),
            LlmBackend::OpenAI(c) => c.model(),
        }
    }
}
