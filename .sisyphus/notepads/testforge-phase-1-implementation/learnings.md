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
