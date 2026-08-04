# Pastral Read-Only History IPC Foundation Design

**Date:** 2026-08-05

**Status:** Approved for implementation by the user's standing instruction to continue incomplete evidence-gated slices on `main`

## 1. Purpose

This slice establishes the first bounded read-only History and literal Search backend over Pastral's authenticated local IPC stack. It is intentionally independent from the concurrent manager branding/UI work and does not claim the complete Phase 3H production lifecycle gate.

The delivered path is:

```text
SQLite metadata + content-free search projection
  -> bounded storage read model
  -> authenticated feature-gated agent read server
  -> existing Protobuf HistoryPage/Search DTOs
  -> bounded preview metadata only
```

## 2. Scope

### Included

- cursor-based recent History ordered by `capture_order DESC`;
- literal lexical Search ordered by `capture_order DESC`;
- `limit + 1` pagination evidence and truthful `has_more`;
- UTF-8-safe preview truncation to the existing IPC maximum of 4,096 bytes;
- unavailable History rows when no search projection exists;
- authenticated `Health`, `HistoryPage`, and `Search` operations through a new `serve-read` command;
- focused storage, protocol, agent, policy, and regression tests.

### Excluded

- manager C ABI changes, C++ DTO mapping, or WinUI integration;
- default resident-agent startup, supervision, reconnect, update handoff, or auto-start;
- paste, mutation, delete, pin, replay, payload transfer, bulk staging, or subscriptions;
- direct blob reads, raw clipboard payloads, source-process inference, or unrestricted previews;
- schema migration or new SQLite columns;
- semantic search, OCR, AI, or network access.

## 3. Compatibility boundary

The existing `serve-health` command remains Health-only. An authenticated History or Search request sent to that command is still rejected, preserving the current manager contract and Phase 3G negative tests.

A new command is added:

```text
pastral-agent-ipc serve-read --data-root <path> [--max-connections <1..16>]
```

`serve-read` advertises and authorizes exactly:

- `Health`;
- `HistoryPage`;
- `Search`.

One request is accepted per authenticated connection. The process exits after the configured bounded number of connections. It is a diagnostic/read-boundary candidate, not the production resident lifecycle.

## 4. Storage read model

Create a storage-owned read model that does not depend on `pastral-ipc-core`:

```rust
pub struct ClipListItem {
    clip_event_id: ClipEventId,
    capture_order: CaptureOrder,
    observed_at: UtcUnixMicros,
    preview: Option<String>,
}

pub struct ClipPage {
    items: Vec<ClipListItem>,
    has_more: bool,
}
```

APIs:

```rust
Storage::history_page(
    before_capture_order: Option<CaptureOrder>,
    limit: usize,
) -> Result<ClipPage, StorageError>

Storage::search_page(
    query: &str,
    limit: usize,
) -> Result<ClipPage, StorageError>
```

Rules:

- `limit` must be `1..=100`; storage rejects rather than silently widening the boundary.
- History cursor is exclusive: `capture_order < before_capture_order`.
- History uses `LEFT JOIN search_documents`; missing projection produces `preview = None`.
- Search uses the existing literal FTS query builder and only returns indexed projections.
- Both queries fetch at most `limit + 1` rows and remove the extra row to compute `has_more`.
- SQL reads only metadata and `search_documents.body`; it never joins blob payloads.
- Preview text is truncated on a valid UTF-8 boundary to at most 4,096 bytes before leaving storage.
- Order is deterministic: `capture_order DESC, clip_event_id ASC`.

## 5. IPC mapping

Storage rows map to `ClipPreviewDto` as follows:

- `preview = Some(text)` -> `ClipPreviewKind::Text`, text value, `unavailable = false`;
- `preview = None` -> `ClipPreviewKind::Unavailable`, empty text, `unavailable = true`;
- `source_label = None` because source metadata is not persisted in schema v1;
- `pinned = false` because pin state is not persisted in schema v1;
- event ID, capture order, and observed time are copied exactly.

No representation bytes are loaded to classify code/link/image/files. Classification remains truthful and conservative until a separate accepted metadata design exists.

## 6. Request handling and errors

`serve-read` decodes one request after the existing mutual authenticated handshake.

- `Health` returns the current real content-free Health snapshot.
- `HistoryPage` validates the existing DTO, reads one bounded page, and returns `HistoryPageResponseDto`.
- `Search` validates the existing DTO, runs literal search, and returns `SearchResponseDto`.
- malformed/unsupported input returns a content-free `ProtocolErrorDto` when a valid correlation ID exists.
- storage query validation maps to `InvalidRequest` or `ResourceLimit` without echoing the query.
- unexpected storage failures map to `Internal` with no path, SQL, payload, or secret detail.
- transport/authentication failures remain fail-closed and do not emit content-bearing diagnostics.

## 7. Security and privacy invariants

- The default `pastral-agent` dependency graph remains Protobuf/transport-free.
- Only the feature-gated IPC binary can expose this read boundary.
- The manager and bridge remain unable to open SQLite or blob storage directly.
- No raw payload, digest, object key, data root, pipe name, SID, secret, nonce, proof, SQL text, or search query is emitted in server markers or protocol errors.
- Existing privacy admission remains authoritative; this slice does not add a bypass for Sensitive/Private data.
- Automated tests use disposable synthetic roots and never read the system clipboard.

## 8. Verification

The slice is accepted when:

- storage tests prove cursor exclusivity, deterministic order, limit rejection, `has_more`, unavailable rows, literal search, and UTF-8-safe 4,096-byte truncation;
- existing storage search tests remain green;
- `serve-health` still rejects authenticated History requests;
- `serve-read` serves real Health, History, and Search across authenticated connections;
- responses contain no more than 100 previews and no preview exceeds 4,096 bytes;
- default agent tests and dependency isolation remain green;
- workspace format, check, tests, Clippy, docs, dependency policy, source policy, and `git diff --check` pass.

## 9. Follow-up gate

The next slice may extend the manager Rust C ABI with caller-owned bounded buffers and map read-only pages into C++ presentation state. Production resident lifecycle, cancellation, reconnect/backoff, simultaneous capture + IPC budgets, and update handoff remain mandatory before Phase 3H can be declared complete.
