# P2-T7 Scheduling Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a local-only scheduling baseline inside TestForge's existing `test-runner` surface so enabled suite schedules can trigger unattended runs while the desktop app is open, using the normal runner/history/artifact pipeline.

**Architecture:** Keep scheduling as a thin trigger-and-state layer. Persist schedule definitions in SQLite, bootstrap a backend scheduler loop from the existing Tauri app lifecycle, and always execute through `RunnerOrchestrationService` rather than inventing a scheduler-only execution model. Embed schedule management UI into `src/routes/test-runner.tsx` and expose it through typed IPC/client seams only.

**Tech Stack:** React 18, TypeScript, Tauri v2, Rust, tokio, rusqlite, existing typed IPC contracts, existing runner/history/artifact services.

---

## File Structure Map

- Create: `src-tauri/migrations/004_add_suite_schedules.sql`
  - Add the persisted schedule definition table and minimal indexes needed for enabled/due schedule lookup.
- Create: `src-tauri/src/services/scheduler_service.rs`
  - Hold schedule CRUD helpers, due-schedule evaluation, diagnostics updates, and the app-local scheduler loop that triggers runner execution.
- Modify: `src-tauri/src/services/mod.rs`
  - Register and re-export the new scheduler service.
- Modify: `src-tauri/src/services/runner_orchestration_service.rs`
  - Add the smallest possible execution attribution support so scheduled runs can be identified without creating a parallel run model.
- Modify: `src-tauri/src/state.rs`
  - Add explicit scheduler runtime state if needed for loop lifecycle/idempotent startup, keeping the same state-machine style as existing run/record/replay state.
- Modify: `src-tauri/src/contracts/dto.rs`
  - Add schedule DTOs and any minimal run-attribution DTO extensions required by the UI/history surface.
- Modify: `src-tauri/src/contracts/commands.rs`
  - Add typed command payloads/responses for schedule list/upsert/toggle/delete or equivalent minimal CRUD/read operations.
- Modify: `src-tauri/src/lib.rs`
  - Wire migration-backed scheduler bootstrapping, register schedule commands, and attach the scheduler loop to existing app startup safely.
- Modify: `src/types/dto.ts`
  - Mirror the new schedule DTOs and any minimal run-attribution fields from Rust.
- Modify: `src/types/commands.ts`
  - Mirror schedule command payload/response maps; keep typed IPC as the only frontend/backend contract.
- Create: `src/services/scheduler-client.ts`
  - Add the thinnest typed client wrapper for schedule CRUD/read operations.
- Modify: `src/routes/test-runner.tsx`
  - Add embedded scheduling UI, schedule status cards, diagnostics, and refresh logic while preserving runner/reporting behavior.
- Modify: `src/App.tsx` (only if needed)
  - Touch only if schedule status must be surfaced at shell level; otherwise leave unchanged.
- Create: `tests/frontend/scheduler-route-p2.test.ts`
  - Source-level regression for scheduling UI placement, typed client/contracts, diagnostics copy, and runner reuse guarantees.
- Modify: `tests/frontend/test-runner-t16.test.ts`
  - Extend runner route regression only where scheduling is intentionally embedded into the existing screen.
- Modify: `tests/frontend/suite-runner-t15.test.ts`
  - Lock that scheduled runs still reuse the same runner/orchestration/persistence path instead of a new execution model.
- Modify: `tests/frontend/reliability-hardening-t18.test.ts`
  - Add scheduler-related idempotency/duplicate-run/blocked-state guard assertions if scheduler lifecycle touches those seams.
- Test: `tests/frontend/security-export-redaction-p2.test.ts`
  - Re-run the existing redaction regression because scheduler diagnostics and failure evidence touch secret-sensitive output surfaces.
- Create: `tests/rust/scheduler_service_p2.rs`
  - Persistence + due-selection + diagnostics + active-run-guard coverage for the backend scheduler service.
- Evidence: `.sisyphus/evidence/p2-task-T7-scheduled-run.txt`
- Evidence: `.sisyphus/evidence/p2-task-T7-schedule-disable.txt`
- Evidence: `.sisyphus/evidence/p2-task-T7-schedule-failure.txt`
- Modify: `.sisyphus/plans/testforge-phase-2-implementation.md`
  - Mark P2-T7 progress/acceptance items honestly after verification.
- Modify: `.sisyphus/boulder.json`
  - Record P2-T7 session/task mappings after the implementation/review flow is complete.

