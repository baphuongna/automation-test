use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::contracts::domain::{StepAction, TestCaseType};
use crate::contracts::dto::{UiStepDto, UiTestCaseDto};
use crate::error::{Result, TestForgeError};

#[derive(Debug, Clone)]
pub struct PersistedUiScriptStepInput {
    pub action: StepAction,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub timeout_ms: u64,
    pub description: String,
    pub confidence: &'static str,
}

#[derive(Debug, Clone)]
pub struct PersistedReplayStep {
    pub id: String,
    pub action: StepAction,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PersistedReplayScript {
    pub start_url: String,
    pub steps: Vec<PersistedReplayStep>,
}

pub struct UiScriptRepository<'a> {
    conn: &'a Connection,
}

impl<'a> UiScriptRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn find_ui_test_case_by_id(&self, test_case_id: &str) -> Result<UiTestCaseDto> {
        let row = self
            .conn
            .query_row(
                "SELECT tc.id, tc.name, us.start_url, tc.case_type, tc.ui_script_id
                 FROM test_cases tc
                 LEFT JOIN ui_scripts us ON us.id = tc.ui_script_id
                 WHERE tc.id = ?1",
                params![test_case_id],
                |record| {
                    Ok((
                        record.get::<_, String>(0)?,
                        record.get::<_, String>(1)?,
                        record.get::<_, Option<String>>(2)?,
                        record.get::<_, String>(3)?,
                        record.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| {
                TestForgeError::Validation(format!("UI test case not found: {test_case_id}"))
            })?;

        if row.3 != "ui" {
            return Err(TestForgeError::Validation(format!(
                "ui.testcase.get requires test case type 'ui': {test_case_id}"
            )));
        }

        if row.4.as_deref() != Some(test_case_id) {
            return Err(TestForgeError::Validation(
                "Ui test case không còn script persisted hợp lệ.".to_string(),
            ));
        }

        let replay_script = self.load_replay_script_by_test_case_id(test_case_id)?;
        let steps = replay_script
            .steps
            .into_iter()
            .map(|step| UiStepDto {
                id: step.id,
                action: step.action,
                selector: step.selector,
                value: step.value,
                timeout_ms: step.timeout_ms,
                confidence: None,
            })
            .collect::<Vec<_>>();

        Ok(UiTestCaseDto {
            id: row.0,
            r#type: TestCaseType::Ui,
            name: row.1,
            start_url: row.2.unwrap_or_else(|| replay_script.start_url),
            steps,
        })
    }

    pub fn persist_recording_snapshot(
        &self,
        test_case_id: &str,
        start_url: &str,
        description: Option<&str>,
        script_name: &str,
        viewport_width: u32,
        viewport_height: u32,
        timeout_ms: u32,
        normalized_steps: &[PersistedUiScriptStepInput],
    ) -> Result<UiTestCaseDto> {
        self.persist_ui_test_case_internal(
            test_case_id,
            script_name,
            start_url,
            description,
            viewport_width,
            viewport_height,
            timeout_ms,
            normalized_steps,
        )
    }

    pub fn upsert_ui_test_case(
        &self,
        test_case: &UiTestCaseDto,
        viewport_width: u32,
        viewport_height: u32,
        timeout_ms: u32,
    ) -> Result<UiTestCaseDto> {
        if !test_case.validate_type() {
            return Err(TestForgeError::Validation(
                "ui.testcase.upsert requires test case type 'ui'".to_string(),
            ));
        }

        let normalized_steps = test_case
            .steps
            .iter()
            .map(|step| PersistedUiScriptStepInput {
                action: step.action,
                selector: step
                    .selector
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                value: step
                    .value
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                timeout_ms: step
                    .timeout_ms
                    .unwrap_or(timeout_ms as u64)
                    .clamp(200, 120_000),
                description: build_step_description(
                    step.action,
                    step.selector.as_deref(),
                    step.value.as_deref(),
                ),
                confidence: compute_step_confidence(
                    step.action,
                    step.selector.as_deref(),
                    step.value.as_deref(),
                ),
            })
            .collect::<Vec<_>>();

        self.persist_ui_test_case_internal(
            &test_case.id,
            &test_case.name,
            &test_case.start_url,
            None,
            viewport_width,
            viewport_height,
            timeout_ms,
            &normalized_steps,
        )
    }

    fn persist_ui_test_case_internal(
        &self,
        test_case_id: &str,
        script_name: &str,
        start_url: &str,
        description: Option<&str>,
        viewport_width: u32,
        viewport_height: u32,
        timeout_ms: u32,
        normalized_steps: &[PersistedUiScriptStepInput],
    ) -> Result<UiTestCaseDto> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO ui_scripts (id, name, description, start_url, viewport_width, viewport_height, timeout_ms, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, COALESCE((SELECT created_at FROM ui_scripts WHERE id = ?1), ?8), ?9)",
            params![
                test_case_id,
                script_name,
                description,
                start_url,
                viewport_width as i64,
                viewport_height as i64,
                timeout_ms as i64,
                now,
                now,
            ],
        )?;

        self.conn.execute(
            "INSERT OR REPLACE INTO test_cases (id, name, description, case_type, api_endpoint_id, ui_script_id, data_table_id, tags_json, enabled, created_at, updated_at) VALUES (?1, COALESCE((SELECT name FROM test_cases WHERE id = ?1), ?2), NULL, 'ui', NULL, ?3, NULL, '[]', 1, COALESCE((SELECT created_at FROM test_cases WHERE id = ?1), ?4), ?5)",
            params![test_case_id, script_name, test_case_id, now, now],
        )?;

        self.conn.execute(
            "DELETE FROM ui_script_steps WHERE script_id = ?1",
            params![test_case_id],
        )?;

        let mut persisted_steps = Vec::with_capacity(normalized_steps.len());
        for (index, step) in normalized_steps.iter().enumerate() {
            let step_id = format!("step-{}", Uuid::new_v4());
            self.conn.execute(
                "INSERT INTO ui_script_steps (id, script_id, step_order, step_type, selector, value, timeout_ms, description, confidence, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    step_id,
                    test_case_id,
                    index as i64,
                    step_action_to_storage(step.action),
                    step.selector,
                    step.value,
                    step.timeout_ms as i64,
                    step.description,
                    step.confidence,
                    now,
                    now,
                ],
            )?;
            persisted_steps.push(UiStepDto {
                id: step_id,
                action: step.action,
                selector: step.selector.clone(),
                value: step.value.clone(),
                timeout_ms: Some(step.timeout_ms),
                confidence: None,
            });
        }

        Ok(UiTestCaseDto {
            id: test_case_id.to_string(),
            r#type: TestCaseType::Ui,
            name: script_name.to_string(),
            start_url: start_url.to_string(),
            steps: persisted_steps,
        })
    }

    pub fn load_replay_script_by_test_case_id(
        &self,
        test_case_id: &str,
    ) -> Result<PersistedReplayScript> {
        let start_url = self
            .conn
            .query_row(
                "SELECT start_url FROM ui_scripts WHERE id = ?1",
                params![test_case_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .unwrap_or_default();

        let linked_script_id = self
            .conn
            .query_row(
                "SELECT ui_script_id FROM test_cases WHERE id = ?1",
                params![test_case_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or_else(|| {
                TestForgeError::Validation(format!("UI test case not found: {test_case_id}"))
            })?;

        if linked_script_id.as_deref() != Some(test_case_id) {
            return Err(TestForgeError::Validation(
                "Ui test case không còn script replay hợp lệ. Có thể script tham chiếu đã bị xóa hoặc không còn đồng bộ.".to_string(),
            ));
        }

        let mut statement = self.conn.prepare(
            "SELECT id, step_type, selector, value, timeout_ms FROM ui_script_steps WHERE script_id = ?1 ORDER BY step_order ASC",
        )?;
        let mut rows = statement.query(params![test_case_id])?;

        let mut steps = Vec::new();
        while let Some(row) = rows.next()? {
            let step_type: String = row.get(1)?;
            let action = storage_to_step_action(&step_type).ok_or_else(|| {
                TestForgeError::Validation(format!("Unsupported replay step_type: {step_type}"))
            })?;

            steps.push(PersistedReplayStep {
                id: row.get(0)?,
                action,
                selector: row.get(2)?,
                value: row.get(3)?,
                timeout_ms: row
                    .get::<_, Option<i64>>(4)?
                    .map(|value| value.max(0) as u64),
            });
        }

        if steps.is_empty() {
            return Err(TestForgeError::Validation(
                "Ui test case không còn step replay khả dụng. Có thể script tham chiếu đã bị xóa hoặc rỗng.".to_string(),
            ));
        }

        Ok(PersistedReplayScript { start_url, steps })
    }

    pub fn resolve_script_name(&self, test_case_id: &str) -> Result<String> {
        let result = self.conn.query_row(
            "SELECT name FROM test_cases WHERE id = ?1",
            params![test_case_id],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(name) if !name.trim().is_empty() => Ok(name),
            _ => Ok(format!("UI Script {test_case_id}")),
        }
    }

    pub fn delete_test_case_and_script(&self, test_case_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM ui_script_steps WHERE script_id = ?1",
            params![test_case_id],
        )?;
        self.conn.execute(
            "DELETE FROM ui_scripts WHERE id = ?1",
            params![test_case_id],
        )?;

        let affected = self.conn.execute(
            "DELETE FROM test_cases WHERE id = ?1 AND case_type = 'ui'",
            params![test_case_id],
        )?;

        if affected == 0 {
            return Err(TestForgeError::Validation(format!(
                "UI test case not found: {test_case_id}"
            )));
        }

        Ok(())
    }
}

fn step_action_to_storage(action: StepAction) -> &'static str {
    match action {
        StepAction::Navigate => "navigate",
        StepAction::Click => "click",
        StepAction::Fill => "fill",
        StepAction::Select => "select",
        StepAction::Check => "check",
        StepAction::Uncheck => "uncheck",
        StepAction::WaitFor => "wait_for",
        StepAction::AssertText => "assert_text",
    }
}

fn storage_to_step_action(value: &str) -> Option<StepAction> {
    match value {
        "navigate" => Some(StepAction::Navigate),
        "click" => Some(StepAction::Click),
        "fill" => Some(StepAction::Fill),
        "select" => Some(StepAction::Select),
        "check" => Some(StepAction::Check),
        "uncheck" => Some(StepAction::Uncheck),
        "wait_for" => Some(StepAction::WaitFor),
        "assert_text" => Some(StepAction::AssertText),
        _ => None,
    }
}

fn compute_step_confidence(
    action: StepAction,
    selector: Option<&str>,
    value: Option<&str>,
) -> &'static str {
    let strong_selector = selector
        .map(|item| {
            item.starts_with('#') || item.contains("data-testid") || item.contains("[name=")
        })
        .unwrap_or(false);
    let weak_selector = selector
        .map(|item| item.starts_with('.') || item.contains("nth-child") || item.contains(":nth"))
        .unwrap_or(false);

    match action {
        StepAction::Navigate => {
            if value
                .map(|item| item.starts_with("http://") || item.starts_with("https://"))
                .unwrap_or(false)
            {
                "high"
            } else {
                "medium"
            }
        }
        StepAction::Click | StepAction::Select | StepAction::Check | StepAction::Uncheck => {
            if strong_selector {
                "high"
            } else if selector.is_some() && !weak_selector {
                "medium"
            } else {
                "low"
            }
        }
        StepAction::Fill | StepAction::AssertText => {
            if strong_selector && value.is_some() {
                "high"
            } else if selector.is_some() || value.is_some() {
                "medium"
            } else {
                "low"
            }
        }
        StepAction::WaitFor => {
            if strong_selector || value.is_some() {
                "medium"
            } else {
                "low"
            }
        }
    }
}

fn build_step_description(
    action: StepAction,
    selector: Option<&str>,
    value: Option<&str>,
) -> String {
    match action {
        StepAction::Navigate => format!("Navigate to {}", value.unwrap_or("target URL")),
        StepAction::Click => format!("Click {}", selector.unwrap_or("target element")),
        StepAction::Fill => format!(
            "Fill {} with {}",
            selector.unwrap_or("target field"),
            value.unwrap_or("value")
        ),
        StepAction::Select => format!(
            "Select {} on {}",
            value.unwrap_or("option"),
            selector.unwrap_or("target select")
        ),
        StepAction::Check => format!("Check {}", selector.unwrap_or("target element")),
        StepAction::Uncheck => format!("Uncheck {}", selector.unwrap_or("target element")),
        StepAction::WaitFor => format!("Wait for {}", selector.or(value).unwrap_or("condition")),
        StepAction::AssertText => format!(
            "Assert text {} on {}",
            value.unwrap_or("value"),
            selector.unwrap_or("target element")
        ),
    }
}
