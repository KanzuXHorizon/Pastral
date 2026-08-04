# Pastral Phase 3E — Authenticated per-logon-session named-pipe transport design

**Date:** 2026-08-04
**Status:** Approved for implementation by the user's standing instruction to continue the next evidence-gated slice
**Baseline:** `4414485`

## 1. Purpose

Phase 3E wraps the accepted Phase 3D Rust frame/schema prototype in a real local Windows transport with explicit operating-system isolation and mutual transcript authentication.

The slice must prove that two separate unelevated processes in the same Windows logon session can:

1. create and discover a unique local byte-mode named pipe without accepting a squatted or remote instance;
2. validate each other's kernel-reported process/token/user/logon-session/integrity evidence;
3. load the same per-installation DPAPI-protected secret;
4. complete a nonce- and identity-bound mutual HMAC handshake;
5. exchange one bounded health request/response through the 36-byte frame and Edition 2024 schema layers;
6. reject malformed, stale, wrong-secret, wrong-transcript, timeout, duplicate-instance, and unauthenticated traffic without exposing clipboard or storage content.

This is still a transport prototype. It does not connect the WinUI manager, expose history/search, enable auto-start, or turn the diagnostic agent into a production daemon.

## 2. Security statement

The transport targets a strong Windows boundary against:

- other Windows users;
- another logon session, including another session of the same account;
- remote named-pipe clients;
- accidental/stale/wrong-installation clients;
- frame/schema/protocol confusion and simple replay;
- unauthorized control frames before handshake.

It is **not** a secure enclave against fully compromised code already running as the same unlocked user/session. Such code may access user files, invoke user-scope DPAPI, inject into first-party processes, or invoke ordinary Pastral operations under the user's authority. The implementation and documentation must preserve this residual-risk statement.

## 3. Scope

Implement:

- a pure-Rust `pastral-ipc-auth` crate;
- canonical transcript construction and HMAC-SHA256 mutual proofs;
- bounded replay-cache support for active-process nonces;
- zeroized secret/proof material;
- a Windows-only `pastral-ipc-win` crate containing the reviewed unsafe boundary;
- current token identity extraction: user SID, enabled logon SID, session ID, integrity RID, process ID;
- exact peer validation using named-pipe-reported PID/session plus process token evidence;
- user-scope DPAPI installation-secret protection with UI forbidden;
- CSPRNG generation through `BCryptGenRandom`;
- atomic identity/secret file creation and strict parsing;
- unique per-installation/per-session pipe naming;
- explicit logon-SID-only pipe DACL;
- first-instance anti-squatting and remote-client rejection;
- byte-mode overlapped connect/read/write with bounded deadlines and cancellation;
- server/client frame stream adapters using `pastral-ipc-core`;
- schema evolution for client proof and server accepted proof;
- a cross-process `pastral-ipc-transport-probe` executable;
- static/test/smoke verifier and CI integration;
- measured binary/latency evidence.

Do not implement:

- manager integration or C++ client;
- agent auto-start or service registration;
- history/search/detail operations over the pipe;
- clipboard payload bulk transfer or staging files;
- paste/replay;
- elevated/service broker;
- package/publisher allowlisting;
- Windows Hello or interactive user-presence;
- encryption of ordinary history;
- remote/network transport;
- a claim that DPAPI/HMAC protects against same-session malware;
- production recovery/rotation across upgrades beyond strict versioned files.

## 4. Architecture

### 4.1 `pastral-ipc-auth`

Pure Rust, Windows-binding-free, no I/O.

Dependencies:

- `hmac = 0.12.1`, exact;
- `sha2 = 0.10.9`, existing exact workspace pin;
- `zeroize = 1.8.2`, exact with derive support disabled unless needed;
- `pastral-ipc-core`.

Owns:

- 32-byte `InstallationSecret`;
- 32-byte nonces/proofs;
- canonical handshake transcript;
- client/server HMAC proof generation and constant-time verification;
- negotiated protocol/capability transcript fields;
- bounded nonce replay cache;
- no `Debug` or plaintext display for secret/proof-bearing types.

