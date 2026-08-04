# Pastral Manager Live History/Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the authenticated read-only History/Search backend to the native manager through a caller-owned, versioned C ABI and render live Release data without direct storage access.

**Architecture:** Extend the existing Rust bridge additively: keep Health ABI v1 unchanged, add independently versioned read result/item structures and sizing/fill exports, then load and validate them from C++. The manager provider keeps one worker, performs backend History/Search off the XAML thread, rejects stale generations, and maps only validated UTF-8 preview metadata into existing presentation rows.

**Tech Stack:** Rust 1.97.1, Edition 2024, Protobuf 4.35.0-release, authenticated Windows named pipes, fixed C ABI, C++20/C++/WinRT, WinUI 3, Windows App SDK 2.3.1, PowerShell verification.

## Global Constraints

- Existing `PASTRAL_MANAGER_IPC_ABI_VERSION = 1` and 64-byte Health result remain unchanged.
- New read contract uses `PASTRAL_MANAGER_READ_ABI_VERSION = 1`, 64-byte result, and 64-byte item.
- No Rust pointer, allocation, iterator, or ownership crosses the C boundary.
- Manager never opens SQLite, FTS, clipboard APIs, or blob storage directly.
- At most 100 items and 256 KiB caller-owned UTF-8 arena per query.
- No raw representation/blob payloads; previews remain bounded metadata.
- Default resident agent remains Protobuf/transport-free.
- Automated tests never read or write the user's clipboard.
- Preserve unrelated dirty manager UI/i18n changes and stage exact files/hunks only.

---

### Task 1: Add the fixed read ABI contract

**Files:**
- Modify: `crates/manager-ipc-bridge/include/pastral_manager_ipc_bridge.h`
- Modify: `crates/manager-ipc-bridge/src/abi.rs`
- Modify: `crates/manager-ipc-bridge/src/lib.rs`
- Test: `crates/manager-ipc-bridge/tests/abi.rs`

**Interfaces:**
- Produces: `PastralManagerReadResult`, `PastralManagerClipItem`, read constants, `InsufficientBuffer = 9`, and three read contract size/version exports.

- [ ] **Step 1: Write failing layout/header tests**

Assert exact size, alignment, offsets, status value 9, kind/flag constants, and header markers for:

```text
PASTRAL_MANAGER_READ_ABI_VERSION
PASTRAL_MANAGER_READ_RESULT_BYTES
PASTRAL_MANAGER_CLIP_ITEM_BYTES
pastral_manager_ipc_read_abi_version
pastral_manager_ipc_read_result_size
pastral_manager_ipc_clip_item_size
pastral_manager_ipc_history_w
pastral_manager_ipc_search_w
```

- [ ] **Step 2: Run the ABI test and verify failure**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test abi
```

Expected: compile/test failure because the read structures/constants/exports do not exist.

- [ ] **Step 3: Implement exact `repr(C)` structures and constants**

Use the field order from the design. Add constructors that fully initialize failed, insufficient-buffer, and connected results with zero reserved fields.

- [ ] **Step 4: Export read version and size functions**

Implement pure functions returning the exact constants; do not change existing Health exports.

- [ ] **Step 5: Run focused ABI tests**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test abi
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add crates/manager-ipc-bridge/include/pastral_manager_ipc_bridge.h crates/manager-ipc-bridge/src/abi.rs crates/manager-ipc-bridge/src/lib.rs crates/manager-ipc-bridge/tests/abi.rs
git commit -m "feat: define manager read IPC ABI"
```

### Task 2: Add authenticated Rust History/Search bridge client

**Files:**
- Modify: `crates/manager-ipc-bridge/src/client.rs`
- Modify: `crates/manager-ipc-bridge/src/lib.rs`
- Test: `crates/manager-ipc-bridge/tests/client.rs`

**Interfaces:**
- Consumes: `HistoryPageRequestDto`, `SearchRequestDto`, exact read capability set.
- Produces: `ManagerClipItemSnapshot`, `ManagerReadPageSnapshot`, `query_history`, `query_search`.

- [ ] **Step 1: Write failing real-server client tests**

Start `pastral_agent::ipc_health::serve_read` on a disposable root and prove:

- History returns newest-first rows, unavailable state, and `has_more`;
- Search treats `OR` literally;
- Health-only server rejects read capability negotiation;
- wrong secret, timeout, malformed response, and unexpected response map to fail-closed statuses.

- [ ] **Step 2: Run focused client tests and verify failure**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test client
```

Expected: missing read query APIs.

- [ ] **Step 3: Extract bounded authenticated request helper**

Create an internal helper that loads existing transport material, derives the pipe, connects, validates peer identity, performs `client_handshake_with_capabilities` with exactly `[Health, HistoryPage, Search]`, sends one correlated control request, and returns one validated response plus peer/timing evidence.

Keep `query_health` on the existing Health-only handshake.

- [ ] **Step 4: Implement immutable read page snapshots**

Copy validated DTO fields into Rust-owned immutable values. Preserve event UUID bytes, capture order, timestamp, kind, preview, source label, pinned, unavailable, `has_more`, and request timing.

- [ ] **Step 5: Run client tests and Clippy**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test client
cargo clippy --locked -p pastral-manager-ipc-bridge --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add crates/manager-ipc-bridge/src/client.rs crates/manager-ipc-bridge/src/lib.rs crates/manager-ipc-bridge/tests/client.rs
git commit -m "feat: query read-only pages from manager bridge"
```

### Task 3: Implement caller-owned FFI sizing and fill

**Files:**
- Modify: `crates/manager-ipc-bridge/src/ffi.rs`
- Modify: `crates/manager-ipc-bridge/src/lib.rs`
- Test: `crates/manager-ipc-bridge/tests/ffi.rs`

**Interfaces:**
- Consumes: `query_history`, `query_search`, read ABI structures.
- Produces: `pastral_manager_ipc_history_w`, `pastral_manager_ipc_search_w`.

- [ ] **Step 1: Write failing invalid-input and buffer-protocol tests**

Cover null result, bad ABI/size, null/nonzero path/query, invalid UTF-16, relative path, timeout outside `50..=5000`, limit outside `1..=100`, invalid cursor, item pointer/capacity disagreement, text pointer/capacity disagreement, and capacity over 256 KiB.

Verify every non-null result is initialized and output buffers retain sentinel bytes on failure.

- [ ] **Step 2: Write failing real sizing/fill tests**

For History and Search:

1. call with zero capacities and null buffers;
2. assert `InsufficientBuffer` and exact required capacities;
3. allocate caller-owned item array and byte arena;
4. call again;
5. assert `Connected`, exact rows/offsets/lengths/flags, no Rust pointers, and unchanged bytes outside written ranges.

