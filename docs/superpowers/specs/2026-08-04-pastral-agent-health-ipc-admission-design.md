# Pastral Phase 3F — Measured Agent Health IPC Admission Design

**Date:** 2026-08-04

**Status:** Approved for implementation under the standing project-completion instruction

## 1. Purpose

Phase 3F proves that the authenticated Phase 3E transport can serve a real content-free agent Health response while preserving the resident-agent ownership and footprint gates.

The phase does not immediately link Protocol Buffers or `pastral-ipc-win` into the default `pastral-agent.exe`. Instead it creates a measured admission executable that reuses the real agent health/storage code, owns the diagnostic storage root as a single process, serves exactly one authenticated Health request, records binary/private-working-set evidence, and exits.

The result decides whether a later resident-agent build may adopt the transport. It is not an alternate production control host and is never run concurrently with a production agent over the same root.

## 2. Why this slice comes before manager integration

The durable process model requires:

- `pastral-agent.exe` to remain the exclusive storage/migration/blob owner;
- manager and CLI to use IPC only;
- resident footprint to remain within the accepted budget;
- Protobuf/runtime adoption to be evidence-based rather than assumed.

Three approaches were considered.

### 2.1 Directly link transport into the current agent

Advantages:

- shortest route to a `serve-health` command;
- closest to the final process topology.

Rejected for this slice because:

- it changes every agent build before incremental binary/private-working-set evidence exists;
- it invalidates the current dependency statement that the diagnostic resident agent remains Protobuf-free;
- a failed budget result would require backing transport dependencies out of a production binary.

### 2.2 Create an independent long-running control host

Advantages:

- leaves agent binary unchanged;
- manager could connect earlier.

Rejected because:

- another process would either open storage directly or need a second internal transport;
- direct storage access would violate exclusive ownership and duplicate migration/policy logic;
- it would create a process topology that the product does not intend to ship.

### 2.3 Measured admission executable reusing the agent library

Selected because:

- default `pastral-agent` dependency graph remains unchanged and Protobuf-free;
- the admission executable executes the real identity/privacy/storage health path;
- only one process opens the diagnostic storage root;
- binary and private-working-set impact can be measured before resident adoption;
- all work is reusable: the health snapshot and server transaction move into the agent after admission.

## 3. Scope

Phase 3F implements:

1. one content-free `AgentHealthSnapshot` in `pastral-agent`;
2. refactoring `health-check` to use that snapshot;
3. a new `pastral-agent-ipc-probe.exe` diagnostic package;
4. parent, baseline-child, and server-child modes;
5. one authenticated Health-only named-pipe transaction using the real agent snapshot;
6. current-process and child-process private-working-set measurement through the reviewed Windows boundary;
7. Release binary-size and memory comparison against default `pastral-agent.exe`;
8. strict static/dependency/source policy;
9. PowerShell/CI verification and Phase 3F evidence.

## 4. Non-goals

Phase 3F does not implement:

- agent auto-start or session-resident supervision;
- simultaneous clipboard listening and IPC;
- more than one connection or one request;
- history, search, detail, paste, delete, export, rules, subscriptions, or bulk transfer;
- manager/C++ client integration;
- a second storage owner or broker process;
- a Windows service;
- production discovery metadata outside the caller-selected disposable root;
- user-facing settings or UI;
- sensitive/private reveal;
- cross-user, remote, packaged-identity, RDP, or fast-user-switching acceptance;
- a claim that the admission executable is the final resident agent.

## 5. Package boundaries

### 5.1 `pastral-agent`

Remains:

- Protobuf-free;
- `pastral-ipc-auth`-free;
- `pastral-ipc-schema`-free;
- `pastral-ipc-win`-free;
- the owner of identity, privacy policy, storage configuration, and health evaluation.

Adds a serializer-neutral content-free API:

```rust
pub struct AgentHealthSnapshot {
    storage_schema_version: u32,
    capture_enabled: bool,
    privacy_policy_ok: bool,
    storage_integrity_ok: bool,
}

pub fn load_health_snapshot(root: &Path) -> Result<AgentHealthSnapshot, AgentRuntimeError>;
```

The type contains no path, SID, pipe name, profile name, clip count, source, preview, query, payload, hash, or secret.

`run_health_check` uses the same snapshot and continues to print current content-free diagnostic markers. The existing explicit CLI behavior remains unchanged.

### 5.2 `pastral-agent-ipc-probe`

A separate non-production package under `apps/agent-ipc-probe`.

Dependencies:

- `pastral-agent`;
- `pastral-ipc-auth`;
- `pastral-ipc-core`;
- `pastral-ipc-schema`;
- `pastral-ipc-win`.

It contains no clipboard APIs, no direct `pastral-storage` dependency, no direct SQLite path, no manager code, no network, no registry, and no async runtime.

