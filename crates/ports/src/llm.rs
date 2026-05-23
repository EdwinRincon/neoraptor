//! LLM provider port trait and associated types.

use std::time::Duration;

/// Errors that can occur during LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Rate limit exceeded; retry after the specified duration if provided.
    #[error("rate limited; retry after {retry_after:?}")]
    RateLimit {
        /// Optional duration to wait before retrying.
        retry_after: Option<Duration>,
    },

    /// Invalid or missing API key.
    #[error("invalid API key")]
    InvalidApiKey,

    /// Network error occurred.
    #[error("network error: {0}")]
    NetworkError(String),

    /// Server returned an error.
    #[error("server error: {0}")]
    ServerError(String),

    /// Request was invalid.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

/// Request for text completion.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// The prompt to complete.
    pub prompt: String,

    /// Maximum number of tokens to generate.
    pub max_tokens: Option<u32>,

    /// Sampling temperature (0.0 to 2.0).
    pub temperature: Option<f32>,

    /// Model identifier to use.
    pub model: Option<String>,
}

/// Response from text completion.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// Generated text.
    pub text: String,

    /// Total tokens used (prompt + completion).
    pub tokens_used: u32,
}

/// Abstract trait for LLM providers.
///
/// Uses `trait_variant::make` to produce a Send-bound variant safe for
/// `Arc<dyn LlmProvider>` dispatch across `tokio::spawn` boundaries.
#[trait_variant::make(LlmProvider: Send)]
pub trait LocalLlmProvider {
    /// Complete a text prompt.
    ///
    /// # Errors
    ///
    /// Returns `LlmError` if the request fails (rate limit, network, invalid API key, etc.).
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, LlmError>;
}
