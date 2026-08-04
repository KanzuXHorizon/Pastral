# Pastral Agent Health IPC Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that the real Pastral agent Health path can be served through the authenticated Windows transport within explicit binary/private-working-set budgets while the default agent remains Protobuf/transport-free.

**Architecture:** `pastral-agent` gains one serializer-neutral content-free health snapshot used by the existing CLI and the admission server. A separate `pastral-agent-ipc-probe` package starts baseline and authenticated server children over a disposable root, measures both processes through a narrow `pastral-ipc-win` process-memory API, completes one real Health request, enforces footprint ceilings, and exits. The admission executable is diagnostic only and never runs concurrently with a production agent over the same root.

**Tech Stack:** Rust 1.97.1/Edition 2024, SQLite/FTS5 through existing `pastral-storage`, Phase 3E named-pipe/DPAPI/HMAC transport, `windows-sys 0.61.2`, PowerShell 5.1, Windows 11 x64.

## Global Constraints

- Default `pastral-agent` must remain free of `pastral-ipc-auth`, `pastral-ipc-schema`, `pastral-ipc-win`, and all official Protobuf packages.
- `pastral-agent-ipc-probe` may depend on the agent library and Phase 3E transport packages, but never directly on `pastral-storage` or `pastral-clipboard-win`.
- The parent process never opens agent storage; exactly one child owns the selected diagnostic storage root at a time.
- Automated paths never invoke `capture-current`, `listen`, or any clipboard read/write API.
- The server accepts one authenticated connection, one Health request, sends one response, and exits.
- Baseline/server child readiness lines are content-free, single-line, and at most 64 bytes.
- No root, pipe, SID, secret, nonce, proof, clipboard, query, preview, hash, or source value may appear in normal output.
- Process-memory metrics are diagnostics only and never authorization evidence.
- Initial admission ceilings: server private usage `<= 25 MiB`, private delta `<= 8 MiB`, working-set delta `<= 12 MiB`, probe binary delta over default agent `<= 6 MiB`.
- Parent connect timeout is 5 seconds; handshake and Health deadlines are 2 seconds each; child total lifetime is bounded.
- All Windows unsafe calls remain in `crates/ipc-win/src/sys.rs`.
- No auto-start, manager linkage, production discovery, history/search/paste, subscriptions, bulk transfer, service, network, registry, async runtime, or logging backend.

---

### Task 1: Create one content-free agent Health truth

**Files:**
- Create: `apps/agent/src/health.rs`
- Modify: `apps/agent/src/lib.rs`
- Modify: `apps/agent/src/runtime.rs`
- Create test: `apps/agent/tests/health_snapshot.rs`
- Modify test: `apps/agent/tests/runtime.rs`

**Interfaces:**
- Consumes: `AgentIdentity::load_or_create`, `PrivacyPolicyConfig::load_or_create`, existing diagnostic storage configuration and `Storage::runtime_info`/`integrity_check`.
- Produces:

```rust
pub struct AgentHealthSnapshot { /* private fields; no Debug */ }

impl AgentHealthSnapshot {
    pub const fn storage_schema_version(&self) -> u32;
    pub const fn capture_enabled(&self) -> bool;
    pub const fn privacy_policy_ok(&self) -> bool;
    pub const fn storage_integrity_ok(&self) -> bool;
}

pub fn load_health_snapshot(
    data_root: &Path,
) -> Result<AgentHealthSnapshot, AgentRuntimeError>;
```

- `runtime::run_health_check` becomes a formatter over `load_health_snapshot`; it retains the explicit `data-root=` diagnostic line for the CLI.
- `open_storage` moves to `health.rs` or becomes `pub(crate)` without exposing `Storage` publicly.

- [ ] **Step 1: Write RED health snapshot tests**

Create `apps/agent/tests/health_snapshot.rs`:

```rust
use pastral_agent::load_health_snapshot;

#[test]
fn real_disposable_root_returns_content_free_healthy_snapshot() {
    let root = test_root("healthy");
    let snapshot = load_health_snapshot(&root).unwrap();
    assert_eq!(snapshot.storage_schema_version(), 1);
    assert!(!snapshot.capture_enabled());
    assert!(snapshot.privacy_policy_ok());
    assert!(snapshot.storage_integrity_ok());
    cleanup(root);
}

#[test]
fn health_snapshot_type_has_no_debug_payload_surface() {
    let source = include_str!("../src/health.rs");
    assert!(!source.contains("derive(Debug"));
    for forbidden in ["data_root:", "preview:", "query:", "source:", "digest:"] {
        assert!(!source.contains(forbidden));
    }
}
```

Use the existing test-root pattern from `apps/agent/tests/runtime.rs`; do not create a shared test utility unless the duplication exceeds twenty lines.

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --locked -p pastral-agent --test health_snapshot
```

Expected: compile failure because `load_health_snapshot` and `health.rs` do not exist.

- [ ] **Step 3: Implement `AgentHealthSnapshot` and loader**

Implementation requirements:

```rust
pub struct AgentHealthSnapshot {
    storage_schema_version: u32,
    capture_enabled: bool,
    privacy_policy_ok: bool,
    storage_integrity_ok: bool,
}
```

`load_health_snapshot` must:

1. load/create `AgentIdentity`;
2. load/create `PrivacyPolicyConfig`;
3. open existing diagnostic storage;
4. read runtime info;
5. run integrity check;
6. reject if any SQLite/FTS/metadata/search-mapping bit is false;
7. construct snapshot with `capture_enabled = false`, both policy/integrity booleans true.

It must not print, open clipboard, start listener, or expose the root.

- [ ] **Step 4: Refactor CLI health-check**

Replace duplicated identity/privacy/storage/integrity logic in `run_health_check` with:

```rust
let snapshot = load_health_snapshot(data_root)?;
```

Format existing lines from snapshot getters. Preserve current output keys and failure behavior.

- [ ] **Step 5: Add CLI/snapshot agreement assertion**

Update `apps/agent/tests/runtime.rs` so successful output schema equals `snapshot.storage_schema_version()` and all four integrity markers remain `ok`.

- [ ] **Step 6: Run GREEN and quality gates**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-agent --all-targets
cargo clippy --locked -p pastral-agent --all-targets --all-features -- -D warnings
.\eng\verify-agent.ps1 -Mode All
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
```

Expected: PASS; dependency output still states default agent is Protobuf-free.

- [ ] **Step 7: Commit**

```powershell
git add apps/agent/src/health.rs apps/agent/src/lib.rs apps/agent/src/runtime.rs apps/agent/tests/health_snapshot.rs apps/agent/tests/runtime.rs
git commit -m "refactor: centralize agent Health snapshot"
```

---

### Task 2: Add bounded process-memory diagnostics to the reviewed Windows boundary

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/ipc-win/src/sys.rs`
- Create: `crates/ipc-win/src/process_memory.rs`
- Modify: `crates/ipc-win/src/lib.rs`
- Create test: `crates/ipc-win/tests/process_memory.rs`

**Interfaces:**
- Consumes: existing `TransportError`, process handle RAII, `windows-sys 0.61.2`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessMemorySnapshot {
    working_set_bytes: u64,
    private_usage_bytes: u64,
}

impl ProcessMemorySnapshot {
    pub const fn working_set_bytes(self) -> u64;
    pub const fn private_usage_bytes(self) -> u64;
}

pub fn process_memory_snapshot(
    process_id: u32,
) -> Result<ProcessMemorySnapshot, TransportError>;
```

- [ ] **Step 1: Add RED process-memory tests**

