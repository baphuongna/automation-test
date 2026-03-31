//! Error types for TestForge
//!
//! This module defines the error hierarchy used throughout the application.
//! Provides structured error payloads with display/technical messages for IPC.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error code enum - for IPC communication and error classification
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ErrorCode {
    // Database errors
    DbConnection,
    DbMigration,
    DbQuery,
    DbConstraint,

    // Storage errors
    StorageInit,
    StoragePath,
    StorageWrite,
    StorageRead,

    // Validation errors
    Validation,
    InvalidInput,
    MissingField,
    DuplicateEntry,

    // Not found errors
    NotFound,
    EnvironmentNotFound,
    EndpointNotFound,
    ScriptNotFound,
    SuiteNotFound,

    // Variable resolution errors
    VariableMissing,
    VariableCircular,

    // Secret/encryption errors
    SecretEncryption,
    SecretDecryption,
    SecretKeyMissing,
    SecretCorrupt,

    // Browser automation errors
    BrowserLaunch,
    BrowserRuntime,
    ElementNotFound,
    SelectorInvalid,
    StepExecution,

    // API execution errors
    ApiTransport,
    ApiTimeout,
    ApiSsl,

    // State/Concurrency errors
    StateConflict,
    RecordingInProgress,
    RunInProgress,

    // Internal errors
    Internal,
    Unknown,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::DbConnection => "DB_CONNECTION",
            Self::DbMigration => "DB_MIGRATION",
            Self::DbQuery => "DB_QUERY",
            Self::DbConstraint => "DB_CONSTRAINT",
            Self::StorageInit => "STORAGE_INIT",
            Self::StoragePath => "STORAGE_PATH",
            Self::StorageWrite => "STORAGE_WRITE",
            Self::StorageRead => "STORAGE_READ",
            Self::Validation => "VALIDATION",
            Self::InvalidInput => "INVALID_INPUT",
            Self::MissingField => "MISSING_FIELD",
            Self::DuplicateEntry => "DUPLICATE_ENTRY",
            Self::NotFound => "NOT_FOUND",
            Self::EnvironmentNotFound => "ENVIRONMENT_NOT_FOUND",
            Self::EndpointNotFound => "ENDPOINT_NOT_FOUND",
            Self::ScriptNotFound => "SCRIPT_NOT_FOUND",
            Self::SuiteNotFound => "SUITE_NOT_FOUND",
            Self::VariableMissing => "VARIABLE_MISSING",
            Self::VariableCircular => "VARIABLE_CIRCULAR",
            Self::SecretEncryption => "SECRET_ENCRYPTION",
            Self::SecretDecryption => "SECRET_DECRYPTION",
            Self::SecretKeyMissing => "SECRET_KEY_MISSING",
            Self::SecretCorrupt => "SECRET_CORRUPT",
            Self::BrowserLaunch => "BROWSER_LAUNCH",
            Self::BrowserRuntime => "BROWSER_RUNTIME",
            Self::ElementNotFound => "ELEMENT_NOT_FOUND",
            Self::SelectorInvalid => "SELECTOR_INVALID",
            Self::StepExecution => "STEP_EXECUTION",
            Self::ApiTransport => "API_TRANSPORT",
            Self::ApiTimeout => "API_TIMEOUT",
            Self::ApiSsl => "API_SSL",
            Self::StateConflict => "STATE_CONFLICT",
            Self::RecordingInProgress => "RECORDING_IN_PROGRESS",
            Self::RunInProgress => "RUN_IN_PROGRESS",
            Self::Internal => "INTERNAL",
            Self::Unknown => "UNKNOWN",
        };
        write!(f, "{}", s)
    }
}

/// Context information for error debugging
pub type ErrorContext = HashMap<String, serde_json::Value>;

/// Structured error payload for IPC communication
/// Matches spec section 6 error model
#[derive(Debug, Clone)]
pub struct AppError {
    /// Error code để phân loại
    pub code: ErrorCode,
    /// Message thân thiện với user (hiển thị trên UI)
    pub display_message: String,
    /// Message kỹ thuật (cho log/debug)
    pub technical_message: String,
    /// Context bổ sung cho debugging
    pub context: ErrorContext,
    /// Có thể recover được hay không
    pub recoverable: bool,
}

