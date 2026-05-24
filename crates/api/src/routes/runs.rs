//! Run creation endpoints.

use agents_core::ScopeContract;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{CreateRunRequest, CreateRunResponse},
    error::ApiError,
    AppState,
};

/// Create the runs router.
pub fn router() -> Router<AppState> {
    Router::new().route("/internal/runs", post(create_run))
}

/// POST /internal/runs - Create a new test run.
///
/// Validates intent through scope contract, submits to supervisor for execution,
/// and returns the run ID.
async fn create_run(
    State(state): State<AppState>,
    Json(req): Json<CreateRunRequest>,
) -> Result<(StatusCode, Json<CreateRunResponse>), ApiError> {
    // Build description from request
    let description = format!("{}: {}", req.name, req.goal);

    // For Week 4: use a placeholder target
    // In production, this would come from the request or be validated against user permissions
    let target = "test-target.local".to_string();

    // Create scope contract for this run
    // For Week 4: allow the specific test target
    let scope = ScopeContract::builder()
        .allow_target(&target)
        .authorization_id(Uuid::new_v4())
        .build()
        .map_err(|e| ApiError::Internal(format!("Failed to build scope: {}", e)))?;

    // Submit run to supervisor
    let run_id = state
        .supervisor()
        .submit_run(description, target, Arc::new(scope))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to submit run: {}", e)))?;

    Ok((StatusCode::CREATED, Json(CreateRunResponse { run_id })))
}
