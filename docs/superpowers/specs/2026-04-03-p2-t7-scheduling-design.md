# P2-T7 Local Scheduling + Unattended Suite Execution Baseline Design

## Goal

Mở rộng trực tiếp `src/routes/test-runner.tsx` và backend runner hiện có để cung cấp baseline local scheduling cho suite execution không cần thao tác tay tại thời điểm trigger, đồng thời buộc mọi scheduled run đi qua đúng runner/orchestration/artifact pipeline hiện tại thay vì tạo job platform hoặc execution path riêng.

## Scope

### In scope
- Mở rộng `test-runner` làm scheduling surface chính thay vì tạo route riêng.
- Thêm local schedule CRUD tối thiểu cho suite, gồm create/update, enable/disable, inspect state.
- Lưu persisted schedule definition trong SQLite bằng migration mới.
- Tính và hiển thị `last run`, `next run`, enabled state, và failure diagnostics mức baseline.
- Kích hoạt scheduled run khi app desktop đang chạy và scheduler loop đang active.
- Reuse `RunnerOrchestrationService`, persisted run history, run detail, artifact manifests, và runner events hiện có.
- Gắn machine-attributed metadata tối thiểu cho scheduled run để phân biệt với manual run mà không tạo parallel run model.
- Giữ diagnostics và preview secret-safe.

### Out of scope
- Không tạo distributed worker system, remote scheduler, hoặc background service ngoài vòng đời app hiện tại.
- Không hỗ trợ lịch chạy khi app đã tắt hoặc chưa mở.
- Không thêm tray/background resident UX, minimize-to-tray, hay startup-on-login flow trong P2-T7.
- Không tạo scheduler execution pipeline riêng tách khỏi runner orchestration.
- Không xây cron platform đầy đủ, calendar UI phức tạp, hay queue/retry engine đa tầng.
- Không thêm route scheduling riêng nếu baseline có thể nằm trong `test-runner`.

## Existing Seams To Reuse

### Frontend
- `src/routes/test-runner.tsx` đã có suite selection, run history/detail, rerun/cancel, reporting summaries; đây là surface phù hợp để gắn scheduling panel.
- `src/services/runner-client.ts` đã là typed seam cho runner execution/history/detail và phải tiếp tục giữ frontend khỏi raw `invoke()` leakage.
- `src/store/run-store.ts` đã subscribe `runner.execution.*` events; scheduled run khi app đang mở phải reuse event feed này thay vì tạo feed mới.
- `src/App.tsx` + `shell.metadata.get` đã có startup metadata pattern; nếu cần schedule status tổng quát, phải bám pattern typed shell/runtime state hiện có.

### Backend
- `src-tauri/src/services/runner_orchestration_service.rs` là execution seam duy nhất cho suite run; scheduler chỉ được trigger vào seam này.
- `src-tauri/src/repositories/runner_repository.rs` đã persist `test_runs`, `test_run_results`, history/detail, artifact linkage; scheduled run outcomes phải đi vào cùng read model này.
- `src-tauri/src/state.rs` đã có explicit `RunState` với single-active-run guard; scheduler phải tôn trọng guard này để tránh duplicate/parallel execution ngoài scope.
- `src-tauri/src/lib.rs` đã có bootstrap + Tauri command registration; scheduler startup loop và schedule commands phải được wiring ở đây.
- `src-tauri/src/db/migrations.rs` + existing migrations đã là persistence seam chuẩn để thêm schedule table mới.
- `src-tauri/src/services/artifact_service.rs` tiếp tục là seam cho artifact/report metadata và redaction-safe preview/export shaping.

## Proposed UX

### Scheduling layer inside `test-runner`
Thêm một scheduling section rõ ràng trong `test-runner`, cạnh runner controls hoặc reporting/history area, với phạm vi vừa đủ cho internal ops:

1. **Schedule form**
   - Suite selector (default theo suite đang chọn nếu có)
   - Environment selector cho scheduled execution
   - Schedule definition tối thiểu, ưu tiên một baseline dễ verify như local recurring minute/time-based cấu hình đơn giản
   - Enable/disable toggle
   - Save/update action

2. **Schedule status card**
   - Enabled / disabled state
   - Last run timestamp
   - Next run timestamp
   - Last run status
   - Last failure diagnostic summary nếu lần chạy gần nhất fail hoặc không trigger được

