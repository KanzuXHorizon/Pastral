# ADR 0018: Protobuf control schema with bounded named-pipe framing

**Status:** Proposed — Rust framing/schema prototype passes; C++ parity, authenticated transport, fuzzing, and resident-agent adoption gates remain open
**Date:** 2026-08-04

## Context

The Rust agent/CLI and C++/WinRT manager need one versioned cross-language protocol. Named pipes preserve bytes/messages but do not provide a safe application schema, request correlation, bulk-transfer model, or compatibility rules. Hand-written struct layouts, JSON, arbitrary serializers, or direct language enum/object serialization would create ambiguity, allocation risk, and divergent Rust/C++ behavior.

Protocol choice must remain small enough for the resident agent, reject malformed/oversized input before allocation, support explicit adjacent agent/manager version overlap, and keep clipboard payloads out of ordinary control messages.

Official Protobuf supplies C++ and Rust generation, but the official Rust implementation is backed by upb/C++ kernels rather than a pure-Rust parser. Official C++ and Rust support requires exact generated-code/runtime version matching. Therefore wire/schema suitability and resident-runtime suitability are separate decisions.

## Decision

Prototype a two-layer IPC wire contract. The framing, limits, schema rules, and conversion boundaries below are normative for the prototype; final runtime adoption requires the acceptance gates in this ADR:

1. a small fixed little-endian Pastral frame header for bounded transport framing and correlation;
2. Protocol Buffers binary wire format for control/metadata bodies;
3. raw bounded binary chunk bodies for explicitly authorized bulk streams.

### Protocol Buffers toolchain

- `.proto` files are the sole schema authority under `protocols/ipc-schema/`.
- Use Protobuf Edition 2024 for the prototype, with explicit field presence and no custom feature options without a documented need.
- Initial release-train candidate is Protocol Buffers v35.0, revalidated at prototype/bootstrap time.
- Follow the official per-language exact-match requirement for C++ and Rust generator, generated code, and runtime artifacts; do not describe every published language package simply as `35.0` when its language major differs.
- C++ control messages prototype lite runtime generation where supported. Reflection, TextFormat, JSON mapping, dynamic messages, services/gRPC, extensions, groups, and `Any` are not used in core IPC.
- The official Rust Cargo path and C++ generator/runtime are the leading implementation candidate, not yet an accepted resident dependency.
- Prototype the official Rust kernel/build path against at least one credible wire-compatible Rust alternative when actively maintained and security-reviewed. Compare binary/private-working-set impact, allocations, parse latency, native dependencies, MSVC/Cargo integration, reproducibility, fuzzability, advisory process, and license.
- The agent does not load a general async runtime, HTTP stack, gRPC runtime, reflection registry, or JSON support merely for serialization.
- Generated output must be reproducible from pinned schemas/compiler/runtime; CI regenerates and compares descriptor/schema hashes. Whether generated output is committed is selected by bootstrap evidence and must prevent stale codegen/runtime mismatch.

### Framing

Use named-pipe byte mode (`PIPE_TYPE_BYTE`/`PIPE_READMODE_BYTE`); do not depend on Windows message boundaries. Every application frame starts with an exact 36-byte header parsed field-by-field, and code must not cast untrusted bytes to an ABI/layout-dependent struct:

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 0 | 4 | magic | ASCII `PSTR` |
| 4 | 2 | framing major | unsigned little-endian; incompatible changes increment major |
| 6 | 2 | frame kind | closed validated enum |
| 8 | 4 | flags | reserved bits must be zero unless negotiated |
| 12 | 4 | body length | unsigned little-endian; validated against kind/connection limits before allocating the bounded body buffer |
| 16 | 4 | frame sequence | zero for non-bulk frames; zero-based chunk index for `BULK_CHUNK`; `BULK_END_PROTO` equals accepted chunk count |
| 20 | 16 | correlation/stream UUID | canonical UUIDv4 bytes; all zero allowed only for the initial server hello |

