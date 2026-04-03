# TestForge Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stabilize the Phase 1 runtime seams that remain only partially verified, then expand TestForge with advanced reporting, scheduling, CI/CD handoff, collaboration-ready asset flows, and stronger browser/runtime hardening.

**Architecture:** Phase 2 must preserve the same core boundaries established in Phase 1: typed IPC on the frontend, Rust service/repository orchestration on the backend, browser runtime isolation behind `BrowserAutomationService`, SQLite for metadata, and filesystem-backed artifacts. The first half of Phase 2 focuses on turning Phase 1's blocked or unproven runtime seams into repeatable desktop-runtime evidence; the second half adds higher-level product capabilities only after those seams are proven stable.

**Tech Stack:** Tauri v2, React 18, TypeScript, Vite, TailwindCSS, shadcn/ui, Zustand, TanStack Query, react-hook-form, zod, Rust, tokio, reqwest + rustls, rusqlite, SQLite, existing Chromium-first browser runtime adapter behind `BrowserAutomationService`.

---

## TL;DR

> **Quick Summary**: Deliver TestForge Phase 2 in two controlled tracks. First, close the reality gap left after Phase 1 by proving browser replay, mixed-suite execution, and Windows packaging in real runtime conditions. Then add the explicit Phase 2 roadmap items: advanced reporting, scheduling, CI/CD handoff, collaboration-ready asset workflows, and richer browser/runtime stabilization.
>
> **Deliverables**:
> - Real desktop-runtime proof for replay, suite execution, and packaged first-run behavior
> - Hardened browser runtime provisioning and security follow-ups
> - Advanced reporting surfaces and richer run analytics
> - Local scheduling and unattended suite execution baseline
> - CI/CD-ready export/execution handoff contract
> - Collaboration-ready asset import/export and review-safe sharing flow
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES — 3 major implementation waves + final verification wave
> **Critical Path**: P2-T1 → P2-T2 → P2-T3 → P2-T6 → P2-T7 → P2-F1-F4

---

## Context

### Original Request
The user asked to read and analyze the existing planning artifacts, then write a full Phase 2 implementation plan in the repository's existing planning style.

### Phase 1 Reality Check That Drives Phase 2
Phase 1 is substantially implemented, but the final evidence is not strong enough to honestly treat all MVP runtime seams as fully verified. The biggest carried-forward gaps are:
- UI replay runtime remained `BLOCKED` in final QA when Chromium runtime was unavailable.
- Mixed suite execution remained `BLOCKED` in the browser-only preview path because the runner IPC-backed desktop surface was unavailable.
- Windows packaging / first-run behavior was only partially evidenced at the source/build seam and not fully proven with packaged-app runtime evidence.

### Locked Phase 2 Product Direction
Phase 2 roadmap items already called out in the original product plan:
- advanced reporting
- scheduling
- CI/CD integration
- collaboration
- richer browser support stabilization

### Planning Interpretation for Phase 2
Phase 2 must be split into two internal tracks:
1. **Phase 2A — Closure & Stabilization**: resolve blocked/unproven runtime seams inherited from Phase 1.
2. **Phase 2B — Product Expansion**: add the explicit roadmap features only after the runtime foundation is honest and repeatable.

### References
- `.sisyphus/plans/automation-testing-tool.md`
- `.sisyphus/plans/testforge-phase-1-implementation.md`
- `.sisyphus/evidence/final-qa/final-qa-report-2026-04-01.txt`
- `.sisyphus/evidence/task-T14-browser-replay-2026-04-01.txt`
- `.sisyphus/evidence/task-T17-packaging-bootstrap-2026-04-01.txt`
- `.sisyphus/notepads/testforge-phase-1-implementation/decisions.md`
- `.sisyphus/notepads/testforge-phase-1-implementation/problems.md`

### Guardrails Incorporated
- Do not treat preview-only UI evidence as equivalent to real desktop-runtime proof.
- Do not add new product layers on top of unproven replay/suite/package seams.
- Keep browser internals isolated behind `BrowserAutomationService`.
- Preserve the typed frontend client layer; no direct frontend `invoke()` leakage.
- Preserve filesystem artifact storage; do not move screenshots or large exports into SQLite.
- Keep Phase 2 collaboration intentionally lightweight; do not turn this into a multi-user server platform.

---

## Work Objectives

### Core Objective
Deliver a truly stable internal Phase 2 of TestForge that can reliably replay UI flows, run mixed suites in the real desktop runtime, package cleanly for Windows-first distribution, and then add higher-value operational features for reporting, scheduling, CI/CD handoff, and collaboration-safe sharing.

### Concrete Deliverables
- Browser runtime provisioning, replay stabilization, and desktop-runtime proof path
- Mixed-suite execution proof and runner hardening in real Tauri runtime
- Windows packaging and first-run validation evidence
- Advanced reporting baseline with filters, grouped summaries, and trend-ready data views
- Local scheduler and unattended suite execution baseline
- CI/CD-friendly export / execution handoff contract
- Collaboration-safe asset import/export and sharing baseline
- Secret/key lifecycle hardening follow-up

### Definition of Done
- [ ] QA can replay a simple saved UI flow in the target desktop runtime with runtime evidence, not just source or smoke-blocked seams
- [ ] QA can run a mixed API + UI suite in the real desktop runtime and inspect accurate progress/history/detail results
- [ ] Packaged Windows app bootstraps cleanly on first run and reports missing runtime/browser prerequisites clearly
- [ ] Reporting UI can summarize runs by suite/status/date and surface artifact-backed failure detail without leaking secrets
- [ ] Scheduled local suite run executes unattended and writes normal run history/artifacts
- [ ] CI/CD handoff path can produce deterministic machine-readable results and clear exit semantics
- [ ] Collaboration-safe asset import/export flow works without introducing server-backed multi-user scope
- [ ] Secret rotation / hardening follow-up is implemented without regressing existing redaction rules

### Must Have
- Stable typed IPC command/event boundary remains the only frontend/backend contract
- `BrowserAutomationService` remains the only browser runtime abstraction boundary
- Real runtime verification exists for replay, suite execution, and packaged bootstrap flows
- Reporting/export paths continue to redact secrets and sensitive previews by default
- Scheduler and CI handoff reuse existing runner/artifact domain models instead of inventing parallel execution models
- Collaboration flows are file/package based unless the user explicitly approves server-backed expansion later

