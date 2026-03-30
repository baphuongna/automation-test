use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::domain::{
    AssertionOperator, BrowserRuntimeStatus, EntityId, IsoDateTime, ReplayStatus, RunStatus, StepAction,
    TestCaseType, VariableKind,
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
    pub is_default: bool,
    pub created_at: IsoDateTime,
    pub updated_at: IsoDateTime,
    pub variables: Vec<EnvironmentVariableDto>,
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
pub struct ApiRequestDto {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query_params: BTreeMap<String, String>,
    pub body: Option<String>,
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
    pub started_at: IsoDateTime,
    pub finished_at: Option<IsoDateTime>,
    pub passed_count: u32,
    pub failed_count: u32,
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
