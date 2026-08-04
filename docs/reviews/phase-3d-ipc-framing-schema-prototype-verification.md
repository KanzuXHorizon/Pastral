# Phase 3D IPC Framing and Schema Prototype Verification

**Date:** 2026-08-04

**Baseline:** `7b5ce15`

**Implementation head:** `d010c4e`

**Scope:** serializer-neutral 36-byte framing, incremental byte-stream decoding, connection/bulk state, bounded control DTOs, exact official Protocol Buffers Edition 2024 Rust prototype, release measurement probe, dependency/source/CI integration

## 1. Result

Phase 3D is **green for its declared Rust-side framing/schema prototype scope**.

The repository now has a tested and measured protocol foundation:

```text
untrusted byte stream
  -> exact 36-byte header accumulation
  -> structural/header limit validation
  -> exact bounded body allocation
  -> fragmented/coalesced frame decode
  -> handshake/in-flight/bulk connection state
  -> exact official Protobuf parse
  -> explicit presence/enum/length validation
  -> serializer-neutral typed DTO
```

The implementation does not create a production named pipe, connect the agent to the manager, link Protobuf into the resident agent, stage clipboard payloads, or enable any new background behavior.

ADR 0018 remains Proposed. This phase validates the Rust prototype candidate only; C++ parity, authenticated named-pipe transport, fuzzing, resident private-working-set attribution, bulk staging cleanup, and adjacent-version compatibility remain open acceptance gates.

## 2. Commits

```text
8ca30e7 docs: specify IPC framing prototype
f5f053a docs: plan IPC framing prototype
03cfd2c feat: add bounded IPC frame codec
3137c74 feat: add incremental IPC frame decoder
df61f04 feat: add IPC connection state machine
246d69e feat: add validated IPC control DTOs
e4a7344 feat: add Edition 2024 IPC schema prototype
d010c4e test: verify IPC framing and schema prototype
```

Dependency/source/CI integration, ADR updates, documentation, and this report are committed as the evidence batch after final verification.

## 3. Official Protocol Buffers basis

The prototype uses official Protocol Buffers contracts and current release-train guidance:

- Protocol Buffers v35 is the current supported release train used by this phase;
- Edition 2024 is the schema edition authority;
- official Rust runtime/codegen package versions are language-specific `4.35.0-release`;
- exact generator/generated-code/runtime matching is treated as a build constraint;
- official Rust generated bindings use the upb/native kernel rather than a pure-Rust parser;
- reflection, TextFormat, JSON mapping, dynamic messages, services/gRPC, extensions, groups, and `Any` are excluded from the core schema.

Primary references:

- <https://protobuf.dev/support/version-support/>
- <https://protobuf.dev/editions/overview/>
- <https://protobuf.dev/reference/rust/>
- <https://protobuf.dev/reference/rust/rust-generated/>
- <https://github.com/protocolbuffers/protobuf/releases/tag/v35.0>

`prost` was reviewed as an alternative but was not selected for this implementation because the current release is passively maintained and complete Editions support remains unresolved. The serializer-neutral `pastral-ipc-core` remains replaceable if official-runtime resident evidence later fails.

## 4. Package boundaries

### 4.1 `pastral-ipc-core`

Properties:

- `#![forbid(unsafe_code)]`;
- no Windows bindings;
- no serializer/runtime dependency;
- no async runtime;
- no network, pipe, clipboard, storage, logging, or manager dependency;
- depends on `pastral-domain` for typed UUID/time/order invariants and `uuid` for transient correlation generation.

Owns:

- frame kinds and limits;
- exact header codec;
- incremental decoder;
- connection/bulk state;
- serializer-neutral validated control DTOs.

### 4.2 `pastral-ipc-schema`

Properties:

- exact `protobuf = 4.35.0-release`;
- exact `protobuf-codegen = 4.35.0-release`;
- repository-authoritative Edition 2024 `.proto` input;
- generated output under Cargo `OUT_DIR` only;
- conversion from generated DTOs into validated `pastral-ipc-core` DTOs;
- no generated DTO is exposed to domain, storage, agent, or manager APIs.

### 4.3 `pastral-ipc-probe`

Properties:

- Release-only deterministic diagnostic workload;
- no clipboard, storage, named-pipe, network, agent, or manager dependency;
- content-free metric output;
- default 10,000 iterations, bounded to 1–100,000;
- unknown/duplicate/invalid arguments fail closed.

## 5. Exact 36-byte frame

