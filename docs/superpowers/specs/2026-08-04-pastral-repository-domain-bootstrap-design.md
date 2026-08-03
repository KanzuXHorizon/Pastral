# Pastral Repository and Domain Bootstrap Design

**Status:** Approved for implementation
**Date:** 2026-08-04
**Baseline:** `main` at `d4bbb90`

## Purpose

Phase 1 establishes a reproducible Windows MSVC Rust workspace and a single platform-independent `pastral-domain` crate. The slice encodes durable identity, UTC microsecond time, installation-local capture ordering, clipboard-format identity, protection-domain digest policy, immutable capture records, fidelity aggregation, content-free audit records, and derived provenance without introducing Windows, storage, IPC, UI, async, filesystem, network, or serialization concerns.

## Boundaries and non-goals

The workspace contains only `crates/domain`, PowerShell verification/build scripts, a Windows Rust CI workflow, and documentation for this slice. It does not create placeholder directories or implement Win32 clipboard APIs, COM/OLE, SQLite/FTS5, BlobStore, named pipes, Protobuf, executables, WinUI/C++, packaging, logging backends, OCR, semantic search, AI, serde, CMake, vcpkg, or product network access. Windows SDK `10.0.28000.2526` remains a deferred native-slice prerequisite and is reported but not installed or required for this pure-domain build.

## Toolchain and workspace

`rust-toolchain.toml` pins Rust `1.97.1`, host and target `x86_64-pc-windows-msvc`, and the `rustfmt` and `clippy` components. The root Cargo workspace uses resolver `3`, Edition 2024, and `rust-version = "1.97.1"`. Dependency versions are centralized and locked. The only runtime dependencies are `uuid` for cryptographically random UUIDv4 identities and `sha2` for the explicit `sha256-raw-v1` digest suite. No `.cargo/config.toml` is needed because the pinned toolchain and explicit PowerShell verification provide the required behavior without repository-specific linker overrides.

`eng/verify-toolchain.ps1` is PowerShell 5.1-compatible, reports exact Rust tools, active toolchain, target, Visual Studio/MSVC evidence, and installed Windows SDKs. It fails required mismatches and classifies the future Windows SDK as `DeferredForNativeSlice`. `eng/build.ps1` dispatches `Verify`, `Format`, `Check`, `Test`, `Clippy`, `Doc`, or `All`, stops on the first failure, and preserves the failing exit code.

## File and module boundaries

- `src/id.rs`: opaque UUIDv4 typed identifiers and canonical textual parsing/display.
- `src/time.rs`: validated signed UTC Unix microseconds with no clock access.
- `src/capture_order.rs`: strictly positive deterministic local ordering value.
- `src/format_identity.rs`: standard numeric formats and validated registered names; no runtime registered numeric identity.
- `src/protection.rs`: protection domains and digest/dedup policy methods.
- `src/digest.rs`: `sha256-raw-v1`, fixed 32-byte digest, and protected persistent-digest construction.
- `src/fidelity.rs`: representation fidelity and versioned deterministic aggregation.
- `src/representation.rs`: immutable captured representation metadata with private fields.
- `src/clip_event.rs`: immutable event requiring at least one captured representation and preserving capture evidence.
- `src/audit_event.rs`: content-free audit type and hard-deny outcome modeling.
- `src/derived.rs`: immutable derived provenance referencing original parents.
- `src/error.rs`: small manual domain error enum using only `std`.
- `src/lib.rs`: crate policy and deliberate public exports.

## Public interfaces and invariants

Typed IDs (`ClipEventId`, `ClipRepresentationId`, `CaptureAuditEventId`, `DerivedRepresentationId`, `ProfileId`, `ProtectionDomainId`, `TransformationId`) expose `new_v4`, `from_uuid`, `parse_str`, `as_uuid`, and canonical `Display`. `from_uuid` rejects non-v4 UUIDs. IDs contain no ordering, time, source, authorization, or content meaning.