### Must NOT Have (Guardrails)
- No direct leakage of browser runtime internals into frontend or unrelated backend modules
- No embedded browser panel rewrite unless separately approved
- No full multi-browser parity promise in this phase
- No real-time multi-user collaboration backend
- No overbuilt analytics platform, flaky-healing engine, or AI-generated remediation workflows
- No secret values in logs, exports, scheduler diagnostics, CI outputs, or collaboration packages
- No breaking change to existing filesystem artifact policy

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — All verification must be executable by the implementing agent using commands, browser automation, packaged-app interaction, or terminal interaction.

### Test Decision
- **Infrastructure exists**: YES — Phase 1 established frontend regression tests, smoke harnesses, evidence conventions, and runtime seams.
- **Automated tests**: YES (tests-after with stronger runtime verification than Phase 1 where applicable)
- **Frameworks**:
  - Rust: `cargo test`
  - Frontend: `vitest` + `@testing-library/react`
  - Runtime smoke: targeted desktop/browser harnesses and packaged-app validation flows

### QA Policy
Every task must include:
- implementation acceptance criteria
- at least one happy-path scenario
- at least one error/failure scenario
- evidence output under `.sisyphus/evidence/`

### Phase 2 Verification Rules
- A replay feature task is not complete unless it produces desktop-runtime proof, not just preview-path proof.
- A suite task is not complete unless `runner.suite.*` flows are exercised in a real Tauri runtime.
- A packaging task is not complete unless packaged first-run behavior is captured with evidence on a clean Windows state.
- Reporting/scheduler/CI/collaboration tasks must prove secret-safe outputs and artifact consistency.

---

## Execution Strategy

### File Structure Map

> Phase 2 extends the existing repository rather than redefining it. The main hotspots expected to change are below.

```text
/
├── src/
│   ├── App.tsx
│   ├── components/
│   │   └── StatusBar.tsx
│   ├── routes/
│   │   ├── test-runner.tsx
│   │   ├── web-recorder.tsx
│   │   ├── api-tester.tsx
│   │   └── [phase-2 reporting/scheduler routes if needed]
│   ├── services/
│   │   ├── tauri-client.ts
│   │   ├── runner-client.ts
│   │   ├── web-recorder-client.ts
│   │   └── [new typed phase-2 clients as needed]
│   ├── store/
│   │   └── run-store.ts
│   └── types/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── state.rs
│   │   ├── services/
│   │   │   ├── browser_automation_service.rs
│   │   │   ├── runner_orchestration_service.rs
│   │   │   ├── artifact_service.rs
│   │   │   ├── secret_service.rs
│   │   │   └── [scheduler / reporting services if needed]
│   │   ├── repositories/
│   │   │   └── runner_repository.rs
│   │   └── utils/
│   │       └── paths.rs
│   └── migrations/
├── tests/
│   ├── frontend/
│   ├── rust/
│   └── smoke/
└── .sisyphus/
    └── evidence/
```

### Parallel Execution Waves

```text
Wave 1 (Phase 2A — runtime closure & hardening)
├── P2-T1: Browser runtime provisioning + replay stabilization
├── P2-T2: Desktop-runtime replay proof + recorder/replay hydration completion
├── P2-T3: Mixed suite execution proof + runner runtime completion
├── P2-T4: Windows packaging + first-run runtime validation
└── P2-T5: Secret rotation + security hardening follow-up

Wave 2 (Phase 2B — operational product expansion)
├── P2-T6: Advanced reporting baseline + run analytics surfaces
├── P2-T7: Local scheduling + unattended suite execution baseline
├── P2-T8: CI/CD handoff contract + machine-readable execution outputs
└── P2-T9: Collaboration-safe asset package import/export baseline

Wave 3 (integration & release tightening)
├── P2-T10: Browser/runtime stabilization follow-up + optional richer browser readiness seams
├── P2-T11: Reporting/scheduler/CI/collaboration integration hardening
└── P2-T12: Phase 2 smoke flows + operational readiness pass

Wave FINAL (parallel verification)
├── P2-F1: Plan compliance audit
├── P2-F2: Code quality review
├── P2-F3: Real QA scenario execution
└── P2-F4: Scope fidelity and release-readiness check
```

### Dependency Matrix

| Task | Blocked By | Blocks |
|---|---|---|
| P2-T1 | None | P2-T2, P2-T3, P2-T4, P2-T10 |
| P2-T2 | P2-T1 | P2-T3, P2-T12 |
| P2-T3 | P2-T1, P2-T2 | P2-T7, P2-T8, P2-T11, P2-T12 |
| P2-T4 | P2-T1 | P2-T12 |
| P2-T5 | None | P2-T6, P2-T7, P2-T8, P2-T9, P2-T11 |
| P2-T6 | P2-T3, P2-T5 | P2-T11, P2-T12 |
| P2-T7 | P2-T3, P2-T5 | P2-T8, P2-T11, P2-T12 |
| P2-T8 | P2-T3, P2-T5, P2-T7 | P2-T11, P2-T12 |
| P2-T9 | P2-T5 | P2-T11, P2-T12 |
| P2-T10 | P2-T1, P2-T2, P2-T4 | P2-T11, P2-T12 |
| P2-T11 | P2-T6, P2-T7, P2-T8, P2-T9, P2-T10 | P2-T12 |
| P2-T12 | P2-T2, P2-T3, P2-T4, P2-T6, P2-T7, P2-T8, P2-T9, P2-T10, P2-T11 | P2-F1-P2-F4 |

### Agent Dispatch Summary
- **Wave 1**
  - P2-T1 → `deep`
  - P2-T2 → `unspecified-high`
  - P2-T3 → `unspecified-high`
  - P2-T4 → `quick`
  - P2-T5 → `unspecified-high`
- **Wave 2**
  - P2-T6 → `visual-engineering`
  - P2-T7 → `unspecified-high`
  - P2-T8 → `deep`
  - P2-T9 → `unspecified-high`
