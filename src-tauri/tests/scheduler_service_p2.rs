//! Integration coverage for P2-T7 scheduler persistence semantics.

use std::path::PathBuf;

use chrono::{Duration, Utc};
use rusqlite::params;
use tempfile::TempDir;
use testforge::{
    contracts::domain::RunStatus,
    db::Database,
    error::AppError,
    services::scheduler_service::{ScheduleUpsertInput, SchedulerService},
    state::{AppConfig, AppState, RunState, ShellBootstrapSnapshot},
    utils::paths::AppPaths,
    SecretService,
};

fn manifest_migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

fn setup_database() -> (TempDir, Database) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("db").join("testforge.db");
    let database = Database::new_with_migrations_dir(db_path, manifest_migrations_dir()).unwrap();

    (temp_dir, database)
}

fn setup_app_state() -> (TempDir, AppState) {
    let temp_dir = TempDir::new().unwrap();
    let app_data = temp_dir.path().join("app-data");
    let paths = AppPaths::new(app_data.clone());
    paths.bootstrap().unwrap();
    let database =
        Database::new_with_migrations_dir(paths.database_file(), manifest_migrations_dir())
            .unwrap();
    let secret_service = SecretService::new(paths.base.clone());

    let state = AppState::new(
        database,
        secret_service,
        paths,
        ShellBootstrapSnapshot {
            app_version: "0.1.0".to_string(),
            is_first_run: false,
            degraded_mode: false,
            master_key_initialized: true,
        },
    );
    state.update_config(|config: &mut AppConfig| {
        config.default_timeout_ms = 1_000;
    });

    (temp_dir, state)
}

fn seed_suite_and_environment(database: &Database, suite_id: &str, environment_id: &str) {
    database
        .connection()
        .execute(
            "INSERT INTO test_suites (id, name, created_at, updated_at) VALUES (?1, ?2, datetime('now'), datetime('now'))",
            params![suite_id, format!("Suite {suite_id}")],
        )
        .unwrap();

    database
        .connection()
        .execute(
            "INSERT INTO environments (id, name, env_type, is_default, created_at, updated_at) VALUES (?1, ?2, 'development', 0, datetime('now'), datetime('now'))",
            params![environment_id, format!("Env {environment_id}")],
        )
        .unwrap();
}

#[test]
fn migration_creates_schedule_table() {
    let (_temp_dir, database) = setup_database();

    let table_count: i64 = database
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='suite_schedules'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(table_count, 1);
}

#[test]
fn migration_rerun_is_idempotent_for_schedule_table() {
    let (_temp_dir, database) = setup_database();

    let rerun_results = database.run_migrations().unwrap();

    assert!(rerun_results.iter().any(|result| {
        matches!(
            result,
            testforge::db::MigrationResult::Skipped { name } if name == "004_add_suite_schedules.sql"
        )
    }));
}

#[test]
fn schedules_persist_and_reload() {
    let (_temp_dir, database) = setup_database();
    seed_suite_and_environment(&database, "suite-persist", "env-persist");

    let service = SchedulerService::new(database.connection());
    let next_run_at = Utc::now() + Duration::minutes(15);

    let saved = service
        .upsert_schedule(ScheduleUpsertInput {
            id: Some("schedule-persist".to_string()),
            suite_id: "suite-persist".to_string(),
            environment_id: "env-persist".to_string(),
            enabled: true,
            cadence_minutes: 15,
            next_run_at: Some(next_run_at),
        })
        .unwrap();

    let schedules = service.list_schedules().unwrap();

    assert_eq!(schedules.len(), 1);
    assert_eq!(schedules[0].id, "schedule-persist");
    assert_eq!(schedules[0].suite_id, "suite-persist");
    assert_eq!(schedules[0].environment_id, "env-persist");
    assert!(schedules[0].enabled);
    assert_eq!(schedules[0].cadence_minutes, 15);
    assert_eq!(schedules[0].next_run_at, Some(next_run_at.to_rfc3339()));
    assert_eq!(saved.id, schedules[0].id);
}

