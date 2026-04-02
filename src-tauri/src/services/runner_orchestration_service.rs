use std::collections::HashSet;

use chrono::Utc;
use serde_json::json;
use tauri::Emitter;
use uuid::Uuid;

use crate::contracts::commands::RunnerSuiteExecuteResponse;
use crate::contracts::domain::{ReplayStatus, RunStatus, TestCaseType};
use crate::contracts::dto::{RunResultDto, UiReplayResultDto};
use crate::contracts::events::{RunnerExecutionProgressEvent, RunnerExecutionStartedEvent};
use crate::error::{AppError, AppResult, TestForgeError};
use crate::models::DataTableRow;
use crate::repositories::{ApiRepository, PersistedSuiteCase, RunnerRepository};
use crate::services::{ApiExecutionService, BrowserAutomationService, SecretService};
use crate::state::AppState;

#[derive(Debug, Clone)]
struct ExecutionTarget {
    test_case_id: String,
    test_case_type: TestCaseType,
    data_row_id: Option<String>,
}

pub struct RunnerOrchestrationService<'a> {
    runner_repository: RunnerRepository<'a>,
    api_execution_service: ApiExecutionService<'a>,
    browser_automation_service: BrowserAutomationService,
}

impl<'a> RunnerOrchestrationService<'a> {
    pub fn new(
        runner_repository: RunnerRepository<'a>,
        api_execution_service: ApiExecutionService<'a>,
        browser_automation_service: BrowserAutomationService,
    ) -> Self {
        Self {
            runner_repository,
            api_execution_service,
            browser_automation_service,
        }
    }