---

## Chunk 1: Lock the scheduling contract and migration shape first

### Task 1: Add failing source-level tests for schedule contracts and UI placement

**Files:**
- Create: `tests/frontend/scheduler-route-p2.test.ts`
- Modify: `tests/frontend/test-runner-t16.test.ts`
- Modify: `tests/frontend/suite-runner-t15.test.ts`
- Reference: `src/types/commands.ts`
- Reference: `src/types/dto.ts`
- Reference: `src-tauri/src/contracts/commands.rs`
- Reference: `src-tauri/src/contracts/dto.rs`
- Reference: `src/routes/test-runner.tsx`

- [ ] **Step 1: Write the failing scheduling regression assertions**

Add assertions that require:
- scheduling UI to live in `test-runner.tsx`, not a separate route,
- a dedicated typed client seam (`scheduler-client.ts`) or equivalent thin typed scheduler surface,
- typed commands/DTOs for schedule CRUD/read operations,
- schedule state copy for enabled/disabled, last run, next run, and diagnostics,
- explicit evidence that scheduled execution reuses runner/orchestration/history seams rather than inventing a new pipeline.

- [ ] **Step 2: Run the updated source-level tests to confirm failure**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/suite-runner-t15.test.ts`

Run:
`node --import tsx tests/frontend/scheduler-route-p2.test.ts`

Expected:
- at least one assertion fails because scheduling contracts and embedded UI are not implemented yet.

- [ ] **Step 3: Add the minimal schedule DTO and command shapes**

Introduce only the smallest required typed shapes, for example:

```ts
export interface SuiteScheduleDto {
  id: EntityId;
  suiteId: EntityId;
  environmentId: EntityId;
  enabled: boolean;
  cadenceMinutes: number;
  lastRunAt?: IsoDateTime;
  nextRunAt?: IsoDateTime;
  lastRunStatus?: Exclude<RunStatus, "idle">;
  lastError?: string;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}
```

Mirror the chosen shape in Rust contracts and add command payloads/responses for list/upsert/toggle/delete or an equivalent minimal CRUD set.

- [ ] **Step 4: Add the migration stub and lock its intended responsibility in tests**

Create the migration file path and ensure the tests/documentation expect schedule persistence to live in SQLite, not in frontend local state or ad-hoc JSON files.

- [ ] **Step 5: Re-run the source-level tests**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/suite-runner-t15.test.ts`

Run:
`node --import tsx tests/frontend/scheduler-route-p2.test.ts`

Expected:
- contract-shape assertions now pass while backend/service behavior assertions may still fail.

---

## Chunk 2: Implement persisted schedules and backend scheduler service

### Task 2: Add SQLite-backed schedule persistence and due-schedule evaluation

**Files:**
- Create: `src-tauri/migrations/004_add_suite_schedules.sql`
- Create: `src-tauri/src/services/scheduler_service.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/contracts/dto.rs`
- Modify: `src-tauri/src/contracts/commands.rs`
- Create: `tests/rust/scheduler_service_p2.rs`
- Reference: `src-tauri/tests/migrations.rs`

- [ ] **Step 1: Write the failing Rust coverage first**

Add Rust tests that require:
- the new schedule table to exist after migration,
- migration rerun to remain idempotent,
- schedules to persist and reload correctly,
- enabled schedules to be selected as due while disabled schedules are ignored,
- invalid schedule definitions or broken references to update diagnostics clearly.

- [ ] **Step 2: Run the Rust scheduler tests and confirm failure**

Run:
`cargo test --test scheduler_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- FAIL because the migration/service does not exist yet.

- [ ] **Step 3: Implement the migration and minimal scheduler persistence/service logic**

In the migration and `scheduler_service.rs`:
- create a schedule table with suite/environment foreign-reference semantics consistent with existing IDs,
- persist enabled state, cadence/time definition, next/last run timestamps, last status, and last error,
- implement typed CRUD/read operations,
- add due-schedule lookup and next-run calculation helpers,
- keep the service focused on scheduling state, not full execution orchestration.

- [ ] **Step 4: Re-run the Rust scheduler tests**

Run:
`cargo test --test scheduler_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS for persistence and due-selection semantics.

---

## Chunk 3: Attach scheduling to app lifecycle and reuse runner orchestration

