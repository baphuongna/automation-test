# Automation Testing Tool Plan

## TL;DR

> **Quick Summary**: Thiết kế một desktop app cho QA team theo kiến trúc **Tauri + React + Rust + playwright-rs + SQLite** để hỗ trợ API testing, Web UI testing, record & replay, test data management và test runner cho Phase 1.
>
> **Deliverables**:
> - Kiến trúc tổng thể hệ thống
> - Database schema SQLite
> - UI layout cho các màn hình chính
> - Rust backend module design

---

## Context

### Original Request
Phân tích bài toán automation test web/API và quyết định giữa extension hay app. Sau khi phân tích, người dùng chọn hướng **Tauri + playwright-rs**.

### Team Context
- QA team: **low-code**
- Rust experience: **cơ bản**
- Automation testing experience: **chưa từng làm**
- Timeline: **3-6 tháng**, small team
- Workspace hiện tại: **blank slate**, chưa có source code

### Scope Decision
- Chọn **phased approach** thay vì full scope ngay lập tức
- **Phase 1 (3 tháng)**: Core + Essential
  - API Testing Engine
  - Web UI Recorder & Player
  - Test Runner cơ bản
  - Test Data Management
  - Basic Reporting
  - Environment switching
- **Phase 2**: Advanced reporting, scheduling, CI/CD integration, collaboration

### Architecture Decision
- **Chosen**: Tauri + playwright-rs
- **Tradeoff accepted**:
  - playwright-rs là **pre-1.0**
  - Bundle thực tế không còn “siêu nhẹ” vì Playwright browser binaries lớn
  - Team chấp nhận learning curve để giữ hướng Rust/Tauri

### Risk Review Summary
- Playwright **không có Rust support chính thức**, nhưng `playwright-rs` đang active
- `playwright-rs` dùng JSON-RPC tới Playwright Server, cùng pattern với bindings chính thức khác
- `playwright-rs` v0.9 có coverage mạnh ở Locator/Request/Response, nhưng Page/Frame chưa hoàn chỉnh
- Tổng bundle thực tế sẽ tăng mạnh do browser binaries

### Locked MVP Decisions

Các quyết định dưới đây được khóa cứng cho **Phase 1** để tránh scope creep và tránh hiểu sai khi triển khai:

1. **Browser support**
   - Phase 1 chỉ **officially support Chromium**.
   - Firefox và WebKit là **out of scope** cho MVP.

2. **Supported test case types**
   - Phase 1 chỉ hỗ trợ `api` và `ui`.
   - `hybrid` là **out of scope** cho Phase 1.

3. **Web recording scope**
   - Phase 1 chỉ hỗ trợ các action cơ bản:
     - `navigate`
     - `click`
     - `type/fill`
     - `select`
     - `wait_for_selector`
     - `wait_for_navigation`
     - `assert_text`
     - `assert_visible`
     - `screenshot`
   - Out of scope cho Phase 1:
     - multi-tab flows
     - drag & drop
     - upload phức tạp
     - network interception/mocking
     - auto-generate assertions từ traffic
     - full iframe/shadow DOM support

4. **Browser session UI model**
   - Phase 1 **không cam kết embedded browser thật bên trong app panel**.
   - Browser automation có thể chạy trong browser window/session riêng do engine mở.
   - App desktop chịu trách nhiệm hiển thị controls, step stream, status và results.

5. **Pause / Resume**
   - Phase 1 **không support pause/resume** cho suite run hoặc recording.
   - Chỉ support: `Run`, `Stop`, `Rerun Failed`.

6. **Authentication support for API testing**
   - Phase 1 hỗ trợ:
     - `none`
     - `bearer token`
     - `basic auth`
     - `api key` (header/query)
   - Out of scope:
     - OAuth interactive flow
     - mTLS phức tạp
     - custom request signing nâng cao

7. **Basic reporting definition**
    - “Basic reporting” trong Phase 1 chỉ bao gồm:
      - pass/fail/skip summary
      - duration
      - per-run detail
     - sanitized request/response preview
     - screenshot links
     - export HTML/JSON đơn giản
    - Không bao gồm:
      - trend analytics
      - flaky analysis
      - advanced dashboards
    - Run history / runner history view là điểm truy cập chính cho basic reporting trong Phase 1.

8. **Data-driven execution semantics**
   - Nếu test case gắn `data_table`, mỗi row được coi là **một lần execution con**.
   - Mặc định **continue all rows**, không fail-fast toàn bộ table.
   - Suite summary tính theo **executed rows**, không chỉ theo số test case gốc.

9. **Manual script authoring scope**
   - Phase 1 luôn có **step editing cơ bản**.
   - Manual full script authoring from scratch chỉ ở mức tối thiểu, không phải focus chính.
   - Nếu recording không ổn định, step editor sẽ trở thành fallback path chính để tạo script.

10. **Run result persistence policy**
    - Database chỉ lưu summary + sanitized/truncated logs.
    - Artifact lớn (screenshot, export files, raw attachments nếu có) lưu ở filesystem.
    - Không lưu blob ảnh trực tiếp trong SQLite.

11. **Secret storage baseline**
    - Phase 1 dùng **app-managed encryption baseline** cho secrets at rest.
    - OS keychain integration là enhancement tốt nhưng **không bắt buộc để ship MVP**.

12. **Runtime failure isolation**
    - Nếu browser runtime / automation engine lỗi, chỉ block **Web UI testing features**.
    - API testing, environment management, data management vẫn phải usable.

13. **Language / theme scope**
    - Phase 1 không cần i18n framework đầy đủ.
    - Phase 1 dùng **một ngôn ngữ UI duy nhất** cho nội bộ trước.
    - Theme switching là nice-to-have, không phải MVP blocker.

14. **Working product naming**
    - Internal codename cho Phase 1: **TestForge**.
    - Nếu cần nội dung trung tính hơn trong tài liệu kỹ thuật, có thể dùng tên mô tả `Automation Testing Tool`, nhưng UI/product naming mặc định dùng `TestForge`.

---

## Section 1: Kiến trúc tổng thể

### System Layers

```
Tauri Desktop App
├── Frontend: React + TypeScript + Vite
│   ├── API Tester
│   ├── Web Recorder
│   ├── Test Runner
│   ├── Test Data Manager
│   └── Environment Manager
├── IPC Layer: Tauri Commands + Events
├── Backend: Rust + tokio
│   ├── API Test Engine
│   ├── Playwright Engine
│   ├── Test Runner Service
│   └── Storage Service
└── Storage/Runtime
    ├── SQLite
    ├── Playwright Server
    └── File System (screenshots, exports)
```

### Key Technology Stack

| Layer | Technology | Purpose |
|------|------|------|
| Desktop Shell | Tauri v2 | Window management, IPC, native integration |
| Frontend | React 18 + TypeScript 5 + Vite | UI cho QA team |
| Styling | TailwindCSS + shadcn/ui | UI components, layout, accessibility |
| Backend | Rust 1.85+ | Business logic và system access |
| Async Runtime | tokio | Async execution |
| API Engine | reqwest + rustls | HTTP requests |
| Browser Automation | playwright-rs v0.9 | Web UI automation |
| Storage | SQLite + rusqlite | Persist test data |
| Serialization | serde / serde_json | IPC payloads |

