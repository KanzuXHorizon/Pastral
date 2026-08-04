# Pastral IPC Framing and Schema Prototype Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and measure a bounded pure-Rust 36-byte IPC frame/state foundation plus an isolated exact-version official Protocol Buffers Edition 2024 Rust schema prototype.

**Architecture:** `pastral-ipc-core` owns serializer-neutral frame parsing, incremental byte-stream decoding, connection/bulk state, limits, and validated DTOs. `pastral-ipc-schema` owns exact official generated Protobuf DTOs and conversion only. `pastral-ipc-probe` measures deterministic release round trips without opening clipboard, storage, pipe, network, or manager.

**Tech Stack:** Rust 1.97.1/Edition 2024, `uuid 1.18.1`, official `protobuf 4.35.0-release`, official `protobuf-codegen 4.35.0-release`, PowerShell 5.1 verification, GitHub Actions Windows 2025.

## Global Constraints

- Frame header is exactly 36 bytes and decoded field-by-field in little-endian order.
- `pastral-ipc-core` uses `#![forbid(unsafe_code)]`, no Windows binding, no serializer, no async runtime, and no product I/O.
- Control/hello/error bodies are at most 256 KiB; bulk chunks are at most 1 MiB.
- No body allocation occurs before a complete structurally valid header.
- Decoder retains at most one in-progress frame and emits at most 64 frames per `push` by default.
- Official Protobuf runtime and codegen are exact `4.35.0-release` and isolated to schema/probe dependency trees.
- `.proto` authority uses Edition 2024 under `protocols/ipc-schema/`.
- No Tokio, gRPC, HTTP, JSON, reflection, TextFormat, services, `Any`, logging backend, network, clipboard, storage mutation, named pipe, manager integration, or auto-start.
- Existing 113 Rust tests and native manager gates must remain green.

---