### Task 3: Bootstrap the local scheduler loop and trigger normal suite execution

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/services/scheduler_service.rs`
- Modify: `src-tauri/src/services/runner_orchestration_service.rs`
- Modify: `tests/frontend/suite-runner-t15.test.ts`
- Modify: `tests/frontend/reliability-hardening-t18.test.ts`
- Create: `tests/rust/scheduler_service_p2.rs`

- [ ] **Step 1: Extend tests to demand lifecycle-safe scheduler triggering**

Add assertions/tests that require:
- scheduler bootstrap from the existing Tauri setup path,
- no duplicate scheduler loop startup,
- scheduled execution to call the same runner/orchestration path as manual runs,
- active-run guard to prevent overlapping suite runs,
- blocked/failed trigger paths to store actionable diagnostics without pretending success.

- [ ] **Step 2: Run the targeted tests to confirm failure**

Run:
`node --import tsx tests/frontend/suite-runner-t15.test.ts`

Run:
`node --import tsx tests/frontend/reliability-hardening-t18.test.ts`

Run:
`cargo test --test scheduler_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- FAIL because lifecycle wiring and trigger semantics are incomplete.

- [ ] **Step 3: Implement the bootstrap + trigger glue with minimal surface area**

Update backend wiring to:
- start exactly one app-local scheduler loop from existing Tauri bootstrap,
- periodically inspect enabled schedules that are due,
- trigger `RunnerOrchestrationService` instead of a scheduler-only executor,
- update schedule diagnostics/next-run state honestly on success, blocked, invalid-config, or failure paths,
- keep all scheduler state transitions idempotent and safe on restart.

- [ ] **Step 4: Add the smallest run-attribution extension if needed**

If the UI/history needs to distinguish scheduled runs, add only a minimal source/trigger field to existing run summary/detail DTOs and persistence, without creating a second run model.

- [ ] **Step 5: Re-run the lifecycle and orchestration tests**

Run:
`node --import tsx tests/frontend/suite-runner-t15.test.ts`

Run:
`node --import tsx tests/frontend/reliability-hardening-t18.test.ts`

Run:
`cargo test --test scheduler_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS for scheduler bootstrap, runner reuse, and guard/diagnostic semantics.

---

## Chunk 4: Add the typed frontend client and embed scheduling into `test-runner`

### Task 4: Implement the scheduling UI inside the existing runner screen

**Files:**
- Create: `src/services/scheduler-client.ts`
- Modify: `src/types/commands.ts`
- Modify: `src/types/dto.ts`
- Modify: `src/routes/test-runner.tsx`
- Modify: `tests/frontend/scheduler-route-p2.test.ts`
- Modify: `tests/frontend/test-runner-t16.test.ts`

- [ ] **Step 1: Extend the route tests to require the embedded scheduling UI details**

Require the route source to include:
- scheduling section headings/copy,
- suite/environment/cadence controls,
- enable/disable action,
- last run / next run / last status / diagnostics surface,
- refresh/load logic through the typed scheduler client,
- continued coexistence with manual run controls, history, detail, and reporting.

- [ ] **Step 2: Run the route regressions to verify failure**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/scheduler-route-p2.test.ts`

Expected:
- FAIL due to missing scheduling UI and client wiring.

- [ ] **Step 3: Implement the thin typed scheduler client**

Add only the minimum client functions needed, for example:

```ts
async listSchedules(): Promise<SuiteScheduleDto[]>
async upsertSchedule(input: UpsertSuiteScheduleCommand): Promise<SuiteScheduleDto>
async setScheduleEnabled(input: { scheduleId: string; enabled: boolean }): Promise<SuiteScheduleDto>
async deleteSchedule(input: { scheduleId: string }): Promise<{ deleted: true }>
```

Keep it consistent with the existing `runner-client.ts` style.

- [ ] **Step 4: Implement the minimal scheduling UI in `test-runner.tsx`**

Add:
- local form state for editing/saving one schedule at a time,
- schedule list/status card(s) embedded into the runner screen,
- enabled/disabled actions,
- refresh after save/toggle/delete,
- diagnostics rendering that remains concise and secret-safe,
- optional badges/labels for scheduled runs if the backend attribution field exists.

Keep:
- existing runner execution actions,
- existing history/detail hydration,
- existing reporting filters and summaries,
- existing artifact links and sanitized previews.

