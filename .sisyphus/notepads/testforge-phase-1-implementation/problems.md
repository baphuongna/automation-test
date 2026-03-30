- Pending local verification: run cargo test in src-tauri once Rust toolchain is installed to execute newly added Rust contract serialization tests.

- Re-run T4 verification needed on a Rust-enabled machine (`cargo test --manifest-path src-tauri/Cargo.toml contracts`) because current environment lacks cargo.

- Pending final confirmation on Rust-enabled machine: run contract tests to execute new `EnvironmentVariableUpsertCommand` nested-payload serialization test and `EmptyCommandPayload` envelope test in `src-tauri/src/contracts/commands.rs`.
