-- Migration 001: Initial Schema
-- Creates all core tables for TestForge
-- Based on spec section 2 (Database Schema)

-- IMPORTANT: This migration must be idempotent.
-- Do NOT store screenshot blobs in the database.
-- All timestamps are ISO 8601 format with timezone.

-- Start: 001_initial_schema
-- Previous versions: None
-- Applied at: Initial creation

-- Table: environments
-- Purpose: Store environment configurations (development, staging, production)
CREATE TABLE IF NOT EXISTS environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    env_type TEXT NOT NULL DEFAULT 'development'
        CHECK (env_type IN ('development', 'staging', 'production', 'custom')),
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for faster environment lookups
CREATE INDEX IF NOT EXISTS idx_environments_type ON environments(env_type);
CREATE INDEX IF NOT EXISTS idx_environments_default ON environments(is_default);

-- Table: environment_variables
-- Purpose: Store variables for each environment (including secrets)
CREATE TABLE IF NOT EXISTS environment_variables (
    id TEXT PRIMARY KEY,
    environment_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    masked_preview TEXT,
    var_type TEXT NOT NULL DEFAULT 'regular'
        CHECK (var_type IN ('regular', 'secret')),
    enabled INTEGER NOT NULL DEFAULT 1,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (trim(key) <> ''),
    CHECK (var_type = 'regular' OR masked_preview IS NOT NULL),
    
    FOREIGN KEY (environment_id) REFERENCES environments(id) ON DELETE CASCADE
);

-- Unique constraint: one key per environment
CREATE UNIQUE INDEX IF NOT EXISTS idx_env_vars_unique 
    ON environment_variables(environment_id, key);

-- Index for faster lookups by environment
CREATE INDEX IF NOT EXISTS idx_env_vars_env ON environment_variables(environment_id);

-- Table: api_collections
-- Purpose: Group API endpoints into collections
CREATE TABLE IF NOT EXISTS api_collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    parent_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for parent lookup
CREATE INDEX IF NOT EXISTS idx_collections_parent ON api_collections(parent_id);

-- Table: api_endpoints
-- Purpose: Store API endpoint definitions
CREATE TABLE IF NOT EXISTS api_endpoints (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    method TEXT NOT NULL DEFAULT 'GET',
    url TEXT NOT NULL,
    headers_json TEXT DEFAULT '{}',
    body_type TEXT DEFAULT 'none',
    body_json TEXT,
    auth_type TEXT DEFAULT 'none',
    auth_config_json TEXT DEFAULT '{}',
    timeout_ms INTEGER DEFAULT 30000,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (collection_id) REFERENCES api_collections(id) ON DELETE CASCADE
);

-- Index for collection lookup
CREATE INDEX IF NOT EXISTS idx_endpoints_collection ON api_endpoints(collection_id);

-- Table: assertions
-- Purpose: Store assertions for API endpoints
CREATE TABLE IF NOT EXISTS assertions (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL,
    name TEXT NOT NULL,
    assertion_type TEXT NOT NULL,
    target TEXT NOT NULL,
    operator TEXT NOT NULL,
    expected_value TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (endpoint_id) REFERENCES api_endpoints(id) ON DELETE CASCADE
);

-- Index for endpoint lookup
CREATE INDEX IF NOT EXISTS idx_assertions_endpoint ON assertions(endpoint_id);

-- Table: ui_scripts
-- Purpose: Store UI automation scripts
CREATE TABLE IF NOT EXISTS ui_scripts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    start_url TEXT,
    viewport_width INTEGER DEFAULT 1280,
    viewport_height INTEGER DEFAULT 720,
    timeout_ms INTEGER DEFAULT 30000,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Table: ui_script_steps
-- Purpose: Store individual steps for UI scripts
CREATE TABLE IF NOT EXISTS ui_script_steps (
    id TEXT PRIMARY KEY,
    script_id TEXT NOT NULL,
    step_order INTEGER NOT NULL DEFAULT 0,
    step_type TEXT NOT NULL,
    selector TEXT,
    value TEXT,
    timeout_ms INTEGER DEFAULT 5000,
    description TEXT,
    confidence TEXT DEFAULT 'high',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (script_id) REFERENCES ui_scripts(id) ON DELETE CASCADE
);

-- Index for script lookup
CREATE INDEX IF NOT EXISTS idx_ui_steps_script ON ui_script_steps(script_id);
CREATE INDEX IF NOT EXISTS idx_ui_steps_order ON ui_script_steps(script_id, step_order);