### Data Flow: API Test

1. React UI gửi `run_api_test` qua Tauri IPC
2. Rust backend load test case + environment
3. Variable resolver thay `{{variable}}`
4. API engine gửi request bằng reqwest
5. Assertion engine đánh giá kết quả
6. Kết quả được lưu SQLite
7. Tauri event đẩy kết quả ngược về UI

### Data Flow: Web UI Recording

1. React UI gửi `start_recording`
2. Rust backend mở browser qua playwright-rs
3. Recorder capture actions (click, type, navigate, wait)
4. Steps được stream realtime về UI
5. Người dùng stop recording
6. Script steps được persist vào SQLite

---

## Section 2: Database Schema (SQLite)

### Core Tables

- `environments`
- `environment_variables`
- `api_collections`
- `api_endpoints`
- `assertions`
- `ui_scripts`
- `ui_script_steps`
- `test_cases`
- `test_suites`
- `suite_cases`
- `test_runs`
- `test_run_results`
- `data_tables`
- `data_table_rows`

### Design Principles

- API tests và UI tests được gom về abstraction chung là `test_cases`
- `test_suites` quản lý tập hợp nhiều test cases
- `test_runs` và `test_run_results` tách summary khỏi detail logs
- `data_tables` hỗ trợ data-driven testing cho cả API và UI
- `environment_variables` hỗ trợ variable substitution và secret masking
- Trong Phase 1, `test_cases.type` chỉ hỗ trợ `api` và `ui`; `hybrid` không nằm trong MVP scope

### Important Relationships

- `environment_variables.environment_id -> environments.id`
- `api_endpoints.collection_id -> api_collections.id`
- `assertions.endpoint_id -> api_endpoints.id`
- `ui_script_steps.script_id -> ui_scripts.id`
- `test_cases.api_endpoint_id -> api_endpoints.id`
- `test_cases.ui_script_id -> ui_scripts.id`
- `test_cases.data_table_id -> data_tables.id`
- `suite_cases.suite_id -> test_suites.id`
- `suite_cases.case_id -> test_cases.id`
- `test_runs.suite_id -> test_suites.id`
- `test_run_results.run_id -> test_runs.id`
- `test_run_results.case_id -> test_cases.id`

### Important JSON/Text Columns

- `api_endpoints.headers_json`
- `api_endpoints.body_json`
- `api_endpoints.auth_config_json`
- `test_cases.tags_json`
- `test_run_results.request_log_json`
- `test_run_results.response_log_json`
- `test_run_results.assertion_results_json`
- `test_run_results.screenshots_json`
- `data_tables.columns_json`
- `data_table_rows.row_json`

### Persistence Constraints

- `test_run_results` chỉ nên lưu summary + sanitized/truncated text payloads
- artifacts lớn phải lưu ngoài filesystem và chỉ tham chiếu bằng path
- screenshot không lưu blob trực tiếp vào SQLite

### Suggested Indexes

- `idx_api_endpoints_collection`
- `idx_assertions_endpoint`
- `idx_ui_steps_script`
- `idx_ui_steps_order`
- `idx_test_cases_type`
- `idx_suite_cases_suite`
- `idx_test_runs_suite`
- `idx_test_runs_status`
- `idx_run_results_run`
- `idx_run_results_case`
- `idx_data_rows_table`
- `idx_env_vars_env`

### Migration Strategy

- `001_initial_schema.sql`
- `002_seed_default_env.sql`

Migrations sẽ chạy khi app khởi động và được bọc trong transaction.

---

## Section 3: UI Layout

### Main Navigation Layout

- **Sidebar trái**: API Tests, Web Tests, Run Tests, Reports, Environments, Data, Settings
- **Tab bar phía trên**: mở nhiều màn hình/test case cùng lúc
- **Status bar phía dưới**: active environment, connection status, running state

### API Tester Screen

Layout 3 vùng:
- Trái: Collection tree
- Giữa trên: Request builder
- Giữa dưới: Response viewer

UX chính:
- Method dropdown màu sắc rõ ràng
- Tabs: Params / Headers / Body / Auth / Scripts
- Assertion builder dạng no-code
- JSON response viewer với syntax highlight
- Assertions tab hiển thị pass/fail chi tiết

### Web UI Recorder Screen

Layout 2 vùng:
- Trái: Browser session view / live session status
- Phải: Step editor

UX chính:
- Record / Stop / Play / Save
- Realtime hiển thị steps khi user thao tác
- QA có thể sửa selector / value / wait time bằng UI
- Manual add step để chỉnh script không cần code

**Clarification**:
- Phase 1 không bắt buộc nhúng browser thật bên trong panel của app.
- Có thể dùng browser window/session riêng do automation engine mở, còn app hiển thị trạng thái, controls và step stream.

### Test Runner Screen

Layout:
- Suite config toolbar
- Progress bar + counters
- Danh sách test case results
- Detail panel khi chọn failed test

UX chính:
- Realtime progress
- Rerun failed
- Detail logs cho assertion, request, response, screenshots

**Phase 1 constraints**:
- Không support pause/resume.
- Chỉ support Run / Stop / Rerun Failed.

### Test Data Manager

UX chính:
- Spreadsheet-like editing
- Password/secret masking
- Import từ CSV/JSON
- Hiển thị test cases nào đang dùng data table đó

### Environment Manager

Layout 2 panel:
- Trái: danh sách environment
- Phải: variables của environment

UX chính:
- Secret masking
- Set default environment
- Auto-complete `{{variable}}` ở request builder và UI scripts

### Settings Screen

Bao gồm:
- Theme / Language / Auto-save
- Browser defaults
- Test runner options
- Storage paths
- Export/import data

**Clarification**:
- Language/i18n framework đầy đủ không phải MVP blocker.
- Theme switching là nice-to-have, không được phép làm chậm core delivery.

### Reports / History Clarification

- Phase 1 không bắt buộc phải có route `/reports` riêng nếu điều đó làm tăng complexity không cần thiết.
- Basic reporting có thể được gộp vào runner history / run detail views miễn là người dùng vẫn xem được summary, details và export HTML/JSON.

### Global UX Patterns

- Tab system nhiều màn hình
- Search palette (Ctrl+K)
- Keyboard shortcuts
- Status bar luôn hiện environment và run state
- Toast notifications
- Resizable panels
- Context menus
- Drag & drop reorder

---

## Section 4: Rust Backend Architecture

### Proposed Module Tree

```
src-tauri/src/
├── main.rs
├── lib.rs
├── error.rs
├── state.rs
├── db/
├── services/
├── commands/
└── utils/
```

### Key Backend Areas

