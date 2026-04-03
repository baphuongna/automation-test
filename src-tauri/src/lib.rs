//! TestForge - API and Web UI Testing Tool
//! 
//! This library provides the core functionality for TestForge,
//! including environment management, data tables, and secret encryption.

//!
//! # Architecture
//!
//! The library is organized into the following modules:
//!
//! - `error` - Error types and error handling
//! - `models` - Data models for database entities
//! - `repositories` - Database repository layer
//! - `services` - Business logic services
//! - `db` - Database connection and migrations
//! - `utils` - Utility functions (paths, crypto)
//! - `contracts` - IPC contracts for frontend communication
//! - `state` - Application state management

//!
//! # Quick Start
//!
//! ```rust,no_run
//! use testforge::{
//!     AppState, AppConfig,
//!     db::{Database, MigrationRunner},
//!     services::SecretService,
//!     state::ShellBootstrapSnapshot,
//!     utils::paths::AppPaths,
//! };
//!
//! fn main() {
//!     // Initialize paths
//!     let paths = AppPaths::new("/path/to/app/data".into());
//!     paths.bootstrap().unwrap();
//!     
//!     // Initialize database
//!     let database = Database::new(paths.database_file()).unwrap();
//!     
//!     // Initialize secret service
//!     let secret_service = SecretService::new(paths.base.clone());
//!     secret_service.initialize().unwrap();
//!     
//!     // Create app state
//!     let shell_bootstrap_snapshot = ShellBootstrapSnapshot {
//!         app_version: "0.1.0".to_string(),
//!         is_first_run: true,
//!         degraded_mode: false,
//!         master_key_initialized: true,
//!     };
//!     let app_state = AppState::new(database, secret_service, paths, shell_bootstrap_snapshot);
//!     
//!     // Use app_state...
//! }
//! ```

pub mod models;
pub mod repositories;
pub mod services;
pub mod db;
pub mod utils;
pub mod error;
pub mod contracts;
pub mod state;

use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tauri::{Manager, State};

use contracts::commands::{
    ApiExecuteCommand, BrowserHealthCheckCommand, BrowserRecordingCancelCommand,
    BrowserRecordingStartCommand, BrowserRecordingStopCommand, BrowserReplayCancelCommand,
    BrowserReplayStartCommand, RunnerRunDetailCommand, RunnerRunHistoryCommand,
    RunnerSuiteCancelCommand, RunnerSuiteExecuteCommand,
    DataTableCreateCommand, DataTableExportCommand, DataTableImportCommand, DataTableRowUpsertCommand,
    DataTableUpdateCommand, DeleteByIdCommand, EmptyCommandPayload, EnvironmentCreateCommand,
    EnvironmentUpdateCommand, EnvironmentVariableUpsertCommand, UiTestCaseGetCommand,
};
use contracts::dto::{
    ApiExecutionResultDto, BrowserHealthDto,
    DataTableAssociationMetadataDto, DataTableColumnDto, DataTableDto, DataTableExportDto,
    DataTableImportResultDto, DataTableRowDto, EnvironmentDto, EnvironmentVariableDto,
    RunDetailDto, RunHistoryEntryDto, UiReplayResultDto, UiTestCaseDto,
};
use models::{ColumnDefinition, DataTable, DataTableRow, Environment, EnvironmentType as ModelEnvironmentType, VariableType};
use repositories::{ApiRepository, DataTableRepository, EnvironmentRepository, RunnerRepository, UiScriptRepository};
use services::artifact_service::ArtifactKind;

// Re-export main types for convenience
pub use error::{AppError, AppResult, Result, TestForgeError};
pub use state::{AppState, AppConfig, RecordingState, RunState};
pub use db::{Database, DbConnection, MigrationRunner, MigrationResult};
pub use services::{ApiExecutionService, EnvironmentService, SecretService};
pub use utils::paths::AppPaths;

fn contract_env_type_to_model(value: contracts::domain::EnvironmentType) -> ModelEnvironmentType {
    match value {
        contracts::domain::EnvironmentType::Development => ModelEnvironmentType::Development,
        contracts::domain::EnvironmentType::Staging => ModelEnvironmentType::Staging,
        contracts::domain::EnvironmentType::Production => ModelEnvironmentType::Production,
        contracts::domain::EnvironmentType::Custom => ModelEnvironmentType::Custom,
    }
}

fn model_env_type_to_contract(value: &ModelEnvironmentType) -> contracts::domain::EnvironmentType {
    match value {
        ModelEnvironmentType::Development => contracts::domain::EnvironmentType::Development,
        ModelEnvironmentType::Staging => contracts::domain::EnvironmentType::Staging,
        ModelEnvironmentType::Production => contracts::domain::EnvironmentType::Production,
        ModelEnvironmentType::Custom => contracts::domain::EnvironmentType::Custom,
    }
}

