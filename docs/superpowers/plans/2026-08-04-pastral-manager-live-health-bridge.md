# Pastral Manager Live Health Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the C++/WinUI manager to a real authenticated agent Health endpoint through a versioned Rust DLL bridge without direct storage access or XAML-thread blocking.

**Architecture:** Extract the Phase 3F Health server into a feature-gated agent module and binary candidate. Add a Rust `cdylib` with a fixed content-free C ABI, load it by exact full path from C++, and change the manager provider to perform one bounded Health request on a worker thread before applying state through the DispatcherQueue.

**Tech Stack:** Rust 1.97.1/Edition 2024, existing Protobuf 4.35.0-release schema, Phase 3E Windows named-pipe/DPAPI/HMAC transport, C ABI, C++20/C++/WinRT, WinUI 3, Windows App SDK 2.3.1, MSBuild, PowerShell 5.1.

## Global Constraints

- Default `pastral-agent.exe` remains transport/Protobuf-free.
- IPC-enabled agent code is available only through Cargo feature `ipc-health` and binary `pastral-agent-ipc.exe`.
- Manager and bridge never open SQLite/blob/clipboard directly.
- No network, HTTP, gRPC, JSON, Rust async runtime, or process-wide retry loop.
- All manager IPC runs off the XAML UI thread.
- No heap pointer, string ownership, Rust enum layout, `HANDLE`, secret, nonce, proof, UUID, or error text crosses the C ABI.
- Bridge timeout is bounded to 50–5,000 ms.
- Data-root UTF-16 input is bounded to 32,767 code units and rejects embedded NUL/invalid UTF-16.
- Debug synthetic and live diagnostic modes never mix.
- Release manager uses live bridge and honestly reports Disconnected/Error when unavailable.
- Automated gates never invoke clipboard-reading commands.

---

## File structure

### Agent candidate

- Modify `apps/agent/Cargo.toml` — optional IPC dependencies, feature, binary target.
- Create `apps/agent/src/ipc_health.rs` — reusable sequential Health server.
- Create `apps/agent/src/ipc_cli.rs` — strict `serve-health` parser.
- Create `apps/agent/src/ipc_main.rs` — feature-gated candidate executable.
- Modify `apps/agent/src/lib.rs` — conditional exports.
- Modify `apps/agent/src/error.rs` — no change unless shared errors are required; prefer dedicated IPC error.
- Create `apps/agent/tests/ipc_health.rs` — feature-gated server tests.
- Create `apps/agent/tests/ipc_cli.rs` — feature-gated CLI tests.
- Modify `apps/agent-ipc-probe/src/server.rs` — delegate to shared server for one connection.

### Rust bridge

- Create `crates/manager-ipc-bridge/Cargo.toml` — `cdylib` + `rlib`.
- Create `crates/manager-ipc-bridge/src/lib.rs` — safe exports.
- Create `crates/manager-ipc-bridge/src/client.rs` — bounded Health client and status mapping.
- Create `crates/manager-ipc-bridge/src/abi.rs` — `repr(C)` types and constants.
- Create `crates/manager-ipc-bridge/src/ffi.rs` — only reviewed unsafe boundary.
- Create `crates/manager-ipc-bridge/include/pastral_manager_ipc_bridge.h` — repository-owned C header.
- Create `crates/manager-ipc-bridge/tests/client.rs` — live/missing/wrong-secret/protocol tests.
- Create `crates/manager-ipc-bridge/tests/abi.rs` — size/alignment/status/header parity.
- Create `crates/manager-ipc-bridge/tests/ffi.rs` — null/UTF-16/timeout/panic containment.

### Native manager

- Create `apps/manager/Pastral.Manager/Services/ManagerIpcBridge.h` — loader/result wrapper.
- Create `apps/manager/Pastral.Manager/Services/ManagerIpcBridge.cpp` — full-path secure load and call.
- Modify `apps/manager/Pastral.Manager/Services/IManagerDataProvider.h` — async completion contract.
- Modify `apps/manager/Pastral.Manager/Services/ManagerDataProvider.cpp` — synthetic/live provider and worker.
- Modify `apps/manager/Pastral.Manager/Pages/HomePage.xaml.h/.cpp` — Loading + DispatcherQueue completion.
- Modify `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.h/.cpp` — same async/generation behavior.
- Modify `apps/manager/Pastral.Manager/ViewModels/ManagerState.h` — optional schema/integrity native fields only if UI needs them.
- Modify `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj` — Rust bridge build/copy and source/header entries.
- Modify `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw` — truthful live state copy.

