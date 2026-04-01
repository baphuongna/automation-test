# TestForge Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a usable internal MVP of TestForge for QA teams, covering API testing, basic Web UI recording/replay, suite execution, environments, data tables, basic reporting, packaging, and fallback-ready browser abstraction.

**Architecture:** TestForge is a Windows-first Tauri v2 desktop application with a React/TypeScript frontend and a Rust backend. The implementation must keep browser automation isolated behind `BrowserAutomationService`, preserve a stable IPC boundary, store core metadata in SQLite, keep large artifacts on the filesystem, and enforce strict secret-redaction rules throughout the product.

**Tech Stack:** Tauri v2, React 18, TypeScript, Vite, TailwindCSS, shadcn/ui, Zustand, TanStack Query, react-hook-form, zod, Rust, tokio, reqwest + rustls, rusqlite, playwright-rs v0.9, SQLite.

---

## TL;DR

> **Quick Summary**: Implement TestForge as a Chromium-first desktop QA tool using a fallback-ready browser abstraction and strict scope control. Deliver API testing first, browser recording/replay second, then suite orchestration, hardening, packaging, and release readiness.
>
> **Deliverables**:
> - Working desktop shell with typed IPC and SQLite bootstrap
> - Environment, secret storage, and data table management
> - API collections/endpoints/assertions with run results
> - Web recorder + step editor + replay flow on Chromium
> - Suite runner, history, export, packaging, and fallback gate
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES — 4 major implementation waves + final verification wave
> **Critical Path**: T1 → T4 → T8 → T12 → T15 → F1-F4

---

## Context

### Original Request
The user asked to analyze automation testing for web/API, choose between extension vs app, selected the desktop-app direction, and approved a detailed specification for TestForge Phase 1.

### Interview + Spec Summary
**Locked product decisions**:
- Windows-first internal desktop distribution
- Chromium-only support in Phase 1
- Test types only: `api` and `ui`
- No pause/resume, no hybrid tests, no multi-tab flows
- Browser runs in separate window; not embedded in the shell
- App-managed secret encryption baseline; OS keychain optional enhancement
- Basic reporting only; no analytics dashboards

**Technical constraints**:
- Blank-slate repository expected by executor
- Browser layer must stay behind `BrowserAutomationService`
- SQLite stores metadata; artifacts stay on filesystem
- UI contract must use typed IPC + stable domain events

### References
- Primary source of truth: `.sisyphus/drafts/automation-testing-tool-spec.md`
- Critical spec sections:
  - Sections 5-6: locked MVP scope and exclusions
  - Section 8 + Appendix A: database model and schema baseline
  - Section 9 + Appendix C: recorder UX and session behavior
  - Section 10 + Appendix D: backend/browser abstraction and DTO contracts
  - Sections 12-14: error model, testing strategy, security/privacy
  - Sections 16-18: milestones, fallback triggers, exit criteria

### Metis Review — Guardrails Incorporated
- Explicitly lock out API scope creep: no GraphQL, no WebSockets, no pre/post-request scripts, no response chaining automation in Phase 1
- Explicitly lock out recorder scope creep: no assertion auto-generation, no conditional logic, no loops, no advanced iframe/shadow DOM support
- Add performance and reliability acceptance categories to the implementation plan
- Treat Week-6 browser viability gate as mandatory, not advisory
- Add edge-case handling for oversized responses, missing variables, browser crash mid-recording, empty suites, and key-file corruption

---

## Work Objectives

### Core Objective
Deliver a shippable internal MVP that lets QA users create environments, define API tests, record/edit/replay simple UI scripts, execute mixed suites, review results, and package the app for Windows-first distribution.

### Concrete Deliverables
- Desktop shell with typed IPC, state boundaries, and settings bootstrap
- SQLite schema, migrations, app-data directory setup, and secret encryption baseline
- API testing feature set with endpoint CRUD, auth, assertions, and previews
- Browser recording/replay behind `BrowserAutomationService`
- Suite execution, history, exports, screenshots-on-fail, and release packaging

### Definition of Done
- [ ] App can bootstrap from empty local data directory without manual dev tooling
- [ ] QA can create environment + secret variables safely
- [ ] QA can create/run API test and inspect pass/fail details
- [ ] QA can record, edit, save, and replay a simple UI flow in Chromium
- [ ] QA can run a mixed suite and inspect progress/history
- [ ] Browser failure paths do not crash API-only features
- [ ] Packaging and first-run flow work on target Windows environment

### Must Have
- Stable typed IPC command/event boundary
- `BrowserAutomationService` abstraction with fallback-ready contract
- Strict redaction and encryption rules for secrets
- File-system artifact storage; no screenshot blobs in SQLite
- One active recording session at a time; one active UI replay at a time

### Must NOT Have (Guardrails)
- No multi-browser parity work in Phase 1
- No GraphQL, WebSocket, or scripting engines for API tests
- No hybrid API/UI test case model
- No embedded browser panel work
- No pause/resume, undo/redo, advanced analytics, CI/CD integration, or collaboration features
- No direct frontend `invoke()` calls outside the typed client layer
- No leaking `playwright-rs` internals outside browser service layer

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — All verification must be executable by the implementing agent using commands, browser automation, or terminal interaction.

### Test Decision
- **Infrastructure exists**: NO — blank slate project
- **Automated tests**: YES (tests-after with strong unit/integration coverage + selective smoke)
- **Frameworks**:
  - Rust: `cargo test`
  - Frontend: `vitest` + `@testing-library/react`
  - Desktop/browser smoke: browser-driven smoke harness / Tauri-compatible smoke flow

### QA Policy
Every task must include:
- implementation acceptance criteria
- at least one happy-path scenario
- at least one error/failure scenario
- evidence output under `.sisyphus/evidence/`

### Performance / Reliability Acceptance Categories
- App reaches usable shell state within a practical internal target (define in implementation as measurable telemetry)
- API run, UI replay, and stop/cancel flows must terminate predictably rather than hang indefinitely
- Failed migration or missing master key must fail clearly and block unsafe flows
- Browser cleanup and run termination must be idempotent

---

## Execution Strategy

### File Structure Map

> This repository starts blank. The following structure is the intended decomposition boundary.

