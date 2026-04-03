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

function assertIncludesAll(source: string, needles: string[], message: string): void {
  assert(needles.every((needle) => source.includes(needle)), message);
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
  existsSync(resolve("src-tauri/icons/icon.ico")),
  "P2-T4 source/seam regression requires a real Windows icon asset at src-tauri/icons/icon.ico so packaged-resource generation remains wired."
);

assert(
  packageJsonSource.includes('"version": "0.1.0"'),
  "P2-T4 source/seam regression assumes package.json remains the canonical package version source for packaged metadata alignment."
);

assert(
  !tauriConfigSource.includes('"beforeDevCommand": "npm run dev"'),
  "P2-T4 phải giữ Tauri config tránh beforeDevCommand chạy npm run dev để packaging/bootstrap seam tiếp tục bám build-only policy."
);

assertIncludesAll(
  tsCommandSource,
  ['"shell.metadata.get": Record<string, never>;', '"shell.metadata.get": ShellMetadataDto;'],
  "P2-T4 phải giữ typed command shell.metadata.get trong TypeScript command contract map."
);

assertIncludesAll(
  tsDtoSource,
  [
    "export interface ShellMetadataDto",
    "appVersion: string;",
    "isFirstRun: boolean;",
    "browserRuntime: BrowserHealthDto;"
  ],
  "P2-T4 phải giữ ShellMetadataDto tối thiểu cho version/bootstrap/browser runtime summary."
);

assertIncludesAll(
  tauriClientSource,
  ['invokeCommand("shell.metadata.get", {})', "export async function getShellMetadata()"],
  "P2-T4 phải giữ typed IPC trong tauri-client và expose helper getShellMetadata()."
);

assertIncludesAll(
  appSource,
  ["getShellMetadata", "<StatusBar", "shellMetadata="],
  "P2-T4 phải fetch shell metadata trong App và truyền shellMetadata xuống StatusBar qua seam typed hiện có."
);

assert(
  !statusBarSource.includes("v0.1.0"),
  "P2-T4 không được hardcode version UI; packaged shell metadata phải là nguồn runtime-canonical."
);

assertIncludesAll(
  statusBarSource,
  [
    "shellMetadata.appVersion",
    "shellMetadata.browserRuntime.message",
    "Browser automation unavailable",
    "API/data features remain usable"
  ],
  "P2-T4 phải hiển thị version runtime-canonical và runtime guidance actionable trực tiếp từ shell metadata trong StatusBar."
);

assertIncludesAll(
  rustLibSource,
  [
    "fn shell_metadata_get",
    "ShellMetadataDto",
    "BrowserAutomationService::new",
    "tauri::generate_handler![",
    "shell_metadata_get"
  ],
  "P2-T4 phải giữ backend handler shell_metadata_get và đăng ký nó ở library command surface."
);

assert(
  rustMainSource.includes("testforge::run();"),
  "P2-T4 phải giữ packaged shell bootstrap đi qua library run() entrypoint."
);

assertIncludesAll(
  rustStateSource,
  ["pub struct ShellBootstrapSnapshot", "pub fn shell_bootstrap_snapshot", "is_first_run"],
  "P2-T4 phải lưu bootstrap snapshot tối thiểu trong AppState để shell đọc qua seam typed hiện có."
);

assert(
  rustPathsSource.includes("pub fn settings_file_existed_before_bootstrap") ||
    rustPathsSource.includes("pub fn detect_first_run") ||
    rustMainSource.includes("is_first_run"),
  "P2-T4 phải suy ra first-run/bootstrap metadata từ seam AppPaths/bootstrap hiện có thay vì invent subsystem khác."
);

assert(
  rustBrowserServiceSource.includes("Browser flows are blocked while API-only features remain available") ||
    rustBrowserServiceSource.includes("API/data features remain usable"),
  "P2-T4 runtime guidance phải nói rõ browser flows unavailable/degraded nhưng API/data features vẫn dùng được."
);

console.log(
  "Packaging/bootstrap T17 source/seam regression passed. Runtime-packaged proof remains a separate requirement."
);
