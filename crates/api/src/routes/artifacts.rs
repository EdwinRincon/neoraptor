//! Artifact endpoints (stub implementation).

use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::AppState;

/// Create the artifacts router.
pub fn router() -> Router<AppState> {
    Router::new().route(
        "/internal/actions/{action_id}/artifacts",
        get(list_artifacts),
    )
}

/// GET /internal/actions/:action_id/artifacts - List artifacts for an action.
///
/// **Stub implementation**: Returns a placeholder message until action-to-artifact
/// schema is implemented in Week 5+.
///
/// TODO: Implement when action-to-artifact mapping schema exists in the database.
async fn list_artifacts(Path(_action_id): Path<Uuid>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "message": "Artifact listing not yet implemented. Coming in Week 5 with action-to-artifact schema."
        })),
    )
}
