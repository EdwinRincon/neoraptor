//! Run creation endpoints.

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use bytes::Bytes;
use ports::{Event, EventBus};
use std::time::SystemTime;
use uuid::Uuid;

use crate::{
    dto::{CreateRunRequest, CreateRunResponse, EventPayloadHelper},
    error::ApiError,
    AppState,
};

/// Create the runs router.
pub fn router() -> Router<AppState> {
    Router::new().route("/internal/runs", post(create_run))
}

/// POST /internal/runs - Create a new test run.
///
/// Creates a run by generating a UUID and appending a `run_created` event to the event log.
async fn create_run(
    State(state): State<AppState>,
    Json(req): Json<CreateRunRequest>,
) -> Result<(StatusCode, Json<CreateRunResponse>), ApiError> {
    let run_id = Uuid::new_v4();

    // Create run_created event payload
    let payload = EventPayloadHelper::run_created(run_id, req.name, req.goal);
    let payload_bytes = serde_json::to_vec(&payload.0)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize payload: {}", e)))?;

    // Append event to event bus
    let event = Event {
        id: 0, // Will be assigned by the event bus
        timestamp: SystemTime::now(),
        payload_type: "run_created".to_string(),
        payload: Bytes::from(payload_bytes),
    };

    state.event_bus().append(event).await?;

    Ok((StatusCode::CREATED, Json(CreateRunResponse { run_id })))
}