    pub async fn execute_suite(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        suite_id: &str,
        environment_id: &str,
        rerun_failed_from_run_id: Option<&str>,
    ) -> AppResult<RunnerSuiteExecuteResponse> {
        let suite = self
            .runner_repository
            .load_suite(suite_id)
            .map_err(map_runner_error)?;

        if suite.cases.is_empty() {
            return Err(AppError::validation("Suite rỗng, không có test case để chạy.")
                .with_context("suiteId", suite_id));
        }

        let rerun_targets = rerun_failed_from_run_id
            .map(|run_id| self.runner_repository.load_failed_targets(run_id, suite_id))
            .transpose()
            .map_err(map_runner_error)?
            .unwrap_or_default();

        if rerun_failed_from_run_id.is_some() && rerun_targets.is_empty() {
            return Err(AppError::validation(
                "Không có failed result phù hợp để rerun cho suite run đã chọn.",
            )
            .with_context("suiteId", suite_id)
            .with_context("rerunFailedFromRunId", rerun_failed_from_run_id));
        }

        let rerun_target_keys = rerun_targets
            .iter()
            .map(|item| (item.case_id.clone(), item.data_row_id.clone()))
            .collect::<HashSet<(String, Option<String>)>>();

        let execution_plan = self
            .build_execution_plan(&suite.cases, rerun_failed_from_run_id, &rerun_target_keys)
            .map_err(map_runner_error)?;

        if execution_plan.is_empty() {
            return Err(AppError::validation(
                "Suite không có target khả dụng sau khi expand data rows enabled.",
            )
            .with_context("suiteId", suite_id));
        }

        let total_count = execution_plan.len() as u32;
        let run_id = format!("run-{}", Uuid::new_v4());
        state.start_run(run_id.clone(), suite_id.to_string())?;

        self.runner_repository
            .create_suite_run(&run_id, suite_id, environment_id, total_count)
            .map_err(map_runner_error)?;
        self.runner_repository
            .update_run_summary(
                &run_id,
                RunStatus::Running,
                0,
                0,
                0,
                None,
            )
            .map_err(map_runner_error)?;

        self.emit_started(app, &run_id, suite_id, environment_id)?;

        let execution = self
            .execute_plan(state, app, &run_id, environment_id, &execution_plan)
            .await;

        state.finish_run(&run_id);

        match execution {
            Ok(_) => Ok(RunnerSuiteExecuteResponse {
                run_id,
                suite: suite.dto,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn cancel_suite(
        &self,
        state: &AppState,
        run_id: &str,
        _app: &tauri::AppHandle,
    ) -> AppResult<bool> {
        state.request_run_cancel(run_id)
    }

    fn build_execution_plan(
        &self,
        cases: &[PersistedSuiteCase],
        rerun_failed_from_run_id: Option<&str>,
        rerun_target_keys: &HashSet<(String, Option<String>)>,
    ) -> Result<Vec<ExecutionTarget>, TestForgeError> {
        let mut plan = Vec::new();

        for case in cases {
            if !case.enabled {
                continue;
            }

            if rerun_failed_from_run_id.is_some() {
                if case.data_table_id.is_some() {
                    let rows = self.load_rows_for_case(case)?;
                    for row in rows {
                        if rerun_target_keys.contains(&(case.test_case_id.clone(), Some(row.id.clone()))) {
                            plan.push(ExecutionTarget {
                                test_case_id: case.test_case_id.clone(),
                                test_case_type: case.case_type,
                                data_row_id: Some(row.id),
                            });
                        }
                    }

                    if rerun_target_keys.contains(&(case.test_case_id.clone(), None)) {
                        plan.push(ExecutionTarget {
                            test_case_id: case.test_case_id.clone(),
                            test_case_type: case.case_type,
                            data_row_id: None,
                        });
                    }
                } else if rerun_target_keys.contains(&(case.test_case_id.clone(), None)) {
                    plan.push(ExecutionTarget {
                        test_case_id: case.test_case_id.clone(),
                        test_case_type: case.case_type,
                        data_row_id: None,
                    });
                }

                continue;
            }

            if case.data_table_id.is_some() {
                let rows = self.load_rows_for_case(case)?;
                if rows.is_empty() {
                    continue;
                }

                for row in rows {
                    plan.push(ExecutionTarget {
                        test_case_id: case.test_case_id.clone(),
                        test_case_type: case.case_type,
                        data_row_id: Some(row.id),
                    });
                }
                continue;
            }

            plan.push(ExecutionTarget {
                test_case_id: case.test_case_id.clone(),
                test_case_type: case.case_type,
                data_row_id: None,
            });
        }

        Ok(plan)
    }

    async fn execute_plan(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        run_id: &str,
        environment_id: &str,
        execution_plan: &[ExecutionTarget],
    ) -> AppResult<RunResultDto> {
        let _api_parallel_limit = state.config().max_concurrent_api.min(4);
        let total_count = execution_plan.len() as u32;
        let mut passed_count = 0u32;
        let mut failed_count = 0u32;
        let skipped_count = 0u32;
        let mut completed_count = 0u32;

        for target in execution_plan {
            if state.is_run_cancel_requested(run_id)? {
                let result = self
                    .finalize_run(
                        app,
                        run_id,
                        RunStatus::Cancelled,
                        passed_count,
                        failed_count,
                        skipped_count,
                    )
                    .map_err(map_runner_error)?;
                return Ok(result);
            }

            let target_result = match target.test_case_type {
                TestCaseType::Api => self
                    .execute_api_target(run_id, environment_id, target)
                    .await,
                TestCaseType::Ui => self.execute_ui_target(state, app, run_id, target),
            };

            let (status, data_row_id) = match target_result {
                Ok(value) => value,
                Err(error) => {
                    let _ = self.finalize_failed_run(
                        app,
                        run_id,
                        passed_count,
                        failed_count.saturating_add(1),
                        skipped_count,
                    );
                    return Err(error);
                }
            };

            match status {
                RunStatus::Passed => passed_count += 1,
                RunStatus::Failed => failed_count += 1,
                RunStatus::Skipped => {}
                RunStatus::Cancelled => {}
                RunStatus::Running | RunStatus::Queued | RunStatus::Idle => {}
            }
            completed_count += 1;

            self.emit_progress(
                app,
                run_id,
                &target.test_case_id,
                target.test_case_type,
                data_row_id.as_deref(),
                status,
                completed_count,
                total_count,
                passed_count,
                failed_count,
                skipped_count,
            )?;
        }

        let final_status = if failed_count > 0 {
            RunStatus::Failed
        } else {
            RunStatus::Passed
        };

        self.finalize_run(
            app,
            run_id,
            final_status,
            passed_count,
            failed_count,
            skipped_count,
        )
        .map_err(map_runner_error)
    }

    async fn execute_api_target(
        &self,
        run_id: &str,
        environment_id: &str,
        target: &ExecutionTarget,
    ) -> AppResult<(RunStatus, Option<String>)> {
        let result = self
            .api_execution_service
            .execute_for_suite_run(
                run_id,
                environment_id,
                &target.test_case_id,
                target.data_row_id.as_deref(),
            )
            .await
            .map_err(map_runner_error)?;

        Ok((
            if result.status == "passed" {
                RunStatus::Passed
            } else {
                RunStatus::Failed
            },
            target.data_row_id.clone(),
        ))
    }

    fn execute_ui_target(
        &self,
        state: &AppState,
        app: &tauri::AppHandle,
        run_id: &str,
        target: &ExecutionTarget,
    ) -> AppResult<(RunStatus, Option<String>)> {
        let replay_result: UiReplayResultDto = self
            .browser_automation_service
            .start_replay_for_suite_run(state, app, run_id, &target.test_case_id)?;

        let status = match replay_result.status {
            ReplayStatus::Passed => RunStatus::Passed,
            ReplayStatus::Cancelled => RunStatus::Cancelled,
            _ => RunStatus::Failed,
        };

        self.runner_repository
            .insert_case_result_if_absent(
                run_id,
                &target.test_case_id,
                None,
                match status {
                    RunStatus::Passed => "passed",
                    RunStatus::Cancelled => "cancelled",
                    RunStatus::Skipped => "skipped",
                    _ => "failed",
                },
                "{}",
                &json!({
                    "replayRunId": replay_result.run_id,
                    "failedStepId": replay_result.failed_step_id,
                })
                .to_string(),
                "[]",
                &json!(replay_result.screenshot_path.into_iter().collect::<Vec<_>>()).to_string(),
                None,
                None,
                0,
            )
            .map_err(map_runner_error)?;

        Ok((status, None))
    }

    fn load_rows_for_case(&self, case: &PersistedSuiteCase) -> Result<Vec<DataTableRow>, TestForgeError> {
        let Some(table_id) = &case.data_table_id else {
            return Ok(Vec::new());
        };

        self.runner_repository.load_enabled_data_rows(table_id)
    }

    fn finalize_failed_run(
        &self,
        app: &tauri::AppHandle,
        run_id: &str,
        passed_count: u32,
        failed_count: u32,
        skipped_count: u32,
    ) -> AppResult<RunResultDto> {
        self.finalize_run(
            app,
            run_id,
            RunStatus::Failed,
            passed_count,
            failed_count,
            skipped_count,
        )
        .map_err(map_runner_error)
    }

    fn finalize_run(
        &self,
        app: &tauri::AppHandle,
        run_id: &str,
        status: RunStatus,
        passed_count: u32,
        failed_count: u32,
        skipped_count: u32,
    ) -> Result<RunResultDto, TestForgeError> {
        let updated = self.runner_repository.update_run_summary_if_active(
            run_id,
            status,
            passed_count,
            failed_count,
            skipped_count,
            Some(&Utc::now().to_rfc3339()),
        )?;

        if !updated {
            return self.runner_repository.load_run_result(run_id);
        }

        let result = self.runner_repository.load_run_result(run_id)?;
        app.emit("runner.execution.completed", result.clone())
            .map_err(|error| TestForgeError::InvalidOperation(format!(
                "Không thể phát runner.execution.completed: {error}"
            )))?;
        Ok(result)
    }

    fn emit_started(
        &self,
        app: &tauri::AppHandle,
        run_id: &str,
        suite_id: &str,
        environment_id: &str,
    ) -> AppResult<()> {
        let payload = RunnerExecutionStartedEvent {
            run_id: run_id.to_string(),
            suite_id: suite_id.to_string(),
            environment_id: environment_id.to_string(),
        };
        app.emit("runner.execution.started", payload)
            .map_err(|error| AppError::internal(format!("Không thể phát runner.execution.started: {error}")))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_progress(
        &self,
        app: &tauri::AppHandle,
        run_id: &str,
        test_case_id: &str,
        test_case_type: TestCaseType,
        data_row_id: Option<&str>,
        status: RunStatus,
        completed_count: u32,
        total_count: u32,
        passed_count: u32,
        failed_count: u32,
        skipped_count: u32,
    ) -> AppResult<()> {
        let payload = RunnerExecutionProgressEvent {
            run_id: run_id.to_string(),
            test_case_id: test_case_id.to_string(),
            test_case_type,
            data_row_id: data_row_id.map(ToOwned::to_owned),
            status,
            completed_count,
            total_count,
            passed_count,
            failed_count,
            skipped_count,
        };
        app.emit("runner.execution.progress", payload)
            .map_err(|error| AppError::internal(format!("Không thể phát runner.execution.progress: {error}")))
    }
}

fn map_runner_error(error: TestForgeError) -> AppError {
    match error {
        TestForgeError::Validation(message) => AppError::validation(message),
        TestForgeError::EndpointNotFound { id } => AppError::not_found("endpoint", id),
        TestForgeError::DataTableNotFound { id } => AppError::not_found("data table", id),
        TestForgeError::DataTableRowNotFound { id } => AppError::not_found("data table row", id),
        TestForgeError::EnvironmentNotFound { id } => AppError::not_found("environment", id),
        TestForgeError::Database(error) => AppError::from(error),
        TestForgeError::Io(error) => AppError::from(error),
        TestForgeError::Serialization(error) => AppError::from(error),
        other => AppError::internal(other.to_string()),
    }
}

#[allow(dead_code)]
fn _keep_dependency_imports(_api_repository: ApiRepository<'_>, _secret_service: &SecretService) {}