3. **Execution visibility**
   - Scheduled run xuất hiện trong cùng run history/detail/reporting surface hiện tại
   - Khi app đang mở, live progress vẫn đi qua runner event/status hiện có

4. **Operational honesty states**
   - Nếu app ở degraded/runtime-blocked state khiến UI targets không chạy được, scheduling panel phải hiển thị diagnostics rõ ràng thay vì giả thành công
   - Disabled schedule phải hiển thị rõ là không trigger
   - Nếu đến giờ trigger nhưng đang có run khác hoạt động, phải surface trạng thái skipped/blocked/deferred rõ ràng thay vì l silently spawn execution mới

## Data Design

### Preferred approach
Thêm persisted schedule definition tối thiểu bằng bảng riêng, nhưng **không** thêm run persistence model riêng. Scheduled execution vẫn tạo `test_runs`/`test_run_results`/artifacts bằng pipeline hiện có.

### Required persisted schedule shape
P2-T7 cần một schedule definition có thể lưu và hydrate qua app restart. Shape tối thiểu nên bao gồm:
- `id`
- `suite_id`
- `environment_id`
- `enabled`
- recurrence/time definition tối thiểu đủ cho baseline local trigger
- `last_run_at`
- `next_run_at`
- `last_run_status`
- `last_error`
- timestamps tạo/cập nhật

### Execution attribution
Scheduled run cần machine-attributed metadata tối thiểu để QA/history biết đây là scheduled execution. Điều này nên được thêm bằng cách mở rộng DTO/persistence hiện có ở mức nhỏ nhất cần thiết, ví dụ source/trigger metadata gắn vào run summary, thay vì tạo bảng run parallel hoặc history route riêng.

### Scheduler loop
- Loop chỉ hoạt động khi app desktop đang sống.
- Loop được khởi tạo từ backend bootstrap trong `lib.rs`.
- Loop định kỳ đọc persisted schedules enabled, xác định schedule đến hạn, và trigger execution qua `RunnerOrchestrationService`.
- Nếu `RunState` đang bận, scheduler không được bypass guard; phải cập nhật diagnostics/state cho schedule theo cách honest và inspectable.

## Security / Redaction Rules

- Scheduled diagnostics, status text, exported evidence, và previews không được chứa raw secrets.
- Scheduled run khi đi vào normal run history/detail phải tiếp tục chỉ hiển thị sanitized request/response/assertion previews như manual runs.
- Không log raw secret values vào schedule error field hoặc runtime evidence.
- P2-T7 không được phá redaction guardrails đã khóa ở P2-T5 và re-verified ở P2-T6.

## Error Handling

- Schedule definition invalid → fail rõ ràng ở typed boundary, không silent fallback.
- Schedule disabled → không trigger execution và phải reflect state rõ ràng trong UI/status.
- Trigger tới hạn nhưng app đang có active run → không spawn run mới; lưu/actionable diagnostic phù hợp.
- Suite hoặc environment đã bị xoá/không còn hợp lệ → schedule status phải báo invalid configuration rõ ràng.
- Scheduled execution fail do runner/runtime/browser issue → failure diagnostic phải inspectable từ scheduling surface và/hoặc run history.
- App restart/bootstrap lại → persisted schedule vẫn được hydrate; không duplicate run cho cùng trigger window vì scheduler loop restart.

## Testing Strategy

### Frontend
- Tạo `tests/frontend/scheduler-route-p2.test.ts` để khóa:
  - scheduling UI nằm trong `test-runner`, không phải route riêng,
  - typed client/contracts cho schedule CRUD/read,
  - schedule status copy (`enabled`, `disabled`, `last run`, `next run`, diagnostics),
  - schedule section vẫn reuse runner/history surface thay vì execution model mới.
- Cập nhật `tests/frontend/test-runner-t16.test.ts` nếu cần để khóa schedule panel xuất hiện cạnh runner/history seams hiện có.

### Backend / migration / orchestration
- Tạo `tests/rust/scheduler_service_p2.rs` cho:
  - migration tạo schedule table,
  - enable/disable + load persisted schedules,
  - due schedule selection,
  - active-run guard behavior,
  - diagnostics update cho invalid/blocked/failed trigger path.
- Nếu mở rộng run metadata để đánh dấu scheduled source, phải có regression test cho persisted mapping tương ứng.

