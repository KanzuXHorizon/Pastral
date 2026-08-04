# Pastral Phase 3D — IPC framing and schema runtime prototype design

**Date:** 2026-08-04
**Status:** Approved for implementation by the user's standing instruction to continue the next evidence-gated slice
**Baseline:** `7b5ce15`

## 1. Purpose

Phase 3D creates the bounded, serializer-isolated IPC protocol foundation required before Pastral can expose live agent state to the manager. It does not yet open a production named pipe or connect the WinUI manager.

The slice must answer two independent questions:

1. Can Pastral parse, validate, encode, fragment, and coalesce its fixed 36-byte byte-stream frames without unbounded allocation or dependence on Windows message boundaries?
2. Can the official Protocol Buffers v35 release-train Rust runtime compile reproducibly with Edition 2024 schemas and remain sufficiently isolated that it can be replaced if footprint or build evidence fails?

## 2. Scope

Implement:

- a pure-Rust `pastral-ipc-core` crate;
- exact 36-byte frame encode/decode;
- incremental byte-stream decoder with bounded header/body/frame output state;
- structural and connection-state validation;
- ordered bulk-transfer state validation without staging/persistence;
- a protocol DTO/domain-validation boundary;
- `protocols/ipc-schema/pastral_ipc_v1.proto` as the schema authority;
- an isolated `pastral-ipc-schema` crate using exact official Protocol Buffers `4.35.0-release` Rust runtime and code generator artifacts;
- Edition 2024 schemas for hello, health, history-page, search, bounded error, and bulk-end metadata;
- reproducible schema digest verification;
- a release probe that measures incremental binary size and warm parse/serialize latency;
- source/dependency policy updates scoped only to IPC crates and the probe;
- CI/build tasks for framing, schema generation, tests, and measurement smoke.

Do not implement:

- production named-pipe server/client;
- pipe ACLs, impersonation, token/session validation, anti-squatting, or challenge secret persistence;
- manager integration;
- clipboard payload transport;
- bulk staging files or storage commits;
- paste/replay commands;
- auto-start;
- Tokio, gRPC, HTTP, JSON, reflection, TextFormat, dynamic messages, services, or `Any`;
- C++ generated-code/runtime integration in this slice;
- acceptance of ADR 0018 as final.

## 3. Decision and alternatives

### Selected approach

Use two replaceable layers:

1. `pastral-ipc-core` owns framing, limits, state, UUID/correlation rules, and validated product DTOs with no serializer dependency.
2. `pastral-ipc-schema` owns only generated Protocol Buffers DTOs plus bounded conversion functions.

The official Rust runtime is selected for prototype measurement because Protocol Buffers v35.0 is the current supported release train, supports Edition 2024, and publishes exact-matched Rust runtime/codegen crates. The official runtime remains provisional because it uses a non-Rust upb kernel and exact Rust/C++ runtime matching is required.

### Rejected default: `prost`

`prost` remains a comparison candidate, not the implementation default. Its current release is passively maintained and Edition support remains incomplete. Selecting it now would contradict the repository's Edition 2024 requirement and weaken the cross-language exact-version evidence goal.

### Rejected default: custom TLV/control serializer

A project-owned serializer could be smaller, but Pastral would own schema evolution, unknown-field behavior, code generation, and Rust/C++ parity. That increases parser and compatibility risk. The framing layer remains custom because its exact bounded semantics are already normative; the control schema does not.

### Rollback

If official runtime footprint/build/security evidence fails:

- remove `pastral-ipc-schema` and its generated/native dependencies;
- retain `pastral-ipc-core`, frame fixtures, limits, state machine, and schema-neutral DTO validators;
- amend ADR 0018 with the measured alternative;
- no storage, manager, or agent runtime model changes are required.

## 4. Package boundaries

### `crates/ipc-core`

Properties:

- `#![forbid(unsafe_code)]`;
- no Windows bindings;
- no serializer/runtime dependency;
- no async runtime;
- depends only on `uuid` if required for canonical UUIDv4 validation;
- exposes no payload logging/debug representation.

Modules:

- `error.rs` — closed stable error enum;
- `frame.rs` — frame header, kind, flags, correlation ID, encode/decode;
- `decoder.rs` — bounded incremental byte-stream state machine;
- `connection.rs` — handshake/ready/bulk protocol state;
- `dto.rs` — serializer-neutral validated hello/request/response/preview types;
- `limits.rs` — all protocol ceilings and constructor validation.

