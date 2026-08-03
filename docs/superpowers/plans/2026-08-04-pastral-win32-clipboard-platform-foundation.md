# Pastral Win32 Clipboard Platform Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reviewed Windows-only clipboard boundary for listener notifications, sequence evidence, ordered format inspection, bounded HGLOBAL copying, and exact `CF_UNICODETEXT` extraction.

**Architecture:** Isolate all unsafe Win32 calls in one `sys.rs` module and expose safe, bounded, non-serializable adapter types from `pastral-clipboard-win`. Use a dedicated standard-library thread with a message-only window for notifications and a separate `!Send`/`!Sync` read session for clipboard inspection.

**Tech Stack:** Rust 1.97.1, Edition 2024, `windows-sys = 0.61.2`, Win32 User32/Kernel32 APIs, Windows PowerShell 5.1.

## Global Constraints

- Baseline is `8fb4f59` on `main`; preserve unrelated files and commit only green batches.
- No product executable, COM/OLE, storage orchestration, IPC, UI, packaging, telemetry, AI, or networking.
- No test may call `EmptyClipboard` or `SetClipboardData` or overwrite the user's real clipboard.
- Runtime clipboard format IDs are ephemeral and never become durable `ClipboardFormatIdentity` for registered formats.
- `pastral-domain` and `pastral-storage` must remain free of Windows dependencies.
- All direct Win32 calls and unsafe code live only in `crates/clipboard-win/src/sys.rs`.
- Every unsafe block has a local `SAFETY:` comment and passes `clippy::undocumented_unsafe_blocks`.
- Listener callback never opens the clipboard, parses payloads, blocks on channel delivery, or calls storage.
- Cargo gates use `--locked`; no remote push or PR.

---

### Task 1: Commit the approved native-boundary design

**Files:**
- Create: `docs/superpowers/specs/2026-08-04-pastral-win32-clipboard-platform-foundation-design.md`
- Create: `docs/superpowers/plans/2026-08-04-pastral-win32-clipboard-platform-foundation.md`

- [ ] **Step 1: Self-review scope and placeholders**

```powershell
Select-String -Path docs/superpowers/specs/2026-08-04-pastral-win32-clipboard-platform-foundation-design.md,docs/superpowers/plans/2026-08-04-pastral-win32-clipboard-platform-foundation.md -Pattern '\b(TBD|TODO|FIXME)\b'
git diff --check
git status --short
```

Expected: no unresolved placeholder and only the two new documents.

- [ ] **Step 2: Commit**

```powershell
git add docs/superpowers/specs/2026-08-04-pastral-win32-clipboard-platform-foundation-design.md docs/superpowers/plans/2026-08-04-pastral-win32-clipboard-platform-foundation.md
git commit -m "docs: specify Win32 clipboard platform foundation"
```

---

### Task 2: Add crate policy, sequence, formats, and Unicode parser

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/clipboard-win/Cargo.toml`
- Create: `crates/clipboard-win/src/lib.rs`
- Create: `crates/clipboard-win/src/error.rs`
- Create: `crates/clipboard-win/src/sequence.rs`
- Create: `crates/clipboard-win/src/format.rs`
- Create: `crates/clipboard-win/src/unicode_text.rs`
- Create: `crates/clipboard-win/src/sys.rs`
- Modify: `eng/verify-dependencies.ps1`
- Modify: `eng/verify-source-policy.ps1`

**Produces:** `ClipboardSequence`, `RuntimeClipboardFormatId`, `RuntimeFormatKind`, `ClipboardFormatDescriptor`, `CapturedUnicodeText`, parser tests, and narrowed unsafe/dependency policy.

- [ ] **Step 1: Write failing pure tests**

Cover sequence zero/nonzero, runtime ID zero rejection, known standard/private/GDI/registered/unknown classification, and Unicode parser cases including empty, surrogate pair, missing NUL, invalid surrogate, CRLF, normalization-distinct bytes, and padding after first NUL.

- [ ] **Step 2: Run red command**

```powershell
cargo test --locked -p pastral-clipboard-win --all-targets
```

Expected: package/targets do not yet exist.

- [ ] **Step 3: Add exact Windows dependency and crate lints**

Pin `windows-sys = 0.61.2` with only Foundation, DataExchange, LibraryLoader, Memory, and WindowsAndMessaging features. Add:

```rust
#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]
```

- [ ] **Step 4: Implement pure types and parser**

Keep runtime IDs separate from durable identities. Parse UTF-16LE bytes through the first aligned NUL, include that NUL in raw logical bytes, reject invalid UTF-16, and never normalize.

- [ ] **Step 5: Narrow policy gates**

Dependency verification must inspect domain/storage separately and allow the pinned Windows binding family only for clipboard-win. Source policy must allow unsafe only in `crates/clipboard-win/src/sys.rs` and still reject unsafe elsewhere, networking, process spawning, extension loading, database attachment, WAL, secrets, keys, and machine-local files.

- [ ] **Step 6: Verify and commit**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-clipboard-win --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
eng/verify-dependencies.ps1
eng/verify-source-policy.ps1
```

```powershell
git add Cargo.toml Cargo.lock crates/clipboard-win eng/verify-dependencies.ps1 eng/verify-source-policy.ps1
git commit -m "feat: add Win32 clipboard format inspection"
```

---

### Task 3: Add safe HGLOBAL and read-session boundary