Process spawning is permitted only in its exact main/integration-test files.

### 5.3 `pastral-ipc-win`

Adds a narrow process-memory query API behind the existing reviewed unsafe boundary:

```rust
pub struct ProcessMemorySnapshot {
    working_set_bytes: u64,
    private_usage_bytes: u64,
}

pub fn process_memory_snapshot(process_id: u32)
    -> Result<ProcessMemorySnapshot, TransportError>;
```

Implementation uses Windows process-memory counters with bounded fixed structures and minimum process query access. Handles are closed on every path. Memory metrics are diagnostics, not authorization evidence.

## 6. Probe modes

Strict argument shapes:

```text
pastral-agent-ipc-probe
pastral-agent-ipc-probe --baseline-child --data-root <path>
pastral-agent-ipc-probe --server-child --data-root <path>
```

No aliases, optional flags, arbitrary pipe names, iterations, production-root defaults, or positional arguments are accepted.

### 6.1 Parent mode

1. create a unique disposable root;
2. create/load Phase 3E transport material without opening storage;
3. build/locate the default Release `pastral-agent.exe` and current Release probe;
4. start baseline child;
5. wait for a content-free readiness line;
6. query baseline child memory;
7. close baseline child stdin and require clean exit;
8. start server child;
9. wait for a content-free readiness line after storage and pipe initialization;
10. query server child memory;
11. connect using the Phase 3E authenticated client;
12. validate server PID/session/token evidence;
13. complete mutual HMAC handshake;
14. send one Health request;
15. validate the Health response against real snapshot invariants;
16. require clean server-child exit;
17. inspect Release executable sizes;
18. compute bounded deltas;
19. remove the disposable root;
20. print content-free metrics.

### 6.2 Baseline child

1. load the real `AgentHealthSnapshot` from the selected root;
2. hold the initialized snapshot and storage-completed state in memory;
3. print `agent-baseline-ready=ok`;
4. wait only for stdin EOF;
5. exit.

It does not create transport material, a pipe, or a listener.

### 6.3 Server child

1. load the real `AgentHealthSnapshot` from the selected root;
2. load Phase 3E transport material;
3. derive current-session pipe name;
4. build and inspect logon-SID-only security;
5. create one first-instance server;
6. print `agent-health-server-ready=ok` only after the pipe exists;
7. accept one bounded client;
8. validate peer kernel/token evidence;
9. complete mutual authentication;
10. require one correlated Health request;
11. map the held `AgentHealthSnapshot` to `HealthResponseDto`;
12. send one response;
13. close and exit.

Storage is opened/evaluated before the pipe begins accepting requests. The response does not trigger a second storage open or long-running integrity scan.

## 7. Health semantics

The snapshot means:

- `storage_schema_version > 0` — storage opened and supplied its schema version;
- `capture_enabled = false` — this admission executable does not run clipboard capture;
- `privacy_policy_ok = true` — strict privacy configuration loaded or was atomically created;
- `storage_integrity_ok = true` — SQLite, FTS, metadata, and search mapping checks all passed.

Any identity, privacy, storage-open, runtime-info, or integrity failure prevents readiness and prevents pipe creation.

The IPC response is content-free and does not include `data_root` even though CLI `health-check` retains its explicit local diagnostic path marker.

## 8. Process synchronization

Readiness uses one line on child stdout because:

- it is content-free;
- it avoids another synchronization API or discovery channel;
- it lets the parent measure memory at a defined initialized state;
- the child still accepts no command over stdout.

Rules:

- readiness line maximum 64 bytes;
- unexpected, duplicate, oversized, EOF-before-ready, or stderr output fails closed;
- parent never logs the disposable root;
- baseline child shutdown is stdin EOF;
- server child exits after one Health response;
- child lifetime has a hard parent deadline;
- parent kills a stuck diagnostic child only as test/probe cleanup, not as production supervision policy.

## 9. Footprint evidence

### 9.1 Executable metrics

Report:

- default Release agent bytes;
- Release admission-probe bytes;
- binary delta bytes.

The binary delta is an upper-bound admission signal because the probe also includes parent orchestration and diagnostic metric code not required by a final resident server.

### 9.2 Process memory metrics

At initialized readiness, report:

- baseline child working set;
- baseline child private usage;
- server child working set;
- server child private usage;
- working-set delta;
- private-usage delta.

The baseline and server use the same executable, eliminating code-layout differences from the runtime initialization comparison. Absolute server private usage is also checked against the existing 25 MiB resident target.

### 9.3 Admission ceilings

Initial smoke ceilings, not release SLAs:

- server child private usage: `<= 25 MiB`;
- transport initialization private delta: `<= 8 MiB`;
- transport initialization working-set delta: `<= 12 MiB`;
- admission-probe binary delta over default agent: `<= 6 MiB`;
- connect: `< 5 s` hard timeout;
- handshake and Health each `< 2 s` hard timeout;
- all reported byte/duration metrics nonzero where applicable.