Initial frame kinds:

- `HELLO_PROTO` — Protobuf negotiation/challenge body;
- `CONTROL_PROTO` — Protobuf request/response/event body;
- `BULK_CHUNK` — raw bytes for an already authorized transfer;
- `BULK_END_PROTO` — Protobuf final length/digest/result body;
- `PROTOCOL_ERROR_PROTO` — bounded Protobuf error body before close when safe.

Unknown frame kind, nonzero unnegotiated flag, invalid zero/nonzero sequence, duplicate/out-of-order bulk sequence, invalid UUID use, truncated header/body, length overflow, or frame-before-handshake closes the connection. The header is transport metadata, not an authentication tag.

Initial hard ceilings, adjustable downward by operation but not upward without measured memory/latency/DoS evidence and protocol review:

- control/hello/error body: 256 KiB;
- bulk chunk body: 1 MiB;
- in-flight control requests: 16 per connection and 64 globally;
- one active bulk transfer per connection initially;
- one bulk transfer logical total: operation/policy-specific, never inferred solely from a 32-bit frame length;
- nesting/recursion, string, repeated-field, page, result-count, and parser peak-allocation limits enforced through runtime controls where available and validated/measured before domain conversion.

### Control schema rules

- `HELLO_PROTO` negotiates protocol major/minor and capabilities and binds them to connection state. Post-handshake request/response/event envelopes contain exactly one known operation/event `oneof`, operation-specific deadline fields where required, and no clipboard payload.
- The 16-byte frame-header correlation UUID is the sole request/response/event correlation authority. Control bodies do not duplicate it; operation-specific domain/transfer IDs have distinct names and semantics.
- Exactly one known operation/event must be present. An envelope containing only unknown fields or an unknown action is rejected.
- Every enum defines `*_UNSPECIFIED = 0`; security/policy/action enums reject zero and unknown numeric values unless the field is explicitly informational and forward-compatible.
- Security-critical scalars use explicit field presence and are validated; parser defaults never authorize behavior.
- Deleted fields and names are permanently `reserved`; field numbers are never reused.
- `map` is avoided where deterministic ordering, duplicate-key rejection, authorization, or signatures/transcript hashing matter; use repeated key/value records with explicit duplicate validation.
- Unknown fields may be preserved by runtimes but are never interpreted as authorization. A message requiring an unknown capability/action fails safely.
- Strings are bounded UTF-8 display/identifier fields. Paths, titles, source labels, and user text receive field-specific limits/redaction and are never log-safe merely because parsing succeeded.
- Domain UUIDs are fixed 16-byte fields, not free-form UUID strings, except human-readable diagnostics/export boundaries.
- Civil times are signed UTC microseconds; durations/deadlines use explicit relative integer units and overflow validation.
- No raw HWND, HANDLE, pointer, COM object, SQLite row ID, runtime registered-format ID, encryption key, or arbitrary serialized object appears in the schema.

### Bulk transfer

Clipboard/image/file/export payloads do not appear in `CONTROL_PROTO`:

1. a control request authorizes a direction, transfer UUID, representation/export ID, expected maximum, sensitivity/protection policy, and optional expected raw digest where allowed;
2. receiver allocates no whole-transfer buffer by default and returns an accepted transfer window/policy;
3. `BULK_CHUNK` frames use the transfer UUID and explicit zero-based `frame sequence`; each accepted value is exactly the previous value plus one;
4. receiver streams to bounded memory or a private staging file and applies cancellation/deadline/quota/disk/authorization-freshness checks;
5. `BULK_END_PROTO` uses sequence equal to accepted chunk count and commits exact total length, permitted digest, and result;
6. gap, duplicate/reordered chunk, sequence/count overflow, excess bytes, cancellation, disconnect, or authorization expiry destroys staging output;
7. Private/sensitive transfer never introduces a plaintext digest merely for IPC; integrity is bound to the encrypted/authorized operation according to the protection-domain design.