### Task 1: Pure frame model and exact header codec

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/ipc-core/Cargo.toml`
- Create: `crates/ipc-core/src/lib.rs`
- Create: `crates/ipc-core/src/error.rs`
- Create: `crates/ipc-core/src/limits.rs`
- Create: `crates/ipc-core/src/frame.rs`
- Test: `crates/ipc-core/tests/frame.rs`

**Interfaces:**
- Produces `pub const FRAME_HEADER_BYTES: usize = 36`.
- Produces `FrameKind::{HelloProto, ControlProto, BulkChunk, BulkEndProto, ProtocolErrorProto}` with numeric values 1–5.
- Produces `CorrelationId::initial_server_hello()`, `CorrelationId::new_v4()`, `CorrelationId::from_bytes([u8; 16])`, `is_zero()`, and `as_bytes()`.
- Produces `FrameHeader::new(kind, body_length, sequence, correlation, limits) -> Result<Self, IpcError>`.
- Produces `FrameHeader::encode() -> [u8; 36]` and `FrameHeader::decode(&[u8; 36], limits) -> Result<Self, IpcError>`.
- Produces `FrameLimits::default()` with 256 KiB control, 1 MiB bulk, 64 frames/push, 16 in-flight requests.

- [ ] **Step 1: Add workspace member and failing frame tests**

Create tests that assert:

```rust
assert_eq!(FRAME_HEADER_BYTES, 36);
let correlation = CorrelationId::new_v4();
let header = FrameHeader::new(
    FrameKind::ControlProto,
    17,
    0,
    correlation,
    FrameLimits::default(),
).unwrap();
assert_eq!(FrameHeader::decode(&header.encode(), FrameLimits::default()).unwrap(), header);
```

Also test invalid magic, major, kind, flags, control max+1, bulk zero/max+1, nonzero control sequence, zero non-server correlation, and non-v4/non-RFC4122 correlation bytes.

- [ ] **Step 2: Run RED**

Run:

```powershell
cargo test --locked -p pastral-ipc-core --test frame -- --nocapture
```

Expected: compile failure because `pastral-ipc-core` public API does not exist.

- [ ] **Step 3: Implement frame model and codec**

Implement exact offsets and checked conversions. Never cast input to a struct. `FrameHeader::decode` copies slices into fixed arrays and uses `from_le_bytes`.

`IpcError` initial variants:

```rust
pub enum IpcError {
    InvalidLimit(&'static str),
    InvalidMagic,
    UnsupportedFramingMajor(u16),
    UnknownFrameKind(u16),
    ReservedFlags(u32),
    BodyLengthExceeded { kind: FrameKind, length: u32, limit: u32 },
    EmptyBulkChunk,
    InvalidFrameSequence,
    InvalidCorrelation,
    DecoderPoisoned,
    TruncatedFrame,
    TooManyFrames,
    InvalidConnectionState,
    DuplicateCorrelation,
    InFlightLimitExceeded,
    BulkNotAuthorized,
    BulkSequenceMismatch { expected: u32, actual: u32 },
    BulkLengthExceeded,
    BulkChunkLimitExceeded,
    BulkEndMismatch,
    IntegerOverflow,
    InvalidDto(&'static str),
    Schema(String),
}
```

- [ ] **Step 4: Run GREEN and quality checks**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-ipc-core --test frame -- --nocapture
cargo clippy --locked -p pastral-ipc-core --all-targets -- -D warnings
```

Expected: all pass.

- [ ] **Step 5: Commit**

```powershell
git add Cargo.toml Cargo.lock crates/ipc-core
git commit -m "feat: add bounded IPC frame codec"
```

---

### Task 2: Incremental byte-stream decoder

**Files:**
- Create: `crates/ipc-core/src/decoder.rs`
- Modify: `crates/ipc-core/src/lib.rs`
- Test: `crates/ipc-core/tests/decoder.rs`

**Interfaces:**
- Produces `Frame { header: FrameHeader, body: Vec<u8> }` with bounded getters.
- Produces `FrameDecoder::new(FrameLimits) -> Self`.
- Produces `FrameDecoder::push(&mut self, input: &[u8]) -> Result<Vec<Frame>, IpcError>`.
- Produces `FrameDecoder::finish(self) -> Result<(), IpcError>`.
- Produces diagnostic getters `allocated_body_capacity()` and `is_poisoned()` for tests/probe.

- [ ] **Step 1: Write failing decoder tests**

Create deterministic helper `encoded_frame(kind, body, sequence, correlation)` and tests:

```rust
for split in 0..=FRAME_HEADER_BYTES {
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    assert!(decoder.push(&bytes[..split]).unwrap().is_empty());
    assert_eq!(decoder.push(&bytes[split..]).unwrap(), vec![expected.clone()]);
    decoder.finish().unwrap();
}
```

Add each body split, one-byte feeds, three coalesced frames, complete plus partial next frame, max frames+1, truncated finish, invalid-header poison, push-after-poison, and assertion that body capacity remains zero until all 36 valid header bytes arrive.

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked -p pastral-ipc-core --test decoder -- --nocapture
```

Expected: compile failure for missing decoder types.

- [ ] **Step 3: Implement bounded state machine**

Use:

```rust
enum DecoderState {
    Header { bytes: [u8; FRAME_HEADER_BYTES], filled: usize },
    Body { header: FrameHeader, bytes: Vec<u8>, filled: usize },
    Poisoned,
}
```

Copy only the needed portion from each input slice. Allocate body with exact validated length. Emit empty-body frames immediately. Poison on any protocol error. Reject more than `max_frames_per_push` frames.

- [ ] **Step 4: Run GREEN and quality checks**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-ipc-core --test decoder -- --nocapture
cargo clippy --locked -p pastral-ipc-core --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```powershell
git add crates/ipc-core
git commit -m "feat: add incremental IPC frame decoder"
```

---

### Task 3: Connection and bulk-transfer protocol state

**Files:**
- Create: `crates/ipc-core/src/connection.rs`
- Modify: `crates/ipc-core/src/lib.rs`
- Test: `crates/ipc-core/tests/connection.rs`

**Interfaces:**
- Produces `ConnectionPhase::{AwaitClientHello, Ready, BulkReceiving, Closed}`.
- Produces `ServerConnection::new(FrameLimits) -> Self`.
- Produces `accept(&mut self, &Frame) -> Result<AcceptedFrame, IpcError>`.
- Produces `complete_request(correlation)`, `cancel_request(correlation)`.
- Produces `authorize_bulk(transfer_id, max_bytes: u64, max_chunks: u32)`.
- Produces `AcceptedFrame::{ClientHello, Control, BulkChunk { sequence, bytes }, BulkEnd}`.
- Produces `phase()`, `in_flight_count()`, `bulk_progress()` diagnostics.

- [ ] **Step 1: Write failing state tests**

Cover:

- control before hello closes;
- valid client hello transitions to ready;
- second hello rejects;
- duplicate correlation rejects;
- 16 requests accepted, 17th rejected;
- completion frees one slot;
- bulk without authorization rejects;
- authorized chunks 0,1 and matching end succeed;
- duplicate/gap/reorder, max chunk count, checked byte excess, wrong transfer UUID, and wrong end sequence/length close state.

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked -p pastral-ipc-core --test connection -- --nocapture
```

Expected: compile failure for missing connection types.

- [ ] **Step 3: Implement connection state**

Use a bounded `BTreeSet<CorrelationId>` for in-flight requests. Store at most one `BulkState`:

```rust
struct BulkState {
    transfer_id: CorrelationId,
    next_sequence: u32,
    accepted_chunks: u32,
    accepted_bytes: u64,
    max_chunks: u32,
    max_bytes: u64,
}
```

Every protocol error calls a private `close_with(error)` that sets phase `Closed` before returning.

- [ ] **Step 4: Run GREEN and core suite**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-ipc-core --all-targets -- --nocapture
cargo clippy --locked -p pastral-ipc-core --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```powershell
git add crates/ipc-core
git commit -m "feat: add IPC connection state machine"
```

---

### Task 4: Serializer-neutral validated control DTOs

**Files:**
- Create: `crates/ipc-core/src/dto.rs`
- Modify: `crates/ipc-core/src/lib.rs`
- Test: `crates/ipc-core/tests/dto.rs`

**Interfaces:**
- Produces `Capability::{Health, HistoryPage, Search}`.
- Produces validated `ServerHelloDto`, `ClientHelloDto`, `HealthRequestDto`, `HealthResponseDto`, `HistoryPageRequestDto`, `SearchRequestDto`, `ClipPreviewDto`, `ProtocolErrorDto`, and request/response enums.
- Constructors validate every byte/count/string/time/order bound and expose read-only getters.
- Produces constants for nonce, query, terms, preview, source label, error detail, page, and result limits.

- [ ] **Step 1: Write failing DTO tests**

Test exact valid boundaries and each max+1/zero/invalid case. Include UUIDv4, nonce 32-byte length, page 1–100, query 1–1024 bytes and at most 32 whitespace terms, preview 4096 bytes, source label 256, error detail 512, nonzero capture order, supported UTC microsecond range, unique capabilities, and maximum 100 previews.

- [ ] **Step 2: Run RED**

```powershell
cargo test --locked -p pastral-ipc-core --test dto -- --nocapture
```

- [ ] **Step 3: Implement minimal validated DTOs**

Use private fields and constructors. Do not derive `Debug` for types containing preview/search text; implement only `Clone`, `PartialEq`, and `Eq` where needed.

- [ ] **Step 4: Run GREEN**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-ipc-core --all-targets
cargo clippy --locked -p pastral-ipc-core --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```powershell
git add crates/ipc-core
git commit -m "feat: add validated IPC control DTOs"
```

---

### Task 5: Edition 2024 schema and exact official Rust runtime

**Files:**
- Modify: `Cargo.toml`
- Create: `protocols/ipc-schema/pastral_ipc_v1.proto`
- Create: `crates/ipc-schema/Cargo.toml`
- Create: `crates/ipc-schema/build.rs`
- Create: `crates/ipc-schema/src/lib.rs`
- Create: `crates/ipc-schema/src/convert.rs`
- Test: `crates/ipc-schema/tests/roundtrip.rs`
- Test: `crates/ipc-schema/tests/validation.rs`

**Interfaces:**
- Generated entry module is `pastral_ipc_v1` included from Cargo `OUT_DIR` generated entry point.
- Produces `encode_request`, `decode_request`, `encode_response`, `decode_response`, `encode_server_hello`, `decode_client_hello`, `encode_protocol_error`, and `decode_bulk_end` functions.
- Each decode accepts `&[u8]`, enforces the 256 KiB caller/body ceiling, parses generated DTO, then converts into validated `pastral-ipc-core` DTOs.
- Exposes exact runtime marker `PROTOBUF_RELEASE: &str = "4.35.0-release"` and `schema_sha256() -> [u8; 32]`.

- [ ] **Step 1: Write Edition 2024 `.proto` authority**

Use:

```proto
edition = "2024";
package pastral.ipc.v1;

option optimize_for = LITE_RUNTIME;
```

Define closed operation/capability/error enums with zero unspecified values, request/response `oneof`, and no services, maps, extensions, groups, `Any`, paths, handles, payload bytes, or duplicated correlation field.

- [ ] **Step 2: Add crate shell and failing tests**

Pin exact dependencies:

```toml
protobuf = "=4.35.0-release"
protobuf-codegen = "=4.35.0-release"
```

Tests call the conversion API and expect round-trip equality plus rejection of missing operation, unknown/zero action enum, invalid nonce/UUID, page/query/preview/result limits, zero capture order, and invalid timestamp.

- [ ] **Step 3: Run RED/build probe**

```powershell
cargo test --locked -p pastral-ipc-schema --all-targets -- --nocapture
```

Expected: compile/test failure until generated module and conversion are implemented. Record any native toolchain/build errors exactly; do not substitute another runtime silently.

- [ ] **Step 4: Implement build and conversion**

`build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=../../protocols/ipc-schema/pastral_ipc_v1.proto");
    protobuf_codegen::CodeGen::new()
        .inputs(["../../protocols/ipc-schema/pastral_ipc_v1.proto"])
        .include("../../protocols/ipc-schema")
        .generate_and_compile()
        .expect("exact protobuf code generation must succeed");
}
```

Compute schema SHA-256 at runtime from `include_bytes!` or generated build metadata without network/tool download.

Generated DTO conversion must copy into bounded validated core types; parser defaults never authorize.

- [ ] **Step 5: Run GREEN and inspect dependency tree**

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-ipc-schema --all-targets -- --nocapture
cargo clippy --locked -p pastral-ipc-schema --all-targets -- -D warnings
cargo tree --locked -p pastral-ipc-schema
```

- [ ] **Step 6: Commit**

```powershell
git add Cargo.toml Cargo.lock protocols/ipc-schema crates/ipc-schema
git commit -m "feat: add Edition 2024 IPC schema prototype"
```

---

### Task 6: Release round-trip and footprint probe

**Files:**
- Modify: `Cargo.toml`
- Create: `apps/ipc-probe/Cargo.toml`
- Create: `apps/ipc-probe/src/main.rs`
- Create: `eng/verify-ipc-prototype.ps1`
- Modify: `eng/build.ps1`
- Test: verifier smoke through PowerShell

**Interfaces:**
- Executable: `pastral-ipc-probe.exe`.
- Arguments: `--iterations <1..100000>` only; default 10000; unknown/missing values fail closed.
- Output keys: `ipc-probe=ok`, `protobuf-release`, `schema-sha256`, `iterations`, `round-trips`, `executable-bytes`, `average-roundtrip-ns`, `one-byte-average-ns`, `coalesced-average-ns`, `max-body-capacity`.
- `eng/verify-ipc-prototype.ps1 -Mode Static|Test|Probe|All`.

- [ ] **Step 1: Write failing probe/static verifier expectations**

Static verifier requires:

- exact proto edition/package;
- exact 36-byte constant;
- exact Protobuf versions;
- no forbidden service/gRPC/JSON/reflection/Tokio/network terms in IPC source/manifests;
- probe has no clipboard/storage/pipe/manager dependency;
- only protocol source and build outputs are used.

- [ ] **Step 2: Implement deterministic probe**

Build one health request and one 100-item history response with bounded synthetic metadata. For each iteration:

1. encode validated DTO;
2. construct `ControlProto` header;
3. encode frame;
4. decode once with one-byte chunks;
5. parse and validate body;
6. encode/decode a coalesced pair;
7. assert semantic equality and capacity bounds.

Use `Instant` and checked totals. Output no preview/search contents.

- [ ] **Step 3: Integrate build task**

Add `IpcPrototype` to `eng/build.ps1` and invoke verifier `All`. Do not add it to `pastral-agent` dependencies.

- [ ] **Step 4: Run verifier**

```powershell
.\eng\verify-ipc-prototype.ps1 -Mode All
.\eng\build.ps1 -Task IpcPrototype
```

Expected: static/test/release probe pass and 10,000 round trips complete.

- [ ] **Step 5: Commit**

```powershell
git add Cargo.toml Cargo.lock apps/ipc-probe eng/verify-ipc-prototype.ps1 eng/build.ps1
git commit -m "test: verify IPC framing and schema prototype"
```

---

### Task 7: Dependency/source/CI integration and verification evidence

**Files:**
- Modify: `eng/verify-dependencies.ps1`
- Modify: `eng/verify-source-policy.ps1`
- Modify: `.github/workflows/rust-ci.yml`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Modify: `docs/adr/0018-ipc-schema-and-framing.md`
- Create: `docs/reviews/phase-3d-ipc-framing-schema-prototype-verification.md`

**Interfaces:**
- Dependency policy permits official Protobuf packages only in schema/probe trees and continues to reject them from domain/storage/agent-core/clipboard/agent.
- Source policy scans protocol and IPC source without scanning generated `target/OUT_DIR` output.
- CI triggers on `crates/ipc-core/**`, `crates/ipc-schema/**`, `apps/ipc-probe/**`, and `protocols/ipc-schema/**` and runs `IpcPrototype`.

- [ ] **Step 1: Update dependency and source policy**

Assert exact official package versions from `cargo tree`. Reject `prost`, `tonic`, `tokio`, `serde`, `serde_json`, `hyper`, `reqwest`, `grpc`, reflection, TextFormat, and networking packages in IPC trees.

- [ ] **Step 2: Update CI and validate YAML**

Use a local YAML parser if installed; otherwise manually inspect and record that hosted execution is unproven. Keep Windows 2025 and locked commands.

- [ ] **Step 3: Update ADR and docs**

ADR remains `Proposed`. Add measured official Rust prototype evidence and state whether it passes the Rust-side build/size/latency gate. Do not claim C++ parity, named-pipe authentication, or ADR acceptance.

- [ ] **Step 4: Run fresh aggregate verification**

```powershell
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
cargo tree --locked --workspace
git diff --check
git fsck --no-progress --no-dangling
git status --short
```

Expected:

- all prior 113 tests plus new IPC tests pass;
- official schema and probe build in Debug/Release as configured;
- agent health-check smoke passes without linking IPC schema;
- manager Debug/Release/UIA passes;
- working tree clean after evidence commit.

- [ ] **Step 5: Commit evidence**

```powershell
git add .github/workflows/rust-ci.yml eng README.md CHANGELOG.md docs
git commit -m "docs: record IPC prototype evidence"
```

- [ ] **Step 6: Final hygiene commit only if required**

If `git show --check` reports Markdown trailing whitespace, remove it and commit:

```powershell
git commit -am "docs: normalize IPC evidence formatting"
```