### `crates/ipc-schema`

Properties:

- exact `protobuf = 4.35.0-release`;
- exact `protobuf-codegen = 4.35.0-release` build dependency;
- generated code built from repository `.proto` authority;
- no network, async, gRPC, reflection, JSON, TextFormat, services, `Any`, or logging dependency;
- conversion functions accept generated DTOs and return `pastral-ipc-core` validated DTOs;
- generated types never enter domain/storage/manager APIs directly.

### `apps/ipc-probe`

A release-only diagnostic executable used by verification. It:

- constructs deterministic synthetic control DTOs;
- serializes, frames, incrementally decodes, parses, validates, and reserializes;
- reports only timing/count/size/version/schema-digest data;
- never opens clipboard, storage, manager, pipe, network, or user data;
- exits nonzero when any bound or round-trip invariant fails.

## 5. Frame contract

Header size is exactly 36 bytes:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | magic `PSTR` |
| 4 | 2 | framing major |
| 6 | 2 | frame kind |
| 8 | 4 | flags |
| 12 | 4 | body length |
| 16 | 4 | frame sequence |
| 20 | 16 | correlation/transfer UUID bytes |

Initial framing major is `1`.

Frame kinds use explicit numeric values:

- `1`: `HelloProto`;
- `2`: `ControlProto`;
- `3`: `BulkChunk`;
- `4`: `BulkEndProto`;
- `5`: `ProtocolErrorProto`.

Unknown values are rejected.

Rules:

- magic must match exactly;
- framing major must equal `1`;
- flags must be zero;
- control/hello/error body length must be at most 256 KiB;
- bulk chunk body length must be at most 1 MiB and nonzero;
- sequence must be zero for hello/control/error;
- bulk chunk sequence is validated by active bulk state;
- bulk end sequence equals accepted chunk count;
- all-zero correlation is legal only for the outbound initial server hello;
- every other correlation is canonical RFC 4122 UUIDv4 bytes;
- body allocation occurs only after the complete header passes structural and state-independent limits.

## 6. Incremental decoder

`FrameDecoder` owns one in-progress frame at a time:

- fixed `[u8; 36]` header buffer;
- body `Vec<u8>` allocated exactly to validated body length;
- no retained input accumulation buffer;
- configurable maximum emitted frames per `push` call, default 64;
- decoder becomes poisoned after a protocol error and accepts no further input;
- `finish()` rejects any partial header or body;
- zero-length control/hello/error bodies are structurally legal; schema validation may reject them;
- every header split point, every body split point, one-byte input, coalesced frames, and complete+partial coalescing are tested.

## 7. Connection state

Server-side inbound prototype state:

- `AwaitClientHello`;
- `Ready`;
- `BulkReceiving`;
- `Closed`.

Rules:

- `AwaitClientHello` accepts only nonzero-correlated `HelloProto`;
- `Ready` accepts `ControlProto` and explicitly authorized bulk frames;
- `ProtocolErrorProto` is never accepted as an ordinary request;
- frame-before-handshake closes state;
- protocol error poisons/closes state;
- duplicate request correlation while in flight is rejected;
- maximum 16 in-flight correlations per connection;
- a test API explicitly completes/cancels a correlation;
- one bulk transfer per connection;
- `authorize_bulk` sets transfer UUID, maximum bytes, and maximum chunk count;
- chunk sequence must equal the exact next value;
- accepted byte count uses checked arithmetic and cannot exceed maximum;
- valid bulk end must match chunk count and total length;
- any bulk error cancels the transfer and closes the prototype connection.

No authorization decision is inferred from body fields in this slice.

## 8. Edition 2024 schema

Package: `pastral.ipc.v1`.

Messages:

- `ServerHello`;
- `ClientHello`;
- `RequestEnvelope`;
- `ResponseEnvelope`;
- `HealthRequest` / `HealthResponse`;
- `HistoryPageRequest` / `HistoryPageResponse`;
- `SearchRequest` / `SearchResponse`;
- `ClipPreview`;
- `BulkEnd`;
- `ProtocolError`.

The schema contains no clipboard payload.

Bounded semantic fields:

- protocol major/minor and capability values;
- 32-byte nonce fields;
- UUIDs as 16 bytes;
- page limit maximum 100;
- search query maximum 1024 UTF-8 bytes and 32 terms;
- preview text maximum 4096 UTF-8 bytes;
- source label maximum 256 UTF-8 bytes;
- error developer detail maximum 512 UTF-8 bytes;
- response preview count maximum 100;
- observed times are signed UTC microseconds;
- capture order is nonzero unsigned 64-bit;
- security/action enums reject zero and unknown values;
- every request/response envelope must contain exactly one recognized operation.