### Verification

- Create `eng/verify-manager-ipc-bridge.ps1` — Rust ABI/native load/Release manager live smoke.
- Modify `eng/verify-native-manager.ps1` — preserve synthetic smoke and add/live delegation if appropriate.
- Modify `eng/build.ps1` — `ManagerIpcBridge` task and `Full` integration.
- Modify `eng/verify-dependencies.ps1` — exact bridge/feature package rules.
- Modify `eng/verify-source-policy.ps1` — exact FFI/DLL-load/process-test boundaries.
- Modify `.github/workflows/rust-ci.yml` — Rust bridge and agent feature tests.
- Add/update native CI workflow if one exists; otherwise document hosted native UIA as local evidence only.

---

### Task 1: Feature-gated shared agent Health server

**Files:**
- Modify: `apps/agent/Cargo.toml`
- Create: `apps/agent/src/ipc_health.rs`
- Create: `apps/agent/src/ipc_cli.rs`
- Create: `apps/agent/src/ipc_main.rs`
- Modify: `apps/agent/src/lib.rs`
- Test: `apps/agent/tests/ipc_health.rs`
- Test: `apps/agent/tests/ipc_cli.rs`
- Modify: `apps/agent-ipc-probe/src/server.rs`

**Interfaces:**
- Produces:
  - `HealthServerConfig::new(data_root: PathBuf, max_connections: NonZeroUsize, timeout: Duration)`
  - `serve_health<W: Write>(config: HealthServerConfig, output: &mut W) -> Result<HealthServerSummary, AgentIpcError>`
  - `parse_ipc_arguments(...) -> Result<AgentIpcCommand, AgentIpcCliError>`
- Consumes existing `load_health_snapshot`, `pastral-ipc-win`, `pastral-ipc-schema`, and `pastral-ipc-auth`.

- [ ] **Step 1: Add failing CLI tests**

Test exact accepted form:

```rust
serve-health --data-root C:\temp\root --max-connections 4
```

Reject missing command/root/value, zero, values above 16, duplicate flags, unknown flags, and positional arguments.

- [ ] **Step 2: Run RED CLI test**

```powershell
cargo test --locked -p pastral-agent --features ipc-health --test ipc_cli
```

Expected: FAIL because feature/module/parser do not exist.

- [ ] **Step 3: Add failing server tests**

Cover:

- real Health response for two sequential clients;
- Health snapshot reloaded before the second response;
- first-instance collision before readiness;
- authenticated non-Health rejection;
- max-connections termination;
- content-free readiness/summary output;
- default agent tests compile without feature.

- [ ] **Step 4: Run RED server test**

```powershell
cargo test --locked -p pastral-agent --features ipc-health --test ipc_health
```

Expected: FAIL because server API does not exist.

- [ ] **Step 5: Add optional dependencies and feature**

```toml
[features]
default = []
ipc-health = [
  "dep:pastral-ipc-auth",
  "dep:pastral-ipc-core",
  "dep:pastral-ipc-schema",
  "dep:pastral-ipc-win",
]

[[bin]]
name = "pastral-agent-ipc"
path = "src/ipc_main.rs"
required-features = ["ipc-health"]
```

Use optional path dependencies only.

- [ ] **Step 6: Implement strict IPC CLI**

Implement `AgentIpcCommand::ServeHealth { data_root, max_connections }` with default one connection and exact 1–16 bound.

- [ ] **Step 7: Implement shared server**

Use Phase 3E DACL/pipe/peer/handshake code. Reload `AgentHealthSnapshot` before each response. Accept one operation per connection and close after response. Emit only:

```text
agent-ipc-ready=1
agent-ipc-connections-served=<n>
```

