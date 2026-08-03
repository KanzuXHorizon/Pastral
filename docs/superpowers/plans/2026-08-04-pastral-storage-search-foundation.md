# Pastral Storage and Lexical Search Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a verified single-owner SQLite/FTS5 storage foundation with internal/external ordinary blob placement, exact-byte integrity, bounded literal search, deletion, and reconciliation.

**Architecture:** Keep `pastral-domain` free of persistence concerns and add `pastral-storage` as a synchronous crate over one rusqlite connection and one controlled data root. The caller supplies a versioned blob-placement policy, while storage owns schema, transactions, exact encoding, filesystem lifecycle, search, integrity, and reconciliation.

**Tech Stack:** Rust 1.97.1, Edition 2024, `rusqlite = 0.40.1` with only `bundled` and `blob`, SQLite 3.53.2 rollback journal, FTS5, Windows PowerShell 5.1.

## Global Constraints

- Baseline is `6d5f3b6` on `main`; preserve unrelated files and keep the tree clean between commits.
- Use `#![forbid(unsafe_code)]` in both Rust crates.
- `pastral-domain` must not depend on SQLite, filesystem, Windows, async, network, UI, logging, or serialization.
- `pastral-storage` must not add tokio, serde, chrono/time, thiserror, tracing/log, Windows crates, IPC, UI, encryption, or network clients.
- Only ordinary payloads may be persisted. Sensitive and Private payloads must fail before plaintext write or indexing.
- Use rollback `journal_mode=DELETE`, `synchronous=FULL`, and `foreign_keys=ON`; do not enable WAL.
- No production default blob threshold is introduced; all placement comes from `BlobPlacementPolicy`.
- No remote push, PR, history rewrite, squash, installer, executable, or `Start-DevSpace-MCP-Cloudflared.ps1` commit.
- Write failing tests first, record red commands, then commit only green batches.

---

### Task 1: Lock the design and dependency boundary

**Files:**
- Create: `docs/superpowers/specs/2026-08-04-pastral-storage-search-foundation-design.md`
- Create: `docs/superpowers/plans/2026-08-04-pastral-storage-search-foundation.md`

**Interfaces:**
- Consumes: Phase 1 domain public API and repository storage/privacy authority.
- Produces: fixed crate boundaries, schema v1, public signatures, acceptance criteria, and commit batches.

- [ ] **Step 1: Scan the written spec and plan for unresolved placeholders**

Run:

```powershell
Select-String -Path docs/superpowers/specs/2026-08-04-pastral-storage-search-foundation-design.md,docs/superpowers/plans/2026-08-04-pastral-storage-search-foundation.md -Pattern '\b(TBD|TODO|FIXME|placeholder)\b'
```

Expected: no unresolved requirement placeholder. Incidental text explaining that placeholders are prohibited is acceptable only when complete.

- [ ] **Step 2: Verify baseline and document diff**

Run:

```powershell
git status --short
git diff --check
git diff -- docs/superpowers/specs/2026-08-04-pastral-storage-search-foundation-design.md docs/superpowers/plans/2026-08-04-pastral-storage-search-foundation.md
```

Expected: only the two new documents; `git diff --check` exits zero.

- [ ] **Step 3: Commit the approved design batch**

```powershell
git add docs/superpowers/specs/2026-08-04-pastral-storage-search-foundation-design.md docs/superpowers/plans/2026-08-04-pastral-storage-search-foundation.md
git commit -m "docs: specify storage and lexical search foundation"
```

---

### Task 2: Add storage contracts and schema foundation

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/storage/Cargo.toml`
- Create: `crates/storage/src/lib.rs`
- Create: `crates/storage/src/error.rs`
- Create: `crates/storage/src/config.rs`
- Create: `crates/storage/src/ids.rs`
- Create: `crates/storage/src/encoding.rs`
- Create: `crates/storage/src/schema.rs`
- Create: `crates/storage/src/test_support.rs`
- Modify: `crates/domain/src/id.rs`
- Modify: `crates/domain/src/lib.rs`

**Interfaces:**
- Consumes: domain typed IDs, `ProtectionDomain`, `ClipboardFormatIdentity`, `Fidelity`.
- Produces:
  - `BlobObjectId` in `pastral-domain`.
  - `StoragePolicyVersion::new(u32) -> Result<Self, StorageError>`.
  - `StorageLimits::new(...) -> Result<Self, StorageError>`.
  - `BlobPlacement`, `BlobPlacementContext`, `BlobPlacementPolicy`.
  - schema v1 creation/open verification and stable encode/decode helpers.

- [ ] **Step 1: Write failing contract and schema tests**

Add tests that call the not-yet-implemented APIs:

```rust
#[test]
fn zero_limits_and_policy_versions_are_rejected() {
    assert!(StoragePolicyVersion::new(0).is_err());
    assert!(StorageLimits::new(0, 1, 1, 1, 1).is_err());
}

