//! Browser automation service baseline for T11 runtime health/fallback scaffolding.
//!
//! Service này chỉ cung cấp abstraction Chromium-only và health-check ổn định
//! qua DTO/contracts. Không expose bất kỳ Playwright/browser internals nào
//! ra ngoài service boundary.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde_json::json;
use tauri::Emitter;
use uuid::Uuid;

use crate::contracts::domain::{
    BrowserRuntimeStatus, RecordingStatus, ReplayStatus, StepAction, TestCaseType,
};
use crate::contracts::dto::{BrowserHealthDto, UiReplayResultDto, UiStepDto, UiTestCaseDto};
use crate::contracts::events::{
    BrowserRecordingStatusChangedEvent, BrowserRecordingStepCapturedEvent,
    BrowserReplayProgressEvent,
};
use crate::error::{AppError, AppResult};
use crate::services::artifact_service::{ArtifactKind, ArtifactService};
use crate::state::{AppState, RecordingSnapshot};
use crate::utils::paths::AppPaths;

/// Chromium-only runtime service boundary.
pub struct BrowserAutomationService {
    paths: AppPaths,
}

impl BrowserAutomationService {
    /// Create a new browser automation service.
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    /// Check current browser runtime health for Chromium-only baseline.
    pub fn check_runtime_health(&self) -> BrowserHealthDto {
        let runtime = self.detect_chromium_runtime();

        BrowserHealthDto {
            runtime_status: runtime.status,
            message: runtime.message,
            checked_at: Utc::now().to_rfc3339(),
        }
    }

    /// Emit browser health changed event using stable event contract.
    pub fn emit_health_changed(
        &self,
        app: &tauri::AppHandle,
        health: &BrowserHealthDto,
    ) -> AppResult<()> {
        app.emit("browser.health.changed", health).map_err(|error| {
            AppError::internal(format!(
                "Không thể phát browser.health.changed event: {error}"
            ))
        })
    }

    /// Start a recording session and emit initial recording events.
    pub fn start_recording(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        test_case_id: &str,
        start_url: &str,
    ) -> AppResult<()> {
        state.start_recording_session(test_case_id.to_string(), start_url.to_string())?;
        self.emit_recording_status(app, test_case_id, RecordingStatus::Recording)?;

        // Step bootstrap tối thiểu: luôn capture bước navigate đầu phiên.
        let bootstrap_step = UiStepDto {
            id: format!("step-{}", Uuid::new_v4()),
            action: StepAction::Navigate,
            selector: None,
            value: Some(start_url.to_string()),
            timeout_ms: Some(state.config().default_step_timeout_ms as u64),
            confidence: None,
        };
        state.record_captured_step(bootstrap_step.clone())?;
        self.emit_recording_step_captured(app, test_case_id, &bootstrap_step)?;

        let health = self.check_runtime_health();
        if health.runtime_status == BrowserRuntimeStatus::Unavailable {
            let message = "Browser runtime unavailable during recording start".to_string();
            state.mark_recording_failed(message.clone(), true)?;
            self.emit_recording_status(app, test_case_id, RecordingStatus::Failed)?;
            return Err(AppError::new(
                crate::error::ErrorCode::BrowserRuntime,
                "Browser recorder tạm thời không khả dụng.",
                message,
            )
            .with_context("testCaseId", test_case_id)
            .with_recoverable(true));
        }

        Ok(())
    }

    /// Stop recording, normalize/calculate confidence, and persist into ui_scripts/ui_script_steps.
    pub fn stop_recording(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        test_case_id: &str,
    ) -> AppResult<UiTestCaseDto> {
        let Some(snapshot) = state.stop_recording(test_case_id)? else {
            return Err(AppError::validation(
                "Recording session is no longer active. Không có phiên recording hoạt động để dừng.",
            )
            .with_context("testCaseId", test_case_id));
        };
        if snapshot.last_error.is_some() {
            self.emit_recording_status(app, test_case_id, RecordingStatus::Failed)?;
        }

        let normalized_steps = self.normalize_steps(snapshot.captured_steps.clone());
        let persisted = self.persist_recording_snapshot(state, &snapshot, &normalized_steps)?;

        self.emit_recording_status(app, test_case_id, RecordingStatus::Stopped)?;
        Ok(persisted)
    }

    /// Cancel active recording session.
    pub fn cancel_recording(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        test_case_id: &str,
    ) -> AppResult<bool> {
        let changed = state.cancel_recording(test_case_id)?;
        if changed {
            self.emit_recording_status(app, test_case_id, RecordingStatus::Stopped)?;
        }
        Ok(changed)
    }

    /// Start Chromium-only replay from persisted ui_script_steps and execute sequentially.
    pub fn start_replay(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        test_case_id: &str,
    ) -> AppResult<UiReplayResultDto> {
        let health = self.check_runtime_health();
        if health.runtime_status != BrowserRuntimeStatus::Healthy {
            return Err(AppError::new(
                crate::error::ErrorCode::BrowserRuntime,
                "Browser replay tạm thời không khả dụng.",
                health.message,
            )
            .with_context("testCaseId", test_case_id)
            .with_recoverable(true));
        }

        let run_id = format!("run-{}", Uuid::new_v4());
        state.start_replay(run_id.clone(), test_case_id.to_string())?;

        let result = self.execute_replay(state, app, &run_id, test_case_id);
        state.finish_replay(&run_id);
        result
    }

