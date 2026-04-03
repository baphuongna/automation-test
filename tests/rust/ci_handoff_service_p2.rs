/// Contract-first RED tests for P2-T8 CI handoff service.
use serde_json::{json, Value};
use tempfile::TempDir;
use testforge::contracts::domain::RunStatus;
use testforge::contracts::dto::{RunDetailDto, RunHistoryEntryDto, RunResultDto};
use testforge::repositories::RunnerRepository;
use testforge::services::artifact_service::ArtifactService;
use testforge::services::ci_handoff_service::{
    build_ci_handoff_contract_json, sanitize_ci_handoff_text, CiHandoffArtifactReference,
    CiHandoffFailure, CiHandoffFailureDetails, CiHandoffProjectionInput,
    CiHandoffRedactionMetadata, CiHandoffRunMetadata, CiHandoffService, CiHandoffStatus,
    CiHandoffSummary,
};
use testforge::utils::paths::AppPaths;

fn base_run_metadata() -> CiHandoffRunMetadata {
    CiHandoffRunMetadata {
        run_id: "run_123".to_string(),
        suite_id: "suite_456".to_string(),
        suite_name: "Smoke Suite".to_string(),
        trigger_source: "ci".to_string(),
        trigger_actor: "pipeline".to_string(),
        started_at: "2026-04-03T12:33:10.000Z".to_string(),
        finished_at: "2026-04-03T12:34:56.000Z".to_string(),
        duration_ms: 106_000,
    }
}

fn base_artifacts() -> Vec<CiHandoffArtifactReference> {
    vec![CiHandoffArtifactReference {
        artifact_id: "artifact_1".to_string(),
        kind: "report_json".to_string(),
        path: "C:\\exports\\ci\\run_123.json".to_string(),
        relative_path: "exports/ci/run_123.json".to_string(),
    }]
}

fn base_redaction() -> CiHandoffRedactionMetadata {
    CiHandoffRedactionMetadata {
        applied: true,
        policy_version: "phase2-default".to_string(),
        notes: vec![
            "Sensitive headers masked".to_string(),
            "Secret-backed variables omitted or redacted".to_string(),
        ],
    }
}

fn build_minimal_run_detail(run_status: RunStatus) -> RunDetailDto {
    RunDetailDto {
        summary: RunHistoryEntryDto {
            summary: RunResultDto {
                run_id: "run_123".to_string(),
                status: run_status,
                suite_id: Some("suite_456".to_string()),
                environment_id: Some("env_1".to_string()),
                started_at: "2026-04-03T12:33:10.000Z".to_string(),
                finished_at: Some("2026-04-03T12:34:56.000Z".to_string()),
                total_count: 2,
                passed_count: 2,
                failed_count: 0,
                skipped_count: 0,
            },
            suite_name: Some("Smoke Suite".to_string()),
            environment_name: "Development".to_string(),
        },
        results: vec![],
        artifacts: vec![],
    }
}

