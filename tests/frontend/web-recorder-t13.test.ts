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

const webRecorderRouteSource = readProjectFile("src/routes/web-recorder.tsx");
const webRecorderClientSource = readProjectFile("src/services/web-recorder-client.ts");
const webRecorderPreviewClientSource = readProjectFile("src/services/web-recorder-preview-client.ts");
const tauriClientSource = readProjectFile("src/services/tauri-client.ts");
const commandsSource = readProjectFile("src/types/commands.ts");
const eventsSource = readProjectFile("src/types/events.ts");
const dtoSource = readProjectFile("src/types/dto.ts");
const hookSource = readProjectFile("src/hooks/useTauriEvent.ts");
const cssSource = readProjectFile("src/index.css");

assert(
  webRecorderRouteSource.includes("Preflight") &&
    webRecorderRouteSource.includes("Session status") &&
    webRecorderRouteSource.includes("Captured steps") &&
    webRecorderRouteSource.includes("Step editor") &&
    webRecorderRouteSource.includes("Start recording") &&
    webRecorderRouteSource.includes("Stop and save") &&
    webRecorderRouteSource.includes("Cancel session"),
  "Web Recorder route must replace the placeholder with preflight, session status, live stream, step editor, and recorder controls."
);

assert(
  webRecorderRouteSource.includes("Loading") &&
    webRecorderRouteSource.includes("No draft yet") &&
    webRecorderRouteSource.includes("Conflict blocked") &&
    webRecorderRouteSource.includes("recoverable") &&
    webRecorderRouteSource.includes("Low confidence"),
  "Web Recorder route must expose explicit loading, empty draft, conflict-blocked, recoverable failure, and low-confidence states."
);

assert(
  webRecorderRouteSource.includes("browser.health.changed") &&
    webRecorderRouteSource.includes("browser.recording.status.changed") &&
    webRecorderRouteSource.includes("browser.recording.step.captured") &&
    webRecorderRouteSource.includes("useTauriEvent"),
  "Web Recorder route must subscribe to typed browser health, recording status, and step captured events via useTauriEvent."
);

assert(
  webRecorderRouteSource.includes("selector") &&
    webRecorderRouteSource.includes("timeoutMs") &&
    webRecorderRouteSource.includes("Move up") &&
    webRecorderRouteSource.includes("Move down") &&
    webRecorderRouteSource.includes("Add step") &&
    webRecorderRouteSource.includes("Delete"),
  "Web Recorder editor must support selector/value/timeout editing plus add, delete, and reorder controls."
);

assert(
  webRecorderClientSource.includes('"browser.health.check"') &&
    webRecorderClientSource.includes('"browser.recording.start"') &&
    webRecorderClientSource.includes('"browser.recording.stop"') &&
    webRecorderClientSource.includes('"browser.recording.cancel"') &&
    webRecorderClientSource.includes('"ui.testcase.upsert"') &&
    webRecorderClientSource.includes('"ui.testcase.delete"') &&
    webRecorderClientSource.includes('"__TAURI_INTERNALS__" in window'),
  "Web Recorder client must stay on the typed IPC surface and reuse the established preview detection pattern."
);

assert(
  !webRecorderRouteSource.includes("invoke(") &&
    !webRecorderClientSource.includes("invoke(") &&
    tauriClientSource.includes("{ payload }"),
  "Web Recorder frontend must not bypass the shared tauri-client invoke boundary."
);

assert(
  commandsSource.includes('"browser.health.check"') &&
    commandsSource.includes('"browser.recording.start"') &&
    commandsSource.includes('"browser.recording.stop"') &&
    commandsSource.includes('"browser.recording.cancel"') &&
    eventsSource.includes('"browser.health.changed"') &&
    eventsSource.includes('"browser.recording.status.changed"') &&
    eventsSource.includes('"browser.recording.step.captured"') &&
    hookSource.includes("listen<EventPayloadMap"),
  "Web Recorder must consume the existing typed T12 command/event seams through useTauriEvent."
);

assert(
  hookSource.includes("window.addEventListener") &&
    hookSource.includes("CustomEvent") &&
    hookSource.includes("detail") &&
    webRecorderPreviewClientSource.includes("window.dispatchEvent") &&
    webRecorderPreviewClientSource.includes("browser.recording.step.captured"),
  "Preview fallback must provide a recorder-local event path equivalent to the typed Tauri event stream so captured steps reach the live editor immediately."
);

assert(
  dtoSource.includes("confidence?:") || dtoSource.includes("confidence:"),
  "UiStepDto must surface recorder confidence so the UI can highlight low-confidence steps deterministically."
);

assert(
  cssSource.includes(".web-recorder__") &&
    cssSource.includes("web-recorder__step--low-confidence") &&
    cssSource.includes("web-recorder__feedback--error") &&
    cssSource.includes("web-recorder__status-badge--blocked"),
  "Web Recorder styles must define dedicated recorder layout, low-confidence emphasis, error feedback, and conflict-blocked badge states."
);

console.log("Web Recorder T13 regression test passed.");