#[test]
fn upsert_without_schedule_id_reuses_existing_logical_schedule_target() {
    let (_temp_dir, database) = setup_database();
    seed_suite_and_environment(&database, "suite-unique", "env-unique");

    let service = SchedulerService::new(database.connection());
    let first_next_run_at = Utc::now() + Duration::minutes(5);
    let second_next_run_at = Utc::now() + Duration::minutes(30);

    let first = service
        .upsert_schedule(ScheduleUpsertInput {
            id: None,
            suite_id: "suite-unique".to_string(),
            environment_id: "env-unique".to_string(),
            enabled: true,
            cadence_minutes: 5,
            next_run_at: Some(first_next_run_at),
        })
        .unwrap();

    let second = service
        .upsert_schedule(ScheduleUpsertInput {
            id: None,
            suite_id: "suite-unique".to_string(),
            environment_id: "env-unique".to_string(),
            enabled: false,
            cadence_minutes: 30,
            next_run_at: Some(second_next_run_at),
        })
        .unwrap();

    let schedules = service.list_schedules().unwrap();

    assert_eq!(schedules.len(), 1);
    assert_eq!(first.id, second.id);
    assert_eq!(schedules[0].id, first.id);
    assert!(!schedules[0].enabled);
    assert_eq!(schedules[0].cadence_minutes, 30);
    assert_eq!(
        schedules[0].next_run_at,
        Some(second_next_run_at.to_rfc3339())
    );
}

#[test]
fn enabled_due_schedules_are_selected_and_disabled_are_ignored() {
    let (_temp_dir, database) = setup_database();
    seed_suite_and_environment(&database, "suite-due", "env-due");
    seed_suite_and_environment(&database, "suite-future", "env-future");
    seed_suite_and_environment(&database, "suite-disabled", "env-disabled");

    let service = SchedulerService::new(database.connection());
    let now = Utc::now();

    service
        .upsert_schedule(ScheduleUpsertInput {
            id: Some("schedule-due".to_string()),
            suite_id: "suite-due".to_string(),
            environment_id: "env-due".to_string(),
            enabled: true,
            cadence_minutes: 5,
            next_run_at: Some(now - Duration::minutes(1)),
        })
        .unwrap();

    service
        .upsert_schedule(ScheduleUpsertInput {
            id: Some("schedule-future".to_string()),
            suite_id: "suite-future".to_string(),
            environment_id: "env-future".to_string(),
            enabled: true,
            cadence_minutes: 5,
            next_run_at: Some(now + Duration::minutes(10)),
        })
        .unwrap();

    service
        .upsert_schedule(ScheduleUpsertInput {
            id: Some("schedule-disabled".to_string()),
            suite_id: "suite-disabled".to_string(),
            environment_id: "env-disabled".to_string(),
            enabled: false,
            cadence_minutes: 5,
            next_run_at: Some(now - Duration::minutes(2)),
        })
        .unwrap();

    let due = service.list_due_schedules(now).unwrap();

    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, "schedule-due");
}

#[test]
fn invalid_or_broken_references_update_diagnostics_clearly() {
    let (_temp_dir, database) = setup_database();
    seed_suite_and_environment(&database, "suite-invalid", "env-invalid");

    let service = SchedulerService::new(database.connection());
    let now = Utc::now();

    service
        .upsert_schedule(ScheduleUpsertInput {
            id: Some("schedule-broken".to_string()),
            suite_id: "suite-invalid".to_string(),
            environment_id: "env-invalid".to_string(),
            enabled: true,
            cadence_minutes: 5,
            next_run_at: Some(now - Duration::minutes(1)),
        })
        .unwrap();

    database
        .connection()
        .execute(
            "DELETE FROM test_suites WHERE id = ?1",
            params!["suite-invalid"],
        )
        .unwrap();

    database
        .connection()
        .execute(
            "INSERT INTO suite_schedules (id, suite_id, environment_id, enabled, cadence_minutes, next_run_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, 0, ?4, ?5, ?5)",
            params![
                "schedule-invalid",
                "suite-missing",
                "env-invalid",
                now.to_rfc3339(),
                now.to_rfc3339()
            ],
        )
        .unwrap();

    let schedules = service.refresh_invalid_diagnostics().unwrap();

    let broken = schedules
        .iter()
        .find(|schedule| schedule.id == "schedule-broken")
        .unwrap();
    assert_eq!(broken.last_run_status, Some(RunStatus::Failed));
    assert!(broken
        .last_error
        .as_deref()
        .unwrap()
        .contains("suite reference is missing"));

    let invalid = schedules
        .iter()
        .find(|schedule| schedule.id == "schedule-invalid")
        .unwrap();
    assert_eq!(invalid.last_run_status, Some(RunStatus::Failed));
    assert!(invalid
        .last_error
        .as_deref()
        .unwrap()
        .contains("cadence_minutes must be greater than 0"));
}