Implemented header:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | magic `PSTR` |
| 4 | 2 | framing major `1` |
| 6 | 2 | closed frame kind |
| 8 | 4 | zero reserved flags |
| 12 | 4 | body length |
| 16 | 4 | frame sequence |
| 20 | 16 | correlation/transfer UUID bytes |

Frame kinds:

```text
1 HelloProto
2 ControlProto
3 BulkChunk
4 BulkEndProto
5 ProtocolErrorProto
```

Structural limits:

```text
control/hello/error body: 256 KiB
bulk chunk body: 1 MiB and nonzero
frames emitted per push: 64
in-flight requests: 16
```

Validation occurs field-by-field with little-endian conversions. Input is never cast to a native struct.

Rejected before body allocation:

- invalid magic;
- framing-major mismatch;
- unknown frame kind;
- nonzero reserved flags;
- body over its kind ceiling;
- empty bulk chunk;
- nonzero sequence for hello/control/error;
- zero correlation outside hello;
- nonzero correlation that is not RFC 4122 UUIDv4 bytes.

The all-zero correlation remains structurally legal for the outbound initial server hello. Server-side inbound connection state rejects a zero-correlated client hello.

## 6. Incremental decoder

`FrameDecoder` owns exactly one in-progress frame:

```text
Header { [u8; 36], filled }
Body { validated header, exact boxed byte slice, filled }
Poisoned
```

Properties verified:

- no input accumulation buffer;
- no body allocation before all 36 header bytes validate;
- exact body-sized allocation;
- empty-body frames emit immediately;
- one-byte feeds work;
- coalesced multiple frames work;
- complete frame plus partial next frame preserves progress;
- partial header/body fails at `finish()`;
- protocol error poisons the decoder;
- push/finish after poison returns a typed error;
- frame count above the per-push limit poisons the decoder.

Coverage includes every header split from 0 through 36, every split in a representative body, and critical split points for the maximum 256 KiB control body.

## 7. Connection and bulk state

Server phases:

```text
AwaitClientHello
Ready
BulkReceiving
Closed
```

Control rules:

- only nonzero-correlated `HelloProto` is accepted before ready;
- second hello, protocol-error request, or frame before handshake closes the connection;
- request correlations are unique while in flight;
- maximum 16 in-flight requests;
- explicit completion/cancellation releases a correlation;
- protocol violation clears in-flight and bulk state.

Bulk rules:

- one active transfer;
- transfer must be explicitly authorized with nonzero UUID, maximum bytes, and maximum chunks;
- chunk correlation must match the authorized transfer;
- sequence must be exactly the next zero-based value;
- chunk count and total bytes use checked arithmetic;
- excess chunks or bytes close the connection;
- bulk end sequence equals accepted chunk count;
- schema-decoded final total length and chunk count must match progress;
- a post-end chunk is rejected;
- cancel returns to ready without committing anything.

No whole-transfer staging, file I/O, persistence, digest, or clipboard payload processing exists in this phase.

## 8. Serializer-neutral DTO validation

Bounded DTOs cover:

- server/client hello;
- health request/response;
- history-page request/response;
- search request/response;
- clip preview;
- protocol error;
- bulk end.

Current semantic limits:

```text
nonce: exactly 32 bytes and nonzero
instance/event/correlation ID: UUIDv4 binary form
page limit: 1..100
search query: 1..1024 UTF-8 bytes, max 32 terms, no NUL
preview: max 4096 UTF-8 bytes, no NUL
source label: max 256 UTF-8 bytes, no NUL
error detail: max 512 UTF-8 bytes, no NUL
response previews: max 100
capture order: nonzero
observed time: pastral-domain supported UTC microseconds
```

Capabilities are nonempty, unique, and closed. Unavailable previews must have an empty preview and matching kind/state.

Types containing query, preview, source label, or error detail do not derive `Debug`, reducing accidental diagnostic content exposure.

## 9. Edition 2024 schema

Authority:

```text
protocols/ipc-schema/pastral_ipc_v1.proto
package pastral.ipc.v1
edition 2024
```

The schema contains:

- `ServerHello` / `ClientHello`;
- `RequestEnvelope` / `ResponseEnvelope` with exactly one operation `oneof`;
- health, history-page, and search operations;
- bounded preview metadata;
- protocol error;
- bulk-end metadata.

It contains no clipboard payload, path, window handle, OS handle, pointer, SQLite row ID, runtime clipboard-format ID, encryption key, service, map, extension, group, `Any`, or import.

