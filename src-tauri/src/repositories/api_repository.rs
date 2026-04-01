use crate::contracts::domain::AssertionOperator;
use crate::error::{Result, TestForgeError};
use crate::models::{ApiAssertion, ApiAuthConfig, ApiAuthType, ApiEndpoint};
use chrono::Utc;
use rusqlite::{params, Connection};

pub struct ApiRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ApiRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn ensure_default_collection(&self) -> Result<String> {
        let default_id = "default-api-collection".to_string();
        self.conn.execute(
            "INSERT OR IGNORE INTO api_collections (id, name, description, parent_id, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, NULL, 0, ?4, ?5)",
            params![
                default_id,
                "Default Collection",
                "Auto-created by API engine",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;

        Ok("default-api-collection".to_string())
    }

    pub fn upsert_endpoint(&self, endpoint: &ApiEndpoint) -> Result<()> {
        let headers_json = serde_json::to_string(&endpoint.headers)?;
        let query_params_json = serde_json::to_string(&endpoint.query_params)?;
        let auth_config_json = serde_json::to_string(&endpoint.auth_config)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO api_endpoints (id, collection_id, name, description, method, url, headers_json, query_params_json, body_type, body_json, auth_type, auth_config_json, timeout_ms, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, 'raw', ?8, ?9, ?10, ?11, 0, ?12, ?13)",
            params![
                endpoint.id,
                endpoint.collection_id,
                endpoint.name,
                endpoint.method,
                endpoint.url,
                headers_json,
                query_params_json,
                endpoint.body,
                endpoint.auth_type.as_str(),
                auth_config_json,
                endpoint.timeout_ms,
                endpoint.created_at.to_rfc3339(),
                endpoint.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn upsert_test_case_link(
        &self,
        test_case_id: &str,
        name: &str,
        endpoint_id: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO test_cases (id, name, description, case_type, api_endpoint_id, ui_script_id, data_table_id, tags_json, enabled, created_at, updated_at) VALUES (?1, ?2, NULL, 'api', ?3, NULL, NULL, '[]', 1, COALESCE((SELECT created_at FROM test_cases WHERE id = ?1), ?4), ?5)",
            params![test_case_id, name, endpoint_id, now, now],
        )?;

        Ok(())
    }

    pub fn replace_assertions(&self, endpoint_id: &str, assertions: &[ApiAssertion]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM assertions WHERE endpoint_id = ?1",
            params![endpoint_id],
        )?;

        for (index, assertion) in assertions.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO assertions (id, endpoint_id, name, assertion_type, target, operator, expected_value, enabled, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    assertion.id,
                    endpoint_id,
                    assertion.name,
                    assertion.assertion_type,
                    assertion.target,
                    assertion_operator_to_str(assertion.operator),
                    assertion.expected_value,
                    assertion.enabled,
                    index as i32,
                    assertion.created_at.to_rfc3339(),
                    assertion.updated_at.to_rfc3339(),
                ],
            )?;
        }