If a ceiling fails, resident linkage remains blocked and the evidence is recorded rather than hidden by raising the limit.

## 10. Security and privacy

- The admission root is disposable and current-user-only according to normal temp/app-data ACL behavior; pipe access is still explicit logon-SID-only.
- DPAPI remains user scope and is not represented as same-user malware isolation.
- Parent trusts no PID/session field from messages; it validates kernel-reported server identity.
- Server validates kernel-reported client identity before schema parsing.
- No root, pipe, SID, secret, nonce, proof, clipboard, query, preview, hash, or source string may appear in normal probe output.
- Child errors are mapped to broad result classes; raw Windows paths and cryptographic material are not forwarded.
- The process-memory API is diagnostic only and never participates in authentication or authorization.
- Automated tests do not open or mutate the clipboard.

## 11. Error handling

Distinct parent result classes:

- invalid invocation;
- environment/root setup;
- material initialization;
- baseline readiness;
- server readiness;
- child process failure;
- memory query failure;
- transport unavailable;
- peer validation;
- authentication;
- Health protocol/semantic mismatch;
- footprint ceiling failure;
- cleanup failure.

Errors fail closed and return nonzero. The probe never falls back to direct storage access in the parent or to an unauthenticated pipe.

## 12. Tests

### 12.1 Agent health snapshot

- real disposable storage snapshot is valid and content-free;
- health-check output and snapshot agree on schema/integrity state;
- corrupted integrity prevents a successful snapshot;
- no clipboard API is invoked;
- snapshot type has no content-bearing fields or `Debug` payload.

### 12.2 Process memory

- current-process query returns nonzero working/private bytes;
- child-process query succeeds;
- zero/invalid/exited PID fails closed;
- handles remain closeable across repeated queries.

### 12.3 Probe parsing and children

- exact three modes accepted;
- missing, duplicate, unknown, empty, and positional arguments rejected;
- baseline child readiness/EOF exit;
- server child readiness/one request/exit;
- readiness size and stderr rules;
- stuck/missing readiness timeout cleanup.

### 12.4 Cross-process flow

- client/server PIDs distinct;
- current session matches;
- real agent Health response succeeds;
- wrong material fails before response;
- first-instance collision fails;
- invalid request after authentication fails;
- output leak scan passes;
- disposable root removed;
- footprint metrics and ceilings pass.

## 13. Verification and CI

Create `eng/verify-agent-ipc-admission.ps1` with `Static`, `Test`, `Smoke`, and `All` modes.

Static checks:

- default `pastral-agent` has no IPC/Protobuf dependencies;
- admission app has exact allowed dependencies;
- no direct storage/clipboard/network/registry/manager dependency in admission app;
- process spawning only in exact admission main/test files;
- Windows memory API only in `ipc-win/src/sys.rs`;
- required output and forbidden content markers;
- no production default data root or auto-start behavior.

Test:

- agent health snapshot tests;
- process-memory tests;
- admission app unit/integration tests.

Smoke:

- build default agent Release;
- build admission probe Release;
- run parent mode;
- validate output metrics/ceilings/leak rules;
- invalid arguments fail closed.

Integrate:

- `build.ps1 -Task AgentIpcAdmission`;
- `Full` after the dedicated gate is green;
- Windows Rust CI path and step;
- dependency/source policy exact boundaries.

## 14. Acceptance criteria

Phase 3F is complete only when:

- default agent dependency graph remains Protobuf/IPC-transport-free;
- the real agent health snapshot is the single Health truth used by CLI and server probe;
- a distinct server child owns storage and serves one authenticated Health response;
- baseline/server process memory and default/probe binary sizes are measured;
- all initial ceilings pass or resident adoption is explicitly blocked;
- no automated path reads/writes clipboard;
- `All`, agent, IPC transport, native manager build/UIA, dependency/source policy, Git integrity, and tracked-output policy remain green;
- evidence states that manager and resident agent are still unlinked.

## 15. Rollback

If Phase 3F fails:

- keep `AgentHealthSnapshot` only if it improves health-check consistency and remains dependency-neutral;
- remove the admission package and process-memory diagnostics;
- retain Phase 3E transport foundation;
- do not add transport dependencies to the default agent;
- record the failed footprint or lifecycle gate and evaluate a smaller serializer/runtime or a different final linkage strategy.

## 16. Next slice after admission

If Phase 3F passes, Phase 3G may implement official C++ schema/runtime parity and a bounded WinUI manager Health client. The manager Release provider may move from `Disconnected` to live Health state only after cross-language framing, schema, DPAPI material access, token/session validation, HMAC transcript parity, timeout/reconnect behavior, and UI-thread isolation all pass.
