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
  clientSource.includes("invokeCommand(\"runner.suite.list\"") &&
    clientSource.includes("invokeCommand(\"runner.run.history\"") &&
    clientSource.includes("invokeCommand(\"runner.run.detail\"") &&
    clientSource.includes("invokeCommand(\"runner.suite.execute\"") &&
    !clientSource.includes("invoke("),
  "T16 phải thêm thin runner client cho suite list, run history, run detail và vẫn giữ raw invoke phía sau tauri-client."
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
  rustLibSource.includes("pub fn runner_suite_list") &&
    rustLibSource.includes("pub fn runner_run_history") &&
    rustLibSource.includes("pub fn runner_run_detail") &&
    rustLibSource.includes("RunnerRepository::new"),
  "T16 phải expose Tauri handlers tối thiểu cho suite list, run history, và run detail."
);

assert(
  rustMainSource.includes("runner_suite_list") &&
    rustMainSource.includes("runner_run_history") &&
    rustMainSource.includes("runner_run_detail"),
  "T16 phải đăng ký read-side runner handlers trong generate_handler của Tauri app."
);

assert(
  runStoreSource.includes("subscribeRunnerEvents") && tauriClientSource.includes("{ payload }"),
  "T16 phải tiếp tục dùng run-store + typed tauri-client seams hiện hữu thay vì tạo live transport mới."
);

console.log("Test Runner T16 regression/source test passed.");