```rust
#[test]
fn current_process_memory_is_nonzero_and_repeatable() {
    let pid = std::process::id();
    for _ in 0..32 {
        let snapshot = process_memory_snapshot(pid).unwrap();
        assert!(snapshot.working_set_bytes() > 0);
        assert!(snapshot.private_usage_bytes() > 0);
    }
}

#[test]
fn zero_invalid_and_exited_processes_fail_closed() {
    assert!(process_memory_snapshot(0).is_err());
    assert!(process_memory_snapshot(u32::MAX).is_err());
    let mut child = Command::new("cmd.exe").args(["/d", "/c", "exit", "0"]).spawn().unwrap();
    let pid = child.id();
    assert!(child.wait().unwrap().success());
    assert!(process_memory_snapshot(pid).is_err());
}
```

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked -p pastral-ipc-win --test process_memory
```

Expected: missing public API.

- [ ] **Step 3: Pin the required Windows feature**

Add `Win32_System_ProcessStatus` to the existing workspace `windows-sys` feature list. Do not add another Windows binding crate.

- [ ] **Step 4: Implement the native query only in `sys.rs`**

Use `K32GetProcessMemoryInfo`/`PROCESS_MEMORY_COUNTERS_EX` with a fixed stack structure. Open the process using the minimum access accepted on the target Windows baseline. Convert `WorkingSetSize` and `PrivateUsage` with checked `u64` conversion and reject zero results. Reuse/extend the existing owned process-handle type; close every handle.

- [ ] **Step 5: Implement safe wrapper and getters**

`process_memory.rs` performs PID validation and maps the native snapshot. It contains no unsafe code.

- [ ] **Step 6: Run GREEN and full transport regression**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-ipc-win --all-targets
cargo clippy --locked -p pastral-ipc-win --all-targets --all-features -- -D warnings
.\eng\build.ps1 -Task IpcTransport
.\eng\verify-source-policy.ps1
```

- [ ] **Step 7: Commit**

```powershell
git add Cargo.toml crates/ipc-win/src/sys.rs crates/ipc-win/src/process_memory.rs crates/ipc-win/src/lib.rs crates/ipc-win/tests/process_memory.rs
git commit -m "feat: measure IPC process memory"
```

---

### Task 3: Create strict admission CLI and baseline child

**Files:**
- Modify: `Cargo.toml`
- Create: `apps/agent-ipc-probe/Cargo.toml`
- Create: `apps/agent-ipc-probe/src/main.rs`
- Create: `apps/agent-ipc-probe/src/cli.rs`
- Create: `apps/agent-ipc-probe/src/child.rs`
- Create test: `apps/agent-ipc-probe/tests/cli.rs`
- Create test: `apps/agent-ipc-probe/tests/baseline_child.rs`

**Interfaces:**
- Consumes: `pastral_agent::load_health_snapshot`, `pastral_ipc_win::random_bytes` for disposable root identity.
- Produces strict modes:

```rust
pub enum AdmissionMode {
    Parent,
    BaselineChild { data_root: PathBuf },
    ServerChild { data_root: PathBuf },
}

pub fn parse_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<AdmissionMode, AdmissionError>;
```

Child helper:

```rust
pub fn run_baseline_child(
    data_root: &Path,
    input: impl BufRead,
    output: impl Write,
) -> Result<(), AdmissionError>;
```

- [ ] **Step 1: Write RED parser tests**

Accept only:

```text
[]
[--baseline-child, --data-root, <nonempty>]
[--server-child, --data-root, <nonempty>]
```

Reject unknown modes/flags, missing/empty root, duplicates, extra positional values, non-Unicode flags, and parent flags.

- [ ] **Step 2: Run RED**

```powershell
cargo test -p pastral-agent-ipc-probe --test cli
```

Expected: package/API missing.

- [ ] **Step 3: Create package and strict parser**

Dependencies at this task:

```toml
pastral-agent = { path = "../agent" }
pastral-ipc-win = { path = "../../crates/ipc-win" }
```

Do not add schema/auth/core until the server task needs them.

- [ ] **Step 4: Write RED baseline child test**

