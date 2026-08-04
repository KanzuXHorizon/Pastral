# Pastral Authenticated Named-Pipe Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify a real cross-process, same-logon-session, explicit-DACL Windows named-pipe transport with kernel/token peer validation, DPAPI installation secret, mutual HMAC transcript authentication, bounded overlapped I/O, and one authenticated health round trip.

**Architecture:** `pastral-ipc-auth` owns pure Rust transcript/HMAC/replay logic. `pastral-ipc-win` owns all Win32 token/SID/DPAPI/RNG/security-descriptor/named-pipe/overlapped I/O behavior. `pastral-ipc-transport-probe` is a separate-process diagnostic harness. Existing agent and manager remain unconnected.

**Tech Stack:** Rust 1.97.1/Edition 2024, `hmac 0.12.1`, `sha2 0.10.9`, `zeroize 1.8.2`, `uuid 1.18.1`, `windows-sys 0.61.2`, official Protobuf `4.35.0-release`, `protoc 35.0`, PowerShell 5.1, Windows 11 x64.

## Global constraints

- Do not add transport/schema dependencies to `pastral-agent` or the WinUI manager.
- No Tokio, async runtime, IOCP framework, network, registry, JSON, gRPC, logging backend, service, elevation, auto-start, or clipboard/storage production access.
- Named-pipe APIs exist only in `pastral-ipc-win`; process spawning exists only in `pastral-ipc-transport-probe`.
- Pipe is byte-mode, overlapped, local-only, first-instance, one-instance, explicit DACL.
- DACL normal client ACE is the current enabled logon SID only; no broad user/Everyone/AuthUsers/Interactive/Network/SYSTEM ACE.
- Client uses `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION`.
- Peer PID/session/user/logon SID/integrity comes from kernel/process tokens, never schema fields.
- DPAPI is user scope with `CRYPTPROTECT_UI_FORBIDDEN`; no machine scope.
- Secret/proof-bearing types never implement `Debug` or `Display` and are zeroized on drop.
- All connect/read/write operations have deadlines and exact cancellation/drain behavior.
- Existing 157 Rust tests, IPC framing probe, agent smoke, native manager builds/UIA remain green.

---

### Task 1: Evolve handshake schema and implement pure auth core

**Files:**
- Modify: `Cargo.toml`
- Modify: `protocols/ipc-schema/pastral_ipc_v1.proto`
- Modify: `crates/ipc-core/src/dto.rs`
- Modify: `crates/ipc-core/src/lib.rs`
- Modify: `crates/ipc-schema/src/convert.rs`
- Modify: `crates/ipc-schema/src/lib.rs`
- Modify tests: `crates/ipc-core/tests/dto.rs`
- Modify tests: `crates/ipc-schema/tests/roundtrip.rs`
- Modify tests: `crates/ipc-schema/tests/validation.rs`
- Create: `crates/ipc-auth/Cargo.toml`
- Create: `crates/ipc-auth/src/lib.rs`
- Create: `crates/ipc-auth/src/error.rs`
- Create: `crates/ipc-auth/src/material.rs`
- Create: `crates/ipc-auth/src/transcript.rs`
- Create: `crates/ipc-auth/src/replay.rs`
- Test: `crates/ipc-auth/tests/authentication.rs`
- Test: `crates/ipc-auth/tests/replay.rs`

**Schema changes:**

```proto
message ClientHello {
  ...
  bytes authentication_proof = 7;
}

message ServerAccepted {
  uint32 selected_minor = 1;
  repeated Capability accepted_capabilities = 2;
  bytes authentication_proof = 3;
}
```

**Core DTO changes:**

- `ClientHelloDto` gains exact 32-byte authentication proof.
- Add `ServerAcceptedDto` with selected minor, unique/sorted accepted capabilities, exact 32-byte proof.
- Add encode/decode functions for `ServerAccepted`.
- Proof bytes remain owned private fields; no content-bearing `Debug`.

**Auth interfaces:**

