/// <reference types="node" />

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type SmokeStatus = "SMOKE_PASS" | "SMOKE_BLOCKED" | "SMOKE_FAIL";
type CriteriaVerdict = "SATISFIED" | "BLOCKED" | "FAILED";

interface CriterionAssessment {
  name: string;
  verdict: CriteriaVerdict;
  rationale: string;
}

interface RuntimeEvidenceMarker {
  packagedRuntimeExecuted?: boolean;
  executablePath?: string;
  firstRunVerified?: boolean;
  shellMetadataVerified?: boolean;
  versionMatchesPackageJson?: boolean;
  browserRuntimeGuidanceVerified?: boolean;
  runtimeStatusMessage?: string;
  browserRuntimeStatus?: string;
  missingRuntimeGuidance?: string;
  diagnosticsCaptured?: boolean;
  notes?: string;
  capturedAt?: string;
}

interface SmokeReport {
  status: SmokeStatus;
  details: string;
  criteria: CriterionAssessment[];
  diagnostics: Record<string, unknown>;
}

const WINDOWS_PACKAGED_BOOTSTRAP_P2_SMOKE = "WINDOWS_PACKAGED_BOOTSTRAP_P2_SMOKE";
const CURRENT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(CURRENT_DIRECTORY, "..", "..");
const EVIDENCE_DIRECTORY = resolve(PROJECT_ROOT, ".sisyphus", "evidence");
const SUMMARY_EVIDENCE_PATH = resolve(EVIDENCE_DIRECTORY, "p2-task-T4-windows-packaged-bootstrap.txt");
const LEGACY_SOURCE_EVIDENCE_PATH = resolve(
  EVIDENCE_DIRECTORY,
  "task-T17-packaging-bootstrap-2026-04-01.txt"
);
const PACKAGED_RUNTIME_MARKER_PATH = resolve(
  EVIDENCE_DIRECTORY,
  "p2-task-T4-windows-packaged-bootstrap-ran.json"
);

const EXPECTED_PACKAGED_RUNTIME_PATHS = [
  "src-tauri/target/release/testforge.exe",
  "src-tauri/target/release/bundle/msi/TestForge_0.1.0_x64_en-US.msi",
  "src-tauri/target/release/bundle/nsis/TestForge_0.1.0_x64-setup.exe"
] as const;

const REQUIRED_EVIDENCE_SCENARIOS = [
  "packaged executable launches on Windows first run",
  "shell metadata reports version + first-run bootstrap state from packaged runtime",
  "missing browser/runtime guidance remains actionable without claiming browser success",
  "operator captured first-run diagnostics and runtime metadata evidence"
] as const;

function readProjectFile(relativePath: string): string {
  return readFileSync(resolve(PROJECT_ROOT, relativePath), "utf8");
}

function ensureEvidenceDirectory(): void {
  mkdirSync(EVIDENCE_DIRECTORY, { recursive: true });
}

function assessCriterion(name: string, verdict: CriteriaVerdict, rationale: string): CriterionAssessment {
  return { name, verdict, rationale };
}

function determineSmokeStatus(criteria: CriterionAssessment[]): SmokeStatus {
  if (criteria.some((criterion) => criterion.verdict === "FAILED")) {
    return "SMOKE_FAIL";
  }

  if (criteria.some((criterion) => criterion.verdict === "BLOCKED")) {
    return "SMOKE_BLOCKED";
  }

  return "SMOKE_PASS";
}

function renderEvidence(report: SmokeReport): string {
  const criteriaSection = report.criteria
    .map((criterion) => `- [${criterion.verdict}] ${criterion.name}: ${criterion.rationale}`)
    .join("\n");

  return [
    "P2-T4 Windows packaged bootstrap smoke scaffold",
    `Status: ${report.status}`,
    `Details: ${report.details}`,
    "",
    "Required evidence scenarios:",
    ...REQUIRED_EVIDENCE_SCENARIOS.map((scenario) => `- ${scenario}`),
    "",
    "Expected packaged-runtime evidence paths:",
    ...EXPECTED_PACKAGED_RUNTIME_PATHS.map((entry) => `- ${entry}`),
    `- ${PACKAGED_RUNTIME_MARKER_PATH}`,
    "",
    "Criteria:",
    criteriaSection,
    "",
    "Diagnostics:",
    JSON.stringify(report.diagnostics, null, 2)
  ].join("\n");
}

function buildBlockedReport(reason: string, diagnostics: Record<string, unknown>): SmokeReport {
  const criteria = REQUIRED_EVIDENCE_SCENARIOS.map((scenario) =>
    assessCriterion(
      scenario,
      "BLOCKED",
      `${reason} This smoke preserves the packaged-runtime evidence contract and refuses to treat source assertions as packaged proof.`
    )
  );

  return {
    status: "SMOKE_BLOCKED",
    details: reason,
    criteria,
    diagnostics
  };
}