Unknown fields never grant capabilities or authorization.

## 9. Schema reproducibility

Verification records:

- exact runtime/codegen versions;
- SHA-256 of the `.proto` source;
- generated entry point presence;
- clean rebuild with `--locked`;
- no generated source drift outside Cargo `OUT_DIR`;
- no automatic external tool download in build scripts.

The crate's build script uses only the exact pinned `protobuf-codegen` crate and repository inputs.

## 10. Performance evidence

`ipc-probe` reports:

- executable file size;
- schema/runtime version marker;
- schema SHA-256;
- iterations;
- total and average nanoseconds for:
  - serialize;
  - frame encode;
  - one-byte fragmented decode;
  - coalesced decode;
  - parse and post-parse validation;
- peak body capacity observed by the decoder;
- warm round-trip success count.

Initial smoke thresholds are intentionally wide and correctness-focused:

- 10,000 typical round trips complete successfully;
- average full round trip below 1 ms on the current reference machine;
- decoder body capacity never exceeds the validated body length;
- no idle loop, timer, thread, network, clipboard, storage, or pipe activity;
- release probe and agent incremental binary sizes are recorded, not yet product acceptance claims.

## 11. Tests

### Frame tests

- canonical header round trip;
- invalid magic/major/kind/flags;
- control and bulk length boundaries;
- invalid sequence rules;
- zero/non-v4/non-RFC4122 correlation rules;
- arbitrary alignment decode;
- exact 36-byte output.

### Decoder tests

- each header split from 0 through 36;
- each body split for representative and maximum-sized control body;
- one-byte feeds;
- multiple coalesced frames;
- complete frame plus partial next frame;
- truncated finish;
- maximum frames per push;
- poisoned-state behavior;
- no body allocation before valid complete header.

### Connection tests

- hello order;
- frame before handshake;
- duplicate/in-flight correlation bounds;
- complete/cancel correlation;
- bulk happy path;
- duplicate/gap/reorder/overflow/excess/invalid end;
- close/poison behavior.

### Schema tests

- valid hello/health/history/search/error/bulk-end round trip;
- missing/unknown operation;
- nonce/UUID length;
- zero/unknown enum;
- page/query/term/string/result limits;
- invalid time/capture order;
- generated DTO never bypasses validated conversion;
- exact version and schema digest checks.

## 12. Build and policy integration

Add:

- `eng/verify-ipc-prototype.ps1` with `Static`, `Test`, `Probe`, `All`;
- `eng/build.ps1 -Task IpcPrototype`;
- Rust CI path coverage for IPC crates, schema, protocol files, and verifier.

Dependency policy:

- `pastral-domain`, `pastral-agent-core`, and `pastral-storage` remain free of Protobuf and Windows bindings;
- official Protobuf dependencies are permitted only in `pastral-ipc-schema` and `pastral-ipc-probe` dependency trees;
- Tokio, async runtimes, gRPC, HTTP, JSON, generic serialization frameworks, logging backends, and network crates remain forbidden;
- the resident `pastral-agent` does not link the schema runtime in Phase 3D.

Source policy:

- protocol/generated-input files are scanned;
- generated `OUT_DIR` remains untracked;
- no payload/log/network/process APIs are introduced.

## 13. Acceptance criteria

Phase 3D is complete when:

1. all new focused tests pass;
2. all existing 113 Rust tests continue to pass;
3. formatter, workspace check, Clippy, docs, dependency policy, and source policy pass;
4. frame decoder fragmentation/coalescing matrix passes;
5. schema runtime builds from exact locked versions and Edition 2024 source;
6. release probe completes 10,000 deterministic round trips and emits bounded metrics;
7. manager Debug/Release and UI Automation regressions pass;
8. no production pipe, clipboard read, storage mutation, manager live-data claim, or auto-start is added;
9. a verification report records runtime/build/size/latency evidence and whether ADR 0018 may advance.

## 14. Next slice

If Phase 3D passes, Phase 3E may implement authenticated per-user named-pipe transport around the accepted frame/schema layers, including explicit DACL, remote rejection, first-instance anti-squatting, peer PID/token/session validation, challenge binding, connection limits, and Rust CLI/agent integration. Manager integration remains after a real authenticated transport and C++ schema/client path are green.