```rust
pub struct InstallationSecret(Zeroizing<[u8; 32]>);
pub struct Nonce([u8; 32]);
pub struct AuthenticationProof(Zeroizing<[u8; 32]>);
pub struct PeerTranscriptIdentity {
    process_id: u32,
    session_id: u32,
    integrity_rid: u32,
}
pub struct HandshakeTranscript { ... }
pub enum ProofRole { Client, Server }

pub fn compute_proof(
    secret: &InstallationSecret,
    transcript: &HandshakeTranscript,
    role: ProofRole,
) -> AuthenticationProof;

pub fn verify_proof(...) -> Result<(), AuthError>;
```

Transcript fields exactly match the design and serialize to a fixed canonical byte vector with checked capacities.

- [ ] Write schema/DTO/auth tests before implementation.
- [ ] RED: missing fields/types/functions.
- [ ] Implement exact pins `hmac = 0.12.1`, `zeroize = 1.8.2`.
- [ ] Add deterministic HMAC vector and per-field tamper matrix.
- [ ] Implement `NonceReplayCache` maximum 1024, hashed entries, duplicate rejection, FIFO eviction.
- [ ] Run formatter, all ipc-core/schema/auth tests, Clippy, dependency tree.
- [ ] Commit: `feat: add IPC mutual authentication core`.

---

### Task 2: Windows CSPRNG, DPAPI, strict identity and secret files

**Files:**
- Modify: `Cargo.toml` Windows feature list
- Create: `crates/ipc-win/Cargo.toml`
- Create: `crates/ipc-win/src/lib.rs`
- Create: `crates/ipc-win/src/error.rs`
- Create: `crates/ipc-win/src/sys.rs`
- Create: `crates/ipc-win/src/random.rs`
- Create: `crates/ipc-win/src/dpapi.rs`
- Create: `crates/ipc-win/src/config.rs`
- Test: `crates/ipc-win/tests/config.rs`
- Test: `crates/ipc-win/tests/dpapi.rs`

**Windows features:**

- `Win32_Security`
- `Win32_Security_Authorization`
- `Win32_Security_Cryptography`
- `Win32_Storage_FileSystem`
- existing Foundation/Memory/Threading

**Config interfaces:**

```rust
pub struct TransportIdentity { instance_id: CorrelationId, secret_version: u32 }
pub struct TransportMaterial {
    identity: TransportIdentity,
    secret: InstallationSecret,
}

pub fn load_or_create_transport_material(root: &Path) -> Result<TransportMaterial, TransportError>;
pub fn derive_pipe_name(identity: &TransportIdentity, session_id: u32) -> Result<PipeName, ...>;
```

**DPAPI envelope:**

```text
magic PSE1 (4)
version u16 LE = 1
reserved u16 = 0
ciphertext length u32 LE
ciphertext
```

Maximum total 16 KiB.

- [ ] Write RED tests for strict identity, malformed files, concurrent publication, canonical pipe name, DPAPI exact round trip, wrong entropy, truncated/oversized envelope, RNG nonzero/nonrepeat.
- [ ] Implement `BCryptGenRandom` and DPAPI wrappers in reviewed unsafe `sys.rs` only.
- [ ] Ensure DPAPI/local buffers are copied then freed and plaintext uses `InstallationSecret` zeroization.
- [ ] Implement atomic create-new staging/rename and fail-closed existing-file behavior.
- [ ] Run focused tests, Clippy, source policy.
- [ ] Commit: `feat: add DPAPI IPC transport identity`.

---

### Task 3: Current/process token identity and peer comparison

**Files:**
- Create: `crates/ipc-win/src/token.rs`
- Modify: `crates/ipc-win/src/sys.rs`
- Modify: `crates/ipc-win/src/lib.rs`
- Test: `crates/ipc-win/tests/token.rs`

**Interfaces:**

```rust
pub struct SidBytes(Vec<u8>);
pub struct TokenIdentity {
    user_sid: SidBytes,
    logon_sid: SidBytes,
    session_id: u32,
    integrity_rid: u32,
    process_id: u32,
}
pub enum PeerMismatch { UserSid, LogonSid, Session, Integrity, ProcessId }

pub fn current_token_identity() -> Result<TokenIdentity, TransportError>;
pub fn process_token_identity(process_id: u32) -> Result<TokenIdentity, TransportError>;
pub fn validate_peer(
    expected_current: &TokenIdentity,
    kernel_process_id: u32,
    kernel_session_id: u32,
    observed: &TokenIdentity,
) -> Result<ValidatedPeer, PeerMismatch>;
```

