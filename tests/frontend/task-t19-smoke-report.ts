import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import { fileURLToPath } from "node:url";

type SmokeStatus = "SMOKE_PASS" | "SMOKE_BLOCKED" | "SMOKE_FAIL";
type GateVerdict = "PASS" | "BLOCKED" | "FAIL";
type CriteriaVerdict = "SATISFIED" | "BLOCKED" | "FAILED";

interface CommandReport {
  name: string;
  command: string;
  status: "passed" | "failed" | "blocked";
  exitCode: number | null;
  stdout: string;
  stderr: string;
  details: string;
}

interface CriterionAssessment {
  name: string;
  verdict: CriteriaVerdict;
  rationale: string;
}

interface SmokeSummary {
  verdict: GateVerdict;
  criteria: CriterionAssessment[];
  commands: CommandReport[];
}

const CURRENT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(CURRENT_DIRECTORY, "..", "..");
const EVIDENCE_DIRECTORY = resolve(PROJECT_ROOT, ".sisyphus", "evidence");
const SMOKE_SUMMARY_PATH = resolve(EVIDENCE_DIRECTORY, "task-T19-smoke-summary.txt");
const BROWSER_GATE_PATH = resolve(EVIDENCE_DIRECTORY, "task-T19-browser-gate.txt");

const MVP_EXIT_CRITERIA = [
  "App can bootstrap from empty local data directory without manual dev tooling",
  "QA can create environment + secret variables safely",
  "QA can create/run API test and inspect pass/fail details",
  "QA can record, edit, save, and replay a simple UI flow in Chromium",
  "QA can run a mixed suite and inspect progress/history",
  "Browser failure paths do not crash API-only features",
  "Packaging and first-run flow work on target Windows environment"
] as const;

const MINIMUM_SMOKE_SET = [
  { name: "environment preview seam", file: "tests/frontend/environment-manager.test.ts", command: 'node --import tsx tests/frontend/environment-manager.test.ts' },
  { name: "API tester seam", file: "tests/frontend/api-tester-t9.test.ts", command: 'node --import tsx tests/frontend/api-tester-t9.test.ts' },
  { name: "web recorder seam", file: "tests/frontend/web-recorder-t13.test.ts", command: 'node --import tsx tests/frontend/web-recorder-t13.test.ts' },
  { name: "runner/history seam", file: "tests/frontend/test-runner-t16.test.ts", command: 'node --import tsx tests/frontend/test-runner-t16.test.ts' },
  { name: "suite orchestration seam", file: "tests/frontend/suite-runner-t15.test.ts", command: 'node --import tsx tests/frontend/suite-runner-t15.test.ts' },
  { name: "reliability hardening seam", file: "tests/frontend/reliability-hardening-t18.test.ts", command: 'node --import tsx tests/frontend/reliability-hardening-t18.test.ts' },
  { name: "packaging/bootstrap seam", file: "tests/frontend/packaging-bootstrap-t17.test.ts", command: 'node --import tsx tests/frontend/packaging-bootstrap-t17.test.ts' },
  { name: "browser replay contract seam", file: "tests/frontend/browser-replay-t14.test.ts", command: 'node --import tsx tests/frontend/browser-replay-t14.test.ts' },
  { name: "browser-replay-t14-smoke.ts", file: "tests/frontend/browser-replay-t14-smoke.ts", command: 'node --import tsx tests/frontend/browser-replay-t14-smoke.ts' }
] as const;

function ensureEvidenceDirectory(): void {
  mkdirSync(EVIDENCE_DIRECTORY, { recursive: true });
}

function runCommand(command: string): CommandReport {
  const result = spawnSync(command, {
    cwd: PROJECT_ROOT,
    shell: true,
    encoding: "utf8",
    timeout: 120000
  });

  const stdout = (result.stdout ?? "").trim();
  const stderr = (result.stderr ?? "").trim();

  return buildCommandReport(command, result, stdout, stderr);
}

