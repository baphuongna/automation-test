use crate::contracts::domain::AssertionOperator;
use crate::contracts::dto::{
    ApiAssertionDto, ApiAssertionResultDto, ApiAuthDto, ApiExecutionResultDto, ApiRequestDto,
    ApiRequestPreviewDto,
};
use crate::error::TestForgeError;
use crate::models::{ApiAssertion, ApiAuthConfig, ApiAuthType, ApiEndpoint, EnvironmentVariable, VariableType};
use crate::repositories::{ApiRepository, EnvironmentRepository};
use crate::services::SecretService;
use base64::Engine;
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

const MAX_BODY_PREVIEW_BYTES: usize = 2_048;

pub enum ExecutionPersistenceTarget<'a> {
    Standalone {
        environment_id: &'a str,
        test_case_id: &'a str,
    },
    SuiteRun {
        run_id: &'a str,
        environment_id: &'a str,
        test_case_id: &'a str,
        data_row_id: Option<&'a str>,
    },
}

pub struct ApiExecutionService<'a> {
    api_repository: ApiRepository<'a>,
    environment_repository: EnvironmentRepository<'a>,
    secret_service: &'a SecretService,
    client: reqwest::Client,
}

impl<'a> ApiExecutionService<'a> {
    pub fn new(
        api_repository: ApiRepository<'a>,
        environment_repository: EnvironmentRepository<'a>,
        secret_service: &'a SecretService,
    ) -> Self {
        Self {
            api_repository,
            environment_repository,
            secret_service,
            client: reqwest::Client::new(),
        }
    }

    pub fn upsert_test_case(
        &self,
        test_case_id: &str,
        name: &str,
        request: &ApiRequestDto,
        assertions: &[ApiAssertionDto],
    ) -> Result<(), TestForgeError> {
        let _ = self.api_repository.ensure_default_collection()?;

        let mut endpoint = ApiEndpoint::new(name.to_string(), request.method.clone(), request.url.clone());
        endpoint.id = test_case_id.to_string();
        endpoint.headers = request.headers.clone();
        endpoint.query_params = request.query_params.clone();
        endpoint.body = request.body.clone();
        endpoint.auth_type = to_model_auth_type(request.auth.as_ref())?;
        endpoint.auth_config = to_model_auth_config(request.auth.as_ref());

        self.api_repository.upsert_endpoint(&endpoint)?;
        self.api_repository
            .upsert_test_case_link(test_case_id, name, &endpoint.id)?;

        let model_assertions = assertions
            .iter()
            .map(|assertion| {
                validate_assertion_operator(assertion.operator)?;
                let mut model = ApiAssertion::new(
                    test_case_id.to_string(),
                    format!("{:?} ({})", assertion.operator, assertion.id),
                    assertion.operator,
                    assertion.expected_value.clone(),
                );
                model.id = assertion.id.clone();
                model.target = assertion.source_path.clone().unwrap_or_else(|| "$.".to_string());
                model.assertion_type = operator_to_assertion_type(assertion.operator).to_string();
                Ok(model)
            })
            .collect::<Result<Vec<_>, TestForgeError>>()?;

        self.api_repository.replace_assertions(test_case_id, &model_assertions)?;
        Ok(())
    }

    pub fn delete_test_case(&self, id: &str) -> Result<(), TestForgeError> {
        self.api_repository.delete_endpoint(id)
    }

    pub async fn execute(
        &self,
        test_case_id: Option<&str>,
        environment_id: &str,
        request: ApiRequestDto,
        assertions: Vec<ApiAssertionDto>,
    ) -> Result<ApiExecutionResultDto, TestForgeError> {
        let persistence_target = test_case_id.map(|case_id| ExecutionPersistenceTarget::Standalone {
            environment_id,
            test_case_id: case_id,
        });
        self.execute_with_persistence(environment_id, request, assertions, persistence_target)
            .await
    }