The pipe is local and not treated as an end-to-end encrypted transport. ACL/peer/session/handshake/operation authorization still apply; same-user residual risk remains.

### Compatibility

- Framing major mismatch: reject before body parse with a bounded error when safe.
- Pastral protocol major mismatch: reject.
- Pastral protocol minor overlap: negotiate the minimum mutually supported minor plus explicit capabilities.
- Protobuf generator/generated-code/C++/Rust runtime exact matching is a build constraint distinct from Pastral wire-schema compatibility.
- A client cannot send an operation unless both peers advertised support during the bound handshake and the local UI/CLI implementation supports the resulting schema. A body field cannot grant a capability absent from connection state.
- Rolling package update supports only the explicitly tested adjacent overlap window; no indefinite compatibility promise.
- Stored database/export schemas are separate from IPC Protobuf messages. Generated IPC types never become domain/storage models directly; conversion validates all invariants.
- Wire-safe schema changes receive compatibility fixtures; wire-unsafe changes require a protocol-major change and migration/update plan.

## Rust prototype evidence — 2026-08-04

Phase 3D provides evidence for the Rust-side framing/schema candidate without accepting the final transport or resident dependency:

- `pastral-ipc-core` implements the exact 36-byte header, field-by-field little-endian parsing, bounded incremental byte-stream decoding, handshake/in-flight/bulk state, and serializer-neutral validated DTOs with no unsafe code, Windows binding, serializer, async runtime, or I/O dependency.
- `pastral-ipc-schema` generates Edition 2024 Rust bindings from `protocols/ipc-schema/pastral_ipc_v1.proto` using exact `protoc 35.0`, `protobuf 4.35.0-release`, and `protobuf-codegen 4.35.0-release`.
- The schema SHA-256 is `409c0da02f90e70e9bb1acbf1d7818d31ffcee3b61480cfa4ab250a5a8f493d8`; generated output remains in Cargo `OUT_DIR` and is not tracked.
- Official generated bindings contain the expected upb/native unsafe implementation and generator-specific Clippy style findings. The repository permits those only inside the generated module; handwritten schema conversion remains `deny(unsafe_code)`, and `pastral-ipc-core` remains `forbid(unsafe_code)`.
- Thirty framing/decoder/connection/DTO tests and eleven schema round-trip/adversarial tests pass. Coverage includes every header split, representative body splits, one-byte feeds, coalesced frames, poison/truncation, correlation and bulk ordering, missing oneofs, malformed wire data, zero/unknown enums, and all current semantic bounds.
- The isolated Release probe completes 10,000 of 10,000 deterministic round trips. A representative final run measured a 379,904-byte executable, 129,576 ns average full round trip, 713 ns one-byte decoder component, 583 ns coalesced decoder component, and 7,869-byte maximum body capacity for the synthetic 100-item response.
- These measurements are machine-specific prototype evidence, not a product SLA. The current 2,137,088-byte Release agent remains protobuf-free, so resident-agent incremental binary/private-working-set impact is deliberately not yet claimed.
- Dependency policy proves official Protobuf packages are isolated to `pastral-ipc-schema` and `pastral-ipc-probe`; agent, clipboard, domain, storage, agent-core, and ipc-core remain protobuf-free.
- CI is configured to retrieve the official `protoc-35.0-win64.zip` asset, verify SHA-256 `d1cede9e308cc3eb072392af1c02ccae4bdd3d2f374ec2970dbd8cdfdaa91363`, and expose exact `libprotoc 35.0` before locked workspace gates. Hosted execution remains unproven until GitHub Actions runs the workflow.

This evidence advances the official Rust runtime from an untested candidate to the selected Rust prototype runtime only. It does not accept ADR 0018 because C++ wire/runtime parity, authenticated named-pipe transport, fuzzing, staging cleanup, resident memory attribution, and adjacent-version compatibility remain incomplete.

