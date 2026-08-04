# Pastral Phase 3G — Manager Live Health Bridge Design

**Date:** 2026-08-04

**Status:** Approved for implementation by the continuing project directive

## 1. Goal

Connect the native C++/WinUI manager to a real agent-owned, authenticated, content-free Health endpoint without allowing the manager to open SQLite/blob storage, blocking the XAML UI thread, duplicating security-critical IPC code in C++, or silently changing the default diagnostic agent dependency graph.

Phase 3G produces a working live manager connection state. It does not yet expose clipboard history, search results, payloads, paste, or destructive commands.

## 2. Decision

Use a small Rust `cdylib` IPC bridge loaded by the C++ manager.

The bridge reuses the verified Rust framing, Protobuf conversion, DPAPI material handling, named-pipe transport, peer validation, and HMAC handshake. C++ receives only a fixed-size content-free result structure through a versioned C ABI.

The agent side is integrated as a feature-gated Health server candidate built from the existing `pastral-agent` package. The default `pastral-agent.exe` remains transport/Protobuf-free. The IPC-enabled candidate is a separate binary target from the same package and uses the same agent library, identity, privacy, and storage Health truth.

### 2.1 Why this is the default

1. The Rust transport already passes security, malformed-input, timeout, replay, cross-process, and footprint gates.
2. Reimplementing frame parsing, Protobuf, DPAPI, HMAC, token/session validation, and cancellation in C++ would duplicate the highest-risk code before the manager has even proven one live operation.
3. The current machine/repository has no approved CMake/Ninja/libprotobuf-lite build path. Official Protobuf C++ lite generation is viable later, but introducing an additional toolchain now would slow the live vertical slice and expand build/supply-chain surface.
4. A fixed C ABI with no heap ownership crossing the boundary is narrow, testable, and reversible.

Official Protobuf C++ documentation confirms that `LITE_RUNTIME` removes reflection/descriptors and links against the smaller `libprotobuf-lite`, so a later C++ parity harness remains viable when its exact source-build toolchain is admitted. Phase 3G does not block that work.

### 2.2 Trade-offs accepted

- The manager output gains one Rust DLL and import/runtime deployment responsibility.
- FFI becomes a reviewed unsafe boundary.
- C++ does not independently parse the protocol in this slice.
- The production package must keep bridge DLL and manager versions aligned.

### 2.3 Reversal trigger

Reconsider direct C++ `libprotobuf-lite` only if one of these is proven:

- bridge DLL packaging/signing cannot be made reliable;
- bridge startup or private-memory cost exceeds the manager budget;
- ABI versioning materially blocks adjacent-version updates;
- a non-Rust client must share the exact native implementation;
- security review finds the FFI boundary riskier than a duplicated C++ client.

Rollback is removal of the bridge build/copy target and restoration of the existing disconnected Release provider. No database migration or stored-data change is involved.

## 3. Hard constraints

- Windows 11 x64 first.
- Rust `1.97.1`, Edition 2024, locked dependencies.
- C++20/C++/WinRT and Windows App SDK `2.3.1` remain the manager UI stack.
- Manager never opens SQLite, FTS, blob, staging, clipboard, or private agent files other than public transport identity plus DPAPI-protected transport material through the Rust bridge.
- All connect/read/write work runs off the XAML UI thread.
- No process-wide unbounded retry loop.
- No network API, HTTP, gRPC, JSON, async Rust runtime, or embedded browser.
- No clipboard-reading command is run by automated gates.
- Default `pastral-agent` remains Protobuf/transport-free.
- Live data is cleared on disconnect, protocol mismatch, authentication failure, timeout, or bridge unload.
- Debug synthetic data remains clearly labeled and never mixes with live Health state.

## 4. Scope

Phase 3G implements:

1. a feature-gated agent Health server binary target;
2. bounded multi-connection Health serving for diagnostic/live-manager use;
3. a Rust manager IPC bridge `cdylib` and C header;
4. fixed ABI/result validation and panic containment;
5. a native bridge probe for ABI/load/Health behavior;
6. secure full-path DLL loading from the manager executable directory;
7. asynchronous manager provider loading;
8. live Connected, Disconnected, ProtocolMismatch, and Error UI states;
9. retry initiated only by user action or one bounded startup attempt;
10. manager Release UI Automation with a real agent Health server;
11. dependency/source/build/CI policy and evidence.

Phase 3G does not implement:

- History/Search/payload return;
- subscriptions or change notifications;
- paste/replay or bulk transfer;
- agent auto-start or process supervision;
- long-running production clipboard + IPC lifecycle;
- C++ Protobuf runtime parity;
- Private/Sensitive reveal;
- settings/rules/profile mutation;
- packaging/signing/update delivery.

## 5. Agent Health server candidate

### 5.1 Cargo feature and binary

