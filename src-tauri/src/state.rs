//! Application state management.

use std::sync::{Arc, RwLock};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::services::SecretService;
use crate::utils::paths::AppPaths;

/// Application configuration bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub default_timeout_ms: u32,
    pub max_concurrent_api: usize,
    pub screenshot_on_fail: bool,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub default_step_timeout_ms: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30_000,
            max_concurrent_api: 4,
            screenshot_on_fail: true,
            viewport_width: 1280,
            viewport_height: 720,
            default_step_timeout_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording {
        script_id: String,
        start_time: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunState {
    Idle,
    Running {
        run_id: String,
        suite_id: String,
        start_time: chrono::DateTime<chrono::Utc>,
    },
}

/// Shared app state created during backend bootstrap.
pub struct AppState {
    db: Arc<RwLock<Database>>,
    secret_service: Arc<RwLock<SecretService>>,
    paths: AppPaths,
    config: RwLock<AppConfig>,
    active_environment_id: RwLock<Option<String>>,
    recording_state: RwLock<RecordingState>,
    run_state: RwLock<RunState>,
    degraded_mode: RwLock<bool>,
    master_key_initialized: RwLock<bool>,
}

impl AppState {
    pub fn new(db: Database, secret_service: SecretService, paths: AppPaths) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            secret_service: Arc::new(RwLock::new(secret_service)),
            paths,
            config: RwLock::new(AppConfig::default()),
            active_environment_id: RwLock::new(None),
            recording_state: RwLock::new(RecordingState::Idle),
            run_state: RwLock::new(RunState::Idle),
            degraded_mode: RwLock::new(false),
            master_key_initialized: RwLock::new(false),
        }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn db(&self) -> Arc<RwLock<Database>> {
        Arc::clone(&self.db)
    }

    pub fn secret_service(&self) -> Arc<RwLock<SecretService>> {
        Arc::clone(&self.secret_service)
    }

    pub fn is_degraded_mode(&self) -> bool {
        *self.degraded_mode.read().unwrap()
    }

    pub fn set_degraded_mode(&self, value: bool) {
        *self.degraded_mode.write().unwrap() = value;
    }

    pub fn is_master_key_initialized(&self) -> bool {
        *self.master_key_initialized.read().unwrap()
    }

    pub fn set_master_key_initialized(&self, value: bool) {
        *self.master_key_initialized.write().unwrap() = value;
    }

    pub fn active_environment(&self) -> Option<String> {
        self.active_environment_id.read().unwrap().clone()
    }

    pub fn set_active_environment(&self, id: String) {
        *self.active_environment_id.write().unwrap() = Some(id);
    }

    pub fn recording_state(&self) -> RecordingState {
        self.recording_state.read().unwrap().clone()
    }

    pub fn start_recording(&self, script_id: String) -> AppResult<()> {
        let mut state = self.recording_state.write().unwrap();
        match &*state {
            RecordingState::Idle => {
                *state = RecordingState::Recording {
                    script_id,
                    start_time: chrono::Utc::now(),
                };
                Ok(())
            }
            RecordingState::Recording { .. } => Err(AppError::recording_in_progress()),
        }
    }

    pub fn stop_recording(&self) {
        *self.recording_state.write().unwrap() = RecordingState::Idle;
    }

    pub fn run_state(&self) -> RunState {
        self.run_state.read().unwrap().clone()
    }

    pub fn start_run(&self, run_id: String, suite_id: String) -> AppResult<()> {
        let mut state = self.run_state.write().unwrap();
        match &*state {
            RunState::Idle => {
                *state = RunState::Running {
                    run_id,
                    suite_id,
                    start_time: chrono::Utc::now(),
                };
                Ok(())
            }
            RunState::Running { .. } => Err(AppError::run_in_progress()),
        }
    }

    pub fn stop_run(&self) {
        *self.run_state.write().unwrap() = RunState::Idle;
    }

    pub fn config(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }

    pub fn update_config<F>(&self, apply: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        apply(&mut self.config.write().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::TempDir;

    fn create_state() -> AppState {
        let temp_dir = TempDir::new().unwrap();
        let app_data = temp_dir.path().join("app-data");
        let paths = AppPaths::new(app_data.clone());
        paths.bootstrap().unwrap();

        let migrations_dir = temp_dir.path().join("migrations");
        fs::create_dir_all(&migrations_dir).unwrap();
        fs::write(
            migrations_dir.join("001_test.sql"),
            "CREATE TABLE IF NOT EXISTS state_probe (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        let database = Database::new_with_migrations_dir(paths.database_file(), migrations_dir).unwrap();
        let secret_service = SecretService::new(paths.base.clone());

        AppState::new(database, secret_service, paths)
    }

    #[test]
    fn app_config_default_matches_bootstrap_expectations() {
        let config = AppConfig::default();
        assert_eq!(config.default_timeout_ms, 30_000);
        assert_eq!(config.max_concurrent_api, 4);
        assert!(config.screenshot_on_fail);
    }

    #[test]
    fn recording_state_transitions_work() {
        let state = create_state();

        assert_eq!(state.recording_state(), RecordingState::Idle);
        state.start_recording("script-1".to_string()).unwrap();
        assert!(matches!(state.recording_state(), RecordingState::Recording { .. }));
        state.stop_recording();
        assert_eq!(state.recording_state(), RecordingState::Idle);
    }

    #[test]
    fn concurrent_recording_is_rejected() {
        let state = create_state();

        state.start_recording("script-1".to_string()).unwrap();
        let result = state.start_recording("script-2".to_string());
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(matches!(error.code, crate::error::ErrorCode::RecordingInProgress));
    }

    #[test]
    fn concurrent_run_is_rejected() {
        let state = create_state();

        state.start_run("run-1".to_string(), "suite-1".to_string()).unwrap();
        let result = state.start_run("run-2".to_string(), "suite-2".to_string());
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(matches!(error.code, crate::error::ErrorCode::RunInProgress));
    }
}
