# Phase 2B Win32 clipboard platform foundation verification

**Date:** 2026-08-04
**Baseline commit:** `8fb4f59`
**Branch:** `main`
**Verified implementation:** the local commits `46eaa35`, `2e5d1fa`, and the following CI/documentation batch, immediately before this report commit.

## 1. Environment

- Repository: `F:\Pastral`
- OS: Microsoft Windows NT `10.0.26200.0`
- Shell: Windows PowerShell `5.1.26100.8972`
- Rust host/target: `x86_64-pc-windows-msvc`
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`
- rustup active toolchain: `1.97.1-x86_64-pc-windows-msvc`, selected by `rust-toolchain.toml`
- rustfmt: `1.9.0-stable (8bab26f4f6 2026-07-14)`
- Clippy: `0.1.97 (8bab26f4f6 2026-07-14)`
- Visual Studio: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`
- MSVC linker: `VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
- Installed Windows SDK: `10.0.26100.0`
- Deferred native-slice Windows SDK `10.0.28000.2526`: not installed; the current APIs compile and run against the installed SDK and do not require that future native baseline package
- Windows binding: `windows-sys 0.61.2`
- Binding support crate: `windows-link 0.2.1`

## 2. Delivered scope

Phase 2B adds the first Windows-only clipboard boundary without creating a product agent or end-to-end history pipeline.

Implemented:

- `pastral-clipboard-win` with Windows-only compilation.
- Direct Win32 calls, raw handles, pointers, callback ABI, and unsafe blocks isolated to `crates/clipboard-win/src/sys.rs`.
- Crate-level denial of unsafe operations outside explicit unsafe blocks and denial of undocumented unsafe blocks.
- Dedicated standard-library listener thread with:
  - process-local window class;
  - message-only window;
  - `AddClipboardFormatListener` registration;
  - `WM_CLIPBOARDUPDATE` handling;
  - bounded `sync_channel(1)` notification delivery;
  - nonblocking coalescing through `try_send`;
  - startup handshake only after window/listener registration succeeds;
  - private stop message;
  - `PostThreadMessageW` shutdown fallback using the captured listener thread ID;
  - deterministic join and best-effort non-panicking Drop.
- Transient `ClipboardSequence` abstraction where raw zero is `Unavailable` and nonzero values remain opaque evidence.
- Thread-bound `ClipboardSession` read guard with successful `OpenClipboard`/`CloseClipboard` pairing and no internal retry or sleep.
- Ordered `EnumClipboardFormats` handling that resets and checks Win32 last error to distinguish normal end-of-list from failure.
- Nonzero bounded maximum format count.
- Explicit runtime format identity separate from durable identity.
- Known standard, private, GDI-object, registered, and reserved/unknown format classification.
- Exact registered format-name resolution through bounded-growing `GetClipboardFormatNameW` buffers.
- Bounded HGLOBAL copying using `GlobalSize`, `GlobalLock`, immediate owned-byte copy, and `GlobalUnlock`, without freeing clipboard-owned handles.
- First adapter for `CF_UNICODETEXT`:
  - exact logical UTF-16LE bytes through and including the first aligned NUL;
  - allocation padding excluded;
  - valid UTF-16 required;
  - empty text accepted;
  - no Unicode normalization, trimming, newline rewriting, or replacement-character decoding.
- Synthetic global-memory tests and real listener-window/message-loop tests that do not write clipboard content.
- Dependency policy that keeps `pastral-domain` and `pastral-storage` free of Windows bindings while allowing only pinned `windows-sys`/`windows-link` for the clipboard crate.
- Source policy that allows unsafe code only in `crates/clipboard-win/src/sys.rs` and scans tracked plus untracked nonignored source.
- Focused `eng/build.ps1 -Task Clipboard` and Windows CI coverage.
- README, changelog, developer setup, design, plan, and this verification report.

Not implemented:

- a resident capture agent, worker, CLI, service, or executable;
- capture retry/backoff, source attribution, policy evaluation, observation queue, storage commit, capture-order allocation, or overlay confirmation;
- COM/OLE, `IDataObject`, `FORMATETC`, `STGMEDIUM`, `IStream`, delayed rendering, virtual files, or callback cancellation;
- `CF_TEXT`, `CF_OEMTEXT`, locale, HTML, RTF, URL, file/drop, bitmap/DIB/PNG, shell, GDI, metafile, or arbitrary custom-format adapters;
- clipboard write/paste publication, `EmptyClipboard`, `SetClipboardData`, origin marker, or replay ownership;
- IPC, UI, packaging, installer, signing, telemetry, OCR, semantic search, AI, or networking.

## 3. Dependency boundary

The clipboard crate uses:

```text
pastral-clipboard-win v0.1.0
├── pastral-domain v0.1.0
└── windows-sys v0.61.2
    └── windows-link v0.2.1
