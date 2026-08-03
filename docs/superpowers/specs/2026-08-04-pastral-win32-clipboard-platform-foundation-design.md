# Pastral Win32 clipboard platform foundation design

**Status:** Approved for autonomous implementation by the 2026-08-04 continuation request.
**Date:** 2026-08-04
**Baseline:** `8fb4f59`

## 1. Decision

Build Phase 2B as a narrowly bounded Windows-only Rust crate that owns the first reviewed Win32 clipboard boundary:

- a dedicated listener thread with a message-only window and `AddClipboardFormatListener`;
- transient clipboard sequence snapshots;
- RAII read-only clipboard open/close ownership;
- ordered runtime format enumeration and stable format-identity resolution;
- bounded immediate copy of HGLOBAL-backed data;
- a first adapter for `CF_UNICODETEXT` that preserves exact logical UTF-16LE bytes through the first terminating NUL and exposes a validated Unicode view.

Do not add a resident agent executable, COM/OLE, storage orchestration, source attribution, retry policy, delayed-render cancellation, paste publication, arbitrary custom-format persistence, UI, IPC, packaging, or product network access in this slice.

## 2. Authority and current research

Repository authority:

- `docs/architecture/threading-and-com-apartments.md`
- `docs/architecture/clipboard-lifecycle.md`
- `docs/architecture/clipboard-event-identity.md`
- `docs/architecture/clipboard-format-policy.md`
- `docs/adr/0011-windows-support-baseline.md`
- `docs/adr/0015-clipboard-capture-threading.md`
- `docs/adr/0016-data-identity-time-and-digests.md`
- `docs/security/privacy-model.md`
- `docs/testing/strategy.md`

Primary-source findings applied:

- `AddClipboardFormatListener` posts `WM_CLIPBOARDUPDATE` to a registered window; the message itself is not an event identity.
- `GetClipboardSequenceNumber` is scoped to the current window station and returns zero when clipboard access is unavailable; it is transient evidence, not a durable ID.
- `EnumClipboardFormats` requires an open clipboard, returns formats in source priority order, and distinguishes end-of-list from error through `GetLastError == ERROR_SUCCESS` after a zero result.
- Registered format runtime IDs are in `0xC000..=0xFFFF`; durable identity is the exact registered name, not the runtime number.
- `GetClipboardFormatNameW` can truncate when the caller buffer is too small, so resolution uses bounded growth and fails rather than persisting a possibly truncated name.
- `GetClipboardData` returns clipboard-owned, untrusted data. The adapter copies immediately and never frees the returned handle.
- `GlobalSize` may exceed the originally requested allocation, and `GlobalLock` can return null for zero-sized/discarded objects. Size, pointer, terminator, alignment, and UTF-16 validity are checked before domain construction.
- `windows-sys 0.61.2` is the pinned low-overhead binding crate for this boundary.

## 3. Scope

### Included

- New crate `crates/clipboard-win`, package name `pastral-clipboard-win`.
- Windows-only compilation and tests.
- Unsafe code isolated to `src/sys.rs` with `unsafe_op_in_unsafe_fn` and `clippy::undocumented_unsafe_blocks` denied.
- Standard-library listener thread and bounded `std::sync::mpsc`-style notification delivery; no async runtime.
- Message-only listener window lifecycle:
  - register class;
  - create window;
  - add listener;
  - process messages;
  - remove listener;
  - destroy window;
  - stop/join deterministically.
- Listener callback only emits a lightweight notification with transient sequence evidence; it does not open or parse the clipboard in the window procedure.
- `ClipboardSession` read guard that is `!Send` and `!Sync`, closes exactly once on drop, and exposes enumeration/capture methods only while the clipboard is open.
- Runtime format ID and classification types that cannot be confused with durable `ClipboardFormatIdentity`.
- Known standard-format whitelist, private/GDI ranges, registered-name resolution, and explicit unknown/reserved classification.
- Ordered enumeration with configurable nonzero maximum format count.
- Bounded HGLOBAL copy helper.
- `CF_UNICODETEXT` parser/adapter with exact logical bytes, validation, and no text normalization.
- Pure/parser, synthetic HGLOBAL, listener lifecycle/message-loop, classification, and error-path tests.
- CI/build/dependency/source-policy/documentation updates and milestone evidence.

