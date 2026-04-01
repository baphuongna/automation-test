- Replaced legacy broken src/types/index.ts with a barrel-export file to stabilize typecheck boundary and keep T4 contract modules authoritative.

## T4 rerun decisions (2026-03-30)

- Kept command/event payload naming unchanged from existing shared baseline because both `src/types/*` and `src-tauri/src/contracts/*` are already aligned and no contradictory canonical file exists at the expected draft path.
- Expanded shared error contract taxonomy by adding `INTERNAL` family and `INTERNAL_UNEXPECTED_ERROR` in both TS and Rust contract layers to make cross-layer classification exhaustive for fallback/unexpected failures.
- Added `isErrorCode()` type guard in `src/types/errors.ts` to prevent ad-hoc raw-string parsing and keep frontend error handling anchored to shared contract codes.

## T4 rerun decisions: cross-layer payload consistency (2026-03-30)

- Chosen canonical shape for `environment.variable.upsert`: keep the existing TypeScript nested payload (`environmentId` + `variable { id, key, kind, value }`) and align Rust contract to the same structure.
- Enforced envelope consistency for empty-payload commands by introducing `EmptyCommandPayload` in Rust and applying it to `environment.list` command variant.
- Tightened event error contract by introducing `AppErrorScope` enum in Rust and replacing free-form `String` scope to match frontend constrained union semantics.

## T1: Workspace Bootstrap + App Shell Skeleton (2026-03-30)

### Key Technical Decisions

#### 1. Keep T1 Shell Strictly Placeholder-Only
**Decision**: Implement sidebar, tab bar, status bar, and route screens as static placeholders without business logic or typed IPC usage.

**Rationale**:
- Matches the T1 acceptance criteria exactly
- Avoids scope creep into T3/T4/T6+ concerns
- Keeps the shell buildable while the rest of the product evolves

#### 2. Use Static CSS Instead of Broken Tailwind Scaffold
**Decision**: Replace invalid Tailwind placeholder directives in `src/index.css` with plain CSS dedicated to the shell layout.

**Rationale**:
- Restores build stability immediately
- Keeps visual verification simple for shell-only acceptance
- Avoids introducing extra Tailwind config work outside T1 scope

#### 3. Add a Minimal Shell Smoke Test Script
**Decision**: Add `npm test` backed by `tests/frontend/shell-smoke.test.ts` using `tsx`.

**Rationale**:
- Satisfies T1's expectation for a basic verification script
- Protects required shell composition without introducing heavy test infrastructure
- Keeps the test fast and deterministic for repeated bootstrap checks

#### 4. Use an Inline SVG Favicon for Shell Verification Cleanup
**Decision**: Resolve the built-preview `/favicon.ico` 404 by adding a `rel="icon"` data URI directly in `index.html`.

**Rationale**:
- Changes only one shell entry file
- Avoids adding binary assets or public directory complexity for a placeholder shell
- Stops favicon-related console/network noise during T1/T3 verification

## T5: Secret Storage Baseline (2026-03-30)

### Key Technical Decisions

#### 1. AES-256-GCM for Secret Encryption
**Decision**: Use AES-256-GCM for secret encryption.

**Rationale**:
- Authenticated encryption (integrity + confidentiality)
- Security audited by NCC Group
- Hardware acceleration support (AES-NI)
- Industry standard for secret storage

**Alternatives Considered**:
- AES-CBC: No authentication, vulnerable to padding oracle attacks
- ChaCha20-Poly1305: Good but GCM is more widely supported

#### 2. Random Nonce Per Secret
**Decision**: Generate unique 12-byte random nonce for each encryption operation.

**Rationale**:
- Prevents nonce reuse attacks
- Each secret has unique ciphertext even with same plaintext
- Nonce prepended to ciphertext for storage

#### 3. Masked Preview Pattern: `ab***yz`
**Decision**: Show first 2 and last 2 characters with `***` in between.

**Examples**:
- `"password123"` → `"pa***23"`
- `"abc"` → `"a*c"`
- `"x"` → `"*"`