    pub async fn execute_for_suite_run(
        &self,
        run_id: &str,
        environment_id: &str,
        test_case_id: &str,
        data_row_id: Option<&str>,
    ) -> Result<ApiExecutionResultDto, TestForgeError> {
        let endpoint = self.api_repository.find_endpoint(test_case_id)?;
        let assertions = self
            .api_repository
            .find_assertions(test_case_id)?
            .into_iter()
            .map(|assertion| ApiAssertionDto {
                id: assertion.id,
                operator: assertion.operator,
                expected_value: assertion.expected_value,
                source_path: Some(assertion.target),
            })
            .collect::<Vec<_>>();

        let request = ApiRequestDto {
            method: endpoint.method,
            url: endpoint.url,
            headers: endpoint.headers,
            query_params: endpoint.query_params,
            body: endpoint.body,
            auth: None,
        };

        self.execute_with_persistence(
            environment_id,
            request,
            assertions,
            Some(ExecutionPersistenceTarget::SuiteRun {
                run_id,
                environment_id,
                test_case_id,
                data_row_id,
            }),
        )
        .await
    }

    async fn execute_with_persistence(
        &self,
        environment_id: &str,
        request: ApiRequestDto,
        assertions: Vec<ApiAssertionDto>,
        persistence_target: Option<ExecutionPersistenceTarget<'_>>,
    ) -> Result<ApiExecutionResultDto, TestForgeError> {
        for assertion in &assertions {
            validate_assertion_operator(assertion.operator)?;
        }

        let resolved = self.resolve_request(environment_id, &request)?;
        let request_preview = build_request_preview(&resolved);

        let start = Instant::now();
        let method = parse_method(&resolved.method)?;
        let mut builder = self.client.request(method, &resolved.url);

        for (key, value) in &resolved.headers {
            builder = builder.header(key, value);
        }

        if !resolved.query_params.is_empty() {
            builder = builder.query(&resolved.query_params);
        }

        if let Some(body) = &resolved.body {
            builder = builder.body(body.clone());
        }

        match builder.send().await {
            Ok(response) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let status_code = response.status().as_u16();
                let headers = normalize_headers(response.headers());
                let response_body = response.text().await.unwrap_or_default();
                let body_preview = normalize_body_preview(&response_body);

                let assertion_results = evaluate_assertions(status_code, &headers, &response_body, &assertions);
                let has_failed_assertions = assertion_results.iter().any(|item| !item.passed);

                let result = ApiExecutionResultDto {
                    status: if has_failed_assertions {
                        "failed".to_string()
                    } else {
                        "passed".to_string()
                    },
                    transport_success: true,
                    failure_kind: if has_failed_assertions {
                        Some("assertion".to_string())
                    } else {
                        None
                    },
                    error_code: if has_failed_assertions {
                        Some("API_ASSERTION_FAILED".to_string())
                    } else {
                        None
                    },
                    error_message: if has_failed_assertions {
                        Some("One or more assertions failed".to_string())
                    } else {
                        None
                    },
                    status_code: Some(status_code),
                    duration_ms,
                    body_preview: body_preview.clone(),
                    response_headers: headers.clone(),
                    assertions: assertion_results.clone(),
                    request_preview: request_preview.clone(),
                };

                if let Some(target) = persistence_target.as_ref() {
                    self.persist_result(target, &result)?;
                }

                Ok(result)
            }
            Err(error) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let error_code = if error.is_timeout() {
                    "API_TIMEOUT"
                } else {
                    "API_REQUEST_FAILED"
                };

                let result = ApiExecutionResultDto {
                    status: "failed".to_string(),
                    transport_success: false,
                    failure_kind: Some("transport".to_string()),
                    error_code: Some(error_code.to_string()),
                    error_message: Some(error.to_string()),
                    status_code: None,
                    duration_ms,
                    body_preview: String::new(),
                    response_headers: BTreeMap::new(),
                    assertions: Vec::new(),
                    request_preview: request_preview.clone(),
                };

                if let Some(target) = persistence_target.as_ref() {
                    self.persist_result(target, &result)?;
                }

                Ok(result)
            }
        }
    }

    fn persist_result(
        &self,
        target: &ExecutionPersistenceTarget<'_>,
        result: &ApiExecutionResultDto,
    ) -> Result<(), TestForgeError> {
        let request_log_json = serde_json::to_string(&result.request_preview)?;
        let response_log_json = serde_json::to_string(&serde_json::json!({
            "statusCode": result.status_code,
            "headers": result.response_headers,
            "bodyPreview": result.body_preview,
            "transportSuccess": result.transport_success,
        }))?;
        let assertion_results_json = serde_json::to_string(&result.assertions)?;

        match target {
            ExecutionPersistenceTarget::Standalone {
                environment_id,
                test_case_id,
            } => self.api_repository.insert_run_result(
                environment_id,
                test_case_id,
                &result.status,
                &request_log_json,
                &response_log_json,
                &assertion_results_json,
                result.error_message.as_deref(),
                result.error_code.as_deref(),
                result.duration_ms,
            ),
            ExecutionPersistenceTarget::SuiteRun {
                run_id,
                environment_id,
                test_case_id,
                data_row_id,
            } => self.api_repository.insert_suite_run_result(
                run_id,
                environment_id,
                test_case_id,
                *data_row_id,
                &result.status,
                &request_log_json,
                &response_log_json,
                &assertion_results_json,
                result.error_message.as_deref(),
                result.error_code.as_deref(),
                result.duration_ms,
            ),
        }
    }

    fn resolve_request(&self, environment_id: &str, request: &ApiRequestDto) -> Result<ApiRequestDto, TestForgeError> {
        let environment = self.environment_repository.find_by_id(environment_id)?;
        let variables = self
            .environment_repository
            .find_variables_by_environment(&environment.id)?
            .into_iter()
            .filter(|item| item.enabled)
            .collect::<Vec<_>>();

        let mut resolved = request.clone();
        resolved.url = resolve_with_variables(&request.url, &variables, self.secret_service)?;

        let mut headers = BTreeMap::new();
        for (key, value) in &request.headers {
            headers.insert(key.clone(), resolve_with_variables(value, &variables, self.secret_service)?);
        }
        resolved.headers = headers;

        let mut query_params = BTreeMap::new();
        for (key, value) in &request.query_params {
            query_params.insert(key.clone(), resolve_with_variables(value, &variables, self.secret_service)?);
        }
        resolved.query_params = query_params;

        if let Some(body) = &request.body {
            resolved.body = Some(resolve_with_variables(body, &variables, self.secret_service)?);
        }

        resolved.auth = resolve_auth(request.auth.as_ref(), &variables, self.secret_service)?;

        apply_supported_auth(&mut resolved)?;
        Ok(resolved)
    }
}