Add optional `ipc-health` dependencies to `apps/agent/Cargo.toml`:

- `pastral-ipc-auth`;
- `pastral-ipc-core`;
- `pastral-ipc-schema`;
- `pastral-ipc-win`.

Add a binary target:

```text
pastral-agent-ipc.exe
```

with `required-features = ["ipc-health"]`.

The default `pastral-agent.exe` target and default dependency graph remain unchanged.

### 5.2 Command

Exact diagnostic command:

```text
pastral-agent-ipc serve-health --data-root <path> [--max-connections <1..=16>]
```

No missing/unknown/duplicate/positional argument is accepted. The default connection count is one for diagnostics. A verifier may request enough connections for Home/History retry coverage.

### 5.3 Ownership and startup

The server candidate:

1. loads the shared `AgentHealthSnapshot`;
2. opens/owns storage through the agent library only;
3. creates/loads transport material;
4. creates the protected first pipe instance;
5. emits readiness only after Health and pipe security pass;
6. accepts one connection at a time;
7. performs mutual authentication and Health-only authorization;
8. reloads Health before each response so storage integrity is not cached indefinitely;
9. closes each connection after one response;
10. exits after the configured successful/terminal connection count;
11. releases storage and pipe resources on all paths.

This candidate must never run concurrently with another agent process over the same data root in automated tests.

## 6. Rust manager IPC bridge

### 6.1 Package

Create:

```text
crates/manager-ipc-bridge
```

Crate types:

```text
cdylib
rlib
```

The package depends only on:

- `pastral-ipc-core`;
- `pastral-ipc-schema`;
- `pastral-ipc-win`;
- `zeroize` where required by existing material handling.

It never depends on agent, storage, clipboard, WinUI, or process spawning.

### 6.2 C ABI

Export exactly:

```c
uint32_t pastral_manager_ipc_abi_version(void);
uint32_t pastral_manager_ipc_result_size(void);
int32_t pastral_manager_ipc_health_w(
    const uint16_t* data_root,
    size_t data_root_length,
    uint32_t timeout_ms,
    PastralManagerHealthResult* result);
```

`PastralManagerHealthResult` is a fixed `repr(C)` structure containing only:

- ABI version;
- structure size;
- status enum;
- storage schema version;
- integrity bit flags;
- server PID;
- session ID;
- bounded timing values;
- reserved zero fields.

No pointer, string, allocation, Rust enum layout, bool layout, `HANDLE`, UUID, secret, nonce, proof, or error text crosses the ABI.

### 6.3 Status values

Stable status enum:

```text
0 Connected
1 Disconnected
2 Timeout
3 ProtocolMismatch
4 AuthenticationFailed
5 Unhealthy
6 InvalidArgument
7 InternalError
8 AbiMismatch
```

Unknown values are treated by C++ as `Error`.

### 6.4 FFI safety

The bridge:

- validates null pointers and exact result size;
- bounds data-root UTF-16 length to 32,767 code units;
- rejects invalid UTF-16 and embedded NUL;
- bounds timeout to 50–5,000 ms;
- initializes the result to a deterministic failure state before work;
- catches panics with `catch_unwind` and never unwinds across FFI;
- contains all raw pointer access in one reviewed `ffi.rs` module;
- writes no secret or path to output/logs;
- zeroes reserved fields;
- performs one connection attempt and one Health request only.

## 7. Native DLL loading

The manager adds `ManagerIpcBridge.h/.cpp`.

Loading rules:

1. resolve the manager executable directory with `GetModuleFileNameW`;
2. append exact filename `pastral-manager-ipc-bridge.dll`;
3. call `LoadLibraryExW` with the full path and `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR`;
4. resolve exactly the three exported symbols;
5. require exact ABI version and result size;
6. reject missing/duplicate/unknown symbols or values;
7. free the module only after no worker call is active.

The loader never searches the current working directory, PATH, user-writable plugin directories, or registry paths.

## 8. Data-root selection

Production root:

```text
%LOCALAPPDATA%\Pastral
```

Diagnostic/UIA override is accepted only when both environment variables are present:

```text
PASTRAL_MANAGER_DIAGNOSTIC=1
PASTRAL_MANAGER_DATA_ROOT=<absolute path>
```

Rules:

- override must be absolute;
- empty, relative, device, UNC, or embedded-NUL paths fail closed;
- output never prints the path;
- manager still does not open storage; it passes the path to the bridge only;
- release documentation labels this override diagnostic and unsupported for normal launch.

## 9. Asynchronous manager provider

Replace synchronous `LoadSnapshot()` with:

```cpp
virtual void LoadSnapshotAsync(
    std::function<void(ManagerSnapshot)> completion) = 0;
```

Provider behavior:

- returns control immediately;
- performs bridge loading and IPC on one bounded worker thread;
- permits at most one active request per provider;
- a new Retry request cancels/invalidates the prior result generation;
- completion is invoked exactly once unless provider destruction cancels it;
- provider destruction joins/stops the worker without blocking indefinitely;
- callback receives native immutable `ManagerSnapshot` data only.

Pages:

1. apply a Loading snapshot immediately;
2. invoke provider asynchronously;
3. capture a C++/WinRT weak reference;
4. marshal completion through `DispatcherQueue::TryEnqueue`;
5. discard stale generation results;
6. apply Connected/Disconnected/ProtocolMismatch/Error on the UI thread.

No bridge or pipe operation occurs on the XAML UI thread.

## 10. UI state mapping

### Connected

Requirements:

- status title: `Pastral agent is connected`;
- status detail states authenticated Health verification;
- active profile remains `Ordinary` until profile IPC exists;
- storage summary includes schema version and integrity success only;
- capture value is `Connected`;
- clips remain empty because History is not implemented;
- no synthetic notice in Release/live mode.

### Disconnected

Used for missing pipe or unavailable agent.

- clear all clips;
- show Retry;
- do not claim corruption or authentication failure;
- no tight automatic retry.

### ProtocolMismatch

Used for frame/protocol/ABI mismatch.

- clear all clips;
- show Error severity;
- state that agent and manager versions are incompatible;
- Retry remains available only after update/restart.

### Error

Used for authentication failure, invalid DPAPI material, unhealthy integrity, invalid diagnostic root, bridge load failure, or internal error.

- clear all clips;
- show Error severity;
- never display secret/path/Win32 numeric details in normal UI.

### Loading

- applied before each startup/retry request;
- actions disabled;
- prior live data cleared immediately.

## 11. Debug synthetic behavior

Default Debug launch retains the existing six clearly labeled synthetic records.

When `PASTRAL_MANAGER_DIAGNOSTIC=1` is set, Debug uses the live bridge path instead of synthetic data. Synthetic and live records never coexist in one snapshot.

Release always uses the live bridge path and falls back to honest Disconnected/Error states.

## 12. Build integration

Add a manager pre-link target that:

1. invokes the pinned Cargo toolchain;
2. builds `pastral-manager-ipc-bridge` in matching Debug/Release profile;
3. copies the exact DLL into the manager output directory;
4. fails if the DLL or exported header is missing;
5. never downloads dependencies or tools;
6. keeps Cargo lock enforcement.

The C header is repository-owned and checked against Rust ABI layout tests. No generated binding tool is required.

The IPC-enabled agent candidate is built only by its dedicated verifier and manager live-Health smoke, not by the default agent gate.

## 13. Verification

### Rust

- ABI size/alignment/offset constants;
- null/invalid UTF-16/embedded-NUL/overlong root rejection;
- timeout bounds;
- panic containment;
- status mapping for missing pipe, timeout, protocol mismatch, wrong secret, unhealthy response, and success;
- no heap pointer crosses ABI;
- default agent tree remains Protobuf/transport-free;
- IPC agent feature tree contains only exact approved packages.

### Native bridge probe

A small C++ console probe validates:

- secure full-path DLL load;
- exact symbol resolution;
- ABI version/size;
- connected Health result against a real agent server;
- missing DLL, missing agent, incompatible ABI, and invalid root fail closed;
- output is content-free.

### Manager UI Automation

Live diagnostic run:

1. create disposable root;
2. launch `pastral-agent-ipc serve-health` with bounded connections;
3. launch Release manager with diagnostic environment;
4. wait for Home Connected state;
5. verify schema/integrity text contains no root or secret;
6. stop agent;
7. click Retry;
8. verify Disconnected and cleared item count;
9. verify UI remains responsive during bounded timeout;
10. close cleanly.

Existing Debug synthetic History filtering/selection/no-results coverage remains green.

## 14. Acceptance criteria

Phase 3G is complete when:

- feature-gated agent Health server passes bounded multi-connection tests;
- bridge ABI and native loader pass positive/negative tests;
- manager Release displays real Connected Health state through the bridge;
- stopping the server and retrying produces Disconnected with no stale live data;
- no IPC call blocks the UI thread;
- default agent remains Protobuf/transport-free;
- manager never opens storage directly;
- Debug synthetic and live modes remain separate;
- Debug/Release manager builds and both synthetic/live UIA smokes pass;
- aggregate Rust, dependency, source, native policy, and Git integrity gates pass;
- evidence records bridge/manager footprint and open risks.

## 15. Next slice

After Phase 3G passes, Phase 3H may add read-only paged History and literal Search over the same authenticated bridge. Paste/replay, bulk transfer, subscriptions, mutation, auto-start, and Private/Sensitive operations remain separate authorization and lifecycle slices.