#[test]
fn scheduler_runtime_start_is_idempotent() {
    let (_temp_dir, state) = setup_app_state();

    assert!(!state.is_scheduler_started());

    state.mark_scheduler_started().unwrap();
    let error = state.mark_scheduler_started().unwrap_err();

    assert!(state.is_scheduler_started());
    assert!(matches!(error, AppError { .. }));
    assert!(error
        .technical_message
        .contains("Scheduler loop already active"));
}

#[test]
fn schedule_tick_blocks_when_another_run_is_active_and_updates_diagnostics() {
    let (_temp_dir, state) = setup_app_state();
    {
        let db = state.db();
        let guard = db.lock().unwrap();
        seed_suite_and_environment(&guard, "suite-blocked", "env-blocked");
        SchedulerService::new(guard.connection())
            .upsert_schedule(ScheduleUpsertInput {
                id: Some("schedule-blocked".to_string()),
                suite_id: "suite-blocked".to_string(),
                environment_id: "env-blocked".to_string(),
                enabled: true,
                cadence_minutes: 5,
                next_run_at: Some(Utc::now() - Duration::minutes(1)),
            })
            .unwrap();
    }

    state
        .start_run("run-manual".to_string(), "suite-other".to_string())
        .unwrap();

    let db = state.db();
    let guard = db.lock().unwrap();
    let service = SchedulerService::new(guard.connection());
    let processed = service
        .schedule_tick(&state, Utc::now(), |_schedule| Ok(()))
        .unwrap();
    let schedule = service
        .list_schedules()
        .unwrap()
        .into_iter()
        .find(|item| item.id == "schedule-blocked")
        .unwrap();

    assert_eq!(processed, 0);
    assert!(matches!(state.run_state(), RunState::Running { .. }));
    assert_eq!(schedule.last_run_status, Some(RunStatus::Skipped));
    assert!(schedule
        .last_error
        .as_deref()
        .unwrap()
        .contains("Blocked: another suite run is already active"));
}

#[test]
fn schedule_tick_updates_timestamps_when_trigger_succeeds() {
    let (_temp_dir, state) = setup_app_state();
    {
        let db = state.db();
        let guard = db.lock().unwrap();
        seed_suite_and_environment(&guard, "suite-success", "env-success");
        SchedulerService::new(guard.connection())
            .upsert_schedule(ScheduleUpsertInput {
                id: Some("schedule-success".to_string()),
                suite_id: "suite-success".to_string(),
                environment_id: "env-success".to_string(),
                enabled: true,
                cadence_minutes: 15,
                next_run_at: Some(Utc::now() - Duration::minutes(1)),
            })
            .unwrap();
    }

    let db = state.db();
    let guard = db.lock().unwrap();
    let service = SchedulerService::new(guard.connection());
    let before = Utc::now();
    let processed = service
        .schedule_tick(&state, before, |_schedule| Ok(()))
        .unwrap();
    let schedule = service
        .list_schedules()
        .unwrap()
        .into_iter()
        .find(|item| item.id == "schedule-success")
        .unwrap();

    let next_run_at =
        chrono::DateTime::parse_from_rfc3339(schedule.next_run_at.as_deref().unwrap())
            .unwrap()
            .with_timezone(&Utc);
    let last_run_at =
        chrono::DateTime::parse_from_rfc3339(schedule.last_run_at.as_deref().unwrap())
            .unwrap()
            .with_timezone(&Utc);

    assert_eq!(processed, 1);
    assert_eq!(schedule.last_error, None);
    assert_eq!(schedule.last_run_status, Some(RunStatus::Queued));
    assert!(last_run_at >= before);
    assert!(next_run_at >= before + Duration::minutes(15));
}

