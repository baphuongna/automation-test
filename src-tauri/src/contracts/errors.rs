use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorFamily {
    Validation,
    Db,
    Api,
    Browser,
    Runner,
    Security,
    Internal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ValidationRequiredField,
    ValidationInvalidFormat,
    ValidationInvalidState,
    DbConnectionFailed,
    DbQueryFailed,
    DbConstraintViolation,
    ApiRequestBuildFailed,
    ApiRequestFailed,
    ApiAssertionFailed,
    BrowserNotAvailable,
    BrowserRecordingFailed,
    BrowserReplayFailed,
    RunnerSuiteEmpty,
    RunnerExecutionFailed,
    RunnerCancelFailed,
    SecuritySecretAccessDenied,
    SecurityKeyMissing,
    SecurityKeyCorrupted,
    InternalUnexpectedError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub display_message: String,
    pub technical_message: String,
    pub context: BTreeMap<String, serde_json::Value>,
    pub recoverable: bool,
}

pub fn error_family_from_code(code: ErrorCode) -> ErrorFamily {
    match code {
        ErrorCode::ValidationRequiredField
        | ErrorCode::ValidationInvalidFormat
        | ErrorCode::ValidationInvalidState => ErrorFamily::Validation,
        ErrorCode::DbConnectionFailed | ErrorCode::DbQueryFailed | ErrorCode::DbConstraintViolation => {
            ErrorFamily::Db
        }
        ErrorCode::ApiRequestBuildFailed | ErrorCode::ApiRequestFailed | ErrorCode::ApiAssertionFailed => {
            ErrorFamily::Api
        }
        ErrorCode::BrowserNotAvailable
        | ErrorCode::BrowserRecordingFailed
        | ErrorCode::BrowserReplayFailed => ErrorFamily::Browser,
        ErrorCode::RunnerSuiteEmpty | ErrorCode::RunnerExecutionFailed | ErrorCode::RunnerCancelFailed => {
            ErrorFamily::Runner
        }
        ErrorCode::SecuritySecretAccessDenied
        | ErrorCode::SecurityKeyMissing
        | ErrorCode::SecurityKeyCorrupted => ErrorFamily::Security,
        ErrorCode::InternalUnexpectedError => ErrorFamily::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::{error_family_from_code, ErrorCode, ErrorFamily};

    #[test]
    fn maps_error_code_to_family() {
        assert_eq!(
            error_family_from_code(ErrorCode::ValidationInvalidState),
            ErrorFamily::Validation
        );
        assert_eq!(error_family_from_code(ErrorCode::DbQueryFailed), ErrorFamily::Db);
        assert_eq!(error_family_from_code(ErrorCode::ApiAssertionFailed), ErrorFamily::Api);
        assert_eq!(
            error_family_from_code(ErrorCode::BrowserReplayFailed),
            ErrorFamily::Browser
        );
        assert_eq!(
            error_family_from_code(ErrorCode::RunnerExecutionFailed),
            ErrorFamily::Runner
        );
        assert_eq!(
            error_family_from_code(ErrorCode::SecurityKeyCorrupted),
            ErrorFamily::Security
        );
        assert_eq!(
            error_family_from_code(ErrorCode::InternalUnexpectedError),
            ErrorFamily::Internal
        );
    }
}
