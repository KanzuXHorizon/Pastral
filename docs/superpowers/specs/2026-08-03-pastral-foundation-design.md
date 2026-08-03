# Pastral Foundation Design

**Date:** 2026-08-03
**Status:** Historical Phase 0 baseline; amended for implementation readiness by ADR 0015–0017, the ADR 0006 hybrid-blob amendment, proposed ADR 0018, and [`../../reviews/phase-0-adversarial-audit.md`](../../reviews/phase-0-adversarial-audit.md)
**Product:** Pastral — Native Windows Clipboard Intelligence and History Platform

## 1. Purpose

This design defines the research, architecture, privacy, security, performance, UX, testing, and repository foundations that must exist before Pastral feature implementation begins.

Pastral is a Windows 11-native clipboard memory and paste platform built around five differentiators:

1. one immutable logical clip event containing multiple clipboard representations;
2. high-fidelity replay rather than text-only history;
3. source-aware organization and deterministic rules;
4. non-activating, focus-safe overlays;
5. local-first operation with conservative resource use and explicit privacy controls.

## 2. Phase 0 scope

Phase 0 produces documentation and repository governance only. It does not implement clipboard capture, persistence, overlay rendering, search, paste injection, manager UI, rules, OCR, or semantic search.

Deliverables:

- official-source research report;
- competitor and gap analysis;
- product vision, user stories, scope, and non-goals;
- architecture overview and process/data-flow diagrams;
- fourteen initial ADRs;
- threat model and privacy model;
- performance budgets and benchmark methodology;
- UX interaction specification;
- testing strategy;
- brand direction and preliminary name-clearance report;
- repository initialization plan;
- project governance files and CI design notes;
- verified internal-consistency report.

## 3. Decisions

### 3.1 Platform

- Windows 11 only.
- Development and compatibility floor for MVP engineering: Windows 11 24H2, build 26100.
- At release-candidate time, the public minimum is raised to the oldest Windows 11 Home/Pro release still serviced by Microsoft. If release occurs after 2026-10-13, the expected public floor is Windows 11 25H2, build 26200.
- x64 first. ARM64 follows only after x64 correctness, security, accessibility, and performance gates pass.
- Build with the latest stable Windows SDK available to the pinned toolchain, while runtime capability checks protect the declared minimum OS.

### 3.2 Process model

- `pastral-agent.exe`: the only always-running process; Rust + Win32/COM/OLE.
- Overlay runs in the agent unless measurements prove isolation is necessary.
- `pastral-worker.exe`: launched only for bounded untrusted or expensive work; no resident OCR/model.
- `pastral-manager.exe`: C++/WinRT + WinUI 3; runs on demand, hosts Quick Paste as an activation/window mode, and never opens the database directly.
- `pastral-cli.exe`: Rust diagnostics and administration client.
- Agent owns SQLite and blob storage.
- Local IPC uses a versioned named-pipe protocol granting ordinary access through a least-privilege current logon-SID ACE, with runtime user/logon-session/token validation, anti-squatting/remote rejection, peer validation, and operation authorization. Proposed ADR 0018 adds bounded 36-byte framing, Protobuf Edition 2024 control schemas, and sequenced bulk transfer while keeping the resident runtime unselected pending evidence. IPC is not claimed as a secure enclave against fully compromised same-user code.

### 3.3 Rendering

The initial overlay rendering choice is DirectComposition with Direct2D and DirectWrite. The overlay HWND uses non-activating window styles and show/position flags, rejects activation through `WM_MOUSEACTIVATE`, remains out of taskbar and Alt+Tab, and supports Per-Monitor V2 DPI.

A Windows.UI.Composition prototype may be benchmarked before implementation. Switching requires an ADR update and evidence that it improves reliability, footprint, or maintainability without compromising the Rust agent boundary.

### 3.4 Data model

- A clipboard notification creates a transient `ClipboardObservation`; only a successfully captured current state with at least one representation creates an immutable `ClipEvent`.
- A successful durable `ClipEvent` contains one or more `ClipRepresentation` records. Notifications/attempts are transient observations; denied, failed, skipped, or degraded outcomes use content-free audit records where policy permits, and source-owned hard deny creates no durable row.
- Durable public identity uses opaque UUIDv4; civil time uses UTC microseconds; installation-local order uses storage-assigned `capture_order`; ordinary raw blob identity uses versioned `sha256-raw-v1`.
- Raw bytes and fidelity metadata are preserved where safe.
- Transformations create derived representations and never mutate originals.
- Payload storage is content-addressed for ordinary clips through one `BlobStore` contract; internal SQLite BLOB versus external-file placement is selected by versioned Windows benchmark policy rather than assumed universally.
- Sensitive/Private clips use random blob identifiers and no persistent plaintext digest/deduplication by default; a future keyed-equality scheme requires a separate accepted privacy decision.
- Duplicate payload storage may be deduplicated while every meaningful copy occurrence remains recorded.

### 3.5 Privacy defaults

- Local-only and network-silent.
- No account, cloud sync, remote AI, content telemetry, or content-bearing logs.
- Default retention: 90 days with a 5 GB automatic-cleanup target for ordinary unpinned history. Pinned/protected clips are exempt, may exceed the target with visible warnings, and remain included in storage reporting.
- Password managers and reliably identified private-browser contexts are excluded by default.
- Highly confident passwords, OTPs, API tokens, private keys, recovery codes, and similar secrets are not stored by default.
- Hidden metadata-only `SensitiveItemSkipped` audit events are enabled by default for 24 hours and contain only broad detector/policy class, active profile, and coarse timestamp—no preview, value hash, OCR text, precise source, size, structure, or reconstructable content. Users may disable or shorten retention.
- Users may explicitly enable encrypted sensitive retention for narrow scopes. Sensitive payloads never enter FTS, OCR, semantic indexing, duplicate stacks, previews, or notifications.
- Clipboard-owner exclusion signals such as `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0` are hard-deny signals, are not overridden by normal settings, and create no durable clip/audit row.