- [ ] **Step 5: Re-run the route regressions**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/scheduler-route-p2.test.ts`

Expected:
- PASS for scheduling placement and embedded UI assertions.

---

## Chunk 5: Verify honest unattended behavior, diagnostics, and build health

### Task 5: Prove scheduled execution is operationally honest and non-regressive

**Files:**
- Test: `tests/frontend/suite-runner-t15.test.ts`
- Test: `tests/frontend/test-runner-t16.test.ts`
- Test: `tests/frontend/reliability-hardening-t18.test.ts`
- Test: `tests/frontend/security-export-redaction-p2.test.ts`
- Test: `tests/frontend/scheduler-route-p2.test.ts`
- Test: `tests/rust/scheduler_service_p2.rs`
- Evidence: `.sisyphus/evidence/p2-task-T7-scheduled-run.txt`
- Evidence: `.sisyphus/evidence/p2-task-T7-schedule-disable.txt`
- Evidence: `.sisyphus/evidence/p2-task-T7-schedule-failure.txt`

- [ ] **Step 1: Run the full targeted regression set**

Run:
`node --import tsx tests/frontend/suite-runner-t15.test.ts`

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/reliability-hardening-t18.test.ts`

Run:
`node --import tsx tests/frontend/security-export-redaction-p2.test.ts`

Run:
`node --import tsx tests/frontend/scheduler-route-p2.test.ts`

Run:
`cargo test --test scheduler_service_p2 --manifest-path src-tauri/Cargo.toml`

Expected:
- PASS for the scheduling baseline regressions.

- [ ] **Step 2: Capture the success-path evidence**

Run the desktop runtime verification flow using `interactive_bash`/runtime harness and record the exact commands used in the evidence file.

Minimum execution shape:
- start the desktop runtime with scheduler active,
- configure one enabled schedule for a runnable suite through the app/backend seam,
- wait through the trigger window,
- inspect the resulting run history/detail/artifact outputs through existing typed/history seams.

Produce `.sisyphus/evidence/p2-task-T7-scheduled-run.txt` with:
- the exact commands or harness steps used,
- how the schedule was configured,
- how the app runtime remained open,
- what run/history/artifact result was observed,
- any runtime limitations honestly noted.

- [ ] **Step 3: Capture the disabled-schedule evidence**

Run the same desktop/runtime verification flow with the schedule disabled, again recording the exact commands or harness steps used.

Produce `.sisyphus/evidence/p2-task-T7-schedule-disable.txt` with:
- the exact commands or harness steps used,
- the disabled schedule state,
- the waited trigger window,
- proof that no new run was created.

- [ ] **Step 4: Capture the failure/blocked evidence**

Run a failure/blocked desktop/runtime verification flow with intentionally invalid or blocked prerequisites, recording the exact commands or harness steps used.

Produce `.sisyphus/evidence/p2-task-T7-schedule-failure.txt` with:
- the exact commands or harness steps used,
- the intentional blocked/invalid/failing setup,
- the observed diagnostic output,
- proof that no raw secrets leaked into diagnostics.

- [ ] **Step 5: Run diagnostics and build verification**

Run `lsp_diagnostics` on every changed TS and Rust file.

Run:
`npm run build`

Expected:
- diagnostics clean on changed files,
- build succeeds with no new errors.

- [ ] **Step 6: Update plan/state tracking files explicitly**

Update:
- `.sisyphus/plans/testforge-phase-2-implementation.md`
  - mark P2-T7 complete only after all acceptance criteria and evidence are truly satisfied.
- `.sisyphus/boulder.json`
  - add P2-T7 review/implementation session mappings needed for continuity.

---

## Completion Checklist

- [ ] Schedule definitions persist through SQLite migration/bootstrap.
- [ ] Scheduler loop starts from app lifecycle without duplicate startup.
- [ ] Scheduled runs reuse normal runner/history/artifact seams.
- [ ] Disabled schedules do not execute.
- [ ] Failed/blocked scheduled triggers expose actionable, secret-safe diagnostics.
- [ ] Embedded scheduling UI coexists cleanly with existing `test-runner` behavior.
- [ ] Targeted tests pass.
- [ ] Evidence files are written.
- [ ] Redaction regression remains green.
- [ ] `npm run build` passes.
- [ ] `.sisyphus/plans/testforge-phase-2-implementation.md` and `.sisyphus/boulder.json` are updated.

---

Plan complete and saved to `docs/superpowers/plans/2026-04-03-p2-t7-scheduling-implementation.md`. Ready to execute?
