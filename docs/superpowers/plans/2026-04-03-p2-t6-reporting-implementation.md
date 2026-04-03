# P2-T6 Reporting Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand TestForge's existing `test-runner` route into a practical reporting surface with filterable run history, grouped summaries, failure drilldown, and trend-ready aggregates without introducing a separate analytics platform.

**Architecture:** Keep `src/routes/test-runner.tsx` as the single reporting UI surface and extend the existing typed runner read model instead of creating a parallel reporting backend. Backend changes stay concentrated in `runner_repository.rs`, shared contracts, and Tauri handlers only if current history/detail commands cannot support the required filters and grouped summaries. Redaction/export guardrails remain unchanged and are verified through existing artifact/report regression seams.

**Tech Stack:** React 18, TypeScript, Tauri v2, Rust, rusqlite, existing typed IPC contracts, filesystem-backed artifact manifests.

---

## File Structure Map

- Modify: `src/routes/test-runner.tsx`
  - Add reporting filter controls, grouped summaries, grouped failed-case drilldown, and trend-ready summary sections.
- Modify: `src/services/runner-client.ts`
  - Extend typed client support for reporting filters if command/query payloads change.
- Modify: `src/types/commands.ts`
  - Mirror any new read-side payload shape for reporting filters.
- Modify: `src/types/dto.ts`
  - Add minimal reporting aggregate/filter DTOs only if current history/detail structures are insufficient.
- Modify: `src-tauri/src/contracts/commands.rs`
  - Keep Rust command contracts aligned with frontend typed payloads.
- Modify: `src-tauri/src/contracts/dto.rs`
  - Keep Rust DTO contracts aligned with frontend DTOs.
- Modify: `src-tauri/src/repositories/runner_repository.rs`
  - Add minimal read/query/filter/grouping support for reporting needs; no new persistence model.
- Modify: `src-tauri/src/lib.rs`
  - Update Tauri handler input/output wiring if command payloads change.
- Test: `tests/frontend/test-runner-t16.test.ts`
  - Extend source-level regression for reporting additions inside the existing runner route.
- Create: `tests/frontend/reporting-route-p2.test.ts`
  - Add source-level regression focused on filters, grouped summaries, trend-ready aggregates, and failure drilldown states.
- Evidence: `.sisyphus/evidence/p2-task-T6-reporting-filters.txt`
- Evidence: `.sisyphus/evidence/p2-task-T6-reporting-redaction.txt`

---

## Chunk 1: Extend typed reporting read model

### Task 1: Lock the intended command + DTO shape in tests first

**Files:**
- Modify: `tests/frontend/test-runner-t16.test.ts`
- Create: `tests/frontend/reporting-route-p2.test.ts`
- Reference: `src/types/commands.ts`
- Reference: `src/types/dto.ts`
- Reference: `src-tauri/src/contracts/commands.rs`
- Reference: `src-tauri/src/contracts/dto.rs`

- [ ] **Step 1: Write the failing reporting regression assertions**

Add assertions that require:
- `test-runner.tsx` to include reporting copy such as filter/date/status/grouped summary/failure grouping/trend-ready wording.
- `runner-client.ts` to accept reporting filter payload shape if command input changes.
- typed contracts to expose only the minimal reporting read model additions.
- no new plaintext/ciphertext/masked secret DTO leakage.

- [ ] **Step 2: Run the new and updated source-level tests to confirm they fail**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/reporting-route-p2.test.ts`

Expected:
- at least one assertion fails because reporting filter/grouping support has not been implemented yet.

- [ ] **Step 3: Add minimal DTO and command contract changes**

If current payloads are insufficient, add only the smallest required shapes, for example:

```ts
export interface RunHistoryFilterDto {
  suiteId?: EntityId;
  status?: RunStatus;
  startedAfter?: IsoDateTime;
  startedBefore?: IsoDateTime;
}

export interface RunHistoryGroupSummaryDto {
  totalRuns: number;
  passedRuns: number;
  failedRuns: number;
  cancelledRuns: number;
  failureCategoryCounts: Array<{ category: string; count: number }>;
}
```

And mirror them in Rust contracts.

- [ ] **Step 4: Update command payload maps only if needed**

Keep the reporting read path under the existing runner history/detail boundary where possible. If filters can be passed on `runner.run.history`, prefer that over inventing a new command.

- [ ] **Step 5: Run the source-level tests again**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/reporting-route-p2.test.ts`

Expected:
- DTO/command-shape assertions now pass, while route/repository assertions may still fail.

---

## Chunk 2: Add repository-side reporting filter and grouping support

### Task 2: Implement backend read-side reporting support with validation

