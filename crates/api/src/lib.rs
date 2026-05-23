//! NEORAPTOR internal API server.
//!
//! Provides HTTP endpoints for testing and exercising the persistence stack
//! (EventBus, ArtifactStore). This is an internal-only API for v0.1.

#![allow(missing_docs)]

mod dto;
mod error;
mod routes;

use axum::{routing::get, Router};
use db::{DbPool, PostgresArtifactStore, PostgresEventBus};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub use error::ApiError;

/// Application state shared across all request handlers.
///
/// Uses concrete Postgres implementations since api/ is the integration point.
#[derive(Clone)]
pub struct AppState {
    pool: DbPool,
    event_bus: Arc<PostgresEventBus>,
    artifact_store: Arc<PostgresArtifactStore>,
}

impl AppState {
    /// Create a new AppState with the given dependencies.
    pub fn new(
        pool: DbPool,
        event_bus: Arc<PostgresEventBus>,
        artifact_store: Arc<PostgresArtifactStore>,
    ) -> Self {
        Self {
            pool,
            event_bus,
            artifact_store,
        }
    }

    /// Get a reference to the database pool.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Get a reference to the event bus.
    pub fn event_bus(&self) -> &Arc<PostgresEventBus> {
        &self.event_bus
    }

    /// Get a reference to the artifact store.
    pub fn artifact_store(&self) -> &Arc<PostgresArtifactStore> {
        &self.artifact_store
    }
}

/// Create the application router with all routes and middleware.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .merge(routes::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "ok"
}