- **Wave 3**
  - P2-T10 → `deep`
  - P2-T11 → `unspecified-high`
  - P2-T12 → `unspecified-high`
- **Final**
  - P2-F1 → `oracle`
  - P2-F2 → `unspecified-high`
  - P2-F3 → `unspecified-high`
  - P2-F4 → `deep`

---

## TODOs

- [ ] P2-T1. Browser runtime provisioning + replay stabilization

  **What to do**:
  - Harden `BrowserAutomationService` so replay runtime prerequisites, Chromium discovery, health reporting, and interaction execution are deterministic in the target desktop environment.
  - Reduce the gap between preview-only recorder evidence and desktop replay reality.
  - Preserve the browser abstraction boundary while upgrading runtime readiness and diagnostics.

  **Must NOT do**:
  - Không đẩy browser-specific internals ra ngoài `BrowserAutomationService`.
  - Không fake success cho replay interactions khi runtime thực tế không chứng minh được kết quả.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Runtime/browser stabilization crosses health discovery, replay semantics, and fallback policy.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with P2-T4, P2-T5)
  - **Blocks**: P2-T2, P2-T3, P2-T4, P2-T10
  - **Blocked By**: None

  **Files**:
  - Modify: `src-tauri/src/services/browser_automation_service.rs`
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src-tauri/src/state.rs`
  - Modify: `src/services/web-recorder-client.ts`
  - Test: `tests/frontend/browser-replay-t14.test.ts`
  - Test: `tests/frontend/browser-replay-t14-smoke.ts`

  **Acceptance Criteria**:
  - [ ] Chromium/runtime discovery is deterministic and surfaces actionable diagnostics.
  - [ ] Replay interaction path no longer depends on preview-only fallback to appear healthy.
  - [ ] Runtime health changes are reflected through existing shell metadata / event seams.

  **QA Scenarios**:
  ```
  Scenario: Replay runtime preflight reports actionable diagnostics
    Tool: Bash
    Preconditions: Machine with and without valid Chromium runtime available
    Steps:
      1. Trigger browser health check in both environments
      2. Inspect returned status and guidance
      3. Save command output
    Expected Result: Health result clearly distinguishes healthy vs blocked runtime states
    Failure Indicators: Ambiguous status or missing remediation guidance
    Evidence: .sisyphus/evidence/p2-task-T1-runtime-health.txt

  Scenario: Replay interaction path refuses unsupported/broken runtime honestly
    Tool: Bash + smoke harness
    Preconditions: Runtime intentionally misconfigured
    Steps:
      1. Run replay smoke harness
      2. Observe status output
      3. Confirm no false-positive pass is emitted
    Expected Result: Harness reports a truthful blocked/fail result with diagnostics
    Failure Indicators: Silent failure or fake PASS
    Evidence: .sisyphus/evidence/p2-task-T1-replay-runtime-blocked.txt
  ```

  **Commit**: YES
  - Message: `fix(browser): stabilize runtime provisioning and replay health`

- [x] P2-T2. Desktop-runtime replay proof + recorder/replay hydration completion

  **What to do**:
  - Complete the replay/authoring seams that still rely on local workspace cache or preview-only UI assumptions.
  - Ensure saved scripts can be loaded, replayed, and inspected from the real desktop runtime with evidence.
  - Close the gap between “record flow works” and “saved script replay works” in actual runtime conditions.

  **Must NOT do**:
  - Không dùng browser-only preview path làm bằng chứng duy nhất cho desktop replay.
  - Không thêm editor/recorder scope creep như assertions auto-generation, loops, hay multi-tab flows.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Crosses typed client, recorder route, persistence hydration, and runtime proof.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Blocks**: P2-T3, P2-T12
  - **Blocked By**: P2-T1

  **Files**:
  - Modify: `src/routes/web-recorder.tsx`
  - Modify: `src/services/web-recorder-client.ts`
  - Modify: `src/types/commands.ts`
  - Modify: `src/types/dto.ts`
  - Modify: `src-tauri/src/lib.rs`
  - Create: `src-tauri/src/repositories/ui_script_repository.rs`
  - Test: `tests/frontend/browser-replay-t14.test.ts`
  - Test: `tests/frontend/browser-replay-t14-smoke.ts`

  **Acceptance Criteria**:
  - [ ] Saved UI scripts can be loaded/hydrated in the desktop runtime without relying on session-local cache only.
  - [ ] Simple replay flow runs end-to-end in the target runtime with evidence.
  - [ ] Replay result exposes truthful status, failed-step detail, and screenshot-on-fail path where applicable.

  **QA Scenarios**:
  ```
  Scenario: Replay a saved UI script in desktop runtime
    Tool: interactive_bash + browser automation
    Preconditions: Valid Chromium runtime, one saved script exists
    Steps:
      1. Launch real desktop app runtime
      2. Load a saved script from persisted storage
      3. Trigger replay and inspect result status
    Expected Result: Replay completes or fails truthfully with inspectable details
    Failure Indicators: Missing hydration, no replay surface, or unverifiable result
    Evidence: .sisyphus/evidence/p2-task-T2-desktop-replay.txt

  Scenario: Replay failure captures screenshot and failed-step context
    Tool: interactive_bash + browser automation
    Preconditions: Script with intentional failure step
    Steps:
      1. Launch desktop runtime
      2. Run failing script
      3. Inspect failure detail and artifact path
    Expected Result: Screenshot and failure metadata are persisted consistently
    Failure Indicators: Missing artifact, generic failure, or broken result DTO
    Evidence: .sisyphus/evidence/p2-task-T2-replay-fail.txt
  ```

  **Commit**: YES
  - Message: `feat(ui-replay): complete desktop replay hydration and runtime proof`

- [ ] P2-T3. Mixed suite execution proof + runner runtime completion

  **What to do**:
  - Validate and harden mixed suite execution in the real desktop runtime, including history hydration, live progress, cancel, rerun-failed, and detail inspection.
  - Ensure suite execution no longer depends on preview-only paths for evidence.
  - Tighten runner orchestration where runtime sequencing or result consistency remains weak.

  **Must NOT do**:
  - Không thêm distributed execution hay parallel runner complexity vượt Phase 2 scope.
  - Không tạo model runner thứ hai song song với seam hiện có.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cutting runtime proof across backend orchestration, repository, UI state, and event flow.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Blocks**: P2-T6, P2-T7, P2-T8, P2-T11, P2-T12
  - **Blocked By**: P2-T1, P2-T2

  **Files**:
  - Modify: `src-tauri/src/services/runner_orchestration_service.rs`
  - Modify: `src-tauri/src/repositories/runner_repository.rs`
  - Modify: `src/routes/test-runner.tsx`
  - Modify: `src/store/run-store.ts`
  - Modify: `src/services/runner-client.ts`
  - Modify: `src/types/dto.ts`
  - Test: `tests/frontend/suite-runner-t15.test.ts`
  - Test: `tests/frontend/test-runner-t16.test.ts`
  - Create: `tests/smoke/desktop-suite-runtime-p2-smoke.ts`

  **Acceptance Criteria**:
  - [ ] Real desktop runtime can load suites/history and execute a mixed suite.
  - [ ] Progress, final result, rerun-failed, and detail inspection remain consistent.
  - [ ] Cancel/stop semantics remain idempotent under mixed execution conditions.

  **QA Scenarios**:
  ```
  Scenario: Run mixed API + UI suite in desktop runtime
    Tool: interactive_bash + browser automation
    Preconditions: Valid environment, at least one API case and one UI case in a suite
    Steps:
      1. Launch desktop runtime
      2. Open Test Runner and hydrate suites/history
      3. Execute mixed suite and inspect progress + final summary
    Expected Result: Mixed suite executes successfully with persisted history and detail records
    Failure Indicators: Runner screen cannot hydrate, disabled run path, or inconsistent result persistence
    Evidence: .sisyphus/evidence/p2-task-T3-mixed-suite.txt

  Scenario: Rerun failed targets only after mixed suite failure
    Tool: interactive_bash + browser automation
    Preconditions: Prior mixed suite run contains at least one failed target
    Steps:
      1. Select historical failed run
      2. Trigger rerun failed
      3. Compare execution scope to failed targets only
    Expected Result: Only failed targets rerun and new run history remains consistent
    Failure Indicators: Wrong target scope or corrupt run history
    Evidence: .sisyphus/evidence/p2-task-T3-rerun-failed.txt
  ```

  **Commit**: YES
  - Message: `feat(runner): prove and harden mixed suite desktop execution`

- [x] P2-T4. Windows packaging + first-run runtime validation

  **What to do**:
  - Produce honest packaged-app verification for Windows-first distribution, including first-run bootstrap, version/runtime surface, and missing-runtime guidance.
  - Ensure packaged flows exercise the same bootstrap policy as desktop runtime and do not corrupt app-data on failure.

  **Reference framing**:
  - Use Phase 1 packaging artifacts only as partial seam baselines (`tests/frontend/packaging-bootstrap-t17.test.ts`, `.sisyphus/evidence/task-T17-packaging-bootstrap-2026-04-01.txt`).
  - Do not assume packaged-runtime evidence already exists; this task must create the first real packaged-app runtime evidence set for Windows first-run and missing-runtime behavior.

  **Must NOT do**:
  - Không đánh dấu packaging complete chỉ bằng source assertions hoặc config inspection.
  - Không thêm auto-update infrastructure trong task này.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Scope is narrow but evidence-sensitive around packaging/bootstrap validation.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with P2-T1, P2-T5)
  - **Blocks**: P2-T12
  - **Blocked By**: P2-T1

  **Files**:
  - Modify: `src-tauri/tauri.conf.json`
  - Modify: `src-tauri/src/main.rs`
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src-tauri/src/utils/paths.rs`
  - Modify: `src/App.tsx`
  - Modify: `src/components/StatusBar.tsx`
  - Test: `tests/frontend/packaging-bootstrap-t17.test.ts`
  - Create: `tests/smoke/windows-packaged-bootstrap-p2-smoke.ts`

  **Acceptance Criteria**:
  - [ ] Packaged Windows app initializes clean app-data on first run.
  - [ ] Packaged app shows accurate version/runtime metadata in shell UI.
  - [ ] Missing runtime/browser prerequisite fails clearly while preserving API-only usability and user data safety.

  **QA Scenarios**:
  ```
  Scenario: First-run bootstrap from packaged Windows app
    Tool: interactive_bash
    Preconditions: Packaged build available; clean user profile/app-data path
    Steps:
      1. Launch packaged app in clean state
      2. Observe bootstrap result and generated directories
      3. Capture shell version/runtime guidance
    Expected Result: App boots, initializes storage, and surfaces runtime metadata clearly
    Failure Indicators: Startup crash, missing directories, or stale shell metadata
    Evidence: .sisyphus/evidence/p2-task-T4-first-run.txt

  Scenario: Packaged app under missing browser/runtime condition
    Tool: interactive_bash
    Preconditions: Packaged app with browser runtime intentionally unavailable
    Steps:
      1. Launch packaged app
      2. Attempt browser-related action
      3. Confirm API-only features remain usable
    Expected Result: Actionable guidance is shown and app data remains intact
    Failure Indicators: Global app crash or unusable error surface
    Evidence: .sisyphus/evidence/p2-task-T4-runtime-missing.txt
  ```

  **Commit**: YES
  - Message: `chore(release): verify packaged first-run and runtime guidance`

