# P2-T8 CI/CD Handoff Contract Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the narrowest useful CI/CD handoff path to TestForge so one suite execution can produce a deterministic, secret-safe JSON artifact with stable `passed` / `failed` / `blocked` semantics for external pipeline consumption.

**Architecture:** Keep P2-T8 as a thin contract layer on top of the existing runner pipeline. Trigger execution through the current orchestration service, project the persisted result into a versioned CI handoff DTO, and persist that DTO as a filesystem-backed artifact through the existing artifact policy instead of inventing a second execution or reporting model.

**Tech Stack:** Tauri v2, Rust, tokio, rusqlite, React 18, TypeScript, existing typed IPC contracts, existing runner repository/orchestration services, existing artifact/redaction services.

---

## File Structure Map

- Create: `src-tauri/src/services/ci_handoff_service.rs`
  - Hold the thin orchestration wrapper that maps one suite execution into CI handoff request/response semantics and normalizes `passed` / `failed` / `blocked` states.
- Modify: `src-tauri/src/services/mod.rs`
  - Register and re-export the CI handoff service if the dedicated service file is used.
- Modify: `src-tauri/src/services/runner_orchestration_service.rs`
  - Add the smallest possible support needed to let the CI handoff path trigger one suite run and receive stable final run semantics without creating a second execution model.
- Modify: `src-tauri/src/services/artifact_service.rs`
  - Add persistence for the canonical CI handoff JSON artifact and reuse existing redaction-safe serialization helpers and path policy.
- Modify: `src-tauri/src/repositories/runner_repository.rs`
  - Add or extend the minimum read helpers needed to project persisted run/detail/artifact data into the CI handoff JSON schema deterministically.
- Modify: `src-tauri/src/contracts/commands.rs`
  - Add typed Rust request/response contract shapes for the CI handoff invocation path.
- Modify: `src-tauri/src/contracts/dto.rs`
  - Add DTOs for the thin invocation response and the versioned CI handoff JSON projection if the Rust contracts centralize those shapes.
- Modify: `src-tauri/src/lib.rs`
  - Register the new typed command and wire the service through the existing Tauri boundary without leaking raw invoke patterns elsewhere.
- Modify: `src/types/commands.ts`
  - Mirror the CI handoff command and response types on the frontend typed contract map.
- Modify: `src/types/dto.ts`
  - Mirror the thin response DTO and any shared CI handoff DTO types that the typed client or tests need.
- Create: `src/services/ci-client.ts` (if a distinct typed client improves clarity)
  - Add the thinnest typed wrapper around the new IPC command. If it is truly only one method and fits current patterns better, fold it into `src/services/runner-client.ts` instead.
- Modify: `src/services/runner-client.ts` (only if `ci-client.ts` is not created)
  - Expose the CI handoff command through the existing typed client layer without direct `invoke()` leakage.
- Modify: `src/routes/test-runner.tsx` (only if a minimal operational trigger or artifact inspection surface is needed)
  - Keep any UI touch extremely small and consistent with existing runner/reporting patterns.
- Create: `tests/rust/ci_handoff_service_p2.rs`
  - Add backend contract/integration coverage for happy path, failed path, blocked path, deterministic JSON shape, and redaction safety.
- Create: `tests/frontend/ci-handoff-p2.test.ts`
  - Add source-level regression that locks typed commands, typed client boundaries, and contract reuse assumptions.
- Modify: `tests/frontend/contracts.test.ts`
  - Extend typed contract coverage so P2-T8 cannot silently drift from the TS/Rust command boundary.
- Modify: `tests/frontend/test-runner-t16.test.ts`
  - Extend runner route/source regression only if the route gains a minimal CI handoff affordance.
- Modify: `tests/frontend/security-export-redaction-p2.test.ts`
  - Re-run and extend the redaction regression so CI handoff artifacts stay secret-safe.
- Evidence: `.sisyphus/evidence/p2-task-T8-ci-output.txt`
- Evidence: `.sisyphus/evidence/p2-task-T8-ci-redaction.txt`
- Evidence: `.sisyphus/evidence/p2-task-T8-ci-blocked.txt` (recommended for explicit blocked exit semantics)
- Modify: `.sisyphus/plans/testforge-phase-2-implementation.md`
  - Mark P2-T8 progress and acceptance criteria honestly after verification.