The test creates a disposable root, feeds an empty input stream (EOF), and asserts exactly:

```text
agent-baseline-ready=ok
```

It also asserts that identity, privacy policy, and storage files were created by the real health snapshot and no transport identity/secret file exists.

- [ ] **Step 5: Implement baseline child**

1. load snapshot;
2. ensure its four invariants are healthy;
3. write and flush one readiness line;
4. read until EOF with a fixed 64-byte buffer;
5. reject any non-whitespace command bytes rather than treating stdin as a control channel;
6. exit on EOF.

- [ ] **Step 6: Run GREEN and package quality gates**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-agent-ipc-probe --test cli --test baseline_child
cargo clippy --locked -p pastral-agent-ipc-probe --all-targets --all-features -- -D warnings
```

- [ ] **Step 7: Verify default agent isolation**

```powershell
cargo tree --locked -p pastral-agent --edges all
```

Expected: no `protobuf`, `pastral-ipc-auth`, `pastral-ipc-schema`, or `pastral-ipc-win`.

- [ ] **Step 8: Commit**

```powershell
git add Cargo.toml Cargo.lock apps/agent-ipc-probe
git commit -m "test: add agent IPC admission baseline"
```

---

### Task 4: Serve one authenticated real agent Health response

**Files:**
- Modify: `apps/agent-ipc-probe/Cargo.toml`
- Create: `apps/agent-ipc-probe/src/server.rs`
- Create: `apps/agent-ipc-probe/src/protocol.rs`
- Modify: `apps/agent-ipc-probe/src/main.rs`
- Modify: `apps/agent-ipc-probe/src/child.rs`
- Create test: `apps/agent-ipc-probe/tests/server_child.rs`
- Create test: `apps/agent-ipc-probe/tests/cross_process.rs`

**Interfaces:**
- Adds dependencies:

```toml
pastral-ipc-auth = { path = "../../crates/ipc-auth" }
pastral-ipc-core = { path = "../../crates/ipc-core" }
pastral-ipc-schema = { path = "../../crates/ipc-schema" }
```

- Produces:

```rust
pub fn run_server_child(
    data_root: &Path,
    output: impl Write,
) -> Result<(), AdmissionError>;

pub fn run_parent() -> Result<AdmissionMetrics, AdmissionError>;
```

- [ ] **Step 1: Write RED server-child test**

Test in-process server thread plus a real Phase 3E client:

1. initialize material;
2. start server child function;
3. read readiness;
4. connect/authenticate;
5. send one Health request;
6. assert response values equal `load_health_snapshot` values;
7. assert server exits after one response.

Expected RED: server functions absent.

- [ ] **Step 2: Implement snapshot-to-DTO mapping**

```rust
fn health_response(snapshot: &AgentHealthSnapshot) -> Result<ResponseDto, AdmissionError> {
    Ok(ResponseDto::Health(HealthResponseDto::new(
        snapshot.storage_schema_version(),
        snapshot.capture_enabled(),
        snapshot.privacy_policy_ok(),
        snapshot.storage_integrity_ok(),
    )?))
}
```

No direct storage calls in `protocol.rs`.

- [ ] **Step 3: Implement server child lifecycle**

Order is mandatory:

1. load real health snapshot;
2. load transport material;
3. current token identity;
4. derive pipe name;
5. build and inspect security descriptor; require exact Phase 3E inspection booleans/count/mask;
6. create first server;
7. write/flush `agent-health-server-ready=ok`;
8. connect with 5-second deadline;
9. validate client peer;
10. mutual server handshake with 64-entry replay cache and 2-second deadline;
11. read one control frame with 2-second deadline;
12. require nonzero correlation and exact Health request;
13. send correlated Health response;
14. exit.

Any other operation, second frame, wrong kind, zero correlation, or error returns nonzero and no fallback.

- [ ] **Step 4: Write RED cross-process parent test**

The integration test executes `CARGO_BIN_EXE_pastral-agent-ipc-probe` with no arguments and requires:

```text
agent-ipc-admission=ok
cross-process=true
health=ok
```

PIDs must be distinct; root/pipe/SID/secret/nonce/proof/clipboard markers absent.

- [ ] **Step 5: Implement parent child orchestration without metrics ceilings yet**

Parent:

1. create disposable root;
2. create/load material;
3. spawn server child with piped stdout/stderr and null stdin;
4. read exactly one readiness line with a bounded deadline helper;
5. connect/authenticate/send Health;
6. validate content-free response;
7. wait for successful child exit;
8. remove root;
9. print only success/PID/session/timing markers.

Use a dedicated readiness reader thread/channel or bounded polling; never block indefinitely on `read_line`.

- [ ] **Step 6: Add negative tests**

- wrong material root fails authentication;
- first-instance collision prevents server readiness;
- authenticated non-Health request is rejected;
- child stderr or unexpected readiness line fails parent;
- child total timeout triggers cleanup;
- invalid arguments exit `2` before root/transport creation.

- [ ] **Step 7: Run GREEN**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-agent-ipc-probe --all-targets
cargo clippy --locked -p pastral-agent-ipc-probe --all-targets --all-features -- -D warnings
.\eng\build.ps1 -Task IpcTransport
.\eng\verify-agent.ps1 -Mode All
```

