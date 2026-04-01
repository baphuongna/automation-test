use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::contracts::domain::{RunStatus, TestCaseType};
use crate::contracts::dto::{RunResultDto, SuiteDto, SuiteItemDto};
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
    pub environment_id: String,
    pub status: RunStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub total_count: u32,
    pub passed_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
}

#[derive(Debug, Clone)]
pub struct FailedRunTarget {
    pub case_id: String,
    pub data_row_id: Option<String>,
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

    pub fn load_failed_targets(
        &self,
        run_id: &str,
        suite_id: &str,
    ) -> Result<Vec<FailedRunTarget>> {
        let mut stmt = self.conn.prepare(
            "SELECT trr.case_id, trr.data_row_id
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
            "SELECT id, suite_id, environment_id, status, started_at, completed_at, total_cases, passed, failed, skipped
             FROM test_runs WHERE id = ?1",
            params![run_id],
            |row| {
                let status_raw: String = row.get(3)?;
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
                    environment_id: row.get(2)?,
                    status,
                    started_at: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| Utc::now().to_rfc3339()),
                    finished_at: row.get(5)?,
                    total_count: row.get::<_, i64>(6)? as u32,
                    passed_count: row.get::<_, i64>(7)? as u32,
                    failed_count: row.get::<_, i64>(8)? as u32,
                    skipped_count: row.get::<_, i64>(9)? as u32,
                })
            },
        )?;

        Ok(RunResultDto {
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
        })
    }
}