**Files:**
- Create: `crates/clipboard-win/src/hglobal.rs`
- Create: `crates/clipboard-win/src/session.rs`
- Create: `crates/clipboard-win/src/enumeration.rs`
- Modify: `crates/clipboard-win/src/sys.rs`
- Modify: `crates/clipboard-win/src/lib.rs`
- Modify: `crates/clipboard-win/src/error.rs`

**Produces:** `ClipboardSession::open_read`, ordered bounded enumeration, registered-name resolution, bounded HGLOBAL copy, and `capture_unicode_text`.

- [ ] **Step 1: Write failing tests**

Add sys-independent enumeration state-machine tests and synthetic HGLOBAL tests for exact copy, max-size rejection, zero/null handle, first-NUL truncation, and registered-name exactness. Do not mutate the real clipboard.

- [ ] **Step 2: Implement RAII session**

`ClipboardSession` owns a successful `OpenClipboard(NULL)` and closes exactly once in Drop. Include an `Rc` marker so it is `!Send`/`!Sync`. Do not retry internally.

- [ ] **Step 3: Implement ordered enumeration**

Reset last error before every `EnumClipboardFormats` call. A zero result with `ERROR_SUCCESS` ends normally; any other code fails. Enforce `NonZeroUsize` maximum before pushing beyond the bound.

- [ ] **Step 4: Resolve stable identities**

Known standard IDs become `StandardFormatId`. Registered range IDs call bounded-growing `GetClipboardFormatNameW`, reject truncation/invalid UTF-16, then construct `RegisteredFormatName`. Private/GDI/unknown values remain explicitly non-durable classifications.

- [ ] **Step 5: Implement bounded HGLOBAL copy and Unicode adapter**

Use `GlobalSize`, compare against `max_bytes`, `GlobalLock`, immediate byte copy, and `GlobalUnlock`. Never free a clipboard-owned handle. `capture_unicode_text` requests only `CF_UNICODETEXT` and passes copied bytes to the pure parser.

- [ ] **Step 6: Verify and commit**

```powershell
eng/build.ps1 -Task All
cargo test --locked -p pastral-clipboard-win --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
```

```powershell
git add crates/clipboard-win
git commit -m "feat: add bounded clipboard read session"
```

---

### Task 4: Add the dedicated listener thread

**Files:**
- Create: `crates/clipboard-win/src/listener.rs`
- Modify: `crates/clipboard-win/src/sys.rs`
- Modify: `crates/clipboard-win/src/lib.rs`
- Modify: `crates/clipboard-win/src/error.rs`

**Produces:** `ClipboardListener`, `ClipboardNotifications`, bounded/coalescing notification delivery, deterministic stop/join.

- [ ] **Step 1: Write failing listener lifecycle tests**

Tests start the real message-only window/listener, post a test `WM_CLIPBOARDUPDATE` without changing clipboard content, receive one notification, stress repeated posts against capacity one, stop, and repeat start/stop.

- [ ] **Step 2: Implement thread-local callback state**

Store only the bounded sender in listener-thread local state. The unsafe window procedure handles `WM_CLIPBOARDUPDATE`, a private stop message, and destroy/quit. It performs no payload work.

- [ ] **Step 3: Implement startup handshake and cleanup**

Do not return success until class/window/listener registration succeeds. On any partial failure, destroy/remove resources in reverse order. `stop` posts the private message and joins; Drop is best effort and non-panicking.

- [ ] **Step 4: Verify and commit**

```powershell
cargo test --locked -p pastral-clipboard-win --all-targets -- --nocapture
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
eng/verify-source-policy.ps1
```

```powershell
git add crates/clipboard-win
git commit -m "feat: add clipboard listener foundation"
```

---

### Task 5: Extend CI, docs, and final evidence

**Files:**
- Modify: `.github/workflows/rust-ci.yml`
- Modify: `eng/build.ps1`
- Modify: `docs/operations/developer-setup.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Create: `docs/reviews/phase-2b-win32-clipboard-platform-verification.md`

- [ ] **Step 1: Add focused clipboard task and CI lane**

Add `-Task Clipboard` running:

```powershell
cargo test --locked -p pastral-clipboard-win --all-targets
```

Keep workspace format/check/test/Clippy/doc/dependency/source gates and fail-fast behavior.

- [ ] **Step 2: Document native boundary and limitations**

Record exact Windows commands, `windows-sys 0.61.2`, unsafe isolation policy, listener/read-session behavior, no clipboard-writing tests, and excluded COM/OLE/product-agent scope.

- [ ] **Step 3: Run fresh final verification**

```powershell
git status --short
eng/verify-toolchain.ps1
eng/build.ps1 -Task All
eng/build.ps1 -Task Clipboard
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
cargo tree --locked --workspace
eng/verify-dependencies.ps1
eng/verify-source-policy.ps1
git diff --check
git fsck --no-progress --no-dangling
git show --check HEAD
```

Run focused listener, HGLOBAL, Unicode parser, enumeration, and domain/storage no-Windows dependency evidence.

- [ ] **Step 4: Write honest report**

Include baseline/final commit, exact environment/dependencies, unsafe locations, commands/results, test counts, red/green evidence, source/dependency policy, files changed, scope exclusions, skipped live clipboard overwrite/COM/agent checks, and residual risks.

- [ ] **Step 5: Commit docs/evidence and verify clean state**

```powershell
git add .github/workflows/rust-ci.yml eng/build.ps1 docs/operations/developer-setup.md README.md CHANGELOG.md docs/reviews/phase-2b-win32-clipboard-platform-verification.md
git commit -m "ci: verify Win32 clipboard foundation"
git status --short
git log --oneline -15
git show --check HEAD
```
