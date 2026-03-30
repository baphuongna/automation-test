//! Environment model
//! 
//! Represents a test environment (e.g., dev, staging, production)
//! containing environment variables for configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Environment type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentType {
    Development,
    Staging,
    Production,
    Custom,
}

impl Default for EnvironmentType {
    fn default() -> Self {
        Self::Development
    }
}

impl EnvironmentType {
    /// Check if this environment type requires warning before destructive actions
    pub fn requires_warning(&self) -> bool {
        matches!(self, Self::Production)
    }

    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
            Self::Custom => "custom",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Some(Self::Development),
            "staging" | "stage" => Some(Self::Staging),
            "production" | "prod" => Some(Self::Production),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Environment model representing a test environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Unique identifier
    pub id: String,
    /// Environment name
    pub name: String,
    /// Environment description
    #[serde(default)]
    pub description: Option<String>,
    /// Environment type
    #[serde(default)]
    pub env_type: EnvironmentType,
    /// Whether this is the default environment
    #[serde(default)]
    pub is_default: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Environment {
    /// Create a new environment with the given name
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            env_type: EnvironmentType::default(),
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new environment with all fields
    pub fn with_details(
        name: String,
        description: Option<String>,
        env_type: EnvironmentType,
        is_default: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            env_type,
            is_default,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.updated_at = Utc::now();
        self
    }

    /// Set as default environment
    pub fn set_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self.updated_at = Utc::now();
        self
    }

    /// Check if this environment requires warning
    pub fn requires_warning(&self) -> bool {
        self.env_type.requires_warning()
    }

    /// Validate persistence constraints for the environments table.
    pub fn validate_for_storage(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Environment name cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_creation() {
        let env = Environment::new("Test Environment".to_string());
        assert!(!env.id.is_empty());
        assert_eq!(env.name, "Test Environment");
        assert_eq!(env.env_type, EnvironmentType::Development);
        assert!(!env.is_default);
    }

    #[test]
    fn test_environment_type_warning() {
        assert!(!EnvironmentType::Development.requires_warning());
        assert!(!EnvironmentType::Staging.requires_warning());
        assert!(EnvironmentType::Production.requires_warning());
        assert!(!EnvironmentType::Custom.requires_warning());
    }

    #[test]
    fn test_environment_type_parse() {
        assert_eq!(
            EnvironmentType::from_str("production"),
            Some(EnvironmentType::Production)
        );
        assert_eq!(
            EnvironmentType::from_str("PROD"),
            Some(EnvironmentType::Production)
        );
        assert_eq!(EnvironmentType::from_str("invalid"), None);
    }

    #[test]
    fn test_environment_serialization() {
        let env = Environment::new("Test".to_string());
        let json = serde_json::to_string(&env).unwrap();
        let deserialized: Environment = serde_json::from_str(&json).unwrap();
        assert_eq!(env.id, deserialized.id);
        assert_eq!(env.name, deserialized.name);
    }

    #[test]
    fn test_environment_requires_non_empty_name() {
        let env = Environment::new("   ".to_string());
        let error = env.validate_for_storage().unwrap_err();
        assert_eq!(error, "Environment name cannot be empty");
    }
}
