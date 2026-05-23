//! Route modules for the internal API.

pub mod artifacts;
pub mod events;
pub mod runs;

use axum::Router;

use crate::AppState;

/// Create the routes router with all sub-routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(runs::router())
        .merge(events::router())
        .merge(artifacts::router())
}