- [x] P2-T5. Secret rotation + security hardening follow-up

  **What to do**:
  - Complete the secret-management follow-up left open after Phase 1, especially key rotation and stronger handling around exports, scheduled runs, and collaboration-safe packages.
  - Reconfirm that all new Phase 2 outputs stay aligned with redaction policy.

  **Must NOT do**:
  - Không weaken redaction defaults vì tiện debug.
  - Không log raw secret values trong scheduler, CI handoff, hay collaboration paths.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cuts storage, export, and new operational outputs.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with P2-T1, P2-T4)
  - **Blocks**: P2-T6, P2-T7, P2-T8, P2-T9, P2-T11
  - **Blocked By**: None

  **Files**:
  - Modify: `src-tauri/src/services/secret_service.rs`
  - Modify: `src-tauri/src/services/artifact_service.rs`
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src/types/dto.ts`
  - Create: `tests/frontend/security-export-redaction-p2.test.ts`
  - Create: `tests/rust/secret_rotation_p2.rs`

  **Acceptance Criteria**:
  - [ ] Key rotation or equivalent secret-hardening follow-up is implemented and verified.
  - [ ] New Phase 2 outputs remain redacted by default.
  - [ ] Existing secrets remain recoverable or fail clearly during rotation/migration paths.

  **QA Scenarios**:
  ```
  Scenario: Rotate or refresh secret material safely
    Tool: Bash + cargo test
    Preconditions: Existing encrypted secret data present
    Steps:
      1. Trigger key rotation / hardening flow
      2. Reload protected entities
      3. Verify no raw secrets leak in logs or outputs
    Expected Result: Secret handling remains correct and rotation path is auditable
    Failure Indicators: Decryption loss, corruption, or raw secret leakage
    Evidence: .sisyphus/evidence/p2-task-T5-key-rotation.txt

  Scenario: Export/report path remains redacted after hardening changes
    Tool: Bash
    Preconditions: Test entities containing secret variables
    Steps:
      1. Generate export/report output
      2. Inspect serialized content
      3. Verify masked secret values
    Expected Result: Secrets remain redacted everywhere by default
    Failure Indicators: Raw secret values in output
    Evidence: .sisyphus/evidence/p2-task-T5-export-redaction.txt
  ```

  **Commit**: YES
  - Message: `fix(security): add secret rotation and output hardening`

- [x] P2-T6. Advanced reporting baseline + run analytics surfaces

  **What to do**:
  - Expand Phase 1 basic reporting into a usable operational reporting surface for QA leads.
  - Add filterable summaries, grouped failure views, artifact-aware detail drilldown, and trend-ready aggregates without building a heavyweight analytics platform.

  **Must NOT do**:
  - Không biến reporting thành analytics dashboard quá scope.
  - Không duplicate existing runner detail model bằng một reporting backend riêng biệt nếu chưa cần.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Heavy UI/reporting work with supporting backend query extensions.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with P2-T9)
  - **Blocks**: P2-T11, P2-T12
  - **Blocked By**: P2-T3, P2-T5

  **Files**:
  - Modify: `src/routes/test-runner.tsx`
  - Create: `src/routes/reporting.tsx` (if route split becomes necessary)
  - Modify: `src/services/runner-client.ts`
  - Modify: `src/types/dto.ts`
  - Modify: `src-tauri/src/repositories/runner_repository.rs`
  - Modify: `src-tauri/src/services/artifact_service.rs`
  - Test: `tests/frontend/test-runner-t16.test.ts`
  - Create: `tests/frontend/reporting-route-p2.test.ts`

  **Acceptance Criteria**:
  - [x] Users can filter runs by suite/status/date range.
  - [x] Reporting surface shows grouped summaries and artifact-backed failure drilldown.
  - [x] Trend-ready aggregates exist without exposing raw secrets in previews or exports.

  **QA Scenarios**:
  ```
  Scenario: Filter and inspect historical run summaries
    Tool: Playwright
    Preconditions: Multiple runs with varied suites/statuses exist
    Steps:
      1. Open reporting surface
      2. Apply suite/status/date filters
      3. Inspect grouped summary and one failed run detail
    Expected Result: Filters and drilldown behave consistently with persisted run data
    Failure Indicators: Incorrect counts, broken filters, or missing artifacts
    Evidence: .sisyphus/evidence/p2-task-T6-reporting-filters.txt

  Scenario: Reporting/export previews remain secret-safe
    Tool: Bash + Playwright
    Preconditions: Run history includes secret-backed requests
    Steps:
      1. Open reporting detail
      2. Export one report
      3. Inspect UI preview and exported content
    Expected Result: Secrets remain masked in UI and exported artifacts
    Failure Indicators: Unredacted sensitive values
    Evidence: .sisyphus/evidence/p2-task-T6-reporting-redaction.txt
  ```

  **Commit**: YES
  - Message: `feat(reporting): add advanced run summaries and drilldown`

- [ ] P2-T7. Local scheduling + unattended suite execution baseline

  **What to do**:
  - Add local scheduling for unattended suite execution with a narrow, internal-ops-friendly scope.
  - Scheduled runs must reuse the same runner/orchestration/artifact model as manual runs.
  - Surface schedule state, last/next run, and failure diagnostics without inventing a separate job platform.

  **Must NOT do**:
  - Không xây distributed worker system.
  - Không tạo execution path riêng tách khỏi runner orchestration hiện có.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Scheduling crosses backend lifecycle, persistence, and runner reuse.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with P2-T8 after backend model settles)
  - **Blocks**: P2-T8, P2-T11, P2-T12
  - **Blocked By**: P2-T3, P2-T5

  **Files**:
  - Create: `src-tauri/src/services/scheduler_service.rs`
  - Create: `src-tauri/migrations/004_add_suite_schedules.sql`
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src-tauri/src/state.rs`
  - Modify: `src-tauri/src/services/runner_orchestration_service.rs`
  - Create: `src/services/scheduler-client.ts`
  - Create: `src/routes/schedules.tsx` or embed into existing runner/settings surface
  - Create: `tests/frontend/scheduler-route-p2.test.ts`
  - Create: `tests/rust/scheduler_service_p2.rs`

  **Acceptance Criteria**:
  - [ ] Users can create, enable/disable, and inspect a local schedule for a suite.
  - [ ] Scheduled runs execute through the normal runner pipeline and produce standard history/artifacts.
  - [ ] Failed scheduled runs surface actionable diagnostics.

  **QA Scenarios**:
  ```
  Scenario: Scheduled suite executes unattended
    Tool: Bash + interactive_bash
    Preconditions: One enabled schedule exists for a runnable suite
    Steps:
      1. Start desktop runtime with scheduler active
      2. Wait for scheduled trigger
      3. Inspect resulting run history and artifacts
    Expected Result: Scheduled run appears like a normal run with machine-attributed metadata
    Failure Indicators: No run, duplicate runs, or corrupted result persistence
    Evidence: .sisyphus/evidence/p2-task-T7-scheduled-run.txt

  Scenario: Disabled schedule does not execute
    Tool: Bash
    Preconditions: Existing schedule is disabled
    Steps:
      1. Wait through trigger window
      2. Inspect run history
    Expected Result: No new run is created
    Failure Indicators: Unexpected execution or stale status surface
    Evidence: .sisyphus/evidence/p2-task-T7-schedule-disable.txt
  ```

  **Commit**: YES
  - Message: `feat(schedule): add local suite scheduling baseline`