fn normalize_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, String> {
    let mut normalized = BTreeMap::new();
    for (key, value) in headers {
        normalized.insert(
            key.to_string().to_lowercase(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }
    normalized
}

fn normalize_body_preview(body: &str) -> String {
    let chars = body.chars().collect::<Vec<_>>();
    if chars.len() > MAX_BODY_PREVIEW_BYTES {
        format!(
            "{}...(truncated)",
            chars[..MAX_BODY_PREVIEW_BYTES].iter().collect::<String>()
        )
    } else {
        body.to_string()
    }
}

fn resolve_auth(
    auth: Option<&ApiAuthDto>,
    variables: &[EnvironmentVariable],
    secret_service: &SecretService,
) -> Result<Option<ApiAuthDto>, TestForgeError> {
    let Some(current) = auth else {
        return Ok(None);
    };

    let mut resolved = current.clone();
    if let Some(value) = &current.value {
        resolved.value = Some(resolve_with_variables(value, variables, secret_service)?);
    }
    if let Some(value) = &current.token {
        resolved.token = Some(resolve_with_variables(value, variables, secret_service)?);
    }
    if let Some(value) = &current.username {
        resolved.username = Some(resolve_with_variables(value, variables, secret_service)?);
    }
    if let Some(value) = &current.password {
        resolved.password = Some(resolve_with_variables(value, variables, secret_service)?);
    }

    Ok(Some(resolved))
}

fn apply_supported_auth(request: &mut ApiRequestDto) -> Result<(), TestForgeError> {
    let Some(auth) = request.auth.clone() else {
        return Ok(());
    };

    match auth.r#type.as_str() {
        "none" => Ok(()),
        "bearer" => {
            let token = auth
                .token
                .ok_or_else(|| TestForgeError::Validation("Bearer auth requires token".to_string()))?;
            request
                .headers
                .insert("Authorization".to_string(), format!("Bearer {token}"));
            Ok(())
        }
        "basic" => {
            let username = auth
                .username
                .ok_or_else(|| TestForgeError::Validation("Basic auth requires username".to_string()))?;
            let password = auth
                .password
                .ok_or_else(|| TestForgeError::Validation("Basic auth requires password".to_string()))?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            request
                .headers
                .insert("Authorization".to_string(), format!("Basic {encoded}"));
            Ok(())
        }
        "api_key" => {
            let key = auth
                .key
                .ok_or_else(|| TestForgeError::Validation("API key auth requires key".to_string()))?;
            let value = auth
                .value
                .ok_or_else(|| TestForgeError::Validation("API key auth requires value".to_string()))?;
            match auth.location.as_deref().unwrap_or("header") {
                "query" => {
                    request.query_params.insert(key, value);
                    Ok(())
                }
                "header" => {
                    request.headers.insert(key, value);
                    Ok(())
                }
                _ => Err(TestForgeError::Validation(
                    "API key auth location must be header or query".to_string(),
                )),
            }
        }
        unsupported => Err(TestForgeError::Validation(format!(
            "Unsupported auth type: {unsupported}"
        ))),
    }
}

fn build_request_preview(request: &ApiRequestDto) -> ApiRequestPreviewDto {
    let mut headers = request.headers.clone();
    redact_sensitive_map(&mut headers);

    let mut query_params = request.query_params.clone();
    redact_sensitive_map(&mut query_params);

    ApiRequestPreviewDto {
        method: request.method.clone(),
        url: request.url.clone(),
        headers,
        query_params,
        body_preview: request.body.as_deref().map(normalize_body_preview),
        auth_preview: auth_preview(request.auth.as_ref()),
    }
}

fn auth_preview(auth: Option<&ApiAuthDto>) -> String {
    match auth {
        None => "none".to_string(),
        Some(value) => match value.r#type.as_str() {
            "none" => "none".to_string(),
            "bearer" => "bearer [REDACTED]".to_string(),
            "basic" => "basic [REDACTED]".to_string(),
            "api_key" => "api_key [REDACTED]".to_string(),
            _ => "unsupported".to_string(),
        },
    }
}

