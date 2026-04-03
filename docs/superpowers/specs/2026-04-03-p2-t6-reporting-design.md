# P2-T6 Advanced Reporting Baseline Design

## Goal

Mở rộng trực tiếp `src/routes/test-runner.tsx` để cung cấp baseline reporting thực dụng cho QA leads, gồm filterable run history, grouped summaries, artifact-backed failure drilldown, và trend-ready aggregates nhẹ, đồng thời giữ nguyên redaction/export guardrails hiện có.

## Scope

### In scope
- Mở rộng màn `test-runner` hiện tại thay vì tạo reporting product surface riêng.
- Bổ sung filtering theo suite, status, và date range cho historical runs.
- Bổ sung grouped summaries dựa trên persisted run data.
- Bổ sung failure grouping dựa trên `failureCategory` hiện có.
- Bổ sung artifact-backed drilldown cho failed runs.
- Bổ sung trend-ready aggregate dữ liệu ở mức nhẹ, không thành dashboard analytics.
- Giữ secret-safe previews và report/export behavior.

### Out of scope
- Không tạo route reporting riêng nếu không bị ép bởi complexity.
- Không tạo reporting backend/service độc lập với runner read model.
- Không xây charting platform hoặc analytics dashboard đầy đủ.
- Không thêm execution model mới song song với runner history/detail hiện có.
- Không thêm flow export/report mới hoặc format export mới trong P2-T6.

## Existing Seams To Reuse

### Frontend
- `src/routes/test-runner.tsx` là reporting surface chính cần mở rộng.
- `src/services/runner-client.ts` là typed client seam cho run history/detail.
- `src/types/dto.ts` đã có `RunHistoryEntryDto`, `RunDetailDto`, `RunCaseResultDto`, `ArtifactManifestDto`, `ReportExportDto`.
- `src/store/run-store.ts` tiếp tục giữ live run state, không trở thành nơi tính toán reporting persistence.

### Backend
- `src-tauri/src/repositories/runner_repository.rs` là read-model seam chính cho history/detail/filter/group aggregates.
- `src-tauri/src/services/artifact_service.rs` là seam cho redaction-safe artifact/report shaping khi cần.
- `src-tauri/src/contracts/dto.rs` và `src-tauri/src/contracts/commands.rs` phải tiếp tục mirror typed frontend contract.
- `src-tauri/src/lib.rs` chỉ mở rộng command surface nếu dữ liệu cần cho P2-T6 chưa thể lấy từ command hiện có.

## Proposed UX

### Reporting layer inside `test-runner`
Thêm một vùng reporting rõ ràng ngay trong `test-runner` screen, phía trên hoặc cạnh historical run list:

1. **Filter bar**
   - Suite selector
   - Status selector
   - Date range selector
   - Reset filters action

2. **Summary cards**
   - Total runs in current filter set
   - Passed / failed / cancelled counts
   - Failure-category distribution tóm tắt

3. **Run history results**
   - Filtered list vẫn là điểm vào drilldown chính
   - Selection behavior tiếp tục hydrate `RunDetailDto`

4. **Failure drilldown**
   - Group failed cases theo `failureCategory`
   - Hiển thị sanitized request/response/assertion previews hiện có
   - Giữ artifact links hiện có để điều tra nhanh

5. **Trend-ready aggregates**
   - Recent status distribution
   - Failure category distribution trong filtered set
   - Nhẹ, text/card/table-based; chưa cần chart phức tạp

## Data Design

### Preferred approach
Ưu tiên tái sử dụng `runner.run.history` và `runner.run.detail` với mở rộng tối thiểu.

### If current payloads are insufficient
Cho phép bổ sung nhẹ DTO/query fields để backend trả thêm aggregate/filter metadata, nhưng phải tuân thủ:
- không duplicate `RunDetailDto` bằng reporting DTO song song nếu chỉ khác presentation,
- không tạo persistence path mới,
- không thêm sensitive raw payload fields.

### Candidate additions
- Filter params cho history query nếu hiện tại mới hỗ trợ suite-only.
- Aggregate payload nhỏ gọn cho filtered set nếu frontend không nên tự tính hết từ pageless history.
- Failure grouping metadata nếu có thể trả ra từ repository layer gọn hơn.

## Security / Redaction Rules

- Tiếp tục chỉ render sanitized previews từ dữ liệu runner/repository hiện có.
- Không hiển thị raw request body, ciphertext, masked preview internals, hoặc secret-like export content.
- P2-T6 không mở rộng capability export; nếu UI tham chiếu tới artifact/report export đã tồn tại thì phải giữ nguyên redaction logic hiện có trong `artifact_service.rs`.
- Không phá các regression guardrail đã thêm ở P2-T5.

## Error Handling

- Không có run data phù hợp filter → empty state rõ ràng.
- Run detail thiếu artifact → hiển thị trạng thái thiếu artifact, không giả lập link.
- Filter payload invalid → fail rõ ràng ở typed boundary, không silent fallback khó đoán.
- Backend query mở rộng phải trả lỗi có nghĩa nếu date range/status không hợp lệ.

## Testing Strategy

### Frontend
- Cập nhật `tests/frontend/test-runner-t16.test.ts` để khóa reporting/filter/drilldown behavior mới nếu phù hợp.
- Tạo `tests/frontend/reporting-route-p2.test.ts` cho grouped summaries, filters, trend-ready aggregate presentation.

### Backend / failure-path
- Nếu `runner_repository.rs` được mở rộng cho filter/query validation hoặc aggregate shaping, phải thêm automated coverage cho invalid filter/date/status path.
- Nếu UI hỗ trợ missing-artifact drilldown state, phải có automated test cho case artifact metadata thiếu hoặc không resolve được.
- Phải có ít nhất một automated error-path test xác nhận reporting surface vẫn secret-safe khi dữ liệu chứa preview/export-like sensitive fragments.

### Regression
- Chạy lại `tests/frontend/export-artifact-t10.test.ts`.
- Chạy lại `tests/frontend/security-export-redaction-p2.test.ts`.

### Evidence
- `.sisyphus/evidence/p2-task-T6-reporting-filters.txt`
- `.sisyphus/evidence/p2-task-T6-reporting-redaction.txt`

## Acceptance Mapping

1. **Users can filter runs by suite/status/date range**
   - Delivered by filter bar + filtered history query/view model.

2. **Reporting surface shows grouped summaries and artifact-backed failure drilldown**
   - Delivered by summary cards + grouped failure detail + existing artifact links.

3. **Trend-ready aggregates exist without exposing raw secrets in previews or exports**
   - Delivered by lightweight aggregate cards/sections + unchanged redaction/export guards.

## Implementation Direction

Thực hiện P2-T6 bằng cách mở rộng `test-runner.tsx` như reporting surface chính, chỉ tăng backend/query surface ở mức tối thiểu cần thiết để hỗ trợ filter/group/aggregate. Mọi thay đổi phải bám các seam runner/history/detail/artifact/redaction đang tồn tại thay vì phát minh layer mới.