- [ ] P2-T8. CI/CD handoff contract + machine-readable execution outputs

  **What to do**:
  - Define and implement the narrowest useful CI/CD handoff path for TestForge.
  - Produce deterministic machine-readable outputs, stable exit semantics, and exportable run definitions/results that external pipelines can consume.
  - Keep this as a handoff/integration contract, not a full headless platform rewrite unless explicitly approved later.

  **Must NOT do**:
  - Không biến task này thành full CLI platform hoặc remote execution control plane.
  - Không phá typed contracts đã tồn tại để phục vụ CI riêng.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires architecture judgment around execution contracts, exit codes, and integration boundaries.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Blocks**: P2-T11, P2-T12
  - **Blocked By**: P2-T3, P2-T5, P2-T7

  **Files**:
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src-tauri/src/services/runner_orchestration_service.rs`
  - Modify: `src-tauri/src/services/artifact_service.rs`
  - Create: `src-tauri/src/services/ci_handoff_service.rs` (if needed)
  - Create: `src/services/ci-client.ts` (if typed frontend management is needed)
  - Modify: `src/types/dto.ts`
  - Create: `tests/rust/ci_handoff_service_p2.rs`
  - Create: `tests/frontend/ci-handoff-p2.test.ts`

  **Acceptance Criteria**:
  - [ ] Machine-readable run output format is documented and stable.
  - [ ] CI handoff path exposes clear success/fail/blocked semantics.
  - [ ] Exported payloads remain secret-safe and artifact-consistent.

  **QA Scenarios**:
  ```
  Scenario: Produce machine-readable execution output for pipeline consumption
    Tool: Bash
    Preconditions: Runnable suite and CI handoff path configured
    Steps:
      1. Trigger CI handoff/export path
      2. Capture generated output payload
      3. Verify status and artifact references
    Expected Result: Output is deterministic, parseable, and aligned with run results
    Failure Indicators: Ambiguous status or inconsistent payload shape
    Evidence: .sisyphus/evidence/p2-task-T8-ci-output.txt

  Scenario: CI handoff remains redacted with secret-backed inputs
    Tool: Bash
    Preconditions: Suite uses secret-backed values
    Steps:
      1. Trigger CI handoff/export
      2. Inspect payload/log output
    Expected Result: No raw secrets appear in machine-readable outputs
    Failure Indicators: Raw secret leakage
    Evidence: .sisyphus/evidence/p2-task-T8-ci-redaction.txt
  ```

  **Commit**: YES
  - Message: `feat(ci): add pipeline handoff contract and outputs`

- [ ] P2-T9. Collaboration-safe asset package import/export baseline

  **What to do**:
  - Add a lightweight collaboration baseline through import/export/shareable asset packages for environments, collections, suites, and supporting metadata.
  - Focus on review-safe, secret-aware packaging rather than server-backed collaboration.

  **Must NOT do**:
  - Không xây multi-user realtime sync.
  - Không export raw secrets by default.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Crosses packaging, metadata modeling, and security policy.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with P2-T6)
  - **Blocks**: P2-T11, P2-T12
  - **Blocked By**: P2-T5

  **Files**:
  - Modify: `src-tauri/src/services/artifact_service.rs`
  - Create: `src-tauri/src/services/collaboration_package_service.rs`
  - Modify: `src-tauri/src/lib.rs`
  - Create: `src/services/collaboration-client.ts`
  - Create: `src/routes/collaboration.tsx` or embed into settings/reporting surface
  - Modify: `src/types/dto.ts`
  - Create: `tests/frontend/collaboration-package-p2.test.ts`
  - Create: `tests/rust/collaboration_package_p2.rs`

  **Acceptance Criteria**:
  - [ ] Users can export a shareable asset package without raw secrets.
  - [ ] Import flow validates payloads and reports conflicts clearly.
  - [ ] Shared package metadata is sufficient for review/use by another internal user.

  **QA Scenarios**:
  ```
  Scenario: Export collaboration-safe asset package
    Tool: Bash + Playwright
    Preconditions: Existing suites/environments/data tables present
    Steps:
      1. Trigger export package flow
      2. Inspect generated package contents
      3. Verify secret masking and artifact references
    Expected Result: Package is shareable internally without exposing raw secrets
    Failure Indicators: Missing dependencies or secret leakage
    Evidence: .sisyphus/evidence/p2-task-T9-export-package.txt

  Scenario: Import collaboration package with validation feedback
    Tool: Bash + Playwright
    Preconditions: Valid and intentionally-invalid package samples available
    Steps:
      1. Import valid package
      2. Import invalid/conflicting package
      3. Inspect validation/error messaging
    Expected Result: Valid import succeeds; invalid import fails clearly and safely
    Failure Indicators: Silent overwrite or vague validation errors
    Evidence: .sisyphus/evidence/p2-task-T9-import-package.txt
  ```

  **Commit**: YES
  - Message: `feat(collab): add secret-safe asset package sharing baseline`

- [ ] P2-T10. Browser/runtime stabilization follow-up + richer browser readiness seams

  **What to do**:
  - After the Phase 2A runtime proof tasks land, reassess browser/runtime readiness for broader browser support stabilization.
  - Improve runtime abstraction seams so future browser expansion remains possible without promising full parity now.

  **Must NOT do**:
  - Không cam kết full Firefox/WebKit parity trong task này.
  - Không leak runtime-specific branches ra khắp codebase.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Requires architecture-level refinement after replay/package stabilization.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: P2-T11, P2-T12
  - **Blocked By**: P2-T1, P2-T2, P2-T4

  **Files**:
  - Modify: `src-tauri/src/services/browser_automation_service.rs`
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src/types/dto.ts`
  - Test: `tests/frontend/browser-replay-t14.test.ts`
  - Create: `tests/rust/browser_readiness_p2.rs`

  **Acceptance Criteria**:
  - [ ] Browser runtime abstraction is cleaner and more future-ready than Phase 1 baseline.
  - [ ] Readiness diagnostics distinguish current support vs future-ready seams clearly.
  - [ ] No current Chromium path regresses while improving extensibility.

  **QA Scenarios**:
  ```
  Scenario: Browser readiness diagnostics remain truthful after abstraction hardening
    Tool: Bash
    Preconditions: Current Chromium runtime available
    Steps:
      1. Run readiness checks
      2. Inspect reported capabilities and limitations
    Expected Result: Current support and limitations are explicit and accurate
    Failure Indicators: Over-promising unsupported runtime capabilities
    Evidence: .sisyphus/evidence/p2-task-T10-browser-readiness.txt

  Scenario: Existing Chromium replay path remains stable
    Tool: smoke harness
    Preconditions: Chromium runtime available
    Steps:
      1. Run replay smoke harness
      2. Compare result to pre-change baseline
    Expected Result: Chromium replay remains stable or improves
    Failure Indicators: Regression in current supported path
    Evidence: .sisyphus/evidence/p2-task-T10-chromium-regression.txt
  ```

  **Commit**: YES
  - Message: `refactor(browser): harden future-ready runtime seams`