### Excluded

- COM/OLE initialization, `IDataObject`, `FORMATETC`, `STGMEDIUM`, `IStream`, delayed-render worker isolation, or message pumping during foreign COM calls.
- Actual capture orchestration, retry/backoff, storage commit, capture order allocation, overlay confirmation, or source context.
- `CF_TEXT`, `CF_OEMTEXT`, locale conversion, HTML, RTF, URL, file/drop, image, virtual-file, shell, GDI, metafile, or arbitrary custom-format adapters.
- Clipboard write/paste paths, `EmptyClipboard`, `SetClipboardData`, ownership transfer, or origin marker.
- A product agent/worker/CLI executable.
- Automated tests that overwrite the user's real clipboard.
- Windows App SDK, WinUI, C++, CMake, vcpkg manifest, installer, signing, telemetry, OCR, semantic search, AI, or networking.

## 4. Dependency decision

Workspace dependency:

```toml
windows-sys = { version = "=0.61.2", default-features = false, features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_System_DataExchange",
    "Win32_System_LibraryLoader",
    "Win32_System_Memory",
    "Win32_System_Threading",
    "Win32_UI_WindowsAndMessaging",
] }
```

Rationale:

- `windows-sys` supplies declarations/constants without allocating wrapper abstractions and is appropriate for a small reviewed native boundary.
- Exact pinning and `--locked` preserve reproducibility.
- Only the required Win32 feature namespaces are enabled. `Win32_Graphics_Gdi` is included because the generated `WNDCLASSW`/`RegisterClassW` declarations are feature-gated on the class background-brush type even though Pastral does not create or own a GDI brush in this slice. `Win32_System_Threading` is included only for `GetCurrentThreadId`, which enables a `PostThreadMessageW` shutdown fallback if the listener HWND can no longer accept the private stop message.
- No `windows`, `windows-core`, COM feature, async runtime, logging backend, serialization framework, or retry dependency is added.

The existing dependency policy changes from globally forbidding Windows bindings to enforcing:

- `pastral-domain` and `pastral-storage` remain free of all Windows binding packages;
- `pastral-clipboard-win` may use only the pinned `windows-sys` family plus existing workspace crates;
- async, network, serialization, alternate database, IPC, logging, and UI dependencies remain forbidden globally.

## 5. Crate and file boundaries

```text
crates/clipboard-win/
├─ Cargo.toml
└─ src/
   ├─ lib.rs          # exports, Windows-only crate policy
   ├─ error.rs        # ClipboardError and sanitized Win32 error context
   ├─ sequence.rs     # transient sequence snapshot
   ├─ format.rs       # runtime IDs, classification, stable identity resolution
   ├─ session.rs      # read-only OpenClipboard/CloseClipboard guard
   ├─ enumeration.rs  # ordered EnumClipboardFormats loop and limits
   ├─ hglobal.rs      # bounded copy facade
   ├─ unicode_text.rs # CF_UNICODETEXT logical-byte parser/adapter
   ├─ listener.rs     # safe listener lifecycle and notification API
   └─ sys.rs          # all direct unsafe Win32 calls and test allocation helpers
```

All direct Win32 calls, raw pointers, callback ABI, handle casts, and unsafe blocks remain in `sys.rs`. Safe modules operate on checked Rust values.

## 6. Public interfaces

### Sequence evidence

```rust
pub enum ClipboardSequence {
    Unavailable,
    Available(u32),
}

pub struct ClipboardNotification {
    sequence: ClipboardSequence,
}
```

Zero maps to `Unavailable`. No ordering arithmetic, copy-count inference, durable identity, or wall-time semantics are attached.

### Listener