```text
/
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── routes/
│   ├── components/
│   ├── hooks/
│   ├── store/
│   ├── services/
│   │   ├── tauri-client.ts
│   │   ├── api-client.ts
│   │   ├── runner-client.ts
│   │   └── browser-client.ts
│   ├── types/
│   └── lib/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── error.rs
│   │   ├── state.rs
│   │   ├── commands/
│   │   ├── services/
│   │   ├── db/
│   │   └── utils/
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
Wave 1 (foundation scaffolding — start immediately)
├── T1: Workspace bootstrap + app shell skeleton
├── T2: SQLite, migrations, app-data paths, and settings bootstrap
├── T3: Shared frontend foundations (router, stores, typed IPC client)
├── T4: Error model + domain contracts + DTO/types baseline
└── T5: Secret storage baseline + environment/data models

Wave 2 (core feature verticals)
├── T6: Environment Manager UI + commands (depends: T2, T3, T4, T5)
├── T7: Data Table Manager UI + commands (depends: T2, T3, T4, T5)
├── T8: API engine + endpoint/assertion persistence (depends: T2, T4, T5)
├── T9: API Tester UI + result viewer (depends: T3, T4, T8)
└── T10: Export + artifact path service baseline (depends: T2, T4, T5)

Wave 3 (browser risk track + runner prep)
├── T11: BrowserAutomationService + runtime health/fallback scaffolding (depends: T2, T4)
├── T12: Recorder pipeline + step normalization + persistence (depends: T2, T4, T11)
├── T13: Web Recorder / Step Editor UI (depends: T3, T4, T12)
├── T14: UI script replay executor + screenshot-on-fail (depends: T2, T4, T11, T12)
└── T15: Test case + suite runner orchestration + progress events (depends: T6, T7, T8, T10, T14)

Wave 4 (integration, packaging, hardening)
├── T16: Runner / History UI + detail panel + rerun-failed (depends: T3, T4, T15)
├── T17: Packaging, first-run bootstrap, and Windows distribution flow (depends: T1, T2, T10, T11)
├── T18: Reliability hardening, stop/cancel, degraded mode, and edge-case handling (depends: T8, T14, T15, T17)
└── T19: Smoke flows, MVP acceptance pass, and Week-6 browser viability gate report (depends: T9, T13, T16, T18)

Wave FINAL (parallel verification)
├── F1: Plan compliance audit
├── F2: Code quality review
├── F3: Real QA scenario execution
└── F4: Scope fidelity and fallback-readiness check
```

### Dependency Matrix

| Task | Blocked By | Blocks |
|---|---|---|
| T1 | None | T17 |
| T2 | None | T6, T7, T8, T10, T11, T12, T17 |
| T3 | None | T6, T7, T9, T13, T16 |
| T4 | None | T6, T7, T8, T9, T10, T11, T12, T13, T14, T16 |
| T5 | None | T6, T7, T8, T10 |
| T6 | T2, T3, T4, T5 | T15 |
| T7 | T2, T3, T4, T5 | T15 |
| T8 | T2, T4, T5 | T9, T15, T18 |
| T9 | T3, T4, T8 | T19 |
| T10 | T2, T4, T5 | T15, T17 |
| T11 | T2, T4 | T12, T14, T17 |
| T12 | T2, T4, T11 | T13, T14 |
| T13 | T3, T4, T12 | T19 |
| T14 | T2, T4, T11, T12 | T15, T18 |
| T15 | T6, T7, T8, T10, T14 | T16, T18 |
| T16 | T3, T4, T15 | T19 |
| T17 | T1, T2, T10, T11 | T18 |
| T18 | T8, T14, T15, T17 | T19 |
| T19 | T9, T13, T16, T18 | F1-F4 |

### Agent Dispatch Summary
- **Wave 1**
  - T1 → `quick`
  - T2 → `unspecified-high`
  - T3 → `quick`
  - T4 → `deep`
  - T5 → `unspecified-high`
- **Wave 2**
  - T6 → `visual-engineering`
  - T7 → `visual-engineering`
  - T8 → `deep`
  - T9 → `visual-engineering`
  - T10 → `quick`
- **Wave 3**
  - T11 → `deep`
  - T12 → `deep`
  - T13 → `visual-engineering`
  - T14 → `deep`
  - T15 → `unspecified-high`
- **Wave 4**
  - T16 → `visual-engineering`
  - T17 → `quick`
  - T18 → `unspecified-high`
  - T19 → `unspecified-high`
- **Final**
  - F1 → `oracle`
  - F2 → `unspecified-high`
  - F3 → `unspecified-high`
  - F4 → `deep`

---

## TODOs

- [x] T1. Workspace bootstrap + app shell skeleton

  **What to do**:
  - Khởi tạo workspace Tauri v2 + React + TypeScript + Vite theo cấu trúc đã khóa trong plan.
  - Tạo shell cơ bản gồm sidebar, tab bar placeholder, status bar placeholder, route skeleton và settings bootstrap tối thiểu.
  - Thiết lập build scripts, test scripts, lint/typecheck scripts và thư mục `.sisyphus/evidence/` cho verification artifacts.
  - Khóa rule vận hành: dùng `npm run build`, không dùng `npm run dev` trong verification flow cuối.

  **Must NOT do**:
  - Không thêm feature business thật vào task bootstrap.
  - Không thêm dark mode nâng cao, analytics UI, hoặc embedded browser thử nghiệm.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Chủ yếu là scaffold có ranh giới rõ, ít ambiguity nghiệp vụ.
  - **Skills**: `[]`
  - **Skills Evaluated but Omitted**:
    - `webapp-testing`: chưa cần ở bước scaffold ban đầu.

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T2, T3, T4, T5)
  - **Blocks**: T17
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:45-56` - Kiến trúc tổng quát và stack đã khóa.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:603-668` - Frontend stack, typed IPC rule, routing/client boundary.
  - `.sisyphus/plans/testforge-phase-1-implementation.md:62-84` - File structure map phải được bám theo.

  **Acceptance Criteria**:
  - [ ] Project shell khởi tạo thành công theo đúng structure map.
  - [ ] `npm run build` hoàn tất thành công.
  - [ ] App shell render được layout placeholder mà không crash.

  **QA Scenarios**:
  ```
  Scenario: App shell builds successfully
    Tool: Bash
    Preconditions: Dependencies installed
    Steps:
      1. Run `npm run build`
      2. Verify exit code is 0
      3. Save build output log
    Expected Result: Frontend build succeeds with no fatal errors
    Failure Indicators: Build exits non-zero or missing artifacts
    Evidence: .sisyphus/evidence/task-T1-build.txt

  Scenario: Shell layout renders without runtime crash
    Tool: Playwright
    Preconditions: Desktop shell or packaged preview is runnable
    Steps:
      1. Launch app shell test target
      2. Assert sidebar, top tab placeholder, and status bar are visible
      3. Capture screenshot
    Expected Result: Base shell UI renders with no blank screen
    Failure Indicators: Missing shell regions or crash dialog
    Evidence: .sisyphus/evidence/task-T1-shell.png
  ```

  **Commit**: YES
  - Message: `chore(app): bootstrap tauri shell and workspace`