**Rationale**:
- Gives user confirmation without revealing secret
- Consistent with spec requirement: "Masked previews only, never full secrets"

#### 4. Degraded Mode: Block Secret Operations
**Decision**: Block all secret-dependent operations in degraded mode with clear error.

**Blocked Operations**:
- Encrypting new secrets
- Decrypting existing secrets
- Key rotation

**Rationale**:
- Fail fast and fail clearly (per spec)
- Prevents unsafe flows
- User gets actionable error message

#### 5. Master Key Storage
**Decision**: Store master key in file at `app_data_dir/master.key`.

**Security Measures**:
- 32-byte random key generated on first run
- File permissions set to 0600 on Unix
- Key wrapped in `Zeroizing<[u8; 32]>` for memory safety

**Future Enhancement**: OS keychain integration (optional per spec)

## T5 rerun decisions (2026-03-30)

- Kept degraded-mode semantics inside the storage layer by allowing `Database::new(...)` to survive `MasterKeyCorrupted` while secret operations remain blocked by `SecretService`.
- Rejected plaintext fallback completely; repository writes for `VariableType::Secret` now validate encrypted-looking storage shape plus masked preview before persistence.
- Tightened SQLite schema constraints for T5-owned tables to better match the approved acceptance criteria without expanding into UI or downstream feature work.
- Standardized T3 foundations around four isolated Zustand stores (`tabs`, `env`, `run`, `app`) so route resolution stays independent from tab lifecycle.
- Kept all frontend IPC requests behind invokeCommand in src/services/tauri-client.ts; hooks and route placeholders do not call Tauri directly.
## T2: Storage Bootstrap (2026-03-30)
- Chosen architecture: Database bootstrap now always runs file-based SQL migrations from src-tauri/migrations instead of maintaining a second inline schema path.
- Path/bootstrap policy: AppPaths owns creation of db/logs/screenshots/exports/config directories and the default settings.json under the resolved app-data root.
- Failure policy: missing migrations directory or checksum drift returns explicit migration errors rather than silently continuing.

## T2 follow-up decisions: migration tracking consistency (2026-03-31)

- Removed the manual `_migrations` insert from `src-tauri/migrations/001_initial_schema.sql`; only `MigrationRunner` may record applied migrations.
- Standardized `_migrations` DDL behind `create_migration_table()` / `MIGRATIONS_TABLE_SQL` so all bootstrap paths share the same schema and `applied_at` default.
- Locked the canonical tracking semantics with tests that expect `_migrations.name == <full filename>.sql` and `checksum == sha256(file contents)`.
## T5 rerun decisions (2026-03-31)

- Chosen canonical secret persistence boundary: plaintext secret input is only accepted at `EnvironmentService::upsert_variable(...)`; repositories continue to require encrypted-looking storage shape and never perform fallback encryption themselves.
- Chosen canonical degraded bootstrap behavior: if `master.key` is missing while persisted secret rows already exist, bootstrap must force degraded mode and block secret-dependent flows rather than auto-generating a new key.
- Chosen read-path default: `EnvironmentService::list_variables()` and `find_variable_by_id()` replace secret `value` with `masked_preview` so callers do not accidentally receive ciphertext or plaintext in standard DTO/read flows.
- Chosen T5 schema alignment fix: keep migration column names (`columns_json`, `row_json`) authoritative and update repositories to match them instead of changing model semantics or broadening scope.
## T6 decisions (2026-03-31)
- Chosen runtime bridge strategy: keep frontend command names in dotted shared-contract form (`environment.list`) but translate them to underscore Tauri handler names at the last possible point in `src/services/tauri-client.ts`, avoiding any direct `invoke()` usage outside the approved client layer.
- Chosen DTO expansion: add `envType` to shared environment DTO/contracts so production warning behavior is data-driven instead of inferred from free-form names.
- Chosen CRUD completeness boundary: add the minimal `environment.variable.delete` command/handler/UI affordance because repository support already existed and variable deletion is necessary for T6 environment-variable CRUD completeness.
## T6 regression decisions (2026-03-31)
- Kept existing Rust T6 command signatures unchanged (`payload, state`) and fixed the frontend bridge instead, because one adapter change in `src/services/tauri-client.ts` repairs all current T6 handlers consistently without broadening backend scope.
- Kept degraded-mode UI detection aligned to the backend's real serialized code (`SECRET_KEY_MISSING`) instead of trying to rename backend error serialization during this narrow regression fix.
## T6 preview fallback decisions (2026-03-31)
- Kept the real typed Tauri path intact and added a browser-only fallback inside `src/services/environment-client.ts`, rather than branching inside the route, so T6 still has one client boundary.
- Scoped the preview adapter strictly to environment-management flows in `src/services/environment-preview-client.ts`; no generic mock platform or cross-feature preview layer was introduced.
- Chosen deterministic degraded QA control: `previewDegraded=1` query param (with optional localStorage backing) blocks secret upserts in preview and triggers the same T6 degraded-mode UI path.

