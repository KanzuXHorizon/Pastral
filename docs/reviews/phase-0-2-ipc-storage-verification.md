# Phase 0.2 IPC and storage refinement verification

**Date:** 2026-08-04
**Base commit:** `2aec1c7 docs: harden Pastral architecture assumptions`
**Scope:** Remaining IPC schema/framing/runtime-selection debt, clipboard-platform STA terminology, replay callback ownership, payload-store abstraction, benchmark/test/release gates, and source/build scope.

## 1. Purpose

Phase 0.1 corrected the major Windows clipboard, privacy, security, data-identity, source-confidence, Quick Paste, packaging, and performance assumptions. A final adversarial pass found two implementation-splitting decisions still insufficiently closed:

1. IPC had security and versioning requirements but no normative binary frame/schema/bulk-transfer contract or resident-runtime acceptance gate.
2. Raw payload storage could be interpreted as either event-row inline data, an unconditional file-per-payload design, or a backend-neutral blob contract without clear identity/migration semantics.

Phase 0.2 closes those gaps without creating Rust, C++, WinUI, database, package, installer, or executable code.

## 2. Decisions/refinements verified

### IPC contract

- ADR 0018 is **Proposed**, not Accepted, because resident runtime cost/build/security evidence does not exist yet.
- The prototype contract uses Protobuf Edition 2024 control schemas and a fixed 36-byte little-endian Pastral frame.
- The frame includes an explicit 32-bit sequence so bulk duplicate/gap/reorder can be validated independently from UUID correlation.
- The frame-header UUID is the sole request/response/event correlation authority; control bodies do not duplicate it.
- HELLO binds negotiated protocol/capabilities to connection state, and body fields cannot self-grant capabilities.
- Control/hello/error bodies are capped at 256 KiB.
- Raw bulk chunks are capped at 1 MiB, with one active bulk transfer per connection initially.
- Initial request backpressure is 16 in-flight requests per connection and 64 globally.
- Large clipboard/export payloads never appear in normal Protobuf control messages.
- Parsing, DTO conversion, capability negotiation, peer validation, operation authorization, and user-intent authorization are separate gates.
- Edition fields use explicit presence and permanent field-number/name reservation on deletion.
- Security-critical zero/unknown enums, unknown actions, missing presence, duplicate keyed records, invalid frame state, and unnegotiated capabilities fail closed.
- The current release-train prototype candidate is Protocol Buffers v35.0 and must be revalidated at prototype time.
- The official Rust kernel path and at least one credible actively maintained wire-compatible Rust alternative must be measured for binary size, private working set, startup, build complexity, maintenance, and security before ADR 0018 can be accepted.
- No gRPC, loopback HTTP, Tokio solely for IPC, reflection, `Any`, TextFormat, ProtoJSON, extensions, or unbounded control-payload allocation is permitted in the resident core.

### Clipboard apartment ownership

- The normative term is `clipboard-platform STA`, not `capture STA`.
- The STA owns foreign Win32/OLE capture objects/media and Pastral replay `IDataObject` publication/lifetime.
- The control/overlay thread neither invokes foreign clipboard/OLE methods nor serves replay callbacks.
- Replay callbacks use only prevalidated Pastral-owned memory or immutable pre-opened resources and never synchronously call SQLite, IPC, rules, profiles, or UI.
- A blocked clipboard-platform STA degrades capture and paste availability but does not freeze tray, hotkey, overlay, session, or supervision work.

### Payload storage

- Event/representation metadata rows do not duplicate captured/derived payload bytes.
- Every payload is addressed through one versioned `BlobStore` contract.
- Physical storage may be an internal SQLite BLOB row or external staged/final file according to the benchmark-selected/versioned storage policy.
- `sha256-raw-v1` identifies logical raw ordinary bytes independently of physical backend/encoding.
- Protection domain, format, adapter, raw length, physical backend, and storage-policy version remain separate metadata.
- Private/sensitive plaintext uses random identity, no persistent plaintext digest, and no default plaintext deduplication.
- Backend migration must be transactional and preserve identity, reference counts, protection, and recovery invariants.
- `blob-store-lifecycle.md` defines internal/external commit order, external staging/finalization, recovery reconciliation, resumable migration switching, deletion/retention, and low-disk hysteresis without hard-coding an unmeasured backend threshold.

### Native build authority

- The manager uses the supported Visual Studio C++ WinUI `.vcxproj` + MSBuild/NuGet/XAML path.
- The multi-executable MSIX uses a separate `.wapproj`.
- `Pastral.slnx` is the planned solution format subject to verified manager/packaging tool compatibility; a legacy `.sln` fallback requires recorded evidence.
- Experimental Windows App SDK CMake integration is not a release dependency.
- The pure-domain bootstrap does not create empty CMake, WinUI, packaging, vcpkg, or Protobuf scaffolds.

## 3. Verification commands and results

### Whitespace and repository scope

Commands:

```bash
git diff --check
find . -path './.git' -prune -o -type f \
  \( -name '*.rs' -o -name '*.cpp' -o -name '*.cxx' -o -name '*.h' \
     -o -name '*.hpp' -o -name '*.vcxproj' -o -name '*.wapproj' \
     -o -name 'Cargo.toml' -o -name 'CMakeLists.txt' \) -print
git check-ignore -v Start-DevSpace-MCP-Cloudflared.ps1
```