Generated conversion checks explicit presence before accessing every security-relevant scalar. Zero and unknown enums are rejected. Parser defaults never grant a capability or operation.

## 10. Toolchain and reproducibility

Pinned components:

```text
protoc: libprotoc 35.0
protobuf runtime: 4.35.0-release
protobuf codegen: 4.35.0-release
protobuf macros: 4.35.0-release
schema SHA-256: 409c0da02f90e70e9bb1acbf1d7818d31ffcee3b61480cfa4ab250a5a8f493d8
```

Local reference installation:

```text
Google.Protobuf 35.0 user-scoped Winget portable package
C:\Users\KanzuWakazaki\AppData\Local\Microsoft\WinGet\Packages\Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\protoc.exe
```

The build script resolves:

1. explicit `PROTOC`;
2. user-scoped Winget portable package;
3. `protoc` on `PATH`.

The official codegen crate verifies the compiler version. The build script does not download a compiler or other external tool.

Generated code remains in:

```text
Cargo OUT_DIR/protobuf_generated/
```

No generated source is tracked in Git.

## 11. Generated unsafe and lint boundary

Official Rust bindings use the upb/native implementation and generate unsafe FFI code. The implementation boundary is explicit:

- `pastral-ipc-core`: `forbid(unsafe_code)`;
- handwritten `pastral-ipc-schema`: `deny(unsafe_code)`;
- only the `generated` module permits unsafe code;
- only exact generator-produced Clippy findings are allowed in that module;
- generated `OUT_DIR` files are never manually edited;
- repository source contains no handwritten unsafe IPC block.

Allowed generated-only Clippy findings:

```text
derivable_impls
manual_range_patterns
needless_borrow
unnecessary_fallible_conversions
useless_conversion
```

Handwritten schema code remains under workspace `-D warnings`.

## 12. TDD evidence

### 12.1 Frame RED

Frame tests were written before implementation. Compilation failed because these symbols did not exist:

```text
CorrelationId
FrameHeader
FrameKind
FrameLimits
IpcError
FRAME_HEADER_BYTES
```

After implementation, 8 frame tests passed.

### 12.2 Decoder RED

Decoder tests were written before implementation. Compilation failed because these symbols did not exist:

```text
Frame
FrameDecoder
```

After implementation, 8 decoder tests passed.

### 12.3 Connection RED

Connection tests were written before implementation. Compilation failed because these symbols did not exist:

```text
AcceptedFrame
ConnectionPhase
ServerConnection
```

After implementation, 8 connection tests passed.

### 12.4 DTO RED

DTO tests were written before implementation. Compilation failed because the bounded control DTOs/constants did not exist. After implementation, 6 DTO tests passed.

### 12.5 Schema toolchain progression

The first official codegen attempt failed because `protoc` was unavailable. The repository did not silently select another runtime.

After exact official `protoc 35.0` installation, codegen passed its version check. Two build-integration issues were then corrected:

1. absolute input paths caused the codegen crate to expect output outside `OUT_DIR`; input was changed to a path relative to the include root;
2. generated entry output is `protobuf_generated/generated.rs`, not an assumed root `generated.rs`.

The first crate-level `forbid(unsafe_code)` attempt correctly exposed generated upb unsafe code. The boundary was changed to handwritten `deny(unsafe_code)` plus a generated-module-only exception; core remained fully safe.

After conversion implementation, 5 round-trip and 6 adversarial schema tests passed.

### 12.6 Probe RED/GREEN

Probe parser and one-iteration semantic tests were added before the verification gate. Three tests pass:

- default/exact bounded iteration parsing;
- fail-closed invalid arguments;
- one complete request/100-item response round trip.

## 13. Test totals

Fresh workspace total:

| Package | Tests |
|---|---:|
| `pastral-agent` | 14 |
| `pastral-agent-core` | 18 |
| `pastral-clipboard-win` | 22 |
| `pastral-domain` | 26 |
| `pastral-ipc-core` | 30 |
| `pastral-ipc-schema` | 11 |
| `pastral-ipc-probe` | 3 |
| `pastral-storage` | 33 |
| **Total** | **157** |

IPC-focused total:

```text
30 ipc-core
11 ipc-schema
3 ipc-probe
44 total
```

All tests pass.

## 14. Release probe evidence

The probe builds a bounded health request and a synthetic 100-item history response. Every iteration performs:

1. DTO serialization;
2. control-frame construction;
3. one-byte fragmented request decode;
4. request parse and validation;
5. coalesced request/response decode;
6. request/response parse and validation;
7. semantic equality and frame-capacity checks.

Representative runs on the current reference machine:

| Metric | Run 1 | Final `Full` run |
|---|---:|---:|
| Iterations | 10,000 | 10,000 |
| Completed | 10,000 | 10,000 |
| Executable bytes | 379,904 | 379,904 |
| Average full round trip | 129,576 ns | 145,556 ns |
| One-byte decoder component | 713 ns | 907 ns |
| Coalesced decoder component | 583 ns | 695 ns |
| Maximum body capacity | 7,869 | 7,869 |

The verifier uses a deliberately broad 1 ms average smoke ceiling. It records correctness and regression evidence; it is not a release latency SLA.

The current Release agent is 2,137,088 bytes and remains protobuf-free. Therefore the probe size is not represented as resident-agent incremental size, and private working set remains unmeasured.

Probe output contains only version/hash/count/size/timing/capacity fields. It never prints synthetic preview/source contents.

## 15. Dependency and source policy

Dependency policy proves:

- exact `protobuf`, `protobuf-codegen`, and `protobuf-macros` `4.35.0-release`;
- official Protobuf/upb transitive packages are isolated to schema/probe trees;
- agent, agent-core, clipboard-win, domain, ipc-core, and storage remain protobuf-free;
- ipc-core/schema/probe remain Windows-binding free;
- Tokio, async runtimes, `prost`, `tonic`, gRPC, Serde/JSON, HTTP/network stacks, and logging backends are absent from IPC trees.

Source policy now scans `.proto` input and the probe. It rejects product network/process/named-pipe APIs in the current prototype, tracked build/generated output, secret/key files, unsafe product code outside reviewed boundaries, SQLite extension/attach/WAL activation, and existing repository-wide forbidden signatures.

## 16. CI evidence

`.github/workflows/rust-ci.yml` now triggers on IPC core/schema/probe/protocol paths.

The Windows 2025 workflow:

1. downloads official `protoc-35.0-win64.zip`;
2. verifies SHA-256 `d1cede9e308cc3eb072392af1c02ccae4bdd3d2f374ec2970dbd8cdfdaa91363`;
3. exposes exact `libprotoc 35.0` through `PROTOC` and `PATH`;
4. runs locked workspace gates;
5. runs the full IPC prototype verifier.

Local YAML parsing with Python/PyYAML passed:

```text
RUST_CI_YAML=PASS
```

Hosted GitHub Actions execution was not performed in this local phase and remains unproven.

## 17. Aggregate verification

Fresh commands:

```text
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
```

Results:

```text
Rust toolchain: PASS
Formatting: PASS
Workspace check: PASS
157 Rust tests: PASS
Workspace Clippy -D warnings: PASS
Cargo documentation: PASS
Dependency policy: PASS
Source policy: PASS
IPC static/test/Release probe: PASS
Agent static/Debug/Release/health-check smoke: PASS
Native manager static: PASS
Native manager Debug/Release: PASS
Manager History UI Automation: PASS
Manager filtering/selection/no-results: PASS
Manager clean shutdown: PASS
```

No automated command opened, read, or changed the user's clipboard.

## 18. Explicit exclusions and remaining risks

Phase 3D does not implement or claim:

- production named-pipe server/client;
- explicit per-logon-session DACL;
- first-instance anti-squatting;
- remote-client rejection;
- peer PID/token/user/session validation;
- challenge/installation-secret authentication;
- authorization/deadline/cancellation transport enforcement;
- C++ generated code/runtime or Rust/C++ wire parity;
- fuzzing of frame/parser/conversion code;
- bulk staging files, digest verification, cleanup, or storage commit;
- resident-agent protobuf linkage or memory attribution;
- manager live snapshots/search;
- adjacent-version compatibility fixtures;
- production protocol acceptance;
- auto-start, replay, paste, Quick Paste, overlay, packaging, or update behavior.

The official Rust runtime uses native/upb code. Its supply-chain, license, advisory, memory, and crash behavior require continuing review before resident adoption.

## 19. Conclusion

The 36-byte framing/state/schema design is now executable, bounded, reproducible, and measured on Rust/Windows x64. The official Protocol Buffers Rust runtime is viable as the selected Rust prototype runtime under isolation.

ADR 0018 remains Proposed. The next admissible slice is authenticated per-user named-pipe transport around this foundation, not direct manager connection or paste/replay behavior.