impl AppError {
    /// Tạo error mới với code và messages
    pub fn new(
        code: ErrorCode,
        display_message: impl Into<String>,
        technical_message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            display_message: display_message.into(),
            technical_message: technical_message.into(),
            context: ErrorContext::new(),
            recoverable: true,
        }
    }

    /// Thêm context vào error
    pub fn with_context(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.context.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Set recoverable flag
    pub fn with_recoverable(mut self, recoverable: bool) -> Self {
        self.recoverable = recoverable;
        self
    }

    // === Factory methods for common errors ===

    /// DB connection error
    pub fn db_connection(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::DbConnection,
            "Không thể kết nối đến cơ sở dữ liệu. Vui lòng khởi động lại ứng dụng.",
            msg,
        )
        .with_recoverable(false)
    }

    /// DB migration error
    pub fn db_migration(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::DbMigration,
            "Lỗi khi cập nhật cơ sở dữ liệu. Vui lòng liên hệ hỗ trợ.",
            msg,
        )
        .with_recoverable(false)
    }

    /// DB query error
    pub fn db_query(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::DbQuery, "Lỗi truy vấn cơ sở dữ liệu.", msg)
    }

    /// DB constraint error
    pub fn db_constraint(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::DbConstraint,
            "Dữ liệu bị trùng hoặc vi phạm ràng buộc.",
            msg,
        )
    }

    /// Storage init error
    pub fn storage_init(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::StorageInit,
            "Không thể khởi tạo thư mục dữ liệu. Vui lòng kiểm tra quyền truy cập.",
            msg,
        )
        .with_recoverable(false)
    }

    /// Storage path error
    pub fn storage_path(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::StoragePath,
            "Đường dẫn không hợp lệ. Vui lòng kiểm tra cấu hình.",
            msg,
        )
        .with_recoverable(false)
    }

    /// Storage read error
    pub fn storage_read(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::StorageRead,
            "Không thể đọc dữ liệu từ bộ nhớ lưu trữ.",
            msg,
        )
    }

    /// Storage write error
    pub fn storage_write(msg: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::StorageWrite,
            "Không thể ghi dữ liệu vào bộ nhớ lưu trữ.",
            msg,
        )
    }

    /// Validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Validation, "Dữ liệu không hợp lệ.", msg)
    }

    /// Not found error
    pub fn not_found(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::NotFound,
            format!("Không tìm thấy {}.", entity_type.into()),
            format!("{} not found with id: {}", entity_type.into(), id.into()),
        )
    }

    /// Variable missing error
    pub fn variable_missing(name: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::VariableMissing,
            format!("Biến '{}' chưa được định nghĩa.", name.into()),
            format!("Variable '{}' is not defined", name.into()),
        )
        .with_context("variable_name", name)
    }

    /// Secret key missing error
    pub fn secret_key_missing() -> Self {
        Self::new(
            ErrorCode::SecretKeyMissing,
            "Khóa mã hóa không khả dụng. Vui lòng cấu hình lại master key.",
            "Master key is not available",
        )
        .with_recoverable(false)
    }

    /// Recording already in progress error
    pub fn recording_in_progress() -> Self {
        Self::new(
            ErrorCode::RecordingInProgress,
            "Đang có một phiên ghi UI khác hoạt động.",
            "Another recording session is already in progress",
        )
    }

    /// Test run already in progress error
    pub fn run_in_progress() -> Self {
        Self::new(
            ErrorCode::RunInProgress,
            "Đang có một phiên chạy test khác hoạt động.",
            "Another test run is already in progress",
        )
    }

    /// Internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, "Đã xảy ra lỗi nội bộ.", msg)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.technical_message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 5)?;
        state.serialize_field("code", &self.code.to_string())?;
        state.serialize_field("displayMessage", &self.display_message)?;
        state.serialize_field("technicalMessage", &self.technical_message)?;
        state.serialize_field("context", &self.context)?;
        state.serialize_field("recoverable", &self.recoverable)?;
        state.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(e, msg) => {
                let tech_msg = format!("SQLite error: {:?} - {:?}", e, msg);
                match e.code {
                    rusqlite::ErrorCode::NotFound => Self::not_found("record", "unknown"),
                    rusqlite::ErrorCode::ConstraintViolation => Self::db_constraint(tech_msg),
                    _ => Self::db_query(tech_msg),
                }
            }
            _ => Self::db_connection(err.to_string()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::storage_init(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            "Dữ liệu JSON không hợp lệ.",
            err.to_string(),
        )
    }
}