#[test]
fn open_creates_schema_and_required_pragmas() {
    let root = TestRoot::new();
    let storage = Storage::open(root.path(), limits(), FixedPolicy::internal()).unwrap();
    let info = storage.runtime_info().unwrap();
    assert_eq!(info.schema_version, 1);
    assert_eq!(info.journal_mode, "delete");
    assert_eq!(info.synchronous, 2);
    assert!(info.foreign_keys);
    assert!(info.fts5_enabled);
}
```

- [ ] **Step 2: Run focused tests and capture red evidence**

```powershell
cargo test -p pastral-storage config schema -- --nocapture
```

Expected: compilation failure because the crate/APIs do not exist. Record the command and first relevant failure in the milestone notes.

- [ ] **Step 3: Add exact workspace dependency and crate manifest**

Use:

```toml
[workspace]
members = ["crates/domain", "crates/storage"]

[workspace.dependencies]
rusqlite = { version = "=0.40.1", default-features = false, features = ["bundled", "blob"] }
```

`crates/storage/Cargo.toml` depends on `pastral-domain` by path and `rusqlite.workspace = true`.

- [ ] **Step 4: Add `BlobObjectId` and storage value types**

Extend the existing typed-ID macro invocation with:

```rust
typed_id!(BlobObjectId);
```

Implement nonzero validated `StoragePolicyVersion`, nonzero `StorageLimits`, placement types, and a test-only fixed policy.

- [ ] **Step 5: Implement schema v1 and runtime verification**

Create tables and indexes exactly as the design specifies. Use `PRAGMA user_version`, reject versions greater than 1, configure and verify:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = DELETE;
PRAGMA synchronous = FULL;
```

Check FTS5 support through `pragma_compile_options` and create the contentless-delete FTS table.

- [ ] **Step 6: Run green quality gates for the batch**

```powershell
cargo fmt --all
cargo test -p pastral-domain --all-targets
cargo test -p pastral-storage --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
```

Expected: all pass.

- [ ] **Step 7: Commit the contracts/schema batch**

```powershell
git add Cargo.toml Cargo.lock crates/domain crates/storage
git commit -m "feat: add storage contracts and schema foundation"
```

---

### Task 3: Persist ordinary blobs and immutable clip metadata

**Files:**
- Create: `crates/storage/src/commit.rs`
- Create: `crates/storage/src/blob.rs`
- Create: `crates/storage/src/repository.rs`
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/error.rs`
- Modify: `crates/storage/src/encoding.rs`
- Modify: `crates/storage/src/schema.rs`

**Interfaces:**
- Consumes: `ClipEvent`, `ClipRepresentation`, exact raw payload bytes, placement policy.
- Produces:
  - `RepresentationPayload::new(id, bytes)`.
  - `SearchProjection::new(text, limits)`.
  - `ClipCommit::new(event, payloads, search_projection, limits)`.
  - `Storage::commit_clip`, `Storage::load_clip`, `Storage::read_representation`.
  - `CommitReceipt`, `StoredClip`, `BlobPlacementSummary`.

- [ ] **Step 1: Write failing validation, internal, external, and dedup tests**

Cover:

```rust
#[test]
fn missing_duplicate_unknown_length_and_digest_payloads_are_rejected_atomically() { /* exact cases */ }

#[test]
fn internal_and_external_payloads_round_trip_exact_bytes() { /* fixed policy per case */ }

#[test]
fn same_domain_digest_reuses_blob_without_merging_events() { /* two event IDs, one blob */ }

#[test]
fn identical_bytes_in_different_domain_ids_do_not_deduplicate() { /* two blobs */ }

