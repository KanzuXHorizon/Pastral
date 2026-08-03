# Phase 0.2 IPC and storage refinement verification

**Date:** 2026-08-04
**Parent commit:** `2aec1c7 docs: harden Pastral architecture assumptions`
**Phase 0.2 commit:** `3048d31 docs: define bounded IPC and blob contracts`
**Verification target:** commit `3048d31` plus a six-file verification amendment: five byte-mode regression contract files and this evidence-report update.
**Scope:** IPC schema/framing/runtime-selection constraints, named-pipe byte-mode behavior, clipboard-platform STA terminology, replay ownership, backend-neutral payload storage, benchmark/test/release gates, and documentation-only repository scope.

## 1. Purpose

Phase 0.1 corrected the major Windows clipboard, privacy, security, data-identity, source-confidence, Quick Paste, packaging, and performance assumptions. Phase 0.2 closes two remaining implementation-splitting ambiguities without creating Rust, C++, WinUI, database, package, installer, or executable code:

1. IPC had security/versioning requirements but no normative bounded frame, schema, bulk-transfer, parser-allocation, or resident-runtime acceptance contract.
2. Raw payload storage could be interpreted as either an unconditional file-per-payload design or an all-in-SQLite design without a benchmark-selected crossover, transactional migration, and backend-independent identity contract.

A post-commit verification pass also found that named-pipe byte mode was specified but not yet represented by explicit fragmentation/coalescing acceptance tests. The amendment adds those gates to ADR 0018, the IPC architecture specification, compatibility matrix, test strategy, and release checklist.

## 2. Decisions verified

### IPC contract

- ADR 0018 is **Proposed**, not Accepted. Its framing/schema rules are approved for prototype, while the resident runtime requires Windows x64 footprint/build/security evidence.
- Named pipes use `PIPE_TYPE_BYTE`/`PIPE_READMODE_BYTE`; Pastral never relies on Windows message or `WriteFile` boundaries.
- The exact application header is 36 bytes and contains magic, framing major, frame kind, flags, body length, explicit frame sequence, and correlation/transfer UUID.
- The frame sequence is zero for non-bulk frames, a zero-based chunk index for `BULK_CHUNK`, and the accepted chunk count for `BULK_END_PROTO`.
- `HELLO_PROTO` binds protocol/capabilities to connection state. Body fields cannot self-grant capabilities.
- The frame-header UUID is the sole request/response/event correlation authority. Control bodies do not duplicate it.
- Control/hello/error bodies are capped at 256 KiB; bulk chunks at 1 MiB; initial limits are 16 in-flight control requests per connection, 64 globally, and one active bulk stream per connection.
- Header validation bounds the body buffer before parse. Generated Protobuf parser recursion/total-byte controls are enabled where supported, and peak internal allocation must be measured/fuzzed separately.
- Schemas use Protobuf Edition 2024 with explicit presence, reserved deleted numbers/names, zero `UNSPECIFIED` enum values, known operation `oneof`, and post-parse domain/authorization validation.
- Protocol Buffers v35.0 is the current release-train prototype candidate, not an accepted resident dependency.
- The official Rust kernel/Cargo path and at least one credible actively maintained wire-compatible Rust alternative must be compared for binary size, private working set, startup, parse/allocation behavior, native build complexity, reproducibility, maintenance, security, and license.
- No gRPC, loopback HTTP, Tokio solely for IPC, reflection, `Any`, TextFormat, ProtoJSON, extensions, or large clipboard payload inside ordinary control messages is permitted.

### Byte-mode regression amendment

Acceptance evidence must cover:

- every split position in the 36-byte header and control/bulk body;
- one byte per read;
- short reads and short writes;
- multiple complete frames coalesced in one read;
- complete plus partial next frame in one read;
- disconnect at every header/body boundary;
- parser independence from `WriteFile` and Windows message boundaries;
- interaction with body limits, cancellation, bulk sequence, backpressure, and connection state.

### Payload storage

