-- Migration 003: artifact manifest metadata baseline for T10
-- Filesystem remains the source of truth for exports/ and screenshots/ payloads.

CREATE TABLE IF NOT EXISTS artifact_manifests (
    id TEXT PRIMARY KEY,
    artifact_type TEXT NOT NULL,
    logical_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    preview_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (trim(file_path) <> ''),
    CHECK (trim(relative_path) <> ''),
    CHECK (
        relative_path LIKE 'exports/%'
        OR relative_path LIKE 'screenshots/%'
    )
);

CREATE INDEX IF NOT EXISTS idx_artifact_manifests_type ON artifact_manifests(artifact_type);
CREATE INDEX IF NOT EXISTS idx_artifact_manifests_created_at ON artifact_manifests(created_at);
