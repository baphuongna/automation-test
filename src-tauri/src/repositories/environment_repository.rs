//! Environment Repository
//!
//! Provides CRUD operations for environments and environment variables.

use crate::error::{Result, TestForgeError};
use crate::models::{Environment, EnvironmentType, EnvironmentVariable, VariableType};
use chrono::Utc;
use rusqlite::{params, Connection};

/// Repository for environments and their variables
pub struct EnvironmentRepository<'a> {
    conn: &'a Connection,
}

impl<'a> EnvironmentRepository<'a> {
    /// Create a new environment repository
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ==================== Environment CRUD ====================

    /// Create a new environment
    pub fn create(&self, environment: &Environment) -> Result<()> {
        environment
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let sql = r#"
            INSERT INTO environments (id, name, description, env_type, is_default, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#;

        self.conn.execute(
            sql,
            params![
                environment.id,
                environment.name,
                environment.description,
                environment.env_type.as_str(),
                environment.is_default,
                environment.created_at.to_rfc3339(),
                environment.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Find an environment by ID
    pub fn find_by_id(&self, id: &str) -> Result<Environment> {
        let sql = r#"
            SELECT id, name, description, env_type, is_default, created_at, updated_at
            FROM environments
            WHERE id = ?1
        "#;

        let env = self.conn.query_row(sql, params![id], |row| {
            Ok(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                env_type: EnvironmentType::from_str(&row.get::<_, String>(3)?)
                    .unwrap_or(EnvironmentType::Custom),
                is_default: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        Ok(env)
    }

    /// Find all environments
    pub fn find_all(&self) -> Result<Vec<Environment>> {
        let sql = r#"
            SELECT id, name, description, env_type, is_default, created_at, updated_at
            FROM environments
            ORDER BY name ASC
        "#;

        let environments = self
            .conn
            .prepare(sql)?
            .query_map([], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    env_type: EnvironmentType::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(EnvironmentType::Custom),
                    is_default: row.get(4)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(environments)
    }

    /// Find the default environment
    pub fn find_default(&self) -> Result<Option<Environment>> {
        let sql = r#"
            SELECT id, name, description, env_type, is_default, created_at, updated_at
            FROM environments
            WHERE is_default = 1
            LIMIT 1
        "#;

        let result = self.conn.query_row(sql, [], |row| {
            Ok(Environment {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                env_type: EnvironmentType::from_str(&row.get::<_, String>(3)?)
                    .unwrap_or(EnvironmentType::Custom),
                is_default: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(env) => Ok(Some(env)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TestForgeError::from(e)),
        }
    }

    /// Update an environment
    pub fn update(&self, environment: &Environment) -> Result<()> {
        environment
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let sql = r#"
            UPDATE environments
            SET name = ?1, description = ?2, env_type = ?3, is_default = ?4, updated_at = ?5
            WHERE id = ?6
        "#;

        let rows_affected = self.conn.execute(
            sql,
            params![
                environment.name,
                environment.description,
                environment.env_type.as_str(),
                environment.is_default,
                environment.updated_at.to_rfc3339(),
                environment.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::EnvironmentNotFound {
                id: environment.id.clone(),
            });
        }

        Ok(())
    }

    /// Delete an environment by ID
    pub fn delete(&self, id: &str) -> Result<()> {
        // First delete all environment variables
        self.conn.execute(
            "DELETE FROM environment_variables WHERE environment_id = ?1",
            params![id],
        )?;

        // Then delete the environment
        let rows_affected = self
            .conn
            .execute("DELETE FROM environments WHERE id = ?1", params![id])?;

        if rows_affected == 0 {
            return Err(TestForgeError::EnvironmentNotFound { id: id.to_string() });
        }

        Ok(())
    }

    // ==================== Environment Variable CRUD ====================

    /// Create an environment variable
    pub fn create_variable(&self, variable: &EnvironmentVariable) -> Result<()> {
        variable
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let sql = r#"
            INSERT INTO environment_variables 
            (id, environment_id, key, value, masked_preview, var_type, enabled, description, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#;

        self.conn.execute(
            sql,
            params![
                variable.id,
                variable.environment_id,
                variable.key,
                variable.value,
                variable.masked_preview,
                variable.var_type.as_str(),
                variable.enabled,
                variable.description,
                variable.created_at.to_rfc3339(),
                variable.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Find all variables for an environment
    pub fn find_variables_by_environment(
        &self,
        environment_id: &str,
    ) -> Result<Vec<EnvironmentVariable>> {
        let sql = r#"
            SELECT id, environment_id, key, value, masked_preview, var_type, enabled, description, created_at, updated_at
            FROM environment_variables
            WHERE environment_id = ?1
            ORDER BY key ASC
        "#;

        let variables = self
            .conn
            .prepare(sql)?
            .query_map(params![environment_id], |row| {
                Ok(EnvironmentVariable {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    masked_preview: row.get(4)?,
                    var_type: VariableType::from_str(&row.get::<_, String>(5)?)
                        .unwrap_or(VariableType::Regular),
                    enabled: row.get(6)?,
                    description: row.get(7)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(variables)
    }

    /// Find a variable by ID
    pub fn find_variable_by_id(&self, id: &str) -> Result<EnvironmentVariable> {
        let sql = r#"
            SELECT id, environment_id, key, value, masked_preview, var_type, enabled, description, created_at, updated_at
            FROM environment_variables
            WHERE id = ?1
        "#;

        let variable = self.conn.query_row(sql, params![id], |row| {
            Ok(EnvironmentVariable {
                id: row.get(0)?,
                environment_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                masked_preview: row.get(4)?,
                var_type: VariableType::from_str(&row.get::<_, String>(5)?)
                    .unwrap_or(VariableType::Regular),
                enabled: row.get(6)?,
                description: row.get(7)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        Ok(variable)
    }

    /// Update an environment variable
    pub fn update_variable(&self, variable: &EnvironmentVariable) -> Result<()> {
        variable
            .validate_for_storage()
            .map_err(TestForgeError::Validation)?;

        let sql = r#"
            UPDATE environment_variables
            SET key = ?1, value = ?2, masked_preview = ?3, var_type = ?4, enabled = ?5, 
                description = ?6, updated_at = ?7
            WHERE id = ?8
        "#;

        let rows_affected = self.conn.execute(
            sql,
            params![
                variable.key,
                variable.value,
                variable.masked_preview,
                variable.var_type.as_str(),
                variable.enabled,
                variable.description,
                variable.updated_at.to_rfc3339(),
                variable.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::EnvironmentVariableNotFound {
                id: variable.id.clone(),
            });
        }

        Ok(())
    }

    /// Delete an environment variable
    pub fn delete_variable(&self, id: &str) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM environment_variables WHERE id = ?1",
            params![id],
        )?;

        if rows_affected == 0 {
            return Err(TestForgeError::EnvironmentVariableNotFound { id: id.to_string() });
        }

        Ok(())
    }

    /// Clear the default flag from all environments
    pub fn clear_default(&self) -> Result<()> {
        self.conn
            .execute("UPDATE environments SET is_default = 0", [])?;
        Ok(())
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
    fn test_create_variable_rejects_plaintext_secret_value() {
        let (db, _temp_dir) = create_test_repository();
        let repo = EnvironmentRepository::new(db.connection());

        let environment = Environment::new("Staging".to_string());
        repo.create(&environment).unwrap();

        let variable = EnvironmentVariable {
            id: "var-1".to_string(),
            environment_id: environment.id.clone(),
            key: "API_KEY".to_string(),
            value: "plaintext-secret".to_string(),
            masked_preview: Some("pl***et".to_string()),
            var_type: VariableType::Secret,
            enabled: true,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let error = repo.create_variable(&variable).unwrap_err();
        assert!(matches!(error, TestForgeError::Validation(_)));
    }

    #[test]
    fn test_find_variables_preserves_encrypted_secret_storage_shape() {
        let (db, _temp_dir) = create_test_repository();
        let repo = EnvironmentRepository::new(db.connection());

        let environment = Environment::new("Staging".to_string());
        repo.create(&environment).unwrap();

        let secret = EnvironmentVariable::new_secret(
            environment.id.clone(),
            "API_KEY".to_string(),
            "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo=".to_string(),
            "ab***yz".to_string(),
        );

        repo.create_variable(&secret).unwrap();

        let stored = repo.find_variables_by_environment(&environment.id).unwrap();
        assert_eq!(stored.len(), 1);
        assert!(stored[0].validate_for_storage().is_ok());
        assert_eq!(stored[0].display_value(), "ab***yz");
    }
}
