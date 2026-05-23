//! Abstract port traits for pluggable external integrations.
//!
//! All traits use `#[trait_variant::make(TraitName: Send)]` to produce
//! Send-bound variants safe for `Arc<dyn Trait>` dispatch across `tokio::spawn`.

pub mod artifact_store;
pub mod event_bus;
pub mod llm;
pub mod sandbox;

// Re-export LLM types
pub use llm::{CompletionRequest, CompletionResponse, LlmError, LlmProvider};

// Re-export Sandbox types
pub use sandbox::{
    ExecutionResult, SandboxCommand, SandboxError, SandboxProfileId, SandboxRuntime,
};

// Re-export EventBus types
pub use event_bus::{Event, EventBus, EventBusError, EventId};

// Re-export ArtifactStore types
pub use artifact_store::{Artifact, ArtifactId, ArtifactStore, ArtifactStoreError};