- [ ] **Step 3: Run FFI tests and verify failure**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test ffi
```

- [ ] **Step 4: Implement shared path/query validation and panic containment**

Reuse the Health path rules. Convert UTF-16 strictly, reject embedded NUL, and wrap all Rust work in `catch_unwind`.

- [ ] **Step 5: Implement no-partial-write copy planning**

Before writing caller output:

- validate all items;
- calculate exact UTF-8 arena bytes with checked arithmetic;
- reject more than 100 items or 256 KiB;
- return exact required capacities when insufficient;
- only then copy items and text.

- [ ] **Step 6: Run FFI/ABI/client tests and Clippy**

```powershell
cargo test --locked -p pastral-manager-ipc-bridge --test abi --test client --test ffi
cargo clippy --locked -p pastral-manager-ipc-bridge --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add crates/manager-ipc-bridge/src/ffi.rs crates/manager-ipc-bridge/src/lib.rs crates/manager-ipc-bridge/tests/ffi.rs
git commit -m "feat: expose caller-owned manager read buffers"
```

### Task 4: Extend secure C++ bridge loading and mapping

**Files:**
- Modify: `apps/manager/Pastral.Manager/Services/ManagerIpcBridge.h`
- Modify: `apps/manager/Pastral.Manager/Services/ManagerIpcBridge.cpp`
- Modify: `apps/manager/Pastral.Manager/Tests/ManagerIpcBridgeProbe.cpp`
- Modify: `eng/verify-manager-ipc-bridge.ps1`

**Interfaces:**
- Consumes: additive read exports and fixed read structs.
- Produces: `IsReadAvailable`, `QueryHistory`, `QuerySearch`, `ManagerIpcBridgePage`, `ManagerIpcBridgeClip`.

- [ ] **Step 1: Extend native probe with failing read ABI assertions**

Probe the exact read export set, sizes, sizing call, fill call, literal Search, unavailable row, insufficient buffer, and invalid UTF-8 rejection.

- [ ] **Step 2: Run native probe and verify failure**

```powershell
.\eng\verify-manager-ipc-bridge.ps1 -Mode Probe
```

Expected: read exports/mapping absent.

- [ ] **Step 3: Extend `BridgeApi` additively**

Keep Health availability independent. Resolve every read export from the same exact DLL path and validate read ABI/result/item sizes before `IsReadAvailable()` succeeds.

- [ ] **Step 4: Implement bounded two-call C++ queries**

Use zero-capacity sizing, allocate exact vectors with hard ceilings, fill, and retry at most once if capacities changed. Reject return/result disagreement, unknown status/kind/flags, nonzero reserved fields, invalid UUID/order/timestamp/ranges, stale output on failure, and invalid UTF-8.

- [ ] **Step 5: Run native probe and static gate**

```powershell
.\eng\verify-manager-ipc-bridge.ps1 -Mode Static
.\eng\verify-manager-ipc-bridge.ps1 -Mode Probe
```

Expected: PASS.

- [ ] **Step 6: Commit exact bridge/probe files**

```powershell
git add apps/manager/Pastral.Manager/Services/ManagerIpcBridge.h apps/manager/Pastral.Manager/Services/ManagerIpcBridge.cpp apps/manager/Pastral.Manager/Tests/ManagerIpcBridgeProbe.cpp eng/verify-manager-ipc-bridge.ps1
git commit -m "feat: map manager read IPC pages"
```

### Task 5: Add provider History/Search operations

**Files:**
- Modify: `apps/manager/Pastral.Manager/ViewModels/ManagerState.h`
- Modify: `apps/manager/Pastral.Manager/Services/IManagerDataProvider.h`
- Modify: `apps/manager/Pastral.Manager/Services/ManagerDataProvider.h`
- Modify: `apps/manager/Pastral.Manager/Services/ManagerDataProvider.cpp`
- Modify: `apps/manager/Pastral.Manager/Pages/HomePage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.h`
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.cpp`

**Interfaces:**
- Consumes: `ManagerIpcBridge::QueryHistory`, `QuerySearch`.
- Produces: asynchronous `LoadSnapshotAsync`, `SearchAsync`, `RefreshAsync`, `hasMore`, query/loading/error state.

- [ ] **Step 1: Add provider contract/static tests first**

Extend static verification to require off-thread History/Search calls, generation checks, clear-on-failure, no direct storage APIs, no per-keystroke unbounded thread creation, and Release synthetic exclusion.

- [ ] **Step 2: Run static gate and verify failure**

```powershell
.\eng\verify-native-manager.ps1 -Mode Static
```

- [ ] **Step 3: Extend manager state without breaking Debug preview**

Add `hasMore`, `query`, and operation state. Preserve existing synthetic snapshot behavior only under Debug.

- [ ] **Step 4: Implement worker commands and stale-generation rejection**

Use the existing persistent worker. Coalesce pending search to the newest query, increment generation for refresh/search, and marshal only accepted completion to the XAML dispatcher.

- [ ] **Step 5: Map validated live items**

Create canonical UUID text, localized type/source/unavailable/truncated metadata, deterministic relative time, and accessible automation names. Clear rows on every failed live connection.

- [ ] **Step 6: Run static and Debug/Release build gates**

```powershell
.\eng\verify-native-manager.ps1 -Mode Static
.\eng\verify-native-manager.ps1 -Mode Build
```

Expected: PASS.

- [ ] **Step 7: Commit provider/state files with selective staging**

Do not include unrelated visual-polish hunks already dirty in the checkout.

### Task 6: Connect WinUI History search and states

