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

const routeSource = readProjectFile("src/routes/test-runner.tsx");
const clientSource = readProjectFile("src/services/runner-client.ts");
const commandSource = readProjectFile("src/types/commands.ts");
const dtoSource = readProjectFile("src/types/dto.ts");
const runStoreSource = readProjectFile("src/store/run-store.ts");
const tauriClientSource = readProjectFile("src/services/tauri-client.ts");
const rustCommandContractSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustDtoContractSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustRepositorySource = readProjectFile("src-tauri/src/repositories/runner_repository.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");

assert(
  routeSource.includes("Runner control") &&
    routeSource.includes("Run history") &&
    routeSource.includes("Run detail") &&
    routeSource.includes("Rerun failed") &&
    routeSource.includes("artifact") &&
    routeSource.includes("failure category") &&
    !routeSource.includes("Placeholder screen for running suites"),
  "T16 route phải thay placeholder bằng màn runner/history/detail thật với rerun-failed, artifact links, và failure category rõ ràng."
);

assert(
  routeSource.includes("subscribeRunnerEvents") &&
    routeSource.includes("useRunStore") &&
    routeSource.includes("completed") &&
    routeSource.includes("passed") &&
    routeSource.includes("failed") &&
    routeSource.includes("skipped"),
  "T16 route phải tái sử dụng run-store và event seam hiện có để hiển thị active progress/counters."
);

assert(
  routeSource.includes("Promise.all([") &&
    routeSource.includes("runnerClient.listSuites()") &&
    routeSource.includes("environmentClient.list()") &&
    routeSource.includes("runnerClient.listRunHistory(selectedSuiteId ? { suiteId: selectedSuiteId } : {})") &&
    routeSource.includes("setHistory(historyItems)") &&
    routeSource.includes("const selection = await hydrateSelectedRun({") &&
    routeSource.includes("previousSelectedRunId: selectedRunId") &&
    routeSource.includes("const runDetail = await runnerClient.getRunDetail({ runId: nextSelectedRunId });"),
  "T16 route phải hydrate suites + environments + run history cùng lúc và tự load run detail từ persisted selection thay vì chỉ render read-side placeholder."
);

assert(
  routeSource.includes("const selectedRun = useMemo(") &&
    routeSource.includes("if (!selectedRun || !selectedRun.suiteId || !activeEnvironmentId)") &&
    routeSource.includes("rerunFailedFromRunId: selectedRun.runId") &&
    routeSource.includes("Rerun failed accepted from historical run") &&
    routeSource.includes("selectedRun.failedCount > 0") &&
    routeSource.includes("Select a persisted suite run and environment before rerunning failures."),
  "T16 route phải buộc rerun-failed xuất phát từ historical failed run đã persist và chỉ mở action khi target scope hợp lệ."
);

assert(
  routeSource.includes("if (!activeRunId)") &&
    routeSource.includes("Already cancelling the active run. Waiting for terminal update.") &&
    routeSource.includes("Cancel requested for active run") &&
    routeSource.includes("No active run right now.") &&
    routeSource.includes("disabled={!activeRunId || isCancelling || isStopping}"),
  "T16 route phải phản ánh cancel idempotent guard semantics trung thực trong desktop runner surface."
);

assert(
  routeSource.includes("Per-case results, artifacts, and sanitized previews") &&
    routeSource.includes("Select a historical run to inspect per-case/per-row results, failure category, artifact links,") &&
    routeSource.includes("Sanitized request preview") &&
    routeSource.includes("Sanitized response preview") &&
    routeSource.includes("Assertion preview") &&
    routeSource.includes("No per-row artifacts."),
  "T16 route phải cho detail inspection seams rõ ràng: per-row results, failure category, artifact links, và sanitized previews thay vì summary-only history." 
);

assert(
  clientSource.includes("invokeCommand(\"runner.suite.list\"") &&
    clientSource.includes("invokeCommand(\"runner.run.history\"") &&
    clientSource.includes("invokeCommand(\"runner.run.detail\"") &&
    clientSource.includes("invokeCommand(\"runner.suite.execute\"") &&
    !clientSource.includes("invoke("),
  "T16 phải thêm thin runner client cho suite list, run history, run detail và vẫn giữ raw invoke phía sau tauri-client."
);

assert(
  clientSource.includes("async listSuites()") &&
    clientSource.includes("async listRunHistory(input: { suiteId?: string } = {})") &&
    clientSource.includes("const payload = input.suiteId ? { suiteId: input.suiteId } : {};") &&
    clientSource.includes("async getRunDetail(input: { runId: string })") &&
    clientSource.includes("async executeSuite(input: { suiteId: string; environmentId: string } | { suiteId: string; environmentId: string; rerunFailedFromRunId: string })") &&
    clientSource.includes("throw result.error ?? new Error(\"Missing command result for runner.run.detail\")"),
  "T16 runner client phải giữ read/write seams mỏng nhưng đủ để hydrate history/detail và rerun-failed mà không nuốt lỗi command result."
);