#### `error.rs`
- Unified `AppError`
- Bao phủ DB, HTTP, Assertion, Playwright, Validation, NotFound, Internal
- Serialize được để trả lỗi qua Tauri IPC

#### `state.rs`
- `AppState` chứa shared DB connection, config, active recording session, active run session
- Dùng `Arc<RwLock<...>>` cho shared mutable state

#### `db/`
- `connection.rs`: tạo/kết nối SQLite
- `migrations.rs`: migration runner
- `models/`: mỗi file phụ trách CRUD cho 1 entity

#### `services/`
- `api_engine.rs`: build + send HTTP request
- `assertion_engine.rs`: evaluate assertions
- `variable_resolver.rs`: thay biến `{{name}}`
- `playwright_engine.rs`: browser automation qua playwright-rs
- `recorder.rs`: action recording
- `script_runner.rs`: chạy UI scripts
- `test_runner.rs`: điều phối suite execution
- `export.rs`: sinh report HTML/JSON

#### `commands/`
- `env_commands.rs`
- `api_commands.rs`
- `ui_commands.rs`
- `runner_commands.rs`
- `data_commands.rs`
- `settings_commands.rs`

### IPC Design

Tauri command handlers là lớp mỏng:
- validate input
- gọi service/model phù hợp
- trả result hoặc AppError

Estimated command groups:
- Environment: 5
- API Collection/Endpoint/Assertion: 14+
- UI Script/Recorder: 10+
- Runner: 5
- Data Table: 6
- Settings: 4

### API Engine Responsibilities

- Resolve variables từ environment
- Build reqwest request
- Gửi request, capture status/headers/body/size/time
- Trả kết quả chuẩn hóa cho UI và assertion engine

### API Auth Scope for Phase 1

Supported:
- none
- bearer token
- basic auth
- api key (header/query)

Out of scope:
- OAuth interactive flow
- mutual TLS phức tạp
- custom request signing nâng cao

### Assertion Engine Responsibilities

Hỗ trợ assertion types:
- status
- header
- body_json
- body_text
- response_time

Operators:
- equals / not_equals
- contains / not_contains
- regex
- exists / not_exists
- less_than / greater_than

### Playwright Engine Responsibilities

- Launch browser theo script config
- Open page/context
- Execute steps tuần tự
- Capture screenshot on fail
- Return per-step results

### Important Technical Caveat

`playwright-rs` là **pre-1.0** nên:
- API có thể thay đổi
- Một số methods có thể chưa hoàn chỉnh
- Cần pin version và chuẩn bị workaround/fallback

### Variable Resolver Responsibilities

- Tìm tất cả placeholders `{{variable}}`
- Replace bằng giá trị từ active environment
- Trả lỗi rõ ràng nếu biến chưa được định nghĩa

### Test Runner Responsibilities

- Load suite + cases + environment
- Tạo run record
- Chạy test cases tuần tự hoặc song song giới hạn bằng semaphore
- Lưu từng result
- Cập nhật summary pass/fail/skip
- Emit progress events cho frontend

### Data-Driven Execution Rule

- Nếu test case gắn `data_table`, mỗi row là một execution con
- Mặc định continue toàn bộ rows
- Run summary phải tính theo số executions thực tế sau khi expand rows

### Dependency Rules

- `commands/` chỉ gọi `services/` và `db/models/`
- `services/` không gọi `commands/`
- `db/models/` không phụ thuộc `services/`
- `error.rs` được import bởi mọi nơi

### Cargo-Level Dependency Direction

- Tauri v2
- tokio + tokio-util
- reqwest + rustls
- playwright-rs v0.9
- rusqlite
- serde / serde_json
- thiserror
- regex / chrono / uuid / log

---

## Section 5: Frontend Architecture

### Frontend Module Structure

```text
src/
├── main.tsx
├── App.tsx
├── routes/
├── components/
│   ├── layout/
│   ├── api/
│   ├── recorder/
│   ├── runner/
│   ├── env/
│   ├── data/
│   └── shared/
├── hooks/
├── store/
├── services/
├── types/
├── lib/
└── styles/
```

### Frontend Principles

- Chia theo domain feature thay vì technical split thuần túy
- Tách rõ layers: routes / components / state+hooks / services
- Frontend chỉ giao tiếp với backend qua Tauri IPC boundary

### Routing Strategy

Routes chính:
- `/api`
- `/ui`
- `/runner`
- `/environments`
- `/data`
- `/settings`
- `/reports`

Sử dụng React Router cho route-level structure, còn tab system xử lý entity-level navigation trong từng feature.

### State Management Strategy

3 tầng state:

1. **Local UI state** — `useState` / `useReducer`
   - form input
   - dialog open/close
   - panel resize
   - selected row/item

2. **Shared UI/App state** — Zustand
   - active environment
   - open tabs
   - global layout state
   - running suite status
   - theme / sidebar / command palette

3. **Async server-backed state** — TanStack Query
   - environments
   - collections
   - endpoint detail
   - suites
   - run history

### Proposed Global Stores

- `tabs-store.ts`
- `env-store.ts`
- `run-store.ts`
- `app-store.ts`

### Tauri Client Layer

Tất cả lệnh `invoke()` phải đi qua wrapper typed trong `services/tauri-client.ts` để:
- chuẩn hóa error handling
- logging
- typing response
- tránh IPC gọi rải rác khắp app

Browser-related services phải đi qua abstraction ổn định (ví dụ `BrowserAutomationService`) để giảm coupling trực tiếp với `playwright-rs` internals và giúp fallback dễ hơn.

### Event Handling Strategy

Realtime events dự kiến:
- `recording_step`
- `recording_stopped`
- `test_case_completed`
- `suite_progress`
- `suite_completed`
- `error_occurred`

Frontend dùng hook chung `use-tauri-event.ts` để subscribe/unsubscribe an toàn.

### Component Patterns

- Container vs Presentational separation cho page lớn
- Form-heavy screen dùng `react-hook-form + zod`
- JSON/body editor dùng CodeMirror hoặc Monaco
- Table/list lớn dùng component chuyên dụng thay vì tự dựng từ đầu

### Feature Page Responsibilities

- `/api`: collection tree, endpoint tabs, run request, response viewer
- `/ui`: start/stop recording, realtime step stream, replay, save script
- `/runner`: run suites, progress stream, failed detail panel
- `/environments`: CRUD environments + variables
- `/data`: CRUD tables + rows + import/export
- `/settings`: theme, browser defaults, storage paths, runner config

### Reporting Scope Clarification

- Phase 1 không bắt buộc route reporting quá phức tạp riêng biệt.
- Có thể gộp basic reporting vào runner/history views miễn là vẫn đáp ứng export HTML/JSON và run detail rõ ràng.

### Error UX Model

3 tầng hiển thị lỗi:
- inline validation error
- panel-level error banner
- global toast/dialog cho lỗi nghiêm trọng

### UX/Usability Rules