## Acceptance gates

Change this ADR to Accepted only after a Windows x64 release prototype demonstrates:

1. exact Rust/C++ round-trip and malformed-input behavior for the same schemas;
2. reproducible generated artifacts/schema hashes from the pinned toolchain;
3. no unsupported generator/generated-code/runtime skew;
4. byte-mode frame parser tests/fuzzing cover every header/body fragmentation point, coalesced multiple frames, short reads/writes, disconnects, and independence from `WriteFile` boundaries; selected Protobuf parser, post-parse validator, and DTO conversion fuzzing enforce recursion/total-byte/peak-allocation limits;
5. resident-agent binary/private-working-set impact attributed and accepted against the 25 MB target;
6. control parse/serialize latency and allocation results at limit boundaries;
7. MSBuild/Cargo/CI integration without unreviewed tool downloads or stale generated code;
8. dependency license/advisory/supply-chain review;
9. bulk gap/duplicate/reorder/sequence-overflow/cancel/disconnect/staging-cleanup tests;
10. version/capability/unknown-field/unknown-enum/security-default compatibility tests.

Failure of the runtime footprint/build gates does not invalidate bounded framing/schema requirements; it triggers a measured runtime or schema alternative decision.

## Consequences

Positive:

- one schema authority for Rust and C++;
- mature binary evolution model with reserved fields and unknown-field handling;
- bounded pre-parse framing and separate large-payload path;
- no HTTP/gRPC/server runtime in the resident agent;
- protocol/domain separation prevents parser defaults from becoming authorization;
- exact codegen/runtime pinning makes compatibility testable;
- an explicit acceptance gate protects the resident footprint and build-reproducibility budgets.

Costs:

- Protocol Buffers compiler/runtime/code generation become dependencies;
- the selected C++ and Rust generators/runtimes must be exact-version pinned and their footprint measured;
- a custom 36-byte frame parser/state machine remains security-sensitive code;
- generated DTO/domain conversion code and compatibility fixtures are required;
- official Rust kernels can add native build/runtime complexity;
- lite/runtime support and Cargo/MSBuild integration need bootstrap evidence;
- final resident runtime choice remains open until measurement.

## Alternatives considered

- **JSON/UTF-8 framing:** easy to inspect but larger, slower to validate, ambiguous around numbers/presence/unknown fields, and prone to accidental payload/log exposure.
- **FlatBuffers:** strong zero-copy properties but more complex verifier/lifetime/schema-evolution semantics than needed for bounded control messages; payloads are streamed separately anyway.
- **CBOR/MessagePack with generic serializers:** compact, but schema/unknown/action compatibility would be project-defined and easier for Rust/C++ implementations to diverge.
- **Custom TLV for all messages:** smallest possible runtime but would make Pastral own schema compiler/evolution/unknown-field behavior and increase parser risk.
- **Wire-compatible Rust generator/runtime such as `prost`:** credible prototype candidate only after current maintenance, Editions/feature support, security, license, and footprint are verified; do not select or reject it from stale reputation.
- **gRPC/loopback HTTP:** unnecessary service/runtime/network-shaped surface for local named-pipe IPC.
- **Put bulk bytes inside Protobuf `bytes`:** rejected because it encourages large whole-message allocations/copies and mixes control authorization with content transport.

## Review triggers

- official Rust/C++ Protobuf runtime footprint causes the resident agent to miss its accepted budget;
- exact-version toolchain integration cannot be made reproducible in Cargo + MSBuild;
- Protobuf or generated-runtime security advisory;
- protocol needs shared memory or zero-copy transfer after measured evidence;
- a fifth language/client is proposed;
- Edition migration or schema feature is needed;
- control limit/throughput or frame-parser fuzzing reveals a design flaw;
- bulk transfer confidentiality requires a stronger same-user/user-presence model.
