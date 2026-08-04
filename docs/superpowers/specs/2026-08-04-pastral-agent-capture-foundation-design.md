# Pastral Agent Capture Foundation Design

**Date:** 2026-08-04  
**Phase:** 3B  
**Status:** Approved by continuation directive and existing Pastral architecture authority

## 1. Goal

Build the first resident-agent vertical slice that can transform a clipboard update into a bounded, immutable ordinary-text capture and persist it through the existing SQLite/FTS5 storage boundary.

The slice must be useful without pretending that IPC, replay, overlay, encryption, source attribution, or broad format fidelity already exists. Automated tests must not write to or replace the user's clipboard.

## 2. Existing authority

This design implements the already approved process and data flow in:

- `docs/superpowers/specs/2026-08-03-pastral-foundation-design.md`;
- `docs/architecture/clipboard-event-identity.md`;
- `docs/architecture/threading-and-com-ownership.md`;
- `docs/testing/strategy.md`;
- `docs/security/privacy-model.md`.

The existing foundation already provides:

- immutable domain identities and representation invariants;
- synchronous single-owner SQLite/FTS5 storage;
- Win32 clipboard listener notifications;
- bounded `CF_UNICODETEXT` extraction with exact logical UTF-16 bytes;
- native manager UI that remains disconnected until versioned IPC exists.

## 3. Decision

**Decision:** introduce a platform-independent `pastral-agent-core` coordinator and a thin Windows `pastral-agent` runtime adapter.

**Why:**

1. Coordinator behavior can be tested with deterministic fake capture sources, clock, sleeper, and sink without mutating the system clipboard.
2. Win32 ownership and unsafe code remain isolated in `pastral-clipboard-win`; storage remains the only SQL owner.
3. IPC can later consume the same agent state without changing capture semantics or manager page contracts.

**Trade-offs accepted:**

- Phase 3B stores only ordinary `CF_UNICODETEXT`.
- Source application identity, private-browser/password-manager exclusion, advanced secret classification, OLE formats, replay markers, and overlay intents remain absent.
- The executable is diagnostic/manual rather than auto-started or packaged.

**Use the alternatives when:**

- A direct monolithic adapter is acceptable only for a throwaway prototype; it is rejected for product code because retry, duplicate suppression, and persistence would be difficult to test.
- IPC-first work resumes only after the bounded frame/schema/runtime evidence required by ADR 0018 is complete.

**Rollback:** remove the `apps/agent` and `crates/agent-core` workspace members and the additive storage capture-commit API. Existing domain, storage, clipboard, and manager APIs remain valid.

## 4. Scope

### 4.1 Included

- Storage-owned installation-local `capture_order` allocation for newly captured events.
- A `NewClipCommit` input that does not allow callers to invent capture order.
- A deterministic capture coordinator with:
  - transient clipboard sequence duplicate suppression;
  - bounded retry schedule;
  - exact ordinary Unicode-text payload handling;
  - no-representation and retry-exhausted outcomes;
  - explicit storage-failure outcome;
  - no background async runtime.
- A Windows adapter using the existing `ClipboardSession` and `CapturedUnicodeText` APIs.
- A `pastral-agent.exe` diagnostic executable with:
  - `health-check` mode;
  - explicit `capture-current` mode;
  - explicit `listen` mode;
  - caller-selectable data root;
  - no network, telemetry, auto-start, service registration, or packaging mutation.
- Static/dependency/source-policy gates and CI/build integration.
- Runtime smoke that performs health checks only; it must not read or alter the user's clipboard unless the operator explicitly invokes `capture-current` or `listen`.

### 4.2 Excluded

- Named-pipe IPC and manager live connection.
- Quick Paste, paste/replay, origin marker publication, and destination validation.
- COM/OLE `IDataObject`, HTML, RTF, images, files, virtual files, or application-private formats.
- Password-manager/private-browser source exclusion and source confidence.
- Sensitive/Private retention, encryption, secret indexing, OCR, semantic search, rules, retention, quota, backup, import/export, packaging, signing, and update.
- Auto-start registration or a Windows service.

## 5. Architecture

### 5.1 Storage capture input

Add `NewClipCommit` to `pastral-storage`. It contains:

- `ClipEventId`;
- `UtcUnixMicros` observation time;
- `ProfileId`;
- ordinary `ProtectionDomain`;
- one or more `ClipRepresentation` values;
- exact payload mappings;
- optional `SearchProjection`.

It intentionally omits `CaptureOrder`.

`Storage::commit_new_clip(NewClipCommit)` starts an immediate transaction, reads the current maximum durable clip/audit order, assigns the next positive order, constructs a valid `ClipEvent`, writes metadata/blobs/search projection, and returns an `AssignedCommitReceipt` containing the event ID and assigned order.

The existing `commit_clip(ClipCommit)` remains for explicit-order test/import/restore paths. Product capture code must use `commit_new_clip`.

### 5.2 Agent core

`pastral-agent-core` is Windows-binding free and defines:

- `CaptureSequence`: nonzero opaque notification evidence;
- `CapturedText`: decoded text plus exact logical UTF-16LE bytes;
- `CaptureSource`: attempts one bounded capture operation;
- `CaptureSink`: commits one ordinary text capture;
- `Clock`: provides UTC microseconds;
- `Sleeper`: performs retry delays;
- `CaptureCoordinator`: owns last successfully handled sequence and retry policy;
- `CaptureOutcome`: explicit result state.

The coordinator never opens SQLite, Win32 clipboard APIs, sleeps through a global async runtime, logs content, or stores preview text outside the commit request.

### 5.3 Windows runtime adapter

`apps/agent` depends on:

- `pastral-agent-core`;
- `pastral-clipboard-win`;
- `pastral-storage`;
- `pastral-domain`;
- pinned `windows-sys` only when an additional Win32 primitive is unavoidable.

The adapter:

1. receives `ClipboardNotification` from `ClipboardListener`;
2. opens a short-lived `ClipboardSession`;
3. requests `CF_UNICODETEXT` with a configured maximum byte limit;
4. maps exact bytes/text into `CapturedText`;
5. asks the coordinator to persist through a storage sink;
6. returns to waiting without polling.

No unsafe code is added outside the reviewed `pastral-clipboard-win/src/sys.rs` boundary in this phase.

## 6. Retry and duplicate semantics

Default retry delays are `[0 ms, 5 ms, 15 ms, 35 ms]`.

- The first attempt occurs immediately.
- Only transient clipboard-open/contention failures are retried.
- Unsupported/no-text results are terminal and are not retried.
- Invalid/corrupt Unicode capture is terminal and reported as capture failure.
- Storage failures are terminal and do not mark the sequence successful.
- A sequence is marked handled only after a terminal non-storage outcome or successful persistence.
- Repeating the same nonzero sequence after success returns `DuplicateNotification` without opening the clipboard again.
- Sequence value `0` is unavailable evidence and is never used as a durable identity.

## 7. Text representation

For `CF_UNICODETEXT`:

- standard format identity is `CF_UNICODETEXT` / numeric ID `13`;
- raw payload is the exact logical UTF-16LE byte sequence including its first aligned NUL terminator and excluding allocation padding;
- search projection uses the decoded Rust `String` without normalization;
- empty decoded text is persisted without an FTS projection;
- fidelity is `FullFidelity` for the captured representation;
- protection domain is the configured ordinary domain;
- no source name, process title, precise owner identity, or content-bearing log is generated.

## 8. Diagnostic executable behavior

`pastral-agent.exe` requires an explicit command:

- `health-check --data-root <path>` opens/migrates storage, runs runtime/integrity checks, prints content-free status, then exits.
- `capture-current --data-root <path>` explicitly reads the current clipboard once and reports a content-free outcome. It does not print clipboard text.
- `listen --data-root <path> [--max-events <n>]` starts the event-driven listener. `--max-events` supports bounded diagnostics; omitting it runs until the process is externally stopped.

No default command silently starts capture. Missing/invalid arguments return nonzero with usage text.

The default ordinary profile/domain identities are persisted in a small content-free configuration file under the chosen data root so captures remain in one protection domain across restarts. File creation uses create-new plus atomic rename semantics where practical. No secret is stored in this file.

## 9. Limits

Initial explicit diagnostic limits:

- maximum Unicode text payload: 16 MiB;
- maximum search projection: 1 MiB;
- maximum search query: 4 KiB;
- maximum search terms: 64;
- reconciliation traversal entries: 100,000;
- ordinary text blob placement: internal SQLite BLOB only for this slice;
- retry attempts: four;
- retry elapsed budget: 55 ms plus API execution time.

These are safe engineering limits, not final benchmark-derived product defaults.

## 10. Error handling

- Clipboard contention produces bounded retries and a content-free terminal outcome.
- Clipboard data/Unicode validation errors do not persist partial events.
- Storage errors do not mark a sequence successfully stored.
- Duplicate event IDs/order conflicts fail the transaction atomically.
- Invalid configuration identity files fail closed; they are not silently replaced.
- Diagnostic output includes error class and operation, never clipboard content or content hash.
- External process termination may interrupt `listen`; SQLite/blob recovery remains owned by storage reconciliation.

## 11. Testing

### 11.1 Agent-core unit tests

- immediate success performs one attempt and one sink commit;
- duplicate successful sequence performs no capture/sink work;
- transient failures follow exact retry delays and stop on success;
- retry exhaustion performs no sink commit;
- no supported representation is terminal without delay;
- storage failure leaves the sequence eligible for another notification;
- empty text produces no search projection;
- non-empty text preserves exact UTF-16 bytes and unnormalized search text.

### 11.2 Storage tests

- first new capture receives order `1`;
- subsequent capture receives the next order across reopen;
- failed commits do not consume an order;
- explicit-order/import commits and new captures cannot collide silently;
- returned receipt order matches loaded event order.

### 11.3 Windows adapter tests

Tests use fake `CaptureSource` for coordinator behavior. Existing clipboard-platform unit tests continue to validate Win32/HGLOBAL/Unicode behavior without replacing the user clipboard.

### 11.4 Executable smoke

- `health-check` against a disposable temporary root exits `0`;
- invalid/missing arguments exit nonzero;
- `capture-current` and `listen` are not run automatically in CI or aggregate local gates.

## 12. Verification gates

Before Phase 3B is complete:

- Rust formatting, check, all tests, Clippy `-D warnings`, and docs pass with `--locked`;
- dependency policy proves domain/storage/agent-core remain Windows-binding free;
- source policy rejects network/process APIs, unsafe outside the existing reviewed boundary, secrets, build output, WAL, extension loading, and database attachment;
- agent `health-check` smoke passes on a disposable root;
- native manager Debug/Release/UIA gates remain green;
- Git diff/integrity/clean-tree checks pass;
- hosted CI is configured, but a hosted run is not claimed unless actually observed.

## 13. Completion statement

Phase 3B proves that Pastral can safely coordinate and persist bounded ordinary Unicode-text captures through a real agent executable and existing native/storage boundaries. It does not prove a release-ready clipboard manager, live manager connection, sensitive-data safety, broad clipboard fidelity, replay, overlay, packaging, or auto-start.