function buildCommandReport(
  command: string,
  result: SpawnSyncReturns<string>,
  stdout: string,
  stderr: string
): CommandReport {
  const statusFromSmoke = parseSmokeStatus(`${stdout}\n${stderr}`);

  if (statusFromSmoke === "SMOKE_BLOCKED") {
    return {
      name: command,
      command,
      status: "blocked",
      exitCode: result.status,
      stdout,
      stderr,
      details: extractFirstMeaningfulLine(stdout, stderr) || "Smoke prerequisite blocked in current environment."
    };
  }

  if (result.status === 0 && !result.error) {
    return {
      name: command,
      command,
      status: "passed",
      exitCode: result.status,
      stdout,
      stderr,
      details: extractFirstMeaningfulLine(stdout, stderr) || "Command completed successfully."
    };
  }

  return {
    name: command,
    command,
    status: "failed",
    exitCode: result.status,
    stdout,
    stderr,
    details: result.error
      ? String(result.error.message || result.error)
      : extractFirstMeaningfulLine(stderr, stdout) || "Command exited with failure."
  };
}

function parseSmokeStatus(content: string): SmokeStatus | null {
  if (content.includes("SMOKE_PASS")) {
    return "SMOKE_PASS";
  }

  if (content.includes("SMOKE_BLOCKED")) {
    return "SMOKE_BLOCKED";
  }

  if (content.includes("SMOKE_FAIL")) {
    return "SMOKE_FAIL";
  }

  return null;
}

function extractFirstMeaningfulLine(...sources: string[]): string {
  for (const source of sources) {
    const line = source
      .split(/\r?\n/)
      .map((entry) => entry.trim())
      .find((entry) => entry.length > 0);

    if (line) {
      return line;
    }
  }

  return "";
}

function sanitizeSnippet(content: string): string {
  const normalized = content.replace(/\r/g, "").trim();
  if (normalized.length === 0) {
    return "(empty)";
  }

  return normalized.length > 500 ? `${normalized.slice(0, 500)}…` : normalized;
}

function runMinimumSmokeSet(): CommandReport[] {
  return MINIMUM_SMOKE_SET.map((entry) => {
    const report = runCommand(entry.command);
    return {
      ...report,
      name: entry.name
    };
  });
}

function findReport(commands: CommandReport[], name: string): CommandReport {
  const report = commands.find((entry) => entry.name === name);
  if (!report) {
    throw new Error(`Missing smoke report for: ${name}`);
  }
  return report;
}

function evaluateMvpExitCriteria(commands: CommandReport[]): CriterionAssessment[] {
  const environmentReport = findReport(commands, "environment preview seam");
  const apiReport = findReport(commands, "API tester seam");
  const recorderReport = findReport(commands, "web recorder seam");
  const suiteReport = findReport(commands, "suite orchestration seam");
  const runnerReport = findReport(commands, "runner/history seam");
  const reliabilityReport = findReport(commands, "reliability hardening seam");
  const packagingReport = findReport(commands, "packaging/bootstrap seam");
  const replaySmokeReport = findReport(commands, "browser-replay-t14-smoke.ts");

  return [
    assessCriterion(
      MVP_EXIT_CRITERIA[0],
      packagingReport,
      "Packaging/bootstrap regression passed, so shell bootstrap/first-run seams are implemented.",
      "Packaging/bootstrap regression is blocked or failing, so empty-dir bootstrap is not fully evidenced."
    ),
    assessCriterion(
      MVP_EXIT_CRITERIA[1],
      environmentReport,
      "Environment preview seam passed, covering environment creation and masked/degraded secret handling.",
      "Environment smoke seam did not pass, so safe environment/secret authoring is not fully evidenced."
    ),
    assessCriterion(
      MVP_EXIT_CRITERIA[2],
      apiReport,
      "API tester seam passed, covering authoring plus pass/fail detail rendering for API runs.",
      "API tester seam did not pass, so API execution/detail behavior is not fully evidenced."
    ),
    assessBrowserReplayCriterion(MVP_EXIT_CRITERIA[3], recorderReport, replaySmokeReport),
    assessMixedSuiteCriterion(MVP_EXIT_CRITERIA[4], suiteReport, runnerReport),
    assessCriterion(
      MVP_EXIT_CRITERIA[5],
      reliabilityReport,
      "Reliability hardening seam passed, explicitly preserving degraded browser isolation from API-only features.",
      "Reliability seam did not pass, so degraded browser isolation from API-only flows is not fully evidenced."
    ),
    assessCriterion(
      MVP_EXIT_CRITERIA[6],
      packagingReport,
      "Packaging/bootstrap seam passed, but only source/seam evidence exists in this environment.",
      "Packaging/bootstrap seam is not passing, so target Windows packaging cannot be considered evidenced."
    )
  ];
}