## T7 decisions (2026-03-31)
- Chosen command namespace: use dotted shared-contract names under `dataTable.*` / `dataTable.row.*` in TypeScript and translate to underscore Tauri handlers only at `src/services/tauri-client.ts`, matching the T6 runtime bridge strategy.
- Chosen metadata boundary: expose association-ready table metadata directly on `DataTableDto.associationMeta` so T15 can consume row availability counts and stable ids without requiring any T7 linkage UI.
- Chosen preview strategy: keep browser-only fallback strictly inside `src/services/data-table-client.ts` and `src/services/data-table-preview-client.ts`; the route remains feature-focused and never branches on runtime platform details.
- Chosen import/export scope: support only deterministic CSV/JSON baseline payloads with full-reject malformed validation, explicitly excluding spreadsheet behavior, formulas, and smart mapping.

## T8 decisions (2026-03-31)

- Chosen persistence mapping: keep `api_endpoints` + `assertions` as the canonical storage, and attach API test cases to `test_cases` through `api_endpoint_id` so run-result foreign keys remain valid without T9 UI work.
- Chosen auth scope: implement only Phase 1 auth types (`none`, `bearer`, `basic`, `api_key`) and reject unsupported types with validation errors.
- Chosen preflight behavior: variable-resolution failures are classified as preflight build failures and returned before outbound request dispatch, with clear `API_REQUEST_BUILD_FAILED` code.
- Chosen result contract extension: `api.execute` returns structured execution payload (status, failure kind, assertion results, normalized/redacted previews) instead of the previous minimal statusCode/duration/body fields.
- Chosen redaction policy: never persist raw auth header/token/basic credential/api-key values in request previews; all sensitive preview values are replaced with `[REDACTED]`.

## T8 follow-up decisions: query params persistence (2026-03-31)

- Chosen root-cause fix boundary: thêm `query_params` vào `ApiEndpoint` model và `query_params_json` vào `api_endpoints` persistence path, thay vì thay đổi DTO/contracts hay bỏ field.
- Chosen migration strategy: thêm migration additive `002_add_api_endpoint_query_params.sql` với `ALTER TABLE ... ADD COLUMN query_params_json TEXT NOT NULL DEFAULT '{}'` để không phá dữ liệu DB hiện có.
- Chosen regression strategy: tăng `tests/frontend/api-engine-t8.test.ts` để bắt buộc sự hiện diện mapping model->service->repository cho `queryParams`, đảm bảo test fail trước fix và pass sau fix.

## T9 decisions (2026-03-31)