- [ ] **Step 8: Commit**

```powershell
git add apps/agent-ipc-probe Cargo.lock
git commit -m "feat: serve authenticated agent Health probe"
```

---

### Task 5: Measure baseline/server memory and binary admission budgets

**Files:**
- Create: `apps/agent-ipc-probe/src/metrics.rs`
- Modify: `apps/agent-ipc-probe/src/main.rs`
- Modify: `apps/agent-ipc-probe/src/child.rs`
- Modify: `apps/agent-ipc-probe/src/server.rs`
- Modify test: `apps/agent-ipc-probe/tests/cross_process.rs`
- Create test: `apps/agent-ipc-probe/tests/metrics.rs`

**Interfaces:**
- Consumes: `process_memory_snapshot`, child readiness helpers, sibling Release executables.
- Produces:

```rust
pub struct AdmissionMetrics {
    client_process_id: u32,
    server_process_id: u32,
    session_id: u32,
    default_agent_binary_bytes: u64,
    admission_binary_bytes: u64,
    binary_delta_bytes: u64,
    baseline_working_set_bytes: u64,
    baseline_private_bytes: u64,
    server_working_set_bytes: u64,
    server_private_bytes: u64,
    working_set_delta_bytes: i64,
    private_delta_bytes: i64,
    connect_micros: u128,
    handshake_micros: u128,
    health_micros: u128,
    total_micros: u128,
}
```

No `Debug` implementation is required; output formatting is explicit.

- [ ] **Step 1: Write RED checked-delta/ceiling tests**

Test:

- positive and negative signed memory deltas;
- binary subtraction underflow rejected;
- exact ceiling accepted;
- one-byte-over each ceiling rejected with `FootprintCeiling`;
- zero absolute memory/binary metrics rejected.

- [ ] **Step 2: Implement pure metric arithmetic and ceilings**

Constants:

```rust
const MIB: u64 = 1024 * 1024;
const MAX_SERVER_PRIVATE_BYTES: u64 = 25 * MIB;
const MAX_PRIVATE_DELTA_BYTES: i64 = 8 * MIB as i64;
const MAX_WORKING_SET_DELTA_BYTES: i64 = 12 * MIB as i64;
const MAX_BINARY_DELTA_BYTES: u64 = 6 * MIB;
```

Negative runtime deltas are valid and reported; only positive values above the ceilings fail.

- [ ] **Step 3: Implement baseline child orchestration in parent**

Spawn baseline child with piped stdin/stdout/stderr. After readiness:

1. query child memory;
2. close stdin to signal EOF;
3. require clean exit and empty stderr.