- ưu tiên label rõ ràng cho QA low-code
- không icon-only cho action quan trọng
- pass/fail/running phải có cả màu và text
- destructive action luôn có confirm dialog
- mỗi screen phải có loading / empty / error / data states rõ ràng

### Frontend Tech Summary

| Concern | Recommendation |
|---|---|
| Framework | React 18 + TypeScript |
| Build tool | Vite |
| Router | React Router |
| UI kit | shadcn/ui |
| Styling | TailwindCSS |
| Forms | react-hook-form + zod |
| Global UI state | Zustand |
| Async data state | TanStack Query |
| Tables | TanStack Table |
| Editor | CodeMirror / Monaco |
| Icons | Lucide |
| Split panes | react-resizable-panels |
| Drag & drop | dnd-kit |

### Frontend Risks

- state bị phân tán sai lớp
- route system và tab system chồng chéo
- embedded browser panel quá phức tạp trong Phase 1
- form builder bị over-engineer quá sớm

Mitigation: giữ feature scope nhỏ, ưu tiên simple UI orchestration trước, không dựng quá nhiều abstraction sớm.

## Section 6: Error Handling & Edge Cases

### Error Handling Goals

- lỗi phải rõ ràng cho QA đọc được
- lỗi phải đủ chi tiết cho dev debug
- không để app crash toàn cục vì lỗi cục bộ
- action thất bại phải có recovery path rõ ràng

### Error Categories

#### 1. Validation Errors

Ví dụ:
- Environment name trống
- URL không hợp lệ
- API method/body không khớp
- Selector rỗng trong UI step
- Duplicate variable key trong cùng environment

Xử lý:
- chặn từ frontend bằng form validation
- backend validate lại lần 2
- hiển thị inline error dưới field

#### 2. Persistence / Database Errors

Ví dụ:
- database file bị lock
- migration fail
- dữ liệu corrupt
- unique constraint conflict

Xử lý:
- retry nhẹ cho lock tạm thời nếu phù hợp
- show dialog lỗi nghiêm trọng khi migration fail
- log đầy đủ context thao tác đang thực hiện
- với conflict thì map sang business message dễ hiểu

#### 3. API Execution Errors

Ví dụ:
- DNS fail
- timeout
- SSL/TLS error
- 401/403/500 response
- invalid JSON response
- redirect loop

Phân biệt rõ:
- **transport error**: request không gửi thành công
- **business/test failure**: request thành công nhưng assertion fail

UI cần hiển thị khác nhau:
- transport error → banner lỗi hệ thống/request
- assertion fail → test result fail với actual vs expected

#### 4. Browser Automation Errors

Ví dụ:
- browser launch fail
- Playwright server unavailable
- browser binary missing
- selector không tìm thấy
- page navigation timeout
- element detached / hidden / blocked
- unsupported playwright-rs API

Xử lý:
- show step nào fail
- chụp screenshot on fail
- lưu selector/action/value tại thời điểm lỗi
- cho phép rerun step/script

#### 5. State / Concurrency Errors

Ví dụ:
- đang recording mà user bấm run suite
- đang chạy suite thì đổi active environment
- mở cùng lúc 2 run conflict resource
- close app giữa chừng khi run đang active

Xử lý:
- app cần rule state machine rõ ràng
- disable action conflict
- xác nhận trước khi hủy tác vụ đang chạy
- persist partially completed run state nếu cần

### Error Presentation Strategy

#### A. User-friendly message

Mỗi lỗi backend nên map sang 2 lớp message:
- `technical_message`: cho log/debug
- `display_message`: cho QA đọc

Ví dụ:
- technical: `Selector '#submit-btn' not found within 5000ms`
- display: `Không tìm thấy nút Submit trong 5 giây. Hãy kiểm tra selector hoặc trạng thái màn hình.`

#### B. Structured error payload

Đề xuất error response shape qua IPC:

```json
{
  "code": "ELEMENT_NOT_FOUND",
  "displayMessage": "Không tìm thấy phần tử cần thao tác.",
  "technicalMessage": "Selector '#submit-btn' not found within 5000ms",
  "context": {
    "selector": "#submit-btn",
    "timeoutMs": 5000,
    "stepOrder": 4
  },
  "recoverable": true
}
```

### Critical Edge Cases

#### 1. Variable Resolution Edge Cases

- variable không tồn tại: `{{token}}`
- variable lồng nhau hoặc circular reference
- empty string nhưng hợp lệ
- secret variable hiển thị/mask sai nơi

Quy tắc:
- missing variable → fail fast, không gửi request
- circular reference → reject khi save hoặc khi resolve
- secret vẫn được resolve đúng nhưng UI phải mask

#### 2. API Body / Assertion Edge Cases

- body type = none nhưng user vẫn nhập body
- response body không phải JSON nhưng assertion dùng JSONPath
- header case-insensitive matching
- status code là 204, không có body
- multipart/form-data phức tạp chưa hỗ trợ đủ ở Phase 1

Quy tắc:
- validation ngay từ builder
- unsupported combination phải báo rõ
- Phase 1 nên giới hạn feature phức tạp nếu chưa đủ chắc

#### 3. Recorder / Step Stability Edge Cases

- selector generated quá mong manh
- element nằm trong iframe
- dynamic IDs thay đổi mỗi lần load
- click gây navigation quá nhanh
- modal/animation làm timing không ổn định

Mitigation:
- ưu tiên selector strategy: data-testid > name > stable CSS > xpath
- sinh wait steps hợp lý khi detect navigation/loading
- flag step “low confidence selector” để QA chỉnh lại
- iframe support nếu có thì explicit, nếu chưa có phải note rõ out-of-scope cho Phase 1

#### 4. Long-running Execution Edge Cases

- suite chạy quá lâu
- browser treo giữa chừng
- app bị minimize hoặc mất focus
- user bấm stop liên tục nhiều lần

Mitigation:
- dùng cancellation token
- stop action phải idempotent
- timeout mặc định + timeout override theo script/case
- emit heartbeat/progress để UI biết engine còn sống

#### 5. Storage / Filesystem Edge Cases

- screenshots folder không ghi được
- export path không tồn tại
- import file sai format
- database nằm ở đường dẫn read-only

Mitigation:
- preflight check trước khi save/export
- create directory nếu thiếu
- import có preview + validation trước commit
- fallback path mặc định trong app data directory

### Recovery Strategy

Khi lỗi xảy ra, hệ thống nên phân loại:

1. **Recoverable, immediate retry được**
   - selector tạm thời chưa xuất hiện
   - DB lock ngắn hạn
   - transient network timeout

2. **Recoverable nhưng cần user action**
   - thiếu variable
   - selector sai
   - invalid environment config

3. **Non-recoverable / blocking**
   - migration fail
   - Playwright runtime missing hoàn toàn
   - database corrupt nghiêm trọng

### Logging Strategy

Mỗi lỗi quan trọng cần log tối thiểu:
- timestamp
- feature/module
- action đang thực hiện
- entity id liên quan
- technical error message
- context payload rút gọn

Không log trực tiếp secret values.

### Safety Rules for Phase 1

