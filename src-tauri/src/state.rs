//! Application state management.

use std::sync::{Arc, RwLock};

use crate::contracts::dto::UiStepDto;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::services::SecretService;
use crate::utils::paths::AppPaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellBootstrapSnapshot {
    pub app_version: String,
    pub is_first_run: bool,
    pub degraded_mode: bool,
    pub master_key_initialized: bool,
}

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
        test_case_id: String,
        start_url: String,
        start_time: chrono::DateTime<chrono::Utc>,
        captured_steps: Vec<UiStepDto>,
    },
    Failed {
        test_case_id: String,
        start_url: String,
        start_time: chrono::DateTime<chrono::Utc>,
        captured_steps: Vec<UiStepDto>,
        last_error: String,
        recoverable: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordingSnapshot {
    pub test_case_id: String,
    pub start_url: String,
    pub captured_steps: Vec<UiStepDto>,
    pub last_error: Option<String>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunState {
    Idle,
    Running {
        run_id: String,
        suite_id: String,
        start_time: chrono::DateTime<chrono::Utc>,
        cancel_requested: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReplayState {
    Idle,
    Running {
        run_id: String,
        test_case_id: String,
        start_time: chrono::DateTime<chrono::Utc>,
        cancel_requested: bool,
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
    replay_state: RwLock<ReplayState>,
    run_state: RwLock<RunState>,
    degraded_mode: RwLock<bool>,
    master_key_initialized: RwLock<bool>,
    shell_bootstrap_snapshot: RwLock<ShellBootstrapSnapshot>,
}

impl AppState {
    pub fn new(
        db: Database,
        secret_service: SecretService,
        paths: AppPaths,
        shell_bootstrap_snapshot: ShellBootstrapSnapshot,
    ) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            secret_service: Arc::new(RwLock::new(secret_service)),
            paths,
            config: RwLock::new(AppConfig::default()),
            active_environment_id: RwLock::new(None),
            recording_state: RwLock::new(RecordingState::Idle),
            replay_state: RwLock::new(ReplayState::Idle),
            run_state: RwLock::new(RunState::Idle),
            degraded_mode: RwLock::new(false),
            master_key_initialized: RwLock::new(false),
            shell_bootstrap_snapshot: RwLock::new(shell_bootstrap_snapshot),
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
        self.shell_bootstrap_snapshot.write().unwrap().degraded_mode = value;
    }

    pub fn is_master_key_initialized(&self) -> bool {
        *self.master_key_initialized.read().unwrap()
    }

    pub fn set_master_key_initialized(&self, value: bool) {
        *self.master_key_initialized.write().unwrap() = value;
        self.shell_bootstrap_snapshot
            .write()
            .unwrap()
            .master_key_initialized = value;
    }

    pub fn shell_bootstrap_snapshot(&self) -> ShellBootstrapSnapshot {
        self.shell_bootstrap_snapshot.read().unwrap().clone()
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

    pub fn start_recording_session(
        &self,
        test_case_id: String,
        start_url: String,
    ) -> AppResult<()> {
        let mut state = self.recording_state.write().unwrap();
        match &*state {
            RecordingState::Idle => {
                *state = RecordingState::Recording {
                    test_case_id,
                    start_url,
                    start_time: chrono::Utc::now(),
                    captured_steps: Vec::new(),
                };
                Ok(())
            }
            RecordingState::Recording { .. } | RecordingState::Failed { .. } => {
                Err(AppError::recording_in_progress())
            }
        }
    }

    pub fn start_recording(&self, script_id: String) -> AppResult<()> {
        self.start_recording_session(script_id, String::new())
    }

    pub fn record_captured_step(&self, step: UiStepDto) -> AppResult<()> {
        let mut state = self.recording_state.write().unwrap();
        match &mut *state {
            RecordingState::Recording { captured_steps, .. }
            | RecordingState::Failed { captured_steps, .. } => {
                captured_steps.push(step);
                Ok(())
            }
            RecordingState::Idle => Err(AppError::validation(
                "Không thể nhận step khi chưa có phiên recording hoạt động.",
            )),
        }
    }

    pub fn mark_recording_failed(&self, error_message: String, recoverable: bool) -> AppResult<()> {
        let mut state = self.recording_state.write().unwrap();
        match &*state {
            RecordingState::Recording {
                test_case_id,
                start_url,
                start_time,
                captured_steps,
            } => {
                *state = RecordingState::Failed {
                    test_case_id: test_case_id.clone(),
                    start_url: start_url.clone(),
                    start_time: *start_time,
                    captured_steps: captured_steps.clone(),
                    last_error: error_message,
                    recoverable,
                };
                Ok(())
            }
            RecordingState::Failed { .. } => Ok(()),
            RecordingState::Idle => Err(AppError::validation(
                "Không thể đánh dấu failed khi không có phiên recording.",
            )),
        }
    }

    pub fn stop_recording(&self, expected_test_case_id: &str) -> AppResult<RecordingSnapshot> {
        let mut state = self.recording_state.write().unwrap();
        let snapshot = match &*state {
            RecordingState::Recording {
                test_case_id,
                start_url,
                captured_steps,
                ..
            } => {
                if test_case_id != expected_test_case_id {
                    return Err(AppError::validation(
                        "Yêu cầu stop recording không khớp testCaseId đang hoạt động.",
                    ));
                }
                RecordingSnapshot {
                    test_case_id: test_case_id.clone(),
                    start_url: start_url.clone(),
                    captured_steps: captured_steps.clone(),
                    last_error: None,
                    recoverable: true,
                }
            }
            RecordingState::Failed {
                test_case_id,
                start_url,
                captured_steps,
                last_error,
                recoverable,
                ..
            } => {
                if test_case_id != expected_test_case_id {
                    return Err(AppError::validation(
                        "Yêu cầu stop recording không khớp testCaseId đang failed.",
                    ));
                }
                RecordingSnapshot {
                    test_case_id: test_case_id.clone(),
                    start_url: start_url.clone(),
                    captured_steps: captured_steps.clone(),
                    last_error: Some(last_error.clone()),
                    recoverable: *recoverable,
                }
            }
            RecordingState::Idle => {
                return Err(AppError::validation(
                    "Không có phiên recording hoạt động để dừng.",
                ));
            }
        };

        *state = RecordingState::Idle;
        Ok(snapshot)
    }

    pub fn cancel_recording(&self, expected_test_case_id: &str) -> AppResult<()> {
        let mut state = self.recording_state.write().unwrap();
        match &*state {
            RecordingState::Recording { test_case_id, .. }
            | RecordingState::Failed { test_case_id, .. } => {
                if test_case_id != expected_test_case_id {
                    return Err(AppError::validation(
                        "Yêu cầu cancel recording không khớp testCaseId đang hoạt động.",
                    ));
                }
                *state = RecordingState::Idle;
                Ok(())
            }
            RecordingState::Idle => Err(AppError::validation(
                "Không có phiên recording hoạt động để hủy.",
            )),
        }
    }

    pub fn run_state(&self) -> RunState {
        self.run_state.read().unwrap().clone()
    }

    pub fn replay_state(&self) -> ReplayState {
        self.replay_state.read().unwrap().clone()
    }

    pub fn start_replay(&self, run_id: String, test_case_id: String) -> AppResult<()> {
        let mut state = self.replay_state.write().unwrap();
        match &*state {
            ReplayState::Idle => {
                *state = ReplayState::Running {
                    run_id,
                    test_case_id,
                    start_time: chrono::Utc::now(),
                    cancel_requested: false,
                };
                Ok(())
            }
            ReplayState::Running { .. } => Err(AppError::new(
                crate::error::ErrorCode::StateConflict,
                "Đang có một phiên replay UI khác hoạt động.",
                "Another UI replay session is already in progress",
            )),
        }
    }

    pub fn request_replay_cancel(&self, expected_run_id: &str) -> AppResult<bool> {
        let mut state = self.replay_state.write().unwrap();
        match &mut *state {
            ReplayState::Running {
                run_id,
                cancel_requested,
                ..
            } => {
                if run_id != expected_run_id {
                    return Err(AppError::validation(
                        "Yêu cầu cancel replay không khớp runId đang hoạt động.",
                    ));
                }

                if *cancel_requested {
                    Ok(false)
                } else {
                    *cancel_requested = true;
                    Ok(true)
                }
            }
            ReplayState::Idle => Ok(false),
        }
    }

    pub fn is_replay_cancel_requested(&self, expected_run_id: &str) -> AppResult<bool> {
        let state = self.replay_state.read().unwrap();
        match &*state {
            ReplayState::Running {
                run_id,
                cancel_requested,
                ..
            } => {
                if run_id != expected_run_id {
                    return Err(AppError::validation(
                        "Yêu cầu kiểm tra replay không khớp runId đang hoạt động.",
                    ));
                }
                Ok(*cancel_requested)
            }
            ReplayState::Idle => Ok(false),
        }
    }

    pub fn finish_replay(&self, expected_run_id: &str) {
        let mut state = self.replay_state.write().unwrap();
        if let ReplayState::Running { run_id, .. } = &*state {
            if run_id == expected_run_id {
                *state = ReplayState::Idle;
            }
        }
    }

    pub fn cancel_replay(&self, expected_run_id: &str) -> AppResult<()> {
        let _ = self.request_replay_cancel(expected_run_id)?;
        self.finish_replay(expected_run_id);
        Ok(())
    }

    pub fn start_run(&self, run_id: String, suite_id: String) -> AppResult<()> {
        let mut state = self.run_state.write().unwrap();
        match &*state {
            RunState::Idle => {
                *state = RunState::Running {
                    run_id,
                    suite_id,
                    start_time: chrono::Utc::now(),
                    cancel_requested: false,
                };
                Ok(())
            }
            RunState::Running { .. } => Err(AppError::run_in_progress()),
        }
    }

    pub fn request_run_cancel(&self, expected_run_id: &str) -> AppResult<bool> {
        let mut state = self.run_state.write().unwrap();
        match &mut *state {
            RunState::Running {
                run_id,
                cancel_requested,
                ..
            } => {
                if run_id != expected_run_id {
                    return Err(AppError::validation(
                        "Yêu cầu cancel suite run không khớp runId đang hoạt động.",
                    ));
                }

                if *cancel_requested {
                    Ok(false)
                } else {
                    *cancel_requested = true;
                    Ok(true)
                }
            }
            RunState::Idle => Ok(false),
        }
    }

    pub fn is_run_cancel_requested(&self, expected_run_id: &str) -> AppResult<bool> {
        let state = self.run_state.read().unwrap();
        match &*state {
            RunState::Running {
                run_id,
                cancel_requested,
                ..
            } => {
                if run_id != expected_run_id {
                    return Err(AppError::validation(
                        "Yêu cầu kiểm tra suite run không khớp runId đang hoạt động.",
                    ));
                }

                Ok(*cancel_requested)
            }
            RunState::Idle => Ok(false),
        }
    }

    pub fn finish_run(&self, expected_run_id: &str) {
        let mut state = self.run_state.write().unwrap();
        if let RunState::Running { run_id, .. } = &*state {
            if run_id == expected_run_id {
                *state = RunState::Idle;
            }
        }
    }

    pub fn stop_run(&self, expected_run_id: &str) {
        self.finish_run(expected_run_id);
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

        let database =
            Database::new_with_migrations_dir(paths.database_file(), migrations_dir).unwrap();
        let secret_service = SecretService::new(paths.base.clone());

        AppState::new(
            database,
            secret_service,
            paths,
            ShellBootstrapSnapshot {
                app_version: "0.1.0".to_string(),
                is_first_run: false,
                degraded_mode: false,
                master_key_initialized: true,
            },
        )
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
        state
            .start_recording_session("script-1".to_string(), "https://example.com".to_string())
            .unwrap();
        assert!(matches!(
            state.recording_state(),
            RecordingState::Recording { .. }
        ));
        let snapshot = state.stop_recording("script-1").unwrap();
        assert_eq!(snapshot.test_case_id, "script-1");
        assert_eq!(state.recording_state(), RecordingState::Idle);
    }

    #[test]
    fn concurrent_recording_is_rejected() {
        let state = create_state();

        state
            .start_recording_session("script-1".to_string(), "https://example.com".to_string())
            .unwrap();
        let result = state
            .start_recording_session("script-2".to_string(), "https://example.com".to_string());
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(matches!(
            error.code,
            crate::error::ErrorCode::RecordingInProgress
        ));
    }

    #[test]
    fn concurrent_run_is_rejected() {
        let state = create_state();

        state
            .start_run("run-1".to_string(), "suite-1".to_string())
            .unwrap();
        let result = state.start_run("run-2".to_string(), "suite-2".to_string());
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(matches!(error.code, crate::error::ErrorCode::RunInProgress));
    }

    #[test]
    fn run_state_cancel_is_idempotent() {
        let state = create_state();

        state
            .start_run("run-1".to_string(), "suite-1".to_string())
            .unwrap();

        let first_cancel = state.request_run_cancel("run-1").unwrap();
        let second_cancel = state.request_run_cancel("run-1").unwrap();

        assert!(first_cancel);
        assert!(!second_cancel);
        assert!(state.is_run_cancel_requested("run-1").unwrap());

        state.finish_run("run-1");
        state.finish_run("run-1");

        assert_eq!(state.run_state(), RunState::Idle);
        assert!(!state.request_run_cancel("run-1").unwrap());
    }

    #[test]
    fn replay_state_transitions_are_idempotent() {
        let state = create_state();

        state
            .start_replay("run-1".to_string(), "script-1".to_string())
            .unwrap();
        assert!(matches!(state.replay_state(), ReplayState::Running { .. }));

        let first_cancel = state.request_replay_cancel("run-1").unwrap();
        let second_cancel = state.request_replay_cancel("run-1").unwrap();
        assert!(first_cancel);
        assert!(!second_cancel);

        assert!(state.is_replay_cancel_requested("run-1").unwrap());

        state.finish_replay("run-1");
        state.finish_replay("run-1");
        assert_eq!(state.replay_state(), ReplayState::Idle);

        assert!(!state.request_replay_cancel("run-1").unwrap());
    }
}