- Modify: `.sisyphus/boulder.json`
  - Record P2-T8 session/task mappings after plan execution and review complete.

---

## Chunk 1: Lock the typed contract and red/green the source-level boundaries first

### Task 1: Add failing source-level contract tests for the CI handoff seam

**Files:**
- Create: `tests/frontend/ci-handoff-p2.test.ts`
- Modify: `tests/frontend/contracts.test.ts`
- Modify: `tests/frontend/test-runner-t16.test.ts` (only if the route will expose a minimal trigger/inspection affordance)
- Reference: `src/types/commands.ts`
- Reference: `src/types/dto.ts`
- Reference: `src-tauri/src/contracts/commands.rs`
- Reference: `src-tauri/src/contracts/dto.rs`
- Reference: `src/services/runner-client.ts`

- [ ] **Step 1: Write the failing source-level assertions**

Add assertions that require:
- a typed CI handoff command to exist in the shared TS/Rust contract map,
- a thin typed client seam (`ci-client.ts` or equivalent extension in `runner-client.ts`),
- the request contract to accept exactly one suite and JSON artifact output metadata,
- the thin response DTO to expose only high-level outcome + artifact path fields,
- the P2-T8 seam to reuse typed IPC instead of direct ad-hoc `invoke()` leakage,
- the design to avoid multi-suite batching or a CLI-only path.

- [ ] **Step 2: Run the source-level tests to confirm failure**

Run:
`node --import tsx tests/frontend/contracts.test.ts`

Run:
`node --import tsx tests/frontend/ci-handoff-p2.test.ts`

Run (only if edited):
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Expected:
- at least one assertion fails because the CI handoff typed command/client/DTO seam does not exist yet.

- [ ] **Step 3: Add the minimal shared command and DTO shapes**

Introduce only the smallest typed shapes needed, for example:

```ts
export interface CiHandoffExecuteCommand {
  suiteId: EntityId;
  trigger: {
    source: "ci";
    actor: "pipeline";
    label?: string;
  };
  output: {
    writeJson: true;
    outputDir?: string;
    fileName?: string;
  };
}

export interface CiHandoffResultDto {
  runId: EntityId;
  suiteId: EntityId;
  status: "passed" | "failed" | "blocked";
  exitCode: 0 | 1 | 2;
  artifactPath: string;
}
```

Mirror the chosen shape in Rust contracts and keep any future JSON artifact schema type separate from the thin invocation response DTO.

- [ ] **Step 4: Add the thin typed client seam**

Create the smallest possible typed client method, for example:

```ts
async function executeCiHandoff(input: CiHandoffExecuteCommand): Promise<CiHandoffResultDto>
```

Keep the implementation style consistent with existing typed client helpers.

- [ ] **Step 5: Re-run the source-level tests**

Run:
`node --import tsx tests/frontend/contracts.test.ts`

Run:
`node --import tsx tests/frontend/ci-handoff-p2.test.ts`

Run (only if edited):
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Expected:
- typed contract/client assertions now pass while backend service behavior tests still fail.

---

## Chunk 2: Add backend failing tests for deterministic artifact shape and exit semantics

### Task 2: Write and red/green the Rust CI handoff contract tests

**Files:**
- Create: `tests/rust/ci_handoff_service_p2.rs`
- Reference: `src-tauri/src/services/runner_orchestration_service.rs`
- Reference: `src-tauri/src/services/artifact_service.rs`
- Reference: `src-tauri/src/repositories/runner_repository.rs`
- Reference: `src-tauri/src/contracts/dto.rs`

- [ ] **Step 1: Write the failing Rust coverage first**

Add tests that require:
- one suite execution can be normalized into a versioned JSON artifact,
- the artifact shape is deterministic and includes required fields,
- `passed` maps to exit code `0`,
- `failed` maps to exit code `1`,
- `blocked` maps to exit code `2`,
- blocked/failure diagnostics are machine-readable and sanitized,
- artifact persistence remains filesystem-backed and artifact-consistent.

- [ ] **Step 2: Run the Rust CI handoff tests and confirm failure**