- Chosen client boundary: thêm `src/services/api-tester-client.ts` làm entrypoint duy nhất cho T9 và giữ raw `invoke()` tiếp tục bị cô lập trong `src/services/tauri-client.ts`, đúng guardrail của plan và pattern T6/T7.
- Chosen preview strategy: chỉ thêm `src/services/api-tester-preview-client.ts` cho browser-only QA với localStorage seed data và redacted request preview; route không tự branch theo runtime.
- Chosen list/load compromise: dùng local workspace cache riêng cho T9 để nuôi collection tree khi chạy qua typed Tauri path, thay vì invent thêm backend command ngoài scope như `api.testcase.list` hoặc `api.testcase.get`.
- Chosen UI structure: bố cục 3 cột (collection tree, request/assertion builder, response/result viewer) để giữ toàn bộ authoring + execution feedback trên một screen `/api-tester`, phù hợp acceptance của T9 mà không lấn sang suite runner hay advanced scripting.
## T10 decisions (2026-03-31)
- Chosen architecture boundary: thêm `ArtifactService` ở backend service layer thay vì thêm command/UI mới, vì T10 chỉ cần baseline reusable cho path + export/report persistence mà chưa cần IPC surface riêng.
- Chosen storage split: artifact payload tiếp tục ghi ra filesystem theo `AppPaths`, còn metadata manifest được lưu vào bảng SQLite mới `artifact_manifests`; quyết định này giữ đúng guardrail "artifacts on filesystem, metadata in SQLite".
- Chosen export baseline: `persist_report_export(...)` chỉ hỗ trợ sanitized `json` và `html` outputs cho report/export artifacts; không kéo CSV runner/history packaging flow của T15/T17 vào sớm.
- Chosen integration seam: nối `data_table_export(...)` với `ArtifactService` bằng một preview-safe persisted JSON artifact + manifest write để chứng minh baseline filesystem persistence đã tồn tại mà không mở thêm route hay command mới.

## T11 decisions (2026-03-31)

- Chosen command surface: bổ sung duy nhất `browser.health.check` vào shared command contracts (TS + Rust), giữ recorder/replay commands hiện hữu nguyên trạng để không mở rộng scope sang T12/T14.
- Chosen architecture boundary: thêm `src-tauri/src/services/browser_automation_service.rs` và dùng handler `browser_health_check` trong `lib.rs`; mọi browser runtime probing/health semantics nằm trong service layer.
- Chosen Chromium-only fallback semantics: nếu runtime bị disable tường minh (`TESTFORGE_BROWSER_AUTOMATION_DISABLED`) => `unavailable`; nếu chưa tìm thấy binary Chromium candidates => `degraded`; tìm thấy candidate tồn tại => `healthy`.
- Chosen event strategy: tái sử dụng event contract sẵn có `browser.health.changed` bằng `app.emit(...)` từ service thay vì tạo event/DTO mới.

## T12 decisions (2026-03-31)

- Chosen command surface: giữ `browser.recording.start`/`browser.recording.stop` và bổ sung tối thiểu `browser.recording.cancel` để đáp ứng tiêu chí stop/cancel flow mà không mở rộng sang replay/UI scope.
- Chosen state machine: mở rộng `RecordingState` thành `Idle | Recording | Failed` với metadata `captured_steps`, `last_error`, `recoverable` nhằm bảo toàn partial steps khi browser/session lỗi recoverable.
- Chosen persistence path: ghi normalized recorder output trực tiếp vào schema sẵn có `ui_scripts` + `ui_script_steps` và link `test_cases.ui_script_id`, không thêm bảng mới.
- Chosen confidence strategy: áp dụng deterministic rule-set `high|medium|low` dựa trên action + selector strength + value presence (không dùng heuristic ngẫu nhiên) để T13 step editor có tín hiệu ổn định.

## T13 decisions (2026-03-31)

- Chosen client boundary: thêm `src/services/web-recorder-client.ts` làm entrypoint duy nhất cho T13 và giữ raw `invoke()` tiếp tục bị cô lập trong `src/services/tauri-client.ts`, đúng guardrail của plan.
- Chosen preview strategy: chỉ thêm `src/services/web-recorder-preview-client.ts` cho browser-only QA với localStorage seed data và scoped `CustomEvent` emission; route không tự branch theo runtime platform.
- Chosen hydration compromise: dùng workspace cache riêng cho T13 để nuôi draft state khi typed seam hiện tại chưa có list/load UI command, thay vì invent backend API ngoài scope.
- Chosen contract extension: thêm `confidence` vào `UiStepDto` ở TS/Rust contracts để T13 hiển thị low-confidence state trực tiếp và ổn định theo pipeline T12.
- Chosen conflict gate: chặn trực quan hành động record khi `run-store` không idle, nhưng không triển khai replay execution thật vì đó là scope T14.

