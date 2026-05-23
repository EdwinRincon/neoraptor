//! Event management endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use ports::{Event, EventBus};
use std::time::SystemTime;
use uuid::Uuid;

use crate::{
    dto::{
        AppendEventRequest, AppendEventResponse, EventDto, EventPayloadHelper, JsonPayload,
        ListEventsQuery, ListEventsResponse,
    },
    error::ApiError,
    AppState,
};

/// Create the events router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/internal/runs/:run_id/events", post(append_event))
        .route("/internal/runs/:run_id/events", get(list_events))
}

/// POST /internal/runs/:run_id/events - Append an event to a run.
///
/// Embeds the run_id in the event payload for filtering (temporary approach).
async fn append_event(
    Path(run_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<AppendEventRequest>,
) -> Result<(StatusCode, Json<AppendEventResponse>), ApiError> {
    // Wrap payload with run_id for filtering
    let wrapped_payload = EventPayloadHelper::wrap(run_id, req.payload);
    let payload_bytes = serde_json::to_vec(&wrapped_payload.0)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize payload: {}", e)))?;

    // Append event to event bus
    let event = Event {
        id: 0, // Will be assigned by the event bus
        timestamp: SystemTime::now(),
        payload_type: req.payload_type,
        payload: Bytes::from(payload_bytes),
    };

    let event_id = state.event_bus().append(event).await?;

    Ok((StatusCode::CREATED, Json(AppendEventResponse { event_id })))
}

/// GET /internal/runs/:run_id/events - List events for a run.
///
/// Reads from event log and filters by run_id embedded in payload (temporary approach).
async fn list_events(
    Path(run_id): Path<Uuid>,
    Query(query): Query<ListEventsQuery>,
    State(state): State<AppState>,
) -> Result<Json<ListEventsResponse>, ApiError> {
    // Read events from cursor
    let cursor = query.from_id.unwrap_or(0);
    let events = state.event_bus().read_from(cursor).await?;

    // Filter and map to DTOs
    let filtered_events: Vec<EventDto> = events
        .into_iter()
        .filter_map(|event| {
            // Parse payload
            let payload: serde_json::Value = serde_json::from_slice(&event.payload).ok()?;
            let json_payload = JsonPayload(payload);

            // Filter by run_id
            if EventPayloadHelper::extract_run_id(&json_payload)? != run_id {
                return None;
            }

            // Convert SystemTime to DateTime<Utc>
            let timestamp: DateTime<Utc> = event.timestamp.into();

            Some(EventDto {
                id: event.id,
                timestamp,
                payload_type: event.payload_type,
                payload: json_payload,
            })
        })
        .collect();

    Ok(Json(ListEventsResponse {
        events: filtered_events,
    }))
}