```rust
pub struct ClipboardListener { /* owns thread/window */ }

impl ClipboardListener {
    pub fn start() -> Result<(Self, ClipboardNotifications), ClipboardError>;
    pub fn stop(self) -> Result<(), ClipboardError>;
}

pub struct ClipboardNotifications { /* bounded receiving endpoint */ }

impl ClipboardNotifications {
    pub fn recv_timeout(&self, timeout: Duration) -> Result<ClipboardNotification, NotificationReceiveError>;
    pub fn try_recv(&self) -> Result<ClipboardNotification, NotificationReceiveError>;
}
```

The implementation uses `sync_channel(1)`. If notifications arrive faster than the consumer, the window procedure uses `try_send`; it coalesces pressure instead of blocking the control thread or allocating an unbounded queue. A later capture coordinator owns detailed pressure accounting.

`ClipboardListener` is not clonable. `stop` posts a private stop message, the listener thread removes itself, destroys its window, exits the message loop, and joins. Drop performs best-effort stop/join without panicking.

### Session and enumeration

```rust
pub struct ClipboardSession { /* !Send + !Sync */ }

impl ClipboardSession {
    pub fn open_read() -> Result<Self, ClipboardError>;
    pub fn enumerate_formats(&self, max_formats: NonZeroUsize) -> Result<Vec<ClipboardFormatDescriptor>, ClipboardError>;
    pub fn capture_unicode_text(&self, max_bytes: NonZeroUsize) -> Result<Option<CapturedUnicodeText>, ClipboardError>;
}
```

No retry or sleep occurs inside `open_read`; the future capture coordinator owns retry policy. Successful open always pairs with `CloseClipboard` through Drop.

### Runtime format descriptors

```rust
pub struct RuntimeClipboardFormatId(NonZeroU32);

pub enum RuntimeFormatKind {
    KnownStandard(ClipboardFormatIdentity),
    Registered(ClipboardFormatIdentity),
    Private,
    GdiObject,
    ReservedOrUnknown,
}

pub struct ClipboardFormatDescriptor {
    runtime_id: RuntimeClipboardFormatId,
    source_ordinal: usize,
    kind: RuntimeFormatKind,
}
```

Only `KnownStandard` and successfully resolved `Registered` variants contain durable identities. Runtime registered numbers remain explicit ephemeral adapter data.

Known standard IDs initially include the documented fixed formats required to classify the current source set, including text, bitmap/DIB, locale, drop, palette, wave/TIFF/RIFF, metafile, owner-display, and display formats. No support claim is implied by classification.

### Unicode text

```rust
pub struct CapturedUnicodeText {
    raw_logical_bytes: Vec<u8>,
    text: String,
}
```

Rules:

- allocation size must be nonzero and not exceed `max_bytes`;
- the first UTF-16 NUL on an aligned code-unit boundary terminates the logical payload;
- raw logical bytes include exactly that first terminating NUL and exclude allocation padding or later data;
- bytes before the terminator must be valid UTF-16;
- no Unicode normalization, newline rewriting, trimming, or replacement-character decoding occurs;
- missing terminator, invalid surrogate sequence, oversized allocation, null handle, lock failure, or unsafe length conversion fails;
- empty text encoded as one terminating UTF-16 NUL is accepted.

## 7. Unsafe boundary policy

`src/sys.rs` is the only product file permitted to contain unsafe operations.

Crate lints:

```rust
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]
```

Every unsafe block has a local `SAFETY:` explanation covering:

- API preconditions;
- pointer validity and byte length;
- handle ownership/non-ownership;
- callback lifetime/thread affinity;
- unlock/close/destroy pairing;
- why Rust aliases or references do not outlive the foreign resource.

Safe modules never expose raw `HWND`, `HANDLE`, `HGLOBAL`, pointers, or `windows-sys` types publicly.

## 8. Listener lifecycle

The listener thread:

1. installs its bounded sender in thread-local state;
2. registers a process-local window class;
3. creates a message-only window;
4. calls `AddClipboardFormatListener`;
5. reports startup success only after registration succeeds;
6. dispatches `WM_CLIPBOARDUPDATE` into `ClipboardNotification` using nonblocking `try_send`;
7. handles a private stop message by removing the listener and destroying the window;
8. posts quit on destroy and exits the message loop;
9. clears thread-local state and returns a sanitized result.

No clipboard open, payload access, allocation proportional to clipboard content, blocking channel send, storage call, UI call, or foreign callback is allowed inside the window procedure.

## 9. Error model

Manual `ClipboardError` variants include:

- listener thread spawn/start/stop/join failure;
- window class/window/listener/message-loop operation plus numeric Win32 code;
- clipboard unavailable/open/close failure;
- enumeration failure and format-count limit exceeded;
- invalid/zero runtime format ID;
- registered-name resolution failure, truncation limit, invalid UTF-16, empty/NUL name;
- unsupported/reserved format identity;
- null data handle, zero/oversized global allocation, lock/unlock failure;
- Unicode text missing terminator, invalid alignment, invalid UTF-16, or bounded-length conversion failure.

Errors contain operation names, runtime IDs, byte counts, and Win32 codes only. They never include clipboard payload bytes or decoded text.

## 10. Tests

Required automated tests:

- sequence zero maps to unavailable; nonzero remains opaque and wrap-safe;
- runtime ID zero rejected;
- known standard, private, GDI-object, registered-range, and unknown/reserved classification;
- registered name resolution returns exact stable name and never embeds runtime ID in durable identity;
- ordered enumeration end-of-list versus error behavior through a fake/sys-independent state machine;
- format-count bound enforced before unbounded allocation;
- Unicode empty, ASCII, BMP, surrogate-pair, CRLF, and byte-distinct normalization examples accepted without rewriting;
- missing NUL, invalid surrogate, odd/truncated code unit, and trailing padding behavior;
- synthetic HGLOBAL exact copy, zero allocation, null handle, and max-size rejection;
- listener starts, receives a posted test `WM_CLIPBOARDUPDATE` through the real message loop, coalesces without blocking, stops, and joins;
- listener repeated start/stop does not leak a registered window/listener;
- source-policy gate confirms unsafe code exists only in `src/sys.rs`;
- dependency gate confirms Windows bindings do not enter domain/storage.

Automated tests do not call `EmptyClipboard` or `SetClipboardData` and therefore do not overwrite the user's real clipboard.

## 11. Observable acceptance criteria

- Workspace builds with pinned Rust 1.97.1 and `windows-sys 0.61.2` on `x86_64-pc-windows-msvc`.
- `pastral-domain` and `pastral-storage` dependency trees remain free of Windows crates.
- All direct unsafe code is isolated to `crates/clipboard-win/src/sys.rs`, documented, and linted.
- Listener startup is not reported until the message-only window and clipboard listener are active.
- Listener callback never opens/parses the clipboard or blocks on notification delivery.
- Stop/drop releases listener/window/thread resources without panic.
- Clipboard session cannot cross threads and closes on every successful open path.
- Enumeration preserves source order, distinguishes normal completion from Win32 failure, and enforces a hard count bound.
- Registered durable identity is exact name only; runtime number remains ephemeral.
- `CF_UNICODETEXT` output is exact logical UTF-16LE bytes through the first NUL plus a validated view, with no normalization.
- No product clipboard write occurs in code or tests.
- Fresh format/check/test/Clippy/doc, dependency/source policy, listener/HGLOBAL/parser tests, diff, secret, scope, and Git checks pass.
- Local batch commits exist; no remote push or PR occurs.

## 12. Commit batches

1. `docs: specify Win32 clipboard platform foundation`
2. `feat: add Win32 clipboard format inspection`
3. `feat: add clipboard listener foundation`
4. `ci: verify Win32 clipboard foundation`
5. `docs: record Win32 clipboard foundation evidence`

Tests and minimal implementation stay together in green commits; observed red evidence is recorded without committing a failing branch.
