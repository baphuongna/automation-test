//! Migration runner for SQLite database bootstrap.

use std::fs;
use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use super::connection::create_migration_table;
use crate::error::{AppError, AppResult};

/// Result of a migration step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationResult {
    Applied { name: String },
    Skipped { name: String },
}

/// File-based SQL migration runner.
pub struct MigrationRunner {
    migrations_dir: PathBuf,
}

impl MigrationRunner {
    pub fn new(migrations_dir: PathBuf) -> Self {
        Self { migrations_dir }
    }

    /// Run all migrations from disk in filename order.
    pub fn run(&self, conn: &Connection) -> AppResult<Vec<MigrationResult>> {
        create_migration_table(conn)?;
        let mut migrations = self.load_migrations()?;
        migrations.sort_by(|left, right| left.name.cmp(&right.name));

        let mut results = Vec::with_capacity(migrations.len());
        for migration in migrations {
            results.push(self.apply_migration(conn, &migration)?);
        }

        Ok(results)
    }

    fn load_migrations(&self) -> AppResult<Vec<Migration>> {
        if !self.migrations_dir.exists() {
            return Err(AppError::db_migration(format!(
                "Migrations directory không tồn tại: {:?}",
                self.migrations_dir
            )));
        }

        let entries = fs::read_dir(&self.migrations_dir).map_err(|error| {
            AppError::storage_read(format!(
                "Không thể đọc thư mục migrations {:?}: {error}",
                self.migrations_dir
            ))
        })?;

        let mut migrations = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| AppError::storage_read(error.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
                continue;
            }

            let content = fs::read_to_string(&path).map_err(|error| {
                AppError::storage_read(format!("Không thể đọc migration {:?}: {error}", path))
            })?;
            let name = path
                .file_name()
                .and_then(|file| file.to_str())
                .ok_or_else(|| AppError::db_migration(format!("Tên migration không hợp lệ: {:?}", path)))?
                .to_owned();

            migrations.push(Migration {
                checksum: Self::calculate_checksum(&content),
                content,
                name,
            });
        }

        Ok(migrations)
    }

    fn apply_migration(&self, conn: &Connection, migration: &Migration) -> AppResult<MigrationResult> {
        let stored_checksum = conn
            .query_row(
                "SELECT checksum FROM _migrations WHERE name = ?1",
                params![migration.name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| AppError::db_query(format!("Không thể kiểm tra migration {}: {error}", migration.name)))?;

        if let Some(existing_checksum) = stored_checksum {
            if existing_checksum != migration.checksum {
                return Err(AppError::db_migration(format!(
                    "Migration '{}' checksum mismatch. Stored '{}', current '{}'.",
                    migration.name, existing_checksum, migration.checksum
                )));
            }

            return Ok(MigrationResult::Skipped {
                name: migration.name.clone(),
            });
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|error| AppError::db_query(format!("Không thể bắt đầu migration transaction: {error}")))?;

        tx.execute_batch(&migration.content).map_err(|error| {
            AppError::db_migration(format!("Thực thi migration '{}' thất bại: {error}", migration.name))
        })?;

        tx.execute(
            "INSERT INTO _migrations (name, checksum) VALUES (?1, ?2)",
            params![migration.name, migration.checksum],
        )
        .map_err(|error| AppError::db_query(format!("Không thể ghi nhận migration {}: {error}", migration.name)))?;

        tx.commit()
            .map_err(|error| AppError::db_query(format!("Không thể commit migration {}: {error}", migration.name)))?;

        Ok(MigrationResult::Applied {
            name: migration.name.clone(),
        })
    }

    fn calculate_checksum(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

struct Migration {
    name: String,
    content: String,
    checksum: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, Connection, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        let migrations_dir = temp_dir.path().join("migrations");
        fs::create_dir_all(&migrations_dir).unwrap();
        fs::write(
            migrations_dir.join("001_test.sql"),
            "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        (temp_dir, conn, migrations_dir)
    }

    #[test]
    fn migration_runner_applies_pending_migrations() {
        let (_temp_dir, conn, migrations_dir) = setup_test_env();

        let runner = MigrationRunner::new(migrations_dir);
        let results = runner.run(&conn).unwrap();

        assert_eq!(results, vec![MigrationResult::Applied { name: "001_test.sql".to_string() }]);
        conn.execute("INSERT INTO test (id) VALUES (1)", []).unwrap();
    }

    #[test]
    fn migration_runner_is_idempotent() {
        let (_temp_dir, conn, migrations_dir) = setup_test_env();

        let runner = MigrationRunner::new(migrations_dir);
        runner.run(&conn).unwrap();

        let results = runner.run(&conn).unwrap();
        assert_eq!(results, vec![MigrationResult::Skipped { name: "001_test.sql".to_string() }]);
    }

    #[test]
    fn migration_runner_detects_checksum_mismatch() {
        let (_temp_dir, conn, migrations_dir) = setup_test_env();

        let runner = MigrationRunner::new(migrations_dir.clone());
        runner.run(&conn).unwrap();

        fs::write(
            migrations_dir.join("001_test.sql"),
            "CREATE TABLE IF NOT EXISTS test_v2 (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        let result = runner.run(&conn);
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn migration_runner_records_full_filename_and_computed_checksum() {
        let (_temp_dir, conn, migrations_dir) = setup_test_env();
        let runner = MigrationRunner::new(migrations_dir.clone());

        runner.run(&conn).unwrap();

        let content = fs::read_to_string(migrations_dir.join("001_test.sql")).unwrap();
        let stored: (String, String) = conn
            .query_row(
                "SELECT name, checksum FROM _migrations ORDER BY id ASC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(stored.0, "001_test.sql");
        assert_eq!(stored.1, MigrationRunner::calculate_checksum(&content));
    }

    #[test]
    fn migration_runner_rejects_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let conn = Connection::open(temp_dir.path().join("test.db")).unwrap();
        let runner = MigrationRunner::new(temp_dir.path().join("missing"));

        let result = runner.run(&conn);
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert!(matches!(error.code, crate::error::ErrorCode::DbMigration));
    }
}
