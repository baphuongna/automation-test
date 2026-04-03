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

interface SmokeReport {
  status: SmokeStatus;
  details: string;
  criteria: CriterionAssessment[];
  diagnostics: Record<string, unknown>;
}

const DESKTOP_SUITE_RUNTIME_P2_SMOKE = "DESKTOP_SUITE_RUNTIME_P2_SMOKE";
const CURRENT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(CURRENT_DIRECTORY, "..", "..");
const EVIDENCE_DIRECTORY = resolve(PROJECT_ROOT, ".sisyphus", "evidence");
const EVIDENCE_PATH = resolve(EVIDENCE_DIRECTORY, "p2-task-T3-desktop-suite-runtime.txt");

const REQUIRED_EVIDENCE_SCENARIOS = [
  "mixed suite execution",
  "rerun failed target scope",
  "progress/history/detail hydration",
  "cancel idempotent guard"
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
    "P2-T3 desktop suite runtime smoke scaffold",
    `Status: ${report.status}`,
    `Details: ${report.details}`,
    "",
    "Required evidence scenarios:",
    ...REQUIRED_EVIDENCE_SCENARIOS.map((scenario) => `- ${scenario}`),
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
      `${reason} This scaffold preserves the required evidence path but does not claim desktop runtime success without real execution artifacts.`
    )
  );

  return {
    status: "SMOKE_BLOCKED",
    details: reason,
    criteria,
    diagnostics
  };
}