No transport material is created by baseline child; parent-created material may already exist in the root but baseline code must not read it.

- [ ] **Step 4: Measure server child at ready state**

After server readiness but before client connection, call `process_memory_snapshot(server_pid)`. Then complete transport flow.

- [ ] **Step 5: Resolve executable sizes**

From `current_exe().parent()` require:

- `pastral-agent.exe`;
- current `pastral-agent-ipc-probe.exe`.

Missing sibling agent is `MissingReleaseArtifact`; parent does not invoke Cargo or build tools itself.

- [ ] **Step 6: Print exact content-free metrics**

Required keys:

```text
agent-ipc-admission=ok
cross-process=true
health=ok
client-pid=
server-pid=
session-id=
default-agent-binary-bytes=
admission-binary-bytes=
binary-delta-bytes=
baseline-working-set-bytes=
baseline-private-bytes=
server-working-set-bytes=
server-private-bytes=
working-set-delta-bytes=
private-delta-bytes=
connect-us=
handshake-us=
health-us=
total-us=
```

- [ ] **Step 7: Strengthen integration leak and ceiling assertions**

Parse every required metric, require distinct PIDs and nonzero absolute bytes/timings, then reject all forbidden markers case-insensitively.

- [ ] **Step 8: Build both Release executables and run parent**

```powershell
cargo build --locked -p pastral-agent --release
cargo build --locked -p pastral-agent-ipc-probe --release
.\target\release\pastral-agent-ipc-probe.exe
```

Do not change ceilings to make the run pass. If a ceiling fails, stop resident admission and record the actual measurement.