function collectScaffoldCriteria(): CriterionAssessment[] {
  const packagingTestSource = readProjectFile("tests/frontend/packaging-bootstrap-t17.test.ts");
  const appSource = readProjectFile("src/App.tsx");
  const statusBarSource = readProjectFile("src/components/StatusBar.tsx");
  const tauriClientSource = readProjectFile("src/services/tauri-client.ts");
  const dtoSource = readProjectFile("src/types/dto.ts");
  const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
  const rustStateSource = readProjectFile("src-tauri/src/state.rs");

  return [
    packagingTestSource.includes("Runtime-packaged proof remains a separate requirement")
      ? assessCriterion(
          "source regression stays separate from packaged proof",
          "SATISFIED",
          "T17 regression explicitly states it only guards source/seam behavior and cannot prove packaged runtime." 
        )
      : assessCriterion(
          "source regression stays separate from packaged proof",
          "FAILED",
          "T17 regression still blurs source/seam coverage with packaged-runtime proof."
        ),
    appSource.includes("getShellMetadata") &&
    appSource.includes("<StatusBar shellMetadata={shellMetadata} />") &&
    tauriClientSource.includes('invokeCommand("shell.metadata.get", {})') &&
    dtoSource.includes("export interface ShellMetadataDto")
      ? assessCriterion(
          "packaged shell metadata seam remains wired",
          "SATISFIED",
          "Source seams still require packaged shell metadata to flow through typed IPC into the app shell."
        )
      : assessCriterion(
          "packaged shell metadata seam remains wired",
          "FAILED",
          "Typed shell metadata seam is no longer strongly represented in the packaging scaffold inputs."
        ),
    statusBarSource.includes("shellMetadata.browserRuntime.message") &&
    statusBarSource.includes("Browser automation unavailable") &&
    statusBarSource.includes("API/data features remain usable")
      ? assessCriterion(
          "missing browser/runtime guidance remains explicit",
          "SATISFIED",
          "StatusBar source still preserves actionable degraded-runtime guidance instead of implying browser success."
        )
      : assessCriterion(
          "missing browser/runtime guidance remains explicit",
          "FAILED",
          "StatusBar no longer preserves the expected degraded browser/runtime guidance contract."
        ),
    rustLibSource.includes("fn shell_metadata_get") && rustStateSource.includes("ShellBootstrapSnapshot")
      ? assessCriterion(
          "first-run bootstrap snapshot seam remains available",
          "SATISFIED",
          "Backend seams still expose first-run/bootstrap state through the shell metadata pathway."
        )
      : assessCriterion(
          "first-run bootstrap snapshot seam remains available",
          "FAILED",
          "Backend seams no longer clearly expose first-run/bootstrap state for packaged runtime validation."
        )
  ];
}