    pub fn start_replay_for_suite_run(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        suite_run_id: &str,
        test_case_id: &str,
    ) -> AppResult<UiReplayResultDto> {
        let health = self.check_runtime_health();
        if health.runtime_status != BrowserRuntimeStatus::Healthy {
            return Err(AppError::new(
                crate::error::ErrorCode::BrowserRuntime,
                "Browser replay tạm thời không khả dụng.",
                health.message,
            )
            .with_context("testCaseId", test_case_id)
            .with_context("suiteRunId", suite_run_id)
            .with_recoverable(true));
        }

        let replay_run_id = format!("{suite_run_id}:{test_case_id}");
        state.start_replay(replay_run_id.clone(), test_case_id.to_string())?;
        let result = self.execute_replay(state, app, &replay_run_id, test_case_id);
        state.finish_replay(&replay_run_id);
        result
    }

    /// Request replay cancellation (idempotent).
    pub fn cancel_replay(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        run_id: &str,
    ) -> AppResult<bool> {
        let changed = state.request_replay_cancel(run_id)?;
        if changed {
            state.finish_replay(run_id);
        }
        if changed {
            self.emit_replay_progress(app, run_id, ReplayStatus::Cancelled, None)?;
        }
        Ok(changed)
    }

    fn detect_chromium_runtime(&self) -> BrowserRuntimeCheckResult {
        if is_chromium_runtime_explicitly_disabled() {
            return BrowserRuntimeCheckResult {
                status: BrowserRuntimeStatus::Unavailable,
                message: "Chromium runtime is explicitly unavailable by configuration. Browser automation is disabled for this runtime.".to_string(),
            };
        }

        let expected = self.chromium_candidates();
        let discovered = expected.iter().find(|path| path.exists());

        if let Some(path) = discovered {
            return BrowserRuntimeCheckResult {
                status: BrowserRuntimeStatus::Healthy,
                message: format!(
                    "Chromium runtime is ready at {} (phase-1 chromium-only).",
                    path.to_string_lossy()
                ),
            };
        }

        if expected.is_empty() {
            return BrowserRuntimeCheckResult {
                status: BrowserRuntimeStatus::Unavailable,
                message: "Chromium runtime is unavailable: no discovery candidates were resolved."
                    .to_string(),
            };
        }

        BrowserRuntimeCheckResult {
            status: BrowserRuntimeStatus::Degraded,
            message: "Chromium runtime is not discovered yet. Browser flows are blocked while API-only features remain available.".to_string(),
        }
    }

    fn chromium_candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        let bundled = self
            .paths
            .base
            .join("ms-playwright")
            .join("chromium")
            .join("chrome-win")
            .join("chrome.exe");
        candidates.push(bundled);

        if let Some(from_env) = std::env::var_os("PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH") {
            candidates.push(PathBuf::from(from_env));
        }

        if let Some(browsers_root) = std::env::var_os("PLAYWRIGHT_BROWSERS_PATH") {
            let root = PathBuf::from(browsers_root);
            candidates.push(root.join("chromium").join("chrome-win").join("chrome.exe"));
        }

