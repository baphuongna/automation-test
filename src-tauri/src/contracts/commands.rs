use serde::{Deserialize, Serialize};

use super::domain::EntityId;
use super::dto::{ApiRequestDto, ApiTestCaseDto, EnvironmentVariableDto, SuiteDto, UiTestCaseDto};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmptyCommandPayload {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCreateCommand {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentUpdateCommand {
    pub id: EntityId,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteByIdCommand {
    pub id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariableUpsertVariable {
    pub id: EntityId,
    pub key: String,
    pub kind: super::domain::VariableKind,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariableUpsertCommand {
    pub environment_id: EntityId,
    pub variable: EnvironmentVariableUpsertVariable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiExecuteCommand {
    pub environment_id: EntityId,
    pub request: ApiRequestDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingStartCommand {
    pub test_case_id: EntityId,
    pub start_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingStopCommand {
    pub test_case_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReplayStartCommand {
    pub test_case_id: EntityId,
    pub environment_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSuiteExecuteCommand {
    pub suite_id: EntityId,
    pub environment_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSuiteCancelCommand {
    pub run_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "command", content = "payload")]
pub enum CommandEnvelope {
    #[serde(rename = "environment.list")]
    EnvironmentList(EmptyCommandPayload),
    #[serde(rename = "environment.create")]
    EnvironmentCreate(EnvironmentCreateCommand),
    #[serde(rename = "environment.update")]
    EnvironmentUpdate(EnvironmentUpdateCommand),
    #[serde(rename = "environment.delete")]
    EnvironmentDelete(DeleteByIdCommand),
    #[serde(rename = "environment.variable.upsert")]
    EnvironmentVariableUpsert(EnvironmentVariableUpsertCommand),
    #[serde(rename = "api.testcase.upsert")]
    ApiTestcaseUpsert(ApiTestCaseDto),
    #[serde(rename = "api.testcase.delete")]
    ApiTestcaseDelete(DeleteByIdCommand),
    #[serde(rename = "api.execute")]
    ApiExecute(ApiExecuteCommand),
    #[serde(rename = "ui.testcase.upsert")]
    UiTestcaseUpsert(UiTestCaseDto),
    #[serde(rename = "ui.testcase.delete")]
    UiTestcaseDelete(DeleteByIdCommand),
    #[serde(rename = "browser.recording.start")]
    BrowserRecordingStart(BrowserRecordingStartCommand),
    #[serde(rename = "browser.recording.stop")]
    BrowserRecordingStop(BrowserRecordingStopCommand),
    #[serde(rename = "browser.replay.start")]
    BrowserReplayStart(BrowserReplayStartCommand),
    #[serde(rename = "runner.suite.execute")]
    RunnerSuiteExecute(RunnerSuiteExecuteCommand),
    #[serde(rename = "runner.suite.cancel")]
    RunnerSuiteCancel(RunnerSuiteCancelCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiExecuteResponse {
    pub status_code: u16,
    pub duration_ms: u64,
    pub body_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSuiteExecuteResponse {
    pub run_id: EntityId,
    pub suite: SuiteDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReplayStartResponse {
    pub run_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AckResponse {
    pub deleted: Option<bool>,
    pub started: Option<bool>,
    pub cancelled: Option<bool>,
}

#[allow(dead_code)]
fn _keep_type_import(_variable: EnvironmentVariableDto) {}

#[cfg(test)]
mod tests {
    use super::{CommandEnvelope, EmptyCommandPayload};

    #[test]
    fn serialize_runner_execute_command() {
        let envelope = CommandEnvelope::RunnerSuiteExecute(super::RunnerSuiteExecuteCommand {
            suite_id: "suite-1".to_string(),
            environment_id: "env-1".to_string(),
        });

        let json = serde_json::to_string(&envelope).expect("failed to serialize command envelope");

        assert!(json.contains("runner.suite.execute"));
        assert!(json.contains("suiteId"));
        assert!(json.contains("environmentId"));
    }

    #[test]
    fn serialize_environment_variable_upsert_command_with_nested_payload() {
        let envelope = CommandEnvelope::EnvironmentVariableUpsert(super::EnvironmentVariableUpsertCommand {
            environment_id: "env-1".to_string(),
            variable: super::EnvironmentVariableUpsertVariable {
                id: "var-1".to_string(),
                key: "API_KEY".to_string(),
                kind: super::super::domain::VariableKind::Secret,
                value: "s3cr3t".to_string(),
            },
        });

        let json = serde_json::to_string(&envelope).expect("failed to serialize upsert command envelope");

        assert!(json.contains("environment.variable.upsert"));
        assert!(json.contains("environmentId"));
        assert!(json.contains("variable"));
        assert!(json.contains("id"));
        assert!(json.contains("kind"));
    }

    #[test]
    fn serialize_environment_list_command_with_explicit_empty_payload() {
        let envelope = CommandEnvelope::EnvironmentList(EmptyCommandPayload {});

        let json = serde_json::to_string(&envelope).expect("failed to serialize environment.list envelope");

        assert!(json.contains("environment.list"));
        assert!(json.contains("payload"));
    }
}