```

Enabled `windows-sys` namespaces:

- `Win32_Foundation`
- `Win32_Graphics_Gdi`
- `Win32_System_DataExchange`
- `Win32_System_LibraryLoader`
- `Win32_System_Memory`
- `Win32_System_Threading`
- `Win32_UI_WindowsAndMessaging`

`Win32_Graphics_Gdi` is required by generated `WNDCLASSW`/`RegisterClassW` declarations even though the slice creates no GDI brush. `Win32_System_Threading` is used only for `GetCurrentThreadId`, enabling a thread-queue shutdown fallback.

Fresh dependency policy confirms:

- `pastral-domain`: no Windows binding package;
- `pastral-storage`: no Windows binding package;
- `pastral-clipboard-win`: exact `windows-sys 0.61.2` plus `windows-link 0.2.1` only;
- no async runtime, serde, alternate database stack, Protobuf/IPC runtime, logging backend, network client, Windows App SDK, WinUI, or high-level `windows` crate.

## 4. Unsafe boundary

All product unsafe code is located in:

```text
crates/clipboard-win/src/sys.rs
```

No other file under `crates/clipboard-win`, `crates/domain`, or `crates/storage` contains an unsafe function, block, trait, impl, or extern boundary.

The unsafe module covers only:

- Open/close/read clipboard API calls;
- transient sequence and format enumeration APIs;
- registered-name buffer calls;
- HGLOBAL size/lock/unlock/copy;
- listener class/window/message APIs;
- callback ABI;
- synthetic test-only global-memory allocation/free.

Each unsafe block has a local `SAFETY:` explanation for pointer validity, buffer lifetime, thread affinity, ownership/non-ownership, or paired cleanup. Public APIs expose no raw HWND, HANDLE, HGLOBAL, pointer, or `windows-sys` type.

`eng/verify-source-policy.ps1` freshly passed and enforces this file boundary while continuing to reject secret/private-key signatures, network/process APIs, SQLite extension loading, database attachment, WAL activation, machine-local launchers, credential/key files, and build output.

## 5. Test inventory and fresh results

Fresh workspace result:

- `pastral-domain`: 26 passed, 0 failed, 0 ignored.
- `pastral-storage`: 30 passed, 0 failed, 0 ignored.
- `pastral-clipboard-win`: 17 passed, 0 failed, 0 ignored.
- Total: **73 passed, 0 failed, 0 ignored**.

Clipboard test coverage includes:

- sequence zero/unavailable and opaque nonzero values;
- runtime ID zero rejection;
- standard/private/GDI/registered/unknown classification;
- registered durable identity contains exact name, not runtime ID;
- ordered enumeration and source ordinal preservation;
- enumeration count bound;
- enumeration error distinct from normal completion;
- synthetic HGLOBAL exact copy and max-size rejection;
- zero-size global allocation rejection;
- empty, ASCII, BMP, surrogate-pair, and CRLF Unicode text;
- first-NUL logical-byte truncation and allocation-padding exclusion;
- byte-distinct precomposed/decomposed text without normalization;
- missing terminator, partial UTF-16 code unit, and invalid surrogate rejection;
- listener startup, real message-only-window dispatch, notification receipt, and stop/join;
- bounded/coalescing notifications under repeated posted update messages;
- repeated listener start/stop lifecycle;
- disconnected receive state.

Focused tests were freshly run for:

```powershell
cargo test --locked -p pastral-clipboard-win listener::tests::listener_receives_posted_update_without_mutating_clipboard -- --exact --nocapture
cargo test --locked -p pastral-clipboard-win hglobal::tests::synthetic_global_copies_exact_bytes_and_enforces_bound -- --exact --nocapture
cargo test --locked -p pastral-clipboard-win unicode_text::tests::normalization_is_not_applied -- --exact --nocapture
```

The listener tests post `WM_CLIPBOARDUPDATE` to Pastral's own message-only test window. They do not call `EmptyClipboard` or `SetClipboardData` and do not overwrite user clipboard content.

## 6. TDD red/green evidence

Initial red command:

```powershell
cargo test --locked -p pastral-clipboard-win --all-targets
```

It failed because the workspace member existed but the crate had no Rust target. This failure was not committed.

Subsequent red/quality findings resolved before green commits:

- generated `WNDCLASSW`/`RegisterClassW` declarations required the narrowly enabled GDI feature;
- `GlobalFree` belonged to the Foundation namespace rather than the Memory import used initially;
- rustfmt differences in the new modules;
- Clippy found test-only imports, a non-const thread-local initializer opportunity, formatting/hygiene issues, and an ignored inner `Result` from `JoinHandle<Result<...>>`;
- source policy initially false-positive because a PowerShell regex call was used directly in a compound conditional; assigning the result before the condition fixed the gate;
- dependency policy initially failed on blank separator lines emitted by workspace `cargo tree`; filtering blank lines fixed the gate;
- lifecycle review added startup-channel join handling and a `PostThreadMessageW` fallback so a failed HWND stop post does not silently detach a live listener thread.

Final green evidence includes 17 focused native tests, 73 workspace tests, strict Clippy, source/dependency policies, and Git checks.

## 7. Fresh commands and results

The following gates were run after the implementation and CI/documentation batches:

| Command | Result |
|---|---|
| `eng/verify-toolchain.ps1` | PASS |
| `eng/build.ps1 -Task All` | PASS; 73 tests plus Clippy/doc/dependency/source policy |
| `eng/build.ps1 -Task Clipboard` | PASS; 17 clipboard tests |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --locked --workspace --all-targets` | PASS |
| `cargo test --locked --workspace --all-targets` | PASS; 73 passed, 0 failed, 0 ignored |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo doc --locked --workspace --no-deps` | PASS |
| `cargo tree --locked --workspace` | PASS |
| `eng/verify-dependencies.ps1` | PASS |
| `eng/verify-source-policy.ps1` | PASS |
| focused listener/HGLOBAL/normalization tests | PASS |
| `git diff --check` | PASS |
| `git fsck --no-progress --no-dangling` | PASS |
| `git show --check HEAD` | PASS |

## 8. Files changed from the Phase 2A baseline

Major additions and updates:

- Workspace manifests and lockfile:
  - `Cargo.toml`
  - `Cargo.lock`
- New Windows crate:
  - `crates/clipboard-win/Cargo.toml`
  - `crates/clipboard-win/src/lib.rs`
  - `crates/clipboard-win/src/error.rs`
  - `crates/clipboard-win/src/sequence.rs`
  - `crates/clipboard-win/src/format.rs`
  - `crates/clipboard-win/src/enumeration.rs`
  - `crates/clipboard-win/src/hglobal.rs`
  - `crates/clipboard-win/src/unicode_text.rs`
  - `crates/clipboard-win/src/session.rs`
  - `crates/clipboard-win/src/listener.rs`
  - `crates/clipboard-win/src/sys.rs`
- Build/policy/CI:
  - `eng/build.ps1`
  - `eng/verify-dependencies.ps1`
  - `eng/verify-source-policy.ps1`
  - `.github/workflows/rust-ci.yml`
- Documentation:
  - Phase 2B design and implementation plan;
  - `docs/operations/developer-setup.md`;
  - `README.md`;
  - `CHANGELOG.md`;
  - this report.

No executable, empty placeholder tree, CMake/vcpkg manifest, installer, `LICENSE`, local launcher, signing material, clipboard fixture executable, or user data was committed.

## 9. Local commit batches

- `46eaa35 docs: specify Win32 clipboard platform foundation`
- `2e5d1fa feat: add Win32 clipboard platform foundation`
- `ci: verify Win32 clipboard foundation`
- Final evidence commit follows this report.

No remote push, pull request, merge, history rewrite, or squash was performed.

## 10. Skipped checks

- GitHub-hosted `windows-2025` CI was not executed remotely because no push or pull request was authorized. Equivalent workflow commands were run locally on Windows.
- Tests did not replace the user's clipboard and therefore did not exercise a real `OpenClipboard` → `GetClipboardData(CF_UNICODETEXT)` capture against controlled clipboard content.
- No separate fixture producer/consumer executable exists yet.
- No contention/retry/backoff, clipboard-owner exit, delayed rendering, sequence-zero environment, fast user switching, window-station isolation, RDP, elevated/UIPI, or clipboard-history/cloud exclusion matrix was run.
- No COM/OLE apartment, message-filter, `IDataObject`, `IStream`, `STGMEDIUM`, virtual-file, or foreign callback test was run.
- No paste/write ownership transfer, origin marker, focus restoration, storage commit, process crash injection, or end-to-end capture/replay test was run.
- No fuzzing or property-testing framework was added for UTF-16, format enumeration, or native message boundaries.
- No performance benchmark measured listener idle cost, notification latency, clipboard-open acquisition, or HGLOBAL copy cost.
- No formal static analysis beyond Rust/Clippy and repository policy scans was run.

## 11. Residual risks and unproven claims

- The listener/window lifecycle is covered by repeated local tests but not by long-running soak, forced thread termination, desktop teardown, session logout, or remote hosted CI.
- The process-local window class intentionally remains registered for process lifetime. Windows and listener resources are released per listener instance, but class unregister behavior is not implemented or required in this library slice.
- If both HWND and thread-queue stop posts fail while the listener thread remains alive, Drop cannot safely force-kill the thread. The fallback substantially narrows this case but does not claim impossible leakage under a severely corrupted process/message subsystem.
- `ClipboardSession::open_read` intentionally performs one open attempt. Retry timing, cancellation, pressure accounting, and final-current-state capture remain future capture-coordinator responsibilities.
- The known-standard classifier identifies fixed IDs but does not claim those formats are supported for capture/replay.
- Only HGLOBAL-backed `CF_UNICODETEXT` is adapted. Other advertised formats remain descriptors only and arbitrary registered/custom bytes are not persisted.
- The Unicode parser trusts the copied allocation boundary reported by `GlobalSize`; malformed or hostile data is bounded and validated, but fuzz coverage is not yet present.
- Source policy is regex/signature based and is not a complete static analyzer or historical secret scanner.
- Windows SDK `10.0.28000.2526`, Windows App SDK, C++/WinUI, packaging, signing, accessibility, and release readiness remain outside this phase.
- No product-level clipboard fidelity, complete rapid-copy capture, privacy-policy execution, storage atomicity across capture, or paste compatibility claim is made by Phase 2B.
