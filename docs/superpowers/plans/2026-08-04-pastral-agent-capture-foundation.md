# Pastral Agent Capture Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a tested Windows agent vertical slice that converts bounded `CF_UNICODETEXT` notifications into immutable ordinary clips persisted through SQLite/FTS5 without mutating the user's clipboard in automated tests.

**Architecture:** Add a storage-owned capture-order commit API, a Windows-binding-free coordinator crate, and a thin Windows executable adapter. Product capture uses event-driven listener notifications and synchronous bounded retries; IPC, replay, overlay, and sensitive retention remain excluded.

**Tech Stack:** Rust 1.97.1, Edition 2024, `pastral-domain`, `pastral-storage`, `pastral-clipboard-win`, SQLite/FTS5, Win32 clipboard APIs through pinned `windows-sys 0.61.2`, Windows PowerShell verification.

## Global Constraints

- Windows 11 x64 is the current runtime target.
- Rust toolchain remains exactly `1.97.1-x86_64-pc-windows-msvc`.
- No Tokio, Serde, logging framework, HTTP/network stack, gRPC, Protobuf runtime, or high-level `windows` crate.
- `pastral-domain`, `pastral-storage`, and `pastral-agent-core` remain Windows-binding free.
- Unsafe code remains limited to `crates/clipboard-win/src/sys.rs`.
- Automated tests must not write to or replace the user's clipboard.
- Product capture stores only ordinary `CF_UNICODETEXT` in this slice.
- Debug/diagnostic output must never print clipboard content or a content hash.
- Existing Rust, native-manager, dependency, source-policy, and Git integrity gates must remain green.

---

### Task 1: Storage-owned capture-order commit

**Files:**
- Modify: `crates/storage/src/commit.rs`
- Modify: `crates/storage/src/repository.rs`
- Modify: `crates/storage/src/lib.rs`
- Test: `crates/storage/src/repository.rs`

**Interfaces:**
- Produces `NewClipCommit::new(...)` without a `CaptureOrder` parameter.
- Produces `Storage::commit_new_clip(NewClipCommit) -> Result<AssignedCommitReceipt, StorageError>`.
- Produces `AssignedCommitReceipt::clip_event_id()`, `capture_order()`, `blob_objects_created()`, and `blob_objects_reused()`.
- Existing `Storage::commit_clip(ClipCommit)` remains valid for explicit-order/import/test paths.

- [ ] **Step 1: Add failing storage tests**

Add tests that construct ordinary Unicode-text representations and assert:

```rust
#[test]
fn new_capture_order_is_assigned_and_survives_reopen() {
    let root = TestRoot::new();
    let first_id;
    {
        let mut storage = Storage::open(root.path(), limits(), FixedPolicy(BlobPlacement::InternalSqlite)).unwrap();
        let first = storage.commit_new_clip(new_text_commit("first")).unwrap();
        let second = storage.commit_new_clip(new_text_commit("second")).unwrap();
        assert_eq!(first.capture_order().get(), 1);
        assert_eq!(second.capture_order().get(), 2);
        first_id = first.clip_event_id();
    }
    let storage = Storage::open(root.path(), limits(), FixedPolicy(BlobPlacement::InternalSqlite)).unwrap();
    assert_eq!(storage.load_clip(first_id).unwrap().unwrap().event().capture_order().get(), 1);
}
```

Also add:

```rust
#[test]
fn failed_new_capture_does_not_consume_order() { /* invalid payload mapping, then valid commit receives order 1 */ }

#[test]
fn explicit_order_collision_is_not_silently_reassigned() { /* explicit order 1 then new capture receives 2 */ }
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
cargo test --locked -p pastral-storage new_capture_order -- --nocapture
cargo test --locked -p pastral-storage failed_new_capture -- --nocapture
cargo test --locked -p pastral-storage explicit_order_collision -- --nocapture
```

Expected: compile failure because `NewClipCommit`, `commit_new_clip`, and `AssignedCommitReceipt` do not exist.

- [ ] **Step 3: Add `NewClipCommit`**

