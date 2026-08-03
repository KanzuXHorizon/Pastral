# Pastral storage and lexical-search foundation design

**Status:** Approved for autonomous implementation by the 2026-08-04 continuation request.
**Date:** 2026-08-04
**Baseline:** `6d5f3b6`

## 1. Decision

Build Phase 2A as a synchronous, single-owner Rust storage crate that persists the Phase 1 immutable domain model in SQLite, stores ordinary payloads through one logical blob contract with internal-SQLite and external-file physical placements, and provides bounded deterministic FTS5 lexical search.

Do not begin Win32 clipboard capture, COM/OLE, IPC, executable processes, WinUI, encryption, packaging, telemetry, OCR, semantic search, or AI in this slice.

Three credible sequencing options were evaluated:

1. Implement storage, Windows capture, IPC, and UI together. Rejected because failures would cross too many trust and recovery boundaries and could not remain green in small commits.
2. Implement SQLite-only storage and hardcode a size threshold. Rejected because repository authority requires one backend-neutral blob contract and benchmark-versioned placement rather than a universal threshold.
3. Implement storage/search foundation with explicit caller-supplied placement policy, internal and external ordinary-payload backends, transactional metadata, bounded search, integrity checks, and reconciliation. Selected because it creates the durable contract required by later capture work without claiming unmeasured policy choices.

## 2. Authority and research constraints

Normative repository sources:

- `docs/architecture/data-model.md`
- `docs/architecture/blob-store-lifecycle.md`
- `docs/architecture/clipboard-event-identity.md`
- `docs/adr/0006-sqlite-blob-store.md`
- `docs/adr/0016-data-identity-time-and-digests.md`
- `docs/security/privacy-model.md`
- `docs/testing/strategy.md`
- `docs/release/checklist.md`

External primary-source findings applied to this design:

- `rusqlite 0.40.1` is the selected pinned wrapper and its bundled build contains SQLite 3.53.2.
- SQLite 3.53.3 fixed a WAL-reset corruption defect after 3.53.2.
- Therefore Phase 2A explicitly selects rollback journal `DELETE` with `synchronous=FULL`; WAL remains unaccepted until the bundled SQLite version and benchmark/crash evidence satisfy the later storage prototype gate.
- FTS5 contentless-delete tables support ordinary `DELETE` and are preferred over legacy contentless tables when backward compatibility is not required.
- Incremental BLOB I/O requires a pre-sized `zeroblob` row and an open blob handle whose row must not be modified while the handle is active.

## 3. Scope

### Included

- New crate `crates/storage` named `pastral-storage`.
- Explicit storage errors, limits, placement policy, write/read/search DTOs, and runtime information.
- SQLite schema version 1 with explicit enum tags and UUID bytes.
- Ordinary payload persistence using:
  - internal SQLite BLOB placement;
  - external staged file placement under a controlled data root.
- Ordinary deduplication by compatible protection-domain ID, digest suite, digest bytes, and exact raw length without merging `ClipEvent` rows.
- Transactional persistence of `ClipEvent`, `ClipRepresentation`, capture audit events, and bounded allowed search projections.
- FTS5 literal lexical search with parameter binding, deterministic ordering, and no snippets generated from payload storage.
- Exact payload readback with length and `sha256-raw-v1` verification.
- Event deletion with shared-reference accounting and retryable external deletion state.
- Bounded startup reconciliation for staging files, unreferenced external objects, missing locators, and pending external deletions.
- SQLite and FTS integrity checks.
- Unit/integration tests using synthetic disposable roots.
- CI, build-script, developer setup, changelog, README, and milestone evidence updates.

### Excluded

- Sensitive or Private plaintext persistence. Commits containing `Sensitive` or `Private` representations are rejected until authenticated encryption exists.
- Key storage, DPAPI, encryption envelopes, reveal authorization, or Private-profile activation.
- Automatic threshold selection or a production default placement threshold.
- WAL, checkpoint policy, background maintenance threads, async runtimes, multi-process access, or connection pooling.
- Streamed clipboard input; Phase 2A accepts bounded owned bytes so persistence invariants can be proven before Win32/OLE acquisition exists.
- Import/export, verified backup/restore, schema downgrade, backend migration, retention/quota policy, pin/tags/annotations, OCR, semantic indexes, typo indexes, or ranking claims.
- Win32, COM/OLE, named pipes, Protobuf, executables, UI, installer, signing, or network product code.

## 4. Dependency decision

Workspace dependency:

```toml
rusqlite = { version = "=0.40.1", default-features = false, features = ["bundled", "blob"] }
```

Rationale:

- `bundled` gives a reproducible reviewed SQLite build on Windows and avoids an unrecorded system SQLite or vcpkg dependency.
- `blob` exposes incremental BLOB I/O for the internal backend.
- Default features are disabled to avoid unrelated cache/WASM behavior.
- No `chrono`, `time`, `serde`, `uuid` integration, URL, CSV, load-extension, hooks, backup, or async feature is enabled.
- Domain UUIDs remain encoded explicitly as 16-byte values; storage does not use rusqlite UUID serialization features.

No new dependency is added for hex encoding, temporary directories, filesystem traversal, errors, logging, or async execution. Small required helpers use `std`.

## 5. Crate and file boundaries

```text
crates/storage/
├─ Cargo.toml
└─ src/
   ├─ lib.rs             # public exports and crate policy
   ├─ error.rs           # StorageError and source/error conversions
   ├─ config.rs          # StorageLimits, policy version, placement interfaces
   ├─ ids.rs             # UUID byte conversion and blob object identity helpers
   ├─ schema.rs          # schema v1, migrations, pragma verification
   ├─ encoding.rs        # explicit stable enum/format/domain encodings
   ├─ commit.rs          # ClipCommit, RepresentationPayload, SearchProjection validation
   ├─ blob.rs            # logical blob identity, internal/external write/read helpers
   ├─ repository.rs      # Storage facade, transactional clip/audit operations
   ├─ search.rs          # bounded literal FTS5 query construction and result mapping
   ├─ maintenance.rs     # integrity and bounded reconciliation
   └─ test_support.rs    # synthetic disposable root helper, compiled only for tests
```

Files stay focused. SQL schema text remains in `schema.rs`; filesystem lifecycle remains in `blob.rs`/`maintenance.rs`; public command semantics remain in `repository.rs`.

## 6. Public interfaces

### Placement policy

```rust
pub struct StoragePolicyVersion(u32);

pub enum BlobPlacement {
    InternalSqlite,
    ExternalFile,
}

pub struct BlobPlacementContext {
    pub raw_length: u64,
    pub protection_domain: ProtectionDomain,
    pub format: ClipboardFormatIdentity,
}

pub trait BlobPlacementPolicy {
    fn version(&self) -> StoragePolicyVersion;
    fn select(&self, context: &BlobPlacementContext) -> BlobPlacement;
}
```

No default threshold implementation is exposed. Tests use a fixed policy. Later benchmark evidence may introduce a versioned policy without changing repository semantics.

### Limits

```rust
pub struct StorageLimits {
    max_payload_bytes: u64,
    max_search_projection_bytes: usize,
    max_search_query_bytes: usize,
    max_search_terms: usize,
    reconciliation_entry_limit: usize,
}
```

Construction rejects zero values. Limits are caller-supplied and observable. Test helpers use explicit values; this design does not label them production-optimal.

### Commit input

```rust
pub struct RepresentationPayload {
    representation_id: ClipRepresentationId,
    bytes: Vec<u8>,
}

pub struct SearchProjection {
    text: String,
}

pub struct ClipCommit {
    event: ClipEvent,
    payloads: Vec<RepresentationPayload>,
    search_projection: Option<SearchProjection>,
}
```

Validation requires exactly one payload for each captured representation, no unknown or duplicate representation ID, exact raw length, exact permitted digest, and ordinary protection domain. A search projection is accepted only for an ordinary event and must be valid UTF-8 `String`, nonempty after Unicode whitespace trimming, NUL-free, and within the configured byte limit.

### Storage facade

```rust
pub struct Storage<P: BlobPlacementPolicy> { /* single-owner state */ }

impl<P: BlobPlacementPolicy> Storage<P> {
    pub fn open(root: impl AsRef<Path>, limits: StorageLimits, policy: P) -> Result<Self, StorageError>;
    pub fn runtime_info(&self) -> Result<StorageRuntimeInfo, StorageError>;
    pub fn commit_clip(&mut self, commit: ClipCommit) -> Result<CommitReceipt, StorageError>;
    pub fn load_clip(&self, id: ClipEventId) -> Result<Option<StoredClip>, StorageError>;
    pub fn read_representation(&self, id: ClipRepresentationId) -> Result<Option<Vec<u8>>, StorageError>;
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, StorageError>;
    pub fn insert_audit_event(&mut self, event: CaptureAuditEvent) -> Result<(), StorageError>;
    pub fn delete_clip(&mut self, id: ClipEventId) -> Result<DeleteReceipt, StorageError>;
    pub fn reconcile(&mut self) -> Result<ReconciliationReport, StorageError>;
    pub fn integrity_check(&self) -> Result<IntegrityReport, StorageError>;
}
```

`Storage` includes an `Rc` ownership marker so it is neither `Send` nor `Sync`. The future agent creates and uses it on the serialized storage-owner thread.

### Stored outputs

`StoredClip` reconstructs the domain `ClipEvent` plus representation metadata and blob placement summaries. It never exposes absolute file paths or SQLite row IDs. `SearchHit` contains `ClipEventId`, `CaptureOrder`, observed UTC time, and aggregate fidelity only; content snippets are excluded from this foundation.

