//! Artifact/export baseline service for T10.
//!
//! Artifacts stay on disk while lightweight manifest metadata can be stored in SQLite.
//! Service này chỉ xây baseline tối thiểu cho path resolution, sanitized HTML/JSON report
//! export, preview-safe persistence helpers, và manifest reuse cho các task sau.
//! Shared DTO dependencies used here:
//! - `pub struct ArtifactManifestDto` lives in contracts/dto.rs
//! - `pub struct ReportExportDto` lives in contracts/dto.rs

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contracts::dto::{ArtifactManifestDto, ReportExportDto};
use crate::error::{AppError, AppResult};
use crate::utils::paths::{ensure_dir_exists, AppPaths};

const REDACTED: &str = "[REDACTED]";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Export,
    Screenshot,
    Report,
}

impl ArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Screenshot => "screenshot",
            Self::Report => "report",
        }
    }

    fn root_dir<'a>(self, paths: &'a AppPaths) -> &'a Path {
        match self {
            Self::Screenshot => &paths.screenshots,
            Self::Export | Self::Report => &paths.exports,
        }
    }

    fn relative_prefix(self) -> &'static str {
        match self {
            Self::Screenshot => "screenshots/",
            Self::Export => "exports/",
            Self::Report => "exports/reports/",
        }
    }
}

pub struct ArtifactService {
    paths: AppPaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCiHandoffArtifact {
    pub file_path: String,
    pub relative_path: String,
    pub manifest: ArtifactManifestDto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CiHandoffArtifactTarget {
    file_path: PathBuf,
    relative_path: String,
    logical_name: String,
    manifest_id: String,
}

impl ArtifactService {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn resolve_artifact_path(
        &self,
        artifact_kind: ArtifactKind,
        scope: &str,
        file_name: &str,
    ) -> AppResult<PathBuf> {
        let safe_scope = sanitize_path_segment(scope);
        let safe_file_name = sanitize_file_name(file_name);
        let directory = artifact_kind.root_dir(&self.paths).join(safe_scope);
        ensure_dir_exists(&directory)?;
        Ok(directory.join(safe_file_name))
    }

    pub fn preview_safe_json_value(&self, value: &Value) -> Value {
        preview_safe_json_value(value)
    }

    pub fn persist_report_export(
        &self,
        artifact_name: &str,
        format: &str,
        payload: &Value,
    ) -> AppResult<ReportExportDto> {
        let safe_format = sanitize_path_segment(format);
        let extension = match safe_format.as_str() {
            "html" | "json" => safe_format.clone(),
            _ => {
                return Err(AppError::validation(format!(
                    "Unsupported report export format: {safe_format}"
                )))
            }
        };

        let safe_name = sanitize_path_segment(artifact_name);
        let file_name = format!("{safe_name}.{extension}");
        let file_path =
            self.resolve_artifact_path(ArtifactKind::Report, artifact_name, &file_name)?;
        let preview_safe = self.preview_safe_json_value(payload);
        let preview_json = serde_json::to_string_pretty(&preview_safe).map_err(|error| {
            AppError::storage_write(format!(
                "Không thể serialize report preview-safe JSON: {error}"
            ))
        })?;

        let content = if extension == "json" {
            preview_json.clone()
        } else {
            build_sanitized_html_report(&safe_name, &preview_json)
        };

        fs::write(&file_path, content).map_err(|error| {
            AppError::storage_write(format!(
                "Không thể ghi report export {:?}: {error}",
                file_path
            ))
        })?;

        let manifest = ArtifactManifestDto {
            id: format!("artifact-{}", uuid::Uuid::new_v4()),
            artifact_type: ArtifactKind::Report.as_str().to_string(),
            logical_name: safe_name.clone(),
            file_path: file_path.to_string_lossy().into_owned(),
            relative_path: format!(
                "{}{}{}",
                ArtifactKind::Report.relative_prefix(),
                sanitize_path_segment(artifact_name),
                format!("/{file_name}")
            ),
            preview_json,
            created_at: Utc::now().to_rfc3339(),
        };

        Ok(ReportExportDto {
            file_name,
            format: extension,
            file_path: file_path.to_string_lossy().into_owned(),
            manifest,
        })
    }

    pub fn persist_artifact_manifest(
        &self,
        conn: &Connection,
        manifest: &ArtifactManifestDto,
    ) -> AppResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO artifact_manifests (id, artifact_type, logical_name, file_path, relative_path, preview_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                manifest.id,
                manifest.artifact_type,
                manifest.logical_name,
                manifest.file_path,
                manifest.relative_path,
                manifest.preview_json,
                manifest.created_at,
            ],
        )
        .map_err(|error| AppError::db_query(format!("Không thể lưu artifact manifest: {error}")))?;

        Ok(())
    }