- một thời điểm chỉ cho phép 1 recording session active
- stop run phải an toàn nếu bấm nhiều lần
- destructive delete cần confirm
- khi app đóng lúc đang run/recording phải có confirm dialog
- screenshot failure không được làm crash toàn bộ test result pipeline

### Section 6 Output Requirements

Frontend và backend đều phải thống nhất:
- error code taxonomy
- display message format
- retry policy
- screenshot-on-fail policy
- cancellation semantics

---

## Next Sections Pending

- Section 7: Testing Strategy
- Section 8: Timeline & Milestones

## Section 7: Testing Strategy

### Testing Goals

- đảm bảo app ổn định dù team còn mới với Rust và automation testing
- phát hiện lỗi sớm ở đúng layer
- tránh phụ thuộc hoàn toàn vào manual QA
- ưu tiên test các luồng cốt lõi của Phase 1 thay vì cố phủ mọi edge case ngay từ đầu

### Test Pyramid cho dự án này

#### 1. Unit Tests

Áp dụng cho các phần có logic thuần:
- `variable_resolver`
- `assertion_engine`
- validators
- formatter/parser helpers
- error mapping
- selector scoring helper (nếu có)

Mục tiêu:
- chạy nhanh
- không phụ thuộc DB/browser/network thật
- bao phủ các rule nghiệp vụ nhỏ nhưng quan trọng

#### 2. Integration Tests

Áp dụng cho:
- DB CRUD + migrations
- Tauri command ↔ service ↔ model flow
- API engine với mock server
- export/import pipeline
- run suite orchestration với fake/stub services nếu cần

Mục tiêu:
- kiểm tra các module làm việc đúng với nhau
- bắt lỗi wiring/config sớm

#### 3. End-to-End / Desktop Flow Tests

Áp dụng cho các luồng người dùng chính:
- tạo environment
- tạo API endpoint và chạy test
- record UI script và replay
- tạo suite và chạy suite
- xem kết quả fail/pass

Mục tiêu:
- xác nhận sản phẩm usable cho QA team
- test theo góc nhìn người dùng thật

### Testing Layers by Stack

#### Rust Backend

Ưu tiên test:
- `cargo test` cho unit + integration
- test migration khởi tạo DB từ zero
- test command handlers với state giả lập hoặc test harness
- test API engine với HTTP mock server

#### Frontend React

Ưu tiên test:
- component behavior quan trọng bằng Vitest + Testing Library
- form validation
- tab store / environment switching / run state store
- event-driven UI updates khi nhận Tauri event payload giả lập

#### Desktop/E2E

Ưu tiên test:
- smoke flows ở mức app behavior
- nếu test E2E desktop quá nặng trong Phase 1, cho phép bắt đầu bằng browser-based component flow + integration smoke thay vì full desktop automation quá sớm

### Proposed Test Tooling

| Layer | Tool |
|---|---|
| Rust unit/integration | `cargo test` |
| HTTP mocking | `wiremock` hoặc mock server tương đương cho Rust |
| Frontend unit/component | `vitest` + `@testing-library/react` |
| Frontend DOM assertions | Testing Library |
| State store tests | Vitest |
| E2E/smoke app flows | lựa chọn nhẹ cho desktop flow, hoặc smoke harness theo capability thực tế |

### What Must Be Tested in Phase 1

#### A. Variable Resolution

Cases bắt buộc:
- resolve 1 biến
- resolve nhiều biến
- thiếu biến → fail fast
- secret variable vẫn resolve đúng nhưng không lộ ở log/UI
- circular reference bị reject

#### B. Assertion Engine

Cases bắt buộc:
- status equals
- header contains
- body_json JSONPath extract
- body_text contains
- response time compare
- invalid JSONPath/non-JSON body handling

#### C. API Engine

Cases bắt buộc:
- gửi GET/POST cơ bản
- gửi headers/body đúng
- parse JSON response đúng
- non-JSON response vẫn hiển thị được
- timeout handling
- transport error vs assertion failure được phân biệt rõ

#### D. Database / Migration

Cases bắt buộc:
- init DB mới từ zero
- chạy migration nhiều lần không hỏng
- CRUD environment
- CRUD api endpoint + assertions
- CRUD ui script + steps
- tạo suite và lưu run result

#### E. Frontend Critical UX

Cases bắt buộc:
- form validation hoạt động
- đổi environment cập nhật toàn app
- tabs mở/đóng/dirty state đúng
- response viewer render state đúng theo loading/error/data
- progress UI cập nhật khi nhận event

#### F. Recording / Script Execution Core Flow

Cases bắt buộc:
- start recording thành công
- realtime step stream cập nhật UI
- stop recording lưu step list đúng
- replay script success path
- step fail tạo screenshot path và hiển thị fail detail

### Suggested Coverage Priorities

Không đặt mục tiêu “100% coverage”. Ưu tiên thực dụng:

- **High confidence coverage** cho:
  - variable resolver
  - assertion engine
  - migrations
  - run orchestration logic
  - environment switching

- **Selective coverage** cho UI phức tạp:
  - test behavior chính
  - không snapshot mọi component nhỏ

### Test Data Strategy

- dùng fixture rõ nghĩa, nhỏ gọn
- có sample environments: Development / Staging
- có sample API endpoint fixtures
- có sample UI script fixture đơn giản (login flow)
- tránh fixture quá lớn và khó đọc

### Mocking Strategy

#### Backend
- mock HTTP server cho API engine
- stub/fake cho một số browser interactions nếu cần ở service-level tests

#### Frontend
- mock Tauri `invoke`
- mock Tauri event listeners
- mock service layer thay vì mock sâu từng component con

### Test Environment Strategy

- DB test riêng dùng temp SQLite file hoặc in-memory SQLite
- frontend test environment độc lập, không phụ thuộc backend thật
- integration tests cho export/import dùng temp directory
- browser automation tests cần clearly separated từ unit/integration để tránh làm suite chậm toàn cục

### CI Test Order Recommendation

1. Rust format/lint
2. Rust unit tests
3. Rust integration tests
4. Frontend lint/type-check
5. Frontend unit/component tests
6. Selected smoke flows

Nguyên tắc:
- fail fast ở lớp rẻ nhất trước
- smoke flows chỉ chạy sau khi layer dưới đã pass

### Risks in Testing Strategy

#### Risk 1: Desktop E2E quá nặng cho Phase 1
Mitigation:
- chỉ chọn 2-3 smoke flows cực quan trọng
- không cố full regression desktop automation ngay trong MVP

#### Risk 2: Team mới → viết test khó maintain
Mitigation:
- ưu tiên test readable, AAA pattern rõ ràng
- fixture nhỏ, helper ít nhưng hữu ích
- tránh over-mocking

#### Risk 3: playwright-rs chưa ổn định
Mitigation:
- unit/integration logic không phụ thuộc quá sâu vào playwright-rs
- tách browser-specific tests thành nhóm riêng

### Phase 1 Minimum Acceptance Test Set

