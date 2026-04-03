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

function assertMatch(source: string, pattern: RegExp, message: string): void {
  assert(pattern.test(source), message);
}

function assertNoMatch(source: string, pattern: RegExp, message: string): void {
  assert(!pattern.test(source), message);
}

function extractMappedTypeBlock(source: string, key: string): string {
  const keyIndex = source.indexOf(`"${key}": {`);
  assert(keyIndex >= 0, `Expected mapped type key to exist: ${key}`);

  const blockStart = source.indexOf("{", keyIndex);
  assert(blockStart >= 0, `Expected opening brace for key: ${key}`);

  let depth = 0;
  for (let index = blockStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(blockStart, index + 1);
      }
    }
  }

  throw new Error(`Could not extract mapped type block for key: ${key}`);
}

function extractInterfaceBlock(source: string, interfaceName: string): string {
  const anchorPattern = new RegExp(`export\\s+interface\\s+${interfaceName}\\s*\\{`);
  const anchorMatch = anchorPattern.exec(source);
  assert(anchorMatch !== null, `Expected interface to exist: ${interfaceName}`);

  const blockStart = source.indexOf("{", anchorMatch!.index);
  assert(blockStart >= 0, `Expected opening brace for interface: ${interfaceName}`);

  let depth = 0;
  for (let index = blockStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(blockStart, index + 1);
      }
    }
  }

  throw new Error(`Could not extract interface block: ${interfaceName}`);
}

const commandSource = readProjectFile("src/types/commands.ts");
const dtoSource = readProjectFile("src/types/dto.ts");
const ciClientSource = readProjectFile("src/services/ci-client.ts");
const rustCommandSource = readProjectFile("src-tauri/src/contracts/commands.rs");
const rustDtoSource = readProjectFile("src-tauri/src/contracts/dto.rs");
const runnerRouteSource = readProjectFile("src/routes/test-runner.tsx");
const ciCommandBlock = extractMappedTypeBlock(commandSource, "ci.handoff.execute");
const ciResultDtoBlock = extractInterfaceBlock(dtoSource, "CiHandoffResultDto");

assert(
  commandSource.includes('"ci.handoff.execute": {') &&
    /suiteId\s*:\s*EntityId/.test(ciCommandBlock) &&
    /trigger\s*:\s*\{[\s\S]*source\s*:\s*"ci"[\s\S]*actor\s*:\s*"pipeline"[\s\S]*\}/.test(
      ciCommandBlock
    ) &&
    /output\s*:\s*\{[\s\S]*writeJson\s*:\s*true[\s\S]*outputDir\?\s*:\s*string[\s\S]*fileName\?\s*:\s*string[\s\S]*\}/.test(
      ciCommandBlock
    ),
  "P2-T8 must add a typed ci.handoff.execute command accepting exactly one suite and canonical JSON output metadata."
);

assert(
  commandSource.includes('"ci.handoff.execute": CiHandoffResultDto;') &&
    /runId\s*:\s*EntityId/.test(ciResultDtoBlock) &&
    /suiteId\s*:\s*EntityId/.test(ciResultDtoBlock) &&
    /artifactPath\s*:\s*string/.test(ciResultDtoBlock) &&
    /status\s*:\s*[\s\S]*"passed"/.test(ciResultDtoBlock) &&
    /status\s*:\s*[\s\S]*"failed"/.test(ciResultDtoBlock) &&
    /status\s*:\s*[\s\S]*"blocked"/.test(ciResultDtoBlock) &&
    /exitCode\s*:\s*[\s\S]*0/.test(ciResultDtoBlock) &&
    /exitCode\s*:\s*[\s\S]*1/.test(ciResultDtoBlock) &&
    /exitCode\s*:\s*[\s\S]*2/.test(ciResultDtoBlock),
  "P2-T8 thin invocation response must stay high-level: status/exitCode/artifactPath/runId/suiteId only."
);

assert(
  /executeCiHandoff\s*\(\s*input\s*:\s*CommandPayloadMap\["ci\.handoff\.execute"\]/.test(
    ciClientSource
  ) &&
    /invokeCommand\(\s*"ci\.handoff\.execute"\s*,/.test(ciClientSource),
  "P2-T8 must keep typed IPC seam via client helper and must not leak direct invoke from route code."
);

assertNoMatch(
  ciClientSource,
  /\binvoke\s*\(/,
  "P2-T8 ci-client must not bypass typed IPC helpers with raw invoke calls."
);

assertNoMatch(
  runnerRouteSource,
  /\binvoke\s*\(/,
  "P2-T8 route code must not leak raw invoke calls."
);

assert(
  /pub\s+struct\s+CiHandoffExecuteCommand\b/.test(rustCommandSource) &&
    /pub\s+struct\s+CiHandoffTrigger\b/.test(rustCommandSource) &&
    /pub\s+struct\s+CiHandoffOutput\b/.test(rustCommandSource) &&
    /pub\s+enum\s+CiHandoffSource\b/.test(rustCommandSource) &&
    /pub\s+enum\s+CiHandoffActor\b/.test(rustCommandSource) &&
    /pub\s+struct\s+CiHandoffWriteJson\b/.test(rustCommandSource) &&
    /pub\s+source\s*:\s*CiHandoffSource\s*,/.test(rustCommandSource) &&
    /pub\s+actor\s*:\s*CiHandoffActor\s*,/.test(rustCommandSource) &&
    /pub\s+write_json\s*:\s*CiHandoffWriteJson\s*,/.test(rustCommandSource) &&
    /CiHandoffExecute\(CiHandoffExecuteCommand\)/.test(rustCommandSource),
  "Rust command contracts must mirror the CI handoff typed command seam."
);

assert(
  /pub\s+struct\s+CiHandoffResultDto\b/.test(rustDtoSource) &&
    /pub\s+status\s*:\s*CiHandoffStatus\s*,/.test(rustDtoSource) &&
    /pub\s+exit_code\s*:\s*CiHandoffExitCode\s*,/.test(rustDtoSource) &&
    /pub\s+artifact_path\s*:\s*String\s*,/.test(rustDtoSource) &&
    /pub\s+run_id\s*:\s*EntityId\s*,/.test(rustDtoSource) &&
    /pub\s+suite_id\s*:\s*EntityId\s*,/.test(rustDtoSource) &&
    /pub\s+enum\s+CiHandoffStatus\b/.test(rustDtoSource) &&
    /pub\s+enum\s+CiHandoffExitCode\b/.test(rustDtoSource) &&
    /Passed\s*=\s*0/.test(rustDtoSource) &&
    /Failed\s*=\s*1/.test(rustDtoSource) &&
    /Blocked\s*=\s*2/.test(rustDtoSource),
  "Rust DTO contracts must mirror CI handoff thin result semantics including blocked status and exit code."
);

assert(
  !/\bsuiteIds\b/.test(ciCommandBlock) &&
    !/\bbatch\b/.test(ciCommandBlock) &&
    !/\bcliOnly\b/.test(ciCommandBlock) &&
    !/\bargv\b/.test(ciCommandBlock) &&
    !/executeCiBatchHandoff/.test(ciClientSource) &&
    !/\bsuiteIds\b/.test(ciClientSource),
  "P2-T8 baseline must avoid multi-suite batching and CLI-only paths in typed seams."
);

console.log("P2-T8 CI handoff source contract regression test passed.");
