//! Abstract port traits for pluggable external integrations.
//!
//! All traits use `#[trait_variant::make(TraitName: Send)]` to produce
//! Send-bound variants safe for `Arc<dyn Trait>` dispatch across `tokio::spawn`.

pub mod llm;

pub use llm::{CompletionRequest, CompletionResponse, LlmError, LlmProvider};