function assessCriterion(
  name: string,
  report: CommandReport,
  passRationale: string,
  nonPassRationale: string
): CriterionAssessment {
  if (report.status === "passed") {
    return { name, verdict: "SATISFIED", rationale: `${passRationale} Evidence: ${report.details}` };
  }

  if (report.status === "blocked") {
    return { name, verdict: "BLOCKED", rationale: `${nonPassRationale} Blocker: ${report.details}` };
  }

  return { name, verdict: "FAILED", rationale: `${nonPassRationale} Failure: ${report.details}` };
}

function assessBrowserReplayCriterion(
  name: string,
  recorderReport: CommandReport,
  replaySmokeReport: CommandReport
): CriterionAssessment {
  if (recorderReport.status !== "passed") {
    return {
      name,
      verdict: recorderReport.status === "blocked" ? "BLOCKED" : "FAILED",
      rationale: `Recorder seam is not proven: ${recorderReport.details}`
    };
  }

  if (replaySmokeReport.status === "passed") {
    return {
      name,
      verdict: "SATISFIED",
      rationale:
        "Recorder seam passed and replay smoke executed real browser-side interactions successfully. " +
        `Evidence: ${replaySmokeReport.details}`
    };
  }

  if (replaySmokeReport.status === "blocked") {
    return {
      name,
      verdict: "BLOCKED",
      rationale:
        "Recorder UI seam is present, but real Chromium replay smoke is blocked in this environment, so successful simple UI replay cannot be claimed. " +
        `Blocker: ${replaySmokeReport.details}`
    };
  }

  return {
    name,
    verdict: "FAILED",
    rationale: `Recorder seam exists, but real replay smoke failed: ${replaySmokeReport.details}`
  };
}

function assessMixedSuiteCriterion(
  name: string,
  suiteReport: CommandReport,
  runnerReport: CommandReport
): CriterionAssessment {
  if (suiteReport.status === "passed" && runnerReport.status === "passed") {
    return {
      name,
      verdict: "SATISFIED",
      rationale:
        "Suite orchestration and runner/history seams both passed, so mixed suite execution and progress/history inspection are evidenced at seam level."
    };
  }

  if (suiteReport.status === "blocked" || runnerReport.status === "blocked") {
    return {
      name,
      verdict: "BLOCKED",
      rationale:
        `Mixed suite proof is blocked. Suite: ${suiteReport.details}. Runner: ${runnerReport.details}`
    };
  }

  return {
    name,
    verdict: "FAILED",
    rationale: `Mixed suite proof failed. Suite: ${suiteReport.details}. Runner: ${runnerReport.details}`
  };
}

function determineSmokeVerdict(criteria: CriterionAssessment[]): GateVerdict {
  if (criteria.some((criterion) => criterion.verdict === "FAILED")) {
    return "FAIL";
  }

  if (criteria.some((criterion) => criterion.verdict === "BLOCKED")) {
    return "BLOCKED";
  }

  return "PASS";
}

function determineBrowserGateVerdict(commands: CommandReport[]): {
  verdict: GateVerdict;
  rationale: string[];
  fallbackRecommendation: string;
} {
  const recorderReport = findReport(commands, "web recorder seam");
  const replayContractReport = findReport(commands, "browser replay contract seam");
  const replaySmokeReport = findReport(commands, "browser-replay-t14-smoke.ts");
  const reliabilityReport = findReport(commands, "reliability hardening seam");

  if (recorderReport.status !== "passed" || replayContractReport.status !== "passed") {
    return {
      verdict: "FAIL",
      rationale: [
        `Recorder seam status: ${recorderReport.status} (${recorderReport.details})`,
        `Replay contract seam status: ${replayContractReport.status} (${replayContractReport.details})`,
        `Reliability seam status: ${reliabilityReport.status} (${reliabilityReport.details})`
      ],
      fallbackRecommendation:
        "Primary browser track is failing at implemented product seams; fallback planning should be activated immediately if this remains true on a runtime-capable machine."
    };
  }

  if (replaySmokeReport.status === "passed") {
    return {
      verdict: "PASS",
      rationale: [
        `Recorder seam passed: ${recorderReport.details}`,
        `Replay smoke passed with real browser interactions: ${replaySmokeReport.details}`,
        `Reliability seam preserved API-only degraded behavior: ${reliabilityReport.details}`
      ],
      fallbackRecommendation: "Remain on the primary Chromium path; no fallback trigger indicated by current smoke evidence."
    };
  }

  if (replaySmokeReport.status === "blocked") {
    return {
      verdict: "BLOCKED",
      rationale: [
        `Recorder seam passed: ${recorderReport.details}`,
        `Replay smoke is blocked by environment/runtime prerequisites: ${replaySmokeReport.details}`,
        `Reliability seam still proves browser failures do not block API-only features: ${reliabilityReport.details}`
      ],
      fallbackRecommendation:
        "Do not mark browser track viable yet. Re-run on a machine with Chromium/runtime prerequisites before deciding whether to stay on the primary path or trigger fallback."
    };
  }

  return {
    verdict: "FAIL",
    rationale: [
      `Recorder seam passed: ${recorderReport.details}`,
      `Replay smoke failed on real browser execution: ${replaySmokeReport.details}`,
      `Reliability seam status: ${reliabilityReport.status} (${reliabilityReport.details})`
    ],
    fallbackRecommendation:
      "Browser track should be considered failing until replay smoke is repaired; if this reproduces on a runtime-capable machine, trigger the fallback decision path."
  };
}

