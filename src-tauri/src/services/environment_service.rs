//! Environment service responsible for secret-safe environment variable flows.
//!
//! Mục tiêu của service này là đảm bảo secret plaintext từ command/input
//! luôn được mã hóa trước khi đi xuống repository và các read/list path mặc định
//! chỉ trả về masked preview thay vì ciphertext hay plaintext.

use chrono::Utc;

use crate::error::{Result, TestForgeError};
use crate::models::{EnvironmentVariable, VariableType};
use crate::repositories::EnvironmentRepository;
use crate::services::SecretService;

/// Service boundary for environment-variable persistence/read semantics.
pub struct EnvironmentService<'a> {
    repository: EnvironmentRepository<'a>,
    secret_service: &'a SecretService,
}

impl<'a> EnvironmentService<'a> {
    /// Create a new environment service.
    pub fn new(repository: EnvironmentRepository<'a>, secret_service: &'a SecretService) -> Self {
        Self {
            repository,
            secret_service,
        }
    }

    /// Upsert an environment variable from command/input payload.
    ///
    /// `raw_value` is treated as plaintext input from the UI/command layer.
    /// Secret values are encrypted here before any repository persistence.
    pub fn upsert_variable(
        &self,
        environment_id: &str,
        variable_id: Option<&str>,
        key: &str,
        var_type: VariableType,
        raw_value: &str,
        enabled: bool,
        description: Option<String>,
    ) -> Result<EnvironmentVariable> {
        let stored_variable = self.prepare_variable_for_storage(
            environment_id,
            variable_id,
            key,
            var_type,
            raw_value,
            enabled,
            description,
        )?;

        if variable_id.is_some() {
            self.repository.update_variable(&stored_variable)?;
        } else {
            self.repository.create_variable(&stored_variable)?;
        }

        Ok(self.mask_variable_for_output(stored_variable))
    }

    /// List variables for an environment with masked/default-safe output.
    pub fn list_variables(&self, environment_id: &str) -> Result<Vec<EnvironmentVariable>> {
        let variables = self.repository.find_variables_by_environment(environment_id)?;
        Ok(variables
            .into_iter()
            .map(|variable| self.mask_variable_for_output(variable))
            .collect())
    }

    /// Find a single variable with masked/default-safe output.
    pub fn find_variable_by_id(&self, id: &str) -> Result<EnvironmentVariable> {
        let variable = self.repository.find_variable_by_id(id)?;
        Ok(self.mask_variable_for_output(variable))
    }

    /// Delete a variable by id.
    pub fn delete_variable(&self, id: &str) -> Result<()> {
        self.repository.delete_variable(id)
    }

    fn prepare_variable_for_storage(
        &self,
        environment_id: &str,
        variable_id: Option<&str>,
        key: &str,
        var_type: VariableType,
        raw_value: &str,
        enabled: bool,
        description: Option<String>,
    ) -> Result<EnvironmentVariable> {
        let now = Utc::now();

        if var_type.is_secret() {
            if self.secret_service.is_degraded() {
                return Err(TestForgeError::DegradedMode(
                    "Cannot persist secret variables while master key is unavailable. Secret-dependent operations are blocked in degraded mode.".to_string(),
                ));
            }

            let encrypted_value = self.secret_service.encrypt(raw_value)?;
            let masked_preview = self.secret_service.generate_masked_preview(raw_value);

            let mut variable = EnvironmentVariable::new_secret(
                environment_id.to_string(),
                key.to_string(),
                encrypted_value,
                masked_preview,
            );

            if let Some(id) = variable_id {
                variable.id = id.to_string();
            }

            variable.enabled = enabled;
            variable.description = description;
            variable.updated_at = now;

            if variable_id.is_some() {
                if let Ok(existing) = self.repository.find_variable_by_id(&variable.id) {
                    variable.created_at = existing.created_at;
                }
            }

            return Ok(variable);
        }

        let mut variable = if let Some(id) = variable_id {
            let mut existing = self.repository.find_variable_by_id(id)?;
            existing.environment_id = environment_id.to_string();
            existing.key = key.to_string();
            existing.value = raw_value.to_string();
            existing.masked_preview = Some(raw_value.to_string());
            existing.var_type = VariableType::Regular;
            existing.enabled = enabled;
            existing.description = description;
            existing.updated_at = now;
            existing
        } else {
            let mut created = EnvironmentVariable::new(
                environment_id.to_string(),
                key.to_string(),
                raw_value.to_string(),
            );
            created.enabled = enabled;
            created.description = description;
            created.updated_at = now;
            created
        };

        variable.var_type = VariableType::Regular;
        variable.masked_preview = Some(raw_value.to_string());
        Ok(variable)
    }

