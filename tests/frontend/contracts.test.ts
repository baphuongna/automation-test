import { ERROR_CODE_TO_FAMILY, errorFamilyFromCode, type ErrorPayload } from "../../src/types/errors";
import type { CommandEnvelope } from "../../src/types/commands";
import type { EventEnvelope } from "../../src/types/events";
import { readFileSync } from "node:fs";
import path from "node:path";

function assertEqual<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(`${message}. expected=${String(expected)} actual=${String(actual)}`);
  }
}

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(message);
  }
}

const payload: ErrorPayload = {
  code: "SECURITY_KEY_CORRUPTED",
  displayMessage: "Không thể mở kho bí mật",
  technicalMessage: "Master key checksum mismatch",
  context: { operation: "decrypt", keyFile: "master.key" },
  recoverable: false
};

assertEqual(errorFamilyFromCode(payload.code), "SECURITY", "Error family mapping failed");
assertEqual(ERROR_CODE_TO_FAMILY.API_REQUEST_FAILED, "API", "API family mapping failed");

const command: CommandEnvelope<"runner.suite.execute"> = {
  command: "runner.suite.execute",
  payload: { suiteId: "suite-1", environmentId: "env-1" }
};

assertEqual(command.payload.environmentId, "env-1", "Command payload mismatch");

const event: EventEnvelope<"app.error"> = {
  event: "app.error",
  payload: {
    scope: "command",
    error: payload
  }
};

assert(event.payload.error.technicalMessage.length > 0, "Error technical message must be present");

const upsertVariableCommand: CommandEnvelope<"environment.variable.upsert"> = {
  command: "environment.variable.upsert",
  payload: {
    environmentId: "env-1",
    variable: {
      id: "var-1",
      key: "API_KEY",
      kind: "secret",
      value: "secret-value"
    }
  }
};

assertEqual(
  upsertVariableCommand.payload.variable.id,
  "var-1",
  "environment.variable.upsert must use nested variable payload"
);

const rustCommandsContract = readFileSync(
  path.resolve(process.cwd(), "src-tauri/src/contracts/commands.rs"),
  "utf-8"
);

assert(
  /pub\s+struct\s+EnvironmentVariableUpsertVariable\s*\{/.test(rustCommandsContract),
  "Rust contract must define nested EnvironmentVariableUpsertVariable payload"
);

assert(
  /pub\s+variable\s*:\s*EnvironmentVariableUpsertVariable\s*,/.test(rustCommandsContract),
  "Rust upsert command must contain nested 'variable' field"
);

assert(
  /EnvironmentList\(EmptyCommandPayload\)/.test(rustCommandsContract),
  "Rust environment.list command must carry explicit empty payload contract"
);

const rustEventsContract = readFileSync(
  path.resolve(process.cwd(), "src-tauri/src/contracts/events.rs"),
  "utf-8"
);

assert(
  /pub\s+enum\s+AppErrorScope\s*\{[\s\S]*Global,[\s\S]*Command,[\s\S]*Runner,[\s\S]*\}/.test(
    rustEventsContract
  ),
  "Rust app.error scope must be constrained by AppErrorScope enum"
);

assert(
  /pub\s+scope\s*:\s*AppErrorScope\s*,/.test(rustEventsContract),
  "Rust AppErrorEvent.scope must use AppErrorScope instead of free-form String"
);
