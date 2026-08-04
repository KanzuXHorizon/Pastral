# Phase 3G Manager Live Health Bridge Verification

**Date:** 2026-08-05

**Baseline:** `d2a4c14`

**Implementation head before evidence:** `d8db64c`

**Scope:** feature-gated real agent Health serving, fixed-size Rust-to-C ABI, secure native DLL loading, asynchronous manager provider, explicit live connection states, build/dependency/source/CI policy, native probe, and Release UI Automation

## 1. Result

Phase 3G is **green for its declared content-free manager Health bridge scope**.

The implemented vertical slice is:

```text
disposable real agent data root
  -> identity + privacy policy + SQLite/FTS integrity
  -> feature-gated bounded pastral-agent-ipc server
  -> protected session-scoped named pipe
  -> kernel peer validation + mutual HMAC authentication
  -> Protobuf Health request/response through the reviewed Rust stack
  -> fixed-size versioned C ABI result
  -> secure exact-path native DLL loader
  -> persistent off-XAML-thread manager provider
  -> Loading / Connected / Disconnected / ProtocolMismatch / Error UI
  -> clear-on-disconnect and generation-based stale-result rejection
```

The manager still does not open SQLite or blob storage directly. The bridge returns only content-free Health truth. Release contains no synthetic History data.

## 2. Commits

```text
9f78519 docs: specify manager live Health bridge
2051d48 docs: plan manager live Health bridge
41d4c43 feat: add feature-gated agent Health server
fa7c9c8 feat: add manager Health IPC client
bfc8ebf feat: expose manager IPC bridge ABI
2e2abcc feat: load manager IPC bridge securely
b78706c feat: show live manager Health state
d8db64c feat: polish live manager health experience
```

The final evidence/documentation commit follows this report.

## 3. Agent Health server boundary

`pastral-agent-ipc.exe` is a feature-gated binary rather than a second default resident executable.

Its contract is deliberately narrow:

- exact `serve-health --data-root <path> [--max-connections <1..16>]` invocation;
- no positional, duplicate, empty, unknown, or out-of-range arguments;
- real identity, privacy policy, storage, FTS, and metadata integrity reload for each request;
- one authenticated Health capability only;
- authenticated non-Health operations rejected;
- first-instance collision fails before readiness;
- bounded connection count and deterministic process exit;
- no clipboard payload, preview, source path, query, digest, secret, nonce, proof, SID, or pipe name in normal output.

The default `pastral-agent.exe` remains Protobuf/transport-free and is not auto-started.

## 4. Bridge ABI

`pastral-manager-ipc-bridge` is a Rust `cdylib` that reuses the accepted Rust IPC stack rather than implementing a second C++ schema/authentication client.

The exported ABI is intentionally fixed and small:

```text
pastral_manager_ipc_abi_version
pastral_manager_ipc_result_size
pastral_manager_ipc_health_w
```

The result layout and every status value are tested against the repository C header. ABI version and structure size are validated before use. The FFI boundary:

- accepts a bounded UTF-16 data-root path and timeout;
- rejects null output, invalid UTF-16, invalid path, invalid timeout, and ABI mismatch;
- initializes the full result before returning;
- contains Rust panic across the C boundary;
- maps disconnect, timeout, authentication failure, protocol mismatch, unhealthy response, invalid argument, and internal failure to explicit fail-closed statuses;
- exposes no Rust allocation or ownership across the ABI.

## 5. Secure native loading

The C++ manager loader:

- derives the directory of `pastral-manager.exe` with `GetModuleFileNameW`;
- uses the exact deployed filename `pastral-manager-ipc-bridge.dll`;
- loads through `LoadLibraryExW` with DLL-local dependency search restriction;
- validates the three expected exports, ABI version, and result size;
- never searches PATH or the current working directory;
- never falls back to direct storage access when the bridge is missing or incompatible.

The MSBuild project builds the locked Rust bridge and copies `pastral_manager_ipc_bridge.dll` beside the manager under the exact deployed hyphenated name.