Ít nhất phải có các smoke scenarios sau:

1. Tạo environment mới → lưu thành công
2. Tạo API endpoint → chạy request → xem assertion pass
3. Tạo API endpoint lỗi assertion → UI hiển thị fail detail đúng
4. Start recording UI flow đơn giản → lưu script
5. Replay script → pass
6. Tạo suite gồm API + UI case → run suite → thấy progress + results
7. Một test fail → screenshot path và detail panel hiển thị đúng

### Testing Philosophy for This Project

- test logic nhiều hơn test implementation detail
- test feature boundaries thay vì snapshot UI quá mức
- ưu tiên confidence cho 20% luồng tạo ra 80% giá trị
- mọi bug critical sau này phải kéo theo thêm regression test tương ứng

## Section 8: Timeline & Milestones

### Planning Assumptions

- team nhỏ
- Rust experience ở mức cơ bản
- automation testing experience gần như từ đầu
- mục tiêu thực tế là **Phase 1 usable MVP** trong khoảng 3 tháng
- advanced enterprise features để sang Phase 2

### High-Level Delivery Strategy

Không đi theo kiểu làm tất cả cùng lúc. Nên đi theo 4 mốc:

1. **Foundation** — dựng khung app và nền tảng kỹ thuật
2. **Core Feature Vertical Slices** — API testing, UI recording, runner
3. **Integration + Hardening** — nối end-to-end, xử lý lỗi, stabilize
4. **MVP Polish** — smoke flows, docs, packaging, handoff

### Recommended 12-Week Timeline

## Milestone 1 — Foundation (Week 1-2)

### Goals
- bootstrap Tauri + React + Rust project structure
- setup SQLite + migrations
- setup IPC command pattern
- setup frontend shell, sidebar, tabs, status bar
- setup testing foundations (Rust + frontend)

### Deliverables
- app khởi động được
- DB init/migration chạy được
- environment CRUD flow cơ bản chạy được end-to-end
- command/service/model skeleton đầy đủ

### Success Criteria
- mở app được ổn định
- tạo/sửa/xóa environment thành công
- active environment hiển thị đúng toàn app
- test/lint cơ bản pass

## Milestone 2 — API Testing Vertical Slice (Week 3-4)

### Goals
- hoàn thành API collections + endpoints CRUD
- request builder usable
- response viewer usable
- assertion engine hoạt động
- run single API test end-to-end

### Deliverables
- collection tree
- request builder tabs (params/headers/body/auth)
- response body/headers/assertions view
- assertion builder no-code
- lưu endpoint + assertions vào DB

### Success Criteria
- QA có thể tạo API test, save, run, xem pass/fail detail
- differentiate transport error vs assertion failure
- environment variables resolve đúng

## Milestone 3 — Web Recorder Vertical Slice (Week 5-6)

### Goals
- tích hợp playwright-rs đủ cho recording cơ bản
- record các actions chính
- step editor usable
- replay script được với flow đơn giản

### Deliverables
- start/stop recording
- realtime step stream lên UI
- lưu script + steps vào DB
- replay script 1 flow đơn giản (ví dụ login)
- screenshot on fail cơ bản

### Success Criteria
- QA record được một flow đơn giản
- chỉnh step bằng UI được
- replay pass/fail có hiển thị step-level result

### Risk Gate

Đây là milestone có rủi ro cao nhất. Cuối week 6 cần checkpoint rõ:
- playwright-rs có đủ capability không?
- browser launch/replay có ổn định không?
- nếu không ổn, phải kích hoạt fallback plan sớm

## Milestone 4 — Test Runner + Data Management (Week 7-8)

### Goals
- hoàn thành test suites
- test run orchestration
- data tables CRUD
- run mixed suite (API + UI)

### Deliverables
- suite editor
- runner screen với progress + detail panel
- data tables editor
- link test cases với data table
- save run history + results

### Success Criteria
- tạo suite gồm API + UI cases
- run suite end-to-end
- thấy progress realtime
- fail detail + screenshot path hiển thị đúng

## Milestone 5 — Error Handling + Hardening (Week 9-10)

### Goals
- hoàn thiện error taxonomy
- improve recovery flows
- stabilize app state conflicts
- add missing validation and empty/error states

### Deliverables
- structured error model áp dụng xuyên suốt app
- retry/stop/cancel flows ổn định
- destructive confirm dialogs
- close-app safety checks
- import/export cơ bản

### Success Criteria
- app không crash ở các failure path phổ biến
- QA hiểu được lỗi và biết phải làm gì tiếp
- smoke scenarios pass với cả happy path và fail path

## Milestone 6 — MVP Polish & Release Candidate (Week 11-12)

### Goals
- hoàn thiện smoke test set
- cleanup UX inconsistencies
- package app nội bộ
- chuẩn bị tài liệu sử dụng nội bộ

### Deliverables
- build release nội bộ
- stable smoke test flows
- sample data/env/scripts
- onboarding guide cho QA team

### Success Criteria
- demo được toàn bộ flow Phase 1
- QA team có thể dùng tool cho use case đơn giản mà không cần dev đứng cạnh
- known issues được liệt kê rõ

### Parallel Work Recommendation

Trong team nhỏ, nên chia vai theo track thay vì chia ngẫu nhiên:

#### Track A — Frontend UX
- app shell
- API builder UI
- recorder UI
- runner UI
- settings/data manager

#### Track B — Rust Backend / Data
- DB schema + migrations
- models + services
- API engine
- runner orchestration
- error model

#### Track C — Browser Automation Spike / Stabilization
- playwright-rs integration
- recording/replay reliability
- selector strategy
- screenshot capture

Track C nên bắt đầu sớm, không chờ tới giữa dự án mới làm, vì đây là technical risk lớn nhất.

### Weekly Review Cadence

Khuyến nghị mỗi tuần có 1 review checkpoint:

- tuần này build được gì?
- blocker kỹ thuật là gì?
- playwright-rs stability thế nào?
- có feature nào đang bị over-scope không?
- smoke flow nào đã pass / chưa pass?

### Scope Control Rules

Để giữ MVP đúng timeline, các thứ sau **không nên nhét vào Phase 1** nếu chưa thật cần:

- advanced analytics dashboard
- scheduling/cron runner
- CI/CD integrations
- team permissions / multi-user collaboration
- visual diff engine phức tạp
- flaky test healing thông minh
- full iframe/shadow DOM coverage nếu chưa chắc

### Go / No-Go Gates

#### Gate 1 — End of Week 2
- nền app có chạy ổn không?
- DB + IPC + shell có chắc không?

#### Gate 2 — End of Week 6
- recorder/replay có usable không?
- playwright-rs có đủ ổn định không?

#### Gate 3 — End of Week 8
- mixed suite run có chạy end-to-end không?
- result model có đủ cho QA dùng không?

#### Gate 4 — End of Week 10
- fail path có ổn định không?
- app có crash dưới các lỗi phổ biến không?