    pub fn persist_ci_handoff_contract_json(
        &self,
        run_id: &str,
        payload: &Value,
        output_dir: Option<&str>,
        file_name: Option<&str>,
    ) -> AppResult<PersistedCiHandoffArtifact> {
        let target = self.build_ci_handoff_artifact_target(run_id, output_dir, file_name)?;
        if let Some(parent) = target.file_path.parent() {
            ensure_dir_exists(parent)?;
            self.verify_path_under_exports_root(parent)?;
        }

        let canonical_json = serde_json::to_string_pretty(payload).map_err(|error| {
            AppError::storage_write(format!(
                "Không thể serialize CI handoff canonical JSON: {error}"
            ))
        })?;

        fs::write(&target.file_path, canonical_json).map_err(|error| {
            AppError::storage_write(format!(
                "Không thể ghi CI handoff artifact {:?}: {error}",
                target.file_path
            ))
        })?;

        let preview_safe = self.preview_safe_json_value(payload);
        let preview_json = serde_json::to_string_pretty(&preview_safe).map_err(|error| {
            AppError::storage_write(format!(
                "Không thể serialize CI handoff preview JSON: {error}"
            ))
        })?;

        let manifest = ArtifactManifestDto {
            id: target.manifest_id,
            artifact_type: "report_json".to_string(),
            logical_name: target.logical_name,
            file_path: target.file_path.to_string_lossy().into_owned(),
            relative_path: target.relative_path.clone(),
            preview_json,
            created_at: Utc::now().to_rfc3339(),
        };

        Ok(PersistedCiHandoffArtifact {
            file_path: target.file_path.to_string_lossy().into_owned(),
            relative_path: target.relative_path,
            manifest,
        })
    }

    pub fn preview_ci_handoff_artifact_reference(
        &self,
        run_id: &str,
        output_dir: Option<&str>,
        file_name: Option<&str>,
    ) -> AppResult<(String, String)> {
        let target = self.build_ci_handoff_artifact_target(run_id, output_dir, file_name)?;
        Ok((
            target.file_path.to_string_lossy().into_owned(),
            target.relative_path,
        ))
    }

    fn resolve_ci_handoff_output_dir(&self, output_dir: Option<&str>) -> AppResult<PathBuf> {
        let default_dir = self.paths.exports.join("ci");
        let Some(raw_output_dir) = output_dir else {
            return Ok(default_dir);
        };

        let normalized = normalize_relative_output_dir(raw_output_dir)?;
        if normalized.as_os_str().is_empty() {
            return Ok(default_dir);
        }

        let normalized_text = normalized.to_string_lossy().replace('\\', "/");
        let normalized_under_exports = if normalized_text == "exports" {
            PathBuf::new()
        } else if normalized_text.starts_with("exports/") {
            normalized
                .strip_prefix("exports")
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| normalized.clone())
        } else {
            normalized
        };

