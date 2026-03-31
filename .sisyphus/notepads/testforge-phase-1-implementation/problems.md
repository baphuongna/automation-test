- 2026-03-31 T6: Rust/Tauri runtime handlers could not be compile-verified locally because `cargo` is unavailable in this environment; verification fell back to shared-contract source assertions plus frontend compile/build evidence.
- 2026-03-31 T8: End-to-end Rust compile/run verification for new API engine modules (`api_execution_service`, `api_repository`, new handlers/contracts) is still pending on a machine with `cargo` + `rust-analyzer`.
- 2026-03-31 T8 follow-up: migration `002_add_api_endpoint_query_params.sql` và các thay đổi repository/model cần được xác nhận thêm bằng `cargo test` trên môi trường có Rust toolchain.
- 2026-03-31 T9: API Tester collection hydration vẫn chưa thể đọc lại persisted test cases trực tiếp từ backend vì current typed IPC surface không có list/load command; route hiện phụ thuộc vào feature-local workspace cache cho tree state ngoài lần save/delete trong session/browser storage.
- 2026-03-31 T10: Rust compile/runtime verification for new artifact/export baseline is still pending on a machine with `cargo` + `rust-analyzer`; current evidence is limited to source assertions, TypeScript typecheck, and production build.

- 2026-03-31 T11: Runtime health baseline đã sẵn sàng qua IPC và event emission, nhưng probing hiện chỉ ở mức discovery scaffold (candidate path/env) chứ chưa launch Chromium thực tế; launch/runtime executor sẽ được hoàn thiện ở các task sau (T12/T14).

- 2026-03-31 T12: End-to-end recorder runtime (browser event tap + crash recovery in live Chromium) chưa thể chạy trong container thiếu `cargo`/Tauri runtime; cần xác nhận thêm trên môi trường có Rust toolchain và app runtime đầy đủ.

- 2026-03-31 T13: End-to-end runtime verification cho Web Recorder UI (Tauri window + browser recorder events thật) vẫn cần được xác nhận thêm trên máy có `cargo`, Rust toolchain, và app runtime đầy đủ; hiện tại verification trong container dừng ở source regression tests + typecheck/build evidence.

- 2026-03-31 T13 bugfix: Preview-local live-step path hiện đã được vá ở hook boundary, nhưng QA end-to-end với browser preview thực tế vẫn nên được rerun trên môi trường có app runtime đầy đủ để xác nhận cảm giác realtime ngoài source/test evidence.
- 2026-03-31 T14: Runtime replay executor hiện mới ở mức backend sequential semantics + screenshot-manifest persistence baseline; chạy browser automation thực tế, Playwright runtime, vẫn cần xác nhận bổ sung trên môi trường có đầy đủ Rust/Tauri runtime.
- 2026-03-31 T14 follow-up: Chromium CLI adapter mới vẫn chưa thay thế hoàn toàn browser automation engine tương tác DOM (click/fill/select) kiểu Playwright; các action này hiện fail rõ ràng có chủ đích để tránh false-positive, và cần runtime engine sâu hơn ở phase tiếp theo nếu muốn hỗ trợ đầy đủ.
- 2026-03-31 T14 interaction: để đạt tương tác UI sâu/phức tạp hơn, event dispatch chính xác, stateful multi-step DOM mutation mạnh, cần runtime automation engine cấp cao hơn Chromium CLI baseline ở phase sau.
- 2026-03-31 T14: interaction executor hiện là baseline CDP script inline; về lâu dài nên tách thành runtime module riêng để quản lý vòng đời browser/session ổn định hơn khi kịch bản phức tạp tăng.
- 2026-03-31 T14 smoke status: `npm run test:t14:smoke` = `SMOKE_BLOCKED` in current environment because Chromium executable is missing; runtime smoke cannot be promoted to `SMOKE_PASS` until one of the checked browser paths exists or `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` is provided.
- 2026-03-31 T14 smoke: khi orchestrator cấp Chromium path hợp lệ, hoặc `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH`, cần chạy lại `npm run test:t14:smoke` để chuyển từ blocked sang runtime proof pass/fail thực tế.
