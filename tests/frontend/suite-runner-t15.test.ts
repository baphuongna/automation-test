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

function includesAll(source: string, fragments: string[]): boolean {
  return fragments.every((fragment) => source.includes(fragment));
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
const schedulingMigrationPath = resolve("src-tauri/migrations/004_add_suite_schedules.sql");

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
    rustRepositoriesModSource.includes("RunnerRepository") &&
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
  rustRunnerServiceSource.includes("match target.test_case_type") &&
    rustRunnerServiceSource.includes("TestCaseType::Api =>") &&
    rustRunnerServiceSource.includes("TestCaseType::Ui =>") &&
    rustRunnerServiceSource.includes("execute_api_target") &&
    rustRunnerServiceSource.includes("execute_ui_target") &&
    !rustRunnerServiceSource.includes("preview-only"),
  "T15 phải chứng minh orchestration mixed suite thật sự điều phối cả API và UI targets qua seam backend riêng, không dựa preview-only path."
);

assert(
  rustRunnerServiceSource.includes("load_failed_targets") &&
    rustRunnerServiceSource.includes("collect::<HashSet<(String, Option<String>)>>()") &&
    rustRunnerServiceSource.includes("rerun_target_keys.contains(&(case.test_case_id.clone(), Some(row.id.clone())))") &&
    rustRunnerServiceSource.includes("rerun_target_keys.contains(&(case.test_case_id.clone(), None))") &&
    !rustRunnerServiceSource.includes("rerun theo case-level set"),
  "T15 phải giới hạn rerun-failed theo target case+dataRow đã fail thay vì nới scope ở level test case chung chung."
);

assert(
  rustRunnerRepositorySource.includes("SELECT DISTINCT trr.case_id, trr.data_row_id") &&
    rustRunnerRepositorySource.includes("WHERE trr.run_id = ?1 AND tr.suite_id = ?2 AND trr.status = 'failed'") &&
    rustRunnerRepositorySource.includes("pub struct FailedRunTarget") &&
    rustRunnerRepositorySource.includes("pub data_row_id: Option<String>"),
  "T15 phải giữ rerun-failed repository seam dựa trên failed persisted targets gồm cả dataRow provenance."
);

assert(
  rustRunnerServiceSource.includes("state.is_run_cancel_requested(run_id)?") &&
    rustRunnerServiceSource.includes("RunStatus::Cancelled") &&
    rustRunnerServiceSource.includes("finalize_run(") &&
    rustRunnerServiceSource.includes("if !updated {") &&
    rustRunnerServiceSource.includes("return self.runner_repository.load_run_result(run_id);") &&
    rustStateSource.includes("pub fn request_run_cancel(&self, expected_run_id: &str)") &&
    rustStateSource.includes("if *cancel_requested {") &&
    rustStateSource.includes("RunState::Idle => Ok(false)") &&
    rustStateSource.includes("pub fn finish_run(&self, expected_run_id: &str)"),
  "T15 phải giữ cancel idempotent xuyên suốt app state + orchestration finalization để repeated cancel hoặc terminal cleanup không tạo completion giả."
);

assert(
  rustRunnerServiceSource.includes("RunnerExecutionStartedEvent") &&
    rustRunnerServiceSource.includes("RunnerExecutionProgressEvent") &&
    rustRunnerServiceSource.includes("app.emit(\"runner.execution.completed\"") &&
    rustRunnerServiceSource.includes("data_row_id: data_row_id.map(ToOwned::to_owned)") &&
    rustRunnerServiceSource.includes("completed_count") &&
    rustRunnerServiceSource.includes("failed_count"),
  "T15 phải phát started/progress/completed events từ orchestration thật để progress/history/detail desktop path bám cùng persisted run semantics."
);

assert(
  migrationSource.includes("CREATE TABLE IF NOT EXISTS test_runs") &&
    migrationSource.includes("CREATE TABLE IF NOT EXISTS test_run_results") &&
    migrationSource.includes("data_row_id TEXT") &&
    !migrationSource.includes("suite_run_results"),
  "T15 phải persist suite run vào schema hiện có test_runs/test_run_results, không tạo persistence model thứ hai."
);

assert(
  includesAll(tsCommandSource, [
    '"scheduler.schedule.list": Record<string, never>;',
    '"scheduler.schedule.upsert": {',
    '"scheduler.schedule.setEnabled": {',
    '"scheduler.schedule.delete": {'
  ]) &&
    includesAll(tsDtoSource, ["export interface SuiteScheduleDto", "cadenceMinutes: number", "lastRunStatus?"]) &&
    includesAll(rustCommandContractSource, [
      '#[serde(rename = "scheduler.schedule.list")]',
      '#[serde(rename = "scheduler.schedule.upsert")]',
      '#[serde(rename = "scheduler.schedule.setEnabled")]',
      '#[serde(rename = "scheduler.schedule.delete")]'
    ]) &&
    rustDtoContractSource.includes("pub struct SuiteScheduleDto") &&
    existsSync(schedulingMigrationPath),
  "P2-T7 phải khóa scheduling contract/migration seam nhưng vẫn reuse runner orchestration/history pipeline hiện có thay vì tạo suite execution model riêng."
);

assert(
  rustLibSource.includes("fn runner_suite_execute") &&
    rustLibSource.includes("fn runner_suite_cancel") &&
    rustLibSource.includes("RunnerOrchestrationService::new") &&
    rustLibSource.includes("runner.execution.started") &&
    rustLibSource.includes("runner.execution.completed"),
  "T15 phải expose Tauri handlers cho runner suite execute/cancel và đi qua RunnerOrchestrationService."
);

assert(
  rustLibSource.includes("start_scheduler_loop") &&
    rustLibSource.includes("SchedulerService::new") &&
    rustLibSource.includes("schedule_tick") &&
    rustLibSource.includes("RunnerOrchestrationService::new") &&
    rustLibSource.includes(".setup(|app| {") &&
    rustLibSource.includes("start_scheduler_loop(app.handle().clone(), Arc::clone(&result.app_state))?;"),
  "P2-T7 Chunk 3 phải bootstrap local scheduler loop ngay từ Tauri setup path và trigger qua RunnerOrchestrationService thay vì tạo execution pipeline riêng."
);

assert(
  rustLibSource.includes("tauri::generate_handler![") &&
    rustLibSource.includes("runner_suite_execute") &&
    rustLibSource.includes("runner_suite_cancel") &&
    rustMainSource.includes("testforge::run();"),
  "T15 phải đăng ký runner suite handlers thông qua library run() entrypoint của Tauri app."
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