        Ok(self.paths.exports.join(normalized_under_exports))
    }

    fn build_ci_handoff_artifact_target(
        &self,
        run_id: &str,
        output_dir: Option<&str>,
        file_name: Option<&str>,
    ) -> AppResult<CiHandoffArtifactTarget> {
        let directory = self.resolve_ci_handoff_output_dir(output_dir)?;
        let default_name = format!("ci-execution-{run_id}.json");
        let safe_file_name = sanitize_file_name(file_name.unwrap_or(&default_name));
        let file_path = directory.join(&safe_file_name);

        let relative_path = file_path
            .strip_prefix(&self.paths.base)
            .map(normalize_relative_path)
            .unwrap_or_else(|_| format!("exports/ci/{safe_file_name}"));

        Ok(CiHandoffArtifactTarget {
            file_path,
            relative_path,
            logical_name: format!("ci-execution-{run_id}"),
            manifest_id: format!("artifact-ci-handoff-{run_id}"),
        })
    }

    fn verify_path_under_exports_root(&self, path: &Path) -> AppResult<()> {
        let exports_root = fs::canonicalize(&self.paths.exports).map_err(|error| {
            AppError::storage_path(format!(
                "Không thể canonicalize exports root {:?}: {error}",
                self.paths.exports
            ))
        })?;
        let candidate = fs::canonicalize(path).map_err(|error| {
            AppError::storage_path(format!(
                "Không thể canonicalize CI handoff output dir {:?}: {error}",
                path
            ))
        })?;

        if candidate.starts_with(&exports_root) {
            Ok(())
        } else {
            Err(AppError::validation(
                "CI handoff outputDir must resolve under app exports root",
            ))
        }
    }
}

fn normalize_relative_output_dir(raw: &str) -> AppResult<PathBuf> {
    let candidate = Path::new(raw.trim());
    if candidate.as_os_str().is_empty() {
        return Ok(PathBuf::new());
    }

    if candidate.is_absolute() {
        return Err(AppError::validation(
            "CI handoff outputDir must be a relative path",
        ));
    }

    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(segment) => normalized.push(segment),
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(AppError::validation(
                        "CI handoff outputDir must not traverse outside exports root",
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::validation(
                    "CI handoff outputDir must be a normalized relative path",
                ));
            }
        }
    }

    Ok(normalized)
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sanitize_path_segment(value: &str) -> String {
    let trimmed = value.trim().to_lowercase();
    let mut output = String::with_capacity(trimmed.len());

    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character);
        } else if matches!(character, '-' | '_' | '.') {
            output.push(character);
        } else if character.is_whitespace() || matches!(character, '/' | '\\' | ':') {
            if !output.ends_with('-') {
                output.push('-');
            }
        }
    }

    let normalized = output.trim_matches('-').to_string();
    if normalized.is_empty() {
        "artifact".to_string()
    } else {
        normalized
    }
}

fn sanitize_file_name(value: &str) -> String {
    let candidate = sanitize_path_segment(value);
    if candidate.contains('.') {
        candidate
    } else {
        format!("{candidate}.json")
    }
}

fn build_sanitized_html_report(title: &str, preview_json: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1><pre>{}</pre></body></html>",
        escape_html(title),
        escape_html(title),
        escape_html(preview_json),
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn preview_safe_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized = Map::new();
            for (key, item) in map {
                if should_redact_key(key) {
                    sanitized.insert(key.clone(), Value::String(REDACTED.to_string()));
                } else {
                    sanitized.insert(key.clone(), preview_safe_json_value(item));
                }
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.iter().map(preview_safe_json_value).collect()),
        Value::String(text) if looks_like_sensitive_value(text) => {
            Value::String(REDACTED.to_string())
        }
        _ => value.clone(),
    }
}

fn should_redact_key(key: &str) -> bool {
    let normalized = key.trim().to_lowercase();
    normalized.contains("authorization")
        || normalized.contains("bearer")
        || normalized.contains("basic")
        || normalized.contains("api_key")
        || normalized.contains("apikey")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("ciphertext")
        || normalized.contains("masked_preview")
        || normalized == "value"
}

fn looks_like_sensitive_value(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.starts_with("bearer ")
        || normalized.starts_with("basic ")
        || contains_secret_like_fragment(&normalized)
        || normalized == REDACTED.to_lowercase()
}

fn contains_secret_like_fragment(normalized: &str) -> bool {
    normalized.contains("ciphertext")
        || normalized.contains("api_key")
        || normalized.contains("token=")
        || normalized.contains("authorization:")
        || normalized.contains("encrypted:")
        || normalized.contains("masked preview")
        || normalized.contains("masked_preview")
        || normalized.contains("password=")
        || normalized.contains("secret=")
}

#[allow(dead_code)]
fn _preview_safe_headers(headers: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    headers
        .iter()
        .map(|(key, value)| {
            if should_redact_key(key) || looks_like_sensitive_value(value) {
                (key.clone(), REDACTED.to_string())
            } else {
                (key.clone(), value.clone())
            }
        })
        .collect()
}
