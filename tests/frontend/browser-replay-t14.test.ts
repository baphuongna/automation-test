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
const rustCommandContractSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustEventContractSource = readProjectFile("src-tauri/src/contracts/events.rs");
const rustDtoContractSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustStateSource = readProjectFile("src-tauri/src/state.rs");
const rustBrowserServiceSource = readProjectFile("src-tauri/src/services/browser_automation_service.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");
const migrationSource = readProjectFile("src-tauri/migrations/003_add_artifact_manifests.sql");
const smokeHarnessSource = readProjectFile("tests/frontend/browser-replay-t14-smoke.ts");
const packageSource = readProjectFile("package.json");

assert(
  tsCommandSource.includes('"browser.replay.start": {') &&
    tsCommandSource.includes('"browser.replay.start": UiReplayResultDto;'),
  "T14 phải map browser.replay.start sang UiReplayResultDto để trả semantic replay result đầy đủ."
);

assert(
  tsEventSource.includes('"browser.replay.progress": {') &&
    tsEventSource.includes("currentStepId?: EntityId"),
  "T14 phải emit progress event browser.replay.progress với currentStepId typed contract."
);

assert(
  tsDtoSource.includes("export interface UiReplayResultDto") &&
    tsDtoSource.includes("status: ReplayStatus") &&
    tsDtoSource.includes("failedStepId?:") &&
    tsDtoSource.includes("screenshotPath?:"),
  "T14 phải giữ shared UiReplayResultDto contract có status + failedStepId + screenshotPath."
);

assert(
  rustCommandContractSource.includes("pub struct BrowserReplayStartCommand") &&
    rustCommandContractSource.includes('#[serde(rename = "browser.replay.start")]') &&
    rustCommandContractSource.includes("BrowserReplayStart(BrowserReplayStartCommand)"),
  "T14 phải có Rust command contract browser.replay.start trong command envelope."
);

assert(
  rustEventContractSource.includes("pub struct BrowserReplayProgressEvent") &&
    rustEventContractSource.includes('#[serde(rename = "browser.replay.progress")]'),
  "T14 phải dùng BrowserReplayProgressEvent ổn định cho progress replay."
);

assert(
  rustDtoContractSource.includes("pub struct UiReplayResultDto") &&
    rustDtoContractSource.includes("pub screenshot_path: Option<String>"),
  "T14 phải giữ Rust UiReplayResultDto có screenshot_path cho failure artifact path."
);

assert(
  rustStateSource.includes("ReplayState") &&
    rustStateSource.includes("start_replay") &&
    rustStateSource.includes("cancel_replay") &&
    rustStateSource.includes("finish_replay"),
  "T14 cần state machine replay riêng với start/cancel/finish idempotent để không orphaned state."
);

assert(
  rustBrowserServiceSource.includes("start_replay") &&
    rustBrowserServiceSource.includes("emit_replay_progress") &&
    rustBrowserServiceSource.includes("ArtifactService") &&
    rustBrowserServiceSource.includes("ArtifactKind::Screenshot") &&
    rustBrowserServiceSource.includes("persist_artifact_manifest") &&
    rustBrowserServiceSource.includes("browser.replay.progress") &&
    rustBrowserServiceSource.includes("UiReplayResultDto"),
  "T14 phải chạy replay tuần tự trong BrowserAutomationService, emit progress và lưu screenshot manifest khi fail."
);

assert(
  rustBrowserServiceSource.includes("for step in") &&
    rustBrowserServiceSource.includes("ReplayStatus::Running") &&
    rustBrowserServiceSource.includes("ReplayStatus::Passed") &&
    rustBrowserServiceSource.includes("ReplayStatus::Failed") &&
    rustBrowserServiceSource.includes("ReplayStatus::Cancelled"),
  "T14 replay phải thực thi step tuần tự và phát status transitions running/passed/failed/cancelled."
);

assert(
  !rustBrowserServiceSource.includes("thread::sleep") &&
    rustBrowserServiceSource.includes("std::process::Command") &&
    rustBrowserServiceSource.includes("--headless") &&
    rustBrowserServiceSource.includes("execute_step"),
  "T14 replay execution không được là sleep/validate giả; phải đi qua runtime adapter thực thi Chromium thật."
);