fn contract_variable_kind_to_model(value: contracts::domain::VariableKind) -> VariableType {
    match value {
        contracts::domain::VariableKind::Plain => VariableType::Regular,
        contracts::domain::VariableKind::Secret => VariableType::Secret,
    }
}

fn model_variable_kind_to_contract(value: &VariableType) -> contracts::domain::VariableKind {
    match value {
        VariableType::Regular => contracts::domain::VariableKind::Plain,
        VariableType::Secret => contracts::domain::VariableKind::Secret,
    }
}

fn to_environment_variable_dto(variable: models::EnvironmentVariable) -> EnvironmentVariableDto {
    let value_masked_preview = variable.display_value().to_string();
    EnvironmentVariableDto {
        id: variable.id,
        key: variable.key,
        kind: model_variable_kind_to_contract(&variable.var_type),
        value_masked_preview,
    }
}

fn to_environment_dto(
    environment: Environment,
    variables: Vec<models::EnvironmentVariable>,
) -> EnvironmentDto {
    EnvironmentDto {
        id: environment.id,
        name: environment.name,
        env_type: model_env_type_to_contract(&environment.env_type),
        is_default: environment.is_default,
        created_at: environment.created_at.to_rfc3339(),
        updated_at: environment.updated_at.to_rfc3339(),
        variables: variables.into_iter().map(to_environment_variable_dto).collect(),
    }
}

fn to_data_table_column_dto(column: ColumnDefinition) -> DataTableColumnDto {
    DataTableColumnDto {
        name: column.name,
        col_type: column.col_type,
    }
}