        Ok(())
    }

    pub fn delete_endpoint(&self, endpoint_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM test_cases WHERE api_endpoint_id = ?1",
            params![endpoint_id],
        )?;

        let affected = self.conn.execute(
            "DELETE FROM api_endpoints WHERE id = ?1",
            params![endpoint_id],
        )?;

        if affected == 0 {
            return Err(TestForgeError::EndpointNotFound {
                id: endpoint_id.to_string(),
            });
        }

        Ok(())
    }

    pub fn find_endpoint(&self, endpoint_id: &str) -> Result<ApiEndpoint> {
        let endpoint = self.conn.query_row(
            "SELECT id, collection_id, name, method, url, headers_json, query_params_json, body_json, auth_type, auth_config_json, timeout_ms, created_at, updated_at FROM api_endpoints WHERE id = ?1",
            params![endpoint_id],
            |row| {
                let headers_json: String = row.get(5)?;
                let query_params_json: String = row.get(6)?;
                let auth_config_json: String = row.get(9)?;
                Ok(ApiEndpoint {
                    id: row.get(0)?,
                    collection_id: row.get(1)?,
                    name: row.get(2)?,
                    method: row.get(3)?,
                    url: row.get(4)?,
                    headers: serde_json::from_str(&headers_json).unwrap_or_default(),
                    query_params: serde_json::from_str(&query_params_json).unwrap_or_default(),
                    body: row.get(7)?,
                    auth_type: ApiAuthType::from_str(&row.get::<_, String>(8)?)
                        .unwrap_or(ApiAuthType::None),
                    auth_config: serde_json::from_str::<ApiAuthConfig>(&auth_config_json).unwrap_or_default(),
                    timeout_ms: row.get(10)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            },
        )?;

        Ok(endpoint)
    }

    pub fn find_assertions(&self, endpoint_id: &str) -> Result<Vec<ApiAssertion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, endpoint_id, name, assertion_type, target, operator, expected_value, enabled, sort_order, created_at, updated_at FROM assertions WHERE endpoint_id = ?1 ORDER BY sort_order ASC",
        )?;

        let assertions = stmt
            .query_map(params![endpoint_id], |row| {
                Ok(ApiAssertion {
                    id: row.get(0)?,
                    endpoint_id: row.get(1)?,
                    name: row.get(2)?,
                    assertion_type: row.get(3)?,
                    target: row.get(4)?,
                    operator: assertion_operator_from_str(&row.get::<_, String>(5)?),
                    expected_value: row.get(6)?,
                    enabled: row.get(7)?,
                    sort_order: row.get(8)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(assertions)
    }

    pub fn insert_run_result(
        &self,
        environment_id: &str,
        test_case_id: &str,
        status: &str,
        request_log_json: &str,
        response_log_json: &str,
        assertion_results_json: &str,
        error_message: Option<&str>,
        error_code: Option<&str>,
        duration_ms: u64,
    ) -> Result<()> {
        let run_id = format!("run-{}", uuid::Uuid::new_v4());
        let run_result_id = format!("run-result-{}", uuid::Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO test_runs (id, suite_id, environment_id, status, total_cases, passed, failed, skipped, started_at, completed_at, duration_ms, created_at) VALUES (?1, NULL, ?2, ?3, 1, ?4, ?5, 0, ?6, ?7, ?8, ?9)",
            params![
                run_id,
                environment_id,
                status,
                if status == "passed" { 1 } else { 0 },
                if status == "failed" { 1 } else { 0 },
                now,
                now,
                duration_ms as i64,
                now,
            ],
        )?;

        self.conn.execute(
            "INSERT INTO test_run_results (id, run_id, case_id, status, duration_ms, request_log_json, response_log_json, assertion_results_json, screenshots_json, error_message, error_code, started_at, completed_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '[]', ?9, ?10, ?11, ?12, ?13)",
            params![
                run_result_id,
                run_id,
                test_case_id,
                status,
                duration_ms as i64,
                request_log_json,
                response_log_json,
                assertion_results_json,
                error_message,
                error_code,
                now,
                now,
                now,
            ],
        )?;

        Ok(())
    }

    pub fn insert_suite_run_result(
        &self,
        run_id: &str,
        _environment_id: &str,
        test_case_id: &str,
        data_row_id: Option<&str>,
        status: &str,
        request_log_json: &str,
        response_log_json: &str,
        assertion_results_json: &str,
        error_message: Option<&str>,
        error_code: Option<&str>,
        duration_ms: u64,
    ) -> Result<()> {
        let run_result_id = format!("run-result-{}", uuid::Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO test_run_results (id, run_id, case_id, data_row_id, status, duration_ms, request_log_json, response_log_json, assertion_results_json, screenshots_json, error_message, error_code, started_at, completed_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, '[]', ?10, ?11, ?12, ?12, ?12)",
            params![
                run_result_id,
                run_id,
                test_case_id,
                data_row_id,
                status,
                duration_ms as i64,
                request_log_json,
                response_log_json,
                assertion_results_json,
                error_message,
                error_code,
                now,
            ],
        )?;

        Ok(())
    }
}

fn assertion_operator_to_str(operator: AssertionOperator) -> &'static str {
    match operator {
        AssertionOperator::StatusEquals => "status_equals",
        AssertionOperator::JsonPathExists => "json_path_exists",
        AssertionOperator::JsonPathEquals => "json_path_equals",
        AssertionOperator::BodyContains => "body_contains",
        AssertionOperator::HeaderEquals => "header_equals",
    }
}

fn assertion_operator_from_str(value: &str) -> AssertionOperator {
    match value {
        "status_equals" => AssertionOperator::StatusEquals,
        "json_path_exists" => AssertionOperator::JsonPathExists,
        "json_path_equals" => AssertionOperator::JsonPathEquals,
        "header_equals" => AssertionOperator::HeaderEquals,
        _ => AssertionOperator::BodyContains,
    }
}
