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

const apiTesterRouteSource = readProjectFile("src/routes/api-tester.tsx");
const apiTesterClientSource = readProjectFile("src/services/api-tester-client.ts");
const apiTesterPreviewClientSource = readProjectFile("src/services/api-tester-preview-client.ts");
const commandSource = readProjectFile("src/types/commands.ts");
const dtoSource = readProjectFile("src/types/dto.ts");
const envStoreSource = readProjectFile("src/store/env-store.ts");
const tauriClientSource = readProjectFile("src/services/tauri-client.ts");

assert(
  apiTesterRouteSource.includes("Collection") &&
    apiTesterRouteSource.includes("Assertions") &&
    apiTesterRouteSource.includes("Response viewer") &&
    apiTesterRouteSource.includes("actual") &&
    apiTesterRouteSource.includes("expected"),
  "API Tester route must render collection, assertion builder, response viewer, and actual-vs-expected result details."
);

assert(
  apiTesterRouteSource.includes("Loading") &&
    apiTesterRouteSource.includes("Empty state") &&
    apiTesterRouteSource.includes("Run request") &&
    apiTesterRouteSource.includes("transport") &&
    apiTesterRouteSource.includes("preflight"),
  "API Tester route must expose explicit loading, empty, run, transport, and preflight states."
);

assert(
  apiTesterClientSource.includes('"api.testcase.upsert"') &&
    apiTesterClientSource.includes('"api.testcase.delete"') &&
    apiTesterClientSource.includes('"api.execute"') &&
    apiTesterClientSource.includes('"__TAURI_INTERNALS__" in window'),
  "API Tester client must stay on the typed IPC surface and support the established browser-preview detection pattern."
);

assert(
  apiTesterPreviewClientSource.includes("localStorage") &&
    apiTesterPreviewClientSource.includes("[REDACTED]") &&
    apiTesterPreviewClientSource.includes("failureKind") &&
    apiTesterPreviewClientSource.includes("requestPreview"),
  "API Tester preview fallback must persist bounded QA state and keep request previews redacted."
);

assert(
  commandSource.includes('"api.testcase.upsert"') &&
    commandSource.includes('"api.execute"') &&
    dtoSource.includes("export interface ApiExecutionResultDto") &&
    dtoSource.includes("export interface ApiAssertionResultDto"),
  "Shared API command and DTO contracts must remain the source of truth for T9 execution UI rendering."
);

assert(
  envStoreSource.includes("activeEnvironmentId") && apiTesterRouteSource.includes("useEnvStore"),
  "API Tester route must reuse environment selection state from the shared env store."
);

assert(
  !apiTesterRouteSource.includes("invoke(") &&
    !apiTesterClientSource.includes("invoke(") &&
    tauriClientSource.includes("{ payload }"),
  "API Tester frontend must not bypass the shared tauri-client invoke boundary."
);

console.log("API Tester T9 regression test passed.");
