//! LLM configuration types.

use secrecy::Secret;
use serde::{Deserialize, Serialize};

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderKind {
    /// OpenAI (GPT-4, GPT-3.5, etc.)
    OpenAi,

    /// Anthropic (Claude)
    Anthropic,

    /// Ollama (local models)
    Ollama,

    /// Perplexity
    Perplexity,
}

/// Model configuration for an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelConfig {
    /// Primary model for general tasks.
    pub primary: String,

    /// Secondary model for fallback or lower-priority tasks.
    pub secondary: Option<String>,

    /// Model for embedding generation.
    pub embedding: Option<String>,
}

/// Configuration for an LLM provider.
///
/// Note: Clone and Serialize are not derived because `Secret<String>` does not
/// implement the required traits in secrecy 0.8. Custom serialization will be
/// added in a later iteration.
#[derive(Debug)]
pub struct LlmConfig {
    /// Provider kind.
    pub provider: LlmProviderKind,

    /// API key (wrapped in Secret<T> to prevent accidental logging).
    pub api_key: Secret<String>,

    /// Optional base URL (for self-hosted or custom endpoints).
    pub base_url: Option<String>,

    /// Model configuration.
    pub models: LlmModelConfig,
}
