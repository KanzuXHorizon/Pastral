# Threading and COM apartments

## 1. Goals

- Keep clipboard notifications, tray, hotkeys, session handling, overlay, and IPC health responsive even when a clipboard owner is slow or hostile.
- Keep foreign COM/OLE ownership inside one reviewed apartment.
- Serialize storage mutation without running SQLite or file I/O on window procedures.
- Avoid periodic wakeups and a continuously running general-purpose async runtime unless measurement justifies one.

## 2. Agent execution contexts

| Context | Apartment/model | Owns | Must not do |
|---|---|---|---|
| Control/overlay thread | STA-capable Win32 message loop; no foreign clipboard COM calls | listener HWND, tray, hotkeys, session/power notifications, overlay HWND/composition device coordination, capture supervision | `IDataObject::EnumFormatEtc/GetData`, large copies, SQLite, parsing, waits |
| Clipboard platform thread | Dedicated STA, `OleInitialize`, message pump, private helper HWND | `OpenClipboard`, privacy-format inspection, foreign capture objects/media, `OleSetClipboard`/`OleIsCurrentClipboard`/`OleFlushClipboard`, Pastral replay data-object lifetime | SQLite, IPC/business queries from COM callbacks, overlay rendering, manager view models, unbounded waits by design |
| Storage executor | Serialized worker/context; COM-free unless a later ADR proves need | SQLite connection/migrations, blob staging/finalization, FTS, retention transactions | foreign clipboard handles/interfaces, UI calls |
| IPC I/O | Overlapped/IOCP or bounded native workers | pipe accepts, framing, cancellation, result streaming | blocking capture/storage thread, unbounded client queues |
| Lightweight work pool | Bounded, lazy | hashing, small classification, non-hostile post-capture work | complex parsers/codecs, OCR, unbounded tasks |
| Worker process | Separate restricted process | hostile/complex parsing and bounded derived jobs | main DB, root keys, broad filesystem/network |

Exact thread count is measured. The table defines ownership, not a requirement that every context consume a permanently active OS thread if an equivalent event-driven implementation preserves the boundaries.

## 3. Clipboard notification handoff

1. Control thread receives `WM_CLIPBOARDUPDATE`.
2. It reads the current sequence, active Pastral transaction marker, foreground HWND/process snapshot, session/profile state, and monotonic observation timestamp using nonblocking calls.
3. It posts a bounded `ClipboardObservation` to the capture STA.
4. It returns from the window procedure immediately.
5. Capture STA rejects stale/equivalent observations, acquires current clipboard state through its own helper HWND (or an explicitly justified null-owner read path), and emits an owned capture result or audit outcome. It does not pass the control-thread listener HWND to `OpenClipboard` as an implicit cross-thread ownership shortcut.
6. Storage executor commits the result.
7. Control thread receives only an immutable overlay/status view model.

The source snapshot is evidence at notification time, not proof that the same process remains owner at capture time.

## 4. Foreign-call rules

- Never invoke a foreign `IDataObject`, `IEnumFORMATETC`, `IStream`, `IStorage`, or `pUnkForRelease` method from the control/overlay, storage, IPC, or UI thread.
- Do not transfer raw `STGMEDIUM`, GDI handles, global-memory pointers, or foreign COM interfaces outside the capture STA.
- Copy validated data to owned buffers, duplicated handles, or staged files first.
- Balance every successful medium acquisition with the documented release path, normally `ReleaseStgMedium`, on the owning apartment.
- Wrap re-entrant state with explicit transaction guards; do not hold database or UI locks during foreign calls.
- A newer observation may make a result stale, but stale cancellation must not leak acquired media or staging files.
- Pastral replay `IDataObject` callbacks execute on the clipboard STA and use only prevalidated owned memory, immutable pre-opened blob/stream resources, and bounded adapter state. They do not query SQLite, IPC, rule engines, or UI synchronously.
- Capture and replay commands are serialized by an explicit state machine. Publishing a new object, flushing/retiring an old object, and processing a foreign update cannot race ownership teardown.

## 5. Cancellation and health

### Soft deadline

A capture attempt has a soft deadline used for metrics, newer-sequence cancellation decisions, and user-visible health. It is not implemented through unsafe thread termination.

### COM cancellation

`CoEnableCallCancellation` and `CoCancelCall` are enabled only around calls proven eligible and only after measuring their overhead. Failure to obtain a cancel object or server non-cooperation is expected and handled as degraded capture.

### Stuck apartment

A supervisor tracks last-progress phase without inspecting payloads. When the capture STA exceeds the hard health threshold:

- stop queueing unbounded work;
- retain only bounded latest-state intent;
- suppress misleading “captured” overlays;
- show `Capture delayed`/`Capture paused` through safe UI;
- allow explicit safe agent restart;
- collect metadata-only phase/duration/result evidence;
- activate per-source safe mode only when source evidence is sufficiently reliable;
- open the capture-broker ADR review if fixtures reproduce the issue.

Do not call `TerminateThread`. A blocked clipboard STA also delays replay publication/callback service, so the degraded-state UI must not promise paste availability; explicit process restart and a future capture-broker/replay-apartment split are review paths.

## 6. Manager and Quick Paste apartments

`pastral-manager.exe` uses the normal WinUI 3 UI STA. IPC, search streaming, preview decoding, and file work remain asynchronous and never block the XAML dispatcher. Quick Paste is another window/activation mode in the same manager process, not a second UI runtime.

## 7. Worker process apartments

Worker job type declares whether COM is forbidden, MTA, or STA. A worker initializes only the required apartment and exposes no general COM activation surface. Jobs that do not require COM do not initialize it.

## 8. Shutdown and session transitions

- Stop accepting new capture observations before tearing down the capture STA.
- Cancel/finish owned staging work before `OleUninitialize`.
- Do not uninitialize COM while foreign interfaces/media remain.
- Overlay/device teardown occurs independently from capture health.
- Lock/suspend clears protected view models and keys before optional long work.
- Unexpected termination remains recoverable through storage reconciliation; graceful shutdown is not a durability prerequisite.

## 9. Required evidence

- delayed-render owner that sleeps, re-enters, exits, and ignores cancellation;
- standard Win32 render callback that blocks;
- `IStream` partial reads and never-ending stream;
- overlay/tray/hotkey responsiveness while capture STA is blocked;
- bounded queue under rapid updates;
- medium/handle leak counts;
- clean shutdown at every capture phase;
- capture broker comparison if in-process recovery misses acceptance criteria.
