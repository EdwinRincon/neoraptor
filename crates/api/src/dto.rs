//! Data transfer objects for API requests and responses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Newtype wrapper for JSON payloads to avoid naked serde_json::Value in public API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonPayload(pub Value);

impl From<Value> for JsonPayload {
    fn from(value: Value) -> Self {
        JsonPayload(value)
    }
}

impl From<JsonPayload> for Value {
    fn from(payload: JsonPayload) -> Self {
        payload.0
    }
}

/// Request to create a new test run.
#[derive(Debug, Deserialize)]
pub struct CreateRunRequest {
    /// Human-readable name for the run.
    pub name: String,
    /// Goal or objective of this run.
    pub goal: String,
}

/// Response after creating a run.
#[derive(Debug, Serialize)]
pub struct CreateRunResponse {
    /// Unique identifier for the created run.
    pub run_id: Uuid,
}

/// Request to append an event to a run.
#[derive(Debug, Deserialize)]
pub struct AppendEventRequest {
    /// Type descriptor for the event (e.g., "scan_started", "action_completed").
    pub payload_type: String,
    /// Event payload as JSON.
    pub payload: JsonPayload,
}

/// Response after appending an event.
#[derive(Debug, Serialize)]
pub struct AppendEventResponse {
    /// Event ID of the newly appended event.
    pub event_id: i64,
}

/// Query parameters for listing events.
#[derive(Debug, Deserialize)]
pub struct ListEventsQuery {
    /// Optional cursor to read events from (exclusive).
    pub from_id: Option<i64>,
}

/// Event DTO for API responses.
#[derive(Debug, Serialize)]
pub struct EventDto {
    /// Event ID.
    pub id: i64,
    /// ISO 8601 timestamp.
    pub timestamp: DateTime<Utc>,
    /// Event payload type.
    pub payload_type: String,
    /// Event payload as JSON.
    pub payload: JsonPayload,
}

/// Response containing a list of events.
#[derive(Debug, Serialize)]
pub struct ListEventsResponse {
    /// Events matching the query.
    pub events: Vec<EventDto>,
}

/// Temporary helper for embedding run_id in event payloads.
///
/// **TEMPORARY (v0.1 / Week 3):** This helper embeds run_id in event payloads as JSON
/// to enable filtering without a proper schema. This will be replaced in Week 5+ when
/// we introduce proper `flow_runs` and event metadata tables/columns.
///
/// Migration path: move run scoping into DB schema (foreign keys, indexed columns)
/// instead of parsing JSON payloads.
pub struct EventPayloadHelper;

impl EventPayloadHelper {
    /// Wrap a user payload with run_id for filtering.
    ///
    /// Creates: `{ "run_id": "<uuid>", "data": <original_payload> }`
    pub fn wrap(run_id: Uuid, payload: JsonPayload) -> JsonPayload {
        serde_json::json!({
            "run_id": run_id.to_string(),
            "data": payload.0
        })
        .into()
    }

    /// Extract run_id from a wrapped payload.
    ///
    /// Returns `Some(uuid)` if the payload has the expected structure, `None` otherwise.
    pub fn extract_run_id(payload: &JsonPayload) -> Option<Uuid> {
        payload
            .0
            .get("run_id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }

    /// Create a run_created event payload.
    ///
    /// This is appended when a new run is created via POST /internal/runs.
    pub fn run_created(run_id: Uuid, name: String, goal: String) -> JsonPayload {
        serde_json::json!({
            "run_id": run_id.to_string(),
            "name": name,
            "goal": goal
        })
        .into()
    }
}
