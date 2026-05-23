//! Postgres-backed ArtifactStore implementation.

use crate::DbPool;
use ports::{Artifact, ArtifactId, ArtifactStore, ArtifactStoreError};
use sqlx::Row;

/// Postgres implementation of the ArtifactStore port.
pub struct PostgresArtifactStore {
    pool: DbPool,
}

impl PostgresArtifactStore {
    /// Create a new PostgresArtifactStore from a database pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl ArtifactStore for PostgresArtifactStore {
    async fn store(&self, artifact: Artifact) -> Result<ArtifactId, ArtifactStoreError> {
        // TODO: switch to query! once prepare-sqlx is run
        sqlx::query(
            r#"
            INSERT INTO artifacts (id, content_type, data)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(artifact.id)
        .bind(&artifact.content_type)
        .bind(artifact.data.as_ref())
        .execute(self.pool.inner())
        .await
        .map_err(|e| ArtifactStoreError::StoreFailed(e.to_string()))?;

        Ok(artifact.id)
    }

    async fn retrieve(&self, id: ArtifactId) -> Result<Artifact, ArtifactStoreError> {
        // TODO: switch to query! once prepare-sqlx is run
        let row = sqlx::query(
            r#"
            SELECT id, content_type, data
            FROM artifacts
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await
        .map_err(|e| ArtifactStoreError::RetrieveFailed(e.to_string()))?
        .ok_or(ArtifactStoreError::NotFound(id))?;

        let id: uuid::Uuid = row
            .try_get("id")
            .map_err(|e| ArtifactStoreError::RetrieveFailed(e.to_string()))?;
        let content_type: String = row
            .try_get("content_type")
            .map_err(|e| ArtifactStoreError::RetrieveFailed(e.to_string()))?;
        let data_bytes: Vec<u8> = row
            .try_get("data")
            .map_err(|e| ArtifactStoreError::RetrieveFailed(e.to_string()))?;

        Ok(Artifact {
            id,
            content_type,
            data: bytes::Bytes::from(data_bytes),
        })
    }
}
