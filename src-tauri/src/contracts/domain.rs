use serde::{Deserialize, Serialize};

pub type EntityId = String;
pub type IsoDateTime = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentType {
    Development,
    Staging,
    Production,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestCaseType {
    Api,
    Ui,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Idle,
    Queued,
    Running,
    Passed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserRuntimeStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    Idle,
    Recording,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    Idle,
    Running,
    Passed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VariableKind {
    Plain,
    Secret,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionOperator {
    StatusEquals,
    JsonPathExists,
    JsonPathEquals,
    BodyContains,
    HeaderEquals,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    Navigate,
    Click,
    Fill,
    Select,
    Check,
    Uncheck,
    WaitFor,
    AssertText,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepConfidence {
    High,
    Medium,
    Low,
}

#[cfg(test)]
mod tests {
    use super::{AssertionOperator, StepAction};

    #[test]
    fn serializes_assertion_operator_with_canonical_contract_name() {
        let value = serde_json::to_value(AssertionOperator::StatusEquals)
            .expect("failed to serialize assertion operator");

        assert_eq!(
            value,
            serde_json::Value::String("status_equals".to_string())
        );
    }

    #[test]
    fn serializes_step_action_with_canonical_contract_name() {
        let value =
            serde_json::to_value(StepAction::WaitFor).expect("failed to serialize step action");

        assert_eq!(value, serde_json::Value::String("wait_for".to_string()));
    }
}
