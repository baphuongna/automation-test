//! Path utilities for TestForge storage bootstrap.
//!
//! T2 storage layout keeps metadata in SQLite and filesystem artifacts under a
//! single app-data root. Screenshot payloads stay on disk, never in SQLite.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const APP_NAME: &str = "TestForge";
const DB_FILE_NAME: &str = "testforge.db";
const SETTINGS_FILE_NAME: &str = "settings.json";
const LOG_FILE_NAME: &str = "app.log";
const MASTER_KEY_FILE_NAME: &str = "master.key";

/// Minimal persisted settings bootstrap for Phase 1 foundation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSettings {
    pub schema_version: u32,
    pub database_path: String,
    pub logs_path: String,
    pub screenshots_path: String,
    pub exports_path: String,
}

impl BootstrapSettings {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            schema_version: 1,
            database_path: paths.database_file().to_string_lossy().into_owned(),
            logs_path: paths.logs.to_string_lossy().into_owned(),
            screenshots_path: paths.screenshots.to_string_lossy().into_owned(),
            exports_path: paths.exports.to_string_lossy().into_owned(),
        }
    }
}

/// App path policy under the app-data root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub base: PathBuf,
    pub db: PathBuf,
    pub logs: PathBuf,
    pub screenshots: PathBuf,
    pub exports: PathBuf,
    pub config: PathBuf,
}

impl AppPaths {
    /// Build the storage layout rooted at `base`.
    pub fn new(base: PathBuf) -> Self {
        Self {
            db: base.join("db"),
            logs: base.join("logs"),
            screenshots: base.join("screenshots"),
            exports: base.join("exports"),
            config: base.join("config"),
            base,
        }
    }

    /// Resolve app paths from the current platform policy without hardcoded machine paths.
    pub fn resolve() -> AppResult<Self> {
        Ok(Self::new(default_app_data_dir()?))
    }

    /// Create the required directory layout and a default settings file if missing.
    pub fn bootstrap(&self) -> AppResult<()> {
        for dir in [
            &self.base,
            &self.db,
            &self.logs,
            &self.screenshots,
            &self.exports,
            &self.config,
        ] {
            ensure_dir_exists(dir)?;
        }

        self.ensure_settings_file()?;
        Ok(())
    }

    /// Create default settings only on first bootstrap.
    pub fn ensure_settings_file(&self) -> AppResult<()> {
        let settings_path = self.settings_file();
        if settings_path.exists() {
            return Ok(());
        }

        let settings = BootstrapSettings::new(self);
        let payload = serde_json::to_string_pretty(&settings)?;
        fs::write(&settings_path, payload).map_err(|error| {
            AppError::storage_init(format!("Không thể tạo settings bootstrap: {error}"))
        })?;
        Ok(())
    }

    pub fn detect_first_run(&self) -> bool {
        !self.settings_file().exists()
    }

    pub fn database_file(&self) -> PathBuf {
        self.db.join(DB_FILE_NAME)
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join(SETTINGS_FILE_NAME)
    }

    pub fn master_key_file(&self) -> PathBuf {
        self.base.join(MASTER_KEY_FILE_NAME)
    }

    pub fn log_file(&self) -> PathBuf {
        self.logs.join(LOG_FILE_NAME)
    }

    pub fn is_initialized(&self) -> bool {
        [
            self.base.as_path(),
            self.db.as_path(),
            self.logs.as_path(),
            self.screenshots.as_path(),
            self.exports.as_path(),
            self.config.as_path(),
        ]
        .iter()
        .all(|path| path.exists())
            && self.settings_file().exists()
    }
}

/// Determine the default app-data directory using platform conventions.
pub fn default_app_data_dir() -> AppResult<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(dirs::data_local_dir)
            .map(|path| path.join(APP_NAME))
            .ok_or_else(|| {
                AppError::storage_path("Không xác định được thư mục LOCALAPPDATA cho TestForge")
            })
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join(APP_NAME)
            })
            .ok_or_else(|| {
                AppError::storage_path("Không xác định được home directory cho TestForge")
            })
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("share")))
            .map(|path| path.join(APP_NAME))
            .ok_or_else(|| {
                AppError::storage_path("Không xác định được app data directory cho TestForge")
            })
    }
}

/// Ensure a directory exists.
pub fn ensure_dir_exists(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path).map_err(|error| {
        AppError::storage_init(format!("Không thể tạo thư mục {:?}: {error}", path))
    })
}

pub fn database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("db").join(DB_FILE_NAME)
}

pub fn master_key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(MASTER_KEY_FILE_NAME)
}

pub fn screenshots_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("screenshots")
}

pub fn exports_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("exports")
}

pub fn logs_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn bootstrap_creates_required_directories_and_settings_file() {
        let temp_dir = TempDir::new().unwrap();
        let paths = AppPaths::new(temp_dir.path().join("app-data"));

        paths.bootstrap().unwrap();

        assert!(paths.is_initialized());
        assert!(paths.database_file().starts_with(&paths.base));
        assert!(paths.settings_file().exists());
    }

    #[test]
    fn bootstrap_settings_are_stable_on_rerun() {
        let temp_dir = TempDir::new().unwrap();
        let paths = AppPaths::new(temp_dir.path().join("app-data"));

        paths.bootstrap().unwrap();
        let original = fs::read_to_string(paths.settings_file()).unwrap();
        paths.bootstrap().unwrap();
        let rerun = fs::read_to_string(paths.settings_file()).unwrap();

        assert_eq!(original, rerun);
    }

    #[test]
    fn helper_paths_stay_under_app_data_root() {
        let root = PathBuf::from("C:/testforge-data");

        assert!(database_path(&root).starts_with(&root));
        assert!(master_key_path(&root).starts_with(&root));
        assert!(screenshots_path(&root).starts_with(&root));
        assert!(exports_path(&root).starts_with(&root));
        assert!(logs_path(&root).starts_with(&root));
    }
}