function collectScaffoldSignals(): SmokeReport {
  const suiteRunnerSource = readProjectFile("tests/frontend/suite-runner-t15.test.ts");
  const testRunnerSource = readProjectFile("tests/frontend/test-runner-t16.test.ts");
  const routeSource = readProjectFile("src/routes/test-runner.tsx");
  const runnerClientSource = readProjectFile("src/services/runner-client.ts");
  const orchestrationSource = readProjectFile("src-tauri/src/services/runner_orchestration_service.rs");
  const repositorySource = readProjectFile("src-tauri/src/repositories/runner_repository.rs");
  const runtimeEvidenceMarkerPath = resolve(EVIDENCE_DIRECTORY, "p2-task-T3-desktop-suite-runtime-ran.json");

  const scaffoldCriteria: CriterionAssessment[] = [];

  scaffoldCriteria.push(
    suiteRunnerSource.includes("mixed suite thật sự điều phối cả API và UI targets") &&
      orchestrationSource.includes("TestCaseType::Api =>") &&
      orchestrationSource.includes("TestCaseType::Ui =>")
      ? assessCriterion(
          "mixed suite execution",
          "SATISFIED",
          "Source/seam guards now assert real mixed API+UI orchestration paths for P2-T3."
        )
      : assessCriterion(
          "mixed suite execution",
          "FAILED",
          "Mixed suite orchestration seam is not asserted strongly enough in the current scaffold inputs."
        )
  );

  scaffoldCriteria.push(
    suiteRunnerSource.includes("rerun-failed theo target case+dataRow") &&
      orchestrationSource.includes("collect::<HashSet<(String, Option<String>)>>()") &&
      repositorySource.includes("SELECT DISTINCT trr.case_id, trr.data_row_id")
      ? assessCriterion(
          "rerun failed target scope",
          "SATISFIED",
          "Regression seams assert rerun-failed target scope at the persisted case+dataRow boundary."
        )
      : assessCriterion(
          "rerun failed target scope",
          "FAILED",
          "Rerun-failed target scoping seam is missing or no longer asserted."
        )
  );

  scaffoldCriteria.push(
    testRunnerSource.includes("hydrate suites + environments + run history cùng lúc") &&
      routeSource.includes("Promise.all([") &&
      routeSource.includes("runnerClient.getRunDetail({ runId: nextSelectedRunId })") &&
      runnerClientSource.includes("async getRunDetail(input: { runId: string })")
      ? assessCriterion(
          "progress/history/detail hydration",
          "SATISFIED",
          "Read-side seams now guard history/detail hydration wiring for the desktop runner surface."
        )
      : assessCriterion(
          "progress/history/detail hydration",
          "FAILED",
          "History/detail hydration wiring is not sufficiently guarded by the current scaffold inputs."
        )
  );

  scaffoldCriteria.push(
    testRunnerSource.includes("cancel idempotent guard semantics") &&
      routeSource.includes("Already cancelling the active run. Waiting for terminal update.") &&
      orchestrationSource.includes("return self.runner_repository.load_run_result(run_id);")
      ? assessCriterion(
          "cancel idempotent guard",
          "SATISFIED",
          "Source seams cover UI guard + backend idempotent completion behavior for repeated cancel requests."
        )
      : assessCriterion(
          "cancel idempotent guard",
          "FAILED",
          "Cancel idempotent guard semantics are not fully represented in the scaffold inputs."
        )
  );

  if (!existsSync(runtimeEvidenceMarkerPath)) {
    return buildBlockedReport(
      "Desktop runtime marker is missing; no real Tauri mixed-suite execution evidence has been captured yet.",
      {
        runtimeEvidenceMarkerPath,
        expectedMarkerShape: {
          runExecuted: true,
          rerunFailedVerified: true,
          historyDetailVerified: true,
          cancelIdempotencyVerified: true,
          capturedAt: "ISO-8601 timestamp",
          notes: "Optional operator/runtime notes"
        },
        scaffoldCriteria
      }
    );
  }

  let marker: Record<string, unknown>;
  try {
    marker = JSON.parse(readFileSync(runtimeEvidenceMarkerPath, "utf8")) as Record<string, unknown>;
  } catch (error) {
    return {
      status: "SMOKE_FAIL",
      details: "Desktop runtime evidence marker exists but could not be parsed as JSON.",
      criteria: scaffoldCriteria,
      diagnostics: {
        runtimeEvidenceMarkerPath,
        error: error instanceof Error ? error.message : String(error)
      }
    };
  }

  const runtimeCriteria: CriterionAssessment[] = [
    assessCriterion(
      "mixed suite execution",
      marker.runExecuted === true ? "SATISFIED" : "FAILED",
      marker.runExecuted === true
        ? "Runtime marker records that a real desktop mixed suite execution was performed."
        : "Runtime marker does not confirm real mixed suite execution."
    ),
    assessCriterion(
      "rerun failed target scope",
      marker.rerunFailedVerified === true ? "SATISFIED" : "FAILED",
      marker.rerunFailedVerified === true
        ? "Runtime marker records that rerun-failed scope matched failed targets only."
        : "Runtime marker does not confirm rerun-failed target scoping."
    ),
    assessCriterion(
      "progress/history/detail hydration",
      marker.historyDetailVerified === true ? "SATISFIED" : "FAILED",
      marker.historyDetailVerified === true
        ? "Runtime marker records successful progress/history/detail inspection in desktop runtime."
        : "Runtime marker does not confirm history/detail inspection success."
    ),
    assessCriterion(
      "cancel idempotent guard",
      marker.cancelIdempotencyVerified === true ? "SATISFIED" : "FAILED",
      marker.cancelIdempotencyVerified === true
        ? "Runtime marker records cancel idempotency verification under mixed execution conditions."
        : "Runtime marker does not confirm cancel idempotency verification."
    )
  ];

  const status = determineSmokeStatus(runtimeCriteria);
  return {
    status,
    details:
      status === "SMOKE_PASS"
        ? "Desktop runtime evidence marker confirms the required P2-T3 mixed suite execution checks."
        : "Desktop runtime evidence marker is present, but one or more required P2-T3 checks are still not confirmed.",
    criteria: runtimeCriteria,
    diagnostics: {
      runtimeEvidenceMarkerPath,
      marker,
      scaffoldCriteria
    }
  };
}

const report = collectScaffoldSignals();
ensureEvidenceDirectory();
writeFileSync(EVIDENCE_PATH, renderEvidence(report), "utf8");

console.log(`[${DESKTOP_SUITE_RUNTIME_P2_SMOKE}] ${report.status} :: ${report.details}`);
console.log(JSON.stringify(report.diagnostics, null, 2));

if (report.status !== "SMOKE_PASS") {
  process.exit(1);
}