### 4.2 `pastral-ipc-win`

Windows-only reviewed unsafe boundary.

Owns:

- current token identity;
- SID copying/string conversion;
- logon SID extraction from token groups using `SE_GROUP_LOGON_ID` and enabled state;
- integrity RID extraction;
- process token identity lookup;
- peer identity validation;
- DPAPI protect/unprotect;
- CSPRNG;
- security descriptor/DACL construction;
- pipe creation/open/connect/read/write/cancel/close;
- overlapped event lifetime;
- pipe endpoint PID/session lookup;
- strict identity/secret files and pipe name;
- frame stream adapter.

No other crate may call raw Win32 named-pipe, token, DPAPI, BCrypt, ACL, or impersonation APIs in this phase.

### 4.3 `pastral-ipc-transport-probe`

A diagnostic executable with these explicit modes:

- parent/default: create disposable root, start server child, run client, verify result, clean up;
- hidden `--server-child`: run one bounded server instance and exit;
- optional hidden negative-test modes used only by integration tests.

The parent and server are separate processes. The normal parent flow:

1. create/load identity and DPAPI secret;
2. derive a session-scoped pipe name;
3. spawn server child with disposable root and maximum one connection;
4. connect as client with bounded retry/deadline;
5. validate server peer identity;
6. complete mutual handshake;
7. send one health request;
8. receive one health response;
9. wait for clean child exit;
10. report content-free metrics.

The probe does not open clipboard, storage, manager, network, registry, or user production roots.

## 5. Dependencies and isolation

Workspace exact pins:

```toml
hmac = "=0.12.1"
zeroize = "=1.8.2"
```

Rules:

- `pastral-ipc-auth` remains Windows-binding-free;
- `pastral-ipc-win` may depend on `windows-sys 0.61.2`, ipc-core, ipc-auth, ipc-schema, uuid, and zeroize;
- official Protobuf is permitted in ipc-win/probe only because they exercise the accepted schema prototype;
- agent remains protobuf- and ipc-win-free in Phase 3E;
- no Tokio, async runtime, I/O completion port runtime, gRPC, HTTP, JSON, generic serialization framework, logging backend, network crate, or registry crate;
- all Windows APIs are statically linked through pinned `windows-sys` features.

## 6. Installation identity and pipe name

### 6.1 Identity file

`ipc-transport-identity.txt` under the caller-selected root:

```text
version=1
instance_id=<canonical lowercase UUIDv4>
secret_version=1
```

Rules:

- strict UTF-8;
- exactly three lines and no unknown/duplicate fields;
- UUIDv4 only;
- unsupported version fails closed;
- create-new staging file, flush, `sync_all`, rename;
- malformed existing file is never replaced silently;
- identity contains no username, SID, path, PID, session content, or secret.

### 6.2 Secret file

`ipc-installation-secret.dpapi`:

- versioned binary envelope;
- maximum 16 KiB total;
- magic, format version, DPAPI ciphertext length, ciphertext;
- plaintext is exactly 32 random bytes;
- new secret generated with `BCryptGenRandom(BCRYPT_USE_SYSTEM_PREFERRED_RNG)`;
- protected using user-scope `CryptProtectData` with `CRYPTPROTECT_UI_FORBIDDEN` and fixed purpose entropy;
- plaintext and temporary proof buffers zeroized where practical;
- DPAPI buffers released with `LocalFree`;
- malformed/truncated/oversized/wrong-context data fails closed and is not replaced;
- file creation is atomic and bounded;
- no machine-scope flag.

User-scope DPAPI is represented as account/offline/cross-user protection and installation binding, not same-user malware protection.

### 6.3 Pipe name

Derived from validated public identity and current session ID:

```text
\\.\pipe\Pastral-v1-s<decimal-session>-<lowercase-instance-uuid>
```

Rules:

- maximum 128 UTF-16 units;
- no username, SID, machine name, profile, clip ID, or secret;
- session ID is kernel/current-token evidence;
- instance UUID is public anti-collision identity, not a credential;
- caller-provided arbitrary pipe names are not accepted by production constructors;
- tests may use an injected UUID only through a test constructor.