assert(
  commandSource.includes('"runner.suite.list": Record<string, never>;') &&
    commandSource.includes('"runner.run.history": {') &&
    commandSource.includes("suiteId?: EntityId") &&
    commandSource.includes('"runner.run.detail": {') &&
    commandSource.includes('"runner.suite.list": SuiteDto[];') &&
    commandSource.includes('"runner.run.history": RunHistoryEntryDto[];') &&
    commandSource.includes('"runner.run.detail": RunDetailDto;'),
  "T16 phải mở rộng typed command contracts cho suite list, run history, và run detail read-side seams."
);

assert(
  dtoSource.includes("export interface RunHistoryEntryDto") &&
    dtoSource.includes("suiteName") &&
    dtoSource.includes("environmentName") &&
    dtoSource.includes("export interface RunDetailDto") &&
    dtoSource.includes("results: RunCaseResultDto[]") &&
    dtoSource.includes("artifacts: ArtifactManifestDto[]") &&
    dtoSource.includes("failureCategory") &&
    dtoSource.includes("requestPreview") &&
    dtoSource.includes("responsePreview"),
  "T16 phải thêm DTO rõ ràng cho history/detail, gồm artifact manifests, sanitized previews, và failure category."
);

assert(
  rustCommandContractSource.includes("pub struct RunnerRunHistoryCommand") &&
    rustCommandContractSource.includes("pub struct RunnerRunDetailCommand") &&
    rustCommandContractSource.includes('#[serde(rename = "runner.suite.list")]') &&
    rustCommandContractSource.includes('#[serde(rename = "runner.run.history")]') &&
    rustCommandContractSource.includes('#[serde(rename = "runner.run.detail")]'),
  "T16 phải mirror read-side runner commands ở Rust contracts để frontend/backend không drift."
);

assert(
  rustDtoContractSource.includes("pub struct RunHistoryEntryDto") &&
    rustDtoContractSource.includes("pub struct RunDetailDto") &&
    rustDtoContractSource.includes("pub struct RunCaseResultDto") &&
    rustDtoContractSource.includes("pub failure_category: String") &&
    rustDtoContractSource.includes("pub request_preview: String") &&
    rustDtoContractSource.includes("pub response_preview: String"),
  "T16 phải mirror Rust DTO contracts cho history/detail với preview sanitized và failure category."
);

assert(
  rustRepositorySource.includes("pub fn list_suites") &&
    rustRepositorySource.includes("pub fn list_run_history") &&
    rustRepositorySource.includes("pub fn load_run_detail") &&
    rustRepositorySource.includes("artifact_manifests") &&
    rustRepositorySource.includes("test_run_results"),
  "T16 phải đọc persisted suites/runs/results/artifacts từ repository hiện có thay vì invent persistence path mới."
);

assert(
  rustRepositorySource.includes("ORDER BY COALESCE(tr.completed_at, tr.started_at, tr.created_at) DESC") &&
    rustRepositorySource.includes("LEFT JOIN test_suites ts ON ts.id = tr.suite_id") &&
    rustRepositorySource.includes("JOIN environments env ON env.id = tr.environment_id") &&
    rustRepositorySource.includes("sanitize_preview_text(&request_log)") &&
    rustRepositorySource.includes("sanitize_preview_text(&response_log)") &&
    rustRepositorySource.includes("classify_failure_category(&error_code, &error_message)") &&
    rustRepositorySource.includes("artifact.file_path == *path || artifact.relative_path == *path"),
  "T16 repository seam phải hydrate history/detail theo persisted ordering, environment/suite context, failure-category derivation, sanitized previews, và artifact linkage thật."
);

assert(
  rustLibSource.includes("fn runner_suite_list") &&
    rustLibSource.includes("fn runner_run_history") &&
    rustLibSource.includes("fn runner_run_detail") &&
    rustLibSource.includes("RunnerRepository::new"),
  "T16 phải expose Tauri handlers tối thiểu cho suite list, run history, và run detail."
);

assert(
  rustLibSource.includes("tauri::generate_handler![") &&
    rustLibSource.includes("runner_suite_list") &&
    rustLibSource.includes("runner_run_history") &&
    rustLibSource.includes("runner_run_detail") &&
    rustMainSource.includes("testforge::run();"),
  "T16 phải đăng ký read-side runner handlers thông qua library run() entrypoint của Tauri app."
);

assert(
  runStoreSource.includes("subscribeRunnerEvents") && tauriClientSource.includes("{ payload }"),
  "T16 phải tiếp tục dùng run-store + typed tauri-client seams hiện hữu thay vì tạo live transport mới."
);

assert(
  runStoreSource.includes("activeRunId: payload.runId") &&
    runStoreSource.includes("progress: {") &&
    runStoreSource.includes("completed: payload.completedCount") &&
    runStoreSource.includes("failed: payload.failedCount") &&
    runStoreSource.includes("skipped: payload.skippedCount") &&
    runStoreSource.includes("payload.status === \"cancelled\"") &&
    runStoreSource.includes("Run cancelled safely. No active run remains."),
  "T16 run-store phải wire progress/completion payload thật để history/detail refresh dựa trên runner semantics thay vì local-only UI state."
);

console.log("Test Runner T16 regression/source test passed.");