function renderSmokeSummary(summary: SmokeSummary): string {
  const criteriaLines = summary.criteria.map(
    (criterion, index) => `${index + 1}. ${criterion.name}\n   - ${criterion.verdict}\n   - ${criterion.rationale}`
  );

  const commandLines = summary.commands.map(
    (command, index) =>
      `${index + 1}. ${command.name}\n` +
      `   - command: ${command.command}\n` +
      `   - status: ${command.status}\n` +
      `   - exitCode: ${command.exitCode ?? "null"}\n` +
      `   - details: ${command.details}\n` +
      `   - stdout: ${sanitizeSnippet(command.stdout)}\n` +
      `   - stderr: ${sanitizeSnippet(command.stderr)}`
  );

  return [
    "T19 smoke acceptance summary",
    "",
    `Overall MVP exit criteria verdict: ${summary.verdict}`,
    "",
    "MVP exit criteria:",
    ...criteriaLines,
    "",
    "Minimum smoke scenarios:",
    ...commandLines,
    "",
    "Notes:",
    "- This report evaluates MVP exit criteria against the real product seams available in the current repository.",
    "- browser viability gate is reported separately in task-T19-browser-gate.txt.",
    "- If runtime prerequisites are missing, blocked evidence is preserved as BLOCKED rather than treated as PASS."
  ].join("\n");
}

function renderBrowserGateReport(
  verdict: GateVerdict,
  rationale: string[],
  fallbackRecommendation: string
): string {
  return [
    "T19 Week-6 browser viability gate",
    "",
    `browser viability gate verdict: ${verdict}`,
    "",
    "Rationale:",
    ...rationale.map((entry, index) => `${index + 1}. ${entry}`),
    "",
    `Fallback recommendation: ${fallbackRecommendation}`,
    "",
    "Decision policy:",
    "- PASS: recorder/replay runtime evidence succeeded and degraded isolation remains intact.",
    "- BLOCKED: current environment cannot honestly prove browser viability because runtime prerequisites are missing.",
    "- FAIL: implemented browser seams or runtime smoke demonstrably fail."
  ].join("\n");
}

function writeEvidenceFile(filePath: string, content: string): void {
  mkdirSync(dirname(filePath), { recursive: true });
  writeFileSync(filePath, `${content}\n`, "utf8");
}

function main(): void {
  ensureEvidenceDirectory();
  rmSync(SMOKE_SUMMARY_PATH, { force: true });
  rmSync(BROWSER_GATE_PATH, { force: true });

  const commands = runMinimumSmokeSet();
  const criteria = evaluateMvpExitCriteria(commands);
  const smokeSummary: SmokeSummary = {
    verdict: determineSmokeVerdict(criteria),
    criteria,
    commands
  };
  const browserGate = determineBrowserGateVerdict(commands);

  writeEvidenceFile(SMOKE_SUMMARY_PATH, renderSmokeSummary(smokeSummary));
  writeEvidenceFile(
    BROWSER_GATE_PATH,
    renderBrowserGateReport(browserGate.verdict, browserGate.rationale, browserGate.fallbackRecommendation)
  );

  console.log(`T19 smoke/report completed. MVP exit criteria verdict: ${smokeSummary.verdict}`);
  console.log(`Evidence written: ${SMOKE_SUMMARY_PATH}`);
  console.log(`Evidence written: ${BROWSER_GATE_PATH}`);
}

main();