#[test]
fn passed_contract_is_deterministic_and_maps_to_exit_code_zero() {
    let payload = CiHandoffProjectionInput {
        schema_version: "1".to_string(),
        contract_type: "testforge.ci.execution-result".to_string(),
        generated_at: "2026-04-03T12:34:56.000Z".to_string(),
        status: CiHandoffStatus::Passed,
        run: base_run_metadata(),
        summary: CiHandoffSummary {
            total_targets: 4,
            passed_targets: 4,
            failed_targets: 0,
            blocked_targets: 0,
            cancelled_targets: 0,
        },
        failure: None,
        artifacts: base_artifacts(),
        redaction: base_redaction(),
    };

    let actual = build_ci_handoff_contract_json(payload);

    let expected = json!({
        "schemaVersion": "1",
        "contractType": "testforge.ci.execution-result",
        "generatedAt": "2026-04-03T12:34:56.000Z",
        "status": "passed",
        "exitCode": 0,
        "run": {
            "runId": "run_123",
            "suiteId": "suite_456",
            "suiteName": "Smoke Suite",
            "triggerSource": "ci",
            "triggerActor": "pipeline",
            "startedAt": "2026-04-03T12:33:10.000Z",
            "finishedAt": "2026-04-03T12:34:56.000Z",
            "durationMs": 106000
        },
        "summary": {
            "totalTargets": 4,
            "passedTargets": 4,
            "failedTargets": 0,
            "blockedTargets": 0,
            "cancelledTargets": 0
        },
        "failure": Value::Null,
        "artifacts": [{
            "artifactId": "artifact_1",
            "kind": "report_json",
            "path": "C:\\exports\\ci\\run_123.json",
            "relativePath": "exports/ci/run_123.json"
        }],
        "redaction": {
            "applied": true,
            "policyVersion": "phase2-default",
            "notes": [
                "Sensitive headers masked",
                "Secret-backed variables omitted or redacted"
            ]
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn failed_contract_maps_to_exit_code_one_and_emits_failure_shape() {
    let payload = CiHandoffProjectionInput {
        schema_version: "1".to_string(),
        contract_type: "testforge.ci.execution-result".to_string(),
        generated_at: "2026-04-03T12:34:56.000Z".to_string(),
        status: CiHandoffStatus::Failed,
        run: base_run_metadata(),
        summary: CiHandoffSummary {
            total_targets: 4,
            passed_targets: 3,
            failed_targets: 1,
            blocked_targets: 0,
            cancelled_targets: 0,
        },
        failure: Some(CiHandoffFailure {
            kind: "assertion".to_string(),
            code: "ASSERTION_FAILED".to_string(),
            message: "One or more assertions failed".to_string(),
            details: Some(CiHandoffFailureDetails {
                target_id: Some("target_checkout".to_string()),
                target_name: Some("Checkout API".to_string()),
                step_id: Some("step_03".to_string()),
                diagnostic: Some("Expected status 200 but got 500".to_string()),
            }),
        }),
        artifacts: base_artifacts(),
        redaction: base_redaction(),
    };

    let actual = build_ci_handoff_contract_json(payload);

    assert_eq!(actual["schemaVersion"], "1");
    assert_eq!(actual["contractType"], "testforge.ci.execution-result");
    assert_eq!(actual["status"], "failed");
    assert_eq!(actual["exitCode"], 1);
    assert_eq!(actual["run"]["runId"], "run_123");
    assert_eq!(actual["run"]["suiteId"], "suite_456");
    assert_eq!(actual["summary"]["totalTargets"], 4);
    assert_eq!(actual["summary"]["failedTargets"], 1);

    assert_eq!(actual["failure"]["kind"], "assertion");
    assert_eq!(actual["failure"]["code"], "ASSERTION_FAILED");
    assert_eq!(
        actual["failure"]["message"],
        "One or more assertions failed"
    );
    assert_eq!(actual["failure"]["details"]["targetId"], "target_checkout");
    assert_eq!(actual["failure"]["details"]["targetName"], "Checkout API");
    assert_eq!(actual["failure"]["details"]["stepId"], "step_03");
    assert_eq!(
        actual["failure"]["details"]["diagnostic"],
        "Expected status 200 but got 500"
    );

    assert_eq!(actual["artifacts"][0]["artifactId"], "artifact_1");
    assert_eq!(actual["artifacts"][0]["kind"], "report_json");
    assert_eq!(
        actual["artifacts"][0]["path"],
        "C:\\exports\\ci\\run_123.json"
    );
    assert_eq!(
        actual["artifacts"][0]["relativePath"],
        "exports/ci/run_123.json"
    );
    assert_eq!(actual["redaction"]["applied"], true);
    assert_eq!(actual["redaction"]["policyVersion"], "phase2-default");
}

#[test]
fn blocked_contract_maps_to_exit_code_two_and_preserves_blocked_summary() {
    let payload = CiHandoffProjectionInput {
        schema_version: "1".to_string(),
        contract_type: "testforge.ci.execution-result".to_string(),
        generated_at: "2026-04-03T12:34:56.000Z".to_string(),
        status: CiHandoffStatus::Blocked,
        run: base_run_metadata(),
        summary: CiHandoffSummary {
            total_targets: 4,
            passed_targets: 0,
            failed_targets: 0,
            blocked_targets: 4,
            cancelled_targets: 0,
        },
        failure: Some(CiHandoffFailure {
            kind: "runtime_blocked".to_string(),
            code: "RUNTIME_PREREQUISITE_MISSING".to_string(),
            message: "Suite execution blocked by runtime prerequisite".to_string(),
            details: Some(CiHandoffFailureDetails {
                target_id: None,
                target_name: None,
                step_id: None,
                diagnostic: Some("Browser runtime unavailable".to_string()),
            }),
        }),
        artifacts: base_artifacts(),
        redaction: base_redaction(),
    };

    let actual = build_ci_handoff_contract_json(payload);

    assert_eq!(actual["status"], "blocked");
    assert_eq!(actual["exitCode"], 2);
    assert_eq!(actual["summary"]["blockedTargets"], 4);
    assert_eq!(actual["failure"]["kind"], "runtime_blocked");
    assert_eq!(actual["failure"]["code"], "RUNTIME_PREREQUISITE_MISSING");
}

#[test]
fn diagnostics_are_sanitized_and_never_emit_raw_secret_material() {
    let sanitized_diagnostic =
        sanitize_ci_handoff_text("Authorization: Bearer sk_live_123; x-api-key=AKIA123");
    let sanitized_message =
        sanitize_ci_handoff_text("Transport failed: Authorization: Bearer sk_live_123");

    assert!(sanitized_diagnostic.contains("[REDACTED]"));
    assert!(!sanitized_diagnostic.contains("Bearer sk_live_"));
    assert!(!sanitized_diagnostic.contains("AKIA"));
    assert!(!sanitized_message.contains("sk_live_"));
    assert!(sanitized_message.contains("[REDACTED]"));
}

#[test]
fn blocked_failure_without_target_source_keeps_details_nullable() {
    let payload = CiHandoffProjectionInput {
        schema_version: "1".to_string(),
        contract_type: "testforge.ci.execution-result".to_string(),
        generated_at: "2026-04-03T12:34:56.000Z".to_string(),
        status: CiHandoffStatus::Blocked,
        run: base_run_metadata(),
        summary: CiHandoffSummary {
            total_targets: 2,
            passed_targets: 0,
            failed_targets: 0,
            blocked_targets: 0,
            cancelled_targets: 2,
        },
        failure: Some(CiHandoffFailure {
            kind: "orchestration".to_string(),
            code: "RUN_INCOMPLETE".to_string(),
            message: "Suite execution did not complete".to_string(),
            details: Some(CiHandoffFailureDetails {
                target_id: None,
                target_name: None,
                step_id: None,
                diagnostic: None,
            }),
        }),
        artifacts: base_artifacts(),
        redaction: base_redaction(),
    };

    let actual = build_ci_handoff_contract_json(payload);
    assert_eq!(actual["status"], "blocked");
    assert_eq!(actual["summary"]["blockedTargets"], 0);
    assert_eq!(actual["summary"]["cancelledTargets"], 2);
    assert_eq!(actual["failure"]["details"]["targetId"], Value::Null);
    assert_eq!(actual["failure"]["details"]["targetName"], Value::Null);
}

#[test]
fn preview_reference_matches_persisted_target_when_overrides_supplied() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let paths = AppPaths::new(temp_dir.path().join("app-data"));
    paths.bootstrap().expect("failed to bootstrap app paths");
    let artifact_service = ArtifactService::new(paths);

    let output_dir = Some("exports/review-overrides");
    let file_name = Some("Result.Final.JSON");
    let (preview_path, preview_relative_path) = artifact_service
        .preview_ci_handoff_artifact_reference("run_123", output_dir, file_name)
        .expect("failed to preview CI handoff artifact path");

    let persisted = artifact_service
        .persist_ci_handoff_contract_json("run_123", &json!({ "ok": true }), output_dir, file_name)
        .expect("failed to persist CI handoff artifact");

    assert_eq!(preview_path, persisted.file_path);
    assert_eq!(preview_relative_path, persisted.relative_path);
    assert!(persisted
        .relative_path
        .ends_with("exports/review-overrides/result.final.json"));
}

#[test]
fn projection_self_reference_uses_override_output_target() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let paths = AppPaths::new(temp_dir.path().join("app-data"));
    paths.bootstrap().expect("failed to bootstrap app paths");
    let artifact_service = ArtifactService::new(paths);
    let connection =
        rusqlite::Connection::open_in_memory().expect("failed to open sqlite memory db");
    let runner_repository = RunnerRepository::new(&connection);
    let service = CiHandoffService::new(runner_repository, artifact_service);

    let detail = build_minimal_run_detail(RunStatus::Passed);
    let projection = service.build_projection_input(
        &detail,
        "ci",
        "pipeline",
        "2026-04-03T12:34:56.000Z",
        Some("exports/custom-ci"),
        Some("custom-output.json"),
        Some(vec![]),
    );

    assert_eq!(projection.artifacts.len(), 1);
    assert_eq!(
        projection.artifacts[0].relative_path,
        "exports/custom-ci/custom-output.json"
    );
    assert!(projection.artifacts[0]
        .path
        .replace('\\', "/")
        .ends_with("/exports/custom-ci/custom-output.json"));
}
