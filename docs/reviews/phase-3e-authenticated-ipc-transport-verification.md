# Phase 3E Authenticated IPC Transport Verification

**Date:** 2026-08-04

**Baseline:** `4414485`

**Implementation head:** `d6c5ae0`

**Scope:** pure mutual-authentication core, strict DPAPI-backed transport material, kernel token/session peer identity, protected logon-SID-only named-pipe security, first-instance/local-only byte-mode endpoints, bounded overlapped stream, mutual authenticated handshake, cross-process Health probe, build/dependency/source/CI integration

## 1. Result

Phase 3E is **green for its declared authenticated Rust/Windows transport-foundation scope**.

The verified flow is:

```text
strict public transport identity
  + user-scope DPAPI installation secret
  + current logon SID/session evidence
  -> protected first-instance local byte pipe
  -> kernel-reported peer PID/session
  -> process-token user/logon SID/session/integrity validation
  -> bounded 36-byte framed stream
  -> ServerHello
  -> proof-bearing ClientHello
  -> replay check
  -> proof-bearing ServerAccepted
  -> authenticated Health request/response
```

The Release probe starts a distinct server child process and completes this flow through a real named pipe. It does not open clipboard, storage, manager, network, registry, or a production user data root.

This phase does **not** link the resident agent or C++ manager to the transport, enable auto-start, implement ordinary history/search over IPC, stage bulk payloads, or claim protection against fully compromised code already running as the same unlocked user.

ADR 0018 remains Proposed. Rust framing/schema plus Windows transport evidence now exists; C++ parity, fuzzing, adjacent-version fixtures, bulk staging cleanup, and resident-agent/manager linkage and memory attribution remain open.

## 2. Commits

```text
0c9eee0 docs: specify authenticated IPC transport
46ffc9e docs: plan authenticated IPC transport
f993e72 feat: add IPC mutual authentication core
592e214 feat: add DPAPI IPC transport identity
65cb242 feat: validate IPC peer token identity
5fa9091 feat: add secure local named-pipe endpoint
e1e2800 feat: add bounded overlapped pipe stream
7aec7d0 feat: add mutual authenticated IPC handshake
6819b51 test: verify authenticated IPC transport
d6c5ae0 ci: verify authenticated IPC transport
```

README, changelog, setup, ADR updates, and this report form the final documentation/evidence batch.

## 3. Package boundaries

### 3.1 `pastral-ipc-auth`

- Windows-binding-free;
- Protobuf-free;
- no I/O, process, pipe, clipboard, storage, manager, network, async runtime, or logging dependency;
- exact `hmac 0.12.1`, `sha2 0.10.9`, and `zeroize 1.8.2`;
- secret/proof types do not implement `Debug` or `Display`;
- secret/proof buffers zeroize on drop;
- canonical transcript owns no clipboard or user content.

### 3.2 `pastral-ipc-win`

- Windows-only reviewed boundary;
- all handwritten Win32 unsafe calls remain in `src/sys.rs`;
- exact `windows-sys 0.61.2`/`windows-link 0.2.1`;
- Protobuf arrives only through exact `pastral-ipc-schema` for handshake control messages;
- no network, registry, clipboard, storage, agent, manager, Tokio, gRPC, HTTP, JSON, or logging runtime.

### 3.3 `pastral-ipc-transport-probe`

- diagnostic executable only;
- parent/default and hidden strict `--server-child --root <path>` modes;
- process spawning is permitted only in its exact main/test source-policy boundaries;
- disposable temporary root;
- content-free output;
- no production identity/root discovery;
- one connection and one Health exchange.

The diagnostic `pastral-agent` and native manager remain independent of `pastral-ipc-win` and official Protobuf.

## 4. Authentication core

### 4.1 Material

- installation secret: exact 32 bytes;
- server nonce: exact 32 bytes, nonzero;
- client nonce: exact 32 bytes, nonzero;
- proof: HMAC-SHA256 exact 32 bytes;
- client and server use distinct role labels.

### 4.2 Canonical transcript

