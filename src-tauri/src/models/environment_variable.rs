//! Environment Variable model
//! 
//! Represents a variable in an environment, which can be a regular
//! variable or a secret variable that requires encryption.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of environment variable
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    /// Regular variable (stored as plaintext)
    #[default]
    Regular,
    /// Secret variable (stored encrypted)
    Secret,
}

impl VariableType {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Secret => "secret",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "regular" => Some(Self::Regular),
            "secret" => Some(Self::Secret),
            _ => None,
        }
    }

    /// Check if this variable type is a secret
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret)
    }
}

/// Environment Variable model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    /// Unique identifier
    pub id: String,
    /// ID of the parent environment
    pub environment_id: String,
    /// Variable key/name (e.g., "API_KEY", "BASE_URL")
    pub key: String,
    /// Variable value (plaintext for regular, encrypted for secret)
    /// Note: For secrets, this contains the encrypted value
    pub value: String,
    /// Masked preview of the value (e.g., "ab***yz")
    #[serde(default)]
    pub masked_preview: Option<String>,
    /// Type of variable
    #[serde(default)]
    pub var_type: VariableType,
    /// Whether this variable is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl EnvironmentVariable {
    /// Create a new regular variable
    pub fn new(environment_id: String, key: String, value: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            environment_id,
            key,
            value: value.clone(),
            masked_preview: Some(value), // For regular variables, preview is the same
            var_type: VariableType::Regular,
            enabled: true,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new secret variable (value should already be encrypted)
    pub fn new_secret(
        environment_id: String,
        key: String,
        encrypted_value: String,
        masked_preview: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            environment_id,
            key,
            value: encrypted_value,
            masked_preview: Some(masked_preview),
            var_type: VariableType::Secret,
            enabled: true,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.updated_at = Utc::now();
        self
    }

    /// Set enabled status
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.updated_at = Utc::now();
        self
    }

    /// Check if this is a secret variable
    pub fn is_secret(&self) -> bool {
        self.var_type.is_secret()
    }

    /// Get the display value (masked for secrets, actual for regular)
    pub fn display_value(&self) -> &str {
        if self.is_secret() {
            self.masked_preview.as_deref().unwrap_or("***")
        } else {
            &self.value
        }
    }

    /// Validate invariants required before persistence.
    ///
    /// Secret variables must only persist encrypted-looking values plus a masked preview.
    pub fn validate_for_storage(&self) -> Result<(), String> {
        if self.key.trim().is_empty() {
            return Err("Environment variable key cannot be empty".to_string());
        }

        if self.environment_id.trim().is_empty() {
            return Err("Environment variable must belong to an environment".to_string());
        }

        if self.is_secret() {
            if self.value.trim().is_empty() {
                return Err("Secret variable must contain encrypted value".to_string());
            }

            if !looks_like_encrypted_secret(&self.value) {
                return Err("Secret variable value must be encrypted before storage".to_string());
            }

            let preview = self.masked_preview.as_deref().unwrap_or("");
            if preview.is_empty() {
                return Err("Secret variable must include masked preview".to_string());
            }

            if preview == self.value {
                return Err("Secret variable preview must not expose stored value".to_string());
            }
        }

        Ok(())
    }
}

fn looks_like_encrypted_secret(value: &str) -> bool {
    if value.len() < 24 {
        return false;
    }

    value
        .bytes()
        .all(|byte| matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_variable_creation() {
        let var = EnvironmentVariable::new(
            "env-1".to_string(),
            "BASE_URL".to_string(),
            "https://api.example.com".to_string(),
        );
        assert!(!var.id.is_empty());
        assert_eq!(var.key, "BASE_URL");
        assert_eq!(var.value, "https://api.example.com");
        assert_eq!(var.var_type, VariableType::Regular);
        assert!(!var.is_secret());
    }

    #[test]
    fn test_secret_variable_creation() {
        let var = EnvironmentVariable::new_secret(
            "env-1".to_string(),
            "API_KEY".to_string(),
            "encrypted_value_here".to_string(),
            "ab***yz".to_string(),
        );
        assert_eq!(var.var_type, VariableType::Secret);
        assert!(var.is_secret());
        assert_eq!(var.display_value(), "ab***yz");
    }

    #[test]
    fn test_variable_type_serialization() {
        assert_eq!(
            serde_json::to_string(&VariableType::Secret).unwrap(),
            "\"secret\""
        );
        assert_eq!(
            serde_json::to_string(&VariableType::Regular).unwrap(),
            "\"regular\""
        );
    }

    #[test]
    fn test_variable_type_parse() {
        assert_eq!(VariableType::from_str("secret"), Some(VariableType::Secret));
        assert_eq!(VariableType::from_str("regular"), Some(VariableType::Regular));
        assert_eq!(VariableType::from_str("invalid"), None);
    }

    #[test]
    fn test_secret_variable_requires_encrypted_storage_shape() {
        let var = EnvironmentVariable {
            id: Uuid::new_v4().to_string(),
            environment_id: "env-1".to_string(),
            key: "API_KEY".to_string(),
            value: "plaintext-secret".to_string(),
            masked_preview: Some("pl***et".to_string()),
            var_type: VariableType::Secret,
            enabled: true,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let error = var.validate_for_storage().unwrap_err();
        assert_eq!(error, "Secret variable value must be encrypted before storage");
    }

    #[test]
    fn test_secret_variable_requires_masked_preview() {
        let var = EnvironmentVariable {
            id: Uuid::new_v4().to_string(),
            environment_id: "env-1".to_string(),
            key: "API_KEY".to_string(),
            value: "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo=".to_string(),
            masked_preview: None,
            var_type: VariableType::Secret,
            enabled: true,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let error = var.validate_for_storage().unwrap_err();
        assert_eq!(error, "Secret variable must include masked preview");
    }

    #[test]
    fn test_regular_variable_allows_plaintext_storage() {
        let var = EnvironmentVariable::new(
            "env-1".to_string(),
            "BASE_URL".to_string(),
            "https://api.example.com".to_string(),
        );

        assert!(var.validate_for_storage().is_ok());
    }

    #[test]
    fn test_secret_variable_preview_must_not_match_encrypted_value() {
        let encrypted_value = "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo=".to_string();
        let var = EnvironmentVariable {
            id: Uuid::new_v4().to_string(),
            environment_id: "env-1".to_string(),
            key: "API_KEY".to_string(),
            value: encrypted_value.clone(),
            masked_preview: Some(encrypted_value),
            var_type: VariableType::Secret,
            enabled: true,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let error = var.validate_for_storage().unwrap_err();
        assert_eq!(error, "Secret variable preview must not expose stored value");
    }
}