fn redact_sensitive_map(values: &mut BTreeMap<String, String>) {
    let sensitive_keys = ["authorization", "x-api-key", "api-key", "token", "password", "secret"];
    let keys = values.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        if sensitive_keys
            .iter()
            .any(|sensitive| key.to_lowercase().contains(sensitive))
        {
            values.insert(key, "[REDACTED]".to_string());
        }
    }
}

fn resolve_with_variables(
    template: &str,
    variables: &[EnvironmentVariable],
    secret_service: &SecretService,
) -> Result<String, TestForgeError> {
    let mut result = template.to_string();
    let mut seen = HashSet::new();

    for _ in 0..10 {
        let placeholders = collect_placeholders(&result);
        if placeholders.is_empty() {
            return Ok(result);
        }

        for key in placeholders {
            let variable = variables.iter().find(|item| item.key == key).ok_or_else(|| {
                TestForgeError::Validation(format!("API_REQUEST_BUILD_FAILED: missing variable '{key}'"))
            })?;

            if !seen.insert(key.clone()) {
                return Err(TestForgeError::Validation(
                    "API_REQUEST_BUILD_FAILED: variable resolution appears circular".to_string(),
                ));
            }

            let replacement = if variable.var_type == VariableType::Secret {
                secret_service.decrypt(&variable.value)?
            } else {
                variable.value.clone()
            };

            result = result.replace(&format!("{{{{{key}}}}}"), &replacement);
        }
    }

    Err(TestForgeError::Validation(
        "API_REQUEST_BUILD_FAILED: variable resolution exceeded depth limit".to_string(),
    ))
}

fn collect_placeholders(input: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut start_index = 0usize;

    while let Some(open_rel) = input[start_index..].find("{{") {
        let open = start_index + open_rel;
        let close_search_start = open + 2;
        if let Some(close_rel) = input[close_search_start..].find("}}") {
            let close = close_search_start + close_rel;
            let name = input[close_search_start..close].trim();
            if !name.is_empty() {
                placeholders.push(name.to_string());
            }
            start_index = close + 2;
        } else {
            break;
        }
    }

    placeholders
}

