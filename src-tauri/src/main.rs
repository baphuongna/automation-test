//! TestForge application entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use testforge::utils::paths::AppPaths;
use testforge::{
    api_execute, api_testcase_delete, api_testcase_upsert, browser_health_check,
    browser_recording_cancel, browser_recording_start, browser_recording_stop,
    browser_replay_cancel, browser_replay_start, data_table_create, data_table_delete,
    data_table_export, data_table_import, data_table_list, data_table_row_delete,
    data_table_row_upsert, data_table_update,
    db::Database,
    environment_create, environment_delete, environment_list, environment_update,
    environment_variable_delete, environment_variable_upsert,
    error::{AppError, AppResult, TestForgeError},
    runner_run_detail, runner_run_history, runner_suite_cancel, runner_suite_execute,
    runner_suite_list,
    services::SecretService,
    shell_metadata_get,
    state::ShellBootstrapSnapshot,
    AppState,
};

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
    let is_first_run = paths.detect_first_run();
    paths.bootstrap()?;

    let database = Database::new(paths.database_file())?;

    let secret_service = SecretService::new(paths.base.clone());
    let degraded_mode = bootstrap_secret_service(&database, &secret_service, &paths)?;

    let shell_bootstrap_snapshot = ShellBootstrapSnapshot {
        app_version: app_handle.package_info().version.to_string(),
        is_first_run,
        degraded_mode,
        master_key_initialized: !degraded_mode,
    };

    let app_state = Arc::new(AppState::new(
        database,
        secret_service,
        paths.clone(),
        shell_bootstrap_snapshot,
    ));
    app_state.set_degraded_mode(degraded_mode);
    app_state.set_master_key_initialized(!degraded_mode);

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

fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use testforge::models::{Environment, VariableType};
    use testforge::repositories::EnvironmentRepository;
    use testforge::services::EnvironmentService;

    #[test]
    fn bootstrap_enters_degraded_mode_when_key_is_missing_but_secret_rows_exist() {
        let temp_dir = TempDir::new().unwrap();
        let paths = AppPaths::new(temp_dir.path().join("app-data"));
        paths.bootstrap().unwrap();

        let database = Database::new(paths.database_file()).unwrap();
        let secret_service = SecretService::new(paths.base.clone());
        secret_service.initialize().unwrap();

        let repo = EnvironmentRepository::new(database.connection());
        let environment = Environment::new("Production".to_string());
        let environment_id = environment.id.clone();
        repo.create(&environment).unwrap();

        let service = EnvironmentService::new(
            EnvironmentRepository::new(database.connection()),
            &secret_service,
        );
        service
            .upsert_variable(
                &environment_id,
                None,
                "API_KEY",
                VariableType::Secret,
                "persisted-secret",
                true,
                None,
            )
            .unwrap();

        fs::remove_file(paths.master_key_file()).unwrap();

        let reopened_db = Database::new(paths.database_file()).unwrap();
        let reopened_secret_service = SecretService::new(paths.base.clone());

        let degraded_mode =
            bootstrap_secret_service(&reopened_db, &reopened_secret_service, &paths).unwrap();

        assert!(degraded_mode);
        assert!(reopened_db.has_persisted_secrets().unwrap());
        assert!(reopened_secret_service.is_degraded());
    }
}