Nếu một gate fail nặng, cần cắt scope thay vì cố nhồi thêm feature.

### MVP Exit Criteria

Phase 1 được xem là hoàn tất khi:

1. QA tạo được environment và variables
2. QA tạo và chạy được API tests với assertions
3. QA record được ít nhất một UI flow đơn giản và replay thành công
4. QA gom API + UI cases vào suite và chạy được suite
5. fail/pass detail đủ rõ để QA tự đọc
6. app không crash ở các lỗi phổ biến
7. smoke test set pass ổn định

### Phase 2 Preview (Not in MVP)

Sau khi MVP ổn định, mới mở Phase 2 cho:
- advanced reporting
- scheduling
- CI/CD integration
- collaboration
- richer browser support stabilization

## Section 9: Security & Privacy

### Security Goals

- bảo vệ secrets và test credentials khỏi bị lộ trong UI, logs và exports
- giới hạn tối đa tác động của desktop app lên hệ thống người dùng
- tránh biến tool test thành nguồn rò rỉ dữ liệu thật
- đảm bảo lỗi, report, screenshot không vô tình chứa thông tin nhạy cảm ngoài ý muốn

### Locked Security Baseline for Phase 1

- dùng app-managed encryption baseline cho secrets at rest
- secret values phải bị redact trong logs và export mặc định
- browser/runtime failure không được làm lộ raw credentials trong error output

### Threat Model (Phase 1)

Không nhắm tới enterprise-grade zero-trust ngay từ đầu, nhưng phải xử lý tốt các rủi ro thực tế sau:

1. QA lưu API keys / tokens / mật khẩu vào environment variables
2. Request/response logs chứa PII hoặc secrets
3. Screenshot UI chứa dữ liệu người dùng thật
4. Exported reports bị chia sẻ nội bộ không kiểm soát
5. App logs ghi ra token/password raw
6. Local SQLite database bị đọc trực tiếp từ máy người dùng

### Secret Handling Rules

#### Secrets bao gồm:
- API keys
- bearer tokens
- passwords
- cookie/session values
- private endpoints hoặc tenant-specific identifiers nhạy cảm

#### Rules bắt buộc:
- secrets phải có cờ `is_secret`
- UI mặc định **mask** secret values
- logs không bao giờ ghi raw secret
- exported report mặc định phải redact secret
- copy secret phải là explicit action, không tự hiển thị toàn phần

### Storage Strategy for Secrets

Phase 1 khuyến nghị mức bảo vệ thực dụng:

#### Option được khuyến nghị
- lưu secrets trong SQLite nhưng **encrypt at rest** trước khi persist
- key mã hóa lấy từ app-local config + OS-backed secret storage nếu khả thi

#### Nếu OS keychain integration chưa kịp cho Phase 1
- vẫn phải mã hóa secrets bằng app-managed key
- chấp nhận đây là giải pháp trung gian, cần note rõ limitation

### Data Classification

Đề xuất chia dữ liệu thành 3 lớp:

#### 1. Non-sensitive
- collection names
- test names
- viewport config
- timeout values

#### 2. Sensitive operational
- base URLs nội bộ
- headers tùy chỉnh
- environment variables thường
- request payloads có dữ liệu test

#### 3. Highly sensitive
- passwords
- auth tokens
- API keys
- cookies/session IDs
- response body chứa PII thật

### Logging Policy

#### Được log:
- timestamp
- feature/module
- action name
- status/result
- duration
- entity id/name không nhạy cảm
- technical error code

#### Không được log raw:
- password
- token
- api_key
- cookie/session
- authorization header full value

#### Masking examples
- `Authorization: Bearer eyJ...` → `Authorization: Bearer ***redacted***`
- `password: secret123` → `password: ***`

### Report / Export Redaction Rules

Mặc định khi export:
- redact secret variables
- redact auth headers
- redact cookie values
- cho phép include/exclude response body đầy đủ bằng option rõ ràng

Report HTML/JSON mặc định nên ưu tiên:
- assertion results
- status/time/error summary
- sanitized request/response preview

không nên mặc định dump toàn bộ raw body nếu có nguy cơ chứa PII.

### Screenshot Privacy Rules

Screenshot là nguồn rò rỉ dữ liệu rất thực tế.

Rules:
- screenshot on fail được bật theo cấu hình rõ ràng
- screenshot path phải nằm trong app data directory hoặc export directory có kiểm soát
- report export không tự embed toàn bộ screenshot nếu user chưa chọn
- cần cảnh báo QA: môi trường production có thể chứa dữ liệu thật

### Video Capture Scope

- Phase 1 chỉ **bắt buộc** screenshot-on-fail.
- Video recording không phải MVP requirement.
- Nếu xuất hiện trong UI/settings, nó chỉ được xem là optional future enhancement chứ không phải blocker để release.

### Environment Safety Rules

Để giảm rủi ro thao tác nhầm với production:

- environment phải có nhãn rõ: Development / Staging / Production
- nếu active environment là Production:
  - hiển thị badge đỏ rõ ràng
  - các action destructive hoặc suite run nên có thêm confirm
- Phase 1 khuyến nghị không bật Production làm default environment

### IPC / Desktop Security Principles

- frontend chỉ gọi đúng whitelist Tauri commands
- không mở shell access tùy ý cho user scripts trong Phase 1
- không cho arbitrary file write ngoài các path được app quản lý nếu không cần thiết
- import file phải validate format trước khi parse sâu

### Dependency & Supply Chain Safety

- pin version cho `playwright-rs`
- review dependency list nhỏ nhất có thể
- tránh thêm package frontend không thật sự cần
- định kỳ rà soát crate/npm package có maintenance tốt không

### Privacy Messaging for QA Team

App cần nói rõ với user nội bộ:
- dữ liệu test nào đang được lưu local
- screenshot/log có thể chứa dữ liệu nhạy cảm
- production environment cần dùng cẩn trọng

### Minimum Security Acceptance Criteria for Phase 1

1. secret variables bị mask trong UI
2. logs không chứa token/password raw
3. export mặc định redact secrets
4. production environment hiển thị cảnh báo rõ ràng
5. secrets được encrypt at rest ở mức tối thiểu chấp nhận được
6. import/export path được kiểm tra an toàn trước khi thao tác

## Section 10: Packaging & Distribution

### Distribution Goals

- QA cài đặt được dễ dàng
- update có kiểm soát
- giảm support burden cho team dev
- phản ánh đúng thực tế bundle size của stack Tauri + Playwright

### Package Size Reality

Tổng package thực tế không chỉ là Tauri binary. Cần tính cả:
- Tauri app binary
- frontend assets
- Playwright runtime/server
- browser binaries

Estimate thực tế:
- app shell + assets: vài chục MB
- browser/runtime: phần nặng nhất
- nếu bundle đủ 3 browsers, tổng size có thể lên khoảng 300-450MB+

### MVP Packaging Recommendation

Để giảm complexity ở Phase 1:
- **officially support Chromium trước**
- chưa cần bundle/test đầy đủ Firefox + WebKit ngay trong MVP