## 7. Schema version 1

`PRAGMA user_version = 1` is the schema authority.

### `clip_events`

- `clip_event_id BLOB PRIMARY KEY` — exactly 16 bytes.
- `observed_at_utc_us INTEGER NOT NULL`.
- `capture_order INTEGER NOT NULL UNIQUE CHECK(capture_order > 0)`.
- `captured_profile_id BLOB NOT NULL CHECK(length(...) = 16)`.
- `protection_kind INTEGER NOT NULL`.
- `protection_domain_id BLOB NOT NULL CHECK(length(...) = 16)`.
- `aggregate_fidelity INTEGER NOT NULL`.
- `fidelity_version INTEGER NOT NULL`.

### `blob_objects`

- `blob_object_id BLOB PRIMARY KEY` — random UUIDv4 storage identity.
- `protection_kind`, `protection_domain_id`.
- `digest_suite INTEGER`, `digest BLOB`, and `raw_length INTEGER`.
- `backend_kind INTEGER`.
- `internal_payload BLOB` or `external_key TEXT`, enforced by a check constraint.
- `storage_policy_version INTEGER`.
- `reference_count INTEGER NOT NULL CHECK(reference_count >= 0)`.
- `lifecycle_state INTEGER NOT NULL` (`Ready` or `PendingDelete`).
- Partial unique ordinary-dedup index over domain ID, digest suite, digest, and raw length when digest is present and state is `Ready`.

### `clip_representations`

- representation ID primary key and clip-event foreign key.
- stable standard numeric format or registered exact name, mutually exclusive.
- protection domain, raw length, optional digest, fidelity.
- blob-object foreign key.
- no payload bytes or physical absolute path.

### `search_documents` and `clip_search_fts`

- `search_documents` maps an integer row key to one clip event and stores the allowed bounded projection text.
- `clip_search_fts` is an FTS5 contentless-delete table containing only the indexed body.
- Both rows are inserted/deleted in the same SQLite transaction as the owning clip.
- Search joins FTS row IDs to `search_documents` and `clip_events`.
- No Sensitive or Private row is accepted.

### `capture_audit_events`

Contains only audit ID, kind, coarse/allowed UTC time, optional capture order, profile ID, and result code. The schema has no payload, preview, digest, snippet, title, URL, path, domain, size, image, or blob-reference column.

## 8. SQLite runtime policy

On every open:

- verify bundled SQLite reports FTS5 support;
- `PRAGMA foreign_keys = ON` and verify it is enabled;
- `PRAGMA journal_mode = DELETE` and verify the returned mode;
- `PRAGMA synchronous = FULL` and verify numeric value;
- set a bounded busy timeout;
- run schema migration in one transaction;
- reject a database whose `user_version` is newer than 1;
- do not enable extension loading;
- do not enable WAL;
- expose SQLite and rusqlite-relevant runtime values through `StorageRuntimeInfo`.

This is a prototype correctness selection, not a final benchmark claim. Later SQLite upgrades and journal-mode evidence may amend ADR 0006.

## 9. Blob lifecycle

### Logical identity

Ordinary payload identity is `sha256-raw-v1` plus exact raw length and exact ordinary protection-domain ID. Format identity remains separate metadata. Blob reuse never merges clip events.

### Internal placement

- Insert `zeroblob(raw_length)` and metadata inside the clip transaction.
- Open a writable incremental BLOB handle and write exact bytes.
- Close the handle before modifying the blob row or committing.
- Verify byte count; readback integrity is checked on demand.

Zero-length payloads are supported through ordinary bound BLOB insertion because incremental BLOB handles cannot represent useful write work for length zero.

### External placement

- Create `.staging/<uuid>.tmp` with `create_new`.
- Write exact owned bytes, `flush`, `sync_all`, and close.
- Construct a controlled relative final key from the ordinary protection-domain UUID plus digest hex; user content and format names never appear in paths.
- Create final parent directories under `objects/`.
- In the serialized owner flow, check compatible dedup before finalization.
- Atomically rename staging to final when unique.
- Insert the authoritative relative key and metadata in SQLite.
- On transaction failure after finalization, attempt immediate cleanup; bounded reconciliation removes any surviving unreferenced object.

All path resolution rejects absolute paths, parent traversal, unexpected separators in stored keys, and resolution outside the configured root.

## 10. Search semantics

`search(query, limit)` is literal lexical search, not a general FTS query language.

- Query must be nonempty, NUL-free, within byte and term limits.
- Unicode whitespace separates terms.
- Every term is emitted as an FTS5 quoted string with embedded quotes doubled.
- Terms are joined with `AND`.
- The final MATCH expression is passed as a bound parameter.
- Results order by `capture_order DESC`, then stable clip ID bytes.
- `limit` must be positive and is clamped to an explicit safe upper bound derived from `StorageLimits`.
- No snippet/highlight function is exposed in this slice.

