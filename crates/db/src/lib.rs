#![allow(missing_docs)]

//! Database layer with Postgres-backed implementations of EventBus and ArtifactStore.

pub mod artifact_store;
mod connect;
pub mod event_bus;

pub use artifact_store::PostgresArtifactStore;
pub use event_bus::PostgresEventBus;

use sqlx::PgPool;
use std::time::Duration;

/// Database errors.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// Connection failed or pool configuration invalid.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Query execution failed.
    #[error("query failed: {0}")]
    QueryFailed(String),

    /// Row not found (maps to NotFound errors in port layer).
    #[error("not found")]
    NotFound,
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => DbError::NotFound,
            _ => DbError::QueryFailed(err.to_string()),
        }
    }
}

/// Newtype wrapper around sqlx::PgPool.
///
/// This provides a stable type for dependency injection and hides sqlx from public APIs.
#[derive(Clone)]
pub struct DbPool(PgPool);

impl DbPool {
    /// Returns a reference to the underlying PgPool.
    pub(crate) fn inner(&self) -> &PgPool {
        &self.0
    }
}

/// Connect to the database and return a validated pool.
///
/// Does NOT run migrations automatically. Migrations should be run explicitly
/// via `sqlx migrate run` or `just migrate`.
///
/// # Errors
///
/// Returns `DbError::ConnectionFailed` if:
/// - The connection URL is invalid
/// - The database is unreachable
/// - Pool configuration is invalid
#[tracing::instrument(skip(cfg), err)]
pub async fn connect_database(cfg: &config::DatabaseConfig) -> Result<DbPool, DbError> {
    use sqlx::postgres::PgPoolOptions;

    let pool = PgPoolOptions::new()
        .min_connections(cfg.pool.min_connections)
        .max_connections(cfg.pool.max_connections)
        .acquire_timeout(Duration::from_secs(cfg.pool.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(cfg.pool.idle_timeout_secs))
        .connect(connect::expose_database_url(cfg))
        .await
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

    Ok(DbPool(pool))
}