#[test]
fn schedule_tick_records_trigger_failure_diagnostics_honestly() {
    let (_temp_dir, state) = setup_app_state();
    {
        let db = state.db();
        let guard = db.lock().unwrap();
        seed_suite_and_environment(&guard, "suite-fail", "env-fail");
        SchedulerService::new(guard.connection())
            .upsert_schedule(ScheduleUpsertInput {
                id: Some("schedule-fail".to_string()),
                suite_id: "suite-fail".to_string(),
                environment_id: "env-fail".to_string(),
                enabled: true,
                cadence_minutes: 5,
                next_run_at: Some(Utc::now() - Duration::minutes(1)),
            })
            .unwrap();
    }

    let db = state.db();
    let guard = db.lock().unwrap();
    let service = SchedulerService::new(guard.connection());
    let processed = service
        .schedule_tick(&state, Utc::now(), |_schedule| {
            Err(AppError::validation("runner trigger failed"))
        })
        .unwrap();
    let schedule = service
        .list_schedules()
        .unwrap()
        .into_iter()
        .find(|item| item.id == "schedule-fail")
        .unwrap();

    assert_eq!(processed, 0);
    assert_eq!(schedule.last_run_status, Some(RunStatus::Failed));
    assert!(schedule
        .last_error
        .as_deref()
        .unwrap()
        .contains("runner trigger failed"));
}

#[test]
fn refresh_invalid_diagnostics_preserves_blocked_operational_state_for_valid_schedule() {
    let (_temp_dir, state) = setup_app_state();
    {
        let db = state.db();
        let guard = db.lock().unwrap();
        seed_suite_and_environment(&guard, "suite-blocked-refresh", "env-blocked-refresh");
        SchedulerService::new(guard.connection())
            .upsert_schedule(ScheduleUpsertInput {
                id: Some("schedule-blocked-refresh".to_string()),
                suite_id: "suite-blocked-refresh".to_string(),
                environment_id: "env-blocked-refresh".to_string(),
                enabled: true,
                cadence_minutes: 5,
                next_run_at: Some(Utc::now() - Duration::minutes(1)),
            })
            .unwrap();
    }

    state
        .start_run("run-existing".to_string(), "suite-other".to_string())
        .unwrap();

    let db = state.db();
    let guard = db.lock().unwrap();
    let service = SchedulerService::new(guard.connection());
    service
        .schedule_tick(&state, Utc::now(), |_schedule| Ok(()))
        .unwrap();

    let refreshed = service.refresh_invalid_diagnostics().unwrap();
    let schedule = refreshed
        .into_iter()
        .find(|item| item.id == "schedule-blocked-refresh")
        .unwrap();

    assert_eq!(schedule.last_run_status, Some(RunStatus::Skipped));
    assert!(schedule
        .last_error
        .as_deref()
        .unwrap()
        .contains("Blocked: another suite run is already active"));
}

#[test]
fn refresh_invalid_diagnostics_preserves_failed_operational_state_for_valid_schedule() {
    let (_temp_dir, state) = setup_app_state();
    {
        let db = state.db();
        let guard = db.lock().unwrap();
        seed_suite_and_environment(&guard, "suite-failed-refresh", "env-failed-refresh");
        SchedulerService::new(guard.connection())
            .upsert_schedule(ScheduleUpsertInput {
                id: Some("schedule-failed-refresh".to_string()),
                suite_id: "suite-failed-refresh".to_string(),
                environment_id: "env-failed-refresh".to_string(),
                enabled: true,
                cadence_minutes: 5,
                next_run_at: Some(Utc::now() - Duration::minutes(1)),
            })
            .unwrap();
    }

    let db = state.db();
    let guard = db.lock().unwrap();
    let service = SchedulerService::new(guard.connection());
    service
        .schedule_tick(&state, Utc::now(), |_schedule| {
            Err(AppError::validation("scheduler trigger transport failed"))
        })
        .unwrap();

    let refreshed = service.refresh_invalid_diagnostics().unwrap();
    let schedule = refreshed
        .into_iter()
        .find(|item| item.id == "schedule-failed-refresh")
        .unwrap();

    assert_eq!(schedule.last_run_status, Some(RunStatus::Failed));
    assert!(schedule
        .last_error
        .as_deref()
        .unwrap()
        .contains("scheduler trigger transport failed"));
}
