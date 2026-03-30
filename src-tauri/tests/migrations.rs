//! Integration coverage for T2 storage bootstrap semantics.

use std::fs;

use tempfile::TempDir;
use testforge::db::{Database, MigrationResult, MigrationRunner};
use testforge::utils::paths::AppPaths;

#[test]
fn fresh_bootstrap_initializes_storage_layout_and_database() {
    let temp_dir = TempDir::new().unwrap();
    let app_paths = AppPaths::new(temp_dir.path().join("app-data"));
    app_paths.bootstrap().unwrap();

    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir_all(&migrations_dir).unwrap();
    fs::write(
        migrations_dir.join("001_test.sql"),
        "CREATE TABLE IF NOT EXISTS integration_probe (id INTEGER PRIMARY KEY);",
    )
    .unwrap();

    let database = Database::new_with_migrations_dir(app_paths.database_file(), migrations_dir).unwrap();

    assert!(app_paths.database_file().exists());
    assert!(app_paths.settings_file().exists());

    let probe_table_count: i64 = database
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='integration_probe'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(probe_table_count, 1);
}

#[test]
fn migration_rerun_is_idempotent_and_preserves_existing_data() {
    let temp_dir = TempDir::new().unwrap();
    let app_paths = AppPaths::new(temp_dir.path().join("app-data"));
    app_paths.bootstrap().unwrap();

    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir_all(&migrations_dir).unwrap();
    fs::write(
        migrations_dir.join("001_test.sql"),
        "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    )
    .unwrap();

    let database = Database::new_with_migrations_dir(app_paths.database_file(), migrations_dir.clone()).unwrap();
    database
        .connection()
        .execute("INSERT INTO users (name) VALUES ('alice')", [])
        .unwrap();

    let rerun_results = MigrationRunner::new(migrations_dir)
        .run(database.connection())
        .unwrap();

    assert_eq!(rerun_results, vec![MigrationResult::Skipped { name: "001_test.sql".to_string() }]);

    let row_count: i64 = database
        .connection()
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .unwrap();
    assert_eq!(row_count, 1);
}

#[test]
fn checksum_mismatch_fails_rerun_clearly() {
    let temp_dir = TempDir::new().unwrap();
    let app_paths = AppPaths::new(temp_dir.path().join("app-data"));
    app_paths.bootstrap().unwrap();

    let migrations_dir = temp_dir.path().join("migrations");
    fs::create_dir_all(&migrations_dir).unwrap();
    let migration_file = migrations_dir.join("001_test.sql");

    fs::write(
        &migration_file,
        "CREATE TABLE IF NOT EXISTS checksum_probe (id INTEGER PRIMARY KEY);",
    )
    .unwrap();

    let database = Database::new_with_migrations_dir(app_paths.database_file(), migrations_dir.clone()).unwrap();

    fs::write(
        &migration_file,
        "CREATE TABLE IF NOT EXISTS checksum_probe_v2 (id INTEGER PRIMARY KEY);",
    )
    .unwrap();

    let error = MigrationRunner::new(migrations_dir)
        .run(database.connection())
        .unwrap_err();

    assert!(error.to_string().contains("checksum mismatch"));
}