- [ ] **Step 8: Implement feature-gated binary**

Parse arguments, call server, return exit 2 for invalid invocation and exit 1 for runtime/security failure.

- [ ] **Step 9: Migrate admission probe server**

Delegate one-connection serving to the shared agent API while preserving Phase 3F output and footprint gates.

- [ ] **Step 10: Run GREEN tests and dependency isolation**

```powershell
cargo test --locked -p pastral-agent --all-targets
cargo test --locked -p pastral-agent --features ipc-health --all-targets
cargo test --locked -p pastral-agent-ipc-probe --all-targets
cargo tree --locked -p pastral-agent
cargo tree --locked -p pastral-agent --features ipc-health
```

Expected: default tree has no Protobuf/IPC packages; feature tree contains only exact approved packages.

- [ ] **Step 11: Run format/Clippy/policies**

```powershell
cargo fmt --all -- --check
cargo clippy --locked -p pastral-agent --all-targets --all-features -- -D warnings
.\eng\verify-agent.ps1 -Mode All
.\eng\verify-agent-ipc-admission.ps1 -Mode All
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
```

- [ ] **Step 12: Commit**

```bash
git add apps/agent apps/agent-ipc-probe Cargo.toml Cargo.lock eng
git commit -m "feat: add feature-gated agent Health server"
```

---

### Task 2: Safe Rust manager Health client

**Files:**
- Create: `crates/manager-ipc-bridge/Cargo.toml`
- Create: `crates/manager-ipc-bridge/src/lib.rs`
- Create: `crates/manager-ipc-bridge/src/client.rs`
- Test: `crates/manager-ipc-bridge/tests/client.rs`
- Modify: root `Cargo.toml`

**Interfaces:**
- Produces:
  - `ManagerHealthStatus`
  - `ManagerHealthSnapshot`
  - `query_health(data_root: &Path, timeout: Duration) -> ManagerHealthSnapshot`
- Consumes Task 1 agent Health server only in tests.

- [ ] **Step 1: Write failing live client test**

Start `serve_health` in a thread, query once, and assert Connected, schema version 1, all integrity flags, nonzero server PID/session, and bounded timings.

- [ ] **Step 2: Write failing status tests**

Cover missing pipe → Disconnected, expired deadline → Timeout, wrong secret → AuthenticationFailed, protocol-major mismatch → ProtocolMismatch, unhealthy response → Unhealthy.

- [ ] **Step 3: Run RED**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test client
```

Expected: FAIL because package/client do not exist.

- [ ] **Step 4: Create package and safe client**

Use `cdylib` and `rlib`. Implement one connect, one mutual handshake, one Health request, one response, no retries. Map internal errors to stable status without exposing error strings.

- [ ] **Step 5: Run GREEN**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test client
cargo clippy --locked -p pastral-manager-ipc-bridge --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/manager-ipc-bridge
git commit -m "feat: add manager Health IPC client"
```

---

### Task 3: Fixed C ABI and generated-free header

**Files:**
- Create: `crates/manager-ipc-bridge/src/abi.rs`
- Create: `crates/manager-ipc-bridge/src/ffi.rs`
- Modify: `crates/manager-ipc-bridge/src/lib.rs`
- Create: `crates/manager-ipc-bridge/include/pastral_manager_ipc_bridge.h`
- Test: `crates/manager-ipc-bridge/tests/abi.rs`
- Test: `crates/manager-ipc-bridge/tests/ffi.rs`

**Interfaces:**
- Produces exact C exports from the design.
- Consumes Task 2 `query_health`.

- [ ] **Step 1: Write failing ABI layout tests**

Assert exact size, alignment, field offsets, status numeric values, reserved zero fields, and header constant parity.

- [ ] **Step 2: Write failing FFI tests**

Cover null path/result, zero/overlong length, embedded NUL, invalid UTF-16, timeout below/above bounds, valid missing-agent status, output initialization, and panic containment test hook under `cfg(test)`.

- [ ] **Step 3: Run RED**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test abi --test ffi
```

- [ ] **Step 4: Implement ABI types**

Use `#[repr(C)]` integer-only fields. No Rust bool or enum representation crosses the boundary.