## 11. Deletion and reconciliation

Deleting a clip is transactional for SQLite metadata, FTS rows, representation rows, and reference counts.

- Shared blobs remain when references are nonzero.
- Internal zero-reference blobs are deleted in the transaction.
- External zero-reference blobs move to `PendingDelete`; after commit, physical deletion is attempted, then the row is removed in a second transaction.
- If AV/reader contention prevents deletion, the pending row remains and `reconcile()` retries without resurrecting the clip.

Reconciliation is bounded by `reconciliation_entry_limit` and reports counts rather than content:

- stale staging entries removed;
- pending external deletes completed or retained;
- unreferenced final object files removed;
- external locators missing their file;
- locator paths rejected as unsafe;
- reference-count mismatches;
- truncated status when the entry limit is reached.

No full digest scan occurs automatically.

## 12. Errors and failure behavior

`StorageError` is a manual `std::error::Error` enum and includes:

- invalid limits or policy version;
- unsupported/newer schema;
- SQLite/runtime pragma mismatch;
- FTS5 unavailable;
- invalid commit cardinality or payload mapping;
- payload length/digest mismatch;
- protected payload requires encryption;
- search projection/query violation;
- duplicate event/representation/capture order;
- unsafe external locator;
- missing/corrupt blob;
- integrity mismatch;
- bounded I/O and SQLite source errors with non-content context.

Errors never include payload bytes or search projection text in `Display` output.

## 13. Observable acceptance criteria

- Workspace builds on pinned Windows Rust 1.97.1 with MSVC.
- `pastral-domain` remains platform-independent and free of SQLite/filesystem dependencies.
- `pastral-storage` has `#![forbid(unsafe_code)]` and no async, network, serde, chrono/time, logging backend, Windows, IPC, UI, or encryption dependency.
- SQLite opens with `foreign_keys=ON`, rollback `DELETE`, `synchronous=FULL`, FTS5 available, and schema version 1.
- Internal and external ordinary payload round trips preserve exact bytes and domain metadata.
- Identical bytes in the same ordinary protection domain reuse one blob while retaining separate clip events.
- Identical bytes in different ordinary protection-domain IDs do not deduplicate.
- Sensitive and Private payload commits fail before plaintext persistence or indexing.
- Missing, duplicate, unknown, wrong-length, or wrong-digest payload mapping is rejected atomically.
- FTS literal search returns only explicitly indexed ordinary clips and resists operator/quote interpretation.
- Audit schema and API cannot carry payload, digest, preview, or blob reference.
- Delete preserves shared blobs and eventually removes zero-reference external files through bounded retry.
- Integrity and reconciliation report corruption/missing files without reading unrelated payloads into logs.
- Fresh format/check/test/Clippy/doc, SQLite integrity, FTS integrity, dependency-tree, diff, secret, scope, and Git checks pass.
- Commits are local, small, meaningful, and green; no remote push or PR occurs.

## 14. Required tests

- invalid zero storage limits and zero policy version rejected;
- runtime pragma and SQLite/FTS version information reported;
- schema creation and reopen preserve version;
- newer schema rejected;
- internal exact-byte round trip including empty payload;
- external exact-byte round trip and controlled relative key;
- payload count, duplicate ID, unknown ID, missing ID, length mismatch, and digest mismatch rejected;
- capture-order and ID uniqueness violations roll back all associated rows/files;
- same-domain ordinary dedup shares blob but keeps two events;
- cross-domain identical ordinary bytes use different blobs;
- Sensitive/Private commits and search projections rejected before persistence;
- explicit ordinary search projection indexed; absent projection not indexed;
- quotes, `OR`, `NOT`, `*`, parentheses, and NUL cannot alter literal-query semantics;
- result order is deterministic by capture order and ID;
- audit insert/read count succeeds and table-column inspection proves forbidden fields absent;
- deleting one of two shared clips preserves payload; deleting final reference removes internal blob;
- external deletion failure state is retryable by reconciliation;
- stale staging and unreferenced external object cleanup are bounded;
- missing external locator is reported without deleting unaffected metadata;
- SQLite `quick_check` and FTS5 `integrity-check` succeed after representative operations;
- dependency graph contains no forbidden runtime category.

## 15. Commit batches

1. `docs: specify storage and lexical search foundation`
2. `feat: add storage contracts and schema foundation`
3. `feat: persist ordinary blobs and clip metadata`
4. `feat: add bounded lexical search and audit storage`
5. `feat: add deletion and storage reconciliation`
6. `ci: verify storage foundation`
7. `docs: record storage foundation evidence`

Every implementation commit includes its tests and is green. Red TDD evidence is recorded in the final milestone report but intentionally failing commits are not created.