- [ ] RED tests for current identity shape, one enabled logon SID, pure mismatch matrix, invalid PID/exit race.
- [ ] Implement two-call bounded `GetTokenInformation` with maximum 64 KiB.
- [ ] Copy valid SIDs; no borrowed token pointers escape.
- [ ] Require exactly one enabled `SE_GROUP_LOGON_ID` group.
- [ ] Query TokenUser, TokenGroups, TokenSessionId, TokenIntegrityLevel.
- [ ] Close process/token handles on every path.
- [ ] Run focused tests/Clippy.
- [ ] Commit: `feat: validate Windows IPC peer identity`.

---

### Task 4: Explicit logon-SID DACL and first-instance named-pipe creation

**Files:**
- Create: `crates/ipc-win/src/security.rs`
- Create: `crates/ipc-win/src/pipe.rs`
- Modify: `crates/ipc-win/src/sys.rs`
- Modify: `crates/ipc-win/src/lib.rs`
- Test: `crates/ipc-win/tests/security.rs`
- Test: `crates/ipc-win/tests/pipe_creation.rs`

**Interfaces:**

```rust
pub struct PipeSecurity { ... }
pub struct PipeServer { ... }
pub struct PipeClient { ... }

pub fn build_logon_sid_pipe_security(identity: &TokenIdentity) -> Result<PipeSecurity, ...>;
pub fn inspect_pipe_security(...) -> Result<SecurityInspection, ...>;
pub fn create_first_pipe_server(name: &PipeName, security: &PipeSecurity) -> Result<PipeServer, ...>;
pub fn open_pipe_client(name: &PipeName, deadline: Instant) -> Result<PipeClient, ...>;
```

- [ ] RED tests for SDDL/security inspection, forbidden broad principals, first-instance collision, local byte-mode client/server open, endpoint PID/session queries.
- [ ] Build protected DACL with only logon SID normal ACE.
- [ ] Server uses exact required flags, buffers, one instance, explicit `SECURITY_ATTRIBUTES`.
- [ ] Client uses overlapped + identification-only SQOS.
- [ ] Verify pipe local byte mode and peer endpoint process/session IDs.
- [ ] Commit: `feat: add secure local named-pipe endpoints`.

---

### Task 5: Overlapped deadlines, cancellation, and frame stream

**Files:**
- Create: `crates/ipc-win/src/overlapped.rs`
- Create: `crates/ipc-win/src/stream.rs`
- Modify: `crates/ipc-win/src/pipe.rs`
- Modify: `crates/ipc-win/src/sys.rs`
- Modify: `crates/ipc-win/src/lib.rs`
- Test: `crates/ipc-win/tests/stream.rs`
- Test: `crates/ipc-win/tests/deadlines.rs`

**Interfaces:**

```rust
pub struct PipeFrameStream<E> { endpoint: E, decoder: FrameDecoder, pending: VecDeque<Frame> }
pub fn connect(&mut PipeServer, deadline: Instant) -> Result<(), ...>;
pub fn write_frame(&mut self, frame: &Frame, deadline: Instant) -> Result<(), ...>;
pub fn read_frame(&mut self, deadline: Instant) -> Result<Frame, ...>;
```

- [ ] RED tests for immediate/pending connect, no-client timeout, silent-client read timeout, cancellation, fractured/coalesced frames, short writes/read zero/disconnect.
- [ ] Implement manual-reset event + one OVERLAPPED per operation.
- [ ] Use `GetOverlappedResultEx`; timeout calls `CancelIoEx` and drains completion.
- [ ] Frame read buffer fixed 64 KiB and feeds Phase 3D decoder.
- [ ] Frame write loops to completion and does not trust message boundaries.
- [ ] Run focused tests/Clippy.
- [ ] Commit: `feat: add bounded overlapped pipe stream`.

---

### Task 6: Mutual authenticated handshake transport

