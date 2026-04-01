import assert from "assert/strict";
import { readFileSync } from "fs";
import { resolve } from "path";

const packageJson = JSON.parse(
  readFileSync(resolve("package.json"), "utf8"),
) as {
  scripts?: Record<string, string>;
  devDependencies?: Record<string, string>;
};

const secretServiceSource = readFileSync(
  resolve("src-tauri/src/services/secret_service.rs"),
  "utf8",
);

assert.equal(
  typeof packageJson.scripts?.lint,
  "string",
  "package.json must expose an executable lint script for F2 evidence",
);

assert.ok(
  packageJson.scripts?.lint?.trim().length,
  "lint script must not be empty",
);

assert.ok(
  secretServiceSource.includes("key.as_slice()"),
  "secret_service.rs must pass a byte slice into Aes256Gcm::new_from_slice",
);

assert.ok(
  !secretServiceSource.includes("Aes256Gcm::new_from_slice(key)"),
  "secret_service.rs must not pass Zeroizing directly into Aes256Gcm::new_from_slice",
);
