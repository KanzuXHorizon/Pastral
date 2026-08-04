# Pastral Manager Live History/Search Design

**Date:** 2026-08-05

**Status:** Approved for implementation

## Goal

Connect the authenticated read-only History/Search backend to the native manager without allowing C++ to open storage directly and without transferring Rust ownership across the C ABI.

## Scope

This slice implements:

- additive caller-owned C ABI exports for one History page and one literal Search page;
- strict ABI/version/size/capacity validation;
- exact mapping from validated IPC preview DTOs into fixed C structs plus a caller-owned UTF-8 arena;
- secure loading and validation of the new exports in the C++ manager;
- asynchronous live History/Search provider behavior with stale-result rejection;
- localized, accessible WinUI loading, connected, empty, no-results, unavailable, truncated, disconnected, protocol-mismatch, and error states;
- native probe, Rust tests, UI Automation, dependency/source policy, and evidence.

This slice does not implement mutations, paste, payload/blob transfer, bulk staging, resident lifecycle supervision, packaging, signing, encryption, or semantic search.

## Architecture

```text
Pastral.Manager History/Search UI
  -> persistent ManagerDataProvider worker
  -> ManagerIpcBridge C++ loader and mapping
  -> caller-owned fixed item array + caller-owned UTF-8 arena
  -> additive pastral-manager-ipc-bridge exports
  -> capability-aware authenticated read client
  -> pastral-agent-ipc serve-read
  -> bounded storage History/Search read model
```

The manager remains storage-blind. Rust may allocate internally while processing the query, but every allocation is consumed before the exported function returns. The ABI exposes only values copied into caller-owned memory.

## Compatibility

The existing Health contract remains unchanged:

- `PASTRAL_MANAGER_IPC_ABI_VERSION` remains `1`;
- `PastralManagerHealthResult` remains 64 bytes;
- the existing three Health exports remain exact and required for Health loading.

The new read contract is additive and independently versioned:

- `PASTRAL_MANAGER_READ_ABI_VERSION = 1`;
- `PastralManagerReadResult` is exactly 64 bytes;
- `PastralManagerClipItem` is exactly 64 bytes;
- dedicated size/version exports allow C++ to reject mismatched DLLs before use.

A manager may continue to use Health when the read exports are absent. Live History/Search is unavailable until every read export and exact contract size is present.

## C ABI

### Status

Existing status values `0..=8` remain unchanged. Add:

```text
PASTRAL_MANAGER_STATUS_INSUFFICIENT_BUFFER = 9
```

`InsufficientBuffer` is used only when an otherwise valid page cannot fit the caller-provided item array or text arena. It is not a transport error.

### Result

`PastralManagerReadResult` contains:

```text
uint32 abi_version
uint32 struct_size
uint32 status
uint32 item_count
uint32 has_more
uint32 required_item_capacity
uint32 required_text_capacity
uint32 server_process_id
uint32 session_id
uint32 reserved0
uint64 connect_us
uint64 handshake_us
uint64 request_us
```

The structure is 64 bytes and 8-byte aligned.

Rules:

- caller initializes `abi_version` and `struct_size`;
- bridge initializes every field on every non-null call;
- failed transport/protocol/authentication results have zero counts, zero capacities, zero peer/timing fields, and zero reserved fields;
- `InsufficientBuffer` returns exact required item and byte capacities but writes no item or text output;
- `Connected` returns item count, `has_more`, peer/timing evidence, and zero reserved fields;
- `item_count <= required_item_capacity <= 100`;
- `has_more` is exactly `0` or `1`.

### Item

`PastralManagerClipItem` contains:

```text
uint8 event_id[16]
uint64 capture_order
int64 observed_at_unix_micros
uint32 kind
uint32 flags
uint32 preview_offset
uint32 preview_length
uint32 source_offset
uint32 source_length
uint32 reserved0
uint32 reserved1
```

The structure is 64 bytes and 8-byte aligned.

Kinds:

```text
PASTRAL_MANAGER_CLIP_KIND_UNAVAILABLE = 0
PASTRAL_MANAGER_CLIP_KIND_TEXT = 1
```

Flags:

```text
PASTRAL_MANAGER_CLIP_PINNED = 1 << 0
PASTRAL_MANAGER_CLIP_UNAVAILABLE = 1 << 1
PASTRAL_MANAGER_CLIP_PREVIEW_TRUNCATED = 1 << 2
```

The event ID is canonical UUID bytes. `capture_order` and timestamp are validated domain values. Offsets and lengths index the caller-owned UTF-8 arena and are never pointers.

For absent source labels, offset and length are zero. For unavailable rows, kind and flag must agree, preview length must be zero, and source remains optional. Unknown kinds, flags, or nonzero reserved fields fail closed in C++.

### Exports

```text
pastral_manager_ipc_read_abi_version
pastral_manager_ipc_read_result_size
pastral_manager_ipc_clip_item_size
pastral_manager_ipc_history_w
pastral_manager_ipc_search_w
```

History arguments include data root, timeout, limit, optional exclusive `before_capture_order` encoded as zero for absent, item array/capacity, text arena/capacity, and result.

