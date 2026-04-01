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

const tsCommandSource = readProjectFile("src/types/commands.ts");
const tsEventSource = readProjectFile("src/types/events.ts");
const tsDtoSource = readProjectFile("src/types/dto.ts");
const tsTauriClientSource = readProjectFile("src/services/tauri-client.ts");
const tsRunStoreSource = readProjectFile("src/store/run-store.ts");
const tsRunnerClientPath = resolve("src/services/runner-client.ts");
const rustCommandContractSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustEventContractSource = readProjectFile("src-tauri/src/contracts/events.rs");
const rustDtoContractSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustStateSource = readProjectFile("src-tauri/src/state.rs");
const rustApiServiceSource = readProjectFile("src-tauri/src/services/api_execution_service.rs");
const rustRunnerServiceSource = readProjectFile("src-tauri/src/services/runner_orchestration_service.rs");
const rustRunnerRepositorySource = readProjectFile("src-tauri/src/repositories/runner_repository.rs");
const rustBrowserServiceSource = readProjectFile("src-tauri/src/services/browser_automation_service.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");
const rustRepositoriesModSource = readProjectFile("src-tauri/src/repositories/mod.rs");
const rustServicesModSource = readProjectFile("src-tauri/src/services/mod.rs");
const migrationSource = readProjectFile("src-tauri/migrations/001_initial_schema.sql");

assert(
  tsCommandSource.includes('"runner.suite.execute": {') &&
    tsCommandSource.includes("rerunFailedFromRunId?: EntityId") &&
    tsCommandSource.includes('"runner.suite.cancel": {') &&
    tsCommandSource.includes('"runner.suite.cancel": { cancelled: true };'),
  "T15 phải mở rộng typed command contract để suite execute hỗ trợ rerun-failed từ run đã persist và cancel response idempotent."
);

assert(
  rustCommandContractSource.includes("pub struct RunnerSuiteExecuteCommand") &&
    rustCommandContractSource.includes("pub rerun_failed_from_run_id: Option<EntityId>") &&
    rustCommandContractSource.includes('#[serde(rename = "runner.suite.execute")]') &&
    rustCommandContractSource.includes("RunnerSuiteExecute(RunnerSuiteExecuteCommand)"),
  "T15 phải mirror runner.suite.execute contract ở Rust với field rerun_failed_from_run_id optional."
);

assert(
  tsEventSource.includes('"runner.execution.started": {') &&
    tsEventSource.includes("environmentId: EntityId") &&
    tsEventSource.includes('"runner.execution.progress": {') &&
    tsEventSource.includes("completedCount: number") &&
    tsEventSource.includes("totalCount: number") &&
    tsEventSource.includes("passedCount: number") &&
    tsEventSource.includes("failedCount: number") &&
    tsEventSource.includes("skippedCount: number") &&
    tsEventSource.includes("dataRowId?: EntityId") &&
    tsEventSource.includes('"runner.execution.completed": RunResultDto;'),
  "T15 phải phát runner progress events đủ dữ liệu aggregate/provenance để frontend theo dõi mixed suite execution chính xác."
);

assert(
  rustEventContractSource.includes("pub struct RunnerExecutionStartedEvent") &&
    rustEventContractSource.includes("pub environment_id: EntityId") &&
    rustEventContractSource.includes("pub struct RunnerExecutionProgressEvent") &&
    rustEventContractSource.includes("pub completed_count: u32") &&
    rustEventContractSource.includes("pub total_count: u32") &&
    rustEventContractSource.includes("pub passed_count: u32") &&
    rustEventContractSource.includes("pub failed_count: u32") &&
    rustEventContractSource.includes("pub skipped_count: u32") &&
    rustEventContractSource.includes("pub data_row_id: Option<EntityId>") &&
    rustEventContractSource.includes('#[serde(rename = "runner.execution.completed")]'),
  "T15 phải đồng bộ Rust runner events với aggregate counts và data row provenance."
);

assert(
  tsDtoSource.includes("export interface RunResultDto") &&
    tsDtoSource.includes("suiteId?: EntityId") &&
    tsDtoSource.includes("environmentId?: EntityId") &&
    tsDtoSource.includes("totalCount: number") &&
    tsDtoSource.includes("skippedCount: number"),
  "T15 phải mở rộng RunResultDto shared contract để summary persisted run đủ total/skipped/suite/environment context."
);

assert(
  rustDtoContractSource.includes("pub struct RunResultDto") &&
    rustDtoContractSource.includes("pub suite_id: Option<EntityId>") &&
    rustDtoContractSource.includes("pub environment_id: Option<EntityId>") &&
    rustDtoContractSource.includes("pub total_count: u32") &&
    rustDtoContractSource.includes("pub skipped_count: u32"),
  "T15 phải mirror RunResultDto aggregate fields ở Rust contract."
);

assert(
  rustStateSource.includes("cancel_requested: bool") &&
    rustStateSource.includes("request_run_cancel") &&
    rustStateSource.includes("is_run_cancel_requested") &&
    rustStateSource.includes("finish_run") &&
    rustStateSource.includes("pub fn stop_run(&self, expected_run_id: &str)"),
  "T15 cần mở rộng RunState theo pattern replay để cancel suite run idempotent và cleanup bằng runId an toàn."
);

