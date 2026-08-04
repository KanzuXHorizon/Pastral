# Phase 3H Read-Only History IPC Foundation Verification

**Date:** 2026-08-05

**Design:** `docs/superpowers/specs/2026-08-05-pastral-read-only-history-ipc-design.md`

**Plan:** `docs/superpowers/plans/2026-08-05-pastral-read-only-history-ipc.md`

**Scope:** storage-owned bounded preview queries, exact authenticated read capabilities, feature-gated agent read serving, aggregate frame-budget enforcement, dependency/source policy, and end-to-end Windows named-pipe evidence

## 1. Result

Phase 3H is **green for the declared agent-side read-only History/Search foundation**.

The verified vertical slice is:

```text
SQLite / FTS5 metadata and search projection
  -> bounded storage read model
  -> exact Health + HistoryPage + Search capability set
  -> same-user/session protected named pipe
  -> kernel peer validation + mutual HMAC authentication
  -> validated Protobuf control DTOs
  -> feature-gated pastral-agent-ipc serve-read
  -> bounded metadata-only History/Search response
```

The manager still does not open SQLite, FTS, clipboard APIs, or blob storage directly. The existing manager C ABI remains Health-only. Phase 3H establishes the backend boundary that a later caller-owned page-buffer ABI and WinUI mapping can consume.

## 2. Commits

```text
2a41723 docs: specify read-only history IPC foundation
af4dcc0 feat: add bounded storage history pages
9d7f2d2 feat: authenticate read-only IPC capabilities
c5d5ddd feat: serve read-only history over IPC
b48fe3b fix: bound read IPC responses to frame limits
54780b7 test: verify read IPC binary cross-process
7f85188 test: move read IPC process evidence to probe
7b98072 test: smoke read IPC admission in release
```

The final documentation/evidence commit follows this report.

## 3. Storage read model

`pastral-storage` owns the query semantics through `ClipListItem`, `ClipPage`, `Storage::history_page`, and `Storage::search_page`.

Verified properties:

- History ordering is `capture_order DESC, clip_event_id ASC`.
- History cursor semantics are exclusive: `capture_order < before_capture_order`.
- Page limits are exactly `1..=100`.
- Queries fetch `limit + 1` rows so `has_more` is truthful without a separate count query.
- Search reuses the existing literal FTS query builder; user text is not interpreted as raw FTS syntax.
- Search is ordered by the same newest-first deterministic order as History.
- Results contain event ID, capture order, observed timestamp, and bounded search projection only.
- Missing search projections become explicit unavailable rows instead of causing blob reads or fabricated text.
- Storage preview output is truncated to at most 4,096 UTF-8 bytes on a valid character boundary.
- The read path does not load representation payloads or external blob files.
- Empty, oversized, and excessive-term search queries remain rejected by the existing DTO/storage limits.

Focused storage coverage includes cursor pagination, literal query behavior, unavailable rows, `has_more`, page-limit rejection, and multi-byte UTF-8 truncation.

## 4. Capability-aware authentication

The Windows transport now supports explicit capability sets while retaining the old Health-only wrappers.

New reusable functions:

```text
client_handshake_with_capabilities
server_handshake_with_capabilities
```

The exact offered/accepted capability sequence is bound into the authenticated transcript. The read server negotiates exactly:

```text
Health
HistoryPage
Search
```

The existing `client_handshake` and `server_handshake` wrappers still advertise and accept only Health. Existing admission, manager bridge, and transport probes therefore do not gain new authority implicitly.

Tests prove:

- Health-only behavior remains unchanged;
- the exact read-only set authenticates successfully;
- mismatched capability sets fail before an authenticated connection exists;
- control frames before authentication remain rejected;
- replay and wrong-secret protections remain unchanged.

## 5. Agent command boundary

The feature-gated `pastral-agent-ipc.exe` exposes two explicit modes:

```text
serve-health --data-root <path> [--max-connections <1..16>]
serve-read   --data-root <path> [--max-connections <1..16>]
```

`serve-health` remains Health-only. `serve-read` authorizes Health, HistoryPage, and Search only.

Both modes preserve the existing strict behavior:

- missing, duplicate, empty, positional, unknown, and out-of-range arguments fail before server creation;
- first-instance pipe collision fails before readiness;
- every connection has bounded connect, handshake, read, and write deadlines;
- one authenticated request is served per connection;
- the process exits after the configured connection bound;
- normal output contains readiness and connection-count markers only;
- data-root paths, search queries, previews, secrets, nonces, proofs, SIDs, and pipe names are not printed.

`serve-read` is a diagnostic candidate. It is not auto-started and is not yet integrated into the default clipboard-owning resident lifecycle.

## 6. Request and error behavior

The read server maps operations as follows:

- `Health` reloads the real content-free agent Health snapshot.
- `HistoryPage` validates the DTO and queries one bounded storage page.
- `Search` validates the literal query DTO and queries one bounded result page.
- malformed authenticated control bodies return `InvalidRequest` with no developer detail;
- invalid search input returns `InvalidRequest` without echoing the query;
- request/resource limit failures return content-free protocol errors;
- storage open/query failures return retryable internal errors without paths or database details;
- unsupported operations are not authorized by the negotiated capability set.

No error response contains preview or query content.

## 7. Aggregate control-frame budget hardening

The initial read implementation correctly bounded each page to 100 items and each preview to 4,096 bytes, but those independent limits allowed a worst-case encoded response larger than the 256 KiB `ControlProto` frame ceiling:

```text
100 items × 4,096 preview bytes > 256 KiB
```

An end-to-end regression test with 101 long UTF-8 previews reproduced the failure as a disconnected client: the validated response could not be placed in a legal control frame.

The server now applies a second aggregate bound before writing the response:

1. map the full requested page;
2. encode the response against the default control-frame budget;
3. binary-search the largest per-preview byte allowance that keeps the whole encoded body at or below 256 KiB;
4. truncate every available preview on a valid UTF-8 boundary;
5. retain all requested rows and the original truthful `has_more` state.

Rows are not silently dropped. This matters especially for Search because the current Search request has no continuation cursor. Unavailable rows remain empty and explicit.

The regression test proves that a 100-row response with long multi-byte previews:

- returns all 100 rows;
- remains readable through the real authenticated pipe/frame stack;
- reports `has_more` when the 101st result exists;
- keeps every preview within the 4,096-byte item ceiling;
- reduces previews as necessary to satisfy the aggregate frame ceiling;
- preserves valid UTF-8 boundaries.

## 8. End-to-end evidence

`apps/agent/tests/ipc_read.rs` covers three real authenticated server scenarios:

1. Health, paged History, unavailable projection, literal Search, deterministic newest-first ordering, and content-free process output.
2. Malformed authenticated request body returning a content-free `InvalidRequest` response with the original correlation ID.
3. Worst-case 100-row long-preview response remaining inside the 256 KiB control-frame budget without dropping rows.

`apps/agent-ipc-probe/tests/cross_process.rs` adds a fourth scenario through the repository's reviewed process-spawn boundary. The probe starts a distinct read-server child, negotiates the exact Health/HistoryPage/Search capability set, serves three authenticated requests, verifies empty bounded History/Search results on a disposable root, emits content-free markers only, and exits at the configured connection bound. The public `serve-read` CLI shape remains covered separately by strict agent CLI tests. `verify-agent-ipc-admission.ps1 -Mode All` now repeats this read path against the optimized Release probe and fails if any operation, PID/session evidence, output privacy check, or child exit check is invalid.

The tests use:

- a disposable real storage root;
- real DPAPI-backed transport material;
- the current logon/session identity;
- the protected named-pipe transport;
- exact capability-aware mutual authentication;
- validated request/response schema conversion;
- bounded process/thread cleanup.

No automated test reads or writes the user's clipboard.

## 9. Aggregate verification

Fresh `.\eng\build.ps1 -Task All` result after `7f85188`:

```text
Rust 1.97.1 / x86_64-pc-windows-msvc: PASS
rustfmt: PASS
cargo check --locked --workspace --all-targets: PASS
cargo test --locked --workspace --all-targets: 238 PASS
Clippy --all-features -D warnings: PASS
cargo doc --no-deps: PASS
dependency policy: PASS
source policy: PASS
```

Test breakdown:

| Package/surface | Tests |
|---|---:|
| Agent, including Health/read IPC | 24 |
| Agent core | 18 |
| Agent IPC admission | 12 |
| Clipboard Win32 | 22 |
| Domain | 26 |
| IPC authentication | 8 |
| IPC core | 30 |
| IPC schema probe | 3 |
| IPC schema | 11 |
| IPC transport probe | 3 |
| IPC Win32 transport/process evidence | 29 |
| Manager IPC bridge | 15 |
| Storage | 37 |
| **Total** | **238** |

The agent total increased from 21 in Phase 3G to 24 through the positive read, malformed-request, and aggregate frame-budget tests. Agent IPC admission increased from 11 to 12 through the reviewed cross-process read probe. IPC Win32 increased from 28 to 29 through exact read-capability negotiation. Storage increased from 33 to 37 through the bounded read-model tests. The manager bridge total is 15 because the adjacent committed read-ABI layout test is included in this fresh aggregate run; live manager History/Search integration remains outside this foundation report.

Fresh Release admission evidence after `7b98072`:

```text
agent-ipc-admission=ok
admission-ceilings=passed
default-agent-binary-bytes=2142208
admission-binary-bytes=2467840
binary-delta-bytes=325632
working-set-delta-bytes=622592
private-delta-bytes=77824
agent-ipc-read=ok
health=ok
history=ok
search=ok
```

The Health and read probes each used distinct client/server processes in the same validated logon session. The read probe emitted no path, pipe, query, preview, SID, secret, nonce, proof, or clipboard marker.

## 10. Dependency and source policy

The default `pastral-agent` dependency graph remains Protobuf- and transport-free. The read server is available only through the existing `ipc-health` feature and pulls only the reviewed packages:

```text
pastral-ipc-auth
pastral-ipc-core
pastral-ipc-schema
pastral-ipc-win
pastral-storage
```

Policy verification confirms:

- official Protobuf `4.35.0-release` remains isolated to feature-gated IPC/probe/bridge surfaces;
- no async runtime or network stack was added;
- no alternate database, JSON, logging, or process-spawn framework was added;
- process spawning remains confined to the existing reviewed `agent-ipc-probe` diagnostic boundary;
- storage remains Windows-binding-free;
- the agent and IPC transport use only the pinned reviewed `windows-sys`/`windows-link` boundary;
- the manager bridge dependency graph is unchanged;
- `verify-manager-ipc-bridge.ps1 -Mode Static` remains green.

## 11. Scope still open

Phase 3H does not implement:

- caller-owned bounded page-buffer exports in `pastral-manager-ipc-bridge`;
- C++ mapping from the fixed ABI into manager presentation state;
- live WinUI History/Search rendering;
- manager-side cancellation/debounce and stale-generation tests for page/search requests;
- Search continuation/cursor semantics;
- production resident startup, supervision, reconnect/backoff, crash recovery, or shutdown integration;
- simultaneous clipboard listener plus IPC server budget/soak evidence;
- generated C++ Protobuf runtime parity;
- parser/schema fuzzing or adjacent-version fixtures;
- raw payload/bulk transfer;
- paste replay, Quick Paste, overlay, encryption, packaging, signing, or updates.

## 12. Next acceptance gate

The next slice should connect this backend to the manager without transferring Rust ownership across the ABI.

It must prove:

1. fixed-version caller-owned History/Search page buffers with exact size/version validation;
2. no Rust pointer, allocation, iterator, or ownership crossing the C boundary;
3. exact mapping of event ID, capture order, timestamp, kind, preview bytes, source label, pinned, unavailable, and `has_more`;
4. bounded UTF-8 to UTF-16 conversion with explicit invalid-data failure;
5. off-XAML-thread calls, cancellation/debounce, generation-based stale-result rejection, and clear-on-disconnect behavior;
6. Release manager displays only live backend rows and never opens storage directly;
7. empty, loading, disconnected, protocol-mismatch, error, unavailable, no-results, and partial/truncated-preview states remain accessible and localized;
8. aggregate Rust, bridge ABI/export, native build, UI Automation, dependency, source, and Git-integrity gates remain green.

Paste, mutation, bulk payload transfer, and packaging remain blocked until the read-only manager slice is green.