**Files:**
- Create: `crates/ipc-win/src/handshake.rs`
- Modify: `crates/ipc-win/src/lib.rs`
- Modify: `crates/ipc-schema/src/convert.rs` as required by actual generated API
- Test: `crates/ipc-win/tests/handshake.rs`

**Interfaces:**

```rust
pub struct AuthenticatedServerConnection { stream, peer, negotiated }
pub struct AuthenticatedClientConnection { stream, peer, negotiated }

pub fn server_handshake(..., replay_cache: &mut NonceReplayCache, deadline: Instant) -> Result<...>;
pub fn client_handshake(..., deadline: Instant) -> Result<...>;
```

- [ ] RED tests using two endpoints for successful handshake and wrong secret/proof/transcript/replay/control-before-auth failures.
- [ ] Validate peer before schema parse.
- [ ] Generate CSPRNG nonces.
- [ ] Send/validate ServerHello, ClientHello proof, ServerAccepted proof.
- [ ] Only Health capability accepted.
- [ ] Bind exact peer PIDs/session/integrity/schema digest/version/capabilities.
- [ ] No authenticated connection type can be constructed publicly except successful handshake.
- [ ] Commit: `feat: add mutual authenticated IPC handshake`.

---

### Task 7: Cross-process transport probe

**Files:**
- Modify: `Cargo.toml`
- Create: `apps/ipc-transport-probe/Cargo.toml`
- Create: `apps/ipc-transport-probe/src/main.rs`
- Create: `apps/ipc-transport-probe/tests/cross_process.rs`

**Modes:**

```text
pastral-ipc-transport-probe              # parent smoke
pastral-ipc-transport-probe --server-child --root <path> --max-connections 1
```

Hidden negative modes are permitted only when strictly parsed and test-only behavior remains content-free.

- [ ] Parent creates disposable root and material.
- [ ] Spawn server child process.
- [ ] Parent client validates server, handshakes, sends HealthRequest, validates synthetic HealthResponse.
- [ ] Verify child PID differs from parent PID and kernel reports both correctly.
- [ ] Test wrong-secret root, tampered proof, first-instance collision, timeout/cleanup.
- [ ] Output only counts/PIDs/session/timing/binary size/result classes; no SID/pipe/root/secret/nonce/proof/content.
- [ ] Build Debug/Release and measure connect/handshake/health/total.
- [ ] Commit: `test: verify authenticated IPC transport`.

---

### Task 8: Verification, dependency/source/CI policy, docs, evidence

**Files:**
- Create: `eng/verify-ipc-transport.ps1`
- Modify: `eng/build.ps1`
- Modify: `eng/verify-dependencies.ps1`
- Modify: `eng/verify-source-policy.ps1`
- Modify: `.github/workflows/rust-ci.yml`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Modify: `docs/architecture/ipc-security-model.md` if implementation evidence refines it
- Modify: `docs/adr/0008-ipc-protocol.md`
- Create: `docs/reviews/phase-3e-authenticated-ipc-transport-verification.md`

**Verifier modes:** `Static`, `Test`, `Smoke`, `All`.

Static checks:

- exact dependencies/versions;
- named-pipe APIs only in ipc-win;
- process spawning only in transport probe;
- required flags/constants/APIs;
- no null/default security attributes;
- no broad DACL principals;
- no machine-scope DPAPI;
- no PIPE_NOWAIT/message mode/network/registry/clipboard/storage/manager/agent linkage;
- no secret/SID/pipe/root/proof output markers.

- [ ] Integrate `build.ps1 -Task IpcTransport`; add to `Full` only after all transport gates pass.
- [ ] Update dependency/source policy with narrow reviewed exceptions.
- [ ] Update CI Windows 2025 paths and run transport smoke.
- [ ] Validate YAML locally.
- [ ] Run final gates:

```powershell
.\eng\build.ps1 -Task IpcTransport
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
cargo tree --locked --workspace
git diff --check
git fsck --no-progress --no-dangling
git status --short
```

- [ ] Write evidence with exact test counts, DACL inspection, peer PID/session evidence, negative cases, binary/latency metrics, toolchain, residual risks, and untested cross-user/remote hosted cases.
- [ ] Commit: `docs: record authenticated IPC transport evidence`.
- [ ] Hygiene commit only if `git show --check` requires it.