assert(
  existsSync(resolve("src-tauri/src/repositories/runner_repository.rs")) &&
    existsSync(resolve("src-tauri/src/services/runner_orchestration_service.rs")) &&
    rustRepositoriesModSource.includes("pub mod runner_repository;") &&
    rustRepositoriesModSource.includes("pub use runner_repository::RunnerRepository;") &&
    rustServicesModSource.includes("pub mod runner_orchestration_service;") &&
    rustServicesModSource.includes("pub use runner_orchestration_service::RunnerOrchestrationService;"),
  "T15 phải thêm repository/service runner chuyên biệt thay vì nhồi orchestration vào frontend hoặc service hiện có."
);

assert(
  rustApiServiceSource.includes("ExecutionPersistenceTarget") &&
    rustApiServiceSource.includes("execute_for_suite_run") &&
    !rustApiServiceSource.includes("suite orchestration creates a separate ad-hoc run"),
  "T15 phải tách persistence seam của API execution để suite orchestration reuse suite-owned run thay vì luôn tạo run mới."
);

assert(
  rustBrowserServiceSource.includes("start_replay_for_suite_run") &&
    rustBrowserServiceSource.includes("UiReplayResultDto") &&
    rustBrowserServiceSource.includes("request_replay_cancel") &&
    !rustBrowserServiceSource.includes("leak playwright internals to runner"),
  "T15 phải cho runner orchestration gọi UI replay qua BrowserAutomationService seam riêng mà không lộ browser internals."
);

assert(
  migrationSource.includes("CREATE TABLE IF NOT EXISTS test_runs") &&
    migrationSource.includes("CREATE TABLE IF NOT EXISTS test_run_results") &&
    migrationSource.includes("data_row_id TEXT") &&
    !migrationSource.includes("suite_run_results"),
  "T15 phải persist suite run vào schema hiện có test_runs/test_run_results, không tạo persistence model thứ hai."
);

assert(
  rustLibSource.includes("pub async fn runner_suite_execute") &&
    rustLibSource.includes("pub fn runner_suite_cancel") &&
    rustLibSource.includes("RunnerOrchestrationService::new") &&
    rustLibSource.includes("runner.execution.started") &&
    rustLibSource.includes("runner.execution.completed"),
  "T15 phải expose Tauri handlers cho runner suite execute/cancel và đi qua RunnerOrchestrationService."
);

assert(
  rustMainSource.includes("runner_suite_execute") && rustMainSource.includes("runner_suite_cancel"),
  "T15 phải đăng ký runner suite handlers trong generate_handler của Tauri app."
);

assert(
  existsSync(tsRunnerClientPath) &&
    readFileSync(tsRunnerClientPath, "utf8").includes('invokeCommand("runner.suite.execute"') &&
    readFileSync(tsRunnerClientPath, "utf8").includes('invokeCommand("runner.suite.cancel"') &&
    !readFileSync(tsRunnerClientPath, "utf8").includes("invoke("),
  "T15 chỉ được thêm seam frontend tối thiểu qua runner-client và phải giữ raw invoke nằm trong tauri-client boundary."
);

assert(
  tsTauriClientSource.includes("function toTauriCommandName") && !tsTauriClientSource.includes('runner.suite.execute" as any'),
  "T15 phải tiếp tục dùng typed tauri-client seam thay vì bypass raw invoke mapping."
);

assert(
  tsRunStoreSource.includes("activeRunId") &&
    tsRunStoreSource.includes("progress") &&
    tsRunStoreSource.includes("setRunState") &&
    tsRunStoreSource.includes("skipped") &&
    tsRunStoreSource.includes("subscribeRunnerEvents") &&
    tsRunStoreSource.includes("runner.execution.progress"),
  "T15 chỉ nên thêm seam-level run-store wiring tối thiểu cho runner events/progress; không build T16 UI ở đây."
);

assert(
  rustRunnerServiceSource.includes("build_execution_plan") &&
    rustRunnerServiceSource.includes("HashSet<(String, Option<String>)>") &&
    rustRunnerServiceSource.includes("RunStatus::Skipped") &&
    rustRunnerServiceSource.includes("finalize_failed_run") &&
    !rustRunnerServiceSource.includes("let rerun_case_ids ="),
  "T15 fix phải lập execution plan theo target case+dataRow, dùng skipped semantics đúng, và có failure finalization rõ ràng thay vì rerun theo case-level set."
);

assert(
  rustRunnerRepositorySource.includes('"skipped"') &&
    rustRunnerRepositorySource.includes("RunStatus::Skipped") &&
    rustRunnerRepositorySource.includes("RunStatus::Running"),
  "T15 fix phải cho repository map running/skipped semantics đúng để persisted summary không kẹt queued hoặc misclassify skipped thành cancelled."
);

console.log("Suite runner T15 regression/source test passed.");
