# Pastral Phase 0.1 Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct factual and architectural gaps in the Phase 0 baseline so clipboard capture, IPC, data identity, source attribution, Quick Paste, privacy, paste dispatch, and testing can be implemented without relying on unsafe or impossible assumptions.

**Architecture:** Phase 0.1 remains documentation/governance work. It selects a hybrid Win32/OLE capture strategy on a dedicated STA, formalizes durable domain identities and clipboard-format identity, scopes IPC security honestly, hosts Quick Paste in the on-demand manager process, and expands test/release gates. No Rust, C++, database, package, or UI implementation is created.

**Tech Stack:** Markdown, Mermaid, Git, Windows 11, Win32 clipboard/OLE/COM, named pipes, DPAPI, SQLite/FTS5, C++/WinRT/WinUI 3, Rust architecture decisions.

## Global Constraints

- Windows 11 only; MVP engineering floor remains Windows 11 24H2 build 26100.
- x64 first; ARM64 follows only after x64 quality gates.
- One always-running process remains the default.
- No clipboard polling.
- Foreign clipboard/OLE calls never run on the agent control/overlay message thread.
- A successful `ClipEvent` contains at least one captured representation; denied/failed/skipped observations use separate audit records.
- Registered clipboard format numeric IDs are runtime-local; durable identity uses standard IDs or registered names.
- Named-pipe ACLs and DPAPI secrets do not claim protection from fully compromised same-user processes.
- Private and sensitive payloads never use ordinary plaintext equality-revealing content addresses.
- Source attribution, paste success, screen-capture exclusion, and clipboard-event completeness are confidence-bearing/best-effort claims.
- No `uiAccess`, elevation, service, or integrity-level bypass is introduced for paste injection.
- No feature implementation begins until Phase 0.1 consistency checks pass.

---

### Task 1: Record adversarial audit and current-source corrections

**Files:**
- Create: `docs/reviews/phase-0-adversarial-audit.md`
- Modify: `docs/research/official-sources.md`
- Modify: `docs/brand/name-clearance.md`
- Modify: `docs/operations/repository-initialization.md`
- Modify: `docs/superpowers/plans/2026-08-03-pastral-phase-0-foundation.md`

**Interfaces:**
- Consumes: current Phase 0 documents and official Microsoft/Rust sources retrieved on 2026-08-04.
- Produces: prioritized findings, corrected Windows SDK pin, and strengthened provisional-name risk.

- [x] **Step 1: Write the adversarial audit**

Record severity, affected documents, factual basis, correction, residual risk, and whether the finding changes architecture or only wording.

- [x] **Step 2: Correct toolchain facts**

Replace Windows SDK `10.0.28000.2270` with current stable `10.0.28000.2526` and preserve the runtime floor/capability-check distinction.

- [x] **Step 3: Strengthen name-clearance findings**

Record the current clipboard product `Pastry` as a confusingly similar category-adjacent name and raise the branding gate without claiming infringement.

- [x] **Step 4: Verify stale pins are absent**

Run:

```bash
grep -RIn '10\.0\.28000\.2270' . --exclude-dir=.git
```

Expected: no matches except a historical correction entry in the adversarial audit when explicitly labeled obsolete.

### Task 2: Formalize capture threading, sequence semantics, and source attribution

**Files:**
- Create: `docs/adr/0015-clipboard-capture-threading.md`
- Create: `docs/architecture/threading-and-com-apartments.md`
- Create: `docs/architecture/clipboard-event-identity.md`
- Create: `docs/architecture/source-context.md`
- Modify: `docs/architecture/process-model.md`
- Modify: `docs/architecture/clipboard-lifecycle.md`
- Modify: `docs/architecture/data-flow.md`
- Modify: `docs/architecture/overview.md`
- Modify: `docs/security/threat-model.md`
- Modify: `docs/performance/budgets.md`

**Interfaces:**
- Consumes: Win32 clipboard listener/sequence contracts, OLE call-cancellation limitations, existing process model.
- Produces: a nonblocking control thread, dedicated capture STA, bounded best-effort capture, honest gap accounting, and confidence-bearing source context.

- [x] **Step 1: Accept the hybrid capture/threading ADR**

Select a control/overlay message thread that never invokes foreign `IDataObject`, a dedicated capture STA with a message pump, a serialized storage executor, and supplemental OLE adapters only when required. Document COM cancellation as best effort, not a hard timeout.

- [x] **Step 2: Define event and sequence semantics**

Specify that `WM_CLIPBOARDUPDATE` carries no event identity, sequence numbers are current window-station state, zero and wrap are handled, delayed rendering can affect increments, intermediate rapid copies may be unobservable, and the final current state must be captured without claiming complete history.

- [x] **Step 3: Define source confidence**

Separate clipboard-owner evidence, foreground snapshot, package/process identity, window metadata, and explicit integrations. Prohibit domain/project claims from window-title guessing by default.

- [x] **Step 4: Update lifecycle, diagrams, threats, and budgets**

Move foreign calls off the control thread, add capture-health degradation, sequence-gap diagnostics, exact clipboard-hold metrics, and review trigger for a separate capture broker if cancellation/recovery is inadequate.

### Task 3: Define durable data and clipboard-format invariants

**Files:**
- Create: `docs/adr/0016-data-identity-time-and-digests.md`
- Create: `docs/architecture/data-model.md`
- Create: `docs/architecture/clipboard-format-policy.md`
- Modify: `docs/product/glossary.md`
- Modify: `docs/architecture/paste-lifecycle.md`
- Modify: `docs/adr/0004-clipboard-data-object-model.md`
- Modify: `docs/adr/0006-sqlite-blob-store.md`
- Modify: `docs/security/privacy-model.md`
- Modify: `docs/security/encryption.md`

**Interfaces:**
- Consumes: immutable-original principle, Windows standard/registered format behavior, SQLite/blob design.
- Produces: distinct observation/audit/clip records, stable IDs/time semantics, versioned digests, protection-domain deduplication, and an adapter-based format matrix.

- [x] **Step 1: Define event/data invariants**

Create `ClipboardObservation`, `CaptureAuditEvent`, `ClipEvent`, immutable capture core, mutable annotations, and representation/provenance relationships. Require at least one representation for `ClipEvent`.

- [x] **Step 2: Define identity, time, and digest rules**

Use opaque UUIDv4 public IDs generated from a CSPRNG, internal SQLite row IDs only as implementation details, UTC microsecond timestamps for civil time, storage-assigned installation-local `capture_order` for durable ordering, monotonic clocks only for durations, and versioned `sha256-raw-v1` for ordinary raw blob content. Sensitive/private blobs use random identifiers and no plaintext digest/deduplication by default.

- [x] **Step 3: Define clipboard format identity and support policy**

Persist standard format IDs or exact registered-format names, never session-local registered numeric IDs. Re-register names on replay. Define safe handling for Unicode/ANSI text, HTML, RTF, URL formats, CF_HDROP, virtual files, DIB/DIBV5, CF_BITMAP, PNG, OLE media, and unknown custom formats.

- [x] **Step 4: Harden encrypted streaming**

Require whole-message buffering within a bound or independently authenticated chunks; never expose unauthenticated plaintext. Bind chunk index/count and representation identity in associated data.

### Task 4: Correct IPC and local-adversary security boundaries

**Files:**
- Create: `docs/architecture/ipc-security-model.md`
- Modify: `docs/adr/0008-ipc-protocol.md`
- Modify: `docs/security/threat-model.md`
- Modify: `docs/security/encryption.md`
- Modify: `docs/security/incident-response.md`
- Modify: `docs/architecture/process-model.md`
- Modify: `SECURITY.md`

**Interfaces:**
- Consumes: Windows named-pipe ACL/token APIs and DPAPI scope.
- Produces: cross-user/session isolation, anti-squatting controls, operation authorization, and honest same-user residual risk.

- [x] **Step 1: Correct the security objective**

State that DACL/logon SID/client-token checks protect cross-user/session access and accidental clients, while a fully compromised same-user process is outside a strong confidentiality boundary.

- [x] **Step 2: Add pipe construction and peer-validation requirements**

Require `FILE_FLAG_FIRST_PIPE_INSTANCE`, `PIPE_REJECT_REMOTE_CLIENTS`, explicit least-privilege DACL, client PID/session/token validation, bounded impersonation, no broad generic rights, and safe discovery/rotation.

- [x] **Step 3: Reframe challenge-response**

Keep nonces/instance binding for stale/wrong-client/replay resistance, but state that a DPAPI user-scope installation secret is not a same-user malware barrier.

- [x] **Step 4: Protect high-risk operations through user intent**

Require explicit UI confirmation or future Windows Hello freshness for sensitive reveal/export/destructive operations; CLI content remains opt-in and private/sensitive data requires separate authorization.

### Task 5: Resolve Quick Paste hosting, paste elevation limits, and privacy UX

**Files:**
- Create: `docs/adr/0017-quick-paste-hosting.md`
- Modify: `docs/architecture/process-model.md`
- Modify: `docs/architecture/paste-lifecycle.md`
- Modify: `docs/ux/quick-paste.md`
- Modify: `docs/ux/overlay.md`
- Modify: `docs/ux/accessibility.md`
- Modify: `docs/security/privacy-model.md`
- Modify: `docs/product/scope.md`
- Modify: `docs/performance/budgets.md`

**Interfaces:**
- Consumes: existing four-process boundary, WinUI manager choice, `SendInput` UIPI limitation, privacy defaults.
- Produces: a concrete Quick Paste host/lifecycle, safe elevated-destination fallback, best-effort capture exclusion, and precise sensitive-skip behavior.

- [x] **Step 1: Accept Quick Paste hosting ADR**

Host Quick Paste as an activation mode/window of the single-instance on-demand `pastral-manager.exe`. Define warm as an already-running UI process and cold separately. A fifth lightweight executable requires measured evidence and a new ADR.

- [x] **Step 2: Define UIPI/elevation behavior**

Do not request `uiAccess`, elevation, or a service. When focus restoration/input injection is blocked or uncertain, leave the chosen data on the clipboard and ask the user to paste manually.

- [x] **Step 3: Define screen-capture and accessibility limits**

Treat `WDA_EXCLUDEFROMCAPTURE`-style behavior as defense in depth, never DRM or a guarantee. Hidden content remains absent from view models/UIA/caches regardless of capture API behavior.

- [x] **Step 4: Refine skip and private-profile defaults**

Hard-deny source signals create no durable history/audit row. High-confidence secret skips create a hidden content-free audit event by default with coarse metadata and short retention. Built-in Private profile requires encrypted payloads and no ordinary FTS/previews from its introduction; Windows Hello remains a later user-presence layer.

- [x] **Step 5: Clarify quota semantics**

Define 5 GB as an automatic-cleanup target rather than an absolute cap because pinned items are exempt. Low-disk reserve pauses new payload capture without deleting pinned data silently.

### Task 6: Expand tests, release gates, and consistency evidence

**Files:**
- Modify: `docs/testing/strategy.md`
- Modify: `docs/testing/compatibility-matrix.md`
- Modify: `docs/testing/test-data-policy.md`
- Modify: `docs/performance/benchmark-methodology.md`
- Modify: `docs/release/checklist.md`
- Modify: `docs/adr/0002-winui3-manager.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/operations/repository-initialization.md`
- Modify: `README.md`
- Modify: `PRODUCT.md`
- Modify: `docs/reviews/phase-0-consistency-review.md`
- Modify: `docs/superpowers/plans/2026-08-04-pastral-phase-0-1-hardening.md`

**Interfaces:**
- Consumes: all Phase 0.1 decisions.
- Produces: regression coverage, updated decision index, revised baseline summary, and a verified hardening commit.

- [x] **Step 1: Add regression and compatibility gates**

Cover registered-format ID changes, sequence zero/wrap/gaps, delayed rendering, capture STA hang, source confidence, hard-deny no-record behavior, same-user boundary statements, elevated/UIPI paste fallback, Quick Paste cold/warm host, format adapters, FTS/WAL deletion remnants, encrypted chunk authentication, and private-profile isolation.

- [x] **Step 2: Update indexes and baseline summaries**

Add ADR 0015–0017 and link new architecture documents. Mark the old consistency review as superseded by the adversarial hardening review where applicable without rewriting historical command evidence.

- [x] **Step 3: Correct stable manager build authority**

Use a Visual Studio C++ WinUI `.vcxproj` with MSBuild/NuGet/XAML as the stable manager build path and a `.wapproj` for the four-executable package. Remove empty root CMake/vcpkg bootstrap requirements; CMake remains optional for a future isolated native library and Windows App SDK CMake consumption is not a stable release dependency while Microsoft labels it Experimental.

- [x] **Step 4: Run full documentation checks**

Run placeholder, stale-version, architecture-term, local-link, ADR-heading, secret-signature, whitespace, and Git scope checks.

- [x] **Step 5: Inspect and commit**

Run:

```bash
git diff --check
git status --short
git diff --stat
git diff --cached --check
```

Commit only the Phase 0.1 documentation/governance changes with:

```bash
git commit -m "docs: harden Pastral architecture assumptions"
```