## 6. Asynchronous provider and UI state

The provider owns one persistent worker rather than creating an unbounded thread per refresh.

Behavior proved by implementation and UI Automation:

- XAML startup shows Loading immediately;
- bridge work occurs off the XAML thread;
- a newer refresh supersedes an older pending generation;
- stale completion is rejected before UI mutation;
- Connected displays only Health/schema truth and zero History items;
- server exit plus Refresh transitions to Disconnected;
- disconnect and every error clear live values instead of retaining stale data;
- ProtocolMismatch remains distinct from ordinary disconnect/error;
- Debug synthetic preview remains clearly labeled and separate;
- Release never substitutes synthetic rows for unavailable live History.

Enforced UIA ceilings are 15 seconds for top-level window creation, 15 seconds for Connected state, 10 seconds for refresh-to-Disconnected, and 5 seconds for clean close.

## 7. Build and verification isolation

Initial concurrent native runs exposed shared-output corruption in the default manager build directories:

- `LNK1104` while `pastral-manager.exe` was locked;
- missing generated `App.xaml` after overlapping XAML generation;
- permission denial on the shared `pch.pch` intermediate.

The bridge and native manager verifiers now override both `OutDir` and `IntDir` into unique ignored directories:

```text
target\verification\pastral-manager-ipc-<GUID>\...
target\verification\pastral-manager-live-<GUID>\...
target\verification\pastral-native-manager-<GUID>\...
```

This preserves normal `apps\manager\Pastral.Manager\x64` artifacts and allows concurrent verification runs to compile independently. Static policy requires these overrides and verifies that native smoke launches the isolated Debug executable, so the isolation cannot be removed silently.

## 8. Aggregate Rust verification

Fresh `.\eng\build.ps1 -Task All` result on 2026-08-05:

```text
Rust 1.97.1 / x86_64-pc-windows-msvc: PASS
rustfmt: PASS
cargo check --locked --workspace --all-targets: PASS
cargo test --locked --workspace --all-targets: 228 PASS
Clippy --all-features -D warnings: PASS
cargo doc --no-deps: PASS
dependency policy: PASS
source policy: PASS
```

Test breakdown:

| Package/surface | Tests |
|---|---:|
| Agent, including feature-enabled Health server | 21 |
| Agent core | 18 |
| Agent IPC admission | 11 |
| Clipboard Win32 | 22 |
| Domain | 26 |
| IPC authentication | 8 |
| IPC core | 30 |
| IPC schema probe | 3 |
| IPC schema | 11 |
| IPC transport probe | 3 |
| IPC Win32 transport/process evidence | 28 |
| Manager IPC bridge | 14 |
| Storage | 33 |
| **Total** | **228** |

## 9. Focused gate results

### 9.1 IPC prototype

`.\eng\build.ps1 -Task IpcPrototype` passes 44 focused tests and 10,000/10,000 Release round trips.

Representative run:

```text
executable-bytes=380416
average-roundtrip-ns=183987
one-byte-average-ns=1102
coalesced-average-ns=889
max-body-capacity=7869
```

### 9.2 Authenticated transport

`.\eng\build.ps1 -Task IpcTransport` passes static policy, authentication/transport tests, and the Release cross-process Health smoke.

Representative run:

```text
connect-us=7223
handshake-us=311
health-us=42
total-us=44037
```

### 9.3 Agent IPC admission

`.\eng\build.ps1 -Task AgentIpcAdmission` passes shared Health, process-memory, admission, Release smoke, ceiling, and leak checks.

Representative run:

```text
default-agent-binary-bytes=2142720
admission-binary-bytes=2414592
binary-delta-bytes=271872
baseline-working-set-bytes=7319552
server-working-set-bytes=7909376
working-set-delta-bytes=589824
baseline-private-bytes=1171456
server-private-bytes=1171456
private-delta-bytes=0
connect-us=73
handshake-us=402
health-us=2334
```

No ceiling was increased.

### 9.4 Agent