Run:
`cargo test --test ci_handoff_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- FAIL because the CI handoff service/DTO/artifact serialization path does not exist yet.

- [ ] **Step 3: Add stable fixture expectations for the canonical JSON contract**

Lock the intended v1 schema through explicit assertions on:
- `schemaVersion`,
- `contractType`,
- `status`,
- `exitCode`,
- run metadata,
- summary fields,
- failure object shape,
- artifact references,
- redaction metadata.

- [ ] **Step 4: Re-run the failing Rust tests to verify they still fail for the right reason**

Run:
`cargo test --test ci_handoff_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- FAIL due to missing implementation, not because the tests themselves are malformed.

---

## Chunk 3: Implement the backend handoff service and artifact persistence using existing runner seams

### Task 3: Add the CI handoff service, runner integration, and filesystem artifact writer

**Files:**
- Create: `src-tauri/src/services/ci_handoff_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/services/runner_orchestration_service.rs`
- Modify: `src-tauri/src/services/artifact_service.rs`
- Modify: `src-tauri/src/repositories/runner_repository.rs`
- Modify: `src-tauri/src/contracts/dto.rs`
- Modify: `tests/rust/ci_handoff_service_p2.rs`

- [ ] **Step 1: Implement the thinnest possible CI handoff service boundary**

Create a focused service that:
- accepts the typed command input,
- validates one-suite baseline assumptions,
- triggers the existing runner orchestration path,
- obtains the final run/read model,
- maps final state into `passed` / `failed` / `blocked`,
- returns a thin result DTO plus a canonical artifact path.

- [ ] **Step 2: Keep runner execution reuse explicit**

Update orchestration only as much as needed so the CI handoff path:
- uses the same execution pipeline as manual and scheduled runs,
- can attribute the trigger source as `ci` if minimal run-attribution support is required,
- never creates a CI-only run model or persistence path.

- [ ] **Step 3: Add canonical JSON artifact persistence**

In `artifact_service.rs`:
- add the smallest serialization helper needed to write the canonical CI handoff JSON artifact,
- reuse existing filesystem path policy and safe-preview/redaction helpers,
- keep artifact naming deterministic per run,
- ensure path validation rejects traversal or unsafe output roots.

- [ ] **Step 4: Add repository/read-model helpers only where necessary**

If the current runner repository does not already expose enough data to build the CI artifact cleanly, add only the minimum helper/query extensions required to obtain:
- final run summary,
- failure categorization,
- linked artifact metadata,
- secret-safe detail projections.

- [ ] **Step 5: Re-run the Rust CI handoff tests**

Run:
`cargo test --test ci_handoff_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS for deterministic schema shape, exit-code mapping, artifact persistence, and blocked/failure normalization.

---

## Chunk 4: Wire the typed command boundary and optional minimal UI/client surface

### Task 4: Expose the CI handoff path through the existing typed IPC boundary

**Files:**
- Modify: `src-tauri/src/contracts/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types/commands.ts`
- Modify: `src/types/dto.ts`
- Create: `src/services/ci-client.ts` or modify `src/services/runner-client.ts`
- Modify: `tests/frontend/ci-handoff-p2.test.ts`
- Modify: `tests/frontend/contracts.test.ts`
- Modify: `src/routes/test-runner.tsx` (only if needed)
- Modify: `tests/frontend/test-runner-t16.test.ts` (only if route touched)

- [ ] **Step 1: Add the typed command registration on both Rust and TypeScript sides**

Ensure:
- the Rust command payload/response shapes mirror the TS contract,
- `lib.rs` registers the new command cleanly,
- the typed command map remains the only frontend/backend boundary.

- [ ] **Step 2: Run the source-level contract tests again**

Run:
`node --import tsx tests/frontend/contracts.test.ts`

Run:
`node --import tsx tests/frontend/ci-handoff-p2.test.ts`

Run (only if route touched):
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Expected:
- PASS for typed command registration, thin client usage, and no direct raw invoke leakage.

- [ ] **Step 3: Keep any UI touch minimal and operational-only**

If the existing route needs a minimal affordance for inspection/triggering during verification:
- add only a small trigger or artifact inspection action,
- keep it inside the existing runner screen,
- do not build a new CI management surface.

- [ ] **Step 4: Re-run any route regression affected by the UI touch**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Expected:
- PASS with existing runner/reporting/scheduling behaviors preserved.

---

## Chunk 5: Re-run redaction regressions and capture evidence honestly

### Task 5: Verify secret safety, deterministic output, and blocked semantics with evidence

**Files:**
- Modify: `tests/frontend/security-export-redaction-p2.test.ts`
- Reference: `tests/rust/ci_handoff_service_p2.rs`
- Evidence: `.sisyphus/evidence/p2-task-T8-ci-output.txt`
- Evidence: `.sisyphus/evidence/p2-task-T8-ci-redaction.txt`
- Evidence: `.sisyphus/evidence/p2-task-T8-ci-blocked.txt`

- [ ] **Step 1: Extend or reaffirm the redaction regression first**

Add assertions that require:
- CI handoff JSON artifacts to mask secret-backed values,
- no raw secrets in failure diagnostics,
- artifact references to remain consistent with the shared artifact policy.

- [ ] **Step 2: Run the redaction regression and confirm it fails if the artifact is not yet sanitized correctly**

Run:
`node --import tsx tests/frontend/security-export-redaction-p2.test.ts`

Expected:
- FAIL if redaction-safe CI output has not been wired correctly yet.

- [ ] **Step 3: Fix any remaining serialization/redaction issues minimally**

Keep fixes focused on:
- serialization helpers,
- failure diagnostic normalization,
- artifact metadata projection,
- path safety or field omission rules.

- [ ] **Step 4: Run the final targeted verification set**

Run:
`node --import tsx tests/frontend/contracts.test.ts`

Run:
`node --import tsx tests/frontend/ci-handoff-p2.test.ts`

Run:
`node --import tsx tests/frontend/security-export-redaction-p2.test.ts`

Run (only if route touched):
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`cargo test --test ci_handoff_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS across contract, backend, and redaction checks.