- Event/representation metadata rows do not duplicate payload bytes.
- Every payload is addressed through one versioned content-addressed `BlobStore` contract.
- Physical storage may be an internal SQLite BLOB row or an external staged/final file according to a benchmark-selected and versioned policy.
- `sha256-raw-v1` identifies exact logical ordinary bytes independently of physical backend.
- Protection domain, format, adapter, logical length, physical locator, and storage-policy version remain distinct metadata.
- Private/sensitive plaintext uses random identity, no persistent plaintext digest, and no default plaintext deduplication.
- Backend migration copies and verifies a candidate, transactionally switches the authoritative locator, and removes the old location only after reader/recovery safety.
- Threshold selection requires supported-Windows measurements with realistic payload distributions, 100k–1M histories, Defender/antivirus, warm/cold cache, durable writes, reads, preview/search, backup/export, cleanup, low disk, crash injection, and migration.

### Native build authority

- The manager uses supported Visual Studio C++ WinUI `.vcxproj` + MSBuild/NuGet/XAML tooling.
- The four-executable MSIX uses a separate `.wapproj`.
- `Pastral.slnx` is the planned solution format; a legacy `.sln` fallback requires recorded tooling evidence.
- Experimental Windows App SDK CMake integration is not a release dependency.
- The pure-domain bootstrap creates no empty WinUI, packaging, CMake, vcpkg, or Protobuf scaffold.

## 3. Fresh verification evidence

### Repository baseline and object integrity

Commands:

```bash
git rev-parse HEAD
git branch --show-current
git fsck --no-progress --no-dangling
git show --check --oneline --no-renames HEAD
```

Observed result:

- baseline HEAD was `3048d31f87dc2ea9e61a034cc789a500e812af2a` on `main`;
- reachable-object integrity check passed;
- committed whitespace check passed.

Ordinary unreachable blobs created by prior index/edit churn were intentionally excluded with `--no-dangling`; they are not reachable corruption and are not product artifacts.

### Markdown links, code fences, and heading hierarchy

A read-only Python checker:

- enumerated every repository Markdown file outside `.git`;
- resolved every local Markdown link within the repository root;
- required balanced fenced-code markers;
- detected duplicate headings only when the same text/level occurred under the same parent-heading path.

Observed result:

- **71 Markdown files** checked;
- local links: PASS;
- code fences: PASS;
- heading hierarchy: PASS.

### ADR structure and audit coverage

Checks required:

- contiguous ADR filenames 0001–0018;
- `Status`, `Date`, `Context`, `Decision`, `Consequences`, `Alternatives considered`, and `Review triggers` in every ADR;
- every ADR linked from `docs/adr/README.md`;
- ADR 0018 status contains `Proposed`;
- adversarial findings are unique and contiguous F-01–F-22;
- the Phase 0.1/0.2 execution plan has no unchecked `Step` checkbox.

Observed result:

- **18 ADRs**, contiguous 0001–0018: PASS;
- ADR structure/index: PASS;
- ADR 0018 remains Proposed: PASS;
- **22 findings**, contiguous F-01–F-22: PASS;
- plan completion: PASS.

### Normative contradiction and placeholder scan

Current normative documents were checked for:

- obsolete 32-byte/proto3 contracts;
- control-body 1 MiB or per-connection 64-request limits;
- wording that treats ADR 0018/runtime choice as Accepted;
- duplicated correlation/capability authority;
- universal external-file or universal SQLite payload backend claims;
- zero-representation `ClipEvent`;
- hard 5 GB quota;
- broad user-SID + logon-SID pipe access wording;
- persisted runtime registered-format IDs;
- stale `sha256-v1` name;
- stable WinUI CMake or ambiguous `.slnx or .sln` authority;
- active `TBD`, `TODO`, `FIXME`, `XXX`, “implement later”, “fill in details”, or generic “appropriate error handling”.

Observed result: no active competing normative contract or unresolved implementation placeholder remained.

The obsolete Windows SDK `10.0.28000.2270` appears only in three explicitly historical/check records:

- this verification document;
- `phase-0-adversarial-audit.md`;
- the Phase 0.1 hardening execution plan.

Active bootstrap/research documents use the corrected SDK snapshot and require revalidation on the actual bootstrap date.

### Authority and terminology checks

