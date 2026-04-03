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

const dtoSource = readFileSync(path.resolve(process.cwd(), "src/types/dto.ts"), "utf-8");
const ciClientSource = readFileSync(path.resolve(process.cwd(), "src/services/ci-client.ts"), "utf-8");

const ciHandoffCommand: CommandEnvelope<"ci.handoff.execute"> = {
  command: "ci.handoff.execute",
  payload: {
    suiteId: "suite-ci-1",
    trigger: {
      source: "ci",
      actor: "pipeline"
    },
    output: {
      writeJson: true,
      outputDir: "exports/ci",
      fileName: "ci-execution-suite-ci-1.json"
    }
  }
};

assertEqual(ciHandoffCommand.payload.trigger.source, "ci", "ci.handoff.execute must lock trigger.source to ci");
assertEqual(ciHandoffCommand.payload.output.writeJson, true, "ci.handoff.execute must require canonical json output");

assert(
  dtoSource.includes("export interface CiHandoffResultDto") &&
    dtoSource.includes('status: "passed" | "failed" | "blocked"') &&
    dtoSource.includes("artifactPath: string") &&
    dtoSource.includes("runId: EntityId") &&
    dtoSource.includes("suiteId: EntityId") &&
    dtoSource.includes("exitCode: 0 | 1 | 2"),
  "TS DTO contract must define thin CiHandoffResultDto with stable status/exit/path/run fields"
);

assert(
  ciClientSource.includes('invokeCommand("ci.handoff.execute"') &&
    ciClientSource.includes("async executeCiHandoff(input:") &&
    !ciClientSource.includes("invoke("),
  "CI handoff seam must use typed invokeCommand client helper without raw invoke leakage"
);