- [ ] **Step 5: Capture evidence with exact outputs and artifact paths**

Record:
- the command/test runs used to produce the canonical JSON artifact,
- the observed `passed` / `failed` / `blocked` semantics,
- the artifact path and a short description of the schema fields checked,
- the redaction checks performed.

Write evidence to:
- `.sisyphus/evidence/p2-task-T8-ci-output.txt`
- `.sisyphus/evidence/p2-task-T8-ci-redaction.txt`
- `.sisyphus/evidence/p2-task-T8-ci-blocked.txt`

If a live runtime/harness limitation prevents one scenario from being fully proven, mark it honestly as `BLOCKED` with the exact reason instead of claiming success.

---

## Chunk 6: Final diagnostics, build verification, and plan/state updates

### Task 6: Verify completion and update plan/state honestly

**Files:**
- Modify: `.sisyphus/plans/testforge-phase-2-implementation.md`
- Modify: `.sisyphus/boulder.json`
- Reference: all changed TS/Rust files

- [ ] **Step 1: Run diagnostics on the changed files**

Run `lsp_diagnostics` on all changed TypeScript and Rust files and fix only issues introduced by P2-T8.

- [ ] **Step 2: Run the required build**

Run:
`npm run build`

Expected:
- PASS with no new build failures.

- [ ] **Step 3: Reconfirm the final targeted test set after the build**

Run:
`node --import tsx tests/frontend/contracts.test.ts`

Run:
`node --import tsx tests/frontend/ci-handoff-p2.test.ts`

Run:
`node --import tsx tests/frontend/security-export-redaction-p2.test.ts`

Run (only if route touched):
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`cargo test --test ci_handoff_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS remains stable after the build.

- [ ] **Step 4: Update the active phase plan and boulder state**

In `.sisyphus/plans/testforge-phase-2-implementation.md`:
- mark P2-T8 complete only if all acceptance criteria are verified,
- check off the P2-T8 acceptance criteria honestly.

In `.sisyphus/boulder.json`:
- append P2-T8 session IDs,
- record task session mappings for planning, implementation, and review work.

- [ ] **Step 5: Create the feature commit after verification succeeds**

Commit:
`feat(ci): add pipeline handoff contract and outputs`

Do not commit if verification is incomplete or dishonest.

---

## Notes for Execution

- Keep implementation backend-led. Frontend should stay thin and typed.
- Favor a dedicated `ci_handoff_service.rs` only if it meaningfully improves boundary clarity; otherwise do not over-split.
- Do not store the full JSON execution payload in SQLite.
- Do not invent new status taxonomies beyond the approved `passed` / `failed` / `blocked` mapping.
- Treat the JSON artifact on disk as the canonical contract; the invocation response is only a pointer plus summary.
- Re-run the secret-redaction regression before calling the task complete.