**Files:**
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml`
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.h`
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/Pages/HomePage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw`
- Modify: `apps/manager/Pastral.Manager/Strings/vi-VN/Resources.resw`
- Modify: `eng/verify-native-manager.ps1`

**Interfaces:**
- Consumes: provider live History/Search operations.
- Produces: localized/accessibility-complete live UI.

- [ ] **Step 1: Extend UI Automation expectations first**

Add deterministic diagnostic mode support that starts a disposable `serve-read` server with seeded storage and verifies:

- connected live History rows;
- literal Search result;
- no-results recovery;
- unavailable row description;
- partial count when `has_more`;
- server exit + refresh clears rows;
- clean close.

- [ ] **Step 2: Run Smoke and verify failure**

```powershell
.\eng\verify-native-manager.ps1 -Mode Smoke
```

- [ ] **Step 3: Wire 250 ms search debounce and refresh**

Empty query requests History. Nonempty query requests backend Search. Cancel timer on page unload and reject stale completions.

- [ ] **Step 4: Add localized accessible states**

Add English/Vietnamese resource keys for loading, partial, empty, no-results, unavailable, truncated, unknown source, and backend unavailable. Use polite live result counts and descriptive automation names.

- [ ] **Step 5: Run native Static/Build/Smoke**

```powershell
.\eng\verify-native-manager.ps1 -Mode Static
.\eng\verify-native-manager.ps1 -Mode Build
.\eng\verify-native-manager.ps1 -Mode Smoke
```

Expected: PASS.

- [ ] **Step 6: Commit UI/resources/verifier with selective staging**

Review every hunk because these files may contain pre-existing visual-polish work.

### Task 7: Integrate exports, CI, policy, and full verification

**Files:**
- Modify: `eng/verify-manager-ipc-bridge.ps1`
- Modify: `eng/build.ps1` only if a distinct gate is required
- Modify: `.github/workflows/rust-ci.yml`
- Modify: `.github/workflows/native-ui-ci.yml` only if interactive read smoke is supported
- Modify: `eng/verify-dependencies.ps1` or `eng/verify-source-policy.ps1` only when new assertions are needed

**Interfaces:**
- Produces: exact export/read contract policy and CI coverage.

- [ ] **Step 1: Make static/export policy fail on missing read contract**

Require all additive exports, exact sizes, no forbidden dependencies, caller-owned buffers, no direct manager storage access, and no content-bearing verification output.

- [ ] **Step 2: Run Static and observe expected failure before integration**

- [ ] **Step 3: Update gate/CI dispatch**

Keep UI Automation local unless the CI runner has an interactive desktop. Rust ABI/client/FFI tests and native compile/probe remain CI-safe.

- [ ] **Step 4: Run focused and aggregate gates**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-manager-ipc-bridge --all-targets
cargo clippy --locked -p pastral-manager-ipc-bridge --all-targets -- -D warnings
.\eng\verify-manager-ipc-bridge.ps1 -Mode All
.\eng\verify-native-manager.ps1 -Mode All
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task IpcTransport
.\eng\build.ps1 -Task AgentIpcAdmission
.\eng\build.ps1 -Task Agent
git diff --check
```

Expected: all PASS with no raised ceiling.

- [ ] **Step 5: Commit policy/CI integration**

```powershell
git add eng/verify-manager-ipc-bridge.ps1 .github/workflows/rust-ci.yml
git commit -m "ci: verify live manager History IPC"
```

### Task 8: Record evidence and remaining scope

**Files:**
- Create: `docs/reviews/phase-3i-manager-live-history-search-verification.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Modify: `docs/adr/0018-ipc-schema-and-framing.md`
- Modify: this plan to check completed tasks

**Interfaces:**
- Produces: repository truth and next acceptance gate.

- [ ] **Step 1: Record exact test counts, artifact sizes, timings, UIA scenarios, buffer ceilings, and failures found**

Mark machine-specific measurements as evidence, not SLA.

- [ ] **Step 2: State remaining limitations precisely**

Keep production lifecycle, pagination, Search continuation, mutation, paste, bulk payload, encryption, packaging, signing, and updates open.

- [ ] **Step 3: Run documentation and Git integrity checks**

```powershell
git diff --check
git fsck --no-dangling
git status --short --branch
```

- [ ] **Step 4: Commit evidence**

```powershell
git add README.md CHANGELOG.md docs/operations/developer-setup.md docs/adr/0018-ipc-schema-and-framing.md docs/reviews/phase-3i-manager-live-history-search-verification.md docs/superpowers/plans/2026-08-05-pastral-manager-live-history-search.md
git commit -m "docs: record live manager History evidence"
```