**Files:**
- Modify: `src-tauri/src/repositories/runner_repository.rs`
- Modify: `src-tauri/src/contracts/commands.rs`
- Modify: `src-tauri/src/contracts/dto.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `tests/frontend/reporting-route-p2.test.ts`

- [ ] **Step 1: Add failing assertions for backend/reporting seams**

Extend `tests/frontend/reporting-route-p2.test.ts` to require:
- repository support for status/date-range filtering,
- grouped summary derivation,
- failure-category grouping,
- invalid filter validation/error handling,
- continued redaction-safe preview usage.

- [ ] **Step 2: Run the reporting regression to verify failure**

Run:
`node --import tsx tests/frontend/reporting-route-p2.test.ts`

Expected:
- FAIL because repository/query/handler support is incomplete.

- [ ] **Step 3: Implement minimal repository query extension**

Inside `runner_repository.rs`:
- extend `list_run_history(...)` or add a closely related helper to accept suite/status/date filters,
- validate invalid date/status combinations clearly,
- keep ordering by the existing persisted time semantics,
- derive grouped run counts and failure-category counts from persisted data,
- do not create new tables or persistence paths,
- continue using sanitized previews for any detail-derived reporting data.

- [ ] **Step 4: Wire typed handler updates in `lib.rs`**

Update the relevant Tauri command handler to pass filter input to the repository and return the updated typed response shape. Do not introduce raw `invoke()` leakage or untyped JSON escape hatches.

- [ ] **Step 5: Re-run reporting contract tests**

Run:
`node --import tsx tests/frontend/reporting-route-p2.test.ts`

Expected:
- backend/source seams now satisfy the reporting regression assertions.

---

## Chunk 3: Expand `test-runner` into the reporting surface

### Task 3: Add filters, grouped summaries, and failure drilldown UI

**Files:**
- Modify: `src/routes/test-runner.tsx`
- Modify: `src/services/runner-client.ts`
- Modify: `src/types/dto.ts`
- Test: `tests/frontend/test-runner-t16.test.ts`
- Test: `tests/frontend/reporting-route-p2.test.ts`

- [ ] **Step 1: Add failing route assertions for the new reporting UI**

Require the route source to include:
- suite/status/date filters,
- reset filters action,
- grouped summary labels,
- grouped failed-result section,
- trend-ready aggregate copy,
- clear empty-state/missing-artifact states.

- [ ] **Step 2: Run the route regressions to verify failure**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/reporting-route-p2.test.ts`

Expected:
- FAIL due to missing UI/reporting elements.

- [ ] **Step 3: Implement the minimal UI changes in `test-runner.tsx`**

Add:
- local filter state for suite/status/date range,
- reporting summary cards for filtered history,
- grouped failed-case rendering by `failureCategory`,
- trend-ready aggregate section using lightweight cards/list output,
- explicit empty state when filters match no runs,
- explicit missing-artifact state in drilldown.

Keep:
- existing runner control actions,
- existing detail hydration flow,
- existing artifact links,
- existing sanitized preview rendering.

- [ ] **Step 4: Extend `runner-client.ts` only as much as needed**

If handler payloads changed, add the thinnest possible client shape, for example:

```ts
async listRunHistory(input: {
  suiteId?: string;
  status?: string;
  startedAfter?: string;
  startedBefore?: string;
} = {})
```

Do not add extra client layers.

- [ ] **Step 5: Run the route regressions again**

Run:
`node --import tsx tests/frontend/test-runner-t16.test.ts`

Run:
`node --import tsx tests/frontend/reporting-route-p2.test.ts`

Expected:
- PASS for the new reporting surface assertions.

---

## Chunk 4: Verify redaction, failure paths, and build health

### Task 4: Prove reporting remains secret-safe and operationally honest

**Files:**
- Reference: `src-tauri/src/services/artifact_service.rs`
- Test: `tests/frontend/export-artifact-t10.test.ts`
- Test: `tests/frontend/security-export-redaction-p2.test.ts`
- Evidence: `.sisyphus/evidence/p2-task-T6-reporting-filters.txt`
- Evidence: `.sisyphus/evidence/p2-task-T6-reporting-redaction.txt`

- [ ] **Step 1: Run redaction regressions after the implementation**

Run:
`node --import tsx tests/frontend/export-artifact-t10.test.ts`

Run:
`node --import tsx tests/frontend/security-export-redaction-p2.test.ts`

Expected:
- PASS, confirming no regression in artifact/report secret handling.

- [ ] **Step 2: Run diagnostics on changed files**

Run language-server diagnostics for all touched TS/Rust files and resolve any introduced errors.

- [ ] **Step 3: Run the required build**

Run:
`$env:NODE_OPTIONS='--max-old-space-size=4096'; npm run build`

Expected:
- build exits successfully.

- [ ] **Step 4: Write evidence for happy path + failure path**

Document:
- filter/grouping behavior on historical runs,
- empty-state or invalid-filter behavior,
- confirmation that reporting previews remain secret-safe,
- commands run and observed results.

- [ ] **Step 5: Update plan/state and commit**

Update:
- `.sisyphus/plans/testforge-phase-2-implementation.md`
- `.sisyphus/boulder.json`

Commit message:
`feat(reporting): add advanced run summaries and drilldown`

---

## Definition-of-Done Mapping

- Filter by suite/status/date range → covered by Chunk 2 + Chunk 3
- Grouped summaries + artifact-backed failure drilldown → covered by Chunk 2 + Chunk 3
- Trend-ready aggregates without secret leakage → covered by Chunk 3 + Chunk 4

---

## Execution Notes

- Prefer extending `runner.run.history` over adding a brand-new reporting command.
- Do not add a separate `reporting.tsx` route in P2-T6.
- Do not add new export formats or export flows in P2-T6.
- Keep changes small and reviewable; this is a reporting baseline, not a dashboard rewrite.
- Reuse existing status/failure/artifact/sanitized-preview semantics everywhere possible.
