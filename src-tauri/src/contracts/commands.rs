use serde::{Deserialize, Serialize};

use super::domain::{EntityId, EnvironmentType};
use super::dto::{
    ApiAssertionDto, ApiExecutionResultDto, ApiRequestDto, ApiTestCaseDto, DataTableColumnDto,
    DataTableExportDto, DataTableImportResultDto, DataTableRowDto, EnvironmentVariableDto,
    RunDetailDto, RunHistoryEntryDto, SuiteDto, UiTestCaseDto,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmptyCommandPayload {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCreateCommand {
    pub name: String,
    pub env_type: EnvironmentType,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentUpdateCommand {
    pub id: EntityId,
    pub name: String,
    pub env_type: EnvironmentType,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteByIdCommand {
    pub id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiTestCaseGetCommand {
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
pub struct EnvironmentVariableDeleteCommand {
    pub id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableCreateCommand {
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<DataTableColumnDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableUpdateCommand {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<DataTableColumnDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableRowUpsertCommand {
    pub table_id: EntityId,
    pub row: DataTableRowDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableImportCommand {
    pub table_id: Option<EntityId>,
    pub name: String,
    pub description: Option<String>,
    pub format: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableExportCommand {
    pub id: EntityId,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiExecuteCommand {
    pub test_case_id: Option<EntityId>,
    pub environment_id: EntityId,
    pub request: ApiRequestDto,
    pub assertions: Vec<ApiAssertionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingStartCommand {
    pub test_case_id: EntityId,
    pub start_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserHealthCheckCommand {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingStopCommand {
    pub test_case_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingCancelCommand {
    pub test_case_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserRecordingCancelResponse {
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReplayStartCommand {
    pub test_case_id: EntityId,
    pub environment_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserReplayCancelCommand {
    pub run_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSuiteExecuteCommand {
    pub suite_id: EntityId,
    pub environment_id: EntityId,
    pub rerun_failed_from_run_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSuiteCancelCommand {
    pub run_id: EntityId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerRunHistoryCommand {
    pub suite_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerRunDetailCommand {
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
    #[serde(rename = "environment.variable.delete")]
    EnvironmentVariableDelete(DeleteByIdCommand),
    #[serde(rename = "dataTable.list")]
    DataTableList(EmptyCommandPayload),
    #[serde(rename = "dataTable.create")]
    DataTableCreate(DataTableCreateCommand),
    #[serde(rename = "dataTable.update")]
    DataTableUpdate(DataTableUpdateCommand),
    #[serde(rename = "dataTable.delete")]
    DataTableDelete(DeleteByIdCommand),
    #[serde(rename = "dataTable.row.upsert")]
    DataTableRowUpsert(DataTableRowUpsertCommand),
    #[serde(rename = "dataTable.row.delete")]
    DataTableRowDelete(DeleteByIdCommand),
    #[serde(rename = "dataTable.import")]
    DataTableImport(DataTableImportCommand),
    #[serde(rename = "dataTable.export")]
    DataTableExport(DataTableExportCommand),
    #[serde(rename = "api.testcase.upsert")]
    ApiTestcaseUpsert(ApiTestCaseDto),
    #[serde(rename = "api.testcase.delete")]
    ApiTestcaseDelete(DeleteByIdCommand),
    #[serde(rename = "api.execute")]
    ApiExecute(ApiExecuteCommand),
    #[serde(rename = "ui.testcase.upsert")]
    UiTestcaseUpsert(UiTestCaseDto),
    #[serde(rename = "ui.testcase.get")]
    UiTestcaseGet(UiTestCaseGetCommand),
    #[serde(rename = "ui.testcase.delete")]
    UiTestcaseDelete(DeleteByIdCommand),
    #[serde(rename = "browser.recording.start")]
    BrowserRecordingStart(BrowserRecordingStartCommand),
    #[serde(rename = "browser.health.check")]
    BrowserHealthCheck(BrowserHealthCheckCommand),
    #[serde(rename = "browser.recording.stop")]
    BrowserRecordingStop(BrowserRecordingStopCommand),
    #[serde(rename = "browser.recording.cancel")]
    BrowserRecordingCancel(BrowserRecordingCancelCommand),
    #[serde(rename = "browser.replay.start")]
    BrowserReplayStart(BrowserReplayStartCommand),
    #[serde(rename = "browser.replay.cancel")]
    BrowserReplayCancel(BrowserReplayCancelCommand),
    #[serde(rename = "runner.suite.execute")]
    RunnerSuiteExecute(RunnerSuiteExecuteCommand),
    #[serde(rename = "runner.suite.list")]
    RunnerSuiteList(EmptyCommandPayload),
    #[serde(rename = "runner.run.history")]
    RunnerRunHistory(RunnerRunHistoryCommand),
    #[serde(rename = "runner.run.detail")]
    RunnerRunDetail(RunnerRunDetailCommand),
    #[serde(rename = "runner.suite.cancel")]
    RunnerSuiteCancel(RunnerSuiteCancelCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSuiteExecuteResponse {
    pub run_id: EntityId,
    pub suite: SuiteDto,
}

#[allow(dead_code)]
fn _keep_t16_type_imports(_history: RunHistoryEntryDto, _detail: RunDetailDto) {}

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

#[allow(dead_code)]
fn _keep_t7_type_imports(_result: DataTableImportResultDto, _export: DataTableExportDto) {}

#[allow(dead_code)]
fn _keep_t8_type_imports(_result: ApiExecutionResultDto) {}

#[cfg(test)]
mod tests {
    use super::{CommandEnvelope, EmptyCommandPayload};

    #[test]
    fn serialize_runner_execute_command() {
        let envelope = CommandEnvelope::RunnerSuiteExecute(super::RunnerSuiteExecuteCommand {
            suite_id: "suite-1".to_string(),
            environment_id: "env-1".to_string(),
            rerun_failed_from_run_id: None,
        });

        let json = serde_json::to_string(&envelope).expect("failed to serialize command envelope");

        assert!(json.contains("runner.suite.execute"));
        assert!(json.contains("suiteId"));
        assert!(json.contains("environmentId"));
    }

    #[test]
    fn serialize_environment_variable_upsert_command_with_nested_payload() {
        let envelope =
            CommandEnvelope::EnvironmentVariableUpsert(super::EnvironmentVariableUpsertCommand {
                environment_id: "env-1".to_string(),
                variable: super::EnvironmentVariableUpsertVariable {
                    id: "var-1".to_string(),
                    key: "API_KEY".to_string(),
                    kind: super::super::domain::VariableKind::Secret,
                    value: "s3cr3t".to_string(),
                },
            });

        let json =
            serde_json::to_string(&envelope).expect("failed to serialize upsert command envelope");

        assert!(json.contains("environment.variable.upsert"));
        assert!(json.contains("environmentId"));
        assert!(json.contains("variable"));
        assert!(json.contains("id"));
        assert!(json.contains("kind"));
    }

    #[test]
    fn serialize_environment_list_command_with_explicit_empty_payload() {
        let envelope = CommandEnvelope::EnvironmentList(EmptyCommandPayload {});

        let json = serde_json::to_string(&envelope)
            .expect("failed to serialize environment.list envelope");

        assert!(json.contains("environment.list"));
        assert!(json.contains("payload"));
    }

    #[test]
    fn serialize_data_table_import_command_with_canonical_t7_name() {
        let envelope = CommandEnvelope::DataTableImport(super::DataTableImportCommand {
            table_id: Some("table-1".to_string()),
            name: "Users".to_string(),
            description: Some("Imported".to_string()),
            format: "csv".to_string(),
            content: "username,password\nalice,secret".to_string(),
        });

        let json = serde_json::to_string(&envelope)
            .expect("failed to serialize dataTable.import envelope");

        assert!(json.contains("dataTable.import"));
        assert!(json.contains("tableId"));
        assert!(json.contains("format"));
    }
}