`.\eng\build.ps1 -Task Agent` passes static policy, Debug/Release builds, and clipboard-free `health-check` smoke.

### 9.5 Manager Health bridge

`.\eng\build.ps1 -Task ManagerIpcBridge` covers:

- static source/build/CI/deployment policy;
- 21 feature-enabled agent tests;
- 14 bridge ABI/client/FFI tests;
- bridge Clippy with warnings denied;
- Release DLL build and exact export inspection;
- runtime/build dependency isolation;
- native C++ ABI probe;
- authenticated real-agent Health probe;
- isolated Release manager build;
- Connected-to-Disconnected UI Automation;
- bounded process cleanup and content-leak checks.

### 9.6 Native manager regression

`.\eng\build.ps1 -Task Manager` passes native policy, isolated Debug/Release builds, runtime window creation, History navigation, search filtering, selection details, no-results recovery, and clean close through UI Automation.

## 10. Representative Phase 3G measurements

Artifact sizes from the verified Release build:

| Artifact | Bytes |
|---|---:|
| `pastral_manager_ipc_bridge.dll` | 414,208 |
| `pastral-agent-ipc.exe` | 2,192,384 |
| `pastral-manager.exe` | 567,296 |
| native C++ IPC probe | 23,552 |

Representative native C++ bridge query:

```text
manager-ipc-probe=ok
status=0
storage-schema=1
capture-enabled=0
privacy-policy-ok=1
storage-integrity-ok=1
connect-us=72
handshake-us=417
health-us=2083
```

A diagnostic connected Release manager sample reported:

```text
working-set-bytes=108445696
private-bytes=150515712
handle-count=966
thread-count=47
```

The manager sample is recorded for trend tracking only. Phase 3G did not define or relax a manager-memory acceptance ceiling, and these values are not release SLAs.

## 11. CI and policy

Windows CI now watches the manager and bridge paths and runs bridge Static and Test modes. Interactive desktop UI Automation remains a local native gate until hosted interactive-desktop evidence exists.

Policy preserves these boundaries:

- the default agent remains Protobuf/transport-free;
- feature-gated agent IPC and the bridge may use only the exact reviewed IPC packages;
- the bridge has no async runtime, network stack, alternate database, JSON stack, or direct SQLite dependency;
- unsafe Rust is limited to the reviewed FFI implementation;
- native DLL loading is limited to the exact C++ bridge service;
- no automated gate reads or writes clipboard content;
- output is scanned for content-bearing and secret transport markers.

## 12. Scope still open

Phase 3G does not implement:

- production startup, single-instance ownership, supervision, reconnect/backoff, or shutdown integration for the default clipboard-owning resident agent;
- simultaneous clipboard listener plus IPC server budget/soak evidence;
- generated C++ Protobuf parser/runtime parity;
- parser/schema fuzzing or adjacent-version fixtures;
- History/Search paging and mutations over IPC;
- payload/bulk transfer and cleanup;
- paste replay, focus-safe confirmation, Quick Paste, or overlay;
- encryption and the Private profile;
- COM/OLE/image/file clipboard formats;
- packaging, installer, signing, updates, or final brand clearance.

## 13. Phase 3H gate

The next slice should establish the **production agent lifecycle and first read-only live History boundary** before any paste or mutation feature.

It must prove:

1. one supervised resident agent owns clipboard observation, storage, and IPC simultaneously;
2. startup, single-instance, reconnect/backoff, shutdown, crash recovery, and update handoff are bounded;
3. simultaneous clipboard and IPC load remains inside accepted resident budgets;
4. paged History/Search returns bounded preview metadata only, never direct database handles or unrestricted payloads;
5. cancellation, stale generation, disconnect, malformed response, unknown capability, and adjacent-version behavior fail closed;
6. manager Release continues to clear stale data and never opens storage directly;
7. compatibility, privacy, accessibility, and UI responsiveness gates remain green.

Paste, bulk payload transfer, packaging, and signing remain blocked until that lifecycle/read-only boundary is green.
