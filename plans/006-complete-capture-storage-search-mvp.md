# Plan 006: Complete capture, storage, search, retention, and recovery MVP

> **Executor instructions:** This is a program plan. Execute it as independently reviewed vertical slices in the listed order, one representation/policy capability at a time. Use an isolated worktree. Never expose real user clipboard content in automated tests, logs, screenshots, or failure output.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- crates/domain crates/clipboard-win crates/agent-core crates/storage apps/agent protocols eng docs/architecture docs/security docs/performance docs/testing docs/adr`

## Status

- **Priority:** P1
- **Effort:** XL
- **Risk:** HIGH
- **Depends on:** Plans 001 and 005
- **Category:** correctness, storage, privacy, performance, product direction
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

Pastral’s durable advantage is one immutable clip event with multiple preserved representations, provenance and honest fidelity. The current resident path captures ordinary `CF_UNICODETEXT` and exposes bounded text previews. V1 requires reliable common Windows formats, typed structured search, retention/quota/low-disk behavior, integrity/reconciliation, backup/restore and recovery without weakening privacy or making the manager a second storage owner.

## Current state

- Domain/storage foundations already model events, representations, fidelity, protection domains, raw-byte digests, capture order, internal/external blobs, FTS mapping and integrity.
- `clipboard-win` safely enumerates formats and captures bounded HGLOBAL Unicode text; source/history controls and owner process observation exist.
- Agent coordinator uses bounded retry and privacy admission before persistence.
- Storage has 37 focused tests for dedup, external/internal blobs, FTS, cursors, integrity, missing/corrupt payloads and protected-payload rejection.
- Read IPC exposes unavailable/text rows with bounded 4,096-byte privacy-safe previews and first-page pagination.
- Missing V1 behavior includes ANSI/OEM/locale fallback, HTML Clipboard Format, RTF, URLs, file lists, images, richer OLE media, multi-representation event commits, structured filters, retention/quota/low disk, long-lived maintenance scheduling, backup/export/import and user-facing recovery.
- Private/sensitive payloads must remain rejected until Plan 008’s encryption gate.

## Architecture

Preserve a three-stage boundary:

1. Clipboard-platform STA acquires foreign Win32/OLE media into bounded owned representation candidates.
2. Agent-core applies source/privacy/sensitive/representation policy and constructs one immutable logical event.
3. Storage transaction validates and commits metadata/search projection/blob references atomically, then performs bounded reconciliation/maintenance.

Every adapter owns format-specific validation and fidelity semantics. No runtime registered-format ID is persisted; registered formats use exact stable names. Search remains parameterized and typed; plain text remains useful without syntax.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Domain/storage | `cargo test --locked -p pastral-domain -p pastral-storage --all-targets` | PASS |
| Clipboard/agent | `cargo test --locked -p pastral-clipboard-win -p pastral-agent-core -p pastral-agent --all-targets --all-features` | PASS |
| Source/dependencies | PowerShell policy gates | PASS |
| Release benchmarks | repository benchmark scripts introduced by slices | budgets reported, no hidden skip |
| Full baseline | `powershell -File eng/build.ps1 -Task Full` | exit 0 |

## Scope

**In scope**

- common format acquisition/adapters and immutable multi-representation commits
- source confidence/privacy controls and self-update suppression
- typed lexical/structured search DTO/domain/storage queries
- retention, quota, low-disk and maintenance policy
- backup/restore/export/import foundations for ordinary data
- crash recovery, reconciliation, migration and scale evidence
- agent IPC/CLI operations needed to expose these capabilities safely

**Out of scope**

- Paste/replay and destination input (Plan 007).
- Encrypted Private/sensitive storage (Plan 008).
- OCR, semantic search, cloud sync, browser extension, arbitrary plugins.
- Universal support for application-private formats.
- Direct manager storage access.

## Program slices

### Slice 1: Define the representation-adapter contract

Add a stable adapter interface that consumes an already-open clipboard read session on the clipboard-platform STA and returns bounded owned candidates with:

- stable format identity;
- exact owned bytes or validated structured metadata;
- media/ownership kind;
- original/derived status;
- fidelity evidence and loss reason;
- safe preview/search projection eligibility;
- size and protection policy.

Adapters must distinguish unsupported, unavailable/transient, malformed, too large and captured. Preserve format enumeration order as evidence, not a universal paste priority.

Write contract tests with synthetic producers for zero/empty, malformed length, delayed owner exit, oversized media, multiple formats and unknown registered names.

**Verify:** domain/clipboard tests demonstrate one event can carry multiple exact candidates without merging occurrences.

### Slice 2: Add text family adapters

Implement, in order:

1. `CF_UNICODETEXT` existing path through the new adapter contract.
2. ANSI/OEM text plus `CF_LOCALE` conversion fallback with exact original bytes retained and Unicode derived representation labeled honestly.
3. HTML Clipboard Format parser: validate header offsets/ranges/UTF-8, preserve original bytes, extract bounded fragment/source URL only when valid and privacy-permitted.
4. RTF: preserve bounded original bytes; generate plain search preview only through a non-executing bounded parser or mark preview unavailable.
5. URL registered formats and Unicode/plain representations with canonical type metadata but no destructive URL normalization.

Tests must include malformed offsets, embedded NUL, invalid encodings, large payloads, line endings, empty content and multiple simultaneous text representations.

**Verify:** exact-byte round trips and search projections pass; malformed media never partially commits.

### Slice 3: Add file-list and image adapters

Implement:

- `CF_HDROP` file references with canonical bounded path parsing, no automatic file open/read, unavailable-state revalidation and privacy-safe display names;
- DIB/DIBV5/bitmap and common encoded image representations with strict dimensions/stride/size/overflow checks;
- bounded thumbnail/preview derivation in an isolated decode path, never on the listener thread;
- optional virtual-file acquisition only after a separate bounded stream/lifetime test slice.

Use fixture producers/consumers and malformed corpora. Do not persist HANDLE, HBITMAP, COM pointer or runtime format ID.

**Verify:** common file/image fixtures create one event with correct representation/fidelity set and bounded memory peaks.

### Slice 4: Harden OLE/COM acquisition ownership

Move all foreign `IDataObject`, `FORMATETC`, `STGMEDIUM`, delayed rendering and stream operations to the dedicated clipboard-platform STA described by ADR 0015. Define cancellation/timeouts and ownership transfer for HGLOBAL, stream, bitmap and file media. A blocked/re-entrant owner must not freeze hotkeys, overlay, tray, IPC or manager.

Add synthetic hostile owners for delay, re-entry, refusal to cancel, malformed medium, owner exit and device loss. Keep the control thread responsive and queue bounded.

**Verify:** fixture suite proves cleanup of every acquired medium and no post-timeout use-after-release.

### Slice 5: Commit one logical multi-representation event atomically

Extend agent/storage commit input so all admitted originals/derived projections share one `ClipEventId`, occurrence/source/profile context and protection domain. Validate:

- at least one usable representation;
- exact digest/domain consistency;
- originals are immutable;
- derived provenance/version is explicit;
- no cross-domain dedup;
- capture order assigned once transactionally;
- a failed representation prevents or quarantines the whole commit according to documented policy, never leaves a misleading partial event.

Search projection is explicit and contains only permitted bounded text. Sensitive/source-hard-deny rules run before blob/digest/index creation.

**Verify:** restart/crash tests around every commit stage; all event/representation/blob invariants pass.

### Slice 6: Add typed structured search

Define a typed query AST shared by Quick Paste and Manager provider contracts, supporting only implemented fields:

- plain terms/exact phrases;
- `type`, `app/source`, `profile`, `before/after`, `has`, `fidelity`, `pinned`, `ext`, tags/collections when those services land.

Parser returns valid terms plus explicit editable diagnostics for invalid filters; never concatenates raw SQL/FTS. Storage compiles parameterized queries with stable newest-first ordering and cursor pagination. Plain literal behavior remains backward compatible.

Add query complexity/term/byte bounds, cancellation and explainable normalized query output.

**Verify:** golden parser/query fixtures, injection-shaped input treated as data, 1k/10k/100k correctness and latency evidence.

### Slice 7: Implement retention, quota and low-disk policy

Add versioned policy with product defaults:

- 90-day ordinary unpinned retention;
- 5 GB automatic-cleanup target, not a hard cap;
- pinned/protected exemptions with warnings;
- measured low-disk reserve that pauses new payload capture while copy behavior continues;
- bounded incremental/cancellable cleanup that yields to capture;
- content-free audit/health state and simulation before manual destructive cleanup.

Use storage-owned maintenance leases/transactions; do not run periodic full scans or busy polling. Sensitive-skip audit defaults to 24-hour content-free retention.

**Verify:** fake clock/disk tests, pinned exemption, shared-blob references, low-disk recovery, cancellation/restart and no silent data loss.

### Slice 8: Add backup, restore, export/import and recovery foundations

Define versioned ordinary-data archive semantics separate from IPC DTOs. Include metadata, both blob backends, representation/provenance, policy version and integrity manifest. Exclude sensitive/private data until its encrypted export contract exists. Validate paths, sizes, reparse points, duplicate IDs, corruption, unsupported/newer versions and interrupted operations.

Restore into staging, verify fully, remap installation-local capture order/IDs when required, then commit atomically. Expose integrity/quarantine/reconciliation and clear recovery outcomes through agent provider APIs.

**Verify:** round trips for internal/external blobs, corruption/truncation/path traversal rejection, interrupted restore cleanup and no overwrite of healthy data without explicit authorization.

### Slice 9: Add lifecycle scheduling and observability

Run maintenance on explicit triggers (startup recovery, low-disk event, scheduled bounded interval, user request) with one owner and cancellation. Health/diagnostics expose content-free timings/counts/result codes. No payload, query, path or preview enters logs by default.

**Verify:** idle soak has no clipboard polling/full scan/network activity; maintenance cannot starve capture/IPC.

### Slice 10: Integrate IPC/CLI and end-to-end fixtures

Add versioned capabilities/operations for structured search, paging, integrity, retention status and bounded ordinary export/backup control. Bulk payload transport follows ADR 0018’s staged chunk contract rather than Protobuf `bytes`. Authorize destructive/export operations with explicit user intent, not connection alone.

Build end-to-end fixture: capture multiple representations → restart → search/filter → inspect metadata → retention simulation → backup/restore. Paste remains disabled until Plan 007.

**Verify:** exact capability negotiation, unauthorized request rejection, adjacent-version fixtures and content-free diagnostics.

## Test plan

- Standard/malformed/oversized text, HTML, RTF, URL, file and image fixtures.
- Multi-format ownership, owner exit, delayed rendering, timeout, re-entry and cleanup.
- Hard deny/sensitive skip before any durable bytes/digest/index.
- Atomic multi-representation commit and crash points.
- Search parser/query bounds, literal behavior, ordering and pagination.
- 1k/10k/100k search and history performance.
- retention/quota/pinned/low-disk/cleanup cancellation and restart.
- backup/restore/import corruption/path/version/interrupt cases.
- idle and combined capture/IPC/maintenance footprint.

## Done criteria

- [ ] Required common formats create honest multi-representation events with exact originals.
- [ ] Malformed/hostile clipboard owners are contained without blocking control surfaces.
- [ ] Source/privacy/sensitive policy runs before durable content creation.
- [ ] Structured search is typed, parameterized, bounded and shared by Manager/Quick Paste contracts.
- [ ] 90-day/5 GB/pinned/low-disk policies are enforced and explainable.
- [ ] Integrity, reconciliation, backup and restore pass crash/corruption tests.
- [ ] 100k ordinary history/search meets accepted budgets on recorded hardware.
- [ ] No direct manager storage access or periodic clipboard polling exists.
- [ ] Full baseline and end-to-end ordinary capture/restart/search/recovery fixture pass.

## STOP conditions

Stop and review when:

- A format requires executing/rendering untrusted active content.
- A proposed adapter cannot define exact ownership/lifetime/bounds.
- Retention or recovery could delete pinned/protected data silently.
- Search implementation requires interpolated SQL/FTS or leaks hidden content.
- A bulk/export path lacks operation authorization and staging cleanup.
- A protected payload would be persisted before Plan 008 encryption gates.
- A slice exceeds resident responsiveness/footprint budgets without new evidence and ADR review.

## Maintenance notes

Add formats through isolated adapters and fixture evidence, not conditionals in the resident loop. Any new persisted representation/search field requires schema migration, export/import versioning, privacy review and replay compatibility review.
