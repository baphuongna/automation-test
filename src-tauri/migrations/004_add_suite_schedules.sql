-- Migration 004: persisted suite schedules baseline for P2-T7
-- Scheduler state is stored locally in SQLite and reused across app restarts.

CREATE TABLE IF NOT EXISTS suite_schedules (
    id TEXT PRIMARY KEY,
    suite_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    cadence_minutes INTEGER NOT NULL,
    last_run_at TEXT,
    next_run_at TEXT,
    last_run_status TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_suite_schedules_enabled_next_run
    ON suite_schedules(enabled, next_run_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_suite_schedules_suite_env
    ON suite_schedules(suite_id, environment_id);
