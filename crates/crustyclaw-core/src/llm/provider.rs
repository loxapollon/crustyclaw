//! LLM provider trait — the core abstraction for chat completions.
//!
//! All LLM backends (Anthropic, OpenAI, local models) implement this trait.
//! The daemon's skill engine dispatches through this interface.

use crate::BoxFuture;

use super::types::{ChatRequest, ChatResponse, StreamChunk};

/// Errors from LLM provider calls.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API request failed: {0}")]
    Request(String),

    #[error("authentication failed (check API key): {0}")]
    Auth(String),

    #[error("rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("context length exceeded: {0}")]
    ContextLength(String),

    #[error("response parse error: {0}")]
    Parse(String),

    #[error("provider error: {status} — {message}")]
    ProviderError { status: u16, message: String },

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout")]
    Timeout,
}

/// Core trait for LLM providers.
///
/// Implementations must be `Send + Sync` for use in the async daemon.
/// Uses `BoxFuture` for object safety (allows `Box<dyn LlmProvider>`).
pub trait LlmProvider: Send + Sync {
    /// Provider display name (e.g. "Anthropic", "OpenAI").
    fn name(&self) -> &str;

    /// Perform a chat completion (non-streaming).
    fn chat(&self, request: &ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>>;

    /// Perform a streaming chat completion.
    ///
    /// Returns a channel receiver that yields streaming chunks.
    fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> BoxFuture<'_, Result<tokio::sync::mpsc::Receiver<Result<StreamChunk, LlmError>>, LlmError>>;
}