### 3.6 Storage and search

- SQLite with FTS5 and structured indexed filters.
- One database owner process.
- WAL is not assumed; it is enabled only after crash, backup, antivirus, low-disk, and shutdown tests justify it.
- Blob commits use temporary files, flush/close, atomic rename, then database commit with recovery reconciliation.
- Search is lexical and deterministic first. Semantic search is explicitly deferred.

### 3.7 Packaging

- Primary release route: signed multi-executable MSIX using a Windows Application Packaging Project or equivalent supported tooling.
- Framework-dependent Windows App SDK packaging is preferred for Store and managed deployment.
- Self-contained unpackaged builds may be produced for CI and diagnostics but are not the initial public distribution promise.
- Portable distribution and WinGet manifests are deferred until signing, update, uninstall, and data-retention behavior are stable.

## 4. Architecture boundaries

### Domain

Pure Rust models and invariants with no Win32 or database calls.

### Clipboard platform layer

Owns clipboard listener registration, sequence tracking, COM apartments, OLE `IDataObject`, Win32 clipboard access, global memory, registered formats, delayed rendering, and safe adapters.

### Capture orchestration

Owns coalescing, bounded retry, self-generated-update suppression, capture policy, persistence transaction, and worker scheduling.

### Storage

Owns migrations, repositories, FTS, retention, quotas, blob lifecycle, backup, integrity, and recovery. UI code never sees SQL.

### Rule engine

Pure, deterministic, versioned, replayable, and testable without Windows.

### Overlay

Consumes immutable view models and returns explicit action intents. It does not own capture, storage, rules, or paste policy.

### Manager

Uses versioned IPC APIs and projects accessible view models. It does not open SQLite or inspect blob files directly.

## 5. Core data flow

1. `WM_CLIPBOARDUPDATE` arrives at the agent message-only window.
2. Agent reads the clipboard sequence number and rejects already-processed/self-generated sequences.
3. A bounded retry policy obtains a short-lived OLE/clipboard snapshot.
4. Safe common representations are captured immediately; large data streams to a staging blob.
5. Capture policy and sensitive detectors decide whether to store, skip, isolate, or record metadata only.
6. Metadata and finalized blobs commit atomically enough to recover after interruption.
7. Lightweight classification and deterministic rules produce overlay and enrichment intents.
8. Expensive parsing is sent to a bounded worker only after durable capture.
9. Manager and CLI query state through authenticated local IPC.

## 6. Error handling principles

- Clipboard contention never blocks the source application; capture retries are short and bounded.
- Capture failure records metadata-only diagnostics without payload.
- Unsupported formats are labeled honestly and skipped or isolated.
- Database or disk failure causes capture degradation, not interception of the user's copy operation.
- Worker crashes cannot terminate the agent.
- Paste failure cannot modify the immutable stored clip.
- Security and privacy rules outrank transformations and convenience rules.
- Equal-priority destructive rules fail closed and require user resolution.

## 7. Performance model

- No clipboard polling.
- No continuously running async runtime without benchmark evidence.
- No periodic full-database scans.
- No resident OCR or semantic model.
- Agent target private working set: under 25 MB steady-state where achievable.
- Ordinary-text synchronous capture-critical path target: under 10 ms.
- Warm overlay first frame target: under 50 ms after successful capture.
- Quick Paste warm-open target: under 80 ms.
- First lexical results target: under 30 ms at 100,000 ordinary text records on reference hardware.

Claims are prohibited until measured with a repeatable harness.

## 8. UX principles

- Overlay defaults to Smart mode and remains non-interactive unless the user explicitly enters interaction mode.
- Ordinary number keys and Escape are never globally intercepted.
- Quick Paste may take focus only because the user invoked it.
- Manager uses Windows system colors and user accent for most UI; Pastral gradient is reserved for brand moments.
- High contrast, text scaling, keyboard navigation, screen readers, reduced motion, transparency disabled, RDP, and 100–300% DPI are first-class test states.
- Learned behavior is always inspectable, reversible, scoped, and explainable.

## 9. Validation gates before feature work

Feature implementation may begin only after:

1. all Phase 0 documents exist;
2. ADR statuses and architecture documents agree;
3. platform/version statements are consistent;
4. threat mitigations map to architecture boundaries and tests;
5. performance budgets have named measurement methods;
6. UX interaction rules define focus, activation, keyboard, and privacy behavior;
7. no unresolved placeholder changes implementation direction;
8. repository governance and initialization plan are committed;
9. a fresh documentation consistency check passes.

## 10. Decomposition after Phase 0

Each later subsystem receives its own design, plan, tests, and verification:

1. repository/toolchain bootstrap and domain skeleton;
2. domain model, storage, and search;
3. clipboard fixture producer/consumer and capture engine;
4. high-fidelity replay;
5. focus-safe overlay;
6. Quick Paste;
7. profiles and deterministic rules;
8. manager UI;
9. security hardening and fuzzing;
10. packaging, compatibility, benchmark, and release candidate.
