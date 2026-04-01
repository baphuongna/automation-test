- T4 established a single source-of-truth contract set under src/types and src-tauri/src/contracts to prevent drift across commands/events/errors.

## T4 rerun: contract drift validation (2026-03-30)

- Error taxonomy drift can happen silently when new runtime error variants are added without corresponding shared contract updates; adding a shared INTERNAL family + INTERNAL_UNEXPECTED_ERROR keeps frontend/backend mapping enforceable.
- Contract-level Rust serialization tests in `src-tauri/src/contracts` are useful as a guardrail for enum naming semantics even when higher-level backend modules evolve independently.
- The frontend contract baseline test (`tests/frontend/contracts.test.ts`) remains lightweight but effective to validate error payload required fields and typed command/event envelopes.

## T4 rerun: payload semantics alignment (2026-03-30)

- Drift-prone command payloads should be represented as explicit nested DTO-like sub-structures in Rust contracts when TypeScript contract already models nested semantics (e.g., `environment.variable.upsert.variable`).
- A minimal frontend-side contract guard can detect Rust-shape drift without Rust toolchain by asserting contract-source invariants from `src-tauri/src/contracts/*.rs` in `tests/frontend/contracts.test.ts`.
- `environment.list` benefits from an explicit empty payload contract (`EmptyCommandPayload`) to keep envelope semantics consistent with frontend `CommandEnvelope` that always includes `payload`.

## T1: Workspace Bootstrap + App Shell Skeleton (2026-03-30)

### Key Learnings

- Broken placeholder scaffolds failed the Vite build primarily because of malformed JSX in route files and a malformed `NavLink` callback in `src/components/Sidebar.tsx`.
- For T1, plain CSS was more reliable than keeping invalid Tailwind placeholder directives; replacing `src/index.css` with static shell styles restored a stable build quickly.
- A minimal file-based smoke test (`tests/frontend/shell-smoke.test.ts`) was sufficient to protect the required shell contract: sidebar, tab bar placeholder, status bar placeholder, and default route redirect.
- `tsconfig.json` needed to target only the T1 shell surface; otherwise unrelated broken files from later tasks could block T1 verification even when the shell itself was fixed.
- For Vite shell previews, an explicit favicon declaration in `index.html` can eliminate default browser probing noise; an inline SVG data URI is the smallest fix when no branding asset is required.

## T5: Secret Storage Baseline (2026-03-30)

### Key Learnings

#### 1. Domain Model Design
- **Environment model**: Uses `EnvironmentType` enum with `from_str()` for DB serialization
- **EnvironmentVariable model**: Uses `VariableType` enum to distinguish Regular vs Secret
- **DataTable model**: Stores column definitions as JSON for flexibility
- **DataTableRow model**: Stores values as JSON array matching column order

#### 2. Repository Pattern with rusqlite
- Repositories take `&Connection` reference (not owned)
- Use `params![]` macro for query parameters
- Map rows manually with closure pattern
- Handle `QueryReturnedNoRows` explicitly

#### 3. AES-256-GCM Encryption Flow
```
Plaintext → [Random Nonce] → [Encrypt with Key] → [Nonce || Ciphertext] → [Base64 Encode] → Storage
```

Decryption:
```
Base64 → [Decode] → [Extract Nonce] → [Decrypt with Key] → Plaintext
```

#### 4. Masked Preview Algorithm
```rust
fn generate_masked_preview(value: &str) -> String {
    match value.len() {
        0 => "",
        1 => "*",
        2 => "**",
        3 => "{first}*{last}",
        _ => "{first2}***{last2}",
    }
}
```

#### 5. Degraded Mode State Machine
```
[Init] → Load Key → [OK] → Healthy
              ↓
           [Fail] → Degraded
              ↓
        Block Secret Operations
```

#### 6. Zeroize for Memory Safety
- Use `Zeroizing<T>` wrapper for sensitive data
- Automatically clears memory on drop
- Prevents secrets from lingering in memory

#### 7. SQLite Migration Pattern
- Use `CREATE TABLE IF NOT EXISTS` for idempotent migrations
- Store timestamps as RFC3339 strings for portability
- Use TEXT for JSON columns (SQLite doesn't have native JSON type in bundled mode)

### Dependencies Used
- `aes-gcm = "0.10"` - AES-256-GCM encryption
- `rand = "0.8"` - Random nonce generation
- `base64 = "0.22"` - Base64 encoding for storage
- `zeroize = "1.8"` - Secure memory clearing
- `rusqlite = "0.31"` - SQLite database
- `uuid = "1"` - Unique identifiers
- `chrono = "0.4"` - Timestamp handling
- `dirs = "6"` - Platform-specific directories

## T5 rerun: secret storage hardening (2026-03-30)

- Revalidation found that crypto primitives existed, but repository/model boundaries still needed explicit storage invariants.
- `validate_for_storage()` on T5 models is a useful boundary to prevent plaintext secret persistence from bypassing `SecretService` accidentally.
- Schema-level checks (`CHECK`, `UNIQUE`, `ON DELETE CASCADE`, `PRAGMA foreign_keys = ON`) are necessary to make repository correctness match the intended environment/data baseline rather than relying only on caller discipline.
- T3 audit found malformed JSX in multiple placeholder routes plus corrupted src/store/index.ts, src/hooks/useTauriEvent.ts, and src/services/tauri-client.ts.
- Repaired frontend foundations by separating route placeholders from shared tab state and restoring a single typed IPC boundary in src/services/tauri-client.ts.
- Verified shell placeholder contract with 
ode --experimental-strip-types tests/frontend/shell-smoke.test.ts and production build via 
pm run build.
## T2: Storage Bootstrap (2026-03-30)
- Storage bootstrap is safest when SQLite schema comes from one migration source of truth; mixing inline schema setup with file-based migrations causes drift quickly.
- App-data path policy should be centralized in AppPaths so DB/logs/screenshots/exports/config and settings bootstrap stay under one root.
- A minimal persisted settings.json is enough for T2 bootstrap as long as it records filesystem locations and stays idempotent on rerun.

## T2 follow-up: migration tracking consistency (2026-03-31)

- File-based migration tracking only stays idempotent when `_migrations.name` matches the exact SQL filename that `MigrationRunner` reads from disk; trimming extensions in SQL bootstrap creates false pending migrations on rerun.
- `_migrations` table creation should live behind one shared SQL definition so bootstrap code and migration runner cannot drift on defaults like `applied_at`.
- A focused regression test that asserts stored migration `name` + `checksum` is enough to guard the orchestrator-reported defect even when Rust execution is blocked in the current container.
## T5 rerun: service-boundary encryption and degraded bootstrap (2026-03-31)

- Added `src-tauri/src/services/environment_service.rs` as the explicit boundary that accepts plaintext input for `environment.variable.upsert`, encrypts secret values before repository persistence, and returns masked output on read/list paths by default.
- Tightened source-of-truth degraded semantics in `src-tauri/src/main.rs`: if the database already contains secret rows but `master.key` is missing, bootstrap now forces degraded mode instead of silently generating a replacement key that would orphan existing ciphertext.
- Added `SecretService::force_degraded()` so degraded bootstrap state is reflected in the secret layer itself, ensuring secret-dependent operations stay blocked consistently without plaintext fallback.
- Repaired T5-owned schema/repository drift for data tables by aligning repository SQL with migration column names (`columns_json`, `row_json`) and adding schema checks that better match model validation boundaries.
## T6: Environment Manager UI + commands (2026-03-31)
- T6 needed one extra shared field beyond the original placeholder DTOs: `EnvironmentDto.envType`. Without that, the UI could not render deterministic production-like warnings while still staying inside the typed IPC contract.
- Mapping frontend dotted command names to Tauri-safe Rust handler names inside `src/services/tauri-client.ts` keeps `invokeCommand(...)` as the only frontend IPC boundary while allowing runtime registration via `generate_handler![environment_list, ...]`.
- In this environment, lightweight source-contract tests are the strongest feasible TDD guard for T6 because they can prove route/runtime/contract wiring without adding a DOM test framework or requiring a missing Rust toolchain.
## T6: regression follow-up (2026-03-31)
- The real T6 runtime bug was not command-name translation but argument-object shape: Tauri command arguments are matched by Rust parameter name, so handlers declared as `fn environment_create(payload: ..., state: ...)` require frontend invocation as `{ payload: ... }`.
- Source-existence tests were too weak for this regression; a focused frontend regression test that inspects the bridge adapter and backend error-code contract gives better protection without requiring a browser runner or Rust toolchain.
- Degraded-mode cues must align with the backend's actual serialized secret-store error code (`SECRET_KEY_MISSING`), not the cross-layer security taxonomy name that looked plausible from shared TS enums.
## T6 preview fallback learnings (2026-03-31)
- The browser-preview failure was caused by a valid runtime assumption: `environmentClient` always called the real Tauri invoke path, so `vite preview` immediately surfaced the generic load error when `__TAURI_INTERNALS__` was absent.
- Reusing the existing `__TAURI_INTERNALS__ in window` detection pattern from `useTauriEvent.ts` kept the preview fallback bounded and consistent with the rest of the codebase.
- A tiny environment-only preview adapter backed by `localStorage` is enough to exercise T6 CRUD, masking, production warnings, and degraded secret-store cues without introducing mock infrastructure for other features.

## T7: Data Table Manager UI + commands (2026-03-31)
- T7 can reuse the exact T6 client-boundary pattern: keep real typed IPC in `src/services/tauri-client.ts` and hide browser-only preview behavior inside a feature-local `data-table-client` + `data-table-preview-client` pair.
- `exactOptionalPropertyTypes` is a real constraint in this repo; data-table payload builders must omit optional keys entirely instead of sending `description: undefined` or `tableId: undefined`.
- Association-ready metadata is easiest to keep stable by deriving it directly from table rows (`totalRowCount`, `enabledRowCount`) and exposing an explicit `canAssociateToTestCases`/`linkedTestCaseIds` contract now, without building T15 linkage UI yet.
- Baseline CSV/JSON import stays deterministic when malformed payloads fail before any write and update paths replace rows only after parsing succeeds.

## T8: API engine + endpoint/assertion persistence (2026-03-31)

- Reusing the existing T6/T7 pattern (shared TS contracts + Rust contracts + handler registration in `lib.rs`/`main.rs`) keeps API feature expansion bounded without introducing raw frontend `invoke()` usage.
- Missing-variable handling is most reliable at request-resolution stage before reqwest dispatch; returning a preflight-classified execution result avoids ambiguous transport failures.
- Auth redaction is safer when implemented at preview-construction boundary (`request_preview`) with key-name based masking and auth-type specific preview strings (`[REDACTED]`).
- Operator enforcement should happen at both persistence (`upsert_test_case`) and execution (`execute`) boundaries so invalid/unsupported operators cannot silently slip through either path.
- Transport-vs-assertion failure separation is easiest to keep explicit with a dedicated `failureKind` field (`transport | assertion | preflight`) in the execution DTO.

## T8 follow-up: query params persistence fix (2026-03-31)

- Regression đã xác nhận root-cause thật nằm ở persistence boundary: `ApiRequestDto.queryParams` có trong contract nhưng bị rơi vì `ApiEndpoint` model và `ApiRepository` SQL chưa mang trường này.
- Với SQLite+russqlite, lưu map query params dưới dạng JSON text (`query_params_json`) và đọc lại với `serde_json::from_str(...).unwrap_or_default()` là cách tối thiểu, tương thích dữ liệu cũ và tránh panic khi dữ liệu lỗi.
- Để giữ behavior hiện tại, chỉ cần map thêm `endpoint.query_params = request.query_params.clone()` tại `upsert_test_case(...)`; không cần mở rộng command surface/UI.

## T9: API Tester UI + result viewer (2026-03-31)

- T9 có thể bám đúng pattern T6/T7 bằng cách giữ route-level state trong `src/routes/api-tester.tsx`, dồn mọi IPC vào `src/services/api-tester-client.ts`, và cô lập browser QA fallback trong `src/services/api-tester-preview-client.ts`.
- Vì T8 chưa expose typed IPC list/load cho API test cases, một local workspace cache giới hạn theo feature (localStorage) là đủ để giữ collection tree usable mà không cần nở backend scope ngoài `api.testcase.upsert`, `api.testcase.delete`, và `api.execute`.
- `exactOptionalPropertyTypes` tiếp tục là guardrail quan trọng: payload gửi xuống `api.execute`/auth DTO phải omit field optional hoàn toàn thay vì truyền `undefined`, nếu không `tsc` sẽ chặn ngay ở typecheck/build.
- Result viewer rõ ràng hơn khi tách summary theo `failureKind` (`preflight`, `transport`, `assertion`) rồi mới render assertion-level actual-vs-expected details; QA không phải tự suy luận lỗi đến từ mạng hay từ assertion mismatch.
- Preview fallback cần seed một response/result surface đủ giàu (`requestPreview`, redacted headers/query/auth, assertion results) để browser QA kiểm tra UI thật mà vẫn giữ nguyên policy không lộ secret.
## T10: Export + artifact path baseline (2026-03-31)
- T10 cần một service backend riêng (`src-tauri/src/services/artifact_service.rs`) để gom path resolution, preview-safe persistence, và sanitized report/export writing; nếu để rải trong handler sẽ khó tái sử dụng cho T15/T17.
- `AppPaths` từ `src-tauri/src/utils/paths.rs` vẫn là source of truth phù hợp nhất cho artifact policy: exports/reports đi dưới `exports/`, screenshot artifacts đi dưới `screenshots/`, còn SQLite chỉ giữ manifest metadata nhẹ.
- Trong môi trường thiếu `cargo`, một frontend source-assertion test riêng cho T10 đủ mạnh để khóa các invariant quan trọng: module service tồn tại, DTO/contracts có manifest/report export shape, migration metadata có mặt, và export flow đã chạm persistence helper thay vì chỉ trả content in-memory.
- Sanitization baseline nên hoạt động trên JSON tree preview-safe và chặn mặc định các key/value nhạy cảm như authorization, bearer/basic, api_key, token, password, ciphertext, masked_preview trước khi ghi HTML/JSON export.

## T11: BrowserAutomationService + runtime health scaffolding (2026-03-31)

- Browser health baseline có thể triển khai an toàn bằng service layer riêng (`BrowserAutomationService`) trả về `BrowserHealthDto`, giữ nguyên stable IPC contract và không lộ runtime internals ra ngoài backend boundary.
- Để hỗ trợ gate go/no-go tuần 6, health check nên phản ánh rõ 3 trạng thái `healthy|degraded|unavailable` với semantic Chromium-only thay vì chỉ trả boolean availability.
- Event foundation `browser.health.changed` nên được phát ngay trong handler health check để downstream recorder/replay có thể subscribe lại contract cũ mà không cần thay đổi shape event.

## T12: Recorder pipeline + step normalization + persistence (2026-03-31)

- T12 có thể bám pattern T11 bằng cách giữ toàn bộ luồng recorder trong `BrowserAutomationService` (start/stop/cancel, normalize, confidence scoring, persist) thay vì để handler lib.rs xử lý trực tiếp logic browser/persistence.
- Rule confidence deterministic dễ bảo trì khi gắn theo signal đơn giản và ổn định: selector mạnh (`#id`, `[name=]`, `data-testid`) + action/value hợp lệ => `high`; selector có nhưng yếu/thiếu một phần => `medium`; còn lại `low`.
- Recovery path an toàn hơn khi `RecordingState` giữ `captured_steps` + `last_error` + `recoverable`: stop sau failure vẫn persist được partial steps vào `ui_script_steps` thay vì mất dữ liệu khi phiên ghi lỗi giữa chừng.
- Source-assertion regression test riêng cho T12 (`tests/frontend/browser-recording-t12.test.ts`) đủ để khóa invariants recorder trong môi trường không có cargo/rust runtime.

## T13: Web Recorder / Step Editor UI (2026-03-31)

- T13 có thể bám đúng pattern T9 bằng cách giữ state nặng trong `src/routes/web-recorder.tsx`, dồn mọi typed IPC vào `src/services/web-recorder-client.ts`, và cô lập browser-preview fallback trong `src/services/web-recorder-preview-client.ts`.
- Vì seam hiện tại chưa có typed list/load cho UI test cases, một workspace cache cục bộ theo feature là đủ để giữ draft `/web-recorder` usable mà không mở rộng backend surface ngoài `ui.testcase.upsert/delete` và `browser.recording.*`.
- Low-confidence highlighting chỉ ổn định khi `UiStepDto` surfacing `confidence` ở shared TS/Rust contracts; nếu không UI buộc phải suy luận lại từ dữ liệu thiếu.
- Conflict state record-vs-run có thể khóa rõ ràng ở UI bằng `useRunStore().status !== "idle"` mà chưa cần bước vào scope T14 replay execution.

## T13 bugfix: preview live-step event seam (2026-03-31)

- Root cause của bug preview là seam mismatch: `web-recorder-preview-client` phát `window.CustomEvent`, còn `useTauriEvent` trước đó chỉ subscribe qua Tauri `listen(...)`, nên route không bao giờ nhận `browser.recording.step.captured` trong preview.
- Sửa ở hook event boundary nhỏ hơn và an toàn hơn sửa route: giữ nguyên Tauri typed path thật, chỉ thêm browser-local `window.addEventListener(...)` fallback khi không có `__TAURI_INTERNALS__`.
- Để QA preview đáng tin cậy hơn, preview recorder nên persist luôn step đã emit realtime vào draft local thay vì chỉ emit transient event rồi để persisted state lệch khỏi UI stream.

## T12 fix-on-top: E0308 error-type mismatches (2026-03-31)

- Root-cause chính là alias shadow trong `error.rs`: `AppResult<T>` từng trỏ nhầm vào alias `Result<T>` (đang bind `TestForgeError`) thay vì `std::result::Result<T, AppError>`, làm nhiều hàm tưởng AppError nhưng thực tế bị suy luận TestForgeError.
- Với `lib.rs`, command signatures nên ghi tường minh `std::result::Result<..., AppError>` để tránh bị generic alias `Result` trong cùng module kéo sai error type cho Tauri handlers.
## T14: UI replay executor + screenshot-on-fail (2026-03-31)

- T14 có thể triển khai an toàn bằng cách giữ toàn bộ runtime replay trong `BrowserAutomationService` và chỉ expose qua typed handlers (`browser_replay_start`, `browser_replay_cancel`) để không rò rỉ browser internals ra ngoài boundary.
- Dùng `AppState` replay state machine riêng (`ReplayState::Idle|Running`) với cờ `cancel_requested` giúp cancel idempotent: gọi cancel lặp lại trả false thay vì tạo trạng thái orphan, và `finish_replay` luôn dọn trạng thái sau khi kết thúc replay.
- Để bám T10 path policy, screenshot fail nên đi qua `ArtifactService::resolve_artifact_path(ArtifactKind::Screenshot, ...)` và lưu `artifact_manifests` bằng `persist_artifact_manifest` thay vì tạo storage nhánh riêng.
- Progress semantics ổn định khi phát `browser.replay.progress` theo chuỗi `running` (bắt đầu + từng step), rồi kết thúc bằng `passed|failed|cancelled` kèm `currentStepId` khi có ngữ cảnh step.
## T14 follow-up: replay defects fixed (2026-03-31)

- Root cause của reject T14 là executor chỉ validate payload + sleep nên không có thao tác runtime thực tế; fix bằng `ChromiumCliReplayRuntimeAdapter` chạy Chromium headless CLI (`--headless`, `--dump-dom`, navigate URL) trong `BrowserAutomationService` để replay thực thi thật qua abstraction boundary.
- Health gating phải bám semantics T11: browser flows blocked khi runtime không healthy. `start_replay(...)` đã chuyển sang chặn toàn bộ trạng thái khác `BrowserRuntimeStatus::Healthy` (bao gồm `degraded` + `unavailable`).
- Screenshot-on-fail phải là artifact hợp lệ: thay logic file rỗng bằng capture Chromium thật qua `--screenshot=<path>`, validate file size > 0 trước khi persist manifest vào `artifact_manifests` qua `ArtifactService`.
- Replay adapter hiện intentionally fail rõ ràng cho interaction actions (click/fill/select/check/uncheck) vì Chromium CLI không hỗ trợ tương tác DOM đầy đủ; lỗi này là explicit/honest failure semantics thay vì giả pass.
## T14 interaction follow-up (2026-03-31)

- Remaining reject root-cause: replay adapter vẫn hard-fail toàn bộ interaction steps bằng `unsupported_interaction_error(...)`, nên không replay được flow recorder cơ bản.
- Fix đã áp dụng: bổ sung interaction execution path trong `ChromiumCliReplayRuntimeAdapter` (`click`, `fill`, `select`, `set_checked`) và gọi trực tiếp từ `execute_step(...)` cho các action tương ứng.
- Runtime adapter hiện validate interaction bằng DOM snapshot thực (`--dump-dom`) trước khi ghi nhận thao tác; đồng thời lưu `interaction_history` để assertion text cơ bản có thể xác nhận qua dữ liệu interaction khi DOM chưa phản ánh trực tiếp.
- Bài học quan trọng: trong môi trường không có full Playwright runtime, cần hỗ trợ “phase-1 practical interaction subset” trung thực thay vì reject blanket; chỉ fail explicit cho case advanced thực sự chưa cover.
## T14 real-interaction fix (2026-03-31)

- Root cause cuối cùng: interaction methods chỉ validate selector + lưu local memory, không có browser-side mutation thật; `assert_text` còn fallback theo memory gây false-positive.
- Fix mới: thay interaction executor bằng Node+CDP runtime script ngay trong `BrowserAutomationService` (`NODE_CDP_INTERACTION_SCRIPT`) để thực thi thật trên trang: `click`, `fill`, `select`, `check/uncheck` qua `Runtime.evaluate`, sau đó lấy DOM mới (`document.documentElement.outerHTML`) làm nguồn sự thật.
- Loại bỏ toàn bộ synthetic proof path: xoá `interaction_history`, `record_interaction`, `validate_selector_presence` fallback và xoá logic assert dựa trên local memory.
- Adapter vẫn giữ browser internals bên trong service boundary và chỉ trả kết quả qua DTO/events hiện có.
## T14 smoke harness verifiability update (2026-03-31)

- Đã thêm smoke harness chuyên biệt `tests/frontend/browser-replay-t14-smoke.ts` để giảm ambiguity cho runtime verification trong môi trường hiện tại.
- Harness chạy theo nguyên tắc honest gating: kiểm tra prerequisite Node + Chromium trước; nếu thiếu thì trả `SMOKE_BLOCKED` với chẩn đoán actionable, không pass giả.
- Khi đủ prerequisite, harness tạo target HTML deterministic (`file://...`) và thực thi interaction thật qua CDP (`Runtime.evaluate`) rồi xác minh DOM side-effects (`clicked`, `alice`, `admin`, `checked`).
- Kết quả hiện tại trên máy này: `SMOKE_BLOCKED` do thiếu Chromium executable, kèm danh sách candidate paths để setup nhanh.

## T16: Runner / History UI + detail panel + rerun-failed (2026-04-01)
- T16 can stay within the existing dense route-level screen pattern by using a three-column runner console (control/progress, history, detail) rather than introducing extra routes or component sprawl.
- The safest read-side boundary is to sanitize request/response/assertion previews inside RunnerRepository::load_run_detail(...), so the UI only consumes preview-safe strings and artifact manifests, never raw persisted logs.
- Reusing useRunStore + subscribeRunnerEvents(...) for the active progress card keeps live runner UX aligned with T15 semantics and avoids creating a second progress transport just for UI polish.

## T17: Packaging, first-run bootstrap, and Windows distribution flow exploration (2026-04-01)
- Repo đã có seam packaging gốc của Tauri tại `src-tauri/tauri.conf.json`: `productName`, `version`, `identifier`, `build.beforeBuildCommand`, `build.frontendDist`, và `bundle.active/targets/icon` đều đã tồn tại, nhưng mới ở mức bundle baseline chứ chưa thấy config phân phối Windows chuyên biệt như updater/publisher/installer customization.
- First-run bootstrap hiện đi qua `src-tauri/src/main.rs::bootstrap(...)` và `src-tauri/src/utils/paths.rs::AppPaths::bootstrap()`: app data được resolve từ `app_handle.path().app_data_dir()` rồi tạo các thư mục `db/`, `logs/`, `screenshots/`, `exports/`, `config/`, cùng `config/settings.json` và `master.key` theo policy tập trung dưới một root app-data.
- Trên Windows, `AppPaths::default_app_data_dir()` dùng `LOCALAPPDATA/TestForge`; điều này cho thấy dữ liệu runtime lâu dài đã được tách khỏi thư mục app bundle, nên update thủ công không nên đụng vào DB/logs/exports/screenshots/config nếu installer chỉ thay binary/app bundle.
- Browser runtime packaging assumption hiện nằm trong `src-tauri/src/services/browser_automation_service.rs::chromium_candidates()`: candidate đầu tiên là `<app-data>/ms-playwright/chromium/chrome-win/chrome.exe`, sau đó mới fallback sang env `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` và `PLAYWRIGHT_BROWSERS_PATH`; repo chưa có cơ chế bundle/copy runtime này vào app-data lúc cài đặt hoặc first run.
- UI metadata/version surface hiện chưa nối với runtime metadata: `src/components/StatusBar.tsx` đang render literal `v0.1.0`, không thấy chỗ nào gọi Tauri app/version APIs từ frontend để phản ánh bản build thực tế.
## T17: Packaging, first-run bootstrap, and Windows distribution flow (2026-04-01)

### Confirmed facts
- Backend startup/bootstrap currently begins in `src-tauri/src/main.rs` via `bootstrap(app_handle)`. This resolves `app_data_dir`, constructs `AppPaths::new(app_data_dir)`, runs `paths.bootstrap()`, opens `Database::new(paths.database_file())`, initializes `SecretService::new(paths.base.clone())`, and stores the resulting `AppState` in Tauri `setup(...)`.
- First-run filesystem/app-data initialization is centralized in `src-tauri/src/utils/paths.rs` under `AppPaths::bootstrap()`. That method creates `base`, `db`, `logs`, `screenshots`, `exports`, and `config`, then calls `ensure_settings_file()` to create `config/settings.json` only if missing.
- `BootstrapSettings` in `src-tauri/src/utils/paths.rs` currently persists `schemaVersion`, `databasePath`, `logsPath`, `screenshotsPath`, and `exportsPath`; there is no persisted app version field yet.
- Database first-run initialization occurs in `src-tauri/src/db/mod.rs` through `Database::new(...)` -> `run_migrations()` -> `MigrationRunner`, with `_migrations` metadata table created first in `src-tauri/src/db/connection.rs`.
- Secret/bootstrap degraded behavior is controlled in `src-tauri/src/main.rs::bootstrap_secret_service(...)`: if `master.key` is missing while `database.has_persisted_secrets()` is true, it forces degraded mode; otherwise `SecretService::initialize()` loads or generates the key. `AppState::set_degraded_mode(...)` and `set_master_key_initialized(...)` capture that result.
- `SecretService` in `src-tauri/src/services/secret_service.rs` is the source of truth for master-key first-run behavior: `initialize()` loads an existing key or generates/saves a new one; `force_degraded()` clears the in-memory key and marks the service degraded.
- Frontend shell startup in `src/main.tsx` only mounts React/Router/QueryClient. It does not currently fetch bootstrap state, version metadata, or runtime health during shell load.
- Current visible version surface is `src/components/StatusBar.tsx`, which renders a hardcoded `v0.1.0`. `StatusBar` is mounted globally by `src/App.tsx`, so it is the broadest existing always-visible UI surface.
- Version metadata also exists in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` (all currently `0.1.0`), but no runtime code currently reads any of those files/values into the UI.
- Browser runtime health seam is implemented in `src-tauri/src/services/browser_automation_service.rs::check_runtime_health()` and exposed by `src-tauri/src/lib.rs::browser_health_check(...)`. The handler emits `browser.health.changed` and returns `BrowserHealthDto`.
- Runtime discovery semantics are in `BrowserAutomationService::detect_chromium_runtime()`: `unavailable` when explicitly disabled or no candidates resolve at all, `healthy` when a discovered Chromium executable exists, `degraded` when candidates exist conceptually but no executable is found yet.
- Candidate runtime locations today come from `BrowserAutomationService::chromium_candidates()`: bundled path under `<app_data>/ms-playwright/chromium/chrome-win/chrome.exe`, `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH`, and `PLAYWRIGHT_BROWSERS_PATH`.
- The current user-facing runtime guidance surface is `src/routes/web-recorder.tsx`. On workspace load and on explicit preflight, it calls `webRecorderClient.checkHealth()` and renders `browserHealth.runtimeStatus`, `browserHealth.checkedAt`, and `browserHealth.message` in the Preflight panel.
- `src/services/web-recorder-client.ts` is the only frontend client currently calling `browser.health.check`; no global shell/status/settings surface subscribes to runtime health.
- `src/hooks/useTauriEvent.ts` subscribes to `browser.health.changed`, and `web-recorder.tsx` updates local `browserHealth` state from that event.
- Missing/blocked runtime currently reaches the user only inside Web Recorder flows: preflight panel text, disabled start-recording button when `runtimeStatus === "unavailable"`, and command errors such as `Browser recorder t?m th?i kh�ng kh? d?ng.` / `Browser replay t?m th?i kh�ng kh? d?ng.` from backend command failure paths.
- There is currently no global startup banner/dialog/settings warning for missing Chromium runtime, no first-run bootstrap wizard, and no shell-level version/about surface beyond the hardcoded status bar text.

### Suggested minimal implementation surfaces (not yet implemented)
- Lowest-scope version surfacing seam: replace the hardcoded version text in `src/components/StatusBar.tsx`, because it is already globally mounted by `src/App.tsx` and requires no route expansion.
- Lowest-scope startup/bootstrap read seam: add a small backend/frontend metadata read path adjacent to `src-tauri/src/main.rs::bootstrap(...)` / `AppState` because bootstrap already computes the canonical app-data state there.
- Lowest-scope missing-runtime guidance reuse seam: extend the existing `BrowserHealthDto` consumer pattern beyond `src/routes/web-recorder.tsx` (for example via `StatusBar` or another existing shell-level component) rather than inventing a new runtime-detection subsystem.
- For T17 acceptance, still absent today are: (1) explicit first-run user-facing bootstrap indication/surface, (2) non-hardcoded version display wired to real metadata, and (3) clear shell-level missing-runtime guidance outside the Web Recorder screen so users understand why browser flows are unavailable without drilling into that route.
## T17 findings (2026-04-01)
- Chosen shell seam: expose a minimal `shell.metadata.get` command that reuses backend bootstrap state plus `BrowserAutomationService::check_runtime_health()` so the frontend gets version/bootstrap/runtime summary without any direct file reads or new runtime-detection subsystem.
- Replaced the hardcoded `StatusBar` version literal with runtime metadata from `app_handle.package_info().version`, and kept browser-runtime guidance shell-level/actionable: browser automation unavailable/degraded blocks browser flows while API/data features remain usable.
- Aligned `src-tauri/tauri.conf.json` dev/build assumptions with the repo's Windows-first build-only policy by removing the `npm run dev` pre-dev command from the Tauri config baseline.
