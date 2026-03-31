use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::contracts::domain::AssertionOperator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiAuthType {
    None,
    Bearer,
    Basic,
    ApiKey,
}

impl ApiAuthType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bearer => "bearer",
            Self::Basic => "basic",
            Self::ApiKey => "api_key",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "none" => Some(Self::None),
            "bearer" => Some(Self::Bearer),
            "basic" => Some(Self::Basic),
            "api_key" => Some(Self::ApiKey),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiAuthConfig {
    pub location: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiEndpoint {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query_params: BTreeMap<String, String>,
    pub body: Option<String>,
    pub auth_type: ApiAuthType,
    pub auth_config: ApiAuthConfig,
    pub timeout_ms: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApiEndpoint {
    pub fn new(name: String, method: String, url: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            collection_id: "default-api-collection".to_string(),
            name,
            method,
            url,
            headers: BTreeMap::new(),
            query_params: BTreeMap::new(),
            body: None,
            auth_type: ApiAuthType::None,
            auth_config: ApiAuthConfig::default(),
            timeout_ms: 30_000,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiAssertion {
    pub id: String,
    pub endpoint_id: String,
    pub name: String,
    pub assertion_type: String,
    pub target: String,
    pub operator: AssertionOperator,
    pub expected_value: String,
    pub enabled: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApiAssertion {
    pub fn new(endpoint_id: String, name: String, operator: AssertionOperator, expected_value: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            endpoint_id,
            name,
            assertion_type: "body_text".to_string(),
            target: "$.".to_string(),
            operator,
            expected_value,
            enabled: true,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }
}
