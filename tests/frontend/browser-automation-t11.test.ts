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
const rustCommandContractSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustServicesModSource = readProjectFile("src-tauri/src/services/mod.rs");
const rustBrowserServiceSource = readProjectFile("src-tauri/src/services/browser_automation_service.rs");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");
const tsEventsSource = readProjectFile("src/types/events.ts");
const rustEventContractsSource = readProjectFile("src-tauri/src/contracts/events.rs");
const rustDtoContractsSource = readProjectFile("src-tauri/src/contracts/dto.rs");

assert(
  tsCommandSource.includes('"browser.health.check": Record<string, never>;') &&
    tsCommandSource.includes('"browser.health.check": BrowserHealthDto;'),
  "T11 phải thêm typed command browser.health.check ở TypeScript contract map."
);

assert(
  rustCommandContractSource.includes("pub struct BrowserHealthCheckCommand") &&
    rustCommandContractSource.includes('#[serde(rename = "browser.health.check")]') &&
    rustCommandContractSource.includes("BrowserHealthCheck(BrowserHealthCheckCommand)"),
  "T11 phải thêm browser.health.check command contract ở Rust với payload rỗng rõ ràng."
);

assert(
  rustServicesModSource.includes("pub mod browser_automation_service") &&
    rustServicesModSource.includes("pub use browser_automation_service::BrowserAutomationService"),
  "T11 phải expose BrowserAutomationService trong service layer."
);

assert(
  rustBrowserServiceSource.includes("pub struct BrowserAutomationService") &&
    rustBrowserServiceSource.includes("pub fn check_runtime_health") &&
    rustBrowserServiceSource.includes("BrowserRuntimeStatus::Healthy") &&
    rustBrowserServiceSource.includes("BrowserRuntimeStatus::Degraded") &&
    rustBrowserServiceSource.includes("BrowserRuntimeStatus::Unavailable") &&
    rustBrowserServiceSource.includes("chromium"),
  "T11 phải có BrowserAutomationService với runtime health check và semantics healthy/degraded/unavailable cho Chromium-only baseline."
);

assert(
  rustBrowserServiceSource.includes("resolved runtime candidate") &&
    rustBrowserServiceSource.includes("checked candidates") &&
    rustBrowserServiceSource.includes("node runtime") &&
    rustBrowserServiceSource.includes("cdp runtime") &&
    rustBrowserServiceSource.includes("source=") &&
    rustBrowserServiceSource.includes("reason="),
  "P2-T1 health diagnostics phải nêu rõ runtime source/path, candidate checks và prerequisite node/cdp để tránh healthy giả."
);

assert(
  rustBrowserServiceSource.includes("app.emit(\"browser.health.changed\"") ||
    rustBrowserServiceSource.includes("emit(\"browser.health.changed\""),
  "T11 phải có foundation phát event browser.health.changed từ browser service."
);

assert(
  rustLibSource.includes("fn browser_health_check") &&
    rustLibSource.includes("BrowserHealthDto") &&
    rustLibSource.includes("BrowserAutomationService::new"),
  "T11 phải thêm backend handler browser_health_check trả BrowserHealthDto thông qua BrowserAutomationService."
);

assert(
  rustLibSource.includes("tauri::generate_handler![") &&
    rustLibSource.includes("browser_health_check") &&
    rustMainSource.includes("testforge::run();"),
  "T11 phải đăng ký browser_health_check thông qua library run() entrypoint của Tauri runtime."
);

assert(
  tsEventsSource.includes('"browser.health.changed": BrowserHealthDto;') &&
    rustEventContractsSource.includes('#[serde(rename = "browser.health.changed")]') &&
    rustDtoContractsSource.includes("pub struct BrowserHealthDto") &&
    rustDtoContractsSource.includes("pub runtime_status: BrowserRuntimeStatus"),
  "T11 phải reuse contract event/DTO browser health hiện có thay vì tạo shape rời rạc."
);

console.log("Browser automation T11 regression/source test passed.");