#[test]
fn protected_payloads_are_rejected_before_any_file_or_row_is_created() { /* sensitive/private */ }
```

- [ ] **Step 2: Run focused tests and record red evidence**

```powershell
cargo test -p pastral-storage repository::tests blob::tests commit::tests -- --nocapture
```

Expected: failures for missing commit/blob/repository behavior.

- [ ] **Step 3: Implement commit cardinality and integrity validation**

Build a map by `ClipRepresentationId`. Require one and only one payload per event representation, exact `u64` length, ordinary domain, and digest equality:

```rust
let computed = RawDigest::sha256_raw_v1(representation.protection_domain(), payload.bytes())?;
if representation.digest() != Some(&computed) {
    return Err(StorageError::PayloadDigestMismatch(representation.id()));
}
```

Never place payload/search text in errors.

- [ ] **Step 4: Implement internal SQLite BLOB placement**

For nonempty payloads insert `zeroblob`, open an incremental writable blob, write exact bytes, close it, and commit. For empty payloads bind an empty BLOB. Persist blob metadata and representation references in one transaction.

- [ ] **Step 5: Implement external staged placement**

Use controlled roots:

```text
<data-root>/.staging/<uuid>.tmp
<data-root>/objects/ordinary/<protection-domain-uuid>/sha256-raw-v1/<digest-hex>.blob
```

Use `OpenOptions::create_new(true)`, `write_all`, `flush`, `sync_all`, close, and `rename`. Store only the relative key. Reject unsafe keys during every read.

- [ ] **Step 6: Implement ordinary dedup and exact readback verification**

Reuse only when domain ID, digest suite, full digest, and raw length match. On read, verify exact length and recompute `sha256-raw-v1`; return integrity errors without content.

- [ ] **Step 7: Run full batch verification and commit**

```powershell
eng/build.ps1 -Task All
cargo test -p pastral-storage --all-targets
cargo tree --workspace
```

Expected: all tests pass and no forbidden dependency appears.

```powershell
git add crates/storage crates/domain Cargo.toml Cargo.lock
git commit -m "feat: persist ordinary blobs and clip metadata"
```

---

### Task 4: Add bounded lexical search and capture-audit persistence

**Files:**
- Create: `crates/storage/src/search.rs`
- Modify: `crates/storage/src/repository.rs`
- Modify: `crates/storage/src/commit.rs`
- Modify: `crates/storage/src/encoding.rs`
- Modify: `crates/storage/src/error.rs`
- Modify: `crates/storage/src/lib.rs`

**Interfaces:**
- Consumes: optional allowed `SearchProjection`, `CaptureAuditEvent`.
- Produces:
  - `Storage::search(&str, usize) -> Result<Vec<SearchHit>, StorageError>`.
  - `Storage::insert_audit_event(CaptureAuditEvent)`.
  - deterministic literal FTS5 query builder.

- [ ] **Step 1: Write failing search and audit tests**

Test explicit indexing, no-projection absence, deterministic capture-order sorting, protected-projection rejection, and literal handling of:

```text
OR NOT * ( ) " quote\0
```

Inspect audit table columns with `PRAGMA table_info(capture_audit_events)` and assert that no normalized column name contains `payload`, `preview`, `digest`, `blob`, `snippet`, `title`, `url`, or `path`.

- [ ] **Step 2: Run focused tests and capture red evidence**

```powershell
cargo test -p pastral-storage search::tests audit -- --nocapture
```

- [ ] **Step 3: Implement bounded literal FTS query construction**

Split by Unicode whitespace, reject empty/NUL/oversize/too-many terms, double embedded quotes, quote each term, and join with `AND`. Pass the expression as a bound SQL parameter.

- [ ] **Step 4: Persist search projection in the clip transaction**

Insert `search_documents` and the matching explicit FTS row ID only for an ordinary event with an allowed projection. Never derive the projection from raw blob bytes inside storage.

- [ ] **Step 5: Persist content-free audit events**

Encode only the fields present on `CaptureAuditEvent`. Do not add a generic metadata map or arbitrary text field.

- [ ] **Step 6: Verify and commit**

```powershell
eng/build.ps1 -Task All
cargo test -p pastral-storage --all-targets
```

```powershell
git add crates/storage
git commit -m "feat: add bounded lexical search and audit storage"
```

---

### Task 5: Add deletion, integrity, and bounded reconciliation

**Files:**
- Create: `crates/storage/src/maintenance.rs`
- Modify: `crates/storage/src/blob.rs`
- Modify: `crates/storage/src/repository.rs`
- Modify: `crates/storage/src/schema.rs`
- Modify: `crates/storage/src/error.rs`
- Modify: `crates/storage/src/lib.rs`

**Interfaces:**
- Produces:
  - `Storage::delete_clip`.
  - `Storage::reconcile`.
  - `Storage::integrity_check`.
  - `DeleteReceipt`, `ReconciliationReport`, `IntegrityReport`.

- [ ] **Step 1: Write failing shared-reference deletion and maintenance tests**

Cover:

```rust
#[test]
fn deleting_one_shared_clip_preserves_blob_and_final_delete_removes_it() { /* internal */ }

#[test]
fn external_pending_delete_is_retryable() { /* simulate lock or retained row */ }

#[test]
fn reconcile_removes_stale_staging_and_unreferenced_objects_with_limit() { /* bounded */ }

#[test]
fn missing_external_file_is_reported_without_destroying_metadata() { /* no silent repair */ }