## T13 bugfix decisions (2026-03-31)

- Chosen fix boundary: vá `src/hooks/useTauriEvent.ts` thay vì route, vì đây là seam consumer đang làm rơi preview events; nhánh Tauri `listen(...)` được giữ nguyên hoàn toàn.
- Chosen preview-local transport: dùng `window.addEventListener(eventName, ...)` + `CustomEvent.detail` chỉ khi không có `__TAURI_INTERNALS__`, tránh tạo generic mock platform dùng chung cho feature khác.
- Chosen preview-state alignment: khi `startRecording()` phát step captured trong preview, ghi luôn step đó vào `draft.steps` local để realtime stream và persisted preview draft không lệch nhau trong QA flow.
## T14 decisions (2026-03-31)

- Chosen command/result boundary: `browser.replay.start` trả `UiReplayResultDto` (status, failedStepId, screenshotPath) thay vì response tối giản chỉ có runId để phản ánh rõ step-level failure semantics ngay tại typed contract.
- Chosen cancellation boundary: bổ sung typed command `browser.replay.cancel` và xử lý idempotent bằng replay state trong `AppState` + service method `cancel_replay(...)`; tránh đẩy cancel semantics vào route/UI.
- Chosen execution strategy: replay đọc `ui_script_steps` đã persisted từ DB, chạy tuần tự theo `step_order`, không song song, không branching, đúng guardrail Chromium-only Phase 1.
- Chosen artifact strategy: screenshot-on-fail tạo file theo policy hiện có (`screenshots/...`) và persist metadata vào `artifact_manifests`; không thêm bảng/migration mới ngoài baseline T10.
## T14 follow-up decisions (2026-03-31)

- Chosen runtime strategy: dùng Chromium CLI adapter nội bộ trong `BrowserAutomationService` thay vì giữ simulated replay; adapter thực thi concrete browser work (navigate/dump-dom/screenshot) mà không leak browser internals ra ngoài service boundary.
- Chosen failure semantics: với actions yêu cầu tương tác DOM sâu (click/fill/select/check/uncheck), adapter trả lại explicit `StepExecution` có context thay vì giả lập success.
- Chosen screenshot strategy: capture thật bằng `--screenshot` và kiểm tra file không rỗng trước khi gọi `ArtifactService::persist_artifact_manifest`, đảm bảo artifact path + manifest đều hợp lệ.
- Chosen gating policy: replay chỉ chạy khi runtime `healthy`; bất kỳ trạng thái không healthy đều chặn ngay từ `start_replay(...)` theo T11.
## T14 interaction follow-up decisions (2026-03-31)

- Chosen seam: giữ toàn bộ interaction execution trong `BrowserAutomationService` qua `ChromiumCliReplayRuntimeAdapter`; không thêm invoke/frontend path mới.
- Chosen minimum viable interaction support: implement `click/fill/select/check/uncheck` bằng strategy validate-DOM + record interaction để đáp ứng replay acceptance path của script recorder cơ bản.
- Chosen test guard: regression test buộc code phải có đường gọi runtime interaction methods và cấm tồn tại `unsupported_interaction_error(...)` để tránh regress về hard-fail behavior.
## T14 real-interaction decisions (2026-03-31)

- Chosen strategy: bỏ Chromium-CLI-only pseudo interaction và chuyển sang Node+CDP interaction executor để có browser-side state change thật cho flow phase-1.
- Chosen assertion policy: `assert_text` chỉ dựa trên DOM snapshot thật (sau interaction), không dùng local bookkeeping để pass.
- Chosen scope guard: chỉ nâng interaction path cho T14 replay executor; không mở rộng sang T15 orchestration hoặc frontend changes.
## T14 smoke harness decisions (2026-03-31)

