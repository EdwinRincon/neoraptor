//! Event bus port trait and associated types.

use bytes::Bytes;
use std::time::SystemTime;

/// Errors that can occur during event bus operations.
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    /// Failed to append event to the log.
    #[error("append failed: {0}")]
    AppendFailed(String),

    /// Failed to read events from the log.
    #[error("read failed: {0}")]
    ReadFailed(String),

    /// Invalid or out-of-range cursor.
    #[error("invalid cursor: {0}")]
    InvalidCursor(String),

    /// Event serialization or deserialization failed.
    #[error("serialization error: {0}")]
    SerializationError(String),
}

/// Event identifier used as a cursor for reading events.
///
/// Events are assigned monotonically increasing IDs by the event bus.
pub type EventId = i64;

/// An event in the append-only event log.
#[derive(Debug, Clone)]
pub struct Event {
    /// Unique event identifier (cursor).
    pub id: EventId,

    /// Timestamp when the event was created.
    pub timestamp: SystemTime,

    /// Type descriptor for the event payload (e.g., "FlowCreated", "ActionCompleted").
    pub payload_type: String,

    /// Serialized event payload as bytes.
    pub payload: Bytes,
}

/// Abstract trait for append-only event bus.
///
/// Uses `trait_variant::make` to produce a Send-bound variant safe for
/// `Arc<dyn EventBus>` dispatch across `tokio::spawn` boundaries.
#[trait_variant::make(EventBus: Send)]
pub trait LocalEventBus {
    /// Append an event to the log.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to append (id will be assigned by the bus)
    ///
    /// # Returns
    ///
    /// The assigned event ID (cursor) for the newly appended event.
    ///
    /// # Errors
    ///
    /// Returns `EventBusError::AppendFailed` if the event cannot be appended.
    async fn append(&self, event: Event) -> Result<EventId, EventBusError>;

    /// Read events starting from a given cursor.
    ///
    /// # Arguments
    ///
    /// * `cursor` - Event ID to start reading from (inclusive)
    ///
    /// # Returns
    ///
    /// A vector of events starting from the cursor. May be empty if no events exist.
    ///
    /// # Errors
    ///
    /// Returns `EventBusError::ReadFailed` or `EventBusError::InvalidCursor` on failure.
    async fn read_from(&self, cursor: EventId) -> Result<Vec<Event>, EventBusError>;
}
