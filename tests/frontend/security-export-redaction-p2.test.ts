import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(message);
  }
}

function readProjectFile(relativePath: string): string {
  const absolutePath = resolve(relativePath);
  assert(existsSync(absolutePath), `Expected file to exist: ${relativePath}`);
  return readFileSync(absolutePath, "utf8");
}

const rustArtifactServiceSource = readProjectFile("src-tauri/src/services/artifact_service.rs");
const rustDtoContractsSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustCiHandoffServiceSource = readProjectFile("src-tauri/src/services/ci_handoff_service.rs");
const tsDtoSource = readProjectFile("src/types/dto.ts");

assert(
  rustArtifactServiceSource.includes("pub fn preview_safe_json_value") &&
    rustArtifactServiceSource.includes("pub fn persist_report_export"),
  "P2-T5 must continue to route report/export payloads through the shared preview-safe artifact seam."
);

assert(
  rustArtifactServiceSource.includes("normalized.contains(\"password\")") &&
    rustArtifactServiceSource.includes("normalized.contains(\"token\")") &&
    rustArtifactServiceSource.includes("normalized.contains(\"secret\")") &&
    rustArtifactServiceSource.includes("normalized.contains(\"ciphertext\")") &&
    rustArtifactServiceSource.includes("normalized.contains(\"masked_preview\")") &&
    rustArtifactServiceSource.includes("normalized == \"value\""),
  "P2-T5 must keep redacting secret-oriented keys, including value-style leakage fields, before report/export payloads are serialized."
);

assert(
  rustArtifactServiceSource.includes("contains_secret_like_fragment") &&
    rustArtifactServiceSource.includes("looks_like_sensitive_value") &&
    rustArtifactServiceSource.includes("masked preview") &&
    rustArtifactServiceSource.includes("encrypted:") &&
    rustArtifactServiceSource.includes("authorization:") &&
    rustArtifactServiceSource.includes("token="),
  "P2-T5 must classify suspicious secret-bearing values and previews so plaintext, ciphertext, and masked-preview strings stay redacted in exports."
);

assert(
  rustArtifactServiceSource.includes("pub fn persist_ci_handoff_contract_json") &&
    rustArtifactServiceSource.includes("let canonical_json = serde_json::to_string_pretty(payload)") &&
    rustArtifactServiceSource.includes("let preview_safe = self.preview_safe_json_value(payload)") &&
    rustArtifactServiceSource.includes("preview_json") &&
    rustArtifactServiceSource.includes("pub fn preview_ci_handoff_artifact_reference") &&
    rustArtifactServiceSource.includes("build_ci_handoff_artifact_target") &&
    rustCiHandoffServiceSource.includes("if !artifacts.iter().any(|artifact| artifact.relative_path == self_relative_path)") &&
    rustCiHandoffServiceSource.includes("default_redaction_metadata") &&
    rustCiHandoffServiceSource.includes("\"redaction\": {") &&
    rustCiHandoffServiceSource.includes("\"policyVersion\":") &&
    rustCiHandoffServiceSource.includes("sanitize_ci_handoff_text"),
  "P2-T8 must keep CI handoff JSON artifact persistence canonical, preview-safe, self-referentially consistent, and redaction-aware."
);

assert(
  !rustDtoContractsSource.includes("pub ciphertext") &&
    !rustDtoContractsSource.includes("pub masked_preview") &&
    !rustDtoContractsSource.includes("pub plaintext") &&
    !tsDtoSource.includes("ciphertext:") &&
    !tsDtoSource.includes("maskedPreview:") &&
    !tsDtoSource.includes("plaintext:"),
  "P2-T5 must not expose plaintext, ciphertext, or masked secret preview fields in shared DTO contracts."
);

console.log("P2 security export redaction regression test passed.");