fn parse_method(value: &str) -> Result<reqwest::Method, TestForgeError> {
    reqwest::Method::from_bytes(value.as_bytes())
        .map_err(|_| TestForgeError::Validation(format!("Unsupported HTTP method: {value}")))
}

fn evaluate_assertions(
    status_code: u16,
    headers: &BTreeMap<String, String>,
    body: &str,
    assertions: &[ApiAssertionDto],
) -> Vec<ApiAssertionResultDto> {
    assertions
        .iter()
        .map(|assertion| evaluate_assertion(status_code, headers, body, assertion))
        .collect()
}

fn evaluate_assertion(
    status_code: u16,
    headers: &BTreeMap<String, String>,
    body: &str,
    assertion: &ApiAssertionDto,
) -> ApiAssertionResultDto {
    let actual = match assertion.operator {
        AssertionOperator::StatusEquals => status_code.to_string(),
        AssertionOperator::BodyContains => body.to_string(),
        AssertionOperator::HeaderEquals => {
            let key = assertion.source_path.as_deref().unwrap_or_default().to_lowercase();
            headers.get(&key).cloned().unwrap_or_default()
        }
        AssertionOperator::JsonPathExists => {
            if let Some(path) = assertion.source_path.as_deref() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    if resolve_json_path(&json, path).is_some() {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    }
                } else {
                    "false".to_string()
                }
            } else {
                "false".to_string()
            }
        }
        AssertionOperator::JsonPathEquals => {
            if let Some(path) = assertion.source_path.as_deref() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    resolve_json_path(&json, path)
                        .map(|value| match value {
                            serde_json::Value::String(text) => text,
                            _ => value.to_string(),
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
    };

    let passed = match assertion.operator {
        AssertionOperator::StatusEquals | AssertionOperator::JsonPathEquals | AssertionOperator::HeaderEquals => {
            actual == assertion.expected_value
        }
        AssertionOperator::BodyContains => actual.contains(&assertion.expected_value),
        AssertionOperator::JsonPathExists => actual == "true",
    };

    ApiAssertionResultDto {
        assertion_id: assertion.id.clone(),
        operator: assertion.operator,
        passed,
        expected_value: assertion.expected_value.clone(),
        actual_value: Some(actual),
        source_path: assertion.source_path.clone(),
        error_code: if passed {
            None
        } else {
            Some("API_ASSERTION_FAILED".to_string())
        },
        message: if passed {
            None
        } else {
            Some("Assertion failed".to_string())
        },
    }
}

fn resolve_json_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let trimmed = path.trim();
    if !trimmed.starts_with("$.") {
        return None;
    }

    let mut current = root;
    for segment in trimmed[2..].split('.') {
        if let serde_json::Value::Object(map) = current {
            current = map.get(segment)?;
        } else {
            return None;
        }
    }

    Some(current.clone())
}

fn validate_assertion_operator(operator: AssertionOperator) -> Result<(), TestForgeError> {
    match operator {
        AssertionOperator::StatusEquals
        | AssertionOperator::JsonPathExists
        | AssertionOperator::JsonPathEquals
        | AssertionOperator::BodyContains
        | AssertionOperator::HeaderEquals => Ok(()),
    }
}

fn operator_to_assertion_type(operator: AssertionOperator) -> &'static str {
    match operator {
        AssertionOperator::StatusEquals => "status",
        AssertionOperator::JsonPathExists | AssertionOperator::JsonPathEquals => "body_json",
        AssertionOperator::BodyContains => "body_text",
        AssertionOperator::HeaderEquals => "header",
    }
}

fn to_model_auth_type(auth: Option<&ApiAuthDto>) -> Result<ApiAuthType, TestForgeError> {
    let Some(value) = auth else {
        return Ok(ApiAuthType::None);
    };

    ApiAuthType::from_str(&value.r#type).ok_or_else(|| {
        TestForgeError::Validation(format!("Unsupported auth type: {}", value.r#type))
    })
}

fn to_model_auth_config(auth: Option<&ApiAuthDto>) -> ApiAuthConfig {
    let Some(value) = auth else {
        return ApiAuthConfig::default();
    };

    ApiAuthConfig {
        location: value.location.clone(),
        key: value.key.clone(),
        value: value.value.clone(),
        token: value.token.clone(),
        username: value.username.clone(),
        password: value.password.clone(),
    }
}