- [ ] P2-T11. Reporting/scheduler/CI/collaboration integration hardening

  **What to do**:
  - Tighten the cross-feature seams introduced in Phase 2 so reporting, scheduling, CI handoff, and collaboration packages all remain consistent with the same runner/result/artifact model.
  - Focus on integration quality, not feature expansion.

  **Must NOT do**:
  - Không mở scope thêm feature mới lớn ở task hardening này.
  - Không để mỗi subsystem serialize status/result theo format khác nhau.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cutting integration hardening across multiple Phase 2 subsystems.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Blocks**: P2-T12
  - **Blocked By**: P2-T6, P2-T7, P2-T8, P2-T9, P2-T10

  **Files**:
  - Modify: `src/types/dto.ts`
  - Modify: `src/types/events.ts`
  - Modify: `src-tauri/src/lib.rs`
  - Modify: `src-tauri/src/services/scheduler_service.rs`
  - Modify: `src-tauri/src/services/ci_handoff_service.rs`
  - Modify: `src-tauri/src/services/collaboration_package_service.rs`
  - Modify: `src/routes/reporting.tsx`
  - Modify: `src/routes/schedules.tsx`
  - Modify: `src/routes/collaboration.tsx`
  - Create: `tests/frontend/phase2-cross-surface-p2.test.ts`
  - Create: `tests/rust/phase2_integration_p2.rs`

  **Acceptance Criteria**:
  - [ ] Reporting, scheduling, CI handoff, and collaboration features consume a consistent run/result/artifact model.
  - [ ] Error semantics remain understandable and secret-safe across all Phase 2 surfaces.
  - [ ] No Phase 2 subsystem bypasses the typed client / service / repository boundaries.

  **QA Scenarios**:
  ```
  Scenario: One run can be observed consistently across reporting, scheduler history, and CI handoff views
    Tool: Playwright + Bash
    Preconditions: At least one scheduled or manual run exists
    Steps:
      1. Inspect the run in reporting
      2. Inspect the same run from scheduler/CI-related surface
      3. Compare summary and artifact references
    Expected Result: Core run facts remain consistent across surfaces
    Failure Indicators: Divergent status, counts, or artifacts
    Evidence: .sisyphus/evidence/p2-task-T11-cross-surface-consistency.txt

  Scenario: Secret-safe error semantics across all Phase 2 outputs
    Tool: Bash
    Preconditions: Trigger one failure path in each new subsystem
    Steps:
      1. Capture errors/logs/outputs
      2. Inspect for clarity and leakage
    Expected Result: Errors are actionable without exposing sensitive data
    Failure Indicators: Raw secrets or opaque generic failures
    Evidence: .sisyphus/evidence/p2-task-T11-error-hardening.txt
  ```

  **Commit**: YES
  - Message: `fix(core): harden phase-2 integration seams`