Search arguments additionally include a bounded UTF-16 query. Search remains literal and currently has no continuation cursor.

## Buffer protocol

The caller may make one sizing call with zero item/text capacity and null output buffers. If the page is nonempty, the bridge returns `InsufficientBuffer` with exact required capacities.

The caller allocates at most:

- 100 item structures;
- the exact required UTF-8 arena bytes reported by the bridge;
- a repository-enforced maximum arena of 256 KiB.

The caller repeats the query. The server may change between calls, so the second call is independently validated. If the new page is larger, the bridge returns a new `InsufficientBuffer`; the provider retries at most once. It never loops without a bound.

The bridge does not partially fill caller buffers on insufficient capacity. Before a successful copy it validates all offsets, lengths, total byte count, item count, status, and DTO invariants.

## Rust client behavior

The bridge uses the exact read capability set:

```text
Health
HistoryPage
Search
```

Each exported page query creates one authenticated connection, sends one validated request, receives one validated response, and closes. It maps:

- disconnect, timeout, protocol mismatch, authentication failure, and internal failure to the existing status taxonomy;
- protocol error responses without exposing developer detail;
- unexpected response variants to protocol mismatch;
- valid History/Search responses to immutable internal page snapshots.

The bridge trusts neither response count nor text lengths beyond the validated DTO and frame limits.

## C++ bridge behavior

The loader continues resolving the DLL beside `pastral-manager.exe` using the exact filename and restricted search flags.

Health availability and read availability are separate:

- `IsAvailable()` requires the existing Health exports and exact Health contract;
- `IsReadAvailable()` additionally requires all read exports and exact read/item sizes.

C++ performs sizing then fill with bounded vectors. It validates:

- return code equals result status;
- all version/size/reserved/status/flag/kind fields;
- item count and capacities;
- UUID not all-zero;
- nonzero capture order;
- valid UTF-8 slices and nonoverlapping in-range offsets;
- unavailable/kind/preview invariants;
- failed statuses contain no stale output.

UTF-8 is converted to UTF-16 with strict `MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, ...)`; invalid data fails the whole page closed.

## Provider and UI behavior

The provider retains one background worker. Requests have monotonically increasing generations.

- initial connected load fetches Health then first History page;
- refresh fetches Health and first History page;
- Search uses a 250 ms debounce and exact literal backend Search;
- empty query restores the latest first History page;
- a newer refresh/search generation supersedes older pending work;
- stale completion is discarded before XAML mutation;
- disconnect, authentication failure, protocol mismatch, or error clears live rows;
- Debug synthetic preview remains explicitly labeled and never mixes with live rows;
- Release never fabricates clips.

The initial page limit is 50. A later pagination slice may add explicit load-more UI; this slice preserves `has_more` in provider state and communicates partial results without pretending the page is complete.

## Presentation mapping

Each live row maps:

- UUID bytes -> canonical lowercase UUID string;
- validated UTF-8 preview -> UTF-16 safe preview;
- unavailable -> localized unavailable label and disabled paste affordance;
- source label -> source field when present, localized unknown source otherwise;
- timestamp -> deterministic local relative-time formatter;
- kind -> localized Text/Unavailable type;
- pinned flag -> pinned state;
- preview-truncated flag -> accessible truncated-preview announcement;
- profile -> Ordinary until encrypted profile support exists;
- representation summary -> localized preview-only description.

## Accessibility and localization

No new user-facing English string is hard-coded in the live History/Search path. English and Vietnamese resources cover:

- loading History;
- partial result count;
- no History yet;
- no Search results;
- unavailable preview;
- truncated preview;
- unknown source;
- backend not available;
- retry/refresh semantics.

Result counts use polite live regions. Search remains keyboard reachable. Unavailable rows have descriptive automation names and do not expose enabled paste actions.

## Security and privacy

- No direct manager storage access.
- No raw payload/blob transfer.
- No query or preview content in process logs, protocol error details, or verification output.
- No PATH/current-directory DLL search.
- No Rust pointer or allocation ownership across the ABI.
- Every output structure is initialized before failure return.
- Buffer insufficiency is explicit and content-free.
- Default resident agent remains Protobuf/transport-free.
- Automated gates do not read or write the user clipboard.

## Verification

Required gates:

- Rust ABI layout/header parity tests;
- invalid null/version/size/path/query/timeout/capacity tests;
- real `serve-read` History and Search round trips through both Rust API and C ABI;
- insufficient-buffer no-partial-write tests;
- exact export inspection;
- native C++ ABI probe for sizing/fill/UTF-8 mapping;
- manager Release UI Automation for connected History, literal Search, no-results, unavailable row, disconnect clear, and clean close;
- `cargo fmt`, focused tests, workspace tests, Clippy, docs, dependency/source policy;
- native static/build/smoke gates and `git diff --check`.

## Deferred

Production resident lifecycle, pagination beyond the first page, Search continuation, mutation, paste, bulk payload transfer, encryption, Quick Paste, overlay, packaging, signing, and updates remain separate acceptance gates.
