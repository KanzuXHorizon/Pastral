# Pastral Phase 0 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a reviewed, internally consistent research, architecture, security, UX, performance, testing, branding, and repository foundation before Pastral feature implementation.

**Architecture:** Phase 0 is documentation-only plus repository governance. It records a Rust Win32/COM agent, on-demand Rust worker and CLI, C++/WinRT WinUI 3 manager, agent-owned SQLite/blob store, versioned per-user named-pipe IPC, and focus-safe native overlay. No clipboard feature code is created in this phase.

**Tech Stack:** Markdown, Mermaid, Git, Windows 11, Rust 1.97.1/Edition 2024 decision record, Windows SDK stable decision record (corrected by Phase 0.1 to 10.0.28000.2526), Windows App SDK 2.3.1 stable decision record, C++20, SQLite/FTS5 design.

## Global Constraints

- Windows 11 only; MVP engineering floor Windows 11 24H2 build 26100.
- Re-evaluate the public OS floor at release candidate against Microsoft servicing; expected floor becomes Windows 11 25H2 build 26200 after 2026-10-13.
- x64 first; ARM64 only after x64 quality gates pass.
- No feature implementation before Phase 0 consistency gates pass.
- Local-first and network-silent by default.
- Default retention is 90 days with a 5 GB automatic-cleanup target for ordinary unpinned history; pinned/protected clips are exempt and may exceed the target with visible warnings (clarified by Phase 0.1).
- Highly confident secrets are not stored by default.
- Clipboard-owner hard-deny formats cannot be overridden by normal settings.
- No clipboard polling, embedded browser, Electron, Tauri, background Node.js, mandatory managed runtime, remote processing, or mandatory AI.
- All optimization and security claims require evidence.

---

### Task 1: Record the approved foundation design

**Files:**
- Create: `docs/superpowers/specs/2026-08-03-pastral-foundation-design.md`

**Interfaces:**
- Consumes: approved product specification and clarified privacy/platform defaults.
- Produces: the authoritative Phase 0 boundary and decisions referenced by all other documents.

- [x] **Step 1: Write the design specification**

Include platform floor, process model, rendering, data model, privacy defaults, storage, packaging, module boundaries, data flow, error handling, performance model, UX principles, and validation gates.

- [x] **Step 2: Scan the design for ambiguity**

Run:

```bash
grep -nE 'TBD|TODO|implement later|fill in|appropriate error handling|similar to' docs/superpowers/specs/2026-08-03-pastral-foundation-design.md
```

Expected: no matches.

### Task 2: Create official-source research and competitor analysis

**Files:**
- Create: `docs/research/official-sources.md`
- Create: `docs/research/competitor-analysis.md`
- Create: `docs/brand/name-clearance.md`

**Interfaces:**
- Consumes: primary Microsoft, Rust, SQLite, and competitor project documentation.
- Produces: traceable evidence for ADRs and a provisional brand-risk record.

- [x] **Step 1: Write the official-source report**

Record the retrieval date, claim supported, URL, and design consequence for Windows lifecycle, Windows App SDK, Rust, clipboard/OLE, overlay/focus, DPI/accessibility, SQLite, DPAPI, process isolation, IPC, and packaging.

- [x] **Step 2: Write the competitor matrix**

Compare Windows Clipboard History, Ditto, CopyQ, PasteBar, and PowerToys Advanced Paste across history, format fidelity, source context, deterministic rules, privacy, overlay/focus, native footprint, and extensibility.

- [x] **Step 3: Write provisional name clearance**

Record searches across general web, GitHub, Microsoft Store search availability limits, package registries, domains, social handles, and trademark databases. State explicitly that this is not legal clearance.

- [x] **Step 4: Verify references are present**

Run:

```bash
grep -RIn 'https://' docs/research docs/brand/name-clearance.md
```

Expected: official URLs and competitor primary URLs appear in every research document.

### Task 3: Define product scope and non-goals

**Files:**
- Create: `docs/product/vision.md`
- Create: `docs/product/scope.md`
- Create: `docs/product/personas.md`
- Create: `docs/product/glossary.md`

**Interfaces:**
- Consumes: foundation design.
- Produces: user outcomes, MVP boundaries, explicit non-goals, personas, and shared vocabulary.

- [x] **Step 1: Write vision and measurable outcomes**

Describe core jobs, product principles, success measures, and acceptance gates without promising unmeasured speed or universal losslessness.

- [x] **Step 2: Write scope and vertical slices**

Separate Phase 0, MVP 1–4, later work, and explicit exclusions.

- [x] **Step 3: Write personas and accessibility contexts**

Include keyboard-first developer, researcher/student, privacy-sensitive professional, designer/content worker, and assistive-technology user.

- [x] **Step 4: Write glossary**

Define `ClipEvent`, `ClipRepresentation`, `DerivedRepresentation`, `Fidelity`, `Profile`, `Rule`, `PasteTransaction`, `SensitiveItemSkipped`, and related terms.

### Task 4: Establish architecture documentation

**Files:**
- Create: `docs/architecture/overview.md`
- Create: `docs/architecture/process-model.md`
- Create: `docs/architecture/data-flow.md`
- Create: `docs/architecture/clipboard-lifecycle.md`
- Create: `docs/architecture/paste-lifecycle.md`

**Interfaces:**
- Consumes: foundation design and ADR decisions.
- Produces: process/component boundaries and Mermaid sequence/data-flow diagrams used by implementation plans.

- [x] **Step 1: Write the architecture overview**

Describe dependency direction and prohibit manager-to-database access and overlay-owned business logic.

- [x] **Step 2: Write the process model**

Document lifetime, privileges, COM apartments, crash boundaries, storage ownership, and IPC directions for agent, worker, manager, and CLI.

- [x] **Step 3: Write capture and paste diagrams**

Use Mermaid diagrams with explicit clipboard event, sequence, bounded retry, capture transaction, worker enrichment, replay data object, delayed rendering, and paste result stages.

- [x] **Step 4: Cross-check executable names**

Run:

```bash
grep -RInE 'pastral-(agent|worker|manager|cli)\.exe' docs/architecture
```

Expected: all four executables are consistently named.

### Task 5: Record the initial ADR set

**Files:**
- Create: `docs/adr/0001-rust-core.md`
- Create: `docs/adr/0002-winui3-manager.md`
- Create: `docs/adr/0003-overlay-rendering.md`
- Create: `docs/adr/0004-clipboard-data-object-model.md`
- Create: `docs/adr/0005-process-boundaries.md`
- Create: `docs/adr/0006-sqlite-blob-store.md`
- Create: `docs/adr/0007-encryption-key-storage.md`
- Create: `docs/adr/0008-ipc-protocol.md`
- Create: `docs/adr/0009-packaging.md`
- Create: `docs/adr/0010-update-strategy.md`
- Create: `docs/adr/0011-windows-support-baseline.md`
- Create: `docs/adr/0012-ocr-approach.md`
- Create: `docs/adr/0013-semantic-search-deferred.md`
- Create: `docs/adr/0014-plugin-system-deferred.md`
- Create: `docs/adr/README.md`

**Interfaces:**
- Consumes: research and foundation design.
- Produces: numbered decisions with context, decision, consequences, rejected alternatives, and review triggers.

- [x] **Step 1: Create ADR index and template convention**

Every ADR contains `Status`, `Date`, `Context`, `Decision`, `Consequences`, `Alternatives considered`, and `Review triggers`.

- [x] **Step 2: Write ADRs 0001–0007**

Cover language/runtime, manager UI, overlay, clipboard object model, process boundaries, storage, and encryption.

- [x] **Step 3: Write ADRs 0008–0014**

Cover IPC, packaging, updates, Windows baseline, OCR, semantic search deferral, and plugin deferral.

- [x] **Step 4: Verify ADR numbering and required headings**

Run:

```bash
for f in docs/adr/[0-9][0-9][0-9][0-9]-*.md; do grep -q '^## Decision' "$f" || exit 1; done
```

Expected: exit code 0.

### Task 6: Define security and privacy models

**Files:**
- Create: `docs/security/threat-model.md`
- Create: `docs/security/privacy-model.md`
- Create: `docs/security/encryption.md`
- Create: `docs/security/incident-response.md`

**Interfaces:**
- Consumes: process/data model and privacy defaults.
- Produces: asset/trust-boundary model, threat-to-mitigation mapping, sensitive-data lifecycle, and incident handling requirements.

- [x] **Step 1: Write the threat model**

Use assets, adversaries, trust boundaries, entry points, STRIDE-style threats, mitigations, residual risks, and security test mapping.

- [x] **Step 2: Write the privacy model**

Define collection, exclusion, sensitive detectors, metadata visibility, retention, deletion, backup/export, logs, diagnostics, and network silence.

- [x] **Step 3: Write encryption envelope design**

Define random data-encryption keys, authenticated encryption, DPAPI user-scope root-key wrapping, versioned envelopes, rotation, zeroization limits, and recovery limitations.

- [x] **Step 4: Write incident-response policy**

Define content exposure triage, key compromise, vulnerable parser, malicious update, data corruption, and release communication steps.

### Task 7: Define performance and testing evidence

**Files:**
- Create: `docs/performance/budgets.md`
- Create: `docs/performance/benchmark-methodology.md`
- Create: `docs/testing/strategy.md`
- Create: `docs/testing/compatibility-matrix.md`
- Create: `docs/testing/test-data-policy.md`

**Interfaces:**
- Consumes: product acceptance criteria and architecture.
- Produces: measurable budgets, reference-hardware policy, benchmark scenarios, test layers, and privacy-safe fixtures.

- [x] **Step 1: Write performance budgets**

Include idle CPU, working set, capture critical path, durable persistence, overlay latency, Quick Paste latency, search datasets, startup, and storage cleanup.

- [x] **Step 2: Write benchmark methodology**

Define warm/cold runs, sample sizes, percentile reporting, ETW/WPA development traces, memory metrics, disk I/O, battery states, and regression thresholds.

- [x] **Step 3: Write test strategy**

Map unit, integration, clipboard fixtures, E2E, UI/accessibility, performance, security, fuzz, packaging, and release gates.

- [x] **Step 4: Write compatibility and fixture policies**

Define destination applications, DPI/display states, RDP, high contrast, reduced motion, private/sensitive synthetic fixtures, and prohibition on real secrets.

### Task 8: Define UX and brand foundations

**Files:**
- Create: `docs/ux/design-system.md`
- Create: `docs/ux/interaction-spec.md`
- Create: `docs/ux/overlay.md`
- Create: `docs/ux/quick-paste.md`
- Create: `docs/ux/manager.md`
- Create: `docs/ux/accessibility.md`
- Create: `docs/brand/brand-guidelines.md`

**Interfaces:**
- Consumes: product principles, accessibility research, privacy policy, and supplied visual references.
- Produces: focus/keyboard behavior, information architecture, visual tokens, motion policy, overlay modes, and logo constraints.

- [x] **Step 1: Write interaction and focus rules**

Specify activation boundaries, keyboard interception rules, pointer behavior, expansion into interactive mode, undo, and privacy-safe previews.

- [x] **Step 2: Write overlay and Quick Paste specifications**

Define states, placement, coalescing, timing, action limits, search syntax, focus restoration, and error/empty states.

- [x] **Step 3: Write manager information architecture**

Define Home, History, Collections, Sources, Profiles, Rules, Paste Queue, Storage, Privacy, Integrations, Settings, Diagnostics, and About.

- [x] **Step 4: Write accessibility and design-system requirements**

Define Segoe UI Variable OS usage, system colors, contrast, UI Automation, text scaling, keyboard navigation, touch targets, RTL, reduced motion, transparency fallback, and screen-reader testing.

- [x] **Step 5: Write brand guidelines**

Define logo geometry, color usage, monochrome/high-contrast variants, app/tray scaling requirements, and anti-patterns. Do not create final legal claims.

### Task 9: Define repository initialization and governance

**Files:**
- Create: `README.md`
- Create: `SECURITY.md`
- Create: `CONTRIBUTING.md`
- Create: `CODE_OF_CONDUCT.md`
- Create: `CHANGELOG.md`
- Create: `.editorconfig`
- Create: `.gitattributes`
- Create: `.gitignore`
- Create: `docs/operations/repository-initialization.md`
- Create: `docs/release/checklist.md`

**Interfaces:**
- Consumes: all Phase 0 decisions.
- Produces: repository policy, ignored local launcher, line-ending rules, contribution gates, security reporting, staged scaffold plan, and release evidence checklist.

- [x] **Step 1: Write repository governance files**

Keep the existing local `Start-DevSpace-MCP-Cloudflared.ps1` outside product source by ignoring it. Do not redistribute credentials, logs, tunnel state, signing material, or user clipboard data.

- [x] **Step 2: Write repository initialization plan**

Specify the future monorepo tree, toolchain bootstrap, dependency pinning, CI sequence, code-signing separation, and first vertical-slice plan.

- [x] **Step 3: Write release checklist skeleton with concrete gates**

Include build, test, accessibility, security, benchmark, packaging, signing, install/update/uninstall, data migration, diagnostics, known limitations, and rollback evidence.

### Task 10: Verify internal consistency and commit Phase 0

**Files:**
- Create: `docs/reviews/phase-0-consistency-review.md`
- Modify: `docs/superpowers/plans/2026-08-03-pastral-phase-0-foundation.md`

**Interfaces:**
- Consumes: every Phase 0 artifact.
- Produces: documented verification evidence and a coherent initial commit.

- [x] **Step 1: Scan for placeholders and forbidden claims**

Run:

```bash
grep -RInE 'TBD|TODO|implement later|production-ready|universally lossless|guaranteed secure' docs README.md SECURITY.md CONTRIBUTING.md CHANGELOG.md || true
```

Expected: no unresolved placeholders; any prohibited phrase appears only as a warning against making that claim.

- [x] **Step 2: Check platform/version consistency**

Run:

```bash
grep -RInE 'Windows 10|Windows 11|26100|26200|2\.3\.1|1\.97\.1' docs README.md
```

Expected: Windows 10 appears only in historical/API context; product support remains Windows 11 only.

- [x] **Step 3: Check privacy-default consistency**

Run:

```bash
grep -RInE '90 days|5 GB|SensitiveItemSkipped|ExcludeClipboardContentFromMonitorProcessing|CanIncludeInClipboardHistory' docs
```

Expected: retention, sensitive skip, and hard-deny semantics are consistent.

- [x] **Step 4: Write consistency review**

Record exact commands, results, contradictions fixed, unresolved risks, and why Phase 0 is ready for the next design cycle.

- [x] **Step 5: Inspect final diff**

Run:

```bash
git status --short
git diff --check
git diff --stat
```

Expected: no whitespace errors and only Phase 0 documentation/governance files plus the ignored local launcher.

- [x] **Step 6: Commit**

Run:

```bash
git add .
git commit -m "docs: establish Pastral foundation architecture"
```

Expected: one coherent root commit on `main`.
