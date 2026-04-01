use serde::{Deserialize, Serialize};

use super::domain::{EntityId, RecordingStatus, ReplayStatus, RunStatus, TestCaseType};
use super::dto::{BrowserHealthDto, RunResultDto, UiStepDto};
use super::errors::ErrorPayload;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorScope {
    Global,
    Command,
    Runner,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorEvent {
    pub scope: AppErrorScope,
    pub error: ErrorPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingStatusChangedEvent {
    pub test_case_id: EntityId,
    pub status: RecordingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingStepCapturedEvent {
    pub test_case_id: EntityId,
    pub step: UiStepDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReplayProgressEvent {
    pub run_id: EntityId,
    pub status: ReplayStatus,
    pub current_step_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerExecutionStartedEvent {
    pub run_id: EntityId,
    pub suite_id: EntityId,
    pub environment_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerExecutionProgressEvent {
    pub run_id: EntityId,
    pub test_case_id: EntityId,
    pub test_case_type: TestCaseType,
    pub data_row_id: Option<EntityId>,
    pub status: RunStatus,
    pub completed_count: u32,
    pub total_count: u32,
    pub passed_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", content = "payload")]
pub enum EventEnvelope {
    #[serde(rename = "app.error")]
    AppError(AppErrorEvent),
    #[serde(rename = "browser.health.changed")]
    BrowserHealthChanged(BrowserHealthDto),
    #[serde(rename = "browser.recording.status.changed")]
    BrowserRecordingStatusChanged(BrowserRecordingStatusChangedEvent),
    #[serde(rename = "browser.recording.step.captured")]
    BrowserRecordingStepCaptured(BrowserRecordingStepCapturedEvent),
    #[serde(rename = "browser.replay.progress")]
    BrowserReplayProgress(BrowserReplayProgressEvent),
    #[serde(rename = "runner.execution.started")]
    RunnerExecutionStarted(RunnerExecutionStartedEvent),
    #[serde(rename = "runner.execution.progress")]
    RunnerExecutionProgress(RunnerExecutionProgressEvent),
    #[serde(rename = "runner.execution.completed")]
    RunnerExecutionCompleted(RunResultDto),
}

#[cfg(test)]
mod tests {
    use super::{AppErrorEvent, AppErrorScope, EventEnvelope};
    use crate::contracts::errors::{ErrorCode, ErrorPayload};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn serialize_app_error_event_with_required_fields() {
        let payload = ErrorPayload {
            code: ErrorCode::SecurityKeyCorrupted,
            display_message: "Không thể truy cập secret store".to_string(),
            technical_message: "key checksum mismatch".to_string(),
            context: BTreeMap::from([(String::from("scope"), json!("startup"))]),
            recoverable: false,
        };

        let event = EventEnvelope::AppError(AppErrorEvent {
            scope: AppErrorScope::Global,
            error: payload,
        });

        let json = serde_json::to_string(&event).expect("failed to serialize app.error event");
        assert!(json.contains("app.error"));
        assert!(json.contains("code"));
        assert!(json.contains("displayMessage"));
        assert!(json.contains("technicalMessage"));
        assert!(json.contains("context"));
        assert!(json.contains("recoverable"));
    }
}
