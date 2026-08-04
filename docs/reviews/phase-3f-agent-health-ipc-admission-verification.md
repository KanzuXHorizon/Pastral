# Phase 3F Agent Health IPC Admission Verification

**Date:** 2026-08-04

**Baseline:** `282a74a`

**Implementation head before evidence:** `1208187`

**Scope:** shared content-free agent Health truth, Windows process-memory evidence, strict baseline/server child admission executable, real authenticated agent Health exchange, Release footprint ceilings, dependency/source/build/CI integration

## 1. Result

Phase 3F is **green for its declared measured agent Health IPC admission scope**.

The repository now proves the following vertical slice:

```text
real disposable agent data root
  -> agent identity + privacy policy + SQLite/FTS integrity
  -> content-free AgentHealthSnapshot
  -> protected logon-session named pipe
  -> kernel PID/session/token validation
  -> mutual HMAC authentication
  -> one Health request
  -> real agent Health response
  -> bounded process cleanup
  -> measured Release binary/private/working-set evidence
```

This phase does not make the admission executable a second production agent. The default `pastral-agent` remains Protobuf/transport-free, is not auto-started, and is not connected to the WinUI manager.

## 2. Commits

```text
dd9eb38 docs: specify agent Health IPC admission
e66fe26 docs: plan agent Health IPC admission
e1c8f7e refactor: centralize agent Health snapshot
fc214e9 feat: measure IPC process memory
4d94666 test: add agent IPC admission baseline
8effc8a feat: serve authenticated agent Health probe
fea2377 test: measure agent IPC admission footprint
1208187 ci: verify agent IPC admission
```

Documentation truth and this report are committed after final verification.

## 3. Shared Health truth

`apps/agent/src/health.rs` owns `AgentHealthSnapshot` and `load_health_snapshot`.

The snapshot contains only:

- storage schema version;
- agent healthy boolean;
- privacy policy healthy boolean;
- SQLite integrity boolean;
- FTS integrity boolean;
- metadata integrity boolean;
- search mapping integrity boolean.

It contains no:

- clipboard text or bytes;
- preview or search query;
- source path or executable name;
- content hash or digest;
- clip/event identity;
- reconstructable user value.

The CLI `health-check` and IPC admission path use this same snapshot rather than duplicating storage/integrity logic. Schema conversion from storage `i64` to IPC `u32` is checked and fails closed.

## 4. Process-memory evidence boundary

`pastral-ipc-win` exposes a safe `ProcessMemorySnapshot` boundary backed by reviewed Win32 code in `src/sys.rs`.

The query:

- rejects PID zero;
- opens the process with bounded query access;
- checks `GetExitCodeProcess` and requires `STILL_ACTIVE`;
- calls `K32GetProcessMemoryInfo` with a fixed `PROCESS_MEMORY_COUNTERS_EX` structure;
- reports working-set and private-usage bytes;
- closes all handles through RAII;
- rejects invalid and terminated process IDs.

This API is diagnostic footprint evidence only. It is not used for authentication or authorization.

## 5. Admission executable

`apps/agent-ipc-probe` defines exactly three invocation shapes:

```text
pastral-agent-ipc-probe
pastral-agent-ipc-probe --baseline-child --data-root <path>
pastral-agent-ipc-probe --server-child --data-root <path>
```

Missing, unknown, duplicate, empty, and positional arguments fail closed.

### 5.1 Baseline child

The baseline child:

1. loads the real agent Health snapshot;
2. creates no pipe and no transport material;
3. emits one content-free readiness marker;
4. blocks on stdin EOF only;
5. rejects any non-whitespace stdin content as a command channel.

### 5.2 Server child

The server child:

1. loads the real agent Health snapshot before transport readiness;
2. creates/loads Phase 3E DPAPI transport material;
3. derives the session-scoped canonical pipe name;
4. builds and inspects the protected logon-SID-only DACL;
5. creates the first remote-rejecting byte-mode pipe instance;
6. emits readiness only after security/pipe creation passes;
7. accepts one client with a bounded deadline;
8. validates the kernel-reported client PID/session/token evidence;
9. completes mutual HMAC authentication with replay protection;
10. accepts exactly one Health request;
11. returns the real agent Health response;
12. rejects any other authenticated operation;
13. exits and releases storage/transport resources.

### 5.3 Parent

The parent:

- creates one disposable root;
- builds the default agent and admission footprint comparison from Release binaries;
- starts baseline and server children separately;
- waits for bounded readiness;
- samples working-set/private usage while each child is live;
- validates the server process ID through the named-pipe kernel peer query;
- performs one authenticated Health request;
- validates all returned integrity booleans and schema version;
- closes stdin, waits for clean child exit, and removes the disposable root;
- uses child guards to kill/wait on every error path;
- emits content-free metrics only.

## 6. Negative and security evidence

Focused tests prove:

- first-instance pipe collision fails before readiness;
- authenticated non-Health request is rejected;
- invalid arguments fail before child/transport creation;
- baseline stdin cannot become a command channel;
- zero, invalid, and terminated PIDs fail closed;
- metric underflow and zero values fail closed;
- each footprint ceiling fails when exceeded by one byte;
- exact ceiling values pass;
- negative runtime deltas are represented safely rather than underflowing;
- output contains no data root, pipe name, SID, secret, nonce, proof, clipboard content, preview, query, or content hash.

The admission package depends on the agent library and reviewed IPC packages, but not directly on `pastral-storage` or `pastral-clipboard-win`. The default agent dependency graph remains unchanged and Protobuf/transport-free.

## 7. Release footprint evidence

Explicit ceilings:

| Metric | Ceiling |
|---|---:|
| Server private usage | 25 MiB |
| Private usage delta | 8 MiB |
| Working-set delta | 12 MiB |
| Admission binary delta | 6 MiB |

Representative Release run:

```text
agent-ipc-admission=ok
cross-process=true
health=ok
admission-ceilings=passed
client-pid=80676
server-pid=80188
session-id=1
default-agent-binary-bytes=2142720
admission-binary-bytes=2413568
binary-delta-bytes=270848
baseline-working-set-bytes=6356992
baseline-private-bytes=1097728
server-working-set-bytes=6963200
server-private-bytes=1150976
working-set-delta-bytes=606208
private-delta-bytes=53248
connect-us=104
handshake-us=377
health-us=55
total-us=129969
```

A separate verifier run measured a 12,288-byte private delta. All runs remained materially below the declared ceilings. No ceiling was increased to make the evidence pass.

These machine-specific values are admission evidence, not release SLAs. They do not replace simultaneous resident clipboard-listener + IPC lifecycle measurement.

## 8. Verification results

### 8.1 Aggregate Rust gate

Fresh `eng/build.ps1 -Task All` result:

```text
Rust toolchain: PASS
rustfmt: PASS
cargo check --workspace --all-targets: PASS
cargo test --workspace --all-targets: 208 PASS
Clippy -D warnings: PASS
cargo doc: PASS
dependency policy: PASS
source policy: PASS
```

Test breakdown:

| Package/surface | Tests |
|---|---:|
| Agent | 16 |
| Agent core | 18 |
| Agent IPC admission | 11 |
| Clipboard Win32 | 22 |
| Domain | 26 |
| IPC authentication | 8 |
| IPC core | 30 |
| IPC schema probe | 3 |
| IPC schema | 11 |
| IPC transport probe | 3 |
| IPC Win32 transport | 27 |
| Storage | 33 |
| **Total** | **208** |

### 8.2 Dedicated admission gate

`eng/verify-agent-ipc-admission.ps1 -Mode All` passes:

- static package/API/dependency policy;
- 3 shared agent Health tests;
- 2 process-memory tests;
- 11 admission tests;
- default agent Release build;
- admission executable Release build;
- authenticated real-agent Health smoke;
- footprint ceilings;
- output leak scan;
- invalid invocation fail-closed behavior.

### 8.3 Adjacent regression gates

The following remained green after the admission implementation:

- IPC framing/schema prototype, including 10,000 Release round trips;
- authenticated IPC transport static/test/Release smoke;
- agent static, Debug/Release build, and clipboard-free Health smoke;
- dependency and source policy;
- workflow YAML parse.

Native manager Debug/Release and UI Automation are rerun on the final documentation head before Phase 3F is closed.

## 9. CI and policy

The Windows workflow now watches and verifies `apps/agent-ipc-probe/**` and runs the admission gate.

Policy guarantees:

- official `protobuf 4.35.0-release` remains exact;
- default `pastral-agent` remains Protobuf/transport-free;
- admission evidence may use the exact reviewed schema/transport packages;
- named-pipe APIs remain inside `ipc-win/src/sys.rs`;
- process spawning is limited to exact diagnostic/test boundaries;
- no product network API is introduced;
- no automated admission or agent gate reads the clipboard.

## 10. Scope still open

Phase 3F does not implement:

- production transport linkage inside `pastral-agent.exe`;
- simultaneous clipboard listener + IPC server lifecycle;
- agent single-instance/startup/shutdown supervision;
- C++ frame/schema/HMAC parity;
- manager live Health connection state;
- reconnect/backoff or UI-thread isolation evidence;
- live History/Search;
- bulk staging;
- parser/schema fuzzing;
- adjacent-version compatibility fixtures;
- auto-start, packaging, signing, or updates;
- COM/OLE formats, replay, Quick Paste, or overlay.

## 11. Next slice

Phase 3G may implement **C++ manager Health parity and live connection state**.

The next slice must prove:

1. exact C++ framing/schema/HMAC transcript parity with Rust fixtures;
2. user-scope DPAPI transport material access without leaking secrets;
3. kernel server PID/session/token validation;
4. bounded connect/read/write timeouts off the XAML UI thread;
5. live Connected/Disconnected/ProtocolMismatch/Error manager states;
6. clear-on-disconnect behavior with no stale live data;
7. Debug synthetic mode remains explicitly separate;
8. Release manager still never opens SQLite/blob storage directly.

History, Search, Paste, auto-start, and production resident linkage remain blocked until this cross-language Health path is green.