The HMAC transcript binds:

- context label and transcript version;
- current schema SHA-256;
- protocol major;
- server/client minor ranges;
- selected minor;
- server/client nonce;
- installation instance UUID;
- server/client PID;
- server/client session ID;
- server/client integrity RID;
- requested and accepted capability sets;
- proof role.

Capability sets are sorted, unique, nonempty, and accepted capabilities must be a subset of requested capabilities.

The deterministic client proof fixture is independently reproduced outside the Rust implementation and is not a self-referential expected value.

### 4.3 Replay cache

- maximum capacity: 1,024;
- active duplicate transcript key is rejected;
- key binds both nonces, client PID, and session;
- bounded FIFO eviction;
- cache stores SHA-256 replay keys rather than raw secret/proof material.

## 5. DPAPI transport material

### 5.1 Public identity

`ipc-transport-identity.txt` is strict UTF-8:

```text
version=1
instance_id=<canonical UUIDv4>
secret_version=1
```

Unknown, duplicated, missing, malformed, non-v4, or unsupported fields fail closed and are not silently replaced.

### 5.2 Secret envelope

`ipc-installation-secret.dpapi`:

- magic `PSE1`;
- envelope version `1`;
- reserved field must be zero;
- exact encoded ciphertext length;
- maximum total 16 KiB;
- user-scope `CryptProtectData`/`CryptUnprotectData`;
- `CRYPTPROTECT_UI_FORBIDDEN`;
- fixed Pastral purpose entropy;
- no machine-scope flag;
- plaintext must decrypt to exactly 32 bytes.

System random bytes use `BCryptGenRandom(BCRYPT_USE_SYSTEM_PREFERRED_RNG)`.

Native DPAPI plaintext is zeroed before `LocalFree`. The copied Rust plaintext is wrapped in zeroizing memory before any release path, including native-release failure.

### 5.3 Atomic publication regression

The initial `fs::rename` approach was rejected by a concurrent test because Windows may replace an existing destination. Publication now uses `MoveFileExW` with write-through and without replace-existing. Eight concurrent creators converge on one identity and one secret; losing writers load the published material.

Malformed existing identity or secret files remain unchanged.

## 6. Token and session identity

`TokenIdentity` owns:

- validated user SID bytes;
- validated enabled logon SID bytes;
- session ID;
- integrity RID;
- process ID.

Extraction rules:

- current/process token opened with minimum query access;
- process opened with `PROCESS_QUERY_LIMITED_INFORMATION`;
- two-call `GetTokenInformation` with maximum 64 KiB per buffer;
- aligned owned buffers;
- `IsValidSid` and `GetLengthSid` before SID copy;
- exactly one enabled `SE_GROUP_LOGON_ID` group required;
- `TokenUser`, `TokenGroups`, `TokenSessionId`, and `TokenIntegrityLevel` queried;
- process/token handles closed through RAII;
- no borrowed token/SID pointer escapes the Win32 call scope.

Peer validation requires exact user SID, logon SID, kernel session, token session, integrity RID, and process ID evidence.

## 7. Pipe security and creation

### 7.1 Name

```text
\\.\pipe\Pastral-v1-s<session>-<lowercase UUIDv4>
```

The name contains no username, SID, machine name, profile, clipboard data, secret, nonce, or proof.

### 7.2 Security descriptor

The descriptor is explicit and protected:

```text
D:P(A;;0xc0100000;;;<current enabled logon SID>)
```

`0xc0100000` is the selected generic read/write/synchronize client mask.

Native inspection verifies:

- DACL present;
- not defaulted;
- protected;
- exactly one ACE;
- exactly one allow ACE;
- exact current logon SID match;
- exact access mask.

No broad current-user, SYSTEM, Everyone, Anonymous, Authenticated Users, Interactive Users, Network, or Builtin Users ACE is added in this phase.

### 7.3 Server/client flags

Server:

- `PIPE_ACCESS_DUPLEX`;
- `FILE_FLAG_OVERLAPPED`;
- `FILE_FLAG_FIRST_PIPE_INSTANCE`;
- `PIPE_TYPE_BYTE`;
- `PIPE_READMODE_BYTE`;
- `PIPE_WAIT`;
- `PIPE_REJECT_REMOTE_CLIENTS`;
- one instance;
- 64 KiB input/output buffers;
- explicit `SECURITY_ATTRIBUTES`.

Client:

- local derived pipe name only;
- read/write access;
- `FILE_FLAG_OVERLAPPED`;
- `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION`;
- byte read mode.

A second first-instance server with the same name is rejected. The name can be reused after the original handle closes.

## 8. Bounded overlapped I/O

Each connect/read/write operation owns one manual-reset event and one zeroed `OVERLAPPED` structure.

- immediate success and `ERROR_IO_PENDING` are handled;
- server connect accepts the `ERROR_PIPE_CONNECTED` race;
- waits use `GetOverlappedResultEx` and remaining deadline milliseconds;
- timeout calls `CancelIoEx` for the exact `OVERLAPPED`;
- cancellation completion is drained before event, handle, or buffer release;
- short writes loop until completion;
- reads use a fixed 64 KiB buffer;
- zero/broken/closing pipe is returned as a distinct disconnect outcome;
- stream framing never relies on `WriteFile` boundaries;
- the Phase 3D incremental `FrameDecoder` handles fragmented/coalesced input.

Tests cover no-client connect timeout, nonexistent-pipe open timeout, silent-client read timeout, clean disconnect, frame exchange, PID/session evidence, and handle closeability after cancellation.

## 9. Mutual handshake

Current negotiated protocol is major `1`, minor `0`, Health capability only.

Server:

1. validates client kernel/token peer evidence;
2. sends ServerHello with server nonce, instance UUID, version, and Health capability;
3. requires a `HELLO_PROTO` ClientHello;
4. verifies echoed server nonce/version/capabilities;
5. builds transcript from local server and validated client identity;
6. verifies client HMAC proof;
7. records replay key;
8. sends correlated ServerAccepted with server proof;
9. constructs `AuthenticatedServerConnection` only after success.

Client:

1. validates server kernel/token peer evidence;
2. requires initial zero-correlation ServerHello;
3. verifies version/capability/instance UUID;
4. builds transcript from validated server and local client identity;
5. sends correlated ClientHello proof;
6. requires correlated ServerAccepted;
7. verifies selected capability and server proof;
8. constructs `AuthenticatedClientConnection` only after success.

Negative tests cover wrong installation secret, bit-flipped proof, field tampering, role mismatch, control-before-authentication, repeated transcript replay, version/capability invalidity, and peer evidence mismatch.

## 10. Cross-process Release smoke

Representative final run:

```text
ipc-transport-probe=ok
cross-process=true
client-pid=55480
server-pid=59152
session-id=1
connect-us=6543
handshake-us=356
health-us=61
total-us=99067
```

Assertions:

- parent and server child PIDs are nonzero and distinct;
- kernel-reported server PID equals spawned child PID;
- both peers are in the expected session;
- mutual handshake succeeds;
- one Health request/response round trips with one correlation UUID;
- invalid arguments exit `2` before transport creation;
- output contains no pipe name, root, SID, secret, nonce, proof, clipboard, query, or preview marker;
- disposable root is removed after completion.

Timing is machine-specific smoke evidence, not a release SLA.

## 11. Test evidence

### 11.1 Focused transport gate

| Package | Tests |
|---|---:|
| `pastral-ipc-auth` | 8 |
| `pastral-ipc-win` | 25 |
| `pastral-ipc-transport-probe` | 3 |
| **Focused total** | **36** |

### 11.2 Workspace aggregate

| Package/surface | Tests |
|---|---:|
| diagnostic agent | 14 |
| agent core | 18 |
| Win32 clipboard | 22 |
| domain | 26 |
| IPC authentication | 8 |
| IPC core | 30 |
| IPC schema probe | 3 |
| IPC schema | 11 |
| IPC transport probe | 3 |
| Windows IPC transport | 25 |
| storage | 33 |
| **Total** | **193** |

