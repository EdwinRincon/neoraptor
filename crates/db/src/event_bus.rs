//! Postgres-backed EventBus implementation.

use crate::DbPool;
use chrono::{DateTime, Utc};
use ports::{Event, EventBus, EventBusError, EventId};
use sqlx::Row;
use std::time::SystemTime;

/// Postgres implementation of the EventBus port.
pub struct PostgresEventBus {
    pool: DbPool,
}

impl PostgresEventBus {
    /// Create a new PostgresEventBus from a database pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl EventBus for PostgresEventBus {
    async fn append(&self, event: Event) -> Result<EventId, EventBusError> {
        // Convert SystemTime to chrono::DateTime for sqlx binding
        let duration_since_epoch = event
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| EventBusError::AppendFailed(format!("invalid timestamp: {}", e)))?;
        let timestamp = DateTime::<Utc>::from_timestamp(
            duration_since_epoch.as_secs() as i64,
            duration_since_epoch.subsec_nanos(),
        )
        .ok_or_else(|| EventBusError::AppendFailed("timestamp out of range".to_string()))?;

        // TODO: switch to query! once prepare-sqlx is run
        let row = sqlx::query(
            r#"
            INSERT INTO event_log (timestamp, payload_type, payload)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(timestamp)
        .bind(&event.payload_type)
        .bind(event.payload.as_ref())
        .fetch_one(self.pool.inner())
        .await
        .map_err(|e| EventBusError::AppendFailed(e.to_string()))?;

        let id: i64 = row
            .try_get("id")
            .map_err(|e| EventBusError::AppendFailed(e.to_string()))?;

        Ok(id)
    }

    async fn read_from(&self, cursor: EventId) -> Result<Vec<Event>, EventBusError> {
        // TODO: switch to query! once prepare-sqlx is run
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, payload_type, payload
            FROM event_log
            WHERE id > $1
            ORDER BY id ASC
            "#,
        )
        .bind(cursor)
        .fetch_all(self.pool.inner())
        .await
        .map_err(|e| EventBusError::ReadFailed(e.to_string()))?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row
                .try_get("id")
                .map_err(|e| EventBusError::ReadFailed(e.to_string()))?;
            let timestamp_db: DateTime<Utc> = row
                .try_get("timestamp")
                .map_err(|e| EventBusError::ReadFailed(e.to_string()))?;
            let payload_type: String = row
                .try_get("payload_type")
                .map_err(|e| EventBusError::ReadFailed(e.to_string()))?;
            let payload_bytes: Vec<u8> = row
                .try_get("payload")
                .map_err(|e| EventBusError::ReadFailed(e.to_string()))?;

            // Convert chrono DateTime to SystemTime
            let timestamp = SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(timestamp_db.timestamp() as u64)
                + std::time::Duration::from_nanos(timestamp_db.timestamp_subsec_nanos() as u64);

            events.push(Event {
                id,
                timestamp,
                payload_type,
                payload: bytes::Bytes::from(payload_bytes),
            });
        }

        Ok(events)
    }
}
