-- Artifacts table for storing binary blobs (tool output, scan results, etc.).
--
-- UUIDs are used as identifiers to avoid enumeration attacks.

CREATE TABLE IF NOT EXISTS artifacts (
    id UUID PRIMARY KEY,
    content_type TEXT NOT NULL,
    data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for retrieval by ID (primary key already covers this, but explicit for clarity).
CREATE INDEX idx_artifacts_id ON artifacts(id);