-- Table: data_tables
-- Purpose: Store data-driven test data tables
CREATE TABLE IF NOT EXISTS data_tables (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    columns_json TEXT NOT NULL DEFAULT '[]',
    CHECK (trim(name) <> ''),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Table: data_table_rows
-- Purpose: Store rows for data tables
CREATE TABLE IF NOT EXISTS data_table_rows (
    id TEXT PRIMARY KEY,
    data_table_id TEXT NOT NULL,
    row_json TEXT NOT NULL DEFAULT '{}',
    row_index INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (row_index >= 0),
    
    FOREIGN KEY (data_table_id) REFERENCES data_tables(id) ON DELETE CASCADE
);

-- Index for table lookup
CREATE INDEX IF NOT EXISTS idx_data_rows_table ON data_table_rows(data_table_id);
CREATE INDEX IF NOT EXISTS idx_data_rows_order ON data_table_rows(data_table_id, row_index);

-- Table: test_cases
-- Purpose: Store test case definitions (can reference API endpoint or UI script)
CREATE TABLE IF NOT EXISTS test_cases (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    case_type TEXT NOT NULL DEFAULT 'api',
    api_endpoint_id TEXT,
    ui_script_id TEXT,
    data_table_id TEXT,
    tags_json TEXT DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (api_endpoint_id) REFERENCES api_endpoints(id) ON DELETE SET NULL,
    FOREIGN KEY (ui_script_id) REFERENCES ui_scripts(id) ON DELETE SET NULL,
    FOREIGN KEY (data_table_id) REFERENCES data_tables(id) ON DELETE SET NULL
);

-- Index for type lookup
CREATE INDEX IF NOT EXISTS idx_test_cases_type ON test_cases(case_type);

-- Table: test_suites
-- Purpose: Store test suite definitions
CREATE TABLE IF NOT EXISTS test_suites (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Table: suite_cases
-- Purpose: Link test cases to suites
CREATE TABLE IF NOT EXISTS suite_cases (
    id TEXT PRIMARY KEY,
    suite_id TEXT NOT NULL,
    case_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (suite_id) REFERENCES test_suites(id) ON DELETE CASCADE,
    FOREIGN KEY (case_id) REFERENCES test_cases(id) ON DELETE CASCADE
);

-- Unique constraint: one case per suite
CREATE UNIQUE INDEX IF NOT EXISTS idx_suite_cases_unique 
    ON suite_cases(suite_id, case_id);

-- Index for suite lookup
CREATE INDEX IF NOT EXISTS idx_suite_cases_suite ON suite_cases(suite_id);

-- Table: test_runs
-- Purpose: Store test run history
CREATE TABLE IF NOT EXISTS test_runs (
    id TEXT PRIMARY KEY,
    suite_id TEXT,
    environment_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    total_cases INTEGER DEFAULT 0,
    passed INTEGER DEFAULT 0,
    failed INTEGER DEFAULT 0,
    skipped INTEGER DEFAULT 0,
    started_at TEXT,
    completed_at TEXT,
    duration_ms INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (suite_id) REFERENCES test_suites(id) ON DELETE SET NULL,
    FOREIGN KEY (environment_id) REFERENCES environments(id)
);

-- Index for suite lookup
CREATE INDEX IF NOT EXISTS idx_test_runs_suite ON test_runs(suite_id);
CREATE INDEX IF NOT EXISTS idx_test_runs_status ON test_runs(status);

-- Table: test_run_results
-- Purpose: Store individual test case results
CREATE TABLE IF NOT EXISTS test_run_results (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    case_id TEXT NOT NULL,
    data_row_id TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    duration_ms INTEGER DEFAULT 0,
    request_log_json TEXT,
    response_log_json TEXT,
    assertion_results_json TEXT,
    screenshots_json TEXT DEFAULT '[]',
    error_message TEXT,
    error_code TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (run_id) REFERENCES test_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (case_id) REFERENCES test_cases(id),
    FOREIGN KEY (data_row_id) REFERENCES data_table_rows(id) ON DELETE SET NULL
);

-- Index for run lookup
CREATE INDEX IF NOT EXISTS idx_run_results_run ON test_run_results(run_id);
CREATE INDEX IF NOT EXISTS idx_run_results_case ON test_run_results(case_id);

-- Migration tracking is recorded exclusively by MigrationRunner using the
-- exact filename and the SQL file checksum after this script executes.
