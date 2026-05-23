//! OpenAI LLM provider adapter.

use ports::{CompletionRequest, CompletionResponse, LlmError, LlmProvider};

/// OpenAI adapter implementing the LlmProvider port.
///
/// This is a stub implementation for v0.1.0 foundation work.
/// Full implementation with HTTP client will be added in subsequent iterations.
#[derive(Debug, Clone)]
pub struct OpenAiAdapter {
    /// API key for OpenAI.
    #[allow(dead_code)]
    api_key: String,

    /// Optional base URL (for proxies or custom endpoints).
    #[allow(dead_code)]
    base_url: Option<String>,
}

impl OpenAiAdapter {
    /// Create a new OpenAI adapter.
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenAI API key
    /// * `base_url` - Optional custom base URL
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self { api_key, base_url }
    }
}

impl LlmProvider for OpenAiAdapter {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Stub implementation - will be replaced with real HTTP client in later iterations
        Err(LlmError::ServerError(
            "OpenAI adapter not yet implemented".to_string(),
        ))
    }
}