fn to_data_table_row_dto(row: DataTableRow) -> Result<DataTableRowDto> {
    let values = row
        .get_values()
        .map_err(|error| TestForgeError::Validation(format!("Data table row values must be valid JSON array: {error}")))?;

    Ok(DataTableRowDto {
        id: row.id,
        values,
        enabled: row.enabled,
        row_index: row.row_index,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn to_data_table_dto(table: DataTable, rows: Vec<DataTableRow>) -> Result<DataTableDto> {
    let row_dtos = rows
        .into_iter()
        .map(to_data_table_row_dto)
        .collect::<Result<Vec<_>>>()?;
    let enabled_row_count = row_dtos.iter().filter(|row| row.enabled).count();
    let total_row_count = row_dtos.len();

    Ok(DataTableDto {
        id: table.id,
        name: table.name,
        description: table.description,
        columns: table.columns.into_iter().map(to_data_table_column_dto).collect(),
        rows: row_dtos,
        association_meta: DataTableAssociationMetadataDto {
            can_associate_to_test_cases: true,
            linked_test_case_ids: Vec::new(),
            total_row_count,
            enabled_row_count,
        },
        created_at: table.created_at.to_rfc3339(),
        updated_at: table.updated_at.to_rfc3339(),
    })
}

fn to_model_columns(columns: &[DataTableColumnDto]) -> Vec<ColumnDefinition> {
    columns
        .iter()
        .map(|column| ColumnDefinition::with_type(column.name.clone(), column.col_type.clone()))
        .collect()
}

fn with_data_table_repository<T>(state: &AppState, run: impl FnOnce(DataTableRepository<'_>) -> Result<T>) -> Result<T> {
    let db = state.db();
    let db_guard = db
        .lock()
        .map_err(|_| TestForgeError::InvalidOperation("Database lock poisoned".to_string()))?;

    run(DataTableRepository::new(db_guard.connection()))
}

fn load_data_table_dto(repository: &DataTableRepository<'_>, table_id: &str) -> Result<DataTableDto> {
    let table = repository.find_by_id(table_id)?;
    let rows = repository.find_rows_by_table(table_id)?;
    to_data_table_dto(table, rows)
}

fn ensure_valid_import_format(format: &str) -> Result<String> {
    let normalized = format.trim().to_lowercase();
    if normalized == "csv" || normalized == "json" {
        Ok(normalized)
    } else {
        Err(TestForgeError::Validation("Import format must be csv or json".to_string()))
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellMetadataDto {
    pub app_version: String,
    pub is_first_run: bool,
    pub degraded_mode: bool,
    pub master_key_initialized: bool,
    pub browser_runtime: BrowserHealthDto,
}

fn build_shell_metadata(state: &AppState) -> ShellMetadataDto {
    let browser_runtime = services::BrowserAutomationService::new(state.paths().clone()).check_runtime_health();
    let bootstrap = state.shell_bootstrap_snapshot();

    ShellMetadataDto {
        app_version: bootstrap.app_version,
        is_first_run: bootstrap.is_first_run,
        degraded_mode: bootstrap.degraded_mode,
        master_key_initialized: bootstrap.master_key_initialized,
        browser_runtime,
    }
}

fn resolve_shell_app_version(app_handle: &tauri::AppHandle) -> String {
    let version = app_handle.package_info().version.to_string();
    if version.trim().is_empty() {
        "0.0.0".to_string()
    } else {
        version
    }
}

fn build_shell_bootstrap_snapshot(
    app_handle: &tauri::AppHandle,
    bootstrap_state: utils::paths::BootstrapState,
    degraded_mode: bool,
) -> state::ShellBootstrapSnapshot {
    state::ShellBootstrapSnapshot {
        app_version: resolve_shell_app_version(app_handle),
        is_first_run: bootstrap_state.is_first_run(),
        degraded_mode,
        master_key_initialized: !degraded_mode,
    }
}

fn parse_csv_import(content: &str) -> Result<(Vec<ColumnDefinition>, Vec<(Vec<String>, bool)>)> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(TestForgeError::Validation("Malformed CSV import: content is empty".to_string()));
    }

    let lines = trimmed
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return Err(TestForgeError::Validation("Malformed CSV import: header row is required".to_string()));
    }

    let header = lines[0]
        .split(',')
        .map(|item| item.trim().to_string())
        .collect::<Vec<_>>();

    if header.is_empty() || header.iter().any(|item| item.is_empty()) {
        return Err(TestForgeError::Validation("Malformed CSV import: header row contains empty columns".to_string()));
    }

    let columns = header
        .into_iter()
        .map(|name| ColumnDefinition::with_type(name, "string".to_string()))
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for (index, line) in lines.iter().skip(1).enumerate() {
        let values = line
            .split(',')
            .map(|item| item.trim().to_string())
            .collect::<Vec<_>>();

        if values.len() != columns.len() {
            return Err(TestForgeError::Validation(format!(
                "Malformed CSV import: row {} has {} values, expected {}",
                index + 2,
                values.len(),
                columns.len()
            )));
        }

        rows.push((values, true));
    }

    Ok((columns, rows))
}

fn parse_json_import(content: &str) -> Result<(Vec<ColumnDefinition>, Vec<(Vec<String>, bool)>)> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ImportColumn {
        name: String,
        col_type: Option<String>,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ImportRow {
        values: Vec<String>,
        enabled: Option<bool>,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ImportPayload {
        columns: Vec<ImportColumn>,
        rows: Vec<ImportRow>,
    }

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(TestForgeError::Validation("Malformed JSON import: content is empty".to_string()));
    }

    let payload: ImportPayload = serde_json::from_str(trimmed)
        .map_err(|_| TestForgeError::Validation("Malformed JSON import: content is not valid JSON".to_string()))?;

    if payload.columns.is_empty() {
        return Err(TestForgeError::Validation("Malformed JSON import: at least one column is required".to_string()));
    }

    let columns = payload
        .columns
        .into_iter()
        .map(|column| {
            let name = column.name.trim().to_string();
            if name.is_empty() {
                return Err(TestForgeError::Validation("Malformed JSON import: column name cannot be empty".to_string()));
            }

            Ok(ColumnDefinition::with_type(
                name,
                column.col_type.unwrap_or_else(|| "string".to_string()),
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut rows = Vec::new();
    for (index, row) in payload.rows.into_iter().enumerate() {
        if row.values.len() != columns.len() {
            return Err(TestForgeError::Validation(format!(
                "Malformed JSON import: row {} has {} values, expected {}",
                index + 1,
                row.values.len(),
                columns.len()
            )));
        }

        rows.push((row.values, row.enabled.unwrap_or(true)));
    }

    Ok((columns, rows))
}

fn parse_import_payload(format: &str, content: &str) -> Result<(Vec<ColumnDefinition>, Vec<(Vec<String>, bool)>)> {
    match ensure_valid_import_format(format)?.as_str() {
        "csv" => parse_csv_import(content),
        "json" => parse_json_import(content),
        _ => Err(TestForgeError::Validation("Import format must be csv or json".to_string())),
    }
}

fn export_to_csv(table: &DataTableDto) -> String {
    let mut lines = Vec::new();
    lines.push(
        table
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect::<Vec<_>>()
            .join(","),
    );

    for row in &table.rows {
        lines.push(row.values.join(","));
    }

    lines.join("\n")
}

fn export_to_json(table: &DataTableDto) -> Result<String> {
    serde_json::to_string_pretty(table).map_err(TestForgeError::from)
}

fn with_environment_repository<T>(state: &AppState, run: impl FnOnce(EnvironmentRepository<'_>) -> Result<T>) -> Result<T> {
    let db = state.db();
    let db_guard = db
        .lock()
        .map_err(|_| TestForgeError::InvalidOperation("Database lock poisoned".to_string()))?;

    run(EnvironmentRepository::new(db_guard.connection()))
}

fn with_environment_service<T>(state: &AppState, run: impl FnOnce(services::EnvironmentService<'_>) -> Result<T>) -> Result<T> {
    let db = state.db();
    let db_guard = db
        .lock()
        .map_err(|_| TestForgeError::InvalidOperation("Database lock poisoned".to_string()))?;
    let secret_service = state.secret_service();
    let secret_guard = secret_service
        .read()
        .map_err(|_| TestForgeError::InvalidOperation("Secret service lock poisoned".to_string()))?;

    run(services::EnvironmentService::new(
        EnvironmentRepository::new(db_guard.connection()),
        &secret_guard,
    ))
}

fn with_api_execution_service<T>(
    state: &AppState,
    run: impl FnOnce(services::ApiExecutionService<'_>) -> Result<T>,
) -> Result<T> {
    let db = state.db();
    let db_guard = db
        .lock()
        .map_err(|_| TestForgeError::InvalidOperation("Database lock poisoned".to_string()))?;
    let secret_service = state.secret_service();
    let secret_guard = secret_service
        .read()
        .map_err(|_| TestForgeError::InvalidOperation("Secret service lock poisoned".to_string()))?;

    run(services::ApiExecutionService::new(
        ApiRepository::new(db_guard.connection()),
        EnvironmentRepository::new(db_guard.connection()),
        &secret_guard,
    ))
}

fn with_ui_script_repository<T>(state: &AppState, run: impl FnOnce(UiScriptRepository<'_>) -> Result<T>) -> Result<T> {
    let db = state.db();
    let db_guard = db
        .lock()
        .map_err(|_| TestForgeError::InvalidOperation("Database lock poisoned".to_string()))?;

    run(UiScriptRepository::new(db_guard.connection()))
}

fn normalize_variable_id(id: &str) -> Option<&str> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn map_command_error(error: TestForgeError) -> AppError {
    match error {
        TestForgeError::Validation(message) => AppError::validation(message),
        TestForgeError::EnvironmentNotFound { id } => AppError::not_found("environment", id),
        TestForgeError::EndpointNotFound { id } => AppError::not_found("endpoint", id),
        TestForgeError::EnvironmentVariableNotFound { id } => AppError::not_found("environment variable", id),
        TestForgeError::DegradedMode(message) => AppError::secret_key_missing().with_context("reason", message),
        TestForgeError::MasterKeyCorrupted => AppError::secret_key_missing(),
        TestForgeError::SecretStoreUnavailable(message)
        | TestForgeError::MasterKey(message)
        | TestForgeError::SecretEncryption(message)
        | TestForgeError::SecretDecryption(message)
        | TestForgeError::InvalidOperation(message)
        | TestForgeError::Base64Decode(message) => AppError::internal(message),
        TestForgeError::Utf8(error) => AppError::internal(error.to_string()),
        TestForgeError::Database(error) => AppError::from(error),
        TestForgeError::Io(error) => AppError::from(error),
        TestForgeError::Serialization(error) => AppError::from(error),
        TestForgeError::DataTableNotFound { id } => AppError::not_found("data table", id),
        TestForgeError::DataTableRowNotFound { id } => AppError::not_found("data table row", id),
    }
}

fn to_preflight_api_result(error: TestForgeError) -> ApiExecutionResultDto {
    ApiExecutionResultDto {
        status: "failed".to_string(),
        transport_success: false,
        failure_kind: Some("preflight".to_string()),
        error_code: Some("API_REQUEST_BUILD_FAILED".to_string()),
        error_message: Some(error.to_string()),
        status_code: None,
        duration_ms: 0,
        body_preview: String::new(),
        response_headers: std::collections::BTreeMap::new(),
        assertions: Vec::new(),
        request_preview: contracts::dto::ApiRequestPreviewDto {
            method: String::new(),
            url: String::new(),
            headers: std::collections::BTreeMap::new(),
            query_params: std::collections::BTreeMap::new(),
            body_preview: None,
            auth_preview: "none".to_string(),
        },
    }
}

#[tauri::command]
fn environment_list(_payload: EmptyCommandPayload, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<Vec<EnvironmentDto>, AppError> {
    with_environment_repository(state.inner().as_ref(), |repository| {
        let environments = repository.find_all()?;
        let mut dtos = Vec::with_capacity(environments.len());

        for environment in environments {
            let variables = if environment.id.is_empty() {
                Vec::new()
            } else {
                let service_result = with_environment_service(state.inner().as_ref(), |service| {
                    service.list_variables(&environment.id)
                });

                match service_result {
                    Ok(items) => items,
                    Err(TestForgeError::DegradedMode(_)) => repository.find_variables_by_environment(&environment.id)?,
                    Err(error) => return Err(error),
                }
            };

            dtos.push(to_environment_dto(environment, variables));
        }

        Ok(dtos)
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn environment_create(payload: EnvironmentCreateCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<EnvironmentDto, AppError> {
    with_environment_repository(state.inner().as_ref(), |repository| {
        if payload.is_default {
            repository.clear_default()?;
        }

        let mut environment = Environment::new(payload.name);
        environment.env_type = contract_env_type_to_model(payload.env_type);
        environment.is_default = payload.is_default;
        environment.updated_at = Utc::now();

        repository.create(&environment)?;

        let variables = repository.find_variables_by_environment(&environment.id)?;
        Ok(to_environment_dto(environment, variables))
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn environment_update(payload: EnvironmentUpdateCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<EnvironmentDto, AppError> {
    with_environment_repository(state.inner().as_ref(), |repository| {
        if payload.is_default {
            repository.clear_default()?;
        }

        let mut environment = repository.find_by_id(&payload.id)?;
        environment.name = payload.name;
        environment.env_type = contract_env_type_to_model(payload.env_type);
        environment.is_default = payload.is_default;
        environment.updated_at = Utc::now();

        repository.update(&environment)?;

        let variables = repository.find_variables_by_environment(&environment.id)?;
        Ok(to_environment_dto(environment, variables))
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn environment_delete(payload: DeleteByIdCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    with_environment_repository(state.inner().as_ref(), |repository| {
        repository.delete(&payload.id)?;
        Ok(contracts::commands::AckResponse {
            deleted: Some(true),
            started: None,
            cancelled: None,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn environment_variable_upsert(payload: EnvironmentVariableUpsertCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<EnvironmentVariableDto, AppError> {
    with_environment_service(state.inner().as_ref(), |service| {
        let variable = service.upsert_variable(
            &payload.environment_id,
            normalize_variable_id(&payload.variable.id),
            &payload.variable.key,
            contract_variable_kind_to_model(payload.variable.kind),
            &payload.variable.value,
            true,
            None,
        )?;

        Ok(to_environment_variable_dto(variable))
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn environment_variable_delete(payload: DeleteByIdCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    with_environment_service(state.inner().as_ref(), |service| {
        service.delete_variable(&payload.id)?;
        Ok(contracts::commands::AckResponse {
            deleted: Some(true),
            started: None,
            cancelled: None,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn api_testcase_upsert(payload: contracts::dto::ApiTestCaseDto, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<contracts::dto::ApiTestCaseDto, AppError> {
    if !payload.validate_type() {
        return Err(map_command_error(TestForgeError::Validation(
            "api.testcase.upsert requires test case type 'api'".to_string(),
        )));
    }

    with_api_execution_service(state.inner().as_ref(), |service| {
        service.upsert_test_case(&payload.id, &payload.name, &payload.request, &payload.assertions)?;
        Ok(payload)
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn api_testcase_delete(payload: DeleteByIdCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    with_api_execution_service(state.inner().as_ref(), |service| {
        service.delete_test_case(&payload.id)?;
        Ok(contracts::commands::AckResponse {
            deleted: Some(true),
            started: None,
            cancelled: None,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn api_execute(payload: ApiExecuteCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<ApiExecutionResultDto, AppError> {
    let db_handle = state.db();
    let db = db_handle
        .lock()
        .map_err(|_| AppError::internal("Database lock poisoned"))?;
    let secret_handle = state.secret_service();
    let secret_guard = secret_handle
        .read()
        .map_err(|_| AppError::internal("Secret service lock poisoned"))?;

    let service = services::ApiExecutionService::new(
        ApiRepository::new(db.connection()),
        EnvironmentRepository::new(db.connection()),
        &secret_guard,
    );

    match tauri::async_runtime::block_on(service.execute(
        payload.test_case_id.as_deref(),
        &payload.environment_id,
        payload.request,
        payload.assertions,
    )) {
        Ok(result) => Ok(result),
        Err(error @ TestForgeError::Validation(_)) => Ok(to_preflight_api_result(error)),
        Err(error) => Err(map_command_error(error)),
    }
}

#[tauri::command]
fn browser_health_check(
    payload: BrowserHealthCheckCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<BrowserHealthDto, AppError> {
    let _ = payload;
    let service = services::BrowserAutomationService::new(state.paths().clone());
    let health = service.check_runtime_health();
    service.emit_health_changed(&app, &health)?;
    Ok(health)
}

#[tauri::command]
fn shell_metadata_get(
    payload: EmptyCommandPayload,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<ShellMetadataDto, AppError> {
    let _ = payload;
    Ok(build_shell_metadata(state.inner().as_ref()))
}

#[tauri::command]
fn browser_recording_start(
    payload: BrowserRecordingStartCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    let service = services::BrowserAutomationService::new(state.paths().clone());
    service.start_recording(
        state.inner().as_ref(),
        &app,
        &payload.test_case_id,
        &payload.start_url,
    )?;

    Ok(contracts::commands::AckResponse {
        deleted: None,
        started: Some(true),
        cancelled: None,
    })
}

#[tauri::command]
fn browser_recording_stop(
    payload: BrowserRecordingStopCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<UiTestCaseDto, AppError> {
    let service = services::BrowserAutomationService::new(state.paths().clone());
    service.stop_recording(state.inner().as_ref(), &app, &payload.test_case_id)
}

#[tauri::command]
fn browser_recording_cancel(
    payload: BrowserRecordingCancelCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    let service = services::BrowserAutomationService::new(state.paths().clone());
    let cancelled = service.cancel_recording(state.inner().as_ref(), &app, &payload.test_case_id)?;

    Ok(contracts::commands::AckResponse {
        deleted: None,
        started: None,
        cancelled: Some(cancelled),
    })
}

#[tauri::command]
fn browser_replay_start(
    payload: BrowserReplayStartCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<UiReplayResultDto, AppError> {
    let service = services::BrowserAutomationService::new(state.paths().clone());
    service.start_replay(state.inner().as_ref(), &app, &payload.test_case_id)
}

#[tauri::command]
fn browser_replay_cancel(
    payload: BrowserReplayCancelCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    let service = services::BrowserAutomationService::new(state.paths().clone());
    let cancelled = service.cancel_replay(state.inner().as_ref(), &app, &payload.run_id)?;

    Ok(contracts::commands::AckResponse {
        deleted: None,
        started: None,
        cancelled: Some(cancelled),
    })
}

#[tauri::command]
fn runner_suite_execute(
    payload: RunnerSuiteExecuteCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<contracts::commands::RunnerSuiteExecuteResponse, AppError> {
    let db_handle = state.db();
    let db = db_handle
        .lock()
        .map_err(|_| AppError::internal("Database lock poisoned"))?;
    let secret_service = state.secret_service();
    let secret_guard = secret_service
        .read()
        .map_err(|_| AppError::internal("Secret service lock poisoned"))?;

    // runner.execution.started / runner.execution.completed are emitted inside RunnerOrchestrationService.
    let service = services::RunnerOrchestrationService::new(
        RunnerRepository::new(db.connection()),
        services::ApiExecutionService::new(
            ApiRepository::new(db.connection()),
            EnvironmentRepository::new(db.connection()),
            &secret_guard,
        ),
        services::BrowserAutomationService::new(state.paths().clone()),
    );
    tauri::async_runtime::block_on(service.execute_suite(
        state.inner().as_ref(),
        &app,
        &payload.suite_id,
        &payload.environment_id,
        payload.rerun_failed_from_run_id.as_deref(),
    ))
}

#[tauri::command]
fn ui_testcase_upsert(
    payload: UiTestCaseDto,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<UiTestCaseDto, AppError> {
    if !payload.validate_type() {
        return Err(map_command_error(TestForgeError::Validation(
            "ui.testcase.upsert requires test case type 'ui'".to_string(),
        )));
    }

    with_ui_script_repository(state.inner().as_ref(), |repository| {
        repository.upsert_ui_test_case(
            &payload,
            state.config().viewport_width,
            state.config().viewport_height,
            state.config().default_timeout_ms,
        )
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn ui_testcase_delete(
    payload: DeleteByIdCommand,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    with_ui_script_repository(state.inner().as_ref(), |repository| {
        repository.delete_test_case_and_script(&payload.id)?;
        Ok(contracts::commands::AckResponse {
            deleted: Some(true),
            started: None,
            cancelled: None,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn ui_testcase_get(
    payload: UiTestCaseGetCommand,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<UiTestCaseDto, AppError> {
    with_ui_script_repository(state.inner().as_ref(), |repository| {
        repository.find_ui_test_case_by_id(&payload.id)
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn runner_suite_list(
    _payload: EmptyCommandPayload,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<Vec<contracts::dto::SuiteDto>, AppError> {
    let db_handle = state.db();
    let db = db_handle
        .lock()
        .map_err(|_| AppError::internal("Database lock poisoned"))?;
    let repository = RunnerRepository::new(db.connection());
    repository.list_suites().map_err(map_command_error)
}

#[tauri::command]
fn runner_run_history(
    payload: RunnerRunHistoryCommand,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<Vec<RunHistoryEntryDto>, AppError> {
    let db_handle = state.db();
    let db = db_handle
        .lock()
        .map_err(|_| AppError::internal("Database lock poisoned"))?;
    let repository = RunnerRepository::new(db.connection());
    repository
        .list_run_history(payload.suite_id.as_deref())
        .map_err(map_command_error)
}

#[tauri::command]
fn runner_run_detail(
    payload: RunnerRunDetailCommand,
    state: State<'_, std::sync::Arc<AppState>>,
) -> std::result::Result<RunDetailDto, AppError> {
    let db_handle = state.db();
    let db = db_handle
        .lock()
        .map_err(|_| AppError::internal("Database lock poisoned"))?;
    let repository = RunnerRepository::new(db.connection());
    repository.load_run_detail(&payload.run_id).map_err(map_command_error)
}

#[tauri::command]
fn runner_suite_cancel(
    payload: RunnerSuiteCancelCommand,
    state: State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    let db_handle = state.db();
    let db = db_handle
        .lock()
        .map_err(|_| AppError::internal("Database lock poisoned"))?;
    let secret_service = state.secret_service();
    let secret_guard = secret_service
        .read()
        .map_err(|_| AppError::internal("Secret service lock poisoned"))?;

    let service = services::RunnerOrchestrationService::new(
        RunnerRepository::new(db.connection()),
        services::ApiExecutionService::new(
            ApiRepository::new(db.connection()),
            EnvironmentRepository::new(db.connection()),
            &secret_guard,
        ),
        services::BrowserAutomationService::new(state.paths().clone()),
    );
    let cancelled = service.cancel_suite(state.inner().as_ref(), &payload.run_id, &app)?;

    Ok(contracts::commands::AckResponse {
        deleted: None,
        started: None,
        cancelled: Some(cancelled),
    })
}

#[tauri::command]
fn data_table_list(_payload: EmptyCommandPayload, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<Vec<DataTableDto>, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        let tables = repository.find_all()?;
        tables
            .into_iter()
            .map(|table| {
                let rows = repository.find_rows_by_table(&table.id)?;
                to_data_table_dto(table, rows)
            })
            .collect::<Result<Vec<_>>>()
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_create(payload: DataTableCreateCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<DataTableDto, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        let mut table = DataTable::new(payload.name, to_model_columns(&payload.columns));
        table.description = payload.description.filter(|value| !value.trim().is_empty());
        repository.create(&table)?;
        load_data_table_dto(&repository, &table.id)
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_update(payload: DataTableUpdateCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<DataTableDto, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        let mut table = repository.find_by_id(&payload.id)?;
        table.name = payload.name;
        table.description = payload.description.filter(|value| !value.trim().is_empty());
        table.columns = to_model_columns(&payload.columns);
        table.updated_at = Utc::now();
        repository.update(&table)?;
        load_data_table_dto(&repository, &table.id)
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_delete(payload: DeleteByIdCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        repository.delete(&payload.id)?;
        Ok(contracts::commands::AckResponse {
            deleted: Some(true),
            started: None,
            cancelled: None,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_row_upsert(payload: DataTableRowUpsertCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<DataTableRowDto, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        let table = repository.find_by_id(&payload.table_id)?;
        if payload.row.values.len() != table.columns.len() {
            return Err(TestForgeError::Validation("Row value count must match the number of columns".to_string()));
        }

        let normalized_id = normalize_variable_id(&payload.row.id).map(|value| value.to_string());
        if let Some(row_id) = normalized_id {
            let mut row = repository.find_row_by_id(&row_id)?;
            row.set_values(payload.row.values)?;
            row.enabled = payload.row.enabled;
            row.row_index = payload.row.row_index;
            row.updated_at = Utc::now();
            repository.update_row(&row)?;
            to_data_table_row_dto(row)
        } else {
            let mut row = DataTableRow::with_index(payload.table_id, payload.row.values, payload.row.row_index);
            row.enabled = payload.row.enabled;
            repository.create_row(&row)?;
            to_data_table_row_dto(row)
        }
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_row_delete(payload: DeleteByIdCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<contracts::commands::AckResponse, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        repository.delete_row(&payload.id)?;
        Ok(contracts::commands::AckResponse {
            deleted: Some(true),
            started: None,
            cancelled: None,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_import(payload: DataTableImportCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<DataTableImportResultDto, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        let format = ensure_valid_import_format(&payload.format)?;
        let (columns, imported_rows) = parse_import_payload(&format, &payload.content)?;

        let table_id = if let Some(table_id) = payload.table_id.filter(|value| !value.trim().is_empty()) {
            let mut table = repository.find_by_id(&table_id)?;
            table.name = payload.name;
            table.description = payload.description.filter(|value| !value.trim().is_empty());
            table.columns = columns;
            table.updated_at = Utc::now();
            repository.update(&table)?;

            for row in repository.find_rows_by_table(&table.id)? {
                repository.delete_row(&row.id)?;
            }

            table.id
        } else {
            let mut table = DataTable::new(payload.name, columns);
            table.description = payload.description.filter(|value| !value.trim().is_empty());
            repository.create(&table)?;
            table.id
        };

        for (index, (values, enabled)) in imported_rows.iter().enumerate() {
            let mut row = DataTableRow::with_index(table_id.clone(), values.clone(), index as i32);
            row.enabled = *enabled;
            repository.create_row(&row)?;
        }

        let table = load_data_table_dto(&repository, &table_id)?;
        Ok(DataTableImportResultDto {
            table,
            imported_row_count: imported_rows.len(),
            format,
        })
    })
    .map_err(map_command_error)
}

#[tauri::command]
fn data_table_export(payload: DataTableExportCommand, state: State<'_, std::sync::Arc<AppState>>) -> std::result::Result<DataTableExportDto, AppError> {
    with_data_table_repository(state.inner().as_ref(), |repository| {
        let table = load_data_table_dto(&repository, &payload.id)?;
        let format = ensure_valid_import_format(&payload.format)?;
        let content = match format.as_str() {
            "csv" => export_to_csv(&table),
            "json" => export_to_json(&table)?,
            _ => return Err(TestForgeError::Validation("Export format must be csv or json".to_string())),
        };

        let artifact_service = services::ArtifactService::new(state.paths().clone());
        let report_payload = json!({
            "kind": ArtifactKind::Export.as_str(),
            "table": table,
            "format": format,
            "content": content,
            "file_path": state.paths().exports.to_string_lossy().to_string(),
        });
        let report_export = artifact_service
            .persist_report_export(
            &format!("data-table-{}", payload.id),
            "json",
            &report_payload,
        )
        .map_err(|error| TestForgeError::InvalidOperation(error.to_string()))?;

        let db = state.db();
        let db_guard = db
            .lock()
            .map_err(|_| TestForgeError::InvalidOperation("Database lock poisoned".to_string()))?;
        artifact_service
            .persist_artifact_manifest(db_guard.connection(), &report_export.manifest)
            .map_err(|error| TestForgeError::InvalidOperation(error.to_string()))?;

        Ok(DataTableExportDto {
            file_name: format!("{}.{}", table.name.to_lowercase().replace(' ', "-"), format),
            format,
            content,
            table,
        })
    })
    .map_err(map_command_error)
}

/// Output of the backend storage bootstrap.
pub struct BootstrapResult {
    pub app_state: Arc<AppState>,
    pub paths: AppPaths,
}

/// Bootstrap storage and state for the desktop app.
pub fn bootstrap(app_handle: &tauri::AppHandle) -> AppResult<BootstrapResult> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|error| {
        AppError::storage_path(format!("Không xác định được app data directory: {error}"))
    })?;

    let paths = AppPaths::new(app_data_dir);
    let bootstrap_state = paths.inspect_bootstrap_state();
    paths.bootstrap()?;

    let database = Database::new(paths.database_file())?;

    let secret_service = SecretService::new(paths.base.clone());
    let degraded_mode = bootstrap_secret_service(&database, &secret_service, &paths)?;

    let shell_bootstrap_snapshot = build_shell_bootstrap_snapshot(
        app_handle,
        bootstrap_state,
        degraded_mode,
    );

    let app_state = Arc::new(AppState::new(
        database,
        secret_service,
        paths.clone(),
        shell_bootstrap_snapshot,
    ));
    Ok(BootstrapResult { app_state, paths })
}

fn bootstrap_secret_service(
    database: &Database,
    secret_service: &SecretService,
    paths: &AppPaths,
) -> AppResult<bool> {
    let key_exists = paths.master_key_file().exists();
    let has_persisted_secrets = database.has_persisted_secrets()?;

    if !key_exists && has_persisted_secrets {
        secret_service.force_degraded();
        return Ok(true);
    }

    match secret_service.initialize() {
        Ok(_) => Ok(false),
        Err(TestForgeError::MasterKeyCorrupted) => {
            secret_service.force_degraded();
            Ok(true)
        }
        Err(error) => Err(AppError::storage_init(format!(
            "Không thể khởi tạo secret storage: {error}"
        ))),
    }
}

/// Run the Tauri shell.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let result = bootstrap(app.handle())?;
            app.manage(result.app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            environment_list,
            environment_create,
            environment_update,
            environment_delete,
            environment_variable_upsert,
            environment_variable_delete,
            api_testcase_upsert,
            api_testcase_delete,
            api_execute,
            ui_testcase_upsert,
            ui_testcase_get,
            ui_testcase_delete,
            browser_health_check,
            shell_metadata_get,
            browser_recording_start,
            browser_recording_stop,
            browser_recording_cancel,
            browser_replay_start,
            browser_replay_cancel,
            runner_suite_execute,
            runner_suite_list,
            runner_run_history,
            runner_run_detail,
            runner_suite_cancel,
            data_table_list,
            data_table_create,
            data_table_update,
            data_table_delete,
            data_table_row_upsert,
            data_table_row_delete,
            data_table_import,
            data_table_export
        ])
        .run(tauri::generate_context![])
        .expect("Error while running Tauri application");
}