Result:

- no whitespace errors;
- no feature/build scaffold exists;
- the local DevSpace launcher remains ignored at repository root.

### Normative contradiction checks

Checked current normative documents for:

- unresolved `TBD`/`TODO`/`FIXME`/placeholder language;
- unsupported `production-ready`, universal-losslessness, or guaranteed-security claims;
- stale Windows SDK `10.0.28000.2270` outside explicit historical correction records;
- obsolete 32-byte/proto3 IPC contracts;
- wording that treats ADR 0018/runtime choice as Accepted;
- current use of `capture STA`/`capture apartment` instead of `clipboard-platform STA`;
- hard 5 GB quota wording;
- optional/underspecified sensitive-skip default;
- durable source-owned hard-deny records;
- same-user IPC/DPAPI confidentiality overclaims;
- persisted runtime registered-format numeric IDs;
- zero-representation `ClipEvent` wording;
- complete-history claims for rapid clipboard replacement;
- stable CMake authority for WinUI.

Result: no active competing normative contract remained. Historical audit/spec/plan statements remain only where explicitly labeled as prior findings or superseded evidence.

### Markdown links and structure

Read-only repository checks validated 71 Markdown files:

- every local Markdown link resolves within the repository;
- every fenced code block closes;
- no duplicate same-level heading exists within the same parent heading path;
- all 18 ADRs are contiguous from 0001 through 0018;
- every ADR has Status, Date, Context, Decision, Consequences, Alternatives considered, and Review triggers;
- every ADR is linked from `docs/adr/README.md`.

A first duplicate-heading checker incorrectly treated repeated `Strengths`/`Lesson` headings under different competitor sections as collisions. The checker was corrected to include the parent heading hierarchy; the documents were not distorted to satisfy an invalid rule.

### Secret-signature scan

A first `git grep` attempt was invalid because a pattern beginning with `-----BEGIN` was parsed as an option. It is not counted as evidence. The scan was rerun correctly with explicit `-e` and checked tracked plus untracked text for private-key headers, AWS-style access keys, GitHub tokens, and common Stripe-like test/live secret prefixes.

Result: no secret-like signature matched outside intentionally excluded policy/example documents.

### Toolchain and policy consistency

Confirmed current normative references for:

- Rust 1.97.1 / Rust Edition 2024;
- Windows SDK `10.0.28000.2526` with runtime floor still Windows 11 build 26100;
- Windows App SDK 2.3.1 stable;
- Protobuf Edition 2024 / v35.0 prototype candidate with ADR 0018 still Proposed;
- 24-hour hidden sensitive-skip audit default;
- no durable source-owned hard-deny record;
- 5 GB automatic-cleanup target, not hard cap;
- random/no-plaintext-digest Private/sensitive storage;
- Private profile unavailable before encryption/non-indexing/lock/recovery gates.

## 4. False-positive and historical-evidence handling

Verification deliberately distinguishes current normative contracts from historical records:

- the obsolete Windows SDK pin remains quoted in the adversarial finding and execution-plan correction instruction;
- the original Phase 0 consistency review retains commands that searched for placeholders;
- old `capture STA` or insufficient-contract language may remain inside explicit historical findings explaining what was corrected;
- ADR 0018 framing/schema is approved for prototype design, while the runtime implementation is intentionally unaccepted pending evidence.

These are not active contradictions and must not be removed merely to make naive global grep output empty.

## 5. What this verification does not prove

No executable or package exists. Therefore this review does not prove:

- clipboard capture/replay correctness;
- Protobuf runtime compatibility or resident footprint;
- Windows build/toolchain availability on the target machine;
- SQLite internal/external blob threshold;
- overlay focus safety;
- paste destination consumption;
- security containment against real malformed inputs;
- benchmark budgets;
- accessibility behavior;
- install/update/uninstall/signing behavior.

Those claims remain blocked on implementation, fixtures, tests, benchmarks, and signed-package evidence.

## 6. Remaining implementation gates

Before accepting ADR 0018 or implementing production IPC:

1. build the 36-byte frame parser and state machine with red/green unit/fuzz tests;
2. compile identical Edition 2024 schemas for C++ and each Rust runtime candidate;
3. produce cross-language golden vectors and exact compatibility checks;
4. measure agent binary/private-working-set/startup/idle impact and build complexity;
5. test all control/bulk bounds, sequencing, cancellation, disconnect, low-disk, and authorization paths;
6. amend ADR 0018 with the measured runtime/toolchain decision.

Before selecting the final BlobStore backend threshold:

1. benchmark internal SQLite BLOB rows versus external files on supported Windows hardware with Defender enabled;
2. test crash/recovery, backup/export, low disk, retention, deletion remnants, migration, and 100k–1M metadata distributions;
3. record threshold/backend policy, journal mode, synchronization, vacuum, and recovery evidence in ADR 0006 or an explicit amendment.

## 7. Readiness conclusion

The documentation is ready to be committed as a distinct Phase 0.2 refinement on top of `2aec1c7`. It closes the schema/framing and storage-contract ambiguity while preserving evidence gates. It does not authorize claiming that Pastral is implemented, performant, secure, lossless, accessible, packaged, or production-ready.
