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

const tsDomainSource = readProjectFile("src/types/domain.ts");
const tsDtoSource = readProjectFile("src/types/dto.ts");
const tsCommandSource = readProjectFile("src/types/commands.ts");
const tsEventSource = readProjectFile("src/types/events.ts");
const rustDomainSource = readProjectFile("src-tauri/src/contracts/domain.rs");
const rustDtoSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const rustCommandSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustEventSource = readProjectFile("src-tauri/src/contracts/events.rs");
const rustStateSource = readProjectFile("src-tauri/src/state.rs");
const rustBrowserServiceSource = readProjectFile("src-tauri/src/services/browser_automation_service.rs");
const rustServicesModSource = readProjectFile("src-tauri/src/services/mod.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");
const migrationSource = readProjectFile("src-tauri/migrations/001_initial_schema.sql");

assert(
  tsDomainSource.includes("export const RECORDING_STATUSES") &&
    tsDomainSource.includes('"failed"') &&
    tsDomainSource.includes('"stopped"'),
  "T12 requires stable recording status contracts including stopped/failed states."
);

assert(
  tsDtoSource.includes("export interface UiStepDto") &&
    tsDtoSource.includes("action: StepAction") &&
    tsDtoSource.includes("timeoutMs?: number"),
  "T12 must keep UiStepDto aligned as typed normalized step payload for recorder events and persistence output."
);

assert(
  tsCommandSource.includes('"browser.recording.start"') &&
    tsCommandSource.includes('"browser.recording.stop"') &&
    tsCommandSource.includes('"browser.recording.cancel"') &&
    tsCommandSource.includes('"browser.recording.stop": UiTestCaseDto;') &&
    tsCommandSource.includes('"browser.recording.cancel": { cancelled: true };'),
  "T12 must expose typed start/stop/cancel recording commands with stop returning persisted UiTestCaseDto."
);

assert(
  tsEventSource.includes('"browser.recording.status.changed"') &&
    tsEventSource.includes('"browser.recording.step.captured"'),
  "T12 must reuse existing typed recording event names for status and captured-step streaming."
);

assert(
  rustDomainSource.includes("pub enum StepAction") &&
    rustDomainSource.includes("Navigate") &&
    rustDomainSource.includes("Click") &&
    rustDomainSource.includes("Fill") &&
    rustDomainSource.includes("Select") &&
    rustDomainSource.includes("Check") &&
    rustDomainSource.includes("Uncheck") &&
    rustDomainSource.includes("WaitFor") &&
    rustDomainSource.includes("AssertText"),
  "T12 normalization must stay inside Phase 1 step model only (navigate/click/fill/select/check/uncheck/wait_for/assert_text)."
);

assert(
  rustCommandSource.includes("pub struct BrowserRecordingStartCommand") &&
    rustCommandSource.includes("pub struct BrowserRecordingStopCommand") &&
    rustCommandSource.includes("pub struct BrowserRecordingCancelCommand") &&
    rustCommandSource.includes('#[serde(rename = "browser.recording.cancel")]') &&
    rustCommandSource.includes("BrowserRecordingCancel(BrowserRecordingCancelCommand)"),
  "T12 must define browser.recording.cancel command contract in Rust."
);

assert(
  rustEventSource.includes('#[serde(rename = "browser.recording.status.changed")]') &&
    rustEventSource.includes('#[serde(rename = "browser.recording.step.captured")]'),
  "T12 must continue using stable recorder event contracts in Rust."
);

assert(
  rustStateSource.includes("pub enum RecordingState") &&
    rustStateSource.includes("Failed") &&
    rustStateSource.includes("last_error") &&
    rustStateSource.includes("recoverable") &&
    rustStateSource.includes("captured_steps") &&
    rustStateSource.includes("cancel_recording") &&
    rustStateSource.includes("record_captured_step"),
  "T12 requires recording state machine with active/failed recovery metadata, buffered steps, and cancel semantics."
);

assert(
  rustServicesModSource.includes("pub mod browser_automation_service") &&
    rustServicesModSource.includes("pub use browser_automation_service::BrowserAutomationService"),
  "T12 recorder pipeline must remain behind BrowserAutomationService boundary."
);

assert(
  rustBrowserServiceSource.includes("start_recording") &&
    rustBrowserServiceSource.includes("stop_recording") &&
    rustBrowserServiceSource.includes("cancel_recording") &&
    rustBrowserServiceSource.includes("normalize") &&
    rustBrowserServiceSource.includes("confidence") &&
    rustBrowserServiceSource.includes("ui_scripts") &&
    rustBrowserServiceSource.includes("ui_script_steps") &&
    rustBrowserServiceSource.includes("browser.recording.status.changed") &&
    rustBrowserServiceSource.includes("browser.recording.step.captured"),
  "T12 must add recorder pipeline in BrowserAutomationService: normalize + confidence + persistence + typed events."
);

assert(
  rustLibSource.includes("pub fn browser_recording_start") &&
    rustLibSource.includes("pub fn browser_recording_stop") &&
    rustLibSource.includes("pub fn browser_recording_cancel") &&
    rustLibSource.includes("BrowserAutomationService::new"),
  "T12 must add backend handlers for recording start/stop/cancel using BrowserAutomationService."
);

assert(
  rustMainSource.includes("browser_recording_start") &&
    rustMainSource.includes("browser_recording_stop") &&
    rustMainSource.includes("browser_recording_cancel"),
  "T12 must register recording start/stop/cancel handlers in Tauri invoke handler."
);

assert(
  rustDtoSource.includes("pub struct UiTestCaseDto") &&
    rustDtoSource.includes("pub steps: Vec<UiStepDto>"),
  "T12 stop flow must return persisted UiTestCaseDto contract."
);

assert(
  migrationSource.includes("CREATE TABLE IF NOT EXISTS ui_scripts") &&
    migrationSource.includes("CREATE TABLE IF NOT EXISTS ui_script_steps") &&
    migrationSource.includes("step_type TEXT NOT NULL") &&
    migrationSource.includes("confidence TEXT DEFAULT 'high'"),
  "T12 must persist recorder output in existing ui_scripts/ui_script_steps schema with confidence field."
);

console.log("Browser recording T12 contract/regression test passed.");