- [ ] **Step 5: Implement reviewed FFI boundary**

Use Rust 2024 `#[unsafe(no_mangle)]`, one `unsafe` module, bounded UTF-16 conversion, `catch_unwind`, and deterministic result initialization.

- [ ] **Step 6: Write matching C header**

Use `stdint.h`, `stddef.h`, `extern "C"`, exact status constants, and `static_assert` guards when compiled as C++.

- [ ] **Step 7: Run GREEN and binary export inspection**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --all-targets
cargo build --locked -p pastral-manager-ipc-bridge --release
powershell -NoProfile -Command "dumpbin /exports target\release\pastral_manager_ipc_bridge.dll"
```

Require exactly the three public export names.

- [ ] **Step 8: Commit**

```bash
git add crates/manager-ipc-bridge
git commit -m "feat: expose manager IPC bridge ABI"
```

---

### Task 4: Native secure loader and probe

**Files:**
- Create: `apps/manager/Pastral.Manager/Services/ManagerIpcBridge.h`
- Create: `apps/manager/Pastral.Manager/Services/ManagerIpcBridge.cpp`
- Create: `apps/manager/Pastral.Manager/Tests/ManagerIpcBridgeProbe.cpp`
- Create or modify native probe project files under `apps/manager/Pastral.Manager/Tests/`
- Modify: `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj`

**Interfaces:**
- Produces `ManagerIpcBridge::QueryHealth(std::wstring const&, uint32_t)` returning a native value object.
- Consumes Task 3 DLL/header.

- [ ] **Step 1: Write native probe expectations**

Probe modes:

```text
--abi
--health --data-root <absolute path>
--missing-dll
```

Output only status/schema/flags/PID/session/timing markers.

- [ ] **Step 2: Implement full-path DLL loader**

Resolve executable directory, append exact DLL name, use full-path `LoadLibraryExW`, resolve exact exports, check ABI version and result size.

- [ ] **Step 3: Implement native result mapping**

Reject unknown status, nonzero reserved fields, size/version mismatch, or impossible Connected flags.

- [ ] **Step 4: Build and run probe**

Start Task 1 server on a disposable root and run native probe. Also verify missing DLL and missing agent states.

- [ ] **Step 5: Commit**

```bash
git add apps/manager/Pastral.Manager/Services apps/manager/Pastral.Manager/Tests apps/manager/Pastral.Manager/Pastral.Manager.vcxproj
git commit -m "feat: load manager IPC bridge securely"
```

---

### Task 5: Asynchronous WinUI provider and live states

**Files:**
- Modify: `apps/manager/Pastral.Manager/Services/IManagerDataProvider.h`
- Modify: `apps/manager/Pastral.Manager/Services/ManagerDataProvider.cpp`
- Modify: `apps/manager/Pastral.Manager/Pages/HomePage.xaml.h`
- Modify: `apps/manager/Pastral.Manager/Pages/HomePage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.h`
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/ViewModels/ManagerState.h`
- Modify: `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw`

**Interfaces:**
- Replaces synchronous `LoadSnapshot()` with async completion.
- Consumes Task 4 `ManagerIpcBridge`.

- [ ] **Step 1: Add Loading and generation tests to UIA script**

Require immediate Loading, eventual Connected, and stale-result rejection after retry.

- [ ] **Step 2: Change provider interface**

```cpp
virtual void LoadSnapshotAsync(std::function<void(ManagerSnapshot)> completion) = 0;
```

Provider owns one `std::jthread`, generation counter, and cancellation state.

- [ ] **Step 3: Implement data-root resolution**

Use `%LOCALAPPDATA%\Pastral` normally. Accept diagnostic override only with both required environment variables and strict absolute-path validation.

- [ ] **Step 4: Preserve Debug synthetic mode**

Debug without diagnostic environment returns six existing synthetic records. Debug diagnostic and all Release runs use live bridge.

- [ ] **Step 5: Implement live mapping**

Map Connected/Disconnected/ProtocolMismatch/Error exactly as the spec. Clear clips before Loading and on every non-Connected state.

