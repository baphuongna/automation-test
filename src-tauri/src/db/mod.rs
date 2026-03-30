//! Database bootstrap module.
//!
//! T2 uses a single migration source of truth from `src-tauri/migrations`.

mod connection;
mod migrations;

use std::path::{Path, PathBuf};

use connection::create_migration_table;
pub use connection::DbConnection;
pub use migrations::{MigrationResult, MigrationRunner};

use crate::error::AppResult;

const DEFAULT_MIGRATIONS_DIR: &str = "migrations";

/// Database bootstrap wrapper.
pub struct Database {
    connection: DbConnection,
    migrations_dir: PathBuf,
}

impl Database {
    /// Create a new database and run migrations from the default migrations directory.
    pub fn new(path: PathBuf) -> AppResult<Self> {
        let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_MIGRATIONS_DIR);
        Self::new_with_migrations_dir(path, migrations_dir)
    }

    /// Create a new database with an explicit migrations directory.
    pub fn new_with_migrations_dir(path: PathBuf, migrations_dir: PathBuf) -> AppResult<Self> {
        let connection = DbConnection::new(&path)?;
        let database = Self {
            connection,
            migrations_dir,
        };

        database.run_migrations()?;
        Ok(database)
    }

    /// Run all pending migrations using the configured migrations directory.
    pub fn run_migrations(&self) -> AppResult<Vec<MigrationResult>> {
        create_migration_table(self.connection.connection())?;
        let runner = MigrationRunner::new(self.migrations_dir.clone());
        runner.run(self.connection.connection())
    }

    /// Return the active SQLite connection wrapper.
    pub fn db_connection(&self) -> &DbConnection {
        &self.connection
    }

    /// Return the raw SQLite connection.
    pub fn connection(&self) -> &rusqlite::Connection {
        self.connection.connection()
    }

    /// Return the database file path.
    pub fn path(&self) -> &Path {
        self.connection.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    fn create_test_migration(migrations_dir: &Path, name: &str, sql: &str) {
        fs::create_dir_all(migrations_dir).unwrap();
        fs::write(migrations_dir.join(name), sql).unwrap();
    }

    #[test]
    fn database_bootstrap_creates_db_and_runs_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db").join("testforge.db");
        let migrations_dir = temp_dir.path().join("migrations");

        create_test_migration(
            &migrations_dir,
            "001_test.sql",
            "CREATE TABLE IF NOT EXISTS bootstrap_probe (id INTEGER PRIMARY KEY);",
        );

        let database = Database::new_with_migrations_dir(db_path.clone(), migrations_dir).unwrap();

        assert!(db_path.exists());
        let count: i64 = database
            .connection()
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='bootstrap_probe'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn database_reruns_migrations_idempotently() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db").join("testforge.db");
        let migrations_dir = temp_dir.path().join("migrations");

        create_test_migration(
            &migrations_dir,
            "001_test.sql",
            "CREATE TABLE IF NOT EXISTS rerun_probe (id INTEGER PRIMARY KEY);",
        );

        let database = Database::new_with_migrations_dir(db_path, migrations_dir).unwrap();
        let rerun = database.run_migrations().unwrap();

        assert_eq!(rerun.len(), 1);
        assert!(matches!(rerun[0], MigrationResult::Skipped { .. }));
    }

    #[test]
    fn database_requires_existing_migrations_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db").join("testforge.db");
        let missing_migrations = temp_dir.path().join("missing-migrations");

        let result = Database::new_with_migrations_dir(db_path, missing_migrations);
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(matches!(error.code, crate::error::ErrorCode::DbMigration));
    }
}
