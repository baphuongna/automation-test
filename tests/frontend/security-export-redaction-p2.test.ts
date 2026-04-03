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
  !rustDtoContractsSource.includes("pub ciphertext") &&
    !rustDtoContractsSource.includes("pub masked_preview") &&
    !rustDtoContractsSource.includes("pub plaintext") &&
    !tsDtoSource.includes("ciphertext:") &&
    !tsDtoSource.includes("maskedPreview:") &&
    !tsDtoSource.includes("plaintext:"),
  "P2-T5 must not expose plaintext, ciphertext, or masked secret preview fields in shared DTO contracts."
);

console.log("P2 security export redaction regression test passed.");
