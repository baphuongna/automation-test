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

const environmentClientSource = readProjectFile("src/services/environment-client.ts");
const previewAdapterPath = "src/services/environment-preview-client.ts";
const previewAdapterSource = readProjectFile(previewAdapterPath);
const routeSource = readProjectFile("src/routes/environment-manager.tsx");

assert(
  environmentClientSource.includes('"__TAURI_INTERNALS__" in window'),
  "Environment client must detect browser preview by checking for __TAURI_INTERNALS__ before using the fallback adapter."
);

assert(
  environmentClientSource.includes("environmentPreviewClient"),
  "Environment client must delegate to a bounded browser-preview adapter when Tauri runtime is unavailable."
);

assert(
  environmentClientSource.includes("Preview fallback active"),
  "Environment client must expose an obvious preview-fallback signal for T6 browser QA."
);

assert(
  previewAdapterSource.includes("localStorage"),
  "Preview adapter must persist T6 browser-preview CRUD state locally for hands-on QA."
);

assert(
  previewAdapterSource.includes("Production Preview"),
  "Preview adapter must seed at least one production-like environment so the warning banner can be exercised."
);

assert(
  previewAdapterSource.includes("masked by default"),
  "Preview adapter seed data must preserve masked-by-default secret behavior."
);

assert(
  previewAdapterSource.includes("previewDegraded") || previewAdapterSource.includes("degraded"),
  "Preview adapter must provide a deterministic degraded-mode switch for browser QA."
);

assert(
  routeSource.includes("getEnvironmentPreviewBanner") && routeSource.includes("previewBanner"),
  "Environment Manager route must visibly communicate when browser-preview fallback is active."
);

console.log("Environment Manager preview fallback regression test passed.");
