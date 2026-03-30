//! Data Table model
//! 
//! Represents a data table for data-driven testing.
//! Contains column definitions and references to rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Column definition for a data table
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColumnDefinition {
    /// Column name
    pub name: String,
    /// Column type (string, number, boolean)
    #[serde(default = "default_column_type")]
    pub col_type: String,
}

fn default_column_type() -> String {
    "string".to_string()
}

impl ColumnDefinition {
    /// Create a new column definition
    pub fn new(name: String) -> Self {
        Self {
            name,
            col_type: "string".to_string(),
        }
    }

    /// Create a column definition with a specific type
    pub fn with_type(name: String, col_type: String) -> Self {
        Self { name, col_type }
    }
}

/// Data Table model for data-driven testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTable {
    /// Unique identifier
    pub id: String,
    /// Table name
    pub name: String,
    /// Table description
    #[serde(default)]
    pub description: Option<String>,
    /// Column definitions
    pub columns: Vec<ColumnDefinition>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl DataTable {
    /// Create a new data table with the given name and columns
    pub fn new(name: String, columns: Vec<ColumnDefinition>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            columns,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new data table from column names
    pub fn from_column_names(name: String, column_names: Vec<String>) -> Self {
        let columns = column_names.into_iter().map(ColumnDefinition::new).collect();
        Self::new(name, columns)
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.updated_at = Utc::now();
        self
    }

    /// Get the number of columns
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Get column names
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    /// Find column index by name
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    /// Validate persistence constraints for data tables.
    pub fn validate_for_storage(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Data table name cannot be empty".to_string());
        }

        if self.columns.is_empty() {
            return Err("Data table must define at least one column".to_string());
        }

        if self.columns.iter().any(|column| column.name.trim().is_empty()) {
            return Err("Data table columns cannot have empty names".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_table_creation() {
        let columns = vec![
            ColumnDefinition::new("username".to_string()),
            ColumnDefinition::new("password".to_string()),
        ];
        let table = DataTable::new("Login Users".to_string(), columns);

        assert!(!table.id.is_empty());
        assert_eq!(table.name, "Login Users");
        assert_eq!(table.column_count(), 2);
    }

    #[test]
    fn test_data_table_from_column_names() {
        let table = DataTable::from_column_names(
            "Test Data".to_string(),
            vec!["col1".to_string(), "col2".to_string(), "col3".to_string()],
        );

        assert_eq!(table.column_count(), 3);
        assert_eq!(table.column_names(), vec!["col1", "col2", "col3"]);
    }

    #[test]
    fn test_column_index() {
        let table = DataTable::from_column_names(
            "Test".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );

        assert_eq!(table.column_index("a"), Some(0));
        assert_eq!(table.column_index("b"), Some(1));
        assert_eq!(table.column_index("notfound"), None);
    }

    #[test]
    fn test_column_definition_serialization() {
        let col = ColumnDefinition::with_type("count".to_string(), "number".to_string());
        let json = serde_json::to_string(&col).unwrap();
        let deserialized: ColumnDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(col.name, deserialized.name);
        assert_eq!(col.col_type, deserialized.col_type);
    }

    #[test]
    fn test_data_table_requires_columns_for_storage() {
        let table = DataTable::new("Users".to_string(), vec![]);
        let error = table.validate_for_storage().unwrap_err();
        assert_eq!(error, "Data table must define at least one column");
    }
}