- [ ] **Step 9: Run package quality gates and commit**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-agent-ipc-probe --all-targets
cargo clippy --locked -p pastral-agent-ipc-probe --all-targets --all-features -- -D warnings
git diff --check
git add apps/agent-ipc-probe
git commit -m "test: measure agent IPC admission footprint"
```

---

### Task 6: Add verifier, build task, dependency/source policy, and CI

**Files:**
- Create: `eng/verify-agent-ipc-admission.ps1`
- Modify: `eng/build.ps1`
- Modify: `eng/verify-dependencies.ps1`
- Modify: `eng/verify-source-policy.ps1`
- Modify: `.github/workflows/rust-ci.yml`

**Interfaces:**
- Produces verifier modes `Static`, `Test`, `Smoke`, `All`.
- Produces build task `AgentIpcAdmission` and adds it to `Full` only after standalone `All` passes.

- [ ] **Step 1: Implement Static verification**

Require files/APIs/constants and assert:

- default agent Cargo tree contains no Protobuf/auth/schema/ipc-win;
- admission package has exact permitted path dependencies;
- no direct `pastral-storage`/clipboard/manager/network/registry/Tokio dependency;
- exact three CLI modes;
- exact readiness/success markers;
- exact four footprint ceilings;
- process memory Win32 symbols only in `ipc-win/src/sys.rs`;
- process spawning only in admission main/tests;
- no production-root default, auto-start, `listen`, or `capture-current` command invocation.

- [ ] **Step 2: Implement Test verification**

Run:

```powershell
cargo test --locked -p pastral-agent --test health_snapshot --test runtime
cargo test --locked -p pastral-ipc-win --test process_memory
cargo test --locked -p pastral-agent-ipc-probe --all-targets
```

- [ ] **Step 3: Implement Smoke verification**

Build default agent Release and admission Release, run parent, parse every required metric, apply the same ceilings independently in PowerShell, scan forbidden markers, and verify invalid arguments return nonzero with empty stdout.

- [ ] **Step 4: Update dependency policy**

- Keep `pastral-agent` in the Protobuf-free package list.
- Add admission package to exact official runtime and Windows-binding lists.
- Require its direct package graph to include `pastral-agent` and `pastral-ipc-win` but reject direct `pastral-storage`/`pastral-clipboard-win` package dependencies by inspecting its manifest as well as the transitive tree statement.
- Keep async/network/logging forbidden sets.

- [ ] **Step 5: Update source policy**

- Include `apps/agent-ipc-probe/**` in product source scanning.
- Permit `std::process::Command` only in exact admission main/integration-test paths plus existing transport-probe paths.
- Keep named-pipe APIs only in `ipc-win/src/sys.rs`.
- Keep unsafe only in reviewed sys boundaries.

- [ ] **Step 6: Integrate build task**

Add `AgentIpcAdmission` to `ValidateSet`, dispatcher, and `Full` after `IpcTransport` and before `Agent`.

- [ ] **Step 7: Update CI**

Add path filters and a Windows step:

```yaml
- name: Verify agent IPC admission
  shell: powershell
  run: ./eng/verify-agent-ipc-admission.ps1 -Mode All
```

- [ ] **Step 8: Verify locally**

```powershell
.\eng\verify-agent-ipc-admission.ps1 -Mode All
.\eng\build.ps1 -Task AgentIpcAdmission
py.exe -3 -c "import yaml,pathlib; yaml.safe_load(pathlib.Path('.github/workflows/rust-ci.yml').read_text(encoding='utf-8')); print('YAML_PARSE=PASS')"
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
```

- [ ] **Step 9: Commit**

```powershell
git add eng .github/workflows/rust-ci.yml
git commit -m "ci: verify agent IPC admission"
```

---

### Task 7: Aggregate verification and Phase 3F evidence

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Modify: `docs/adr/0018-ipc-schema-and-framing.md`
- Create: `docs/reviews/phase-3f-agent-health-ipc-admission-verification.md`

**Interfaces:**
- Consumes exact final metrics/test outputs.
- Produces truthful Phase 3F product status and next-slice gate.

- [ ] **Step 1: Run fresh final gates before writing evidence**

```powershell
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task IpcTransport
.\eng\build.ps1 -Task AgentIpcAdmission
.\eng\build.ps1 -Task Agent
.\eng\build.ps1 -Task Manager
cargo tree --locked --workspace
git diff --check
git fsck --no-progress --no-dangling
```

Record exact test count, binary sizes, working/private memory, deltas, PIDs/session, connect/handshake/Health/total timings, toolchain, and skipped remote/cross-user/C++ evidence.

- [ ] **Step 2: Update durable product truth**

State:

- real agent Health snapshot and measured admission server exist;
- default agent remains transport/Protobuf-free;
- admission ceilings passed or resident adoption remains blocked;
- manager remains disconnected and no auto-start/history/search/paste exists.

- [ ] **Step 3: Update ADR 0018 evidence**

Add agent admission measurements. Do not change ADR to Accepted because C++ parity, fuzzing, adjacent-version, bulk cleanup, and final resident linkage remain open.

- [ ] **Step 4: Write Phase 3F report**

Required sections:

- result/scope/non-goals;
- commits;
- package boundaries;
- shared health semantics;
- baseline/server orchestration;
- footprint methodology and exact results;
- security/privacy interpretation;
- test/CI/toolchain evidence;
- all failures discovered during TDD;
- remaining gates;
- next slice: C++ manager Health parity.

- [ ] **Step 5: Commit evidence**

```powershell
git add README.md CHANGELOG.md docs/operations/developer-setup.md docs/adr/0018-ipc-schema-and-framing.md docs/reviews/phase-3f-agent-health-ipc-admission-verification.md
git commit -m "docs: record agent Health IPC admission evidence"
```

- [ ] **Step 6: Verify final HEAD**

```powershell
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task AgentIpcAdmission
.\eng\build.ps1 -Task Agent
.\eng\build.ps1 -Task Manager
git diff --check
git show --check --oneline --stat HEAD
git fsck --no-progress --no-dangling
git status --short
```

Expected: all green and clean. Do not push or create a PR unless explicitly requested.
