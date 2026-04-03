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

const rustStateSource = readProjectFile("src-tauri/src/state.rs");
const rustRunnerServiceSource = readProjectFile("src-tauri/src/services/runner_orchestration_service.rs");
const rustBrowserServiceSource = readProjectFile("src-tauri/src/services/browser_automation_service.rs");
const rustApiServiceSource = readProjectFile("src-tauri/src/services/api_execution_service.rs");
const rustRunnerRepositorySource = readProjectFile("src-tauri/src/repositories/runner_repository.rs");
const tsRunStoreSource = readProjectFile("src/store/run-store.ts");
const tsRunnerRouteSource = readProjectFile("src/routes/test-runner.tsx");
const rustErrorSource = readProjectFile("src-tauri/src/error.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const tsAppSource = readProjectFile("src/App.tsx");
const rustSchedulerServiceSource = readProjectFile("src-tauri/src/services/scheduler_service.rs");

assert(
  rustStateSource.includes("pub fn cancel_recording(&self, expected_test_case_id: &str) -> AppResult<bool>") &&
    rustStateSource.includes("RecordingState::Idle => Ok(false)") &&
    rustStateSource.includes("pub fn stop_recording(") &&
    rustStateSource.includes("AppResult<Option<RecordingSnapshot>>") &&
    rustStateSource.includes("pub fn cancel_replay(&self, expected_run_id: &str) -> AppResult<bool>"),
  "T18 phải làm stop/cancel recording/replay idempotent ngay tại AppState seam thay vì ném lỗi khi flow đã terminal/idle."
);

assert(
  rustBrowserServiceSource.includes("let Some(snapshot) = state.stop_recording(test_case_id)? else") &&
    rustBrowserServiceSource.includes("state.cancel_recording(test_case_id)?") &&
    rustBrowserServiceSource.includes("if changed {") &&
    rustBrowserServiceSource.includes("Browser flows are blocked while API-only features remain available") &&
    rustBrowserServiceSource.includes("Recording session is no longer active") &&
    rustBrowserServiceSource.includes("Selector not found for") &&
    rustBrowserServiceSource.includes("Ui test case không còn script replay hợp lệ"),
  "T18 phải harden browser service cho cancel/stop idempotent, degraded messaging rõ ràng, session-loss/deleted-reference errors cụ thể, và browser failure không kéo theo API-only semantics."
);

assert(
  rustRunnerServiceSource.includes("update_run_summary_if_active") &&
    rustRunnerServiceSource.includes("insert_case_result_if_absent") &&
    rustRunnerServiceSource.includes("RunStatus::Cancelled") &&
    rustRunnerServiceSource.includes("Suite rỗng, không có test case để chạy."),
  "T18 phải thêm guard orchestration để terminal run chỉ finalize một lần và không persist duplicate completion/cancel rows."
);

assert(
  rustRunnerRepositorySource.includes("pub fn insert_case_result_if_absent(") &&
    rustRunnerRepositorySource.includes("WHERE NOT EXISTS") &&
    rustRunnerRepositorySource.includes("completed_at IS NULL") &&
    rustRunnerRepositorySource.includes("pub fn update_run_summary_if_active("),
  "T18 phải harden repository persistence để chặn duplicate result/finalization records ở seam lưu runner history."
);

assert(
  rustApiServiceSource.includes("BODY_PREVIEW_TRUNCATED") &&
    rustApiServiceSource.includes("normalize_body_preview_bytes") &&
    rustApiServiceSource.includes("String::from_utf8_lossy") &&
    rustApiServiceSource.includes("API_REQUEST_BUILD_FAILED: missing variable") &&
    rustErrorSource.includes("Khóa mã hóa không khả dụng. Vui lòng cấu hình lại master key."),
  "T18 phải giữ missing-variable/corrupted-key errors cụ thể tại API execution seam và truncate preview oversized an toàn theo byte boundary."
);

assert(
  tsRunStoreSource.includes("terminalMessage") &&
    !tsRunStoreSource.includes("isStopping: payload.status === \"cancelled\"") &&
    tsRunStoreSource.includes("status: payload.status") &&
    tsRunStoreSource.includes("runner.execution.completed"),
  "T18 phải ngừng conflates cancelled với stopping trong run-store và giữ terminal feedback riêng, trung thực với backend semantics."
);

assert(
  tsRunnerRouteSource.includes("Already cancelling") &&
    tsRunnerRouteSource.includes("No active run right now.") &&
    tsRunnerRouteSource.includes("terminalMessage") &&
    tsRunnerRouteSource.includes("No persisted runner history matches the current filter"),
  "T18 route phải phản ánh cancel idempotent và cleanup/empty-history state rõ ràng mà không invent flow mới."
);

assert(
  rustErrorSource.includes("pub fn variable_missing") &&
    rustErrorSource.includes("pub fn secret_key_missing") &&
    rustLibSource.includes("Err(error @ TestForgeError::Validation(_)) => Ok(to_preflight_api_result(error))") &&
    tsAppSource.includes("Browser automation unavailable. Browser flows are blocked while API/data features remain usable."),
  "T18 phải giữ browser degraded độc lập với API/data usability và tiếp tục surfacing lỗi cụ thể thay vì fallback generic."
);

assert(
  rustStateSource.includes("scheduler_runtime_started") &&
    rustStateSource.includes("mark_scheduler_started") &&
    rustStateSource.includes("is_scheduler_started") &&
    rustSchedulerServiceSource.includes("schedule_tick") &&
    rustSchedulerServiceSource.includes("state.run_state()") &&
    rustSchedulerServiceSource.includes("RunState::Running") &&
    rustSchedulerServiceSource.includes("Blocked: another suite run is already active") &&
    rustSchedulerServiceSource.includes("trigger_due_schedules") &&
    rustLibSource.includes("start_scheduler_loop"),
  "P2-T7 Chunk 3 phải harden scheduler runtime để chỉ bootstrap một loop, tôn trọng active-run guard, và lưu blocked diagnostics trung thực thay vì spawn execution chồng lấn."
);

console.log("Reliability hardening T18 regression/source test passed.");
