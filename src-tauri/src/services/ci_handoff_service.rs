use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::contracts::dto::{CiHandoffExitCode, CiHandoffResultDto, RunCaseResultDto, RunDetailDto};
use crate::contracts::domain::RunStatus;
use crate::error::{AppError, AppResult};
use crate::repositories::RunnerRepository;
use crate::services::{ArtifactService, RunnerOrchestrationService};
use crate::state::AppState;

const CI_HANDOFF_SCHEMA_VERSION: &str = "1";
const CI_HANDOFF_CONTRACT_TYPE: &str = "testforge.ci.execution-result";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiHandoffStatus {
    Passed,
    Failed,
    Blocked,
}

impl CiHandoffStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }

    pub fn exit_code(self) -> i32 {
        match self {
            Self::Passed => 0,
            Self::Failed => 1,
            Self::Blocked => 2,
        }
    }

    pub fn to_contract_exit_code(self) -> CiHandoffExitCode {
        match self {
            Self::Passed => CiHandoffExitCode::Passed,
            Self::Failed => CiHandoffExitCode::Failed,
            Self::Blocked => CiHandoffExitCode::Blocked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffRunMetadata {
    pub run_id: String,
    pub suite_id: String,
    pub suite_name: String,
    pub trigger_source: String,
    pub trigger_actor: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffSummary {
    pub total_targets: u32,
    pub passed_targets: u32,
    pub failed_targets: u32,
    pub blocked_targets: u32,
    pub cancelled_targets: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffFailureDetails {
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub step_id: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffFailure {
    pub kind: String,
    pub code: String,
    pub message: String,
    pub details: Option<CiHandoffFailureDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffArtifactReference {
    pub artifact_id: String,
    pub kind: String,
    pub path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffRedactionMetadata {
    pub applied: bool,
    pub policy_version: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiHandoffProjectionInput {
    pub schema_version: String,
    pub contract_type: String,
    pub generated_at: String,
    pub status: CiHandoffStatus,
    pub run: CiHandoffRunMetadata,
    pub summary: CiHandoffSummary,
    pub failure: Option<CiHandoffFailure>,
    pub artifacts: Vec<CiHandoffArtifactReference>,
    pub redaction: CiHandoffRedactionMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiHandoffCounts {
    pub passed: u32,
    pub failed: u32,
    pub blocked: u32,
    pub cancelled: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiHandoffFailureCandidate {
    pub case_id: Option<String>,
    pub case_name: Option<String>,
    pub step_id: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
    pub category: Option<String>,
    pub diagnostic: Option<String>,
}

pub fn build_ci_handoff_contract_json(input: CiHandoffProjectionInput) -> Value {
    json!({
        "schemaVersion": input.schema_version,
        "contractType": input.contract_type,
        "generatedAt": input.generated_at,
        "status": input.status.as_str(),
        "exitCode": input.status.exit_code(),
        "run": {
            "runId": input.run.run_id,
            "suiteId": input.run.suite_id,
            "suiteName": input.run.suite_name,
            "triggerSource": input.run.trigger_source,
            "triggerActor": input.run.trigger_actor,
            "startedAt": input.run.started_at,
            "finishedAt": input.run.finished_at,
            "durationMs": input.run.duration_ms,
        },
        "summary": {
            "totalTargets": input.summary.total_targets,
            "passedTargets": input.summary.passed_targets,
            "failedTargets": input.summary.failed_targets,
            "blockedTargets": input.summary.blocked_targets,
            "cancelledTargets": input.summary.cancelled_targets,
        },
        "failure": input.failure.map(|failure| {
            json!({
                "kind": failure.kind,
                "code": failure.code,
                "message": failure.message,
                "details": failure.details.map(|details| {
                    json!({
                        "targetId": details.target_id,
                        "targetName": details.target_name,
                        "stepId": details.step_id,
                        "diagnostic": details.diagnostic,
                    })
                }),
            })
        }),
        "artifacts": input.artifacts.into_iter().map(|artifact| {
            json!({
                "artifactId": artifact.artifact_id,
                "kind": artifact.kind,
                "path": artifact.path,
                "relativePath": artifact.relative_path,
            })
        }).collect::<Vec<Value>>(),
        "redaction": {
            "applied": input.redaction.applied,
            "policyVersion": input.redaction.policy_version,
            "notes": input.redaction.notes,
        }
    })
}

pub struct CiHandoffService<'a> {
    runner_repository: RunnerRepository<'a>,
    artifact_service: ArtifactService,
}

impl<'a> CiHandoffService<'a> {
    pub fn new(runner_repository: RunnerRepository<'a>, artifact_service: ArtifactService) -> Self {
        Self {
            runner_repository,
            artifact_service,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute_for_suite(
        &self,
        orchestration_service: &RunnerOrchestrationService<'a>,
        state: &AppState,
        app: &tauri::AppHandle,
        suite_id: &str,
        environment_id: &str,
        trigger_source: &str,
        trigger_actor: &str,
        generated_at: &str,
        output_dir: Option<&str>,
        file_name: Option<&str>,
    ) -> AppResult<CiHandoffResultDto> {
        let run_result = orchestration_service
            .execute_suite_for_ci_handoff(state, app, suite_id, environment_id)
            .await?;

        let run_id = run_result.run_id.clone();
        let detail = self
            .runner_repository
            .load_run_detail_for_ci_handoff(&run_id)
            .map_err(|error| AppError::internal(format!("Không thể tải run detail cho CI handoff: {error}")))?;

        let projection = self.build_projection_input(
            &detail,
            trigger_source,
            trigger_actor,
            generated_at,
            output_dir,
            file_name,
            None,
        );
        let payload = build_ci_handoff_contract_json(projection);

        let persisted = self.artifact_service.persist_ci_handoff_contract_json(
            &run_id,
            &payload,
            output_dir,
            file_name,
        )?;

        let db_handle = state.db();
        let db_guard = db_handle
            .lock()
            .map_err(|_| AppError::internal("Database lock poisoned"))?;
        self.artifact_service
            .persist_artifact_manifest(db_guard.connection(), &persisted.manifest)?;

        let status = normalize_status_from_run(detail.summary.summary.status);
        Ok(CiHandoffResultDto {
            run_id,
            suite_id: suite_id.to_string(),
            status: map_service_status_to_contract_status(status),
            exit_code: status.to_contract_exit_code(),
            artifact_path: persisted.file_path,
        })
    }

    pub fn build_projection_input(
        &self,
        detail: &RunDetailDto,
        trigger_source: &str,
        trigger_actor: &str,
        generated_at: &str,
        output_dir: Option<&str>,
        file_name: Option<&str>,
        artifact_override: Option<Vec<CiHandoffArtifactReference>>,
    ) -> CiHandoffProjectionInput {
        let status = normalize_status_from_run(detail.summary.summary.status);
        let suite_id = detail
            .summary
            .summary
            .suite_id
            .clone()
            .unwrap_or_else(|| "unknown-suite".to_string());
        let suite_name = detail
            .summary
            .suite_name
            .clone()
            .unwrap_or_else(|| "Unnamed Suite".to_string());
        let finished_at = detail
            .summary
            .summary
            .finished_at
            .clone()
            .unwrap_or_else(|| generated_at.to_string());
        let duration_ms = estimate_duration_ms(&detail.summary.summary.started_at, &finished_at);

        let summary = &detail.summary.summary;
        let counts = derive_counts(detail);

        let artifacts = artifact_override.unwrap_or_else(|| {
            detail
                .artifacts
                .iter()
                .map(|artifact| CiHandoffArtifactReference {
                    artifact_id: artifact.id.clone(),
                    kind: artifact.artifact_type.clone(),
                    path: artifact.file_path.clone(),
                    relative_path: artifact.relative_path.clone(),
                })
                .collect()
        });

        let (self_path, self_relative_path) = self
            .artifact_service
            .preview_ci_handoff_artifact_reference(
                &summary.run_id,
                output_dir,
                file_name,
            )
            .unwrap_or_else(|_| {
                (
                    format!("{}/exports/ci/ci-execution-{}.json", "app", summary.run_id),
                    format!("exports/ci/ci-execution-{}.json", summary.run_id),
                )
            });

        let mut artifacts = artifacts;
        if !artifacts.iter().any(|artifact| artifact.relative_path == self_relative_path) {
            artifacts.push(CiHandoffArtifactReference {
                artifact_id: format!("artifact-ci-handoff-{}", summary.run_id),
                kind: "report_json".to_string(),
                path: self_path,
                relative_path: self_relative_path,
            });
        }

        CiHandoffProjectionInput {
            schema_version: CI_HANDOFF_SCHEMA_VERSION.to_string(),
            contract_type: CI_HANDOFF_CONTRACT_TYPE.to_string(),
            generated_at: generated_at.to_string(),
            status,
            run: CiHandoffRunMetadata {
                run_id: summary.run_id.clone(),
                suite_id,
                suite_name,
                trigger_source: trigger_source.to_string(),
                trigger_actor: trigger_actor.to_string(),
                started_at: summary.started_at.clone(),
                finished_at,
                duration_ms,
            },
            summary: CiHandoffSummary {
                total_targets: summary.total_count,
                passed_targets: counts.passed,
                failed_targets: counts.failed,
                blocked_targets: counts.blocked,
                cancelled_targets: counts.cancelled,
            },
            failure: build_failure_from_candidate(status, select_failure_candidate(detail, status)),
            artifacts,
            redaction: default_redaction_metadata(),
        }
    }
}

fn normalize_status_from_run(status: RunStatus) -> CiHandoffStatus {
    match status {
        RunStatus::Passed => CiHandoffStatus::Passed,
        RunStatus::Failed => CiHandoffStatus::Failed,
        RunStatus::Cancelled
        | RunStatus::Skipped
        | RunStatus::Queued
        | RunStatus::Running
        | RunStatus::Idle => CiHandoffStatus::Blocked,
    }
}

fn derive_counts(detail: &RunDetailDto) -> CiHandoffCounts {
    let passed = detail
        .results
        .iter()
        .filter(|item| item.status == RunStatus::Passed)
        .count() as u32;
    let failed = detail
        .results
        .iter()
        .filter(|item| item.status == RunStatus::Failed)
        .count() as u32;
    let cancelled = detail
        .results
        .iter()
        .filter(|item| item.status == RunStatus::Cancelled)
        .count() as u32;
    let blocked = detail
        .results
        .iter()
        .filter(|item| item.status == RunStatus::Skipped)
        .count() as u32;

    CiHandoffCounts {
        passed,
        failed,
        blocked,
        cancelled,
    }
}

fn map_service_status_to_contract_status(status: CiHandoffStatus) -> crate::contracts::dto::CiHandoffStatus {
    match status {
        CiHandoffStatus::Passed => crate::contracts::dto::CiHandoffStatus::Passed,
        CiHandoffStatus::Failed => crate::contracts::dto::CiHandoffStatus::Failed,
        CiHandoffStatus::Blocked => crate::contracts::dto::CiHandoffStatus::Blocked,
    }
}

fn select_failure_candidate(
    detail: &RunDetailDto,
    status: CiHandoffStatus,
) -> Option<CiHandoffFailureCandidate> {
    if status == CiHandoffStatus::Passed {
        return None;
    }

    match status {
        CiHandoffStatus::Failed => detail
            .results
            .iter()
            .find(|result| result.status == RunStatus::Failed)
            .map(from_case_result_candidate),
        CiHandoffStatus::Blocked => {
            let blocked_like = detail
                .results
                .iter()
                .find(|result| {
                    result.status == RunStatus::Cancelled
                        || result.status == RunStatus::Skipped
                        || result.failure_category.eq_ignore_ascii_case("preflight")
                })
                .map(from_case_result_candidate);

            blocked_like.or(Some(CiHandoffFailureCandidate {
                case_id: None,
                case_name: None,
                step_id: None,
                message: Some("Suite execution did not complete".to_string()),
                code: Some("RUN_INCOMPLETE".to_string()),
                category: Some("orchestration".to_string()),
                diagnostic: None,
            }))
        }
        CiHandoffStatus::Passed => None,
    }
}

fn from_case_result_candidate(result: &RunCaseResultDto) -> CiHandoffFailureCandidate {
    let diagnostic = if !result.response_preview.trim().is_empty() {
        Some(result.response_preview.clone())
    } else {
        result.error_message.clone()
    };

    CiHandoffFailureCandidate {
        case_id: Some(result.case_id.clone()),
        case_name: Some(result.case_name.clone()),
        step_id: None,
        message: result.error_message.clone(),
        code: result.error_code.clone(),
        category: Some(result.failure_category.clone()),
        diagnostic,
    }
}

fn build_failure_from_candidate(
    status: CiHandoffStatus,
    candidate: Option<CiHandoffFailureCandidate>,
) -> Option<CiHandoffFailure> {
    if status == CiHandoffStatus::Passed {
        return None;
    }

    let candidate = candidate.unwrap_or(CiHandoffFailureCandidate {
        case_id: None,
        case_name: None,
        step_id: None,
        message: None,
        code: None,
        category: None,
        diagnostic: None,
    });

    let category = candidate.category.unwrap_or_else(|| "orchestration".to_string());
    let kind = map_failure_kind(&category, status).to_string();
    let code = sanitize_diagnostic(
        &candidate
            .code
            .unwrap_or_else(|| default_failure_code(status).to_string()),
    );
    let message = sanitize_diagnostic(&candidate.message.unwrap_or_else(|| {
        if status == CiHandoffStatus::Blocked {
            "Suite execution did not complete".to_string()
        } else {
            "One or more assertions failed".to_string()
        }
    }));
    let diagnostic = candidate.diagnostic.map(|value| sanitize_diagnostic(&value));

    Some(CiHandoffFailure {
        kind,
        code,
        message,
        details: Some(CiHandoffFailureDetails {
            target_id: candidate.case_id,
            target_name: candidate.case_name,
            step_id: candidate.step_id,
            diagnostic,
        }),
    })
}

fn default_redaction_metadata() -> CiHandoffRedactionMetadata {
    CiHandoffRedactionMetadata {
        applied: true,
        policy_version: "phase2-default".to_string(),
        notes: vec![
            "Sensitive headers masked".to_string(),
            "Secret-backed variables omitted or redacted".to_string(),
        ],
    }
}

fn map_failure_kind(category: &str, status: CiHandoffStatus) -> &'static str {
    let normalized = category.to_ascii_lowercase();
    if status == CiHandoffStatus::Blocked {
        return "runtime_blocked";
    }

    if normalized.contains("assert") {
        return "assertion";
    }
    if normalized.contains("transport") || normalized.contains("network") {
        return "transport";
    }
    if normalized.contains("preflight") {
        return "preflight";
    }

    "orchestration"
}

fn default_failure_code(status: CiHandoffStatus) -> &'static str {
    match status {
        CiHandoffStatus::Passed => "OK",
        CiHandoffStatus::Failed => "ASSERTION_FAILED",
        CiHandoffStatus::Blocked => "RUNTIME_PREREQUISITE_MISSING",
    }
}

fn estimate_duration_ms(started_at: &str, finished_at: &str) -> u64 {
    let started = chrono::DateTime::parse_from_rfc3339(started_at).ok();
    let finished = chrono::DateTime::parse_from_rfc3339(finished_at).ok();
    match (started, finished) {
        (Some(started), Some(finished)) if finished >= started => {
            (finished - started).num_milliseconds().max(0) as u64
        }
        _ => 0,
    }
}

fn sanitize_diagnostic(raw: &str) -> String {
    let mut sanitized = raw.to_string();
    let patterns = [
        (r"(?i)(authorization\s*:\s*bearer\s+)[^\s;]+", "$1[REDACTED]"),
        (r"(?i)(x-api-key\s*[=:]\s*)[^\s;]+", "$1[REDACTED]"),
        (r"(?i)(api[_-]?key\s*[=:]\s*)[^\s;]+", "$1[REDACTED]"),
        (r"(?i)(token\s*[=:]\s*)[^\s;]+", "$1[REDACTED]"),
        (r"(?i)(secret\s*[=:]\s*)[^\s;]+", "$1[REDACTED]"),
    ];

    for (pattern, replacement) in patterns {
        if let Ok(regex) = Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, replacement).to_string();
        }
    }

    if sanitized.contains("sk_live_") || sanitized.contains("AKIA") {
        sanitized = "[REDACTED]".to_string();
    }

    sanitized
}

pub fn sanitize_ci_handoff_text(raw: &str) -> String {
    sanitize_diagnostic(raw)
}

#[allow(dead_code)]
fn _retain_run_case_result_type(_value: &RunCaseResultDto) {}
