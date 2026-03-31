//! Data Table Repository
//! 
//! Provides CRUD operations for data tables and their rows.

use crate::error::{Result, TestForgeError};
use crate::models::{DataTable, DataTableRow, ColumnDefinition};
use rusqlite::{Connection, params};
use chrono::Utc;

/// Repository for data tables and their rows
pub struct DataTableRepository<'a> {
    conn: &'a Connection,
}

impl<'a> DataTableRepository<'a> {
    /// Create a new data table repository
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ==================== Data Table CRUD ====================

    /// Create a new data table
    pub fn create(&self, table: &DataTable) -> Result<()> {
        table
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let columns_json = serde_json::to_string(&table.columns)?;
        
        let sql = r#"
            INSERT INTO data_tables (id, name, description, columns_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#;

        self.conn.execute(
            sql,
            params![
                table.id,
                table.name,
                table.description,
                columns_json,
                table.created_at.to_rfc3339(),
                table.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Find a data table by ID
    pub fn find_by_id(&self, id: &str) -> Result<DataTable> {
        let sql = r#"
            SELECT id, name, description, columns_json, created_at, updated_at
            FROM data_tables
            WHERE id = ?1
        "#;

        let table = self.conn.query_row(sql, params![id], |row| {
            let columns_json: String = row.get(3)?;
            let columns: Vec<ColumnDefinition> = serde_json::from_str(&columns_json)
                .unwrap_or_default();

            Ok(DataTable {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                columns,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        Ok(table)
    }

    /// Find all data tables
    pub fn find_all(&self) -> Result<Vec<DataTable>> {
        let sql = r#"
            SELECT id, name, description, columns_json, created_at, updated_at
            FROM data_tables
            ORDER BY name ASC
        "#;

        let tables = self.conn
            .prepare(sql)?
            .query_map([], |row| {
                let columns_json: String = row.get(3)?;
                let columns: Vec<ColumnDefinition> = serde_json::from_str(&columns_json)
                    .unwrap_or_default();

                Ok(DataTable {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    columns,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tables)
    }

    /// Update a data table
    pub fn update(&self, table: &DataTable) -> Result<()> {
        table
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let columns_json = serde_json::to_string(&table.columns)?;
        
        let sql = r#"
            UPDATE data_tables
            SET name = ?1, description = ?2, columns_json = ?3, updated_at = ?4
            WHERE id = ?5
        "#;

        let rows_affected = self.conn.execute(
            sql,
            params![
                table.name,
                table.description,
                columns_json,
                table.updated_at.to_rfc3339(),
                table.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::DataTableNotFound { id: table.id.clone() });
        }

        Ok(())
    }

    /// Delete a data table by ID
    pub fn delete(&self, id: &str) -> Result<()> {
        // First delete all rows
        self.conn.execute(
            "DELETE FROM data_table_rows WHERE data_table_id = ?1",
            params![id],
        )?;

        // Then delete the table
        let rows_affected = self.conn.execute(
            "DELETE FROM data_tables WHERE id = ?1",
            params![id],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::DataTableNotFound { id: id.to_string() });
        }

        Ok(())
    }

    // ==================== Data Table Row CRUD ====================

    /// Create a data table row
    pub fn create_row(&self, row: &DataTableRow) -> Result<()> {
        row
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let sql = r#"
            INSERT INTO data_table_rows 
            (id, data_table_id, row_json, enabled, row_index, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#;

        self.conn.execute(
            sql,
            params![
                row.id,
                row.data_table_id,
                row.values,
                row.enabled,
                row.row_index,
                row.created_at.to_rfc3339(),
                row.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Find all rows for a data table
    pub fn find_rows_by_table(&self, data_table_id: &str) -> Result<Vec<DataTableRow>> {
        let sql = r#"
            SELECT id, data_table_id, row_json, enabled, row_index, created_at, updated_at
            FROM data_table_rows
            WHERE data_table_id = ?1
            ORDER BY row_index ASC
        "#;

        let rows = self.conn
            .prepare(sql)?
            .query_map(params![data_table_id], |row| {
                Ok(DataTableRow {
                    id: row.get(0)?,
                    data_table_id: row.get(1)?,
                    values: row.get(2)?,
                    enabled: row.get(3)?,
                    row_index: row.get(4)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Find enabled rows for a data table (for test execution)
    pub fn find_enabled_rows(&self, data_table_id: &str) -> Result<Vec<DataTableRow>> {
        let sql = r#"
            SELECT id, data_table_id, row_json, enabled, row_index, created_at, updated_at
            FROM data_table_rows
            WHERE data_table_id = ?1 AND enabled = 1
            ORDER BY row_index ASC
        "#;

        let rows = self.conn
            .prepare(sql)?
            .query_map(params![data_table_id], |row| {
                Ok(DataTableRow {
                    id: row.get(0)?,
                    data_table_id: row.get(1)?,
                    values: row.get(2)?,
                    enabled: row.get(3)?,
                    row_index: row.get(4)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Find a row by ID
    pub fn find_row_by_id(&self, id: &str) -> Result<DataTableRow> {
        let sql = r#"
            SELECT id, data_table_id, row_json, enabled, row_index, created_at, updated_at
            FROM data_table_rows
            WHERE id = ?1
        "#;

        let row = self.conn.query_row(sql, params![id], |row| {
            Ok(DataTableRow {
                id: row.get(0)?,
                data_table_id: row.get(1)?,
                values: row.get(2)?,
                enabled: row.get(3)?,
                row_index: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        Ok(row)
    }

    /// Update a data table row
    pub fn update_row(&self, row: &DataTableRow) -> Result<()> {
        row
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let sql = r#"
            UPDATE data_table_rows
            SET row_json = ?1, enabled = ?2, row_index = ?3, updated_at = ?4
            WHERE id = ?5
        "#;

        let rows_affected = self.conn.execute(
            sql,
            params![
                row.values,
                row.enabled,
                row.row_index,
                row.updated_at.to_rfc3339(),
                row.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::DataTableRowNotFound { id: row.id.clone() });
        }

        Ok(())
    }

    /// Delete a data table row
    pub fn delete_row(&self, id: &str) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM data_table_rows WHERE id = ?1",
            params![id],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::DataTableRowNotFound { id: id.to_string() });
        }

        Ok(())
    }

    /// Count rows for a data table
    pub fn count_rows(&self, data_table_id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM data_table_rows WHERE data_table_id = ?1",
            params![data_table_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Count enabled rows for a data table
    pub fn count_enabled_rows(&self, data_table_id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM data_table_rows WHERE data_table_id = ?1 AND enabled = 1",
            params![data_table_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::TempDir;

    fn create_test_repository() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_create_rejects_table_without_columns() {
        let (db, _temp_dir) = create_test_repository();
        let repo = DataTableRepository::new(db.connection());

        let table = DataTable::new("Users".to_string(), vec![]);

        let error = repo.create(&table).unwrap_err();
        assert!(matches!(error, TestForgeError::Validation(_)));
    }

    #[test]
    fn test_data_table_columns_round_trip_through_columns_json_schema() {
        let (db, _temp_dir) = create_test_repository();
        let repo = DataTableRepository::new(db.connection());

        let table = DataTable::from_column_names(
            "Users".to_string(),
            vec!["username".to_string(), "password".to_string()],
        );

        repo.create(&table).unwrap();
        let stored = repo.find_by_id(&table.id).unwrap();

        assert_eq!(stored.column_names(), vec!["username", "password"]);
    }

    #[test]
    fn test_data_table_rows_round_trip_through_row_json_schema() {
        let (db, _temp_dir) = create_test_repository();
        let repo = DataTableRepository::new(db.connection());

        let table = DataTable::from_column_names(
            "Users".to_string(),
            vec!["username".to_string()],
        );
        repo.create(&table).unwrap();

        let row = DataTableRow::with_index(
            table.id.clone(),
            vec!["alice".to_string()],
            0,
        );
        repo.create_row(&row).unwrap();

        let stored_rows = repo.find_rows_by_table(&table.id).unwrap();
        assert_eq!(stored_rows.len(), 1);
        assert_eq!(stored_rows[0].get_values().unwrap(), vec!["alice"]);
    }
}
