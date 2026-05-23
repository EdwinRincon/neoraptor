-- Artifacts table for storing binary blobs (tool output, scan results, etc.).
--
-- UUIDs are used as identifiers to avoid enumeration attacks.

CREATE TABLE IF NOT EXISTS artifacts (
    id UUID PRIMARY KEY,
    content_type TEXT NOT NULL,
    data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Optional metadata (aligned with schema.dbml)
    kind TEXT,          -- "file", "report", "log", ...
    filename TEXT,
    size_bytes BIGINT
);

-- Primary key already indexes id; this is kept explicit for clarity.
CREATE INDEX IF NOT EXISTS idx_artifacts_id ON artifacts(id);