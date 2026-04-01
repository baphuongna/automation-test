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
const tauriConfigSource = readProjectFile("src-tauri/tauri.conf.json");
const tsCommandSource = readProjectFile("src/types/commands.ts");
const tsDtoSource = readProjectFile("src/types/dto.ts");
const tauriClientSource = readProjectFile("src/services/tauri-client.ts");
const appSource = readProjectFile("src/App.tsx");
const statusBarSource = readProjectFile("src/components/StatusBar.tsx");
const rustLibSource = readProjectFile("src-tauri/src/lib.rs");
const rustMainSource = readProjectFile("src-tauri/src/main.rs");
const rustStateSource = readProjectFile("src-tauri/src/state.rs");
const rustPathsSource = readProjectFile("src-tauri/src/utils/paths.rs");
const rustBrowserServiceSource = readProjectFile("src-tauri/src/services/browser_automation_service.rs");

assert(
  packageJsonSource.includes('"version": "0.1.0"'),
  "T17 regression assumes package.json remains the canonical package version source."
);

assert(
  !tauriConfigSource.includes('"beforeDevCommand": "npm run dev"'),
  "T17 phải bỏ beforeDevCommand chạy npm run dev khỏi Tauri config để packaging/dev flow không lệch khỏi build-only policy."
);

assert(
  tsCommandSource.includes('"shell.metadata.get": Record<string, never>;') &&
    tsCommandSource.includes('"shell.metadata.get": ShellMetadataDto;'),
  "T17 phải thêm typed command shell.metadata.get ở TypeScript contract map."
);

assert(
  tsDtoSource.includes("export interface ShellMetadataDto") &&
    tsDtoSource.includes("appVersion: string;") &&
    tsDtoSource.includes("isFirstRun: boolean;") &&
    tsDtoSource.includes("browserRuntime: BrowserHealthDto;"),
  "T17 phải thêm ShellMetadataDto tối thiểu cho version/bootstrap/browser runtime summary."
);

assert(
  tauriClientSource.includes('invokeCommand("shell.metadata.get", {})') &&
    tauriClientSource.includes("export async function getShellMetadata()"),
  "T17 phải giữ typed IPC trong tauri-client và expose helper getShellMetadata()."
);

assert(
  appSource.includes("getShellMetadata") &&
    appSource.includes("<StatusBar") &&
    appSource.includes("shellMetadata=") &&
    appSource.includes("runtimeStatusMessage="),
  "T17 phải fetch shell metadata trong App và truyền xuống StatusBar qua props thay vì hardcode tại component."
);

assert(
  !statusBarSource.includes("v0.1.0") &&
    statusBarSource.includes("shellMetadata.appVersion") &&
    statusBarSource.includes("runtimeStatusMessage") &&
    statusBarSource.includes("Browser automation unavailable") &&
    statusBarSource.includes("API/data features remain usable"),
  "T17 phải hiển thị version runtime-canonical và shell-level runtime guidance actionable trong StatusBar."
);

assert(
  rustLibSource.includes("pub fn shell_metadata_get") &&
    rustLibSource.includes("ShellMetadataDto") &&
    rustLibSource.includes("BrowserAutomationService::new") &&
    rustMainSource.includes("shell_metadata_get"),
  "T17 phải thêm backend handler shell_metadata_get và đăng ký nó trong main.rs."
);

assert(
  rustStateSource.includes("pub struct ShellBootstrapSnapshot") &&
    rustStateSource.includes("pub fn shell_bootstrap_snapshot") &&
    rustStateSource.includes("is_first_run"),
  "T17 phải lưu bootstrap snapshot tối thiểu trong AppState để shell đọc qua seam typed hiện có."
);

assert(
  rustPathsSource.includes("pub fn settings_file_existed_before_bootstrap") ||
    rustPathsSource.includes("pub fn detect_first_run") ||
    rustMainSource.includes("is_first_run"),
  "T17 phải suy ra first-run/bootstrap metadata từ seam AppPaths/bootstrap hiện có thay vì invent subsystem khác."
);

assert(
  rustBrowserServiceSource.includes("Browser flows are blocked while API-only features remain available") ||
    rustBrowserServiceSource.includes("API/data features remain usable"),
  "T17 runtime guidance phải nói rõ browser flows unavailable/degraded nhưng API/data features vẫn dùng được."
);

console.log("Packaging/bootstrap T17 regression/source test passed.");
