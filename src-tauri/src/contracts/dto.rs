use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::domain::{
    AssertionOperator, BrowserRuntimeStatus, EntityId, EnvironmentType, IsoDateTime, ReplayStatus,
    RunStatus, StepAction, StepConfidence, TestCaseType, VariableKind,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariableDto {
    pub id: EntityId,
    pub key: String,
    pub kind: VariableKind,
    pub value_masked_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDto {
    pub id: EntityId,
    pub name: String,
    pub env_type: EnvironmentType,
    pub is_default: bool,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
    pub variables: Vec<EnvironmentVariableDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableColumnDto {
    pub name: String,
    pub col_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableRowDto {
    pub id: EntityId,
    pub values: Vec<String>,
    pub enabled: bool,
    pub row_index: i32,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableAssociationMetadataDto {
    pub can_associate_to_test_cases: bool,
    pub linked_test_case_ids: Vec<EntityId>,
    pub total_row_count: usize,
    pub enabled_row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableDto {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<DataTableColumnDto>,
    pub rows: Vec<DataTableRowDto>,
    pub association_meta: DataTableAssociationMetadataDto,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableImportResultDto {
    pub table: DataTableDto,
    pub imported_row_count: usize,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataTableExportDto {
    pub file_name: String,
    pub format: String,
    pub content: String,
    pub table: DataTableDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactManifestDto {
    pub id: EntityId,
    pub artifact_type: String,
    pub logical_name: String,
    pub file_path: String,
    pub relative_path: String,
    pub preview_json: String,
    pub created_at: IsoDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReportExportDto {
    pub file_name: String,
    pub format: String,
    pub file_path: String,
    pub manifest: ArtifactManifestDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiAssertionDto {
    pub id: EntityId,
    pub operator: AssertionOperator,
    pub expected_value: String,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiAuthDto {
    pub r#type: String,
    pub location: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequestDto {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query_params: BTreeMap<String, String>,
    pub body: Option<String>,
    pub auth: Option<ApiAuthDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiAssertionResultDto {
    pub assertion_id: EntityId,
    pub operator: AssertionOperator,
    pub passed: bool,
    pub expected_value: String,
    pub actual_value: Option<String>,
    pub source_path: Option<String>,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequestPreviewDto {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query_params: BTreeMap<String, String>,
    pub body_preview: Option<String>,
    pub auth_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiExecutionResultDto {
    pub status: String,
    pub transport_success: bool,
    pub failure_kind: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub body_preview: String,
    pub response_headers: BTreeMap<String, String>,
    pub assertions: Vec<ApiAssertionResultDto>,
    pub request_preview: ApiRequestPreviewDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiTestCaseDto {
    pub id: EntityId,
    pub r#type: TestCaseType,
    pub name: String,
    pub request: ApiRequestDto,
    pub assertions: Vec<ApiAssertionDto>,
}

impl ApiTestCaseDto {
    pub fn validate_type(&self) -> bool {
        self.r#type == TestCaseType::Api
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiStepDto {
    pub id: EntityId,
    pub action: StepAction,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub timeout_ms: Option<u64>,
    pub confidence: Option<StepConfidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiTestCaseDto {
    pub id: EntityId,
    pub r#type: TestCaseType,
    pub name: String,
    pub start_url: String,
    pub steps: Vec<UiStepDto>,
}

impl UiTestCaseDto {
    pub fn validate_type(&self) -> bool {
        self.r#type == TestCaseType::Ui
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiteItemDto {
    pub id: EntityId,
    pub test_case_id: EntityId,
    pub r#type: TestCaseType,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuiteDto {
    pub id: EntityId,
    pub name: String,
    pub items: Vec<SuiteItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunResultDto {
    pub run_id: EntityId,
    pub status: RunStatus,
    pub suite_id: Option<EntityId>,
    pub environment_id: Option<EntityId>,
    pub started_at: IsoDateTime,
    pub finished_at: Option<IsoDateTime>,
    pub total_count: u32,
    pub passed_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunHistoryEntryDto {
    #[serde(flatten)]
    pub summary: RunResultDto,
    pub suite_name: Option<String>,
    pub environment_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunHistoryFilterDto {
    pub suite_id: Option<EntityId>,
    pub status: Option<RunStatus>,
    pub started_after: Option<IsoDateTime>,
    pub started_before: Option<IsoDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FailureCategoryCountDto {
    pub category: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunHistoryGroupSummaryDto {
    pub total_runs: u32,
    pub passed_runs: u32,
    pub failed_runs: u32,
    pub cancelled_runs: u32,
    pub failure_category_counts: Vec<FailureCategoryCountDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunHistoryDto {
    pub entries: Vec<RunHistoryEntryDto>,
    pub group_summary: RunHistoryGroupSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunCaseResultDto {
    pub id: EntityId,
    pub case_id: EntityId,
    pub case_name: String,
    pub test_case_type: TestCaseType,
    pub data_row_id: Option<EntityId>,
    pub data_row_label: Option<String>,
    pub status: RunStatus,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub failure_category: String,
    pub request_preview: String,
    pub response_preview: String,
    pub assertion_preview: String,
    pub artifacts: Vec<ArtifactManifestDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunDetailDto {
    pub summary: RunHistoryEntryDto,
    pub results: Vec<RunCaseResultDto>,
    pub artifacts: Vec<ArtifactManifestDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiReplayResultDto {
    pub run_id: EntityId,
    pub status: ReplayStatus,
    pub failed_step_id: Option<EntityId>,
    pub screenshot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserHealthDto {
    pub runtime_status: BrowserRuntimeStatus,
    pub message: String,
    pub checked_at: IsoDateTime,
}
