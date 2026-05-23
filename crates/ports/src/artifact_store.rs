//! Artifact store port trait and associated types.

use bytes::Bytes;
use uuid::Uuid;

/// Errors that can occur during artifact storage operations.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactStoreError {
    /// Failed to store artifact.
    #[error("store failed: {0}")]
    StoreFailed(String),

    /// Artifact not found.
    #[error("artifact not found: {0}")]
    NotFound(Uuid),

    /// Failed to retrieve artifact.
    #[error("retrieve failed: {0}")]
    RetrieveFailed(String),

    /// Invalid artifact ID.
    #[error("invalid artifact id: {0}")]
    InvalidId(String),
}

/// Unique identifier for an artifact.
pub type ArtifactId = Uuid;

/// Binary artifact with metadata.
#[derive(Debug, Clone)]
pub struct Artifact {
    /// Unique artifact identifier.
    pub id: ArtifactId,

    /// Content type (MIME type or custom descriptor, e.g., "application/octet-stream", "text/plain").
    pub content_type: String,

    /// Binary data of the artifact.
    pub data: Bytes,
}

/// Abstract trait for artifact storage and retrieval.
///
/// Uses `trait_variant::make` to produce a Send-bound variant safe for
/// `Arc<dyn ArtifactStore>` dispatch across `tokio::spawn` boundaries.
#[trait_variant::make(ArtifactStore: Send)]
pub trait LocalArtifactStore {
    /// Store an artifact.
    ///
    /// # Arguments
    ///
    /// * `artifact` - The artifact to store
    ///
    /// # Returns
    ///
    /// The artifact ID of the stored artifact (same as artifact.id).
    ///
    /// # Errors
    ///
    /// Returns `ArtifactStoreError::StoreFailed` if storage fails.
    async fn store(&self, artifact: Artifact) -> Result<ArtifactId, ArtifactStoreError>;

    /// Retrieve an artifact by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The artifact ID to retrieve
    ///
    /// # Returns
    ///
    /// The requested artifact.
    ///
    /// # Errors
    ///
    /// Returns `ArtifactStoreError::NotFound` if the artifact doesn't exist,
    /// or `ArtifactStoreError::RetrieveFailed` on other failures.
    async fn retrieve(&self, id: ArtifactId) -> Result<Artifact, ArtifactStoreError>;
}