`NewClipCommit` stores:

```rust
pub struct NewClipCommit {
    event_id: ClipEventId,
    observed_at: UtcUnixMicros,
    captured_profile_id: ProfileId,
    captured_protection_domain: ProtectionDomain,
    representations: Vec<ClipRepresentation>,
    payloads: Vec<RepresentationPayload>,
    search_projection: Option<SearchProjection>,
}
```

Add getters and an internal:

```rust
pub(crate) fn assign_capture_order(
    self,
    capture_order: CaptureOrder,
) -> Result<ClipCommit, StorageError>
```

It constructs `ClipEvent::new` and maps `DomainError` to `StorageError::Domain`.

- [ ] **Step 4: Refactor transaction write logic**

Extract the existing `commit_clip` transaction body into a private helper that receives:

```rust
fn write_validated_commit(
    transaction: &rusqlite::Transaction<'_>,
    root: &Path,
    limits: StorageLimits,
    policy_version: StoragePolicyVersion,
    validated: &ValidatedCommit<'_>,
    pending: &mut [PendingRepresentation<'_>],
) -> Result<CommitReceipt, StorageError>
```

The helper performs duplicate checks, event/representation/blob/search inserts, and returns a receipt without committing the transaction.

- [ ] **Step 5: Implement transactional order assignment**

`commit_new_clip` must:

1. start `TransactionBehavior::Immediate`;
2. query the maximum non-null `capture_order` across `clip_events` and `capture_audit_events`;
3. checked-add one and construct `CaptureOrder`;
4. call `assign_capture_order`;
5. validate and prepare bounded blob placement;
6. write through `write_validated_commit`;
7. commit the transaction;
8. return `AssignedCommitReceipt`.

Use this query:

```sql
SELECT COALESCE(MAX(capture_order), 0)
FROM (
    SELECT capture_order FROM clip_events
    UNION ALL
    SELECT capture_order FROM capture_audit_events WHERE capture_order IS NOT NULL
)
```

- [ ] **Step 6: Run focused and full storage tests**

Run:

```powershell
cargo test --locked -p pastral-storage new_capture_order -- --nocapture
cargo test --locked -p pastral-storage failed_new_capture -- --nocapture
cargo test --locked -p pastral-storage explicit_order_collision -- --nocapture
cargo test --locked -p pastral-storage --all-targets
```

Expected: PASS with the existing storage suite plus the new order tests.

- [ ] **Step 7: Commit**

```powershell
git add crates/storage/src/commit.rs crates/storage/src/repository.rs crates/storage/src/lib.rs
git commit -m "feat: assign capture order in storage"
```

---

### Task 2: Deterministic agent-core coordinator

**Files:**
- Create: `crates/agent-core/Cargo.toml`
- Create: `crates/agent-core/src/lib.rs`
- Create: `crates/agent-core/src/capture.rs`
- Create: `crates/agent-core/src/coordinator.rs`
- Create: `crates/agent-core/src/error.rs`
- Create: `crates/agent-core/src/ports.rs`
- Modify: `Cargo.toml`
- Modify: `eng/verify-dependencies.ps1`
- Test: `crates/agent-core/src/coordinator.rs`

**Interfaces:**
- Produces `CaptureSequence::new(u32) -> Result<CaptureSequence, AgentError>`.
- Produces `CapturedText::new(String, Vec<u8>) -> Result<CapturedText, AgentError>`.
- Produces `TextCaptureRequest` with observation time, profile, protection domain, and captured text.
- Produces traits `CaptureSource`, `CaptureSink`, `Clock`, and `Sleeper`.
- Produces `CaptureCoordinator::new(CaptureConfig) -> Result<Self, AgentError>`.
- Produces `CaptureCoordinator::handle_notification(...) -> CaptureOutcome`.

- [ ] **Step 1: Add failing coordinator tests**

Write deterministic fakes and tests for:

```rust
#[test]
fn immediate_success_attempts_once_and_commits_once() { /* outcome Stored */ }

#[test]
fn successful_sequence_is_suppressed_on_repeat() { /* second call performs no source/sink work */ }

#[test]
fn transient_failures_follow_exact_retry_schedule() { /* 5, 15 ms then success */ }

#[test]
fn retry_exhaustion_never_commits() { /* four Busy errors */ }

#[test]
fn no_text_is_terminal_without_sleep() { /* Ok(None) */ }

#[test]
fn sink_failure_leaves_sequence_retryable() { /* first StorageFailed, second can store */ }

#[test]
fn empty_text_remains_valid_and_has_no_search_projection_hint() { /* exact terminator bytes */ }

#[test]
fn text_and_exact_utf16_bytes_are_not_normalized() { /* precomposed and decomposed remain distinct */ }
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test --locked -p pastral-agent-core --all-targets
```

Expected: failure because the crate/package does not exist.

- [ ] **Step 3: Implement value types and ports**

Use concrete content-free errors:

```rust
pub enum CaptureSourceError { Busy, InvalidData, PlatformFailure }
pub enum CaptureSinkError { StorageFailure }
pub enum CaptureOutcome {
    Stored { clip_event_id: ClipEventId, capture_order: CaptureOrder },
    DuplicateNotification,
    NoSupportedRepresentation,
    RetryExhausted { attempts: usize },
    InvalidCapture,
    PlatformFailure,
    StorageFailed,
}
```

Traits:

```rust
pub trait CaptureSource {
    fn capture_unicode_text(&mut self, max_bytes: NonZeroUsize)
        -> Result<Option<CapturedText>, CaptureSourceError>;
}

pub trait CaptureSink {
    fn store_text(&mut self, request: TextCaptureRequest)
        -> Result<StoredCapture, CaptureSinkError>;
}

pub trait Clock { fn now_utc_micros(&mut self) -> Result<UtcUnixMicros, AgentError>; }
pub trait Sleeper { fn sleep(&mut self, duration: Duration); }
```

- [ ] **Step 4: Implement coordinator RED→GREEN**

`CaptureConfig` contains:

```rust
profile_id: ProfileId,
protection_domain: ProtectionDomain,
max_unicode_text_bytes: NonZeroUsize,
retry_delays: Vec<Duration>,
```

Require a non-empty retry schedule whose first delay is zero. Default diagnostic schedule is `[0, 5, 15, 35]` milliseconds.

Only `CaptureSourceError::Busy` is retried. Mark the sequence handled after successful storage or a terminal non-storage outcome. Do not mark it handled after `StorageFailed`.

- [ ] **Step 5: Update dependency policy**

Add `pastral-agent-core` to the packages that must remain free of Windows binding packages. Do not add new third-party dependencies.

- [ ] **Step 6: Run crate/workspace quality gates**