Observed required terms/contracts:

- `PIPE_TYPE_BYTE`/`PIPE_READMODE_BYTE`;
- byte-mode fragmentation/coalescing tests;
- sole frame-header correlation authority;
- 256 KiB control ceiling;
- official Rust-kernel versus alternative runtime evidence gate;
- hybrid `BlobStore` with internal SQLite BLOB and external-file lifecycle;
- `clipboard-platform STA` in current normative architecture;
- no current `clipboard STA`, `capture STA`, or `capture apartment` terminology outside explicitly historical review text.

Result: PASS.

### Scope, secrets, whitespace, and local artifacts

Commands/checks:

```bash
git diff --check
git status --short
find . -path './.git' -prune -o -type f \
  \( -name '*.rs' -o -name '*.cpp' -o -name '*.cxx' -o -name '*.h' \
     -o -name '*.hpp' -o -name '*.xaml' -o -name '*.vcxproj' \
     -o -name '*.wapproj' -o -name 'Cargo.toml' \
     -o -name 'CMakeLists.txt' \) -print
git check-ignore -v Start-DevSpace-MCP-Cloudflared.ps1
```

Secret-pattern scan covered private-key headers, AWS-style access keys, GitHub token prefixes, and common Stripe-like secret prefixes.

Observed amendment scope before this report update:

- `docs/adr/0018-ipc-schema-and-framing.md`;
- `docs/architecture/ipc-schema-and-framing.md`;
- `docs/release/checklist.md`;
- `docs/testing/compatibility-matrix.md`;
- `docs/testing/strategy.md`.

Results:

- documentation-only scope: PASS;
- no Rust/C++/WinUI/build/package scaffold: PASS;
- secret-signature scan: PASS;
- whitespace: PASS;
- machine-local DevSpace launcher remains ignored;
- accidental `NUL` artifact is absent.

## 4. What this verification does not prove

No executable, schema compiler invocation, database, package, fixture, UI, or benchmark implementation exists. Therefore this review does not prove:

- clipboard capture/replay correctness;
- named-pipe parser behavior under real fragmentation/coalescing;
- Protobuf runtime compatibility, security, build reproducibility, or resident footprint;
- the SQLite/internal versus external-file threshold;
- crash recovery or backup consistency;
- overlay focus safety;
- paste destination consumption;
- security containment against malformed runtime inputs;
- benchmark budgets;
- accessibility behavior;
- install/update/uninstall/signing behavior.

Those claims remain blocked on implementation, fixtures, tests, benchmarks, and signed-package evidence.

## 5. Remaining implementation gates

Before accepting ADR 0018 or implementing release IPC:

1. write red/green tests for the 36-byte byte-mode frame parser, including every fragmentation/coalescing boundary;
2. compile identical Edition 2024 schemas for C++ and each Rust runtime candidate;
3. generate cross-language golden vectors and exact compatibility checks;
4. fuzz frame, selected Protobuf parser, post-parse validator, DTO conversion, and bulk state machine;
5. measure parser peak allocation plus agent binary/private-working-set/startup/idle impact and build complexity;
6. test all control/bulk bounds, sequence, cancellation, disconnect, low disk, backpressure, capability, correlation, peer/session, and authorization paths;
7. amend ADR 0018 with the measured runtime/toolchain decision.

Before selecting the final `BlobStore` threshold:

1. implement both physical backends behind one contract;
2. benchmark them on supported Windows hardware with Defender enabled;
3. test crash/recovery, deduplication, reference races, backup/export, low disk, deletion remnants, threshold migration, and 100k–1M metadata distributions;
4. record threshold policy, journal mode, synchronization, vacuum, recovery, and migration evidence in ADR 0006 or a dedicated amendment.

## 6. Readiness conclusion

The documentation contract is coherent enough to preserve Phase 0.2 as a design baseline and to proceed to a separate repository/toolchain/pure-domain bootstrap design cycle. ADR 0018 intentionally remains Proposed, and the physical `BlobStore` threshold intentionally remains unselected. This verification does not authorize claims that Pastral is implemented, performant, secure, lossless, accessible, packaged, or production-ready.