## 7. Token identity

`TokenIdentity` contains owned bounded evidence:

- user SID bytes;
- enabled logon SID bytes;
- session ID;
- integrity RID;
- process ID.

Extraction:

1. open current/process token with `TOKEN_QUERY` only;
2. query exact required buffer lengths;
3. cap each token information allocation at 64 KiB;
4. copy SID bytes after `IsValidSid` and `GetLengthSid` checks;
5. locate exactly one enabled token group whose attributes contain `SE_GROUP_LOGON_ID`;
6. reject absent or multiple logon SID evidence;
7. query `TokenSessionId`;
8. query `TokenIntegrityLevel` and extract the final sub-authority RID;
9. close every token/process handle on all paths.

No token pointer or borrowed SID escapes the Win32 call scope.

## 8. Pipe security descriptor

The first server instance uses an explicit self-relative security descriptor built from the current enabled logon SID.

DACL requirements:

- protected DACL;
- one normal allow ACE for the current logon SID granting the minimum pipe read/write/synchronize rights required by the client;
- no broad current-user SID allow ACE;
- no Everyone, Anonymous, Authenticated Users, Interactive Users, Network, Builtin Users, or generic broad principal;
- SYSTEM is not granted in this phase;
- null/default DACL is forbidden.

Implementation may use SDDL plus `ConvertStringSecurityDescriptorToSecurityDescriptorW`, but runtime verification must convert/query the resulting descriptor and assert the logon SID-only policy before pipe creation.

Security descriptor buffers are released with `LocalFree`.

## 9. Pipe creation and connection

Server flags:

```text
PIPE_ACCESS_DUPLEX
FILE_FLAG_OVERLAPPED
FILE_FLAG_FIRST_PIPE_INSTANCE
PIPE_TYPE_BYTE
PIPE_READMODE_BYTE
PIPE_WAIT
PIPE_REJECT_REMOTE_CLIENTS
```

Limits:

- maximum server instances: 1 in prototype;
- input/output OS buffers: 64 KiB each;
- default connect deadline: 5 seconds;
- default frame read/write deadline: 2 seconds;
- maximum authenticated connection lifetime: 15 seconds in probe;
- maximum control frame body inherited from ipc-core: 256 KiB;
- no compatibility `PIPE_NOWAIT`.

Client open:

- `CreateFileW` against the derived local pipe name;
- bounded `WaitNamedPipeW` retry until deadline;
- read/write access only;
- `FILE_FLAG_OVERLAPPED`;
- `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION` to limit server impersonation;
- byte read mode verified with `SetNamedPipeHandleState`/query where applicable.

First-instance creation failure is a possible squatting/stale instance and is not automatically retried without a new identity.

## 10. Overlapped I/O

Every connect/read/write operation owns:

- one manual-reset event;
- zeroed `OVERLAPPED` storage;
- one pipe handle association;
- a bounded deadline.

Rules:

- immediate success and `ERROR_IO_PENDING` are both handled;
- `ERROR_PIPE_CONNECTED` is valid only for server connect race;
- `GetOverlappedResultEx` enforces the remaining deadline;
- timeout invokes `CancelIoEx` for the exact `OVERLAPPED` and drains completion before event/handle release;
- read zero means clean disconnect/end-of-stream;
- write loops until all bytes are written; short writes are not assumed impossible;
- frame reads are byte-stream reads and never rely on `WriteFile` boundaries;
- pipe/frame/decoder errors close the connection;
- every event, pipe, process, token, and local-allocation handle is released exactly once.

## 11. Peer validation

### 11.1 Server validates client

After pipe connection and before frame parse:

1. `GetNamedPipeClientProcessId`;
2. `GetNamedPipeClientSessionId`;
3. open client process with `PROCESS_QUERY_LIMITED_INFORMATION`;
4. open/query client token;
5. require exact user SID match with server current identity;
6. require exact enabled logon SID match;
7. require exact session ID match with both kernel-reported pipe session and server session;
8. require exact integrity RID match for this prototype;
9. bind validated client PID/session/token evidence to the connection transcript.

