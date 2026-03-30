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
- Standardized T3 foundations around four isolated Zustand stores (	abs, env, un, pp) so route resolution stays independent from tab lifecycle.
- Kept all frontend IPC requests behind invokeCommand in src/services/tauri-client.ts; hooks and route placeholders do not call Tauri directly.
## T2: Storage Bootstrap (2026-03-30)
- Chosen architecture: Database bootstrap now always runs file-based SQL migrations from src-tauri/migrations instead of maintaining a second inline schema path.
- Path/bootstrap policy: AppPaths owns creation of db/logs/screenshots/exports/config directories and the default settings.json under the resolved app-data root.
- Failure policy: missing migrations directory or checksum drift returns explicit migration errors rather than silently continuing.