- [x] T2. SQLite, migrations, app-data paths, and settings bootstrap

  **What to do**:
  - Thiết lập SQLite connection, migration runner, app-data directory rules, và filesystem path policy cho DB/logs/screenshots/exports/config.
  - Tạo migration baseline theo Appendix A và bootstrap settings/path config tối thiểu.
  - Bảo đảm migration idempotent và failure path dừng app rõ ràng.

  **Must NOT do**:
  - Không lưu screenshot blob trong DB.
  - Không hardcode path phụ thuộc máy dev.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Có cả DB schema, migration semantics, app-data policy, bootstrap behavior.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: T6, T7, T8, T10, T11, T12, T17
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:239-322` - Database rules, enums, integrity, appendix source-of-truth note.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:765-961` - Detailed SQLite schema baseline.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:644-659` - Packaging/directory expectations.

  **Acceptance Criteria**:
  - [ ] Fresh app bootstrap creates required directories and DB successfully.
  - [ ] Re-running migrations does not corrupt schema.
  - [ ] DB/artifact/config paths resolve under app data directory policy.

  **QA Scenarios**:
  ```
  Scenario: Fresh bootstrap initializes storage layout
    Tool: interactive_bash
    Preconditions: Empty app data directory
    Steps:
      1. Start the app in a clean environment
      2. Observe generated DB, logs, exports, screenshots, and config directories
      3. Save directory listing
    Expected Result: Required directories and DB file are created once with no crash
    Failure Indicators: Missing DB or missing required subdirectories
    Evidence: .sisyphus/evidence/task-T2-bootstrap.txt

  Scenario: Migration rerun is idempotent
    Tool: Bash
    Preconditions: Database already initialized
    Steps:
      1. Trigger migration runner again
      2. Verify no duplicate schema objects and no fatal migration error
    Expected Result: Migration completes safely without schema corruption
    Failure Indicators: Duplicate column/table errors or partial migration state
    Evidence: .sisyphus/evidence/task-T2-migrations.txt
  ```

  **Commit**: YES
  - Message: `feat(storage): add sqlite bootstrap and migration runner`

- [x] T3. Shared frontend foundations (router, stores, typed IPC client)

  **What to do**:
  - Thiết lập React Router routes, global stores (tabs/env/run/app), typed Tauri client layer, và shared event subscription hook.
  - Chuẩn hóa rule: frontend không gọi `invoke()` trực tiếp ngoài client layer.
  - Tạo placeholder state wiring cho active environment, runner status, và app-level layout state.

  **Must NOT do**:
  - Không đặt business logic chạy test ở frontend layer.
  - Không để route system và tab system chồng chéo không kiểm soát.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Ranh giới khá rõ nhưng cần discipline ở API client boundary.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: T6, T7, T9, T13, T16
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:648-674` - Frontend architecture principles, variable syntax, typed IPC rules.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:326-395` - Main screens and UX expectations.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:545-606` - Event payload baseline to mirror on client side.

  **Acceptance Criteria**:
  - [ ] All IPC requests route through one typed client boundary.
  - [ ] Shared stores exist for tab/env/run/app state.
  - [ ] Routes for main screens resolve without runtime errors.

  **QA Scenarios**:
  ```
  Scenario: Frontend routing works for all main screens
    Tool: Playwright
    Preconditions: App shell is runnable
    Steps:
      1. Open each main route/screen entry from the sidebar
      2. Confirm screen title/placeholder content is rendered
      3. Capture one screenshot of route navigation sequence
    Expected Result: All defined Phase-1 screens render without navigation failure
    Failure Indicators: Broken route, blank screen, or JS crash
    Evidence: .sisyphus/evidence/task-T3-routes.png

  Scenario: Direct invoke leakage is absent
    Tool: Bash
    Preconditions: Source tree available
    Steps:
      1. Search frontend code for raw `invoke(` usage outside typed client file
      2. Save results
    Expected Result: No raw invoke usage outside approved client boundary
    Failure Indicators: Any direct invoke call in feature components/hooks
    Evidence: .sisyphus/evidence/task-T3-ipc-boundary.txt
  ```

  **Commit**: YES
  - Message: `feat(frontend): add shared routing stores and typed ipc client`

- [x] T4. Error model + domain contracts + DTO/types baseline

  **What to do**:
  - Thiết lập contract types trung tâm cho commands, events, DTOs, error payloads, enums, status values, và cross-layer schemas.
  - Đồng bộ domain contracts giữa frontend và backend theo spec sections về DB enums, browser DTOs, error families, and stable event contracts.
  - Khóa naming/value semantics để tránh drift giữa các module về sau.

  **Must NOT do**:
  - Không để event/command payload tự phát theo từng feature.
  - Không để frontend suy luận UX bằng parsing raw technical string.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Đây là task cross-cutting, ảnh hưởng trực tiếp tới toàn bộ dependency graph.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: T6, T7, T8, T9, T10, T11, T12, T13, T14, T16
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:275-307` - Canonical enums and data integrity rules.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:430-606` - BrowserAutomationService DTOs, IPC commands/events, event payloads.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:672-721` - Error categories, families, persistence rule.

  **Acceptance Criteria**:
  - [ ] Shared DTO/event/error contracts are defined once and reused.
  - [ ] Command and event payloads align with the approved spec.
  - [ ] Error family taxonomy is enforceable across layers.

  **QA Scenarios**:
  ```
  Scenario: Contract typecheck catches schema drift
    Tool: Bash
    Preconditions: Shared contract files exist
    Steps:
      1. Run frontend typecheck/build and backend contract compilation/tests
      2. Save output logs
    Expected Result: Contract layer compiles cleanly with no incompatible field definitions
    Failure Indicators: Type mismatch between frontend/backend contract usage
    Evidence: .sisyphus/evidence/task-T4-contracts.txt

  Scenario: Error payload includes required fields
    Tool: Bash
    Preconditions: Contract tests or serialization tests exist
    Steps:
      1. Trigger sample error serialization path
      2. Assert payload includes `code`, `displayMessage`, `technicalMessage`, `context`, `recoverable`
    Expected Result: Error payload shape matches the spec baseline
    Failure Indicators: Missing required keys or unstable field names
    Evidence: .sisyphus/evidence/task-T4-errors.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add domain contracts and error model baseline`

- [x] T5. Secret storage baseline + environment/data models

  **What to do**:
  - Implement environment/data domain models and repositories, including encrypted secret persistence, masked previews, and key-file failure semantics.
  - Enforce app-managed encryption baseline and runtime-only secret resolution rules.
  - Wire degraded mode behavior for unreadable secret store without exposing plaintext.

  **Must NOT do**:
  - Không lưu plaintext secret vào DB, logs, exports, hoặc UI previews.
  - Không làm key bootstrap nằm chung lifecycle với data export.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Liên quan storage, security, degraded mode, và persistence invariants.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: T6, T7, T8, T10
  - **Blocked By**: None

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:605-641` - Security baseline and secret access rules.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:1149-1168` - Master key strategy, failure UX, secret lifecycle, redaction rules.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:776-789` - `environment_variables` schema constraints.

  **Acceptance Criteria**:
  - [ ] Secret values persist only as encrypted values + masked preview.
  - [ ] Missing/corrupt master key blocks secret-dependent flows with clear degraded-mode handling.
  - [ ] Environment/data repositories align with schema constraints.

  **QA Scenarios**:
  ```
  Scenario: Secret is masked and encrypted at rest
    Tool: Bash
    Preconditions: Environment with secret variable created
    Steps:
      1. Insert/update a secret variable through the app path
      2. Inspect persisted record storage representation
      3. Confirm plaintext value is absent and masked preview exists
    Expected Result: Only encrypted value + preview are persisted
    Failure Indicators: Plaintext secret found in DB or logs
    Evidence: .sisyphus/evidence/task-T5-secret-storage.txt

  Scenario: Corrupt master key triggers degraded mode
    Tool: interactive_bash
    Preconditions: Existing encrypted secrets and simulated corrupted key bootstrap
    Steps:
      1. Launch app with corrupted/missing key state
      2. Observe startup behavior and blocked secret-dependent flows
      3. Capture dialog/output evidence
    Expected Result: App reports blocking error clearly and prevents unsafe secret resolution
    Failure Indicators: Silent failure, plaintext fallback, or crash loop
    Evidence: .sisyphus/evidence/task-T5-key-failure.txt
  ```

  **Commit**: YES
  - Message: `feat(security): add encrypted secret storage baseline`

- [x] T6. Environment Manager UI + commands

  **What to do**:
  - Xây Environment Manager screen với CRUD environments, variable list, masking UI, default environment selection, và production warning badge behavior.
  - Kết nối typed IPC commands để load/save/update/delete environments và variables.
  - Hiển thị degraded-mode cues khi secret store gặp lỗi.

  **Must NOT do**:
  - Không reveal full secret mặc định sau khi lưu.
  - Không cho phép destructive actions mà không confirm.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Chủ yếu là management UI nhưng có nhiều trạng thái UX nhạy cảm.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: T15
  - **Blocked By**: T2, T3, T4, T5

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:333-395` - Main screens and UX principles.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:612-641` - Security/privacy requirements for masking, production warning, screenshots policy.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:767-789` - Environments and environment variables schema.

  **Acceptance Criteria**:
  - [ ] QA can create/update/delete environments and variables.
  - [ ] Secret variables are masked by default.
  - [ ] Production-like environment shows clear risk warning.

  **QA Scenarios**:
  ```
  Scenario: Create environment with secret and non-secret variables
    Tool: Playwright
    Preconditions: App running with initialized storage
    Steps:
      1. Open Environment Manager
      2. Create environment `Staging`
      3. Add variable `base_url` and secret variable `api_token`
      4. Save and reopen environment detail
    Expected Result: Non-secret value visible, secret masked, save successful
    Failure Indicators: Secret displayed in plaintext or save fails silently
    Evidence: .sisyphus/evidence/task-T6-env-manager.png

  Scenario: Production warning displays correctly
    Tool: Playwright
    Preconditions: Environment marked as production exists
    Steps:
      1. Open production environment
      2. Observe warning badge/banner
      3. Attempt destructive action and verify extra confirmation
    Expected Result: Production risk is visually clear and destructive action needs confirmation
    Failure Indicators: Missing warning or no extra confirmation
    Evidence: .sisyphus/evidence/task-T6-production-warning.png
  ```

  **Commit**: YES
  - Message: `feat(env): add environment manager ui and commands`

- [x] T7. Data Table Manager UI + commands

  **What to do**:
  - Implement data table CRUD, row editing, enabled/disabled row semantics, and import/export baseline for CSV/JSON.
  - Expose association-ready metadata for test cases without adding advanced spreadsheet features.
  - Handle empty table, zero enabled rows, and malformed import validation paths clearly.

  **Must NOT do**:
  - Không thêm spreadsheet engine quá phức tạp.
  - Không thêm formula, cross-row references, hoặc auto-mapping thông minh ngoài baseline.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Data-management UI cần trạng thái rõ ràng và UX an toàn.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: T15
  - **Blocked By**: T2, T3, T4, T5

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:176-186` - Shared foundation scope.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:665-674` - Data table association rule.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:856-874` - Data tables and rows schema.

  **Acceptance Criteria**:
  - [ ] QA can create/edit/delete data tables and rows.
  - [ ] Disabled rows do not execute in data-driven runs.
  - [ ] Invalid import is rejected with actionable error.

  **QA Scenarios**:
  ```
  Scenario: Create data table with enabled and disabled rows
    Tool: Playwright
    Preconditions: App running
    Steps:
      1. Open Data Manager
      2. Create table `login-users`
      3. Add 3 rows and disable 1 row
      4. Save and reload table
    Expected Result: Row state persists and disabled row is visually distinct
    Failure Indicators: Row state lost or disabled row indistinguishable
    Evidence: .sisyphus/evidence/task-T7-data-table.png

  Scenario: Reject malformed CSV import
    Tool: Playwright
    Preconditions: Invalid CSV fixture available
    Steps:
      1. Trigger import flow with malformed file
      2. Observe validation result
    Expected Result: Import blocked with clear error; no partial table corruption
    Failure Indicators: Silent partial import or app crash
    Evidence: .sisyphus/evidence/task-T7-invalid-import.png
  ```

  **Commit**: YES
  - Message: `feat(data): add data table manager and validation flow`

- [x] T8. API engine + endpoint/assertion persistence

  **What to do**:
  - Implement API collection/endpoint/assertion persistence, variable resolution, auth handling, response normalization, and assertion evaluation engine.
  - Handle supported auth types only, response preview truncation, and transport-vs-assertion failure separation.
  - Enforce operator set and error-code persistence policy.

  **Must NOT do**:
  - Không thêm GraphQL, WebSocket, pre/post-request scripts, response chaining automation.
  - Không lưu raw secret/auth header trong persisted previews.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Đây là business logic lõi cho API testing, gồm nhiều invariants và edge cases.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: T9, T15, T18
  - **Blocked By**: T2, T4, T5

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:161-169` - API feature scope.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:275-288` - API enums and operators.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:553-601` - Testing requirements for assertions/API engine.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:798-829` - `api_endpoints` + `assertions` schema.

  **Acceptance Criteria**:
  - [ ] API endpoints and assertions persist and reload correctly.
  - [ ] Variable resolution works, including missing-variable failure path.
  - [ ] Supported auth types run correctly and redact sensitive previews.
  - [ ] Assertion engine distinguishes transport error from business/test failure.

  **QA Scenarios**:
  ```
  Scenario: Run API test with assertion success
    Tool: Bash (curl-equivalent app command or integration harness)
    Preconditions: Mock/test API endpoint available, environment configured
    Steps:
      1. Create endpoint with GET request and status/body assertions
      2. Execute the API test
      3. Inspect saved run result preview
    Expected Result: Test passes, assertion summary is correct, result persisted
    Failure Indicators: Incorrect status classification or missing result record
    Evidence: .sisyphus/evidence/task-T8-api-pass.txt

  Scenario: Missing variable fails before request dispatch
    Tool: Bash
    Preconditions: Endpoint references undefined `{{missing_token}}`
    Steps:
      1. Execute the API test
      2. Inspect error payload and network side effects
    Expected Result: Validation-style failure occurs before outbound request, clear error code persisted
    Failure Indicators: Request sent anyway or vague error output
    Evidence: .sisyphus/evidence/task-T8-missing-variable.txt
  ```

  **Commit**: YES
  - Message: `feat(api): implement endpoint execution and assertions`

- [x] T9. API Tester UI + result viewer

  **What to do**:
  - Build API Tester screen: collection tree, request builder, auth/body/headers tabs, assertion builder UI, response viewer, and result state panels.
  - Integrate with API persistence/execution layer and display actual-vs-expected details.
  - Include error/empty/loading/data states and redacted previews.

  **Must NOT do**:
  - Không thêm advanced scripting tabs hoặc collection-runner ngoài approved scope.
  - Không hiển thị raw secrets trong request preview.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI-heavy screen with form complexity and state display requirements.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: T19
  - **Blocked By**: T3, T4, T8

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:303-316` - API Tester screen layout and UX.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:387-395` - Global UX patterns and reporting placement.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:555-591` - Testing/coverage requirements for assertions.

  **Acceptance Criteria**:
  - [ ] QA can create/edit endpoint definitions from UI.
  - [ ] Running test shows response preview, assertion summary, and failure detail.
  - [ ] Loading/empty/error/data states are explicit and understandable.

  **QA Scenarios**:
  ```
  Scenario: Create and run API endpoint from UI
    Tool: Playwright
    Preconditions: App running with mock API target available
    Steps:
      1. Open API Tester
      2. Create collection and endpoint
      3. Add one passing assertion and run test
      4. Verify response viewer and assertion panel content
    Expected Result: End-to-end UI flow works without leaving the screen
    Failure Indicators: Missing result sections or unusable error state
    Evidence: .sisyphus/evidence/task-T9-api-tester.png

  Scenario: Assertion failure shows actual vs expected clearly
    Tool: Playwright
    Preconditions: Endpoint configured with intentionally failing assertion
    Steps:
      1. Run failing API test
      2. Inspect failure detail panel
    Expected Result: UI distinguishes assertion fail from transport error and shows actual vs expected
    Failure Indicators: Ambiguous failure messaging or missing comparison detail
    Evidence: .sisyphus/evidence/task-T9-assertion-fail.png
  ```

  **Commit**: YES
  - Message: `feat(api-ui): add api tester and result viewer`

- [x] T10. Export + artifact path service baseline

  **What to do**:
  - Implement artifact path resolution, sanitized report export (HTML/JSON), screenshot/export directory policy, and preview-safe persistence helpers.
  - Ensure export excludes encrypted secrets and raw sensitive values by default.
  - Provide reusable service for artifact manifest handling across API/UI/suite runs.

  **Must NOT do**:
  - Không embed screenshot blob vào DB.
  - Không export ciphertext fields hoặc raw auth credentials.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Service-focused, bounded concern, mostly deterministic behavior.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: T15, T17
  - **Blocked By**: T2, T4, T5

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:112-124` - Basic reporting scope.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:612-640` - Security baseline, redaction expectations, screenshot policy.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:930-948` - `test_run_results` artifact-related schema fields.

  **Acceptance Criteria**:
  - [ ] Exported reports contain sanitized summaries and artifact links only.
  - [ ] Artifact directories are created and reused safely.
  - [ ] Export failure surfaces a clear recoverable error.

  **QA Scenarios**:
  ```
  Scenario: Export run report without secret leakage
    Tool: Bash
    Preconditions: At least one run result with auth/secret-backed request exists
    Steps:
      1. Trigger HTML/JSON export
      2. Inspect generated files for presence of secret values
    Expected Result: Export succeeds and secret values are redacted/absent
    Failure Indicators: Plaintext credentials or encrypted secret blobs in export
    Evidence: .sisyphus/evidence/task-T10-export.txt

  Scenario: Missing export path is handled safely
    Tool: interactive_bash
    Preconditions: Invalid or removed export target path
    Steps:
      1. Trigger export into invalid path condition
      2. Observe fallback/error behavior
    Expected Result: Clear recoverable error or safe directory creation path
    Failure Indicators: Crash or silent export loss
    Evidence: .sisyphus/evidence/task-T10-export-error.txt
  ```

  **Commit**: YES
  - Message: `feat(reporting): add export and artifact path services`

- [x] T11. BrowserAutomationService + runtime health/fallback scaffolding

  **What to do**:
  - Build the browser abstraction boundary, runtime health check, Chromium-only launcher, and fallback-ready scaffolding without leaking library internals.
  - Implement stable DTO mapping and event emission foundation.
  - Add explicit viability checks that support the Week-6 go/no-go gate.

  **Must NOT do**:
  - Không expose raw playwright-rs page/context objects outside browser layer.
  - Không thêm support cho Firefox/WebKit.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Highest-risk abstraction boundary, directly tied to fallback plan.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: T12, T14, T17
  - **Blocked By**: T2, T4

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:419-506` - Browser abstraction contract and DTO baseline.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:721-739` - Fallback preparation expectations.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:1031-1063` - Appendix D requirements.

  **Acceptance Criteria**:
  - [ ] Browser service exposes only spec-approved DTOs.
  - [ ] Runtime health check reports readiness/diagnostics.
  - [ ] Chromium-only bootstrap path works or fails clearly.

  **QA Scenarios**:
  ```
  Scenario: Browser runtime health check reports ready
    Tool: Bash
    Preconditions: Chromium runtime available
    Steps:
      1. Invoke browser runtime health check
      2. Capture readiness payload
    Expected Result: `ready=true` with diagnostic context
    Failure Indicators: Unstructured error or missing status fields
    Evidence: .sisyphus/evidence/task-T11-health.txt

  Scenario: Browser runtime unavailable fails clearly
    Tool: interactive_bash
    Preconditions: Simulate missing browser/runtime
    Steps:
      1. Invoke health/start path without runtime availability
      2. Observe error/event output
    Expected Result: Clear browser failure without crashing non-browser features
    Failure Indicators: App-wide crash or untyped error
    Evidence: .sisyphus/evidence/task-T11-runtime-fail.txt
  ```

  **Commit**: YES
  - Message: `feat(browser): add browser automation abstraction and health checks`

- [x] T12. Recorder pipeline + step normalization + persistence

  **What to do**:
  - Implement start/stop/cancel recording flow, session state machine, step capture buffering, normalization, confidence scoring, persistence, and failure recovery.
  - Preserve partial captured steps on browser/session failure where allowed.
  - Enforce one-recording-at-a-time rule.

  **Must NOT do**:
  - Không thêm unsupported step types vào Phase 1 step model.
  - Không giả vờ support multi-tab/loop/assertion auto-generation.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: High-risk runtime flow with multiple failure and persistence states.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: T13, T14
  - **Blocked By**: T2, T4, T11

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:341-384` - Browser recording user flow, window rules, state machine, step editor needs.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:1172-1212` - Appendix C recording UX flow and confidence scoring baseline.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:840-855` - `ui_script_steps` schema.

  **Acceptance Criteria**:
  - [ ] Recording session starts, streams steps, and persists normalized draft on stop.
  - [ ] Confidence scoring marks high/medium/low per baseline.
  - [ ] Browser close/crash preserves partial work where possible and returns recoverable state.

  **QA Scenarios**:
  ```
  Scenario: Record simple login-like flow and persist draft
    Tool: Playwright
    Preconditions: Recordable demo target available
    Steps:
      1. Start recording from Web Recorder screen
      2. Perform navigate + click + fill actions
      3. Stop recording
      4. Inspect resulting step draft
    Expected Result: Ordered steps persisted with confidence labels and editable fields
    Failure Indicators: Missing step stream, unordered steps, or no persisted draft
    Evidence: .sisyphus/evidence/task-T12-recording.png

  Scenario: Browser closes unexpectedly mid-recording
    Tool: Playwright
    Preconditions: Active recording session
    Steps:
      1. Start recording
      2. Close browser window unexpectedly
      3. Observe recovery state in app shell
    Expected Result: Session marked failed/stopped, partial steps retained if available, recoverable message shown
    Failure Indicators: Shell crash, lost state without message, or stuck session
    Evidence: .sisyphus/evidence/task-T12-recording-fail.png
  ```

  **Commit**: YES
  - Message: `feat(recorder): add recording pipeline and step persistence`

- [x] T13. Web Recorder / Step Editor UI

  **What to do**:
  - Build Web Recorder UI with preflight state, session status, realtime step stream, step editor controls, and low-confidence highlighting.
  - Provide editing for selector, value, timeout, add/delete/reorder basic steps.
  - Integrate run/record conflict states visually.

  **Must NOT do**:
  - Không attempt embedded live browser panel.
  - Không thêm complex scripting/branching editor.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UX-heavy screen with stateful interaction design.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: T19
  - **Blocked By**: T3, T4, T12

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:317-332` - Web UI Recorder screen layout.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:341-384` - Recorder flow and step editor rules.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:997-1027` - Appendix C UX emphasis.

  **Acceptance Criteria**:
  - [ ] UI exposes preflight, recording state, stop, and editable step list.
  - [ ] Low-confidence steps are clearly highlighted.
  - [ ] Conflict states (recording vs replay) are visible and action-safe.

  **QA Scenarios**:
  ```
  Scenario: Edit recorded step in step editor
    Tool: Playwright
    Preconditions: Existing recorded script draft available
    Steps:
      1. Open recorded draft
      2. Change selector and timeout for a low-confidence step
      3. Save changes
    Expected Result: Step updates persist and warning indicator remains understandable
    Failure Indicators: Edited values not persisted or low-confidence state hidden
    Evidence: .sisyphus/evidence/task-T13-step-editor.png

  Scenario: Attempt conflicting action during recording
    Tool: Playwright
    Preconditions: Active recording session
    Steps:
      1. Start recording
      2. Try triggering suite run or replay action from UI
    Expected Result: Conflicting action is blocked with clear explanation
    Failure Indicators: Conflicting action proceeds or UI becomes inconsistent
    Evidence: .sisyphus/evidence/task-T13-conflict.png
  ```

  **Commit**: YES
  - Message: `feat(recorder-ui): add web recorder and step editor ui`

- [x] T14. UI script replay executor + screenshot-on-fail

  **What to do**:
  - Execute saved UI steps through browser abstraction, emit step results, capture screenshots on fail, and persist artifact manifests.
  - Respect timeout, stop/cancel, and step-level failure semantics.
  - Keep execution Chromium-only and sequential.

  **Must NOT do**:
  - Không thêm fallback selector engine phức tạp ngoài approved scope.
  - Không swallow browser errors thành false positive pass.

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Runtime-critical execution path with artifacts and error semantics.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: T15, T18
  - **Blocked By**: T2, T4, T11, T12

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:501-548` - Playwright/browser responsibilities and concurrency expectations.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:638-640` - Screenshot policy.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:490-505` - `ScriptRunStepResult`, `ArtifactManifest`, runtime DTOs.

  **Acceptance Criteria**:
  - [ ] Saved UI script can replay with step result emission.
  - [ ] Failures create screenshot artifact paths and clear error payloads.
  - [ ] Stop/cancel semantics are idempotent.

  **QA Scenarios**:
  ```
  Scenario: Replay simple script successfully
    Tool: Playwright
    Preconditions: Saved simple UI script exists
    Steps:
      1. Trigger script replay
      2. Observe per-step progress and completion status
    Expected Result: Script completes with passed step results and no orphaned browser session
    Failure Indicators: Missing progress, hung browser, or incomplete result persistence
    Evidence: .sisyphus/evidence/task-T14-replay-pass.png

  Scenario: Failing step captures screenshot and error detail
    Tool: Playwright
    Preconditions: Script contains intentionally invalid selector step
    Steps:
      1. Run replay
      2. Observe failure handling and screenshot evidence
    Expected Result: Failure result includes error code, screenshot path, and halted/failed step status
    Failure Indicators: No screenshot, no artifact path, or ambiguous failure state
    Evidence: .sisyphus/evidence/task-T14-replay-fail.png
  ```

  **Current QA Status**:
  - `npm run test:t14:smoke` => `SMOKE_BLOCKED`
  - Blocker: Chromium executable not present in current environment
  - Candidate paths checked:
    - `D:\my\research\ms-playwright\chromium\chrome-win\chrome.exe`
    - `D:\my\research\src-tauri\ms-playwright\chromium\chrome-win\chrome.exe`

  **Commit**: YES
  - Message: `feat(ui-runner): add ui replay execution and screenshots`

- [x] T15. Test case + suite runner orchestration + progress events

  **What to do**:
  - Implement test case abstraction handling, suite composition, mixed execution orchestration, rerun-failed support, progress events, and result persistence.
  - Respect concurrency model: API parallel limit 4, UI sequential, no recording/replay conflicts.
  - Include empty suite, disabled rows, deleted reference, and stop/run conflict handling.

  **Must NOT do**:
  - Không cho mixed suite chạy UI cases song song.
  - Không ignore partial failures hoặc hidden skipped states.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Orchestration layer gom nhiều dependency và business rules.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: T16, T18
  - **Blocked By**: T6, T7, T8, T10, T14

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:176-186` - Test management scope.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:513-602` - Concurrency model, IPC/events, testing expectations.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:903-948` - `suite_cases`, `test_runs`, `test_run_results` schema.

  **Acceptance Criteria**:
  - [ ] Mixed suites run with correct concurrency semantics.
  - [ ] Disabled data rows are skipped from expansion.
  - [ ] Rerun-failed flow only targets failed cases.
  - [ ] Progress events and persisted counts stay consistent.

  **QA Scenarios**:
  ```
  Scenario: Run mixed suite with API and UI cases
    Tool: Bash + Playwright
    Preconditions: One API case and one UI case exist in suite
    Steps:
      1. Trigger suite run
      2. Observe progress events and final summary counts
      3. Inspect persisted run history
    Expected Result: API and UI cases execute under correct orchestration and summary is accurate
    Failure Indicators: Wrong counts, missing run history, or UI/API concurrency violation
    Evidence: .sisyphus/evidence/task-T15-suite-run.txt

  Scenario: Empty or invalid suite is blocked safely
    Tool: Playwright
    Preconditions: Empty suite or suite with broken reference exists
    Steps:
      1. Attempt to run the invalid suite
      2. Observe validation/result behavior
    Expected Result: Run blocked with actionable error; no corrupt run record
    Failure Indicators: Silent no-op, crash, or invalid run persisted
    Evidence: .sisyphus/evidence/task-T15-invalid-suite.png
  ```

  **Commit**: YES
  - Message: `feat(runner): add suite orchestration and progress events`

- [x] T16. Runner / History UI + detail panel + rerun-failed

  **What to do**:
  - Build Runner screen, run history list, detail panel, progress bar/counters, and rerun-failed UX.
  - Surface per-case/per-row results, artifact links, sanitized previews, and failure categories.
  - Keep reporting scope basic and embedded in runner/history views.

  **Must NOT do**:
  - Không tạo analytics dashboard nâng cao hoặc route báo cáo riêng phức tạp nếu không cần.
  - Không show unredacted request/response secrets.

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI tổng hợp nhiều trạng thái real-time và chi tiết failure.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: T19
  - **Blocked By**: T3, T4, T15

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:333-345` - Test Runner screen layout.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:393-395` - Reporting UX embedded in history/detail views.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:742-752` - MVP exit criteria for readable pass/fail detail.

  **Acceptance Criteria**:
  - [ ] Run history and detail views render persisted runs correctly.
  - [ ] Rerun-failed only targets failed cases from selected run.
  - [ ] Details include screenshots/artifacts and sanitized previews.

  **QA Scenarios**:
  ```
  Scenario: Inspect completed run history and detail panel
    Tool: Playwright
    Preconditions: At least one suite run exists
    Steps:
      1. Open Runner/History screen
      2. Select a historical run
      3. Inspect summary counts and one failure detail
    Expected Result: History list, detail panel, and artifact links are visible and understandable
    Failure Indicators: Missing run detail, inconsistent counts, or broken artifact navigation
    Evidence: .sisyphus/evidence/task-T16-history.png

  Scenario: Rerun failed cases only
    Tool: Playwright
    Preconditions: Existing run with at least one failed case
    Steps:
      1. Trigger `Rerun Failed`
      2. Observe resulting execution scope and new run summary
    Expected Result: Only previously failed cases rerun
    Failure Indicators: Passed cases rerun unnecessarily or wrong cases omitted
    Evidence: .sisyphus/evidence/task-T16-rerun-failed.png
  ```

  **Commit**: YES
  - Message: `feat(runner-ui): add history detail and rerun-failed ui`

- [x] T17. Packaging, first-run bootstrap, and Windows distribution flow

  **What to do**:
  - Prepare Windows-first packaging flow, first-run initialization, runtime verification, version display, and manual-update-safe directory separation.
  - Ensure installer/first-run path initializes DB/runtime/config and reports actionable guidance on missing browser/runtime.
  - Capture package/runtime assumptions clearly for internal distribution.

  **Must NOT do**:
  - Không thêm auto-update infrastructure cho MVP.
  - Không assume QA must install runtime manually through dev commands if avoidable.

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Bounded release-prep flow with clear acceptance checks.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: T18
  - **Blocked By**: T1, T2, T10, T11

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:644-659` - Packaging direction and acceptance criteria.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:848-890` - Timeline release polish expectations that shape packaging/readiness sequencing.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:848-890` - Timeline release polish expectations.

  **Acceptance Criteria**:
  - [ ] Windows-first package can initialize app-data storage on first run.
  - [ ] Version is surfaced in app UI.
  - [ ] Missing runtime/browser path fails clearly without corrupting user data.

  **QA Scenarios**:
  ```
  Scenario: First-run bootstrap from packaged app
    Tool: interactive_bash
    Preconditions: Packaged Windows build available and no prior app data
    Steps:
      1. Launch packaged app in clean user profile/path state
      2. Observe first-run initialization
      3. Verify data directories and version display
    Expected Result: First run completes successfully and user data path is initialized
    Failure Indicators: Startup crash, missing directories, or missing version info
    Evidence: .sisyphus/evidence/task-T17-first-run.txt

  Scenario: Browser runtime missing in packaged flow
    Tool: interactive_bash
    Preconditions: Simulated missing runtime/browser condition
    Steps:
      1. Start packaged app under missing-runtime condition
      2. Attempt browser-related action
    Expected Result: Actionable guidance displayed; API features remain usable
    Failure Indicators: App-wide failure or non-actionable error
    Evidence: .sisyphus/evidence/task-T17-runtime-missing.txt
  ```

  **Commit**: YES
  - Message: `chore(release): add packaging and first-run bootstrap`

- [x] T18. Reliability hardening, stop/cancel, degraded mode, and edge-case handling

  **What to do**:
  - Harden stop/cancel semantics, degraded-mode handling, missing-variable errors, oversized response preview policy, browser/session loss handling, empty suite handling, deleted-reference handling, and corrupted key failure path.
  - Enforce that browser failures do not block API-only features.
  - Add operational safeguards for long-running tasks and cleanup.

  **Must NOT do**:
  - Không che giấu lỗi bằng generic messages vô dụng.
  - Không để cancel/stop gây orphaned state hoặc duplicate completion records.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cutting hardening across multiple runtime paths and error surfaces.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: T19
  - **Blocked By**: T8, T14, T15, T17

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:672-721` - Error handling rules and persistence semantics.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:619-636` - Secret access and degraded mode behavior.
  - Metis findings summarized in this plan under Context/Guardrails - mandatory edge cases to include.

  **Acceptance Criteria**:
  - [ ] Stop/cancel actions are idempotent across API/UI/suite flows.
  - [ ] Browser failure leaves API features usable.
  - [ ] Oversized response preview is truncated/sanitized instead of breaking persistence.
  - [ ] Missing key / missing variable / empty suite / invalid reference all fail clearly and safely.

  **QA Scenarios**:
  ```
  Scenario: Stop action is idempotent during active run
    Tool: Playwright
    Preconditions: Active suite run in progress
    Steps:
      1. Click Stop once
      2. Click Stop again rapidly
      3. Inspect final run state
    Expected Result: Run stops once cleanly without duplicate error or corrupted status
    Failure Indicators: Double-stop crash, duplicated results, or stuck running state
    Evidence: .sisyphus/evidence/task-T18-stop-idempotent.png

  Scenario: Browser feature failure preserves API usability
    Tool: Playwright + Bash
    Preconditions: Browser runtime intentionally unavailable, API feature available
    Steps:
      1. Trigger browser-related action and observe failure
      2. Immediately run a valid API test
    Expected Result: Browser flow fails clearly, API test still runs successfully
    Failure Indicators: Global app degradation blocks API path
    Evidence: .sisyphus/evidence/task-T18-degraded-mode.txt
  ```

  **Commit**: YES
  - Message: `fix(core): harden degraded mode and stop cancel flows`

- [x] T19. Smoke flows, MVP acceptance pass, and Week-6 browser viability gate report

  **What to do**:
  - Execute the approved smoke set end-to-end, capture evidence, and produce a browser viability checkpoint report based on implemented flows.
  - Validate MVP exit criteria against the real product behavior.
  - Record whether browser track remains on primary path or must trigger fallback.

  **Must NOT do**:
  - Không mark MVP ready khi smoke set hoặc browser gate chưa đủ bằng chứng.
  - Không bỏ qua failure evidence để “giữ timeline”.

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Đây là acceptance/hardening task tổng hợp với evidence discipline cao.
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4
  - **Blocks**: F1-F4
  - **Blocked By**: T9, T13, T16, T18

  **References**:
  - `.sisyphus/drafts/automation-testing-tool-spec.md:576-601` - Minimum smoke set and testing philosophy.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:742-752` - MVP exit criteria.
  - `.sisyphus/drafts/automation-testing-tool-spec.md:721-739` - Fallback trigger logic and architectural preparation.

  **Acceptance Criteria**:
  - [ ] All minimum smoke scenarios execute with evidence.
  - [ ] MVP exit criteria are evaluated explicitly.
  - [ ] Browser viability gate result is documented with pass/fail justification.

  **QA Scenarios**:
  ```
  Scenario: Execute full minimum smoke set
    Tool: Playwright + Bash + interactive_bash
    Preconditions: All core features implemented
    Steps:
      1. Run create environment scenario
      2. Run API pass scenario
      3. Run API fail scenario
      4. Run record simple UI flow scenario
      5. Run replay simple UI flow scenario
      6. Run mixed suite scenario
      7. Run failure detail/screenshot scenario
    Expected Result: Smoke set produces complete evidence and clear pass/fail summary
    Failure Indicators: Missing evidence, unrepeatable flows, or unclear failures
    Evidence: .sisyphus/evidence/task-T19-smoke-summary.txt

  Scenario: Browser viability gate decision
    Tool: Bash
    Preconditions: Browser-related smoke evidence collected
    Steps:
      1. Evaluate recording usability
      2. Evaluate replay stability
      3. Evaluate screenshot-on-fail usability
      4. Write gate verdict
    Expected Result: Explicit PASS/FAIL checkpoint with rationale and fallback recommendation if needed
    Failure Indicators: No gate verdict or evidence-free optimism
    Evidence: .sisyphus/evidence/task-T19-browser-gate.txt
  ```

  **Commit**: YES
  - Message: `test(mvp): run smoke acceptance and browser gate review`

---

## Final Verification Wave

- [x] F1. **Plan Compliance Audit** — `oracle`
  - Verify each locked MVP decision from the spec is preserved.
  - Verify no forbidden Phase-1 features appear in final code paths.
  - Confirm artifacts/evidence exist for all required QA scenarios.

- [x] F2. **Code Quality Review** — `unspecified-high`
  - Run full type/lint/test/build verification.
  - Review for direct IPC leakage, secret logging, unstable abstractions, and over-scoped features.

- [x] F3. **Real QA Scenario Execution** — `unspecified-high`
  - Execute end-to-end scenarios for environment creation, API test, UI record/replay, suite run, and degraded-mode failure.
  - Save final evidence under `.sisyphus/evidence/final-qa/`.

- [x] F4. **Scope Fidelity and Fallback Readiness Check** — `deep`
  - Verify browser abstraction remains isolated and fallback-ready.
  - Confirm Week-6 browser viability gate can be assessed from implemented evidence.

---

## Commit Strategy

- Commit after each task or tightly related pair of tasks in the same wave.
- Use conventional messages, for example:
  - `chore(app): bootstrap tauri shell and workspace`
  - `feat(env): add encrypted environment storage`
  - `feat(api): implement endpoint execution and assertions`
  - `feat(ui-recorder): add recording pipeline and step editor`
  - `feat(runner): add suite orchestration and history`
  - `chore(release): add packaging and first-run bootstrap`

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
- [ ] All Must Have requirements are implemented
- [ ] All Must NOT Have constraints remain absent
- [ ] Browser abstraction and IPC contracts remain stable
- [ ] Secret storage and redaction policies are enforced
- [ ] Smoke scenarios pass with evidence
- [ ] Packaging works for Windows-first internal distribution