Fresh `All` evidence:

```text
cargo fmt --all -- --check: PASS
cargo check --locked --workspace --all-targets: PASS
cargo test --locked --workspace --all-targets: 193 PASS
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings: PASS
cargo doc --locked --workspace --no-deps: PASS
dependency policy: PASS
source policy: PASS
```

Additional fresh evidence:

```text
IpcPrototype static/test/10,000-round-trip Release probe: PASS
IpcTransport static/test/cross-process Release smoke: PASS
agent static/Debug/Release/health-check smoke: PASS
manager static/Debug/Release/UI Automation smoke: PASS
workflow YAML parse: PASS
```

Current IPC prototype measurement after authentication schema evolution:

```text
schema-sha256=2029ac9b19f7eb1644a2c12b3cd570586af9b62c40e130558b63c376676e3077
iterations=10000
round-trips=10000
executable-bytes=380416
average-roundtrip-ns=149472
one-byte-average-ns=866
coalesced-average-ns=682
max-body-capacity=7869
```

## 12. Policy and CI

- official Protobuf `4.35.0-release` is allowed only in schema/probes/Windows transport;
- agent, clipboard, domain, storage, agent-core, ipc-auth, and ipc-core remain Protobuf-free;
- Windows bindings remain exact `windows-sys 0.61.2`/`windows-link 0.2.1` in agent/clipboard/transport surfaces;
- Win32 unsafe code remains limited to reviewed `clipboard-win/src/sys.rs` and `ipc-win/src/sys.rs`;
- named-pipe APIs are allowed only in `ipc-win/src/sys.rs`;
- process spawning is allowed only in exact transport-probe main/integration-test paths;
- CI installs exact `protoc 35.0` from the official release asset and verifies SHA-256 before locked gates;
- CI now executes `verify-ipc-transport.ps1 -Mode All` on `windows-2025`.

The workflow parsed locally with PyYAML. Hosted GitHub Actions execution remains unproven until the remote workflow runs.

## 13. Security interpretation

The transport provides strong operating-system separation for:

- other Windows users;
- another logon session, including another session of the same account;
- remote named-pipe clients;
- stale/wrong installation material;
- replay/confusion using repeated transcript evidence;
- protocol/control use before authentication.

It does not provide a secure enclave against fully compromised code already running as the same unlocked user. Such code may share file/DPAPI authority or invoke the product as the user. Sensitive/private reveal, paste, export, and destructive operations still require separately implemented explicit user-intent and protection-domain policy.

## 14. Remaining gates

Phase 3E does not claim project or IPC release completion. Remaining IPC/product work includes:

- link transport into a bounded resident-agent control server;
- implement C++ Protobuf/schema client parity and manager transport client;
- live manager Health/history/search/reconnect/clear-on-disconnect states;
- parser/schema/transport fuzzing;
- adjacent-version Rust/C++ golden fixtures;
- bulk staging, quota, low-disk, cancellation, disconnect, and cleanup;
- authenticated ordinary history/search and operation authorization;
- sensitive/private reveal and user-presence flows;
- resident incremental binary/private-working-set attribution;
- cross-user, different-logon-session, remote-client, RDP, fast-user-switching, and packaged-identity tests;
- publisher/package identity policy where privileged capabilities require it;
- agent supervision, graceful shutdown, auto-start, packaging, signing, installer, and update integration;
- COM/OLE capture/replay, paste engine, Quick Paste, overlay, encryption, and compatibility matrix.

## 15. Next slice

The next dependency-ordered slice is **Phase 3F: resident agent Health server and live manager connection state**:

1. add an explicit agent IPC-server command with bounded one-client Health-only transport;
2. keep capture/listen/auto-start behavior unchanged;
3. build a C++ client parity prototype for ServerHello/ClientHello/ServerAccepted/Health;
4. replace the manager Release disconnected provider only after authenticated C++ cross-language Health passes;
5. preserve no-direct-SQLite/no-blob-access manager architecture.
