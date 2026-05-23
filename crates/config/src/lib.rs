//! Typed configuration with secret management.
//!
//! All secrets are wrapped in `secrecy::Secret<T>` at deserialization boundaries.

pub mod database;
pub mod llm;
pub mod sandbox;

// Re-export LLM types
pub use llm::{LlmConfig, LlmModelConfig, LlmProviderKind};

// Re-export Database types
pub use database::{DatabaseConfig, DatabasePoolConfig};

// Re-export Sandbox types
pub use sandbox::{SandboxConfig, SandboxProfileConfig};