assert(
  !rustBrowserServiceSource.includes("unsupported_interaction_error(") &&
    !rustBrowserServiceSource.includes("validate_selector_presence(") &&
    !rustBrowserServiceSource.includes("record_interaction(") &&
    !rustBrowserServiceSource.includes("interaction_history(") &&
    !rustBrowserServiceSource.includes("has_text_in_interactions") &&
    rustBrowserServiceSource.includes("runtime.click(") &&
    rustBrowserServiceSource.includes("runtime.fill(") &&
    rustBrowserServiceSource.includes("runtime.select(") &&
    rustBrowserServiceSource.includes("runtime.set_checked(") &&
    rustBrowserServiceSource.includes("execute_interaction(") &&
    rustBrowserServiceSource.includes("NODE_CDP_INTERACTION_SCRIPT") &&
    rustBrowserServiceSource.includes("Runtime.evaluate"),
  "T14 replay interaction phải tạo browser-side state thật qua runtime executor, không chỉ validate selector + ghi local memory."
);

assert(
  rustBrowserServiceSource.includes("ui_script_steps") &&
    rustBrowserServiceSource.includes("step_order") &&
    rustBrowserServiceSource.includes("script_id") &&
    rustBrowserServiceSource.includes("SELECT"),
  "T14 phải đọc saved UI steps từ schema hiện có thay vì tạo storage mới."
);

assert(
  rustBrowserServiceSource.includes("timeout") &&
    rustBrowserServiceSource.includes("step") &&
    rustBrowserServiceSource.includes("cancel"),
  "T14 phải có timeout/step-level failure/cancel semantics rõ ràng trong replay executor."
);

assert(
  rustBrowserServiceSource.includes("BrowserRuntimeStatus::Healthy") &&
    rustBrowserServiceSource.includes("runtime_status != BrowserRuntimeStatus::Healthy") &&
    rustBrowserServiceSource.includes("Browser flows are blocked"),
  "T14 phải block replay khi runtime không healthy (bao gồm degraded), theo semantics T11."
);

assert(
  rustBrowserServiceSource.includes("screenshot_path") &&
    rustBrowserServiceSource.includes("failed_step_id"),
  "T14 failure result cần trả failed_step_id và screenshot_path khi có artifact."
);

assert(
  !rustBrowserServiceSource.includes("fs::write(&screenshot_path, [])") &&
    rustBrowserServiceSource.includes("--screenshot") &&
    rustBrowserServiceSource.includes("capture_failure_screenshot"),
  "T14 screenshot-on-fail phải tạo screenshot bytes thật từ Chromium runtime, không được tạo file rỗng."
);

assert(
  migrationSource.includes("artifact_manifests") &&
    migrationSource.includes("relative_path LIKE 'screenshots/%'"),
  "T14 phải reuse ArtifactService + artifact_manifests baseline cho screenshot-on-fail."
);

assert(
  rustLibSource.includes("pub fn browser_replay_start") &&
    rustLibSource.includes("UiReplayResultDto") &&
    rustLibSource.includes("BrowserAutomationService::new"),
  "T14 phải thêm backend handler browser_replay_start trả UiReplayResultDto qua BrowserAutomationService boundary."
);

assert(
  rustMainSource.includes("browser_replay_start") &&
    rustMainSource.includes("generate_handler![") &&
    rustMainSource.includes("browser_replay_start"),
  "T14 phải đăng ký browser_replay_start trong Tauri invoke handler."
);

assert(
  smokeHarnessSource.includes("BROWSER_REPLAY_T14_SMOKE") &&
    smokeHarnessSource.includes("SMOKE_PASS") &&
    smokeHarnessSource.includes("SMOKE_BLOCKED") &&
    smokeHarnessSource.includes("SMOKE_FAIL") &&
    smokeHarnessSource.includes("chromium executable") &&
    smokeHarnessSource.includes("node") &&
    smokeHarnessSource.includes("NODE_CDP_INTERACTION_SCRIPT") &&
    smokeHarnessSource.includes("Runtime.evaluate") &&
    smokeHarnessSource.includes("file://"),
  "T14 cần smoke harness trung thực: kiểm tra prerequisite, exercise interaction runtime thật trên target deterministic, và không pass giả khi thiếu runtime."
);

assert(
  packageSource.includes('"test:t14:smoke"') &&
    packageSource.includes("browser-replay-t14-smoke.ts"),
  "T14 smoke harness cần có entrypoint chạy trực tiếp từ package scripts để orchestrator verify nhanh."
);

console.log("Browser replay T14 regression/source test passed.");