    fn mask_variable_for_output(&self, mut variable: EnvironmentVariable) -> EnvironmentVariable {
        if variable.is_secret() {
            let masked_preview = variable
                .masked_preview
                .clone()
                .unwrap_or_else(|| "***".to_string());
            variable.value = masked_preview;
        }

        variable
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::db::Database;
    use crate::models::Environment;

    fn setup_service() -> (Database, SecretService, TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path).unwrap();
        let secret_service = SecretService::new(temp_dir.path().to_path_buf());
        secret_service.initialize().unwrap();

        let repo = EnvironmentRepository::new(db.connection());
        let environment = Environment::new("Staging".to_string());
        let environment_id = environment.id.clone();
        repo.create(&environment).unwrap();

        (db, secret_service, temp_dir, environment_id)
    }

    #[test]
    fn upsert_secret_encrypts_before_repository_persistence() {
        let (db, secret_service, _temp_dir, environment_id) = setup_service();
        let service = EnvironmentService::new(EnvironmentRepository::new(db.connection()), &secret_service);

        let stored = service
            .upsert_variable(
                &environment_id,
                None,
                "API_KEY",
                VariableType::Secret,
                "super-secret-value",
                true,
                None,
            )
            .unwrap();

        assert_eq!(stored.var_type, VariableType::Secret);
        assert_eq!(stored.value, "su***ue");
        assert_eq!(stored.masked_preview.as_deref(), Some("su***ue"));

        let raw = EnvironmentRepository::new(db.connection())
            .find_variables_by_environment(&environment_id)
            .unwrap()
            .into_iter()
            .find(|variable| variable.key == "API_KEY")
            .unwrap();

        assert_ne!(raw.value, "super-secret-value");
        assert!(raw.validate_for_storage().is_ok());
        assert_eq!(secret_service.decrypt(&raw.value).unwrap(), "super-secret-value");
        assert_eq!(raw.masked_preview.as_deref(), Some("su***ue"));
    }

    #[test]
    fn list_variables_masks_secret_values_by_default() {
        let (db, secret_service, _temp_dir, environment_id) = setup_service();
        let service = EnvironmentService::new(EnvironmentRepository::new(db.connection()), &secret_service);

        service
            .upsert_variable(
                &environment_id,
                None,
                "API_KEY",
                VariableType::Secret,
                "super-secret-value",
                true,
                None,
            )
            .unwrap();

        let variables = service.list_variables(&environment_id).unwrap();
        let variable = variables.into_iter().find(|item| item.key == "API_KEY").unwrap();

        assert_eq!(variable.value, "su***ue");
        assert_eq!(variable.display_value(), "su***ue");
    }

    #[test]
    fn degraded_mode_blocks_secret_upsert_without_plaintext_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path).unwrap();

        let corrupted_key_path = temp_dir.path().join("master.key");
        std::fs::write(&corrupted_key_path, b"broken-key").unwrap();

        let secret_service = SecretService::new(temp_dir.path().to_path_buf());
        assert!(secret_service.initialize().is_err());

        let repo = EnvironmentRepository::new(db.connection());
        let environment = Environment::new("Production".to_string());
        let environment_id = environment.id.clone();
        repo.create(&environment).unwrap();

        let service = EnvironmentService::new(EnvironmentRepository::new(db.connection()), &secret_service);
        let error = service
            .upsert_variable(
                &environment_id,
                None,
                "API_KEY",
                VariableType::Secret,
                "plaintext-should-never-persist",
                true,
                None,
            )
            .unwrap_err();

        assert!(matches!(error, TestForgeError::DegradedMode(_)));

        let stored = EnvironmentRepository::new(db.connection())
            .find_variables_by_environment(&environment_id)
            .unwrap();
        assert!(stored.is_empty());
    }

    #[test]
    fn regular_variable_upsert_preserves_plaintext_behavior() {
        let (db, secret_service, _temp_dir, environment_id) = setup_service();
        let service = EnvironmentService::new(EnvironmentRepository::new(db.connection()), &secret_service);

        let variable = service
            .upsert_variable(
                &environment_id,
                None,
                "BASE_URL",
                VariableType::Regular,
                "https://api.example.com",
                true,
                None,
            )
            .unwrap();

        assert_eq!(variable.value, "https://api.example.com");

        let stored = EnvironmentRepository::new(db.connection())
            .find_variables_by_environment(&environment_id)
            .unwrap()
            .into_iter()
            .find(|item| item.key == "BASE_URL")
            .unwrap();

        assert_eq!(stored.value, "https://api.example.com");
        assert_eq!(stored.masked_preview.as_deref(), Some("https://api.example.com"));
    }
}