### 11.2 Client validates server

Equivalent checks using:

- `GetNamedPipeServerProcessId`;
- `GetNamedPipeServerSessionId`;
- opened server process token.

No PID/session/user value from a Protobuf message is trusted.

### 11.3 Impersonation

The prototype does not require impersonation because named-pipe PID/session plus process-token queries provide the selected evidence. `ImpersonateNamedPipeClient` is not introduced unless a concrete test proves process-token evidence is insufficient. Avoiding impersonation removes a high-risk `RevertToSelf` failure surface.

## 12. Mutual HMAC handshake

### 12.1 Schema evolution

Extend Edition 2024 schema:

```text
ClientHello.authentication_proof: bytes
ServerAccepted:
  selected_minor: uint32
  accepted_capabilities: repeated Capability
  authentication_proof: bytes
```

All proof fields are exactly 32 bytes and explicitly present.

### 12.2 Transcript

The HMAC input is a canonical project-owned binary transcript, not serialized Protobuf bytes.

Versioned fields in fixed order:

```text
context label and transcript version
schema SHA-256
protocol major
server min/max minor
client min/max minor
selected minor
server nonce
client nonce
server instance UUID
server process ID
client process ID
session ID
server integrity RID
client integrity RID
sorted requested capabilities
sorted accepted capabilities
proof role label: client or server
```

All integer fields are fixed-width little-endian. Capability lists are unique, sorted numeric IDs with bounded counts. No path, username, SID string, clipboard content, or source metadata enters the transcript.

### 12.3 Proofs

- client proof: HMAC-SHA256(secret, transcript with role `client`);
- server proof: HMAC-SHA256(secret, transcript with role `server`);
- verification uses the HMAC crate's constant-time verification;
- proof/secret types do not implement `Debug` or `Display`;
- temporary proof bytes are zeroized where practical;
- wrong secret, nonce, process ID, session, integrity, schema digest, version, capability set, instance ID, or role fails verification;
- control frames before successful mutual authentication are rejected;
- server nonce is CSPRNG-generated per connection;
- client nonce is CSPRNG-generated per connection.

## 13. Replay cache

`NonceReplayCache` is process-local and bounded:

- maximum 1024 entries;
- stores SHA-256 of `(server nonce || client nonce || client PID || session ID)` rather than raw secret/content;
- insertion rejects duplicates;
- FIFO eviction only after capacity is reached;
- cache does not persist across process restart;
- random server nonce remains the primary freshness property;
- cache is defense in depth against accidental nonce reuse/replay within the active process.

## 14. Handshake sequence

```text
server creates first-instance pipe with explicit DACL
client opens local pipe with identification-only SQOS
server and client validate kernel/token peer evidence
server -> HelloProto(ServerHello)
client validates hello and computes client proof
client -> HelloProto(ClientHello + client proof)
server validates transcript/client proof and replay cache
server -> HelloProto(ServerAccepted + server proof)
client validates selected version/capabilities/server proof
connection becomes authenticated
client -> ControlProto(HealthRequest)
server -> ControlProto(HealthResponse)
both close cleanly
```

Connection state must distinguish transport connected, peer validated, client proof verified, server proof verified, authenticated, and closed. `pastral-ipc-core::ServerConnection` remains the frame-order state; transport authentication is an additional gate.

## 15. Health operation

The probe health response contains only:

```text
storage_schema_version = 1
capture_enabled = false
privacy_policy_ok = true
storage_integrity_ok = true
```

These values are synthetic transport-probe constants and are labeled as such in the probe report. The probe does not open real storage or report agent status.

Only `Capability::Health` is accepted in Phase 3E. History/search capability requests are rejected as unsupported by the transport probe even though schema DTOs exist.

## 16. Cross-process probe

Parent flow metrics:

- server child process ID;
- client/parent process ID;
- session ID;
- authenticated connection count;
- health round-trip count;
- connect/handshake/health/total durations;
- server/client release executable size;
- no SID, pipe name, secret, nonce, proof, root path, source, or content printed.

Negative cases:

- second first-instance server creation fails;
- wrong DPAPI secret root fails client proof;
- bit-flipped client proof fails;
- bit-flipped server proof fails;
- stale/replayed client hello fails;
- control before authentication fails;
- connect timeout with no client;
- read timeout with silent connected client;
- malformed/truncated/oversized frame closes;
- invalid peer/session comparison fails in pure validation tests;
- malformed identity/secret files fail closed and are not replaced.

The normal smoke must use separate processes. Unit tests may use threads for low-level handle/deadline tests but cannot be the sole end-to-end evidence.

## 17. Tests

### `pastral-ipc-auth`

- RFC-compatible deterministic HMAC vector;
- client/server role separation;
- every transcript field changes the proof;
- wrong secret and bit flip rejection;
- duplicate/unsorted capability rejection;
- negotiated minor validation;
- zero/all-zero nonce rejection;
- replay cache duplicate and FIFO bound;
- secret/proof types are not debug-printable through public API.

### `pastral-ipc-win` pure/adapter tests

- identity create/reopen/malformed/concurrent publication;
- pipe-name canonicalization and length;
- DPAPI round trip exact 32 bytes;
- wrong entropy and malformed/oversized envelope rejection;
- CSPRNG nonzero/nonrepeat smoke;
- token identity current process fields;
- exactly one enabled logon SID;
- peer comparison user/logon/session/integrity mismatch;
- generated SDDL contains only expected logon SID principal;
- first-instance collision;
- local byte-mode connection;
- separate peer PID/session lookup;
- connect/read timeout and cancellation;
- short/fractured/coalesced frame stream;
- every handle/event closes on success/error.

### Cross-process probe tests

- normal authenticated health round trip;
- wrong-secret rejection;
- tampered proof rejection;
- first-instance collision;
- child timeout/exit cleanup;
- no clipboard/storage/network APIs or content output.

## 18. Build and verification

Add:

- `eng/verify-ipc-transport.ps1` with `Static`, `Test`, `Smoke`, `All`;
- `eng/build.ps1 -Task IpcTransport`;
- CI path coverage and smoke on Windows 2025;
- dependency policy for exact HMAC/zeroize and Windows binding isolation;
- source policy allowing named-pipe/token/DPAPI APIs only in `pastral-ipc-win` and process spawning only in `pastral-ipc-transport-probe`;
- verification report with environment, test, binary, latency, DACL, peer identity, and negative-case evidence.

Static verification requires exact constants/flags/API usage and rejects:

- default/null security attributes;
- broad SID strings/principals;
- machine-scope DPAPI;
- `PIPE_NOWAIT`;
- message-mode pipe assumptions;
- blocking unbounded read/write/connect;
- network APIs;
- logging/printing secret/SID/pipe/root/content material;
- named-pipe APIs outside the reviewed Windows crate;
- manager/agent dependency on transport in this phase.

## 19. Acceptance criteria

Phase 3E is complete when:

1. all auth, Windows adapter, and cross-process probe tests pass;
2. normal cross-process mutual HMAC health round trip passes;
3. server/client kernel PID and session IDs differ/validate as expected in the cross-process flow;
4. explicit DACL inspection proves logon-SID-only normal access and no broad user/everyone principal;
5. second first-instance creation is rejected;
6. wrong secret, tampered client/server proof, replay, unauthenticated control, malformed frame, and deadlines fail closed;
7. DPAPI exact round trip and malformed/wrong-context cases pass;
8. all prior 157 Rust tests remain green;
9. formatter, check, Clippy, docs, dependency/source policies pass;
10. IPC framing probe, agent health smoke, manager Debug/Release/UIA remain green;
11. no clipboard/storage production data is read or written;
12. no manager integration or auto-start is added;
13. working tree and Git integrity are clean after evidence commit.

## 20. Next slice

If Phase 3E passes, Phase 3F may integrate an explicit, non-auto-start agent health/snapshot server and a C++/WinUI client prototype using the same exact schema/golden fixtures. Live history/search remains read-only and privacy-safe; paste/replay remains a later separately authorized slice.
