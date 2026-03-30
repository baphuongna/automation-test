//! TestForge application entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use testforge::{
    db::Database,
    error::{AppError, AppResult},
    services::SecretService,
    AppState,
};
use testforge::utils::paths::AppPaths;

/// Output of the backend storage bootstrap.
pub struct BootstrapResult {
    pub app_state: Arc<AppState>,
    pub paths: AppPaths,
}

/// Bootstrap storage and state for the desktop app.
pub fn bootstrap(app_handle: &tauri::AppHandle) -> AppResult<BootstrapResult> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| AppError::storage_path(format!("Không xác định được app data directory: {error}")))?;

    let paths = AppPaths::new(app_data_dir);
    paths.bootstrap()?;

    let database = Database::new(paths.database_file())?;

    let secret_service = SecretService::new(paths.base.clone());
    let degraded_mode = secret_service.initialize().is_err();

    let app_state = Arc::new(AppState::new(database, secret_service, paths.clone()));
    app_state.set_degraded_mode(degraded_mode);
    app_state.set_master_key_initialized(!degraded_mode);

    Ok(BootstrapResult { app_state, paths })
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
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context![])
        .expect("Error while running Tauri application");
}

fn main() {
    run();
}