function parseRuntimeMarker(markerPath: string): { marker: RuntimeEvidenceMarker | null; error: string | null } {
  try {
    const marker = JSON.parse(readFileSync(markerPath, "utf8")) as RuntimeEvidenceMarker;
    return { marker, error: null };
  } catch (error) {
    return {
      marker: null,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

function collectRuntimeEvidenceReport(scaffoldCriteria: CriterionAssessment[]): SmokeReport {
  if (!existsSync(PACKAGED_RUNTIME_MARKER_PATH)) {
    return buildBlockedReport(
      "Packaged-runtime marker is missing; no real Windows packaged first-run evidence has been captured yet.",
      {
        packagedRuntimeMarkerPath: PACKAGED_RUNTIME_MARKER_PATH,
        expectedPackagedRuntimePaths: EXPECTED_PACKAGED_RUNTIME_PATHS,
        expectedMarkerShape: {
          packagedRuntimeExecuted: true,
          executablePath: "absolute-or-project-relative packaged executable path",
          firstRunVerified: true,
          shellMetadataVerified: true,
          versionMatchesPackageJson: true,
          browserRuntimeGuidanceVerified: true,
          runtimeStatusMessage: "Status text captured from packaged first run",
          browserRuntimeStatus: "healthy|degraded|missing",
          missingRuntimeGuidance: "Actionable operator guidance when browser/runtime is absent",
          diagnosticsCaptured: true,
          capturedAt: "ISO-8601 timestamp",
          notes: "Optional operator/runtime notes"
        },
        legacySourceEvidencePath: LEGACY_SOURCE_EVIDENCE_PATH,
        scaffoldCriteria
      }
    );
  }

  const { marker, error } = parseRuntimeMarker(PACKAGED_RUNTIME_MARKER_PATH);
  if (!marker) {
    return {
      status: "SMOKE_FAIL",
      details: "Packaged-runtime marker exists but could not be parsed as JSON.",
      criteria: scaffoldCriteria,
      diagnostics: {
        packagedRuntimeMarkerPath: PACKAGED_RUNTIME_MARKER_PATH,
        parseError: error
      }
    };
  }

  const runtimeCriteria: CriterionAssessment[] = [
    assessCriterion(
      REQUIRED_EVIDENCE_SCENARIOS[0],
      marker.packagedRuntimeExecuted === true && typeof marker.executablePath === "string" && marker.executablePath.length > 0
        ? "SATISFIED"
        : "FAILED",
      marker.packagedRuntimeExecuted === true && typeof marker.executablePath === "string" && marker.executablePath.length > 0
        ? `Runtime marker records a real packaged Windows executable launch: ${marker.executablePath}`
        : "Runtime marker does not confirm a real packaged Windows executable launch path."
    ),
    assessCriterion(
      REQUIRED_EVIDENCE_SCENARIOS[1],
      marker.firstRunVerified === true && marker.shellMetadataVerified === true && marker.versionMatchesPackageJson === true
        ? "SATISFIED"
        : "FAILED",
      marker.firstRunVerified === true && marker.shellMetadataVerified === true && marker.versionMatchesPackageJson === true
        ? "Runtime marker confirms packaged first-run bootstrap state plus version metadata matched the canonical package version."
        : "Runtime marker does not confirm packaged first-run bootstrap state and version metadata alignment."
    ),
    assessCriterion(
      REQUIRED_EVIDENCE_SCENARIOS[2],
      marker.browserRuntimeGuidanceVerified === true &&
      typeof marker.missingRuntimeGuidance === "string" &&
      marker.missingRuntimeGuidance.trim().length > 0
        ? "SATISFIED"
        : "FAILED",
      marker.browserRuntimeGuidanceVerified === true &&
      typeof marker.missingRuntimeGuidance === "string" &&
      marker.missingRuntimeGuidance.trim().length > 0
        ? `Runtime marker preserves actionable missing browser/runtime guidance: ${marker.missingRuntimeGuidance}`
        : "Runtime marker does not capture actionable missing browser/runtime guidance from the packaged shell."
    ),
    assessCriterion(
      REQUIRED_EVIDENCE_SCENARIOS[3],
      marker.diagnosticsCaptured === true && typeof marker.runtimeStatusMessage === "string" && marker.runtimeStatusMessage.length > 0
        ? "SATISFIED"
        : "FAILED",
      marker.diagnosticsCaptured === true && typeof marker.runtimeStatusMessage === "string" && marker.runtimeStatusMessage.length > 0
        ? `Runtime diagnostics were captured from packaged first run. Status: ${marker.runtimeStatusMessage}`
        : "Runtime marker does not confirm packaged first-run diagnostics capture."
    )
  ];

  const status = determineSmokeStatus(runtimeCriteria);
  return {
    status,
    details:
      status === "SMOKE_PASS"
        ? "Packaged-runtime marker confirms the required P2-T4 Windows first-run checks."
        : "Packaged-runtime marker is present, but one or more required P2-T4 checks are still not confirmed.",
    criteria: runtimeCriteria,
    diagnostics: {
      packagedRuntimeMarkerPath: PACKAGED_RUNTIME_MARKER_PATH,
      expectedPackagedRuntimePaths: EXPECTED_PACKAGED_RUNTIME_PATHS,
      legacySourceEvidencePath: LEGACY_SOURCE_EVIDENCE_PATH,
      marker,
      scaffoldCriteria
    }
  };
}

const sourceEvidenceText = existsSync(LEGACY_SOURCE_EVIDENCE_PATH)
  ? readFileSync(LEGACY_SOURCE_EVIDENCE_PATH, "utf8")
  : "";
const scaffoldCriteria = collectScaffoldCriteria();
const report = sourceEvidenceText.includes("no Windows bundle artifact is claimed here")
  ? collectRuntimeEvidenceReport(scaffoldCriteria)
  : {
      status: "SMOKE_FAIL" as SmokeStatus,
      details: "Legacy T17 evidence no longer documents the lack of packaged Windows proof; the separation contract has drifted.",
      criteria: scaffoldCriteria,
      diagnostics: {
        legacySourceEvidencePath: LEGACY_SOURCE_EVIDENCE_PATH,
        scaffoldCriteria
      }
    };

ensureEvidenceDirectory();
writeFileSync(SUMMARY_EVIDENCE_PATH, renderEvidence(report), "utf8");

console.log(`[${WINDOWS_PACKAGED_BOOTSTRAP_P2_SMOKE}] ${report.status} :: ${report.details}`);
console.log(JSON.stringify(report.diagnostics, null, 2));