Lợi ích:
- giảm package size
- giảm maintenance matrix
- giảm số lượng failure modes khi phân phối nội bộ

Nếu browser runtime gặp lỗi, app vẫn phải cho phép người dùng tiếp tục dùng các tính năng không phụ thuộc browser như API testing, environment management và data management.

### Platform Strategy

Phase 1 ưu tiên:
- **Windows internal distribution first**

Lý do:
- giảm build/test matrix
- tập trung vào môi trường nội bộ chính
- có thể mở rộng multi-platform sau khi MVP ổn định

### Install/Data Separation

Nên tách:

```text
Install Directory/
├── app binary
├── frontend assets
└── bundled runtime/resources

User Data Directory/
├── testforge.db
├── screenshots/
├── exports/
├── logs/
└── config/
```

Nguyên tắc:
- update app không làm mất user data
- backup DB dễ hơn
- permission boundaries rõ ràng hơn

### Versioning Strategy

Khuyến nghị semantic versioning:
- `0.1.0` — internal alpha
- `0.2.0` — feature-complete MVP candidate
- `1.0.0` — stable internal release

### Update Strategy

Phase 1 khuyến nghị:
- **manual update có kiểm soát**

Tức là:
- build package mới
- QA cập nhật theo release nội bộ
- chưa cần auto-update flow trong MVP

Lý do:
- giảm complexity
- chưa cần hạ tầng update server/code-signing phức tạp

### Installer / First-run Requirements

Installer hoặc first-run flow phải đảm bảo:
- init database
- run migrations
- tạo app data directories
- seed dữ liệu mẫu tối thiểu nếu cần
- verify browser/runtime availability

Không nên yêu cầu QA tự cài browser runtime thủ công bằng command line nếu có thể tránh.

### Release Channel Strategy

Đề xuất 2 channel nội bộ:
- **Internal Alpha** — cho core team kiểm thử sớm
- **Internal Beta** — mở rộng cho QA team sau khi smoke flows đã ổn

### Distribution Risks

- bundle quá lớn
- first-run fail vì runtime/browser missing
- update làm hỏng DB user
- QA không biết mình đang dùng version nào

Mitigation:
- support Chromium trước
- preflight runtime checks
- migration/versioning chặt chẽ
- hiển thị version rõ ở Settings/About

### Packaging Acceptance Criteria for Phase 1

1. QA cài app được mà không cần dev tooling
2. app tự init DB/data directories đúng
3. browser runtime usable ngay sau install
4. version hiển thị rõ trong app
5. update thủ công không làm mất user data
6. first-run failure nếu có phải có hướng dẫn rõ ràng

## Section 11: Fallback Plan for Browser Automation Risk

### Why a Fallback Plan Is Required

`playwright-rs` là rủi ro kỹ thuật lớn nhất của kiến trúc hiện tại vì:
- pre-1.0
- ecosystem nhỏ
- coverage chưa hoàn toàn đầy đủ
- có nguy cơ blocker ở recording/replay thực tế

Nếu không chuẩn bị fallback plan từ đầu, dự án có thể bị kéo trễ toàn bộ bởi browser automation layer.

### Trigger Conditions for Fallback Activation

Fallback cần được kích hoạt nếu xảy ra một hoặc nhiều điều sau:

#### Capability Gap
- thiếu API quan trọng cho recorder/replay
- browser launch hoặc screenshot không usable

#### Reliability Gap
- replay quá flaky với flow đơn giản
- crash/runtime cleanup không ổn định

#### Maintenance Gap
- blocker nghiêm trọng không có hướng xử lý rõ
- API changes làm team không theo kịp

#### Timeline Gap
- tới cuối **Week 6** vẫn chưa có record + replay usable cho flow đơn giản

### Fallback Options

#### Fallback 1 — Reduce Scope, Keep `playwright-rs`

Giữ nguyên stack nhưng giảm scope browser automation:
- chỉ support Chromium
- chỉ support subset actions cơ bản
- bỏ advanced cases khỏi Phase 1

Đây là fallback nhẹ nhất, ít rewrite nhất.

#### Fallback 2 — Replace `playwright-rs` with `chromiumoxide`

Giữ Tauri + Rust nhưng đổi browser automation engine sang `chromiumoxide`.

Ưu điểm:
- Rust-native hơn
- maintenance/community tốt hơn
- không phụ thuộc Node.js runtime của Playwright

Nhược điểm:
- phải rewrite browser layer
- mất Playwright-style API familiarity
- Phase 1 gần như chỉ còn Chromium-first

#### Fallback 3 — Remove Live Recording, Keep Script Execution

Nếu recording quá khó nhưng execution vẫn khả thi:
- bỏ live recording ở Phase 1
- dùng manual step builder/editor
- vẫn replay script được

Đây là fallback rất thực tế để vẫn ship giá trị MVP mà không bị block bởi phần khó nhất.

#### Fallback 4 — Ship API-First MVP

Nếu browser automation track kéo chậm toàn bộ dự án:
- chuyển Web UI automation sang Phase 2
- Phase 1 chỉ ship API testing + suites + data + reporting cơ bản

Đây là fallback mạnh nhất nhưng an toàn nhất cho timeline.

### Recommended Fallback Priority

1. Reduce scope, keep `playwright-rs`
2. Remove live recording, keep script execution
3. Replace with `chromiumoxide`
4. Ship API-first MVP

### Fallback Decision Tree

```text
Week 6 checkpoint
│
├─ If playwright-rs works but struggles on advanced cases
│  └─ Reduce scope, keep playwright-rs
│
├─ If execution is possible but recording is unstable
│  └─ Remove live recording, keep script execution
│
├─ If playwright-rs is fundamentally blocked but Rust browser automation is still viable
│  └─ Replace with chromiumoxide
│
└─ If browser automation endangers whole MVP timeline
   └─ Ship API-first MVP
```

### Architectural Preparation Required from Day 1

Để fallback khả thi, kiến trúc phải chuẩn bị trước:

- frontend không phụ thuộc trực tiếp vào Playwright internals
- tạo abstraction `BrowserAutomationService`
- step model trong DB phải trung lập, không gắn cứng Playwright-specific details quá sâu
- tách rõ `recorder` và `script_runner`
- event payload contract phải ổn định dù đổi engine bên dưới

### Recommended Management Rule

Nếu tới cuối **Week 6** mà không đạt được cả 3 điều kiện sau:
- record flow đơn giản usable
- replay flow đơn giản ổn định
- screenshot on fail usable

thì **bắt buộc kích hoạt fallback**, không kéo dài vô hạn vì sunk-cost.

### Fallback Success Criteria

Fallback được xem là thành công nếu:
1. không phải rewrite toàn bộ app shell
2. không phải thay đổi lớn database schema
3. frontend chỉ sửa giới hạn ở browser-related modules
4. timeline MVP vẫn cứu được
5. QA vẫn nhận được sản phẩm usable trong Phase 1