- Chosen location: `tests/frontend/browser-replay-t14-smoke.ts` + npm entrypoint `test:t14:smoke` để orchestrator chạy độc lập, không phụ thuộc full Tauri runtime.
- Chosen smoke target: HTML tĩnh deterministic nội bộ thay vì external website để tránh flakiness mạng và giữ kiểm chứng interaction rõ ràng.
- Chosen status model: `SMOKE_PASS | SMOKE_BLOCKED | SMOKE_FAIL` với diagnostics JSON in stdout; chỉ PASS khi interaction path thực sự chạy và DOM transitions đúng.

## T17 exploration decisions (2026-04-01)
- Chosen confirmed packaging seam set for T17: treat `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/utils/paths.rs`, `src-tauri/src/services/browser_automation_service.rs`, `src-tauri/src/services/artifact_service.rs`, and `src/components/StatusBar.tsx` as the minimal authoritative surface to change or inspect further for Windows packaging/distribution work.
- Chosen bootstrap interpretation: reuse the existing `AppPaths` app-data-root policy as the manual-update-safe separation boundary, because all persisted runtime state already lives outside the app bundle under app-data and no competing path policy exists in the repo.
- Chosen metadata interpretation: current app identity/version source of truth is duplicated across `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`, while the frontend status bar is only a hardcoded placeholder; any future T17 implementation should treat runtime-exposed metadata as canonical instead of the current literal UI string.

## T16 decisions (2026-04-01)
- Chosen read-side seam: add only unner.suite.list, unner.run.history, and unner.run.detail through the existing typed command boundary; rerun-failed continues to flow through unner.suite.execute({ rerunFailedFromRunId }) with no separate rerun command.
- Chosen detail contract: RunDetailDto returns run summary + per-case/per-row results + artifact manifests, while each row carries explicit ailureCategory, equestPreview, esponsePreview, and ssertionPreview so the route can render rich diagnostics without re-parsing backend storage shapes.
- Chosen UI structure: keep reporting embedded inside /test-runner as a route-level three-panel view instead of adding a separate reporting route, matching the locked Phase 1 scope and the existing pi-tester/web-recorder screen style.
## T17: Packaging, first-run bootstrap, and Windows distribution flow (2026-04-01)

### Read-only seam assessment
- Treat `src-tauri/src/main.rs::bootstrap(...)` + `src-tauri/src/utils/paths.rs::AppPaths` as the canonical first-run/bootstrap seam for T17 follow-up work; that path already owns app-data initialization, settings bootstrap, DB open/migrations, and secret-store degraded detection.
- Treat `src-tauri/src/services/browser_automation_service.rs::{check_runtime_health,detect_chromium_runtime,chromium_candidates}` plus `src-tauri/src/lib.rs::browser_health_check(...)` as the canonical runtime-guidance seam; it already encapsulates discovery semantics and emits the stable `browser.health.changed` event.
- Treat `src/components/StatusBar.tsx` as the smallest existing shell-level UI surface for version display because it is already rendered on every route via `src/App.tsx`.
## T17 decisions (2026-04-01)
- Chosen frontend boundary: keep all shell metadata reads behind `src/services/tauri-client.ts` via `getShellMetadata()`; `App.tsx` only consumes the typed helper and subscribes to the existing `browser.health.changed` event to refresh runtime status.
- Chosen backend boundary: store a minimal `ShellBootstrapSnapshot` inside `AppState` during `main.rs::bootstrap(...)` so first-run/degraded/master-key bootstrap facts stay anchored to the canonical startup seam and can be read later without re-running bootstrap side effects.
- Chosen UI surface: use `StatusBar` as the only always-visible shell surface for runtime version + bootstrap/runtime guidance, avoiding any new route, wizard, or onboarding/reporting scope.