### Runtime / unattended evidence
- Phải có evidence cho scheduled execution thật trong desktop runtime đang chạy, không chỉ source-level assertions.
- Nếu harness chưa thể tự chứng minh runtime trigger, phải dùng smoke/evidence pattern trung thực kiểu `PASS/BLOCKED/FAIL`, không claim runtime success khi thiếu marker thực.
- Phải có ít nhất một runtime failure/blocked evidence scenario chứng minh scheduled diagnostics là truthful, inspectable, và secret-safe khi trigger không chạy được hoặc execution fail.

### Regression
- Chạy lại `tests/frontend/suite-runner-t15.test.ts`.
- Chạy lại `tests/frontend/test-runner-t16.test.ts`.
- Chạy lại `tests/frontend/reliability-hardening-t18.test.ts`.
- Chạy lại `tests/frontend/security-export-redaction-p2.test.ts` nếu diagnostics/export path chạm redaction-sensitive fields.

### Evidence
- `.sisyphus/evidence/p2-task-T7-scheduled-run.txt`
- `.sisyphus/evidence/p2-task-T7-schedule-disable.txt`
- `.sisyphus/evidence/p2-task-T7-schedule-failure.txt`

## QA Scenarios

### Scenario: Scheduled suite executes unattended
- Tool: Bash + interactive_bash
- Preconditions: One enabled schedule exists for a runnable suite while desktop runtime is open
- Steps:
  1. Start desktop runtime with scheduler active
  2. Wait for the scheduled trigger window
  3. Inspect resulting run history, run detail, and artifacts
- Expected Result: Scheduled run appears through the normal runner/history pipeline with machine-attributed metadata and standard artifacts
- Failure Indicators: No run created, duplicate runs, or corrupted history/detail persistence
- Evidence: `.sisyphus/evidence/p2-task-T7-scheduled-run.txt`

### Scenario: Disabled schedule does not execute
- Tool: Bash
- Preconditions: Existing schedule is disabled
- Steps:
  1. Wait through the trigger window
  2. Inspect run history and schedule status surface
- Expected Result: No new run is created and schedule state remains clearly disabled
- Failure Indicators: Unexpected execution or stale/ambiguous status surface
- Evidence: `.sisyphus/evidence/p2-task-T7-schedule-disable.txt`

### Scenario: Failed or blocked scheduled trigger surfaces actionable diagnostics
- Tool: Bash + interactive_bash
- Preconditions: An enabled schedule exists, but its suite/environment/runtime preconditions are intentionally invalid or blocked
- Steps:
  1. Start desktop runtime with scheduler active
  2. Wait for the scheduled trigger window
  3. Inspect scheduling status, resulting run history if any, and diagnostic output
  4. Verify no raw secret values appear in diagnostics or previews
- Expected Result: The scheduling surface reports a truthful failed/blocked state with actionable diagnostics; any created run remains inspectable through normal runner detail without secret leakage
- Failure Indicators: Silent non-execution, fake success, uninspectable failure reason, or raw secret exposure
- Evidence: `.sisyphus/evidence/p2-task-T7-schedule-failure.txt`

## Acceptance Mapping

1. **Users can create, enable/disable, and inspect a local schedule for a suite**
   - Delivered by persisted schedule definition + embedded scheduling panel in `test-runner` + typed schedule client/commands.

2. **Scheduled runs execute through the normal runner pipeline and produce standard history/artifacts**
   - Delivered by scheduler loop calling `RunnerOrchestrationService` and reusing `RunnerRepository`/artifact persistence.

3. **Failed scheduled runs surface actionable diagnostics**
   - Delivered by persisted schedule status/diagnostic fields plus normal run failure detail/history when execution actually starts.

## Implementation Direction

Thực hiện P2-T7 như một **scheduler trigger layer mỏng** bám trên runner hiện có: embed UI vào `test-runner`, thêm schedule persistence tối thiểu trong SQLite, khởi tạo local scheduler loop từ backend bootstrap, và trigger suite execution qua `RunnerOrchestrationService`. Mọi thay đổi phải ưu tiên reuse runner/history/detail/artifact/event seams hiện tại; phần mới chỉ là schedule definition + trigger/lifecycle glue cần thiết để chạy unattended khi app đang mở.
