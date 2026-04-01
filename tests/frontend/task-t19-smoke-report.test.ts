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

const packageJsonSource = readProjectFile("package.json");
const t19HarnessSource = readProjectFile("tests/frontend/task-t19-smoke-report.ts");
const runAllTestsSource = readProjectFile("tests/frontend/run-all-tests.ts");
const t14SmokeSource = readProjectFile("tests/frontend/browser-replay-t14-smoke.ts");

assert(
  packageJsonSource.includes('"test:t19:smoke"') &&
    packageJsonSource.includes("tests/frontend/task-t19-smoke-report.ts"),
  "T19 must expose one explicit smoke/report command from package.json."
);

assert(
  t19HarnessSource.includes("task-T19-smoke-summary.txt") &&
    t19HarnessSource.includes("task-T19-browser-gate.txt") &&
    t19HarnessSource.includes("SMOKE_PASS") &&
    t19HarnessSource.includes("SMOKE_BLOCKED") &&
    t19HarnessSource.includes("SMOKE_FAIL"),
  "T19 harness must emit the required evidence files and preserve established smoke semantics."
);

assert(
  t19HarnessSource.includes("App can bootstrap from empty local data directory without manual dev tooling") &&
    t19HarnessSource.includes("QA can create/run API test and inspect pass/fail details") &&
    t19HarnessSource.includes("QA can record, edit, save, and replay a simple UI flow in Chromium") &&
    t19HarnessSource.includes("Browser failure paths do not crash API-only features") &&
    t19HarnessSource.includes("Packaging and first-run flow work on target Windows environment"),
  "T19 evidence must evaluate the plan MVP exit criteria explicitly."
);

assert(
  t19HarnessSource.includes("PASS") &&
    t19HarnessSource.includes("BLOCKED") &&
    t19HarnessSource.includes("FAIL") &&
    t19HarnessSource.includes("browser viability gate") &&
    t19HarnessSource.includes("MVP exit criteria"),
  "T19 harness must report explicit MVP and browser gate verdicts with rationale."
);

assert(
  t19HarnessSource.includes("tests/frontend/api-tester-t9.test.ts") &&
    t19HarnessSource.includes("tests/frontend/web-recorder-t13.test.ts") &&
    t19HarnessSource.includes("tests/frontend/test-runner-t16.test.ts") &&
    t19HarnessSource.includes("tests/frontend/reliability-hardening-t18.test.ts") &&
    t19HarnessSource.includes("browser-replay-t14-smoke.ts"),
  "T19 harness must orchestrate the approved minimum smoke set through existing regression seams plus T14 smoke."
);

assert(
  !t19HarnessSource.includes("invoke(") &&
    runAllTestsSource.includes(".test.ts") &&
    t14SmokeSource.includes("type SmokeStatus = \"SMOKE_PASS\" | \"SMOKE_BLOCKED\" | \"SMOKE_FAIL\";"),
  "T19 must remain a thin reporting wrapper over existing seams without bypassing established frontend boundaries."
);

console.log("T19 smoke/report regression test passed.");
