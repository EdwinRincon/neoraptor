//! Typed configuration with secret management.
//!
//! All secrets are wrapped in `secrecy::Secret<T>` at deserialization boundaries.

pub mod llm;

pub use llm::{LlmConfig, LlmModelConfig, LlmProviderKind};