- [ ] P2-T12. Phase 2 smoke flows + operational readiness pass

  **What to do**:
  - Execute the approved smoke set for Phase 2, capture evidence, and evaluate whether runtime stabilization plus new operational features are honestly ready.
  - Produce a concise operational readiness verdict with explicit PASS/BLOCKED/FAIL distinctions.

  **Must NOT do**:
  - Không mark Phase 2 ready nếu runtime/package/scheduler/CI/collaboration proof vẫn còn suy diễn.
  - Không bỏ qua blocked evidence để giữ narrative đẹp.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Acceptance sweep across all major Phase 2 seams with strong evidence discipline.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Blocks**: P2-F1-P2-F4
  - **Blocked By**: P2-T2, P2-T3, P2-T4, P2-T6, P2-T7, P2-T8, P2-T9, P2-T10, P2-T11

  **Files**:
  - Create: `tests/frontend/p2-smoke-report.ts` or equivalent smoke wrapper
  - Modify: `package.json` (only if adding a dedicated Phase 2 smoke script is justified)
  - Create: `.sisyphus/evidence/p2-task-T12-operational-readiness.txt`

  **Acceptance Criteria**:
  - [ ] Phase 2 minimum smoke scenarios execute with evidence.
  - [ ] Operational readiness verdict is explicit and honest.
  - [ ] Remaining blocked items, if any, are documented with remediation guidance.

  **QA Scenarios**:
  ```
  Scenario: Execute Phase 2 minimum smoke set
    Tool: Playwright + Bash + interactive_bash
    Preconditions: All core Phase 2 features implemented
    Steps:
      1. Run replay runtime smoke
      2. Run mixed suite runtime smoke
      3. Run packaged first-run smoke
      4. Run reporting filter/detail smoke
      5. Run scheduler smoke
      6. Run CI handoff smoke
      7. Run collaboration package smoke
    Expected Result: Evidence set is complete and verdictable
    Failure Indicators: Missing evidence, unverifiable flows, or contradictory outputs
    Evidence: .sisyphus/evidence/p2-task-T12-operational-readiness.txt

  Scenario: Phase 2 readiness verdict with blocker accounting
    Tool: Bash
    Preconditions: Smoke evidence collected
    Steps:
      1. Review all smoke outputs
      2. Classify PASS / BLOCKED / FAIL honestly
      3. Write final readiness summary
    Expected Result: Final summary is explicit, defensible, and evidence-backed
    Failure Indicators: Evidence-free optimism or unclear blocker accounting
    Evidence: .sisyphus/evidence/p2-task-T12-readiness-summary.txt
  ```

  **Commit**: YES
  - Message: `test(phase2): run operational smoke and readiness review`