        candidates
    }

    fn execute_replay(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        run_id: &str,
        test_case_id: &str,
    ) -> AppResult<UiReplayResultDto> {
        let replay_script = self.load_replay_script(state, test_case_id)?;
        if replay_script.steps.is_empty() {
            return Err(
                AppError::validation("Không có step đã lưu để replay cho test case này.")
                    .with_context("testCaseId", test_case_id),
            );
        }

        let runtime = ChromiumCliReplayRuntimeAdapter::new(
            self.resolve_chromium_executable()?,
            state.config().viewport_width,
            state.config().viewport_height,
        );

        let mut current_url = if replay_script.start_url.trim().is_empty() {
            replay_script
                .steps
                .iter()
                .find(|step| step.action == StepAction::Navigate)
                .and_then(|step| step.value.clone())
                .unwrap_or_else(|| "about:blank".to_string())
        } else {
            replay_script.start_url.clone()
        };

        self.emit_replay_progress(app, run_id, ReplayStatus::Running, None)?;

        for step in replay_script.steps {
            if state.is_replay_cancel_requested(run_id)? {
                self.emit_replay_progress(app, run_id, ReplayStatus::Cancelled, Some(&step.id))?;
                return Ok(UiReplayResultDto {
                    run_id: run_id.to_string(),
                    status: ReplayStatus::Cancelled,
                    failed_step_id: None,
                    screenshot_path: None,
                });
            }

            self.emit_replay_progress(app, run_id, ReplayStatus::Running, Some(&step.id))?;

            if let Err(error) = self.execute_step(
                &runtime,
                &step,
                &mut current_url,
                state.config().default_step_timeout_ms as u64,
            ) {
                let screenshot_path = self.capture_failure_screenshot(
                    state,
                    &runtime,
                    run_id,
                    test_case_id,
                    &step.id,
                    &current_url,
                    &error.technical_message,
                )?;

                self.emit_replay_progress(app, run_id, ReplayStatus::Failed, Some(&step.id))?;
                return Ok(UiReplayResultDto {
                    run_id: run_id.to_string(),
                    status: ReplayStatus::Failed,
                    failed_step_id: Some(step.id),
                    screenshot_path,
                });
            }
        }

        self.emit_replay_progress(app, run_id, ReplayStatus::Passed, None)?;
        Ok(UiReplayResultDto {
            run_id: run_id.to_string(),
            status: ReplayStatus::Passed,
            failed_step_id: None,
            screenshot_path: None,
        })
    }

    fn execute_step(
        &self,
        runtime: &ChromiumCliReplayRuntimeAdapter,
        step: &ReplayPersistedStep,
        current_url: &mut String,
        default_timeout_ms: u64,
    ) -> AppResult<()> {
        let timeout_ms = step
            .timeout_ms
            .unwrap_or(default_timeout_ms)
            .clamp(200, 120_000);

        match step.action {
            StepAction::Navigate => {
                let target = step
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .ok_or_else(|| {
                        AppError::new(
                            crate::error::ErrorCode::StepExecution,
                            "Step navigate thiếu URL đích.",
                            "Replay step navigate is missing target URL",
                        )
                    })?;
                if !(target.starts_with("http://") || target.starts_with("https://")) {
                    return Err(AppError::new(
                        crate::error::ErrorCode::StepExecution,
                        "Step navigate có URL không hợp lệ.",
                        format!("Replay step navigate has invalid URL: {target}"),
                    ));
                }

                runtime.navigate(target, timeout_ms)?;
                *current_url = target.to_string();
            }
            StepAction::Fill => {
                self.require_selector(step)?;
                self.require_value(step)?;
                runtime.fill(
                    current_url,
                    step.selector.as_deref().unwrap_or_default(),
                    step.value.as_deref().unwrap_or_default(),
                    timeout_ms,
                )?;
            }
            StepAction::Click | StepAction::Select | StepAction::Check | StepAction::Uncheck => {
                self.require_selector(step)?;
                let selector = step.selector.as_deref().unwrap_or_default();
                match step.action {
                    StepAction::Click => runtime.click(current_url, selector, timeout_ms)?,
                    StepAction::Select => runtime.select(
                        current_url,
                        selector,
                        step.value.as_deref().unwrap_or_default(),
                        timeout_ms,
                    )?,
                    StepAction::Check => {
                        runtime.set_checked(current_url, selector, true, timeout_ms)?
                    }
                    StepAction::Uncheck => {
                        runtime.set_checked(current_url, selector, false, timeout_ms)?
                    }
                    _ => unreachable!("handled by match arm"),
                }
            }
            StepAction::WaitFor | StepAction::AssertText => {
                let has_selector = step
                    .selector
                    .as_deref()
                    .map(str::trim)
                    .map(|item| !item.is_empty())
                    .unwrap_or(false);
                let has_value = step
                    .value
                    .as_deref()
                    .map(str::trim)
                    .map(|item| !item.is_empty())
                    .unwrap_or(false);
                if !(has_selector || has_value) {
                    return Err(AppError::new(
                        crate::error::ErrorCode::StepExecution,
                        "Step thiếu điều kiện chờ/xác nhận.",
                        format!(
                            "Replay step {} is missing selector/value condition",
                            step.id
                        ),
                    ));
                }

                let dom = runtime.dump_dom(current_url, timeout_ms)?;
                if let Some(expected_text) = step
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    if !dom.contains(expected_text) {
                        return Err(AppError::new(
                            crate::error::ErrorCode::StepExecution,
                            "Nội dung mong đợi không xuất hiện trên trang.",
                            format!(
                                "Replay step {} expects text '{}' but current DOM does not contain it",
                                step.id, expected_text
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn require_selector(&self, step: &ReplayPersistedStep) -> AppResult<()> {
        let valid = step
            .selector
            .as_deref()
            .map(str::trim)
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        if valid {
            Ok(())
        } else {
            Err(AppError::new(
                crate::error::ErrorCode::StepExecution,
                "Step thiếu selector bắt buộc.",
                format!("Replay step {} is missing required selector", step.id),
            ))
        }
    }

    fn require_value(&self, step: &ReplayPersistedStep) -> AppResult<()> {
        let valid = step
            .value
            .as_deref()
            .map(str::trim)
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        if valid {
            Ok(())
        } else {
            Err(AppError::new(
                crate::error::ErrorCode::StepExecution,
                "Step thiếu value bắt buộc.",
                format!("Replay step {} is missing required value", step.id),
            ))
        }
    }

    fn load_replay_script(&self, state: &AppState, test_case_id: &str) -> AppResult<ReplayScript> {
        let db = state.db();
        let db_guard = db
            .lock()
            .map_err(|_| AppError::internal("Database lock poisoned"))?;
        let connection = db_guard.connection();

        let start_url = connection
            .query_row(
                "SELECT start_url FROM ui_scripts WHERE id = ?1",
                params![test_case_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
            .unwrap_or_default();

        let test_case_exists = connection
            .query_row(
                "SELECT ui_script_id FROM test_cases WHERE id = ?1",
                params![test_case_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(AppError::from)?;

        let Some(linked_script_id) = test_case_exists else {
            return Err(AppError::not_found("ui test case", test_case_id)
                .with_context("testCaseId", test_case_id));
        };

        if linked_script_id.as_deref() != Some(test_case_id) {
            return Err(AppError::validation(
                "Ui test case không còn script replay hợp lệ. Có thể script tham chiếu đã bị xóa hoặc không còn đồng bộ.",
            )
            .with_context("testCaseId", test_case_id));
        }

        let mut statement = connection.prepare(
            "SELECT id, step_type, selector, value, timeout_ms FROM ui_script_steps WHERE script_id = ?1 ORDER BY step_order ASC",
        )?;
        let mut rows = statement.query(params![test_case_id])?;

        let mut steps = Vec::new();
        while let Some(row) = rows.next()? {
            let step_type: String = row.get(1)?;
            let action = self.parse_storage_action(&step_type).ok_or_else(|| {
                AppError::new(
                    crate::error::ErrorCode::StepExecution,
                    "Step type không được hỗ trợ trong replay.",
                    format!("Unsupported replay step_type: {step_type}"),
                )
            })?;

            let timeout_ms = row
                .get::<_, Option<i64>>(4)?
                .map(|value| value.max(0) as u64);
            steps.push(ReplayPersistedStep {
                id: row.get(0)?,
                action,
                selector: row.get(2)?,
                value: row.get(3)?,
                timeout_ms,
            });
        }

        if steps.is_empty() {
            return Err(AppError::validation(
                "Ui test case không còn step replay khả dụng. Có thể script tham chiếu đã bị xóa hoặc rỗng.",
            )
            .with_context("testCaseId", test_case_id));
        }

        Ok(ReplayScript { start_url, steps })
    }

    fn resolve_chromium_executable(&self) -> AppResult<PathBuf> {
        self.chromium_candidates()
            .into_iter()
            .find(|path| path.exists())
            .ok_or_else(|| {
                AppError::new(
                    crate::error::ErrorCode::BrowserRuntime,
                    "Không tìm thấy Chromium runtime khả dụng để replay.",
                    "No Chromium executable candidate found for replay runtime",
                )
                .with_recoverable(true)
            })
    }

    fn parse_storage_action(&self, value: &str) -> Option<StepAction> {
        match value {
            "navigate" => Some(StepAction::Navigate),
            "click" => Some(StepAction::Click),
            "fill" => Some(StepAction::Fill),
            "select" => Some(StepAction::Select),
            "check" => Some(StepAction::Check),
            "uncheck" => Some(StepAction::Uncheck),
            "wait_for" => Some(StepAction::WaitFor),
            "assert_text" => Some(StepAction::AssertText),
            _ => None,
        }
    }

    fn capture_failure_screenshot(
        &self,
        state: &AppState,
        runtime: &ChromiumCliReplayRuntimeAdapter,
        run_id: &str,
        test_case_id: &str,
        failed_step_id: &str,
        current_url: &str,
        error_message: &str,
    ) -> AppResult<Option<String>> {
        if !state.config().screenshot_on_fail {
            return Ok(None);
        }

        let artifact_service = ArtifactService::new(self.paths.clone());
        let scope = format!("replay-{run_id}");
        let file_name = format!("{test_case_id}-{failed_step_id}.png");
        let screenshot_path =
            artifact_service.resolve_artifact_path(ArtifactKind::Screenshot, &scope, &file_name)?;
        runtime.capture_screenshot(
            if current_url.trim().is_empty() {
                "about:blank"
            } else {
                current_url
            },
            &screenshot_path,
            state.config().default_timeout_ms as u64,
        )?;

        let screenshot_size = fs::metadata(&screenshot_path)
            .map_err(|error| {
                AppError::storage_read(format!(
                    "Không thể đọc metadata screenshot vừa tạo: {error}"
                ))
            })?
            .len();
        if screenshot_size == 0 {
            return Err(AppError::new(
                crate::error::ErrorCode::StorageWrite,
                "Screenshot thất bại: file ảnh rỗng.",
                format!(
                    "Captured screenshot file is empty at {}",
                    screenshot_path.to_string_lossy()
                ),
            ));
        }

        let relative_path = screenshot_path
            .strip_prefix(&self.paths.base)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| format!("screenshots/{scope}/{file_name}"));

        let manifest = crate::contracts::dto::ArtifactManifestDto {
            id: format!("artifact-{}", Uuid::new_v4()),
            artifact_type: ArtifactKind::Screenshot.as_str().to_string(),
            logical_name: format!("replay-failure-{test_case_id}"),
            file_path: screenshot_path.to_string_lossy().into_owned(),
            relative_path,
            preview_json: json!({
                "runId": run_id,
                "testCaseId": test_case_id,
                "failedStepId": failed_step_id,
                "url": current_url,
                "error": error_message,
            })
            .to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        let db = state.db();
        let db_guard = db
            .lock()
            .map_err(|_| AppError::internal("Database lock poisoned"))?;
        artifact_service.persist_artifact_manifest(db_guard.connection(), &manifest)?;

        Ok(Some(screenshot_path.to_string_lossy().into_owned()))
    }
}

#[derive(Debug, Clone)]
struct NormalizedRecordedStep {
    action: StepAction,
    selector: Option<String>,
    value: Option<String>,
    timeout_ms: u64,
    description: String,
    confidence: &'static str,
}

impl BrowserAutomationService {
    fn normalize_steps(&self, raw_steps: Vec<UiStepDto>) -> Vec<NormalizedRecordedStep> {
        raw_steps
            .into_iter()
            .map(|step| {
                let selector = step
                    .selector
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty());
                let value = step
                    .value
                    .as_ref()
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty());
                let timeout_ms = step.timeout_ms.unwrap_or(5_000).clamp(200, 120_000);
                let confidence =
                    self.compute_confidence(step.action, selector.as_deref(), value.as_deref());
                let description =
                    self.build_step_description(step.action, selector.as_deref(), value.as_deref());

                NormalizedRecordedStep {
                    action: step.action,
                    selector,
                    value,
                    timeout_ms,
                    description,
                    confidence,
                }
            })
            .collect()
    }

    fn compute_confidence(
        &self,
        action: StepAction,
        selector: Option<&str>,
        value: Option<&str>,
    ) -> &'static str {
        let strong_selector = selector
            .map(|item| {
                item.starts_with("#") || item.contains("data-testid") || item.contains("[name=")
            })
            .unwrap_or(false);
        let weak_selector = selector
            .map(|item| {
                item.starts_with('.') || item.contains("nth-child") || item.contains(":nth")
            })
            .unwrap_or(false);

        match action {
            StepAction::Navigate => {
                if value
                    .map(|item| item.starts_with("http://") || item.starts_with("https://"))
                    .unwrap_or(false)
                {
                    "high"
                } else {
                    "medium"
                }
            }
            StepAction::Click | StepAction::Select | StepAction::Check | StepAction::Uncheck => {
                if strong_selector {
                    "high"
                } else if selector.is_some() && !weak_selector {
                    "medium"
                } else {
                    "low"
                }
            }
            StepAction::Fill | StepAction::AssertText => {
                if strong_selector && value.is_some() {
                    "high"
                } else if selector.is_some() || value.is_some() {
                    "medium"
                } else {
                    "low"
                }
            }
            StepAction::WaitFor => {
                if strong_selector || value.is_some() {
                    "medium"
                } else {
                    "low"
                }
            }
        }
    }

    fn build_step_description(
        &self,
        action: StepAction,
        selector: Option<&str>,
        value: Option<&str>,
    ) -> String {
        match action {
            StepAction::Navigate => format!("Navigate to {}", value.unwrap_or("target URL")),
            StepAction::Click => format!("Click {}", selector.unwrap_or("target element")),
            StepAction::Fill => format!(
                "Fill {} with {}",
                selector.unwrap_or("target field"),
                value.unwrap_or("value")
            ),
            StepAction::Select => format!(
                "Select {} on {}",
                value.unwrap_or("option"),
                selector.unwrap_or("target select")
            ),
            StepAction::Check => format!("Check {}", selector.unwrap_or("target element")),
            StepAction::Uncheck => format!("Uncheck {}", selector.unwrap_or("target element")),
            StepAction::WaitFor => {
                format!("Wait for {}", selector.or(value).unwrap_or("condition"))
            }
            StepAction::AssertText => format!(
                "Assert text {} on {}",
                value.unwrap_or("value"),
                selector.unwrap_or("target element")
            ),
        }
    }

    fn persist_recording_snapshot(
        &self,
        state: &AppState,
        snapshot: &RecordingSnapshot,
        normalized_steps: &[NormalizedRecordedStep],
    ) -> AppResult<UiTestCaseDto> {
        let script_name = self.resolve_script_name(state, &snapshot.test_case_id)?;
        let db = state.db();
        let db_guard = db
            .lock()
            .map_err(|_| AppError::internal("Database lock poisoned"))?;
        let connection = db_guard.connection();
        let now = Utc::now().to_rfc3339();

        connection.execute(
            "INSERT OR REPLACE INTO ui_scripts (id, name, description, start_url, viewport_width, viewport_height, timeout_ms, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, COALESCE((SELECT created_at FROM ui_scripts WHERE id = ?1), ?8), ?9)",
            params![
                snapshot.test_case_id,
                script_name,
                snapshot.last_error,
                snapshot.start_url,
                state.config().viewport_width as i64,
                state.config().viewport_height as i64,
                state.config().default_timeout_ms as i64,
                now,
                now,
            ],
        )?;

        connection.execute(
            "INSERT OR REPLACE INTO test_cases (id, name, description, case_type, api_endpoint_id, ui_script_id, data_table_id, tags_json, enabled, created_at, updated_at) VALUES (?1, COALESCE((SELECT name FROM test_cases WHERE id = ?1), ?2), NULL, 'ui', NULL, ?3, NULL, '[]', 1, COALESCE((SELECT created_at FROM test_cases WHERE id = ?1), ?4), ?5)",
            params![
                snapshot.test_case_id,
                script_name,
                snapshot.test_case_id,
                now,
                now,
            ],
        )?;

        connection.execute(
            "DELETE FROM ui_script_steps WHERE script_id = ?1",
            params![snapshot.test_case_id],
        )?;

        for (index, step) in normalized_steps.iter().enumerate() {
            connection.execute(
                "INSERT INTO ui_script_steps (id, script_id, step_order, step_type, selector, value, timeout_ms, description, confidence, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    format!("step-{}", Uuid::new_v4()),
                    snapshot.test_case_id,
                    index as i64,
                    self.step_action_to_storage(step.action),
                    step.selector,
                    step.value,
                    step.timeout_ms as i64,
                    step.description,
                    step.confidence,
                    now,
                    now,
                ],
            )?;
        }

        let result_steps = normalized_steps
            .iter()
            .map(|item| UiStepDto {
                id: format!("step-{}", Uuid::new_v4()),
                action: item.action,
                selector: item.selector.clone(),
                value: item.value.clone(),
                timeout_ms: Some(item.timeout_ms),
                confidence: None,
            })
            .collect::<Vec<_>>();

        Ok(UiTestCaseDto {
            id: snapshot.test_case_id.clone(),
            r#type: TestCaseType::Ui,
            name: script_name,
            start_url: snapshot.start_url.clone(),
            steps: result_steps,
        })
    }

    fn step_action_to_storage(&self, action: StepAction) -> &'static str {
        match action {
            StepAction::Navigate => "navigate",
            StepAction::Click => "click",
            StepAction::Fill => "fill",
            StepAction::Select => "select",
            StepAction::Check => "check",
            StepAction::Uncheck => "uncheck",
            StepAction::WaitFor => "wait_for",
            StepAction::AssertText => "assert_text",
        }
    }

    fn resolve_script_name(&self, state: &AppState, test_case_id: &str) -> AppResult<String> {
        let db = state.db();
        let db_guard = db
            .lock()
            .map_err(|_| AppError::internal("Database lock poisoned"))?;
        let connection = db_guard.connection();

        let result = connection.query_row(
            "SELECT name FROM test_cases WHERE id = ?1",
            params![test_case_id],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(name) if !name.trim().is_empty() => Ok(name),
            _ => Ok(format!("UI Script {}", test_case_id)),
        }
    }

    fn emit_recording_status(
        &self,
        app: &tauri::AppHandle,
        test_case_id: &str,
        status: RecordingStatus,
    ) -> AppResult<()> {
        let payload = BrowserRecordingStatusChangedEvent {
            test_case_id: test_case_id.to_string(),
            status,
        };
        app.emit("browser.recording.status.changed", payload)
            .map_err(|error| {
                AppError::internal(format!("Không thể phát event status recording: {error}"))
            })
    }

    fn emit_recording_step_captured(
        &self,
        app: &tauri::AppHandle,
        test_case_id: &str,
        step: &UiStepDto,
    ) -> AppResult<()> {
        let payload = BrowserRecordingStepCapturedEvent {
            test_case_id: test_case_id.to_string(),
            step: step.clone(),
        };
        app.emit("browser.recording.step.captured", payload)
            .map_err(|error| {
                AppError::internal(format!("Không thể phát event step captured: {error}"))
            })
    }

    fn emit_replay_progress(
        &self,
        app: &tauri::AppHandle,
        run_id: &str,
        status: ReplayStatus,
        current_step_id: Option<&str>,
    ) -> AppResult<()> {
        let payload = BrowserReplayProgressEvent {
            run_id: run_id.to_string(),
            status,
            current_step_id: current_step_id.map(ToOwned::to_owned),
        };

        app.emit("browser.replay.progress", payload)
            .map_err(|error| {
                AppError::internal(format!("Không thể phát event replay progress: {error}"))
            })
    }
}

#[derive(Debug, Clone)]
struct ReplayPersistedStep {
    id: String,
    action: StepAction,
    selector: Option<String>,
    value: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
struct ReplayScript {
    start_url: String,
    steps: Vec<ReplayPersistedStep>,
}

#[derive(Debug)]
struct ChromiumCliReplayRuntimeAdapter {
    executable_path: PathBuf,
    viewport_width: u32,
    viewport_height: u32,
    last_dom_snapshot: Mutex<Option<String>>,
}

impl ChromiumCliReplayRuntimeAdapter {
    fn new(executable_path: PathBuf, viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            executable_path,
            viewport_width,
            viewport_height,
            last_dom_snapshot: Mutex::new(None),
        }
    }

    fn click(&self, url: &str, selector: &str, timeout_ms: u64) -> AppResult<()> {
        self.execute_interaction(url, StepAction::Click, selector, None, None, timeout_ms)
    }

    fn fill(&self, url: &str, selector: &str, value: &str, timeout_ms: u64) -> AppResult<()> {
        self.execute_interaction(
            url,
            StepAction::Fill,
            selector,
            Some(value.to_string()),
            None,
            timeout_ms,
        )
    }

    fn select(&self, url: &str, selector: &str, value: &str, timeout_ms: u64) -> AppResult<()> {
        self.execute_interaction(
            url,
            StepAction::Select,
            selector,
            Some(value.to_string()),
            None,
            timeout_ms,
        )
    }

    fn set_checked(
        &self,
        url: &str,
        selector: &str,
        checked: bool,
        timeout_ms: u64,
    ) -> AppResult<()> {
        self.execute_interaction(
            url,
            if checked {
                StepAction::Check
            } else {
                StepAction::Uncheck
            },
            selector,
            None,
            Some(checked),
            timeout_ms,
        )
    }

    fn navigate(&self, url: &str, timeout_ms: u64) -> AppResult<()> {
        let window_size = format!("{},{}", self.viewport_width, self.viewport_height);
        let virtual_budget = timeout_ms.max(200).to_string();

        let output = Command::new(&self.executable_path)
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(format!("--window-size={window_size}"))
            .arg(format!("--virtual-time-budget={virtual_budget}"))
            .arg(url)
            .output()
            .map_err(|error| {
                AppError::new(
                    crate::error::ErrorCode::BrowserLaunch,
                    "Không thể khởi chạy Chromium runtime.",
                    format!("Failed to launch Chromium CLI for navigate: {error}"),
                )
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::new(
                crate::error::ErrorCode::StepExecution,
                "Step navigate thất bại trong Chromium runtime.",
                format!(
                    "Chromium navigate command failed (status: {:?}): {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))
        }
    }

    fn dump_dom(&self, url: &str, timeout_ms: u64) -> AppResult<String> {
        if let Ok(snapshot_guard) = self.last_dom_snapshot.lock() {
            if let Some(snapshot) = snapshot_guard.as_ref() {
                return Ok(snapshot.clone());
            }
        }

        let window_size = format!("{},{}", self.viewport_width, self.viewport_height);
        let virtual_budget = timeout_ms.max(200).to_string();

        let output = Command::new(&self.executable_path)
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(format!("--window-size={window_size}"))
            .arg(format!("--virtual-time-budget={virtual_budget}"))
            .arg("--dump-dom")
            .arg(url)
            .output()
            .map_err(|error| {
                AppError::new(
                    crate::error::ErrorCode::BrowserLaunch,
                    "Không thể truy vấn DOM từ Chromium runtime.",
                    format!("Failed to launch Chromium CLI for --dump-dom: {error}"),
                )
            })?;

        if !output.status.success() {
            return Err(AppError::new(
                crate::error::ErrorCode::StepExecution,
                "Không thể lấy DOM để replay step.",
                format!(
                    "Chromium --dump-dom failed (status: {:?}): {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        let dom = String::from_utf8_lossy(&output.stdout).to_string();
        if let Ok(mut snapshot_guard) = self.last_dom_snapshot.lock() {
            *snapshot_guard = Some(dom.clone());
        }
        Ok(dom)
    }

    fn capture_screenshot(
        &self,
        url: &str,
        output_path: &PathBuf,
        timeout_ms: u64,
    ) -> AppResult<()> {
        let window_size = format!("{},{}", self.viewport_width, self.viewport_height);
        let virtual_budget = timeout_ms.max(200).to_string();
        let output_file = output_path.to_string_lossy().to_string();

        let output = Command::new(&self.executable_path)
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg(format!("--window-size={window_size}"))
            .arg(format!("--virtual-time-budget={virtual_budget}"))
            .arg(format!("--screenshot={output_file}"))
            .arg(url)
            .output()
            .map_err(|error| {
                AppError::new(
                    crate::error::ErrorCode::BrowserLaunch,
                    "Không thể chụp screenshot từ Chromium runtime.",
                    format!("Failed to launch Chromium CLI for screenshot: {error}"),
                )
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::new(
                crate::error::ErrorCode::StepExecution,
                "Screenshot-on-fail thất bại.",
                format!(
                    "Chromium --screenshot failed (status: {:?}): {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))
        }
    }

    fn execute_interaction(
        &self,
        url: &str,
        action: StepAction,
        selector: &str,
        value: Option<String>,
        checked: Option<bool>,
        timeout_ms: u64,
    ) -> AppResult<()> {
        let payload = json!({
            "chromiumPath": self.executable_path.to_string_lossy().to_string(),
            "url": url,
            "action": step_action_name(action),
            "selector": selector,
            "value": value,
            "checked": checked,
            "timeoutMs": timeout_ms,
            "viewportWidth": self.viewport_width,
            "viewportHeight": self.viewport_height,
        })
        .to_string();

        let output = Command::new("node")
            .arg("-e")
            .arg(NODE_CDP_INTERACTION_SCRIPT)
            .arg(payload)
            .output()
            .map_err(|error| {
                AppError::new(
                    crate::error::ErrorCode::BrowserLaunch,
                    "Không thể khởi chạy Node CDP runtime.",
                    format!("Failed to spawn node for CDP interaction: {error}"),
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed: NodeInteractionResult = serde_json::from_str(&stdout).map_err(|error| {
            AppError::new(
                crate::error::ErrorCode::BrowserRuntime,
                "Không đọc được phản hồi interaction từ browser runtime.",
                format!("Failed to parse interaction output '{stdout}': {error}"),
            )
        })?;

        if !output.status.success() || !parsed.ok {
            let raw_error = parsed.error.unwrap_or_else(|| {
                format!(
                    "Node CDP interaction exited with status {:?}",
                    output.status.code()
                )
            });
            let detailed_error = if raw_error.contains("Selector not found") {
                format!(
                    "Selector not found for '{}' during browser interaction.",
                    selector
                )
            } else {
                raw_error
            };
            return Err(AppError::new(
                crate::error::ErrorCode::StepExecution,
                "Interaction step thất bại trong browser runtime.",
                detailed_error,
            ));
        }

        if let Ok(mut snapshot_guard) = self.last_dom_snapshot.lock() {
            *snapshot_guard = Some(parsed.dom);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeInteractionResult {
    ok: bool,
    dom: String,
    error: Option<String>,
}

const NODE_CDP_INTERACTION_SCRIPT: &str = r#"
const cp = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

async function sleep(ms){ return new Promise((r)=>setTimeout(r, ms)); }

function normalizeUrl(value){
  const raw = String(value || '').trim();
  if (!raw) return '';
  return raw.endsWith('/') ? raw.slice(0, -1) : raw;
}

async function waitWs(port, timeoutMs, expectedUrl){
  const normalizedExpected = normalizeUrl(expectedUrl);
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      if (response.ok) {
        const pages = await response.json();
        const page = Array.isArray(pages)
          ? pages.find((item) => {
              if (!item || !item.webSocketDebuggerUrl || item.type !== 'page') return false;
              if (!normalizedExpected) return true;
              const normalizedPageUrl = normalizeUrl(item.url);
              return normalizedPageUrl === normalizedExpected;
            }) ||
            pages.find((item) => {
              if (!item || !item.webSocketDebuggerUrl || item.type !== 'page') return false;
              return !String(item.url || '').startsWith('chrome-extension://');
            })
          : null;
        if (page && page.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
      }
    } catch (_) {}
    await sleep(60);
  }
  throw new Error('Cannot discover CDP websocket endpoint');
}

async function run(){
  const payload = JSON.parse(process.argv[1]);
  const port = 42000 + Math.floor(Math.random() * 3000);
  const userDir = fs.mkdtempSync(path.join(os.tmpdir(), 'tf-cdp-'));
  const chrome = cp.spawn(payload.chromiumPath, [
    '--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
    `--window-size=${payload.viewportWidth},${payload.viewportHeight}`,
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${userDir}`,
    payload.url,
  ], { stdio: ['ignore', 'pipe', 'pipe'] });

  let ws;
  try {
    const wsUrl = await waitWs(port, Math.max(1200, payload.timeoutMs || 5000), payload.url);
    ws = new WebSocket(wsUrl);
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', () => resolve());
      ws.addEventListener('error', () => reject(new Error('WebSocket open failed')));
    });

    let id = 0;
    const pending = new Map();
    ws.addEventListener('message', (event) => {
      const data = JSON.parse(event.data.toString());
      if (data.id && pending.has(data.id)) {
        const promise = pending.get(data.id);
        pending.delete(data.id);
        if (data.error) {
          promise.reject(new Error(data.error.message || 'CDP error'));
        } else if (data.exceptionDetails) {
          promise.reject(new Error(data.exceptionDetails.text || 'Runtime.evaluate exception'));
        } else {
          promise.resolve(data.result || {});
        }
      }
    });

    const send = (method, params = {}) => {
      id += 1;
      const currentId = id;
      return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          pending.delete(currentId);
          reject(new Error('CDP command timeout: ' + method));
        }, Math.max(1200, payload.timeoutMs || 5000));
        pending.set(currentId, {
          resolve: (value) => {
            clearTimeout(timeout);
            resolve(value);
          },
          reject: (error) => {
            clearTimeout(timeout);
            reject(error);
          },
        });
        ws.send(JSON.stringify({ id: currentId, method, params }));
      });
    };

    await send('Page.enable');
    await send('Runtime.enable');
    await send('DOM.enable');

    const safeSelector = JSON.stringify(payload.selector);
    const safeValue = JSON.stringify(payload.value);
    const checked = payload.checked === true ? 'true' : 'false';

    const readinessExpr = `
      (() => {
        const selector = ${safeSelector};
        const ready = document.readyState === 'complete' || document.readyState === 'interactive';
        const present = selector ? Boolean(document.querySelector(selector)) : true;
        return ready && present;
      })()
    `;
    const readyDeadline = Date.now() + Math.max(1200, payload.timeoutMs || 5000);
    let ready = false;
    while (Date.now() < readyDeadline) {
      const readyResult = await send('Runtime.evaluate', { expression: readinessExpr, awaitPromise: true, returnByValue: true });
      ready = Boolean(readyResult?.result?.value);
      if (ready) break;
      await sleep(60);
    }
    if (!ready) {
      throw new Error('Replay target DOM not ready');
    }

    let expr = '';
    if (payload.action === 'click') {
      expr = `(() => { const el = document.querySelector(${safeSelector}); if (!el) throw new Error('Selector not found'); el.click(); return true; })()`;
    } else if (payload.action === 'fill') {
      expr = `(() => { const el = document.querySelector(${safeSelector}); if (!el) throw new Error('Selector not found'); el.focus(); el.value = ${safeValue}; el.dispatchEvent(new Event('input', {bubbles:true})); el.dispatchEvent(new Event('change', {bubbles:true})); return el.value; })()`;
    } else if (payload.action === 'select') {
      expr = `(() => { const el = document.querySelector(${safeSelector}); if (!el) throw new Error('Selector not found'); el.value = ${safeValue}; el.dispatchEvent(new Event('change', {bubbles:true})); return el.value; })()`;
    } else if (payload.action === 'check' || payload.action === 'uncheck') {
      expr = `(() => { const el = document.querySelector(${safeSelector}); if (!el) throw new Error('Selector not found'); el.checked = ${checked}; el.dispatchEvent(new Event('input', {bubbles:true})); el.dispatchEvent(new Event('change', {bubbles:true})); return el.checked; })()`;
    } else {
      throw new Error(`Unsupported action ${payload.action}`);
    }

    await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
    const dom = await send('Runtime.evaluate', { expression: 'document.documentElement.outerHTML', awaitPromise: true, returnByValue: true });
    process.stdout.write(JSON.stringify({ ok: true, dom: dom?.result?.value || '' }));
  } catch (error) {
    process.stdout.write(JSON.stringify({ ok: false, dom: '', error: String(error && error.message ? error.message : error) }));
    process.exitCode = 1;
  } finally {
    try { if (ws && ws.readyState === WebSocket.OPEN) ws.close(); } catch (_) {}
    try { chrome.kill('SIGKILL'); } catch (_) {}
    try { fs.rmSync(userDir, { recursive: true, force: true }); } catch (_) {}
  }
}

run();
"#;

fn step_action_name(action: StepAction) -> &'static str {
    match action {
        StepAction::Navigate => "navigate",
        StepAction::Click => "click",
        StepAction::Fill => "fill",
        StepAction::Select => "select",
        StepAction::Check => "check",
        StepAction::Uncheck => "uncheck",
        StepAction::WaitFor => "wait_for",
        StepAction::AssertText => "assert_text",
    }
}

fn is_chromium_runtime_explicitly_disabled() -> bool {
    std::env::var("TESTFORGE_BROWSER_AUTOMATION_DISABLED")
        .ok()
        .map(|value| value.trim() == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

struct BrowserRuntimeCheckResult {
    status: BrowserRuntimeStatus,
    message: String,
}
