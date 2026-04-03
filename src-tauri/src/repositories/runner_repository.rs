use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::contracts::domain::{RunStatus, TestCaseType};
use serde_json::Value;

use crate::contracts::dto::{
    ArtifactManifestDto, FailureCategoryCountDto, RunCaseResultDto, RunDetailDto, RunHistoryDto,
    RunHistoryEntryDto, RunHistoryFilterDto, RunHistoryGroupSummaryDto, RunResultDto, SuiteDto,
    SuiteItemDto,
};
use crate::error::{Result, TestForgeError};
use crate::models::DataTableRow;

#[derive(Debug, Clone)]
pub struct PersistedSuiteCase {
    pub id: String,
    pub test_case_id: String,
    pub case_type: TestCaseType,
    pub order: i32,
    pub data_table_id: Option<String>,
    pub enabled: bool,
    pub api_endpoint_id: Option<String>,
    pub ui_script_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedSuite {
    pub dto: SuiteDto,
    pub cases: Vec<PersistedSuiteCase>,
}

#[derive(Debug, Clone)]
pub struct PersistedRunSummary {
    pub run_id: String,
    pub suite_id: Option<String>,
    pub suite_name: Option<String>,
    pub environment_id: String,
    pub environment_name: String,
    pub status: RunStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub total_count: u32,
    pub passed_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
}

#[derive(Debug, Clone)]
pub struct PersistedRunCaseResult {
    pub dto: RunCaseResultDto,
}

#[derive(Debug, Clone)]
pub struct FailedRunTarget {
    pub case_id: String,
    pub data_row_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RunStatusCounts {
    pub passed_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
    pub cancelled_count: u32,
    pub completed_count: u32,
}

pub struct RunnerRepository<'a> {
    conn: &'a Connection,
}

impl<'a> RunnerRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn load_suite(&self, suite_id: &str) -> Result<PersistedSuite> {
        let suite = self
            .conn
            .query_row(
                "SELECT id, name FROM test_suites WHERE id = ?1",
                params![suite_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| TestForgeError::Validation(format!("Suite not found: {suite_id}")))?;

        let mut stmt = self.conn.prepare(
            "SELECT sc.id, sc.case_id, tc.case_type, sc.sort_order, tc.data_table_id, tc.enabled, tc.api_endpoint_id, tc.ui_script_id
             FROM suite_cases sc
             JOIN test_cases tc ON tc.id = sc.case_id
             WHERE sc.suite_id = ?1
             ORDER BY sc.sort_order ASC",
        )?;

        let cases = stmt
            .query_map(params![suite_id], |row| {
                let case_type_raw: String = row.get(2)?;
                let case_type = match case_type_raw.as_str() {
                    "ui" => TestCaseType::Ui,
                    _ => TestCaseType::Api,
                };

                Ok(PersistedSuiteCase {
                    id: row.get(0)?,
                    test_case_id: row.get(1)?,
                    case_type,
                    order: row.get(3)?,
                    data_table_id: row.get(4)?,
                    enabled: row.get::<_, bool>(5)?,
                    api_endpoint_id: row.get(6)?,
                    ui_script_id: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let items = cases
            .iter()
            .map(|case| SuiteItemDto {
                id: case.id.clone(),
                test_case_id: case.test_case_id.clone(),
                r#type: case.case_type,
                order: case.order,
            })
            .collect::<Vec<_>>();

        Ok(PersistedSuite {
            dto: SuiteDto {
                id: suite.0,
                name: suite.1,
                items,
            },
            cases,
        })
    }

    pub fn load_enabled_data_rows(&self, data_table_id: &str) -> Result<Vec<DataTableRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, data_table_id, row_json, enabled, row_index, created_at, updated_at
             FROM data_table_rows
             WHERE data_table_id = ?1 AND enabled = 1
             ORDER BY row_index ASC",
        )?;

        let rows = stmt
            .query_map(params![data_table_id], |row| {
                Ok(DataTableRow {
                    id: row.get(0)?,
                    data_table_id: row.get(1)?,
                    values: row.get(2)?,
                    enabled: row.get(3)?,
                    row_index: row.get(4)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn list_suites(&self) -> Result<Vec<SuiteDto>> {
        let mut stmt = self.conn.prepare(
            "SELECT ts.id, ts.name, sc.id, sc.case_id, tc.case_type, sc.sort_order
             FROM test_suites ts
             LEFT JOIN suite_cases sc ON sc.suite_id = ts.id
             LEFT JOIN test_cases tc ON tc.id = sc.case_id
             ORDER BY ts.name COLLATE NOCASE ASC, sc.sort_order ASC",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i32>>(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut suites: Vec<SuiteDto> = Vec::new();
        for (suite_id, suite_name, suite_case_id, case_id, case_type_raw, order) in rows {
            let needs_new = suites
                .last()
                .map(|suite| suite.id != suite_id)
                .unwrap_or(true);

            if needs_new {
                suites.push(SuiteDto {
                    id: suite_id.clone(),
                    name: suite_name,
                    items: Vec::new(),
                });
            }

            if let (Some(item_id), Some(test_case_id), Some(case_type_text), Some(item_order)) =
                (suite_case_id, case_id, case_type_raw, order)
            {
                let item_type = match case_type_text.as_str() {
                    "ui" => TestCaseType::Ui,
                    _ => TestCaseType::Api,
                };

                if let Some(suite) = suites.last_mut() {
                    suite.items.push(SuiteItemDto {
                        id: item_id,
                        test_case_id,
                        r#type: item_type,
                        order: item_order,
                    });
                }
            }
        }

        Ok(suites)
    }

    pub fn list_run_history(&self, filter: RunHistoryFilterDto) -> Result<RunHistoryDto> {
        validate_run_history_filter(&filter)?;

        let status_filter = filter
            .status
            .map(status_to_db_value)
            .transpose()?
            .map(str::to_string);
        let base_query =
            "SELECT tr.id, tr.suite_id, ts.name, tr.environment_id, env.name, tr.status, tr.started_at, tr.completed_at, tr.total_cases, tr.passed, tr.failed, tr.skipped
             FROM test_runs tr
             LEFT JOIN test_suites ts ON ts.id = tr.suite_id
             JOIN environments env ON env.id = tr.environment_id
             WHERE (?1 IS NULL OR tr.suite_id = ?1)
               AND (?2 IS NULL OR tr.status = ?2)
               AND (?3 IS NULL OR julianday(COALESCE(tr.started_at, tr.created_at)) >= julianday(?3))
               AND (?4 IS NULL OR julianday(COALESCE(tr.started_at, tr.created_at)) <= julianday(?4))";
        let order_clause = " ORDER BY COALESCE(tr.completed_at, tr.started_at, tr.created_at) DESC, tr.created_at DESC";

        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<PersistedRunSummary> {
            let status_raw: String = row.get(5)?;
            let status = match status_raw.as_str() {
                "queued" => RunStatus::Queued,
                "running" => RunStatus::Running,
                "skipped" => RunStatus::Skipped,
                "passed" => RunStatus::Passed,
                "cancelled" => RunStatus::Cancelled,
                _ => RunStatus::Failed,
            };

            Ok(PersistedRunSummary {
                run_id: row.get(0)?,
                suite_id: row.get(1)?,
                suite_name: row.get(2)?,
                environment_id: row.get(3)?,
                environment_name: row.get(4)?,
                status,
                started_at: row
                    .get::<_, Option<String>>(6)?
                    .unwrap_or_else(|| Utc::now().to_rfc3339()),
                finished_at: row.get(7)?,
                total_count: row.get::<_, i64>(8)? as u32,
                passed_count: row.get::<_, i64>(9)? as u32,
                failed_count: row.get::<_, i64>(10)? as u32,
                skipped_count: row.get::<_, i64>(11)? as u32,
            })
        };

        let query = format!("{base_query}{order_clause}");
        let mut stmt = self.conn.prepare(&query)?;
        let mapped = stmt.query_map(
            params![
                filter.suite_id,
                status_filter,
                filter.started_after,
                filter.started_before
            ],
            map_row,
        )?;
        let items = mapped.collect::<std::result::Result<Vec<_>, _>>()?;
        let entries = items
            .clone()
            .into_iter()
            .map(Self::to_run_history_entry)
            .collect::<Vec<_>>();
        let group_summary = self.build_group_summary(&items, &filter)?;

        Ok(RunHistoryDto {
            entries,
            group_summary,
        })
    }

    pub fn create_suite_run(
        &self,
        run_id: &str,
        suite_id: &str,
        environment_id: &str,
        total_cases: u32,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO test_runs (id, suite_id, environment_id, status, total_cases, passed, failed, skipped, started_at, completed_at, duration_ms, created_at)
             VALUES (?1, ?2, ?3, 'queued', ?4, 0, 0, 0, ?5, NULL, 0, ?5)",
            params![run_id, suite_id, environment_id, total_cases, now],
        )?;
        Ok(())
    }

    pub fn update_run_summary(
        &self,
        run_id: &str,
        status: RunStatus,
        passed: u32,
        failed: u32,
        skipped: u32,
        completed_at: Option<&str>,
    ) -> Result<()> {
        let status_text = match status {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Skipped => "skipped",
            RunStatus::Passed => "passed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Idle => "idle",
        };

        self.conn.execute(
            "UPDATE test_runs
             SET status = ?2, passed = ?3, failed = ?4, skipped = ?5, completed_at = COALESCE(?6, completed_at)
             WHERE id = ?1",
            params![run_id, status_text, passed, failed, skipped, completed_at],
        )?;
        Ok(())
    }

    pub fn update_run_summary_if_active(
        &self,
        run_id: &str,
        status: RunStatus,
        passed: u32,
        failed: u32,
        skipped: u32,
        completed_at: Option<&str>,
    ) -> Result<bool> {
        let status_text = match status {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Skipped => "skipped",
            RunStatus::Passed => "passed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Idle => "idle",
        };

        let changed = self.conn.execute(
            "UPDATE test_runs
             SET status = ?2, passed = ?3, failed = ?4, skipped = ?5, completed_at = COALESCE(?6, completed_at)
             WHERE id = ?1 AND completed_at IS NULL",
            params![run_id, status_text, passed, failed, skipped, completed_at],
        )?;
        Ok(changed > 0)
    }

    pub fn insert_case_result(
        &self,
        run_id: &str,
        case_id: &str,
        data_row_id: Option<&str>,
        status: &str,
        request_log_json: &str,
        response_log_json: &str,
        assertion_results_json: &str,
        screenshots_json: &str,
        error_message: Option<&str>,
        error_code: Option<&str>,
        duration_ms: u64,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO test_run_results (id, run_id, case_id, data_row_id, status, duration_ms, request_log_json, response_log_json, assertion_results_json, screenshots_json, error_message, error_code, started_at, completed_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?13)",
            params![
                format!("run-result-{}", Uuid::new_v4()),
                run_id,
                case_id,
                data_row_id,
                status,
                duration_ms as i64,
                request_log_json,
                response_log_json,
                assertion_results_json,
                screenshots_json,
                error_message,
                error_code,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn insert_case_result_if_absent(
        &self,
        run_id: &str,
        case_id: &str,
        data_row_id: Option<&str>,
        status: &str,
        request_log_json: &str,
        response_log_json: &str,
        assertion_results_json: &str,
        screenshots_json: &str,
        error_message: Option<&str>,
        error_code: Option<&str>,
        duration_ms: u64,
    ) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "INSERT INTO test_run_results (id, run_id, case_id, data_row_id, status, duration_ms, request_log_json, response_log_json, assertion_results_json, screenshots_json, error_message, error_code, started_at, completed_at, created_at)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?13
             WHERE NOT EXISTS (
               SELECT 1 FROM test_run_results
               WHERE run_id = ?2 AND case_id = ?3 AND (
                 (data_row_id IS NULL AND ?4 IS NULL) OR data_row_id = ?4
               )
             )",
            params![
                format!("run-result-{}", Uuid::new_v4()),
                run_id,
                case_id,
                data_row_id,
                status,
                duration_ms as i64,
                request_log_json,
                response_log_json,
                assertion_results_json,
                screenshots_json,
                error_message,
                error_code,
                now,
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn count_case_results(&self, run_id: &str) -> Result<RunStatusCounts> {
        let counts = self.conn.query_row(
            "SELECT
                 SUM(CASE WHEN status = 'passed' THEN 1 ELSE 0 END) AS passed_count,
                 SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed_count,
                 SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END) AS skipped_count,
                 SUM(CASE WHEN status = 'cancelled' THEN 1 ELSE 0 END) AS cancelled_count,
                 COUNT(*) AS completed_count
             FROM test_run_results
             WHERE run_id = ?1",
            params![run_id],
            |row| {
                Ok(RunStatusCounts {
                    passed_count: row.get::<_, Option<i64>>(0)?.unwrap_or(0).max(0) as u32,
                    failed_count: row.get::<_, Option<i64>>(1)?.unwrap_or(0).max(0) as u32,
                    skipped_count: row.get::<_, Option<i64>>(2)?.unwrap_or(0).max(0) as u32,
                    cancelled_count: row.get::<_, Option<i64>>(3)?.unwrap_or(0).max(0) as u32,
                    completed_count: row.get::<_, i64>(4)?.max(0) as u32,
                })
            },
        )?;

        Ok(counts)
    }

    pub fn load_failed_targets(
        &self,
        run_id: &str,
        suite_id: &str,
    ) -> Result<Vec<FailedRunTarget>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT trr.case_id, trr.data_row_id
             FROM test_run_results trr
             JOIN test_runs tr ON tr.id = trr.run_id
             WHERE trr.run_id = ?1 AND tr.suite_id = ?2 AND trr.status = 'failed'",
        )?;

        let items = stmt
            .query_map(params![run_id, suite_id], |row| {
                Ok(FailedRunTarget {
                    case_id: row.get(0)?,
                    data_row_id: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(items)
    }

    pub fn load_run_result(&self, run_id: &str) -> Result<RunResultDto> {
        let summary = self.conn.query_row(
            "SELECT tr.id, tr.suite_id, ts.name, tr.environment_id, env.name, tr.status, tr.started_at, tr.completed_at, tr.total_cases, tr.passed, tr.failed, tr.skipped
             FROM test_runs tr
             LEFT JOIN test_suites ts ON ts.id = tr.suite_id
             JOIN environments env ON env.id = tr.environment_id
             WHERE tr.id = ?1",
            params![run_id],
            |row| {
                let status_raw: String = row.get(5)?;
                let status = match status_raw.as_str() {
                    "queued" => RunStatus::Queued,
                    "running" => RunStatus::Running,
                    "skipped" => RunStatus::Skipped,
                    "passed" => RunStatus::Passed,
                    "cancelled" => RunStatus::Cancelled,
                    _ => RunStatus::Failed,
                };

                Ok(PersistedRunSummary {
                    run_id: row.get(0)?,
                    suite_id: row.get(1)?,
                    suite_name: row.get(2)?,
                    environment_id: row.get(3)?,
                    environment_name: row.get(4)?,
                    status,
                    started_at: row.get::<_, Option<String>>(6)?.unwrap_or_else(|| Utc::now().to_rfc3339()),
                    finished_at: row.get(7)?,
                    total_count: row.get::<_, i64>(8)? as u32,
                    passed_count: row.get::<_, i64>(9)? as u32,
                    failed_count: row.get::<_, i64>(10)? as u32,
                    skipped_count: row.get::<_, i64>(11)? as u32,
                })
            },
        )?;

        Ok(Self::to_run_result_dto(summary))
    }

    pub fn load_run_detail(&self, run_id: &str) -> Result<RunDetailDto> {
        let summary = self.conn.query_row(
            "SELECT tr.id, tr.suite_id, ts.name, tr.environment_id, env.name, tr.status, tr.started_at, tr.completed_at, tr.total_cases, tr.passed, tr.failed, tr.skipped
             FROM test_runs tr
             LEFT JOIN test_suites ts ON ts.id = tr.suite_id
             JOIN environments env ON env.id = tr.environment_id
             WHERE tr.id = ?1",
            params![run_id],
            |row| {
                let status_raw: String = row.get(5)?;
                let status = match status_raw.as_str() {
                    "queued" => RunStatus::Queued,
                    "running" => RunStatus::Running,
                    "skipped" => RunStatus::Skipped,
                    "passed" => RunStatus::Passed,
                    "cancelled" => RunStatus::Cancelled,
                    _ => RunStatus::Failed,
                };

                Ok(PersistedRunSummary {
                    run_id: row.get(0)?,
                    suite_id: row.get(1)?,
                    suite_name: row.get(2)?,
                    environment_id: row.get(3)?,
                    environment_name: row.get(4)?,
                    status,
                    started_at: row.get::<_, Option<String>>(6)?.unwrap_or_else(|| Utc::now().to_rfc3339()),
                    finished_at: row.get(7)?,
                    total_count: row.get::<_, i64>(8)? as u32,
                    passed_count: row.get::<_, i64>(9)? as u32,
                    failed_count: row.get::<_, i64>(10)? as u32,
                    skipped_count: row.get::<_, i64>(11)? as u32,
                })
            },
        )?;

        let result_artifacts = self.load_artifacts_for_run(&summary.run_id)?;

        let mut stmt = self.conn.prepare(
            "SELECT trr.id, trr.case_id, tc.name, tc.case_type, trr.data_row_id, dtr.row_index, trr.status, trr.duration_ms,
                    trr.error_message, trr.error_code, trr.request_log_json, trr.response_log_json, trr.assertion_results_json, trr.screenshots_json
             FROM test_run_results trr
             JOIN test_cases tc ON tc.id = trr.case_id
             LEFT JOIN data_table_rows dtr ON dtr.id = trr.data_row_id
             WHERE trr.run_id = ?1
             ORDER BY trr.created_at ASC, tc.name COLLATE NOCASE ASC",
        )?;

        let rows = stmt
            .query_map(params![run_id], |row| {
                let status_raw: String = row.get(6)?;
                let status = match status_raw.as_str() {
                    "queued" => RunStatus::Queued,
                    "running" => RunStatus::Running,
                    "skipped" => RunStatus::Skipped,
                    "passed" => RunStatus::Passed,
                    "cancelled" => RunStatus::Cancelled,
                    _ => RunStatus::Failed,
                };

                let case_type_raw: String = row.get(3)?;
                let case_type = match case_type_raw.as_str() {
                    "ui" => TestCaseType::Ui,
                    _ => TestCaseType::Api,
                };

                let screenshots_json = row
                    .get::<_, Option<String>>(13)?
                    .unwrap_or_else(|| "[]".to_string());
                let artifact_paths = parse_string_array(&screenshots_json);
                let artifacts = result_artifacts
                    .iter()
                    .filter(|artifact| {
                        artifact_paths.iter().any(|path| {
                            artifact.file_path == *path || artifact.relative_path == *path
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                let request_log = row
                    .get::<_, Option<String>>(10)?
                    .unwrap_or_else(|| "{}".to_string());
                let response_log = row
                    .get::<_, Option<String>>(11)?
                    .unwrap_or_else(|| "{}".to_string());
                let assertions_log = row
                    .get::<_, Option<String>>(12)?
                    .unwrap_or_else(|| "[]".to_string());
                let error_message = row.get::<_, Option<String>>(8)?;
                let error_code = row.get::<_, Option<String>>(9)?;

                Ok(PersistedRunCaseResult {
                    dto: RunCaseResultDto {
                        id: row.get(0)?,
                        case_id: row.get(1)?,
                        case_name: row.get(2)?,
                        test_case_type: case_type,
                        data_row_id: row.get(4)?,
                        data_row_label: row
                            .get::<_, Option<i64>>(5)?
                            .map(|value| format!("Row {}", value + 1)),
                        status,
                        duration_ms: row.get::<_, i64>(7)?.max(0) as u64,
                        error_message: error_message.clone(),
                        error_code: error_code.clone(),
                        failure_category: classify_failure_category(&error_code, &error_message),
                        request_preview: sanitize_preview_text(&request_log),
                        response_preview: sanitize_preview_text(&response_log),
                        assertion_preview: sanitize_preview_text(&assertions_log),
                        artifacts,
                    },
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(RunDetailDto {
            summary: Self::to_run_history_entry(summary.clone()),
            results: rows.into_iter().map(|item| item.dto).collect(),
            artifacts: result_artifacts,
        })
    }

    fn to_run_result_dto(summary: PersistedRunSummary) -> RunResultDto {
        RunResultDto {
            run_id: summary.run_id,
            status: summary.status,
            suite_id: summary.suite_id,
            environment_id: Some(summary.environment_id),
            started_at: summary.started_at,
            finished_at: summary.finished_at,
            total_count: summary.total_count,
            passed_count: summary.passed_count,
            failed_count: summary.failed_count,
            skipped_count: summary.skipped_count,
        }
    }

    fn to_run_history_entry(summary: PersistedRunSummary) -> RunHistoryEntryDto {
        RunHistoryEntryDto {
            summary: Self::to_run_result_dto(summary.clone()),
            suite_name: summary.suite_name,
            environment_name: summary.environment_name,
        }
    }

    fn load_artifacts_for_run(&self, run_id: &str) -> Result<Vec<ArtifactManifestDto>> {
        let mut stmt = self.conn.prepare(
            "SELECT am.id, am.artifact_type, am.logical_name, am.file_path, am.relative_path, am.preview_json, am.created_at
             FROM artifact_manifests am
             WHERE EXISTS (
                 SELECT 1
                 FROM test_run_results trr
                 WHERE trr.run_id = ?1
                   AND trr.screenshots_json LIKE '%' || am.file_path || '%'
             )
             ORDER BY am.created_at DESC",
        )?;

        let items = stmt
            .query_map(params![run_id], |row| {
                Ok(ArtifactManifestDto {
                    id: row.get(0)?,
                    artifact_type: row.get(1)?,
                    logical_name: row.get(2)?,
                    file_path: row.get(3)?,
                    relative_path: row.get(4)?,
                    preview_json: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(items)
    }

    fn build_group_summary(
        &self,
        items: &[PersistedRunSummary],
        filter: &RunHistoryFilterDto,
    ) -> Result<RunHistoryGroupSummaryDto> {
        let mut failure_category_counts: BTreeMap<String, u32> = BTreeMap::new();
        let status_filter = filter
            .status
            .map(status_to_db_value)
            .transpose()?
            .map(str::to_string);
        let mut stmt = self.conn.prepare(
            "SELECT trr.error_code, trr.error_message
             FROM test_run_results trr
             JOIN test_runs tr ON tr.id = trr.run_id
             WHERE (?1 IS NULL OR tr.suite_id = ?1)
               AND (?2 IS NULL OR tr.status = ?2)
               AND (?3 IS NULL OR julianday(COALESCE(tr.started_at, tr.created_at)) >= julianday(?3))
               AND (?4 IS NULL OR julianday(COALESCE(tr.started_at, tr.created_at)) <= julianday(?4))
               AND trr.status = 'failed'",
        )?;

        let failure_rows = stmt.query_map(
            params![
                filter.suite_id.clone(),
                status_filter,
                filter.started_after.clone(),
                filter.started_before.clone()
            ],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )?;

        for failure_row in failure_rows {
            let (error_code, error_message) = failure_row?;
            let category = classify_failure_category(&error_code, &error_message);
            *failure_category_counts.entry(category).or_insert(0) += 1;
        }

        Ok(RunHistoryGroupSummaryDto {
            total_runs: items.len() as u32,
            passed_runs: items
                .iter()
                .filter(|item| item.status == RunStatus::Passed)
                .count() as u32,
            failed_runs: items
                .iter()
                .filter(|item| item.status == RunStatus::Failed)
                .count() as u32,
            cancelled_runs: items
                .iter()
                .filter(|item| item.status == RunStatus::Cancelled)
                .count() as u32,
            failure_category_counts: failure_category_counts
                .into_iter()
                .map(|(category, count)| FailureCategoryCountDto { category, count })
                .collect(),
        })
    }
}

fn validate_run_history_filter(filter: &RunHistoryFilterDto) -> Result<()> {
    if let (Some(started_after), Some(started_before)) = (
        filter.started_after.as_deref(),
        filter.started_before.as_deref(),
    ) {
        let started_after = parse_iso_filter_timestamp(started_after)?;
        let started_before = parse_iso_filter_timestamp(started_before)?;
        if started_after > started_before {
            return Err(TestForgeError::Validation(
                "startedAfter must be earlier than or equal to startedBefore".to_string(),
            ));
        }
    }

    if let Some(status) = filter.status {
        let _ = status_to_db_value(status)?;
    }

    Ok(())
}

fn parse_iso_filter_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| {
            TestForgeError::Validation(format!(
                "Run history filter timestamp must be RFC3339: {error}"
            ))
        })
}

fn status_to_db_value(status: RunStatus) -> Result<&'static str> {
    match status {
        RunStatus::Queued => Ok("queued"),
        RunStatus::Running => Ok("running"),
        RunStatus::Skipped => Ok("skipped"),
        RunStatus::Passed => Ok("passed"),
        RunStatus::Failed => Ok("failed"),
        RunStatus::Cancelled => Ok("cancelled"),
        RunStatus::Idle => Err(TestForgeError::Validation(
            "Unsupported run history status filter".to_string(),
        )),
    }
}

fn parse_string_array(json_text: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(json_text).unwrap_or_default()
}

fn classify_failure_category(
    error_code: &Option<String>,
    error_message: &Option<String>,
) -> String {
    let code = error_code
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let message = error_message
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    if code.contains("preflight")
        || message.contains("variable")
        || message.contains("build failed")
    {
        return "preflight".to_string();
    }

    if code.contains("assert") || message.contains("assert") {
        return "assertion".to_string();
    }

    if code.contains("cancel") || message.contains("cancel") {
        return "cancelled".to_string();
    }

    if code.contains("timeout") || message.contains("timeout") {
        return "timeout".to_string();
    }

    if code.contains("network")
        || code.contains("transport")
        || message.contains("network")
        || message.contains("http")
    {
        return "transport".to_string();
    }

    "execution".to_string()
}

fn sanitize_preview_text(raw_json: &str) -> String {
    let parsed = serde_json::from_str::<Value>(raw_json)
        .unwrap_or_else(|_| Value::String(raw_json.to_string()));
    let sanitized = sanitize_json_value(parsed);
    serde_json::to_string_pretty(&sanitized).unwrap_or_else(|_| "{}".to_string())
}

fn sanitize_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, item)| {
                    if should_redact_key(&key) {
                        (key, Value::String("[REDACTED]".to_string()))
                    } else {
                        (key, sanitize_json_value(item))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(sanitize_json_value).collect()),
        Value::String(text) if should_redact_value(&text) => {
            Value::String("[REDACTED]".to_string())
        }
        other => other,
    }
}

fn should_redact_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    [
        "authorization",
        "bearer",
        "basic",
        "token",
        "password",
        "secret",
        "api_key",
        "apikey",
        "ciphertext",
        "masked_preview",
        "cookie",
        "set-cookie",
    ]
    .iter()
    .any(|candidate| normalized.contains(candidate))
}

fn should_redact_value(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("bearer ")
        || normalized.contains("basic ")
        || normalized.contains("token")
        || normalized.contains("ciphertext")
        || normalized.contains("api_key")
        || normalized.contains("token=")
        || normalized.contains("authorization:")
        || normalized.contains("encrypted:")
        || normalized.contains("masked preview")
        || normalized.contains("masked_preview")
        || normalized.contains("password=")
        || normalized.contains("secret=")
}