---

## Final Verification Wave

- [ ] P2-F1. **Plan Compliance Audit** — `oracle`
  - Verify each locked Phase 2 scope decision is preserved.
  - Verify Phase 2 did not reintroduce forbidden scope such as server-backed collaboration or browser-internals leakage.
  - Confirm evidence exists for runtime closure work and Phase 2 expansion features.

  **QA Scenarios**:
  ```
  Scenario: Audit implemented Phase 2 scope against plan guardrails
    Tool: Oracle + read-only repo inspection
    Preconditions: All planned Phase 2 tasks implemented and evidence files collected
    Steps:
      1. Compare final code/evidence against this Phase 2 plan
      2. Check each Must Have and Must NOT Have item
      3. Record any scope violations or missing proof
    Expected Result: Explicit compliance verdict with cited violations if present
    Failure Indicators: Missing guardrail audit or evidence-free approval
    Evidence: .sisyphus/evidence/p2-final-F1-plan-compliance.txt
  ```

- [ ] P2-F2. **Code Quality Review** — `unspecified-high`
  - Run full type/test/build verification.
  - Review for direct IPC leakage, secret logging, status-model drift, unstable abstractions, and over-scoped features.

  **QA Scenarios**:
  ```
  Scenario: Run full Phase 2 verification command set
    Tool: Bash
    Preconditions: All Phase 2 code changes completed
    Steps:
      1. Run `npm run build`
      2. Run `npm test`
      3. Run `cargo test`
      4. Save outputs and summarize failures
    Expected Result: Verification commands pass or any pre-existing failures are explicitly isolated
    Failure Indicators: Unexplained failing command or missing command evidence
    Evidence: .sisyphus/evidence/p2-final-F2-code-quality.txt

  Scenario: Review architecture and secret-safety regressions
    Tool: read-only code inspection
    Preconditions: Verification command outputs available
    Steps:
      1. Inspect typed IPC boundaries
      2. Inspect browser/runtime abstraction boundaries
      3. Inspect logs/export/output serializers for secret leakage
    Expected Result: Clear review verdict on architecture integrity and secret safety
    Failure Indicators: Direct invoke leakage, browser leakage, or raw secret exposure
    Evidence: .sisyphus/evidence/p2-final-F2-architecture-review.txt
  ```

- [ ] P2-F3. **Real QA Scenario Execution** — `unspecified-high`
  - Execute end-to-end scenarios for replay, mixed suite execution, packaged first-run, reporting, scheduling, CI handoff, and collaboration package flows.
  - Save final evidence under `.sisyphus/evidence/final-qa-phase-2/` or equivalent agreed evidence directory.

  **QA Scenarios**:
  ```
  Scenario: Execute end-to-end Phase 2 QA scenario pack
    Tool: Playwright + Bash + interactive_bash
    Preconditions: Desktop runtime and packaged runtime prerequisites available
    Steps:
      1. Run replay scenario
      2. Run mixed suite scenario
      3. Run packaged first-run scenario
      4. Run reporting, scheduler, CI handoff, and collaboration scenarios
      5. Save screenshots, command logs, and summaries
    Expected Result: Full QA scenario pack completes with explicit pass/blocked/fail accounting
    Failure Indicators: Missing scenario evidence or unverifiable runtime behavior
    Evidence: .sisyphus/evidence/p2-final-F3-real-qa.txt
  ```

- [ ] P2-F4. **Scope Fidelity and Release-Readiness Check** — `deep`
  - Verify browser abstraction remains isolated and future-ready.
  - Confirm Phase 2 operational surfaces are honest, supportable, and still aligned with the Windows-first desktop product direction.

  **QA Scenarios**:
  ```
  Scenario: Evaluate Phase 2 release readiness and residual blockers
    Tool: Bash + read-only review
    Preconditions: Final QA evidence and verification outputs available
    Steps:
      1. Review all final evidence files
      2. Classify remaining issues as pass/blocked/fail
      3. Write release-readiness verdict with remediation notes
    Expected Result: Honest release-readiness verdict aligned with evidence and scope guardrails
    Failure Indicators: Release approval without blocker accounting or mismatch with evidence
    Evidence: .sisyphus/evidence/p2-final-F4-release-readiness.txt
  ```

---

## Commit Strategy

- Commit after each task or tightly related pair of tasks in the same wave.
- Use conventional messages, for example:
  - `fix(browser): stabilize runtime provisioning and replay health`
  - `feat(ui-replay): complete desktop replay hydration and runtime proof`
  - `feat(runner): prove and harden mixed suite desktop execution`
  - `chore(release): verify packaged first-run and runtime guidance`
  - `feat(reporting): add advanced run summaries and drilldown`
  - `feat(schedule): add local suite scheduling baseline`
  - `feat(ci): add pipeline handoff contract and outputs`
  - `feat(collab): add secret-safe asset package sharing baseline`

---

## Success Criteria

### Verification Commands
```bash
npm install
npm run build
cargo test
npm test
```

### Final Checklist
- [ ] Replay runtime is proven in the target desktop environment
- [ ] Mixed suite execution is proven in the real desktop runtime
- [ ] Windows packaging and first-run flow are proven with packaged-app evidence
- [ ] Reporting, scheduler, CI handoff, and collaboration package flows are each evidenced honestly
- [ ] Secret redaction and rotation policies remain enforced across all new outputs
- [ ] Browser abstraction and typed IPC contracts remain stable
- [ ] Phase 2 operational readiness is documented with explicit PASS/BLOCKED/FAIL accounting