`UtcUnixMicros` supports `-62135596800000000..=253402300799999999`, corresponding to years 0001 through 9999 in the proleptic Gregorian range used by common storage/interop systems. It never reads system time. Timeout and monotonic duration semantics remain outside this type.

`CaptureOrder::new(u64)` rejects zero and provides total deterministic ordering. Allocation belongs to a future serialized storage owner.

`ClipboardFormatIdentity` is either `Standard(StandardFormatId)` or `Registered(RegisteredFormatName)`. Registered names must be non-empty and contain no NUL. The durable registered type cannot store a runtime numeric ID.

`ProtectionDomain` distinguishes `Ordinary(ProtectionDomainId)`, `Sensitive(ProtectionDomainId)`, and `Private(ProtectionDomainId)`. Persistent plaintext digest construction is permitted only for `Ordinary`; digest-based deduplication requires equal protection domains and is never cross-domain.

`RawDigest::sha256_raw_v1(domain, bytes)` hashes exact logical bytes without normalization or descriptors and rejects Sensitive/Private domains. `DigestSuite::Sha256RawV1` and the fixed 32-byte value make suite/version explicit.

`ClipRepresentation::new` stores an ID, stable format identity, protection domain, raw logical length, optional permitted digest, and fidelity. It validates that any digest belongs to the same protection domain. All fields are private and exposed only by shared-reference getters; there is no mutation API or payload/storage field.

`ClipEvent::new` requires an ID, observed UTC time, capture order, captured profile ID, captured protection evidence, and a non-empty vector of captured representations. Descriptors with `Unavailable` or `UnsafeOrUnsupported` fidelity are rejected as event members, so they cannot satisfy cardinality. Identical payloads never merge events because each event receives its own ID.

`CaptureAuditEvent` contains only typed identity, kind, coarse/allowed time, optional capture order, profile ID, and content-free result metadata. `CaptureOutcome::HardDenied` represents the source-owned hard-deny path without constructing an audit record. The type has no payload, preview, digest, path, or blob-reference fields.

Fidelity aggregation is `FidelityAggregationVersion::V1`. It rejects an empty set; `FullFidelity` requires all values full, `CommonFormatsPreserved` wins when any common/full usable representation exists without universal full fidelity, `FallbackOnly` wins over reference-only, `ReferenceOnly` applies when all usable representations are references, and unavailable/unsafe-only sets are rejected.

`DerivedRepresentationProvenance` stores its own ID, parent event and representation IDs, transformation ID, nonzero transformation version, output protection domain, and optional permitted output digest. Construction validates digest policy/domain and never receives mutable access to originals.

## Error handling

A compact `DomainError` enumerates invalid UUID version, invalid UTC range, zero capture order, invalid registered format name, forbidden plaintext digest, digest-domain mismatch, empty clip event, unusable captured representation, empty fidelity input, no usable fidelity, and zero transformation version. Constructors return `Result`; no panic is required for caller-controlled invalid input.

## Test strategy

Tests are colocated with modules and assert public observable contracts. Red evidence is captured by running focused tests before their implementations exist or before validation is added; intentionally failing states are never committed. Coverage includes UUID version/variant and canonical text, typed API boundaries, time range, order, format validation, exact-byte digest semantics, protection policy, nonempty event cardinality, immutable access patterns, separate event identity, content-free audit design, fidelity combinations, and derived parent preservation. Compile-fail infrastructure is omitted because distinct Rust types and function signatures already enforce ID separation without an extra dependency.

## Acceptance criteria

From Windows PowerShell with the pinned MSVC toolchain: toolchain verification, formatting, workspace check, all-target tests, warnings-denied Clippy, and documentation build pass; `cargo tree` contains only the reviewed UUID/SHA dependency graph and no forbidden platform/storage/async/serialization/network/UI dependencies; the final tree is clean; local commits document specification, toolchain, domain implementation, CI, and verification evidence; and the milestone report lists only commands freshly executed with honest skips and residual risks.