- [ ] **Step 6: Marshal page completion**

Use weak page references and `DispatcherQueue::TryEnqueue`. Discard stale generation callbacks.

- [ ] **Step 7: Build Debug/Release**

```powershell
.\eng\verify-native-manager.ps1 -Mode Build
```

- [ ] **Step 8: Run existing synthetic UIA**

```powershell
.\eng\verify-native-manager.ps1 -Mode Smoke
```

- [ ] **Step 9: Commit**

```bash
git add apps/manager/Pastral.Manager
git commit -m "feat: show live manager Health state"
```

---

### Task 6: Live manager verifier, policies, and CI

**Files:**
- Create: `eng/verify-manager-ipc-bridge.ps1`
- Modify: `eng/verify-native-manager.ps1`
- Modify: `eng/build.ps1`
- Modify: `eng/verify-dependencies.ps1`
- Modify: `eng/verify-source-policy.ps1`
- Modify: `.github/workflows/rust-ci.yml`

**Interfaces:**
- Produces `ManagerIpcBridge` build task and live UIA gate.

- [ ] **Step 1: Add static gate**

Require bridge package/header/export names, feature-gated agent binary, async provider contract, secure full-path loader flags, no direct storage/clipboard/network/process spawn in bridge.

- [ ] **Step 2: Add Rust gate**

Run agent feature tests, bridge tests, Release bridge build, default/feature dependency trees, and export inspection.

- [ ] **Step 3: Add native live smoke**

Create disposable root, start `pastral-agent-ipc` with bounded connections, launch Release manager with diagnostic environment, verify Connected Health, terminate server, Retry, verify Disconnected and zero items, close cleanly.

- [ ] **Step 4: Integrate build dispatcher**

Add `ManagerIpcBridge` and include it in `Full` before native manager build.

- [ ] **Step 5: Update dependency/source policies**

Keep default agent Protobuf-free; allow exact IPC packages for feature/bridge only. Permit unsafe only in bridge `ffi.rs` and DLL loading only in exact C++ service.

- [ ] **Step 6: Update CI**

Run Rust bridge/feature tests and static native policy in hosted Windows CI. Mark interactive UIA local until hosted desktop evidence exists.

- [ ] **Step 7: Run gates**

```powershell
.\eng\build.ps1 -Task ManagerIpcBridge
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
```

- [ ] **Step 8: Commit**

```bash
git add eng .github apps/manager/Pastral.Manager/Pastral.Manager.vcxproj
git commit -m "ci: verify manager live Health bridge"
```

---

### Task 7: Aggregate verification and Phase 3G evidence

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Modify: `docs/adr/0018-ipc-schema-and-framing.md`
- Create: `docs/reviews/phase-3g-manager-live-health-bridge-verification.md`

- [ ] **Step 1: Run aggregate Rust gates**

```powershell
.\eng\build.ps1 -Task All
```

Record actual test count.

- [ ] **Step 2: Run focused runtime/native gates**

```powershell
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task IpcTransport
.\eng\build.ps1 -Task AgentIpcAdmission
.\eng\build.ps1 -Task Agent
.\eng\build.ps1 -Task ManagerIpcBridge
.\eng\build.ps1 -Task Manager
```

- [ ] **Step 3: Measure bridge/manager impact**

Record bridge DLL bytes, Release manager bytes/private working set, connection/Health latency, and UI responsiveness ceiling.

- [ ] **Step 4: Update product truth**

State that manager live Health works through the Rust bridge; History/Search/Paste and production agent lifecycle remain unimplemented.

- [ ] **Step 5: Write verification report**

Include commits, architecture, ABI, security/negative tests, native UIA evidence, measurements, aggregate counts, limitations, and Phase 3H gate.

- [ ] **Step 6: Commit evidence**

```bash
git add README.md CHANGELOG.md docs
git commit -m "docs: record manager live Health bridge evidence"
```

- [ ] **Step 7: Run final integrity**

```powershell
git show --check --oneline --stat HEAD
git fsck --no-dangling
git status --short --branch
```

Expected: clean `main`, no tracked build output or secrets.