#[test]
fn sqlite_and_fts_integrity_checks_pass_after_representative_operations() { /* quick_check + FTS */ }
```

- [ ] **Step 2: Run focused tests and capture red evidence**

```powershell
cargo test -p pastral-storage maintenance repository::tests::delet -- --nocapture
```

- [ ] **Step 3: Implement transactional event deletion and reference accounting**

Delete FTS/search/representation/event rows in one transaction. Decrement each referenced blob exactly once per representation. Delete internal zero-reference rows immediately. Mark external zero-reference rows `PendingDelete` before commit.

- [ ] **Step 4: Implement external delete finalization and retry**

After commit, remove controlled external files and then delete the pending database row. Keep pending state if deletion fails due to I/O contention.

- [ ] **Step 5: Implement bounded reconciliation and integrity checks**

Reconcile at most `reconciliation_entry_limit` entries across staging, pending deletes, and final object enumeration. Compare stored reference counts to SQL counts. Use `PRAGMA quick_check` and FTS5 `integrity-check`. Report missing/corrupt state without content.

- [ ] **Step 6: Verify and commit**

```powershell
eng/build.ps1 -Task All
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
```

```powershell
git add crates/storage
git commit -m "feat: add deletion and storage reconciliation"
```

---

### Task 6: Extend CI and developer quality gates

**Files:**
- Modify: `.github/workflows/rust-ci.yml`
- Modify: `eng/build.ps1`
- Modify: `docs/operations/developer-setup.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: local/CI-equivalent storage test and dependency checks.

- [ ] **Step 1: Add explicit storage verification task**

Add `-Task Storage` to `eng/build.ps1` that runs:

```powershell
cargo test --locked -p pastral-storage --all-targets
```

Keep `All` fail-fast and include Storage through the workspace test path without running a command after failure.

- [ ] **Step 2: Extend Windows CI**

Retain existing gates and add an explicit storage test plus dependency/source-policy scans that fail on forbidden runtime categories, secret signatures, unsafe/product-network APIs, and untracked-input drift:

```powershell
cargo test --locked -p pastral-storage --all-targets
cargo tree --locked --workspace
eng/verify-source-policy.ps1
```

The scan must not reject `cc` or the bundled SQLite build chain, but must reject tokio, serde, chrono/time, sqlx, prost/protobuf, tracing/log, reqwest/hyper, windows/windows-sys, and UI crates in product dependencies.

- [ ] **Step 3: Update setup and status documentation**

Record exact PowerShell commands, rusqlite/SQLite versions, rollback-journal rationale, protected-payload rejection, current limitations, and the absence of a benchmark-selected threshold.

- [ ] **Step 4: Verify and commit**

```powershell
eng/verify-toolchain.ps1
eng/build.ps1 -Task All
eng/build.ps1 -Task Storage
eng/build.ps1 -Task SourcePolicy
git diff --check
```

```powershell
git add .github/workflows/rust-ci.yml eng/build.ps1 docs/operations/developer-setup.md README.md CHANGELOG.md
git commit -m "ci: verify storage foundation"
```

---

### Task 7: Record fresh milestone evidence

**Files:**
- Create: `docs/reviews/phase-2a-storage-search-foundation-verification.md`

**Interfaces:**
- Produces: evidence tied to the exact implementation commit.

- [ ] **Step 1: Run fresh final verification from Windows PowerShell**

```powershell
git status --short
eng/verify-toolchain.ps1
eng/build.ps1 -Task All
eng/build.ps1 -Task Storage
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
cargo tree --locked --workspace
eng/verify-dependencies.ps1
eng/verify-source-policy.ps1
cargo test --locked -p pastral-storage maintenance::tests::sqlite_and_fts_integrity_checks_pass_after_operations -- --exact --nocapture
git diff --check
git fsck --no-progress --no-dangling
git show --check HEAD
```

Also run secret-signature and forbidden-scope scans over tracked source/configuration.

- [ ] **Step 2: Write the report using only fresh output**

Include baseline/final commit, OS/shell, exact Rust/Cargo/rustfmt/Clippy/rusqlite/SQLite versions, SQLite pragmas, schema version, dependency tree, red/green evidence, test counts, commands, files changed, scope exclusions, skipped checks, residual risks, and unproven claims.

Explicitly state that WAL, benchmark threshold, crash-at-every-instruction evidence, encryption, backup/restore, Windows capture, IPC, UI, packaging, and remote GitHub-hosted CI remain unproven.

- [ ] **Step 3: Commit the report**

```powershell
git add docs/reviews/phase-2a-storage-search-foundation-verification.md
git commit -m "docs: record storage foundation evidence"
```

- [ ] **Step 4: Verify final repository state**

```powershell
git status --short
git log --oneline -12
git show --stat --oneline HEAD
git show --check HEAD
```

Expected: clean working tree, meaningful local batch commits, no remote changes.
