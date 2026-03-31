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

const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustPathsSource = readProjectFile("src-tauri/src/utils/paths.rs");
const rustServicesModSource = readProjectFile("src-tauri/src/services/mod.rs");
const rustArtifactServiceSource = readProjectFile("src-tauri/src/services/artifact_service.rs");
const rustDtoContractsSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const tsDtoSource = readProjectFile("src/types/dto.ts");
const migration003Source = readProjectFile("src-tauri/migrations/003_add_artifact_manifests.sql");

assert(
  rustPathsSource.includes("pub exports: PathBuf") &&
    rustPathsSource.includes("pub screenshots: PathBuf") &&
    rustPathsSource.includes("pub fn exports_path") &&
    rustPathsSource.includes("pub fn screenshots_path"),
  "T10 must continue using AppPaths as the source of truth for exports and screenshots directories."
);

assert(
  rustServicesModSource.includes("pub mod artifact_service") &&
    rustServicesModSource.includes("pub use artifact_service::ArtifactService"),
  "T10 must expose a dedicated ArtifactService module instead of spreading export/artifact logic across unrelated layers."
);

assert(
  rustArtifactServiceSource.includes("pub struct ArtifactManifestDto") &&
    rustArtifactServiceSource.includes("pub struct ReportExportDto") &&
    rustArtifactServiceSource.includes("pub fn resolve_artifact_path") &&
    rustArtifactServiceSource.includes("pub fn persist_report_export") &&
    rustArtifactServiceSource.includes("pub fn persist_artifact_manifest") &&
    rustArtifactServiceSource.includes("preview_safe") &&
    rustArtifactServiceSource.includes("[REDACTED]"),
  "ArtifactService must provide reusable path resolution, preview-safe persistence helpers, manifest writing, and redacted export baselines."
);

assert(
  rustArtifactServiceSource.includes("authorization") &&
    rustArtifactServiceSource.includes("bearer") &&
    rustArtifactServiceSource.includes("api_key") &&
    rustArtifactServiceSource.includes("ciphertext") &&
    rustArtifactServiceSource.includes("masked_preview"),
  "Artifact/report sanitization must explicitly block raw auth values, ciphertext fields, and secret previews from exports."
);

assert(
  rustLibSource.includes("persist_report_export") &&
    rustLibSource.includes("artifact_service") &&
    rustLibSource.includes("file_path") &&
    rustLibSource.includes("manifest"),
  "Existing backend export flow must start using ArtifactService persistence helpers so T10 is filesystem-based rather than content-only."
);

assert(
  rustDtoContractsSource.includes("pub struct ArtifactManifestDto") &&
    rustDtoContractsSource.includes("pub struct ReportExportDto") &&
    rustDtoContractsSource.includes("pub file_path: String") &&
    rustDtoContractsSource.includes("pub manifest: ArtifactManifestDto"),
  "Rust shared DTO contracts must expose report export and artifact manifest baseline structures for later T15/T17 reuse."
);

assert(
  tsDtoSource.includes("export interface ArtifactManifestDto") &&
    tsDtoSource.includes("export interface ReportExportDto") &&
    tsDtoSource.includes("filePath: string") &&
    tsDtoSource.includes("manifest: ArtifactManifestDto"),
  "TypeScript DTO contracts must mirror the T10 artifact manifest/export baseline types."
);

assert(
  migration003Source.includes("CREATE TABLE IF NOT EXISTS artifact_manifests") &&
    migration003Source.includes("artifact_type") &&
    migration003Source.includes("file_path") &&
    migration003Source.includes("preview_json") &&
    migration003Source.includes("screenshots/"),
  "T10 migration must add SQLite metadata storage for artifact manifests while keeping artifact files themselves on disk."
);

console.log("Export + artifact path T10 regression test passed.");
