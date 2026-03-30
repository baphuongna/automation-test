//! SQLite connection management
//!
//! Module này quản lý kết nối SQLite, bao gồm:
//! - Tạo kết nối đến database file
//! - Thiết lập connection settings (foreign keys, WAL mode)
//! - Connection pool wrapper cho future use

use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use crate::error::{AppError, AppResult};

/// Database connection wrapper
pub struct DbConnection {
    conn: Connection,
    db_path: String,
}

impl DbConnection {
    /// Tạo connection mới đến SQLite database
    /// 
    /// # Arguments
    /// * `db_path` - Đường dẫn đến database file
    /// 
    /// # Errors
    /// Trả về AppError nếu không thể tạo connection
    pub fn new(db_path: &Path) -> AppResult<Self> {
        // Tạo parent directory nếu chưa tồn tại
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::storage_path(format!("Cannot create db directory: {}", e)))?;
        }
        
        // Mở connection với flags phù hợp
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE 
                | OpenFlags::SQLITE_OPEN_CREATE
        ).map_err(|e| AppError::db_connection(format!("Failed to open database: {}", e)))?;
        
        // Thiết lập SQLite pragmas
        let db = Self {
            conn,
            db_path: db_path.to_string_lossy().into_owned(),
        };
        db.configure_pragmas()?;
        
        Ok(db)
    }
    
    /// Configure SQLite pragmas cho performance và integrity
    fn configure_pragmas(&self) -> AppResult<()> {
        // Bật foreign key constraints
        self.conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA busy_timeout = 5000;",
        ).map_err(|e| AppError::db_query(format!("Failed to set pragmas: {}", e)))?;
        
        Ok(())
    }
    
    /// Lấy reference đến underlying connection
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
    
    /// Lấy mutable reference đến connection
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
    
    /// Lấy database path
    pub fn path(&self) -> &Path {
        Path::new(&self.db_path)
    }
    
    /// Execute a transaction
    /// 
    /// # Arguments
    /// * `f` - Closure chứa các database operations
    pub fn transaction<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&rusqlite::Transaction) -> AppResult<T>,
    {
        let tx = self.conn.unchecked_transaction()
            .map_err(|e| AppError::db_query(format!("Failed to begin transaction: {}", e)))?;
        
        let result = f(&tx)?;
        
        tx.commit()
            .map_err(|e| AppError::db_query(format!("Failed to commit transaction: {}", e)))?;
        
        Ok(result)
    }
    
    /// Check if migration metadata table exists
    pub fn migration_table_exists(&self) -> AppResult<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_migrations')",
            [],
            |row| row.get(0),
        ).map_err(|e| AppError::db_query(format!("Failed to check migration table: {}", e)))?;
        
        Ok(exists)
    }
    
    /// Get list of applied migrations
    pub fn get_applied_migrations(&self) -> AppResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM _migrations ORDER BY name ASC"
        ).map_err(|e| AppError::db_query(format!("Failed to prepare statement: {}", e)))?;
        
        let migrations = stmt.query_map([], |row| row.get(0))
            .map_err(|e| AppError::db_query(format!("Failed to query migrations: {}", e)))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| AppError::db_query(format!("Failed to collect migrations: {}", e)))?;
        
        Ok(migrations)
    }
    
    /// Record a migration as applied
    pub fn record_migration(&self, name: &str, checksum: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO _migrations (name, checksum, applied_at) VALUES (?1, ?2, datetime('now'))",
            params![name, checksum],
        ).map_err(|e| AppError::db_query(format!("Failed to record migration: {}", e)))?;
        
        Ok(())
    }
    
    /// Check if a specific migration was applied
    pub fn is_migration_applied(&self, name: &str) -> AppResult<bool> {
        let applied: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?1)",
            params![name],
            |row| row.get(0),
        ).map_err(|e| AppError::db_query(format!("Failed to check migration: {}", e)))?;
        
        Ok(applied)
    }
}

/// Create migration metadata table
pub fn create_migration_table(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
        [],
    ).map_err(|e| AppError::db_migration(format!("Failed to create migration table: {}", e)))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_db_connection_new() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = DbConnection::new(&db_path).unwrap();
        assert!(db_path.exists());
    }
    
    #[test]
    fn test_pragmas_configured() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = DbConnection::new(&db_path).unwrap();
        
        // Verify foreign keys are enabled
        let fk_enabled: i32 = db.conn.query_row(
            "PRAGMA foreign_keys",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(fk_enabled, 1);
    }
    
    #[test]
    fn test_migration_table_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = DbConnection::new(&db_path).unwrap();
        
        // Migration table should not exist initially
        let exists = db.migration_table_exists().unwrap();
        assert!(!exists);
        
        // Create migration table
        create_migration_table(&db.conn).unwrap();
        
        // Now it should exist
        let exists = db.migration_table_exists().unwrap();
        assert!(exists);
    }
    
    #[test]
    fn test_record_and_check_migration() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = DbConnection::new(&db_path).unwrap();
        create_migration_table(&db.conn).unwrap();
        
        // Record a migration
        db.record_migration("001_initial", "abc123").unwrap();
        
        // Check if it was applied
        let applied = db.is_migration_applied("001_initial").unwrap();
        assert!(applied);
        
        // Check non-existent migration
        let applied = db.is_migration_applied("999_nonexistent").unwrap();
        assert!(!applied);
    }
}