/// Result type alias cho AppError
pub type AppResult<T> = std::result::Result<T, AppError>;

// === Legacy TestForgeError for backward compatibility ===

/// Main result type for TestForge operations
pub type Result<T> = std::result::Result<T, TestForgeError>;

/// Main error type for TestForge
#[derive(Error, Debug)]
pub enum TestForgeError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Secret encryption error: {0}")]
    SecretEncryption(String),

    #[error("Secret decryption error: {0}")]
    SecretDecryption(String),

    #[error("Master key error: {0}")]
    MasterKey(String),

    #[error("Master key file corrupted or missing - degraded mode active")]
    MasterKeyCorrupted,

    #[error("Secret store unavailable: {0}")]
    SecretStoreUnavailable(String),

    #[error("Environment not found: {id}")]
    EnvironmentNotFound { id: String },

    #[error("Endpoint not found: {id}")]
    EndpointNotFound { id: String },

    #[error("Environment variable not found: {id}")]
    EnvironmentVariableNotFound { id: String },

    #[error("Data table not found: {id}")]
    DataTableNotFound { id: String },

    #[error("Data table row not found: {id}")]
    DataTableRowNotFound { id: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Operation blocked in degraded mode: {0}")]
    DegradedMode(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Base64 decode error: {0}")]
    Base64Decode(String),
}

impl TestForgeError {
    /// Check if this error indicates degraded mode
    pub fn is_degraded_mode(&self) -> bool {
        matches!(
            self,
            Self::MasterKeyCorrupted | Self::SecretStoreUnavailable(_)
        )
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::EnvironmentNotFound { .. }
                | Self::EndpointNotFound { .. }
                | Self::EnvironmentVariableNotFound { .. }
                | Self::DataTableNotFound { .. }
                | Self::DataTableRowNotFound { .. }
                | Self::Validation(_)
        )
    }

    /// Get error code for IPC communication
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Database(_) => "DATABASE_ERROR",
            Self::Io(_) => "IO_ERROR",
            Self::Serialization(_) => "SERIALIZATION_ERROR",
            Self::SecretEncryption(_) => "SECRET_ENCRYPTION_ERROR",
            Self::SecretDecryption(_) => "SECRET_DECRYPTION_ERROR",
            Self::MasterKey(_) => "MASTER_KEY_ERROR",
            Self::MasterKeyCorrupted => "MASTER_KEY_CORRUPTED",
            Self::SecretStoreUnavailable(_) => "SECRET_STORE_UNAVAILABLE",
            Self::EnvironmentNotFound { .. } => "ENVIRONMENT_NOT_FOUND",
            Self::EndpointNotFound { .. } => "ENDPOINT_NOT_FOUND",
            Self::EnvironmentVariableNotFound { .. } => "ENVIRONMENT_VARIABLE_NOT_FOUND",
            Self::DataTableNotFound { .. } => "DATA_TABLE_NOT_FOUND",
            Self::DataTableRowNotFound { .. } => "DATA_TABLE_ROW_NOT_FOUND",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::DegradedMode(_) => "DEGRADED_MODE",
            Self::InvalidOperation(_) => "INVALID_OPERATION",
            Self::Utf8(_) => "UTF8_ERROR",
            Self::Base64Decode(_) => "BASE64_DECODE_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        let err = TestForgeError::EnvironmentNotFound {
            id: "test".to_string(),
        };
        assert_eq!(err.error_code(), "ENVIRONMENT_NOT_FOUND");
    }

    #[test]
    fn test_is_recoverable() {
        let err = TestForgeError::EnvironmentNotFound {
            id: "test".to_string(),
        };
        assert!(err.is_recoverable());

        let err = TestForgeError::Database(rusqlite::Error::InvalidQuery);
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_is_degraded_mode() {
        let err = TestForgeError::MasterKeyCorrupted;
        assert!(err.is_degraded_mode());

        let err = TestForgeError::Validation("test".to_string());
        assert!(!err.is_degraded_mode());
    }
}