Run:

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-agent-core --all-targets
cargo check --locked --workspace --all-targets
cargo clippy --locked -p pastral-agent-core --all-targets -- -D warnings
.\eng\verify-dependencies.ps1
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add Cargo.toml Cargo.lock crates/agent-core eng/verify-dependencies.ps1
git commit -m "feat: add deterministic capture coordinator"
```

---

### Task 3: Windows storage and clipboard adapters

**Files:**
- Create: `apps/agent/Cargo.toml`
- Create: `apps/agent/src/lib.rs`
- Create: `apps/agent/src/clock.rs`
- Create: `apps/agent/src/config.rs`
- Create: `apps/agent/src/platform.rs`
- Create: `apps/agent/src/storage_sink.rs`
- Test: `apps/agent/src/config.rs`
- Test: `apps/agent/src/storage_sink.rs`
- Modify: `Cargo.toml`
- Modify: `eng/verify-dependencies.ps1`

**Interfaces:**
- Produces `AgentIdentity::load_or_create(root: &Path)`.
- Produces `SystemClock` and `ThreadSleeper` adapters.
- Produces `WindowsClipboardSource` implementing `CaptureSource`.
- Produces `StorageCaptureSink<P>` implementing `CaptureSink`.
- Produces `DiagnosticStoragePolicy`, selecting internal SQLite BLOBs.

- [ ] **Step 1: Add failing identity tests**

Test:

- first load creates stable profile/domain IDs;
- reopen returns the same IDs;
- malformed identity file fails closed;
- no clipboard content is stored in the identity file;
- file has exactly version/profile/domain fields.

Run:

```powershell
cargo test --locked -p pastral-agent identity -- --nocapture
```

Expected: failure because the package does not exist.

- [ ] **Step 2: Implement content-free identity storage**

Use format:

```text
version=1
profile_id=<canonical UUIDv4>
ordinary_domain_id=<canonical UUIDv4>
```

Write a temporary file with `OpenOptions::create_new(true)`, `write_all`, `sync_all`, then rename. If the final file appears concurrently, delete the temporary file and parse the final file. Invalid existing content is an error.

- [ ] **Step 3: Add failing storage-sink tests**

Build a `TextCaptureRequest`, store it in a disposable root, then assert:

- loaded representation format is standard ID `13`;
- raw bytes match exact UTF-16LE including terminator;
- non-empty text is searchable;
- empty text persists but produces no search hit;
- assigned capture orders are increasing.

- [ ] **Step 4: Implement storage sink**

Map a text request to:

```rust
let representation_id = ClipRepresentationId::new_v4();
let digest = RawDigest::sha256_raw_v1(domain, captured.raw_utf16le())?;
let representation = ClipRepresentation::new(
    representation_id,
    ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
    domain,
    raw_length,
    Some(digest),
    Fidelity::FullFidelity,
)?;
```

Create `SearchProjection` only when decoded text contains a non-whitespace character. Call `Storage::commit_new_clip` and return `StoredCapture`.

- [ ] **Step 5: Implement Windows clipboard adapter**

Map:

- `ClipboardError::ClipboardUnavailable` and `Win32 { operation: "OpenClipboard", .. }` to `CaptureSourceError::Busy`;
- Unicode validation/size/handle failures to `InvalidData`;
- other platform failures to `PlatformFailure`.

Always close `ClipboardSession` through RAII and never print text.

- [ ] **Step 6: Enforce dependency boundaries**

Allow `pastral-agent` to depend on pinned `windows-sys/windows-link` only through `pastral-clipboard-win`; reject unexpected alternate Windows bindings. Keep `pastral-agent-core` Windows-free.

- [ ] **Step 7: Run adapter tests and policies**

Run:

```powershell
cargo test --locked -p pastral-agent --lib --all-targets
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
```

Expected: PASS without accessing the user's clipboard.

- [ ] **Step 8: Commit**

```powershell
git add Cargo.toml Cargo.lock apps/agent eng/verify-dependencies.ps1
git commit -m "feat: add Windows capture adapters"
```

---

### Task 4: Diagnostic agent executable

**Files:**
- Create: `apps/agent/src/main.rs`
- Create: `apps/agent/src/cli.rs`
- Create: `eng/verify-agent.ps1`
- Modify: `eng/build.ps1`
- Modify: `.github/workflows/rust-ci.yml`
- Modify: `eng/verify-source-policy.ps1`

**Interfaces:**
- Produces commands `health-check`, `capture-current`, and `listen`.
- Produces build tasks `Agent` and `AgentPolicy`.
- Produces `eng/verify-agent.ps1 -Mode Static|Build|Smoke|All`.

- [ ] **Step 1: Add failing CLI parser tests**

Test exact accepted/rejected forms:

```text
health-check --data-root <path>
capture-current --data-root <path>
listen --data-root <path>
listen --data-root <path> --max-events 3
```

Reject missing command/root, zero max-events, duplicate flags, unknown flags, and positional extras.

- [ ] **Step 2: Run parser tests and verify RED**

Run:

```powershell
cargo test --locked -p pastral-agent cli -- --nocapture
```

Expected: failure because `cli` module/command types do not exist.

- [ ] **Step 3: Implement commands**

`health-check`:

- load/create identity;
- open storage with diagnostic limits/policy;
- call `runtime_info()` and `integrity_check()`;
- print only schema/runtime/integrity status and data-root path;
- exit nonzero if integrity fails.

`capture-current`:

- create a synthetic nonzero local sequence value used only for one explicit attempt;
- call coordinator once;
- print only the `CaptureOutcome` class, event ID/order when stored, and no content.

`listen`:

- start `ClipboardListener`;
- receive notifications with a one-second timeout so bounded `--max-events` state can be checked;
- convert nonzero sequences into `CaptureSequence`;
- process through coordinator;
- count terminal outcomes, not raw/coalesced notifications;
- stop after `--max-events` terminal outcomes when supplied;
- otherwise continue until externally terminated.

- [ ] **Step 4: Add static/build/smoke verifier**

Static mode checks:

- exact commands exist;
- no default silent capture;
- output source contains no text/preview/hash printing;
- explicit max payload and retry limits;
- no network/process APIs;
- agent-core remains Windows-free.

Build mode runs:

```powershell
cargo build --locked -p pastral-agent
cargo build --locked -p pastral-agent --release
```

Smoke mode creates a disposable temporary root and runs:

```powershell
target\debug\pastral-agent.exe health-check --data-root <temp>
```

It must verify exit `0`, expected content-free status markers, and clean up the root. It must not run `capture-current` or `listen`.

- [ ] **Step 5: Integrate orchestration and CI**

Add:

- `Agent` task: `verify-agent.ps1 -Mode All`;
- `AgentPolicy` task: static mode;
- `All`: retain Rust-only behavior but include agent crate in workspace checks/tests;
- `Full`: Rust + agent build/smoke + native manager static/build;
- Rust CI focused agent static/build/smoke step on Windows.

- [ ] **Step 6: Run agent and aggregate gates**

Run:

```powershell
.\eng\verify-agent.ps1 -Mode All
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\verify-source-policy.ps1
git diff --check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add apps/agent/src/main.rs apps/agent/src/cli.rs eng/verify-agent.ps1 eng/build.ps1 eng/verify-source-policy.ps1 .github/workflows/rust-ci.yml
git commit -m "feat: add diagnostic resident agent"
```

---

### Task 5: Documentation and Phase 3B evidence

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Create: `docs/reviews/phase-3b-agent-capture-foundation-verification.md`

**Interfaces:**
- Documents exact commands, implemented behavior, safety boundaries, verification results, and excluded release claims.

- [ ] **Step 1: Update user/developer documentation**

Document:

- ordinary Unicode-text capture foundation exists;
- `health-check` is safe for automated smoke;
- `capture-current` and `listen` explicitly read the user's clipboard and are never run automatically;
- exact build/run commands;
- data-root identity/config format;
- no IPC, manager live connection, replay, overlay, encryption, source exclusion, packaging, or auto-start yet.

- [ ] **Step 2: Run fresh final verification**

Run:

```powershell
.\eng\verify-toolchain.ps1 -RequireNativeManager
.\eng\verify-agent.ps1 -Mode All
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
cargo tree --locked --workspace
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
git diff --check
git fsck --no-progress --no-dangling
git show --check HEAD
```

- [ ] **Step 3: Write evidence report**

Record:

- baseline/final commits;
- OS/toolchain/package versions;
- test counts by crate;
- storage capture-order evidence;
- agent-core retry/duplicate/storage-failure evidence;
- executable health-check smoke path and output classes;
- proof automated tests did not invoke `capture-current`/`listen`;
- native manager regression result;
- exact skipped checks and residual risk;
- next recommended slice: privacy/source policy before default resident capture, then bounded IPC evidence/implementation.

- [ ] **Step 4: Commit docs and evidence**

```powershell
git add README.md CHANGELOG.md docs/operations/developer-setup.md docs/reviews/phase-3b-agent-capture-foundation-verification.md
git commit -m "docs: record agent capture foundation evidence"
```

- [ ] **Step 5: Final clean-tree verification**

Run:

```powershell
.\eng\verify-agent.ps1 -Mode All
git status --short
git show --check HEAD
```

Expected: all gates PASS and an empty working tree.