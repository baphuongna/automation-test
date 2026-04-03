use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::contracts::domain::RunStatus;
use crate::contracts::dto::SuiteScheduleDto;
use crate::error::{AppError, AppResult, Result, TestForgeError};
use crate::state::{AppState, RunState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteScheduleRecord {
    pub id: String,
    pub suite_id: String,
    pub environment_id: String,
    pub enabled: bool,
    pub cadence_minutes: i64,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_status: Option<RunStatus>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ScheduleUpsertInput {
    pub id: Option<String>,
    pub suite_id: String,
    pub environment_id: String,
    pub enabled: bool,
    pub cadence_minutes: i64,
    pub next_run_at: Option<DateTime<Utc>>,
}

pub struct SchedulerService<'a> {
    conn: &'a Connection,
}

pub const SCHEDULED_BLOCKED_MESSAGE: &str = "Blocked: another suite run is already active";
const INVALID_CONFIGURATION_PREFIX: &str = "Schedule invalid:";

impl<'a> SchedulerService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn upsert_schedule(&self, input: ScheduleUpsertInput) -> Result<SuiteScheduleRecord> {
        self.validate_upsert_input(&input)?;

        let schedule_id = match input.id {
            Some(id) => id,
            None => self
                .find_schedule_id_by_target(&input.suite_id, &input.environment_id)?
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
        };
        let now = Utc::now().to_rfc3339();
        let next_run_at = input.next_run_at.map(|value| value.to_rfc3339());

        self.conn.execute(
            "INSERT INTO suite_schedules (
                id,
                suite_id,
                environment_id,
                enabled,
                cadence_minutes,
                next_run_at,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(id) DO UPDATE SET
                suite_id = excluded.suite_id,
                environment_id = excluded.environment_id,
                enabled = excluded.enabled,
                cadence_minutes = excluded.cadence_minutes,
                next_run_at = excluded.next_run_at,
                updated_at = excluded.updated_at",
            params![
                schedule_id,
                input.suite_id,
                input.environment_id,
                input.enabled,
                input.cadence_minutes,
                next_run_at,
                now,
            ],
        )?;

        self.get_schedule(&schedule_id)
    }

    pub fn list_schedules(&self) -> Result<Vec<SuiteScheduleRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, suite_id, environment_id, enabled, cadence_minutes, last_run_at, next_run_at, last_run_status, last_error, created_at, updated_at
             FROM suite_schedules
             ORDER BY created_at ASC, id ASC",
        )?;

        let schedules = stmt
            .query_map([], Self::map_schedule_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(schedules)
    }

    pub fn list_schedule_dtos(&self) -> Result<Vec<SuiteScheduleDto>> {
        self.list_schedules()
            .map(|schedules| schedules.into_iter().map(|item| item.into()).collect())
    }

    pub fn set_schedule_enabled(
        &self,
        schedule_id: &str,
        enabled: bool,
    ) -> Result<SuiteScheduleRecord> {
        let updated = self.conn.execute(
            "UPDATE suite_schedules
             SET enabled = ?2,
                 updated_at = ?3
             WHERE id = ?1",
            params![schedule_id, enabled, Utc::now().to_rfc3339()],
        )?;

        if updated == 0 {
            return Err(TestForgeError::Validation(format!(
                "Schedule not found: {schedule_id}"
            )));
        }

        self.get_schedule(schedule_id)
    }

    pub fn delete_schedule(&self, schedule_id: &str) -> Result<bool> {
        let deleted = self.conn.execute(
            "DELETE FROM suite_schedules WHERE id = ?1",
            params![schedule_id],
        )?;

        Ok(deleted > 0)
    }

    pub fn list_due_schedules(&self, now: DateTime<Utc>) -> Result<Vec<SuiteScheduleRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, suite_id, environment_id, enabled, cadence_minutes, last_run_at, next_run_at, last_run_status, last_error, created_at, updated_at
             FROM suite_schedules
             WHERE enabled = 1
               AND next_run_at IS NOT NULL
               AND cadence_minutes > 0
               AND julianday(next_run_at) <= julianday(?1)
             ORDER BY next_run_at ASC, created_at ASC",
        )?;

        let schedules = stmt
            .query_map(params![now.to_rfc3339()], Self::map_schedule_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(schedules)
    }

    pub fn refresh_invalid_diagnostics(&self) -> Result<Vec<SuiteScheduleRecord>> {
        let schedules = self.list_schedules()?;

        for schedule in &schedules {
            let invalid_message = self.validate_schedule_definition(schedule)?;

            match invalid_message {
                Some(message) => {
                    self.conn.execute(
                        "UPDATE suite_schedules
                         SET last_run_status = ?2,
                             last_error = ?3,
                             updated_at = ?4
                         WHERE id = ?1",
                        params![
                            schedule.id,
                            run_status_to_db_value(RunStatus::Failed),
                            message,
                            Utc::now().to_rfc3339()
                        ],
                    )?;
                }
                None if should_clear_invalid_diagnostic(schedule) => {
                    self.conn.execute(
                        "UPDATE suite_schedules
                         SET last_run_status = NULL,
                             last_error = NULL,
                             updated_at = ?2
                         WHERE id = ?1",
                        params![schedule.id, Utc::now().to_rfc3339()],
                    )?;
                }
                None => {}
            }
        }

        self.list_schedules()
    }

    pub fn schedule_tick<F>(
        &self,
        state: &AppState,
        now: DateTime<Utc>,
        trigger: F,
    ) -> AppResult<usize>
    where
        F: Fn(&SuiteScheduleRecord) -> AppResult<()>,
    {
        let due = self.list_due_schedules(now).map_err(map_scheduler_error)?;
        self.trigger_due_schedules(state, now, due, trigger)
    }

    pub fn trigger_due_schedules<F>(
        &self,
        state: &AppState,
        now: DateTime<Utc>,
        due: Vec<SuiteScheduleRecord>,
        trigger: F,
    ) -> AppResult<usize>
    where
        F: Fn(&SuiteScheduleRecord) -> AppResult<()>,
    {
        let refreshed = self
            .refresh_invalid_diagnostics()
            .map_err(map_scheduler_error)?;
        let refreshed_by_id = refreshed
            .into_iter()
            .map(|schedule| (schedule.id.clone(), schedule))
            .collect::<std::collections::HashMap<_, _>>();

        let mut processed = 0usize;
        for due_schedule in due {
            let Some(schedule) = refreshed_by_id.get(&due_schedule.id) else {
                continue;
            };

            if schedule.last_error.is_some() {
                continue;
            }

            if matches!(state.run_state(), RunState::Running { .. }) {
                self.update_schedule_blocked(&schedule.id, now, SCHEDULED_BLOCKED_MESSAGE)
                    .map_err(map_scheduler_error)?;
                continue;
            }

            match trigger(schedule) {
                Ok(()) => {
                    self.update_schedule_after_trigger_success(schedule, now)
                        .map_err(map_scheduler_error)?;
                    processed += 1;
                }
                Err(error) => {
                    self.update_schedule_after_trigger_failure(&schedule.id, &error)
                        .map_err(map_scheduler_error)?;
                }
            }
        }

        Ok(processed)
    }

    fn get_schedule(&self, schedule_id: &str) -> Result<SuiteScheduleRecord> {
        self.conn
            .query_row(
                "SELECT id, suite_id, environment_id, enabled, cadence_minutes, last_run_at, next_run_at, last_run_status, last_error, created_at, updated_at
                 FROM suite_schedules
                 WHERE id = ?1",
                params![schedule_id],
                Self::map_schedule_row,
            )
            .map_err(Into::into)
    }

    fn find_schedule_id_by_target(
        &self,
        suite_id: &str,
        environment_id: &str,
    ) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT id FROM suite_schedules WHERE suite_id = ?1 AND environment_id = ?2",
                params![suite_id, environment_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    fn validate_upsert_input(&self, input: &ScheduleUpsertInput) -> Result<()> {
        if input.suite_id.trim().is_empty() {
            return Err(TestForgeError::Validation(
                "suite_id is required for scheduler persistence".to_string(),
            ));
        }

        if input.environment_id.trim().is_empty() {
            return Err(TestForgeError::Validation(
                "environment_id is required for scheduler persistence".to_string(),
            ));
        }

        if input.cadence_minutes <= 0 {
            return Err(TestForgeError::Validation(
                "cadence_minutes must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_schedule_definition(
        &self,
        schedule: &SuiteScheduleRecord,
    ) -> Result<Option<String>> {
        if schedule.cadence_minutes <= 0 {
            return Ok(Some(format!(
                "{INVALID_CONFIGURATION_PREFIX} cadence_minutes must be greater than 0"
            )));
        }

        if !self.reference_exists("test_suites", &schedule.suite_id)? {
            return Ok(Some(format!(
                "{INVALID_CONFIGURATION_PREFIX} suite reference is missing ({})",
                schedule.suite_id
            )));
        }

        if !self.reference_exists("environments", &schedule.environment_id)? {
            return Ok(Some(format!(
                "{INVALID_CONFIGURATION_PREFIX} environment reference is missing ({})",
                schedule.environment_id
            )));
        }

        Ok(None)
    }

    fn update_schedule_after_trigger_success(
        &self,
        schedule: &SuiteScheduleRecord,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let next_run_at = now + chrono::Duration::minutes(schedule.cadence_minutes);
        self.conn.execute(
            "UPDATE suite_schedules
             SET last_run_at = ?2,
                 next_run_at = ?3,
                 last_run_status = ?4,
                 last_error = NULL,
                 updated_at = ?2
             WHERE id = ?1",
            params![
                schedule.id,
                now.to_rfc3339(),
                next_run_at.to_rfc3339(),
                run_status_to_db_value(RunStatus::Queued)
            ],
        )?;

        Ok(())
    }

    fn update_schedule_after_trigger_failure(
        &self,
        schedule_id: &str,
        error: &AppError,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE suite_schedules
             SET last_run_status = ?2,
                 last_error = ?3,
                 updated_at = ?4
             WHERE id = ?1",
            params![
                schedule_id,
                run_status_to_db_value(RunStatus::Failed),
                error.technical_message.clone(),
                Utc::now().to_rfc3339()
            ],
        )?;

        Ok(())
    }

    fn update_schedule_blocked(
        &self,
        schedule_id: &str,
        now: DateTime<Utc>,
        message: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE suite_schedules
             SET last_run_status = ?2,
                 last_error = ?3,
                 updated_at = ?4
             WHERE id = ?1",
            params![
                schedule_id,
                run_status_to_db_value(RunStatus::Skipped),
                message,
                now.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    fn reference_exists(&self, table_name: &str, entity_id: &str) -> Result<bool> {
        let sql = format!("SELECT EXISTS(SELECT 1 FROM {table_name} WHERE id = ?1)");
        let exists = self
            .conn
            .query_row(&sql, params![entity_id], |row| row.get(0))?;

        Ok(exists)
    }

    fn map_schedule_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SuiteScheduleRecord> {
        let raw_status = row.get::<_, Option<String>>(7)?;
        Ok(SuiteScheduleRecord {
            id: row.get(0)?,
            suite_id: row.get(1)?,
            environment_id: row.get(2)?,
            enabled: row.get(3)?,
            cadence_minutes: row.get(4)?,
            last_run_at: row.get(5)?,
            next_run_at: row.get(6)?,
            last_run_status: raw_status.as_deref().and_then(run_status_from_db_value),
            last_error: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }
}

impl From<SuiteScheduleRecord> for SuiteScheduleDto {
    fn from(value: SuiteScheduleRecord) -> Self {
        Self {
            id: value.id,
            suite_id: value.suite_id,
            environment_id: value.environment_id,
            enabled: value.enabled,
            cadence_minutes: value.cadence_minutes as u32,
            last_run_at: value.last_run_at,
            next_run_at: value.next_run_at,
            last_run_status: value.last_run_status,
            last_error: value.last_error,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn run_status_to_db_value(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Skipped => "skipped",
        RunStatus::Passed => "passed",
        RunStatus::Cancelled => "cancelled",
        RunStatus::Failed | RunStatus::Idle => "failed",
    }
}

fn run_status_from_db_value(value: &str) -> Option<RunStatus> {
    match value {
        "queued" => Some(RunStatus::Queued),
        "running" => Some(RunStatus::Running),
        "skipped" => Some(RunStatus::Skipped),
        "passed" => Some(RunStatus::Passed),
        "failed" => Some(RunStatus::Failed),
        "cancelled" => Some(RunStatus::Cancelled),
        _ => None,
    }
}

fn should_clear_invalid_diagnostic(schedule: &SuiteScheduleRecord) -> bool {
    matches!(schedule.last_run_status, Some(RunStatus::Failed))
        && schedule
            .last_error
            .as_deref()
            .map(|message| message.starts_with(INVALID_CONFIGURATION_PREFIX))
            .unwrap_or(false)
}

fn map_scheduler_error(error: TestForgeError) -> AppError {
    match error {
        TestForgeError::Validation(message) => AppError::validation(message),
        TestForgeError::Database(source) => AppError::from(source),
        TestForgeError::Io(source) => AppError::from(source),
        TestForgeError::Serialization(source) => AppError::from(source),
        other => AppError::internal(other.to_string()),
    }
}
