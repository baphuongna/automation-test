//! Data Table Row model
//! 
//! Represents a single row in a data table for data-driven testing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Data Table Row model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableRow {
    /// Unique identifier
    pub id: String,
    /// ID of the parent data table
    pub data_table_id: String,
    /// Row values as JSON string (array matching column order)
    pub values: String,
    /// Whether this row is enabled for test execution
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Row index for ordering
    #[serde(default)]
    pub row_index: i32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl DataTableRow {
    /// Create a new data table row
    pub fn new(data_table_id: String, values: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            data_table_id,
            values: serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()),
            enabled: true,
            row_index: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a row with a specific index
    pub fn with_index(data_table_id: String, values: Vec<String>, row_index: i32) -> Self {
        let mut row = Self::new(data_table_id, values);
        row.row_index = row_index;
        row
    }

    /// Set enabled status
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.updated_at = Utc::now();
        self
    }

    /// Set row index
    pub fn set_index(mut self, index: i32) -> Self {
        self.row_index = index;
        self.updated_at = Utc::now();
        self
    }

    /// Get values as a vector
    pub fn get_values(&self) -> Result<Vec<String>, serde_json::Error> {
        serde_json::from_str(&self.values)
    }

    /// Update values
    pub fn set_values(&mut self, values: Vec<String>) -> Result<(), serde_json::Error> {
        self.values = serde_json::to_string(&values)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get a value by column index
    pub fn get_value(&self, index: usize) -> Option<String> {
        self.get_values().ok()?.get(index).cloned()
    }

    /// Validate row persistence constraints.
    pub fn validate_for_storage(&self) -> Result<(), String> {
        if self.data_table_id.trim().is_empty() {
            return Err("Data table row must belong to a data table".to_string());
        }

        self.get_values()
            .map(|_| ())
            .map_err(|_| "Data table row values must be valid JSON array".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_table_row_creation() {
        let row = DataTableRow::new(
            "table-1".to_string(),
            vec!["value1".to_string(), "value2".to_string()],
        );

        assert!(!row.id.is_empty());
        assert_eq!(row.data_table_id, "table-1");
        assert!(row.enabled);
    }

    #[test]
    fn test_get_values() {
        let row = DataTableRow::new(
            "table-1".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );

        let values = row.get_values().unwrap();
        assert_eq!(values, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_get_value_by_index() {
        let row = DataTableRow::new(
            "table-1".to_string(),
            vec!["first".to_string(), "second".to_string()],
        );

        assert_eq!(row.get_value(0), Some("first".to_string()));
        assert_eq!(row.get_value(1), Some("second".to_string()));
        assert_eq!(row.get_value(2), None);
    }

    #[test]
    fn test_set_values() {
        let mut row = DataTableRow::new(
            "table-1".to_string(),
            vec!["old".to_string()],
        );

        row.set_values(vec!["new1".to_string(), "new2".to_string()]).unwrap();
        assert_eq!(row.get_values().unwrap(), vec!["new1", "new2"]);
    }

    #[test]
    fn test_row_serialization() {
        let row = DataTableRow::new(
            "table-1".to_string(),
            vec!["v1".to_string(), "v2".to_string()],
        );

        let json = serde_json::to_string(&row).unwrap();
        let deserialized: DataTableRow = serde_json::from_str(&json).unwrap();
        assert_eq!(row.id, deserialized.id);
        assert_eq!(row.get_values().unwrap(), deserialized.get_values().unwrap());
    }

    #[test]
    fn test_data_table_row_requires_valid_json_values() {
        let mut row = DataTableRow::new("table-1".to_string(), vec!["v1".to_string()]);
        row.values = "not-json".to_string();

        let error = row.validate_for_storage().unwrap_err();
        assert_eq!(error, "Data table row values must be valid JSON array");
    }
}
