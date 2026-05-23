-- Event log table for event sourcing.
--
-- All domain events (scope approved, command validated, sandbox invoked, etc.)
-- are appended here as an immutable log.

CREATE TABLE IF NOT EXISTS event_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    payload_type TEXT NOT NULL,
    payload BYTEA NOT NULL
);

-- Index for reading from a cursor (typical event replay pattern).
CREATE INDEX IF NOT EXISTS idx_event_log_id ON event_log(id);

-- Index for time-range queries (monitoring, dashboards, retention ops).
CREATE INDEX IF NOT EXISTS idx_event_log_timestamp ON event_log(timestamp DESC);