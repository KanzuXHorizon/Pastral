# Phase 0 consistency review

> **Historical status:** This review records the checks performed for commit `3445122`. Its readiness conclusion is superseded by [`phase-0-adversarial-audit.md`](phase-0-adversarial-audit.md), the Phase 0.1 hardening decisions, and [`phase-0-2-ipc-storage-verification.md`](phase-0-2-ipc-storage-verification.md). It is retained to preserve audit history rather than rewritten as if the later findings had been known.

**Date:** 2026-08-03
**Scope:** Research, product, architecture, ADR, security, privacy, performance, testing, UX, brand, repository, and release-planning artifacts.
**Historical conclusion at commit `3445122`:** Phase 0 documentation appeared internally consistent enough to begin the separate repository/toolchain/domain-bootstrap design cycle. It did not establish that Pastral was implemented, benchmarked, secure, accessible, compatible, packaged, or ready for release.

> **Superseded readiness notice (2026-08-04):** The adversarial review in [`phase-0-adversarial-audit.md`](phase-0-adversarial-audit.md) found material gaps in same-user IPC claims, foreign OLE threading, sequence/event semantics, registered-format identity, domain IDs/time/digests, format adapters, source confidence, Quick Paste hosting, UIPI fallback, hard-deny audit behavior, Private-profile guarantees, authenticated streaming, quota semantics, SQLite deletion remnants, toolchain freshness, and naming risk. Repository/toolchain bootstrap is gated on the Phase 0.1 hardening commit plus the Phase 0.2 IPC/storage refinement checks. The command evidence below remains an accurate historical record for commit `3445122` and is not rewritten as if Phase 0.1 existed then.

## 1. Artifact coverage

Phase 0 contains:

- durable product authority: `PRODUCT.md`;
- durable visual authority: `DESIGN.md`;
- product vision, scope, personas, and glossary;
- official-source research and competitor/gap analysis;
- architecture overview, process model, capture data flow, clipboard lifecycle, and paste lifecycle;
- fourteen required initial ADRs;
- threat, privacy, encryption, and incident-response models;
- performance budgets and benchmark methodology;
- test strategy, compatibility matrix, and test-data policy;
- design system, interaction, overlay, Quick Paste, manager, and accessibility specifications;
- provisional brand guidelines and name-clearance research;
- repository initialization plan and release checklist;
- root governance, contribution, security, conduct, changelog, line-ending, and ignore policies.

No Rust/C++ source, Cargo/CMake project, database, executable, installer, CI workflow, generated logo asset, or runtime feature was added.

## 2. Requirement-to-artifact mapping

| Requirement | Primary artifacts | Result |
|---|---|---|
| Product mission, user stories, non-goals | `PRODUCT.md`, `docs/product/*` | Covered |
| Official research | `docs/research/official-sources.md` | Covered with primary-source URLs and retrieval date |
| Competitor gap | `docs/research/competitor-analysis.md` | Covered |
| Architecture/process/data flow | `docs/architecture/*` | Covered |
| Initial ADR set | `docs/adr/0001` through `0014` | Covered |
| Threat/privacy/encryption | `docs/security/*` | Covered |
| Performance budgets/evidence method | `docs/performance/*` | Covered; implementation measurements remain absent by design |
| UX interaction/overlay/Quick Paste/manager | `DESIGN.md`, `docs/ux/*` | Covered |
| Accessibility | `docs/ux/accessibility.md`, testing matrix | Covered as requirements; implementation validation remains absent |
| Brand/name research | `docs/brand/*` | Covered provisionally; no legal clearance or final assets |
| Testing strategy/fixtures | `docs/testing/*` | Covered |
| Repository initialization | `docs/operations/repository-initialization.md` | Covered |
| Release evidence gates | `docs/release/checklist.md` | Covered |

## 3. Cross-document decisions

### Platform

Consistent decision:

- Windows 11 only;
- MVP engineering floor Windows 11 24H2 build 26100;
- release-candidate floor equals the oldest Home/Pro release still serviced by Microsoft;
- expected Windows 11 25H2 build 26200 floor after 2026-10-13;
- x64 first, ARM64 after x64 gates.

Windows 10 appears only as an explicitly rejected alternative/non-goal or compatibility disclaimer.

### Process and ownership

Consistent decision:

- `pastral-agent.exe` is the only resident process and sole primary database owner;
- `pastral-worker.exe`, `pastral-manager.exe`, and `pastral-cli.exe` are on demand;
- manager/CLI use versioned authenticated IPC and never open SQLite directly;
- worker cannot access the main DB/root keys and has no network by default;
- overlay begins inside the agent unless measurements/security require separation.

### Data and fidelity

Consistent decision:

- one immutable `ClipEvent` contains multiple `ClipRepresentation` records;
- derived outputs preserve parent/version/parameter provenance;
- ordinary blobs may deduplicate while occurrences remain distinct;
- sensitive identifiers avoid plaintext equality leakage;
- fidelity is explicit and never universal.

### Privacy and retention

Consistent decision:

- local-first, network-silent core;
- 90-day/5-GB ordinary default;
- pinned items excluded from automatic retention deletion;
- highly confident secrets not stored by default;
- historical Phase 0 wording allowed optional `SensitiveItemSkipped`; Phase 0.1 supersedes it with a hidden coarse 24-hour default audit that remains content-free and user-disableable;
- password-manager and reliable private-context exclusions by default;
- `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0` are hard deny;
- encrypted sensitive retention is explicit, narrow, and excluded from ordinary indexes/previews/exports.

### UX and accessibility

Consistent decision:

- passive overlay has no activation/focus/keyboard ownership;
- Quick Paste takes focus only after explicit invocation and revalidates the destination;
- Windows system colors/user accent govern interaction; violet/cyan is limited brand/provenance material;
- high contrast, text scaling, mixed DPI, reduced motion, transparency disabled, RDP, RTL, touch, keyboard, and UI Automation are release gates;
- hidden sensitive content must be absent from the accessibility tree and caches.

### Deferred scope

Consistent deferrals:

- OCR;
- semantic search;
- scripting/plugins;
- cloud/device sync;
- browser/IDE integrations;
- Windows Hello private-vault UX;
- portable public distribution;
- WinGet manifests;
- ARM64.

These deferrals are not represented as existing functionality.

## 4. Verification commands and results

### Official-source and competitor references

```bash
grep -RIn 'https://' docs/research docs/brand/name-clearance.md
```

Result: PASS. Research documents contain Microsoft, Rust, SQLite, competitor-primary, academic-name-use, and trademark-database references.

### Executable naming

```bash
grep -RInE 'pastral-(agent|worker|manager|cli)\.exe' docs/architecture
```

Result: PASS. Architecture consistently uses all four executable names.

### ADR structure

```bash
for f in docs/adr/[0-9][0-9][0-9][0-9]-*.md; do
  grep -q '^## Decision' "$f" || exit 1
  grep -q '^## Review triggers' "$f" || exit 1
done
```

Result: PASS for ADR 0001 through ADR 0014.

### Placeholder/unsupported-claim scan

```bash
grep -RInE 'TBD|TODO|implement later|fill in details|universally lossless|guaranteed secure' . \
  --exclude='2026-08-03-pastral-phase-0-foundation.md' \
  --exclude='phase-0-consistency-review.md' \
  --exclude-dir=.git
```

Result: PASS with no matches. The execution plan and this review are excluded because both contain the literal scan pattern as documentation.

### Platform/version inspection

```bash
grep -RInE 'Windows 10|Windows 11|26100|26200|2\.3\.1|1\.97\.1|28000\.2270' \
  PRODUCT.md README.md docs
```

Result: PASS after manual inspection. Windows 10 is only a rejected/non-supported context. Toolchain/API versions are consistently described as Phase 0 decisions to revalidate during bootstrap/release.

### Privacy-default inspection

```bash
grep -RInE '90 days|5 GB|SensitiveItemSkipped|ExcludeClipboardContentFromMonitorProcessing|CanIncludeInClipboardHistory' \
  PRODUCT.md README.md docs
```

Result: PASS after manual inspection. Retention, sensitive skip, and hard-deny semantics agree.

### Local Markdown links

A read-only Python check parsed local Markdown links and resolved them relative to each document.

Result after this review was added: PASS across 55 Markdown files.

### Local launcher isolation

```bash
git check-ignore -v Start-DevSpace-MCP-Cloudflared.ps1
```

Result: PASS. The machine-specific DevSpace launcher is ignored by the root `.gitignore` and is not product source.

### Accidental artifact cleanup

A Windows `where` probe produced an unintended WSL file named `NUL`. It was removed immediately and a follow-up `test ! -e NUL` passed. The final staging check verifies it is absent.

## 5. Spec self-review

### Placeholder scan

No unresolved implementation-direction placeholder remains outside the plan's own example scan.

### Internal consistency

No contradiction was found between:

- Windows support baseline;
- process/database ownership;
- overlay focus behavior;
- default privacy/retention;
- sensitive opt-in storage;
- immutable originals;
- deferred OCR/semantic/plugin scope;
- packaging/update direction;
- benchmark-versus-claim policy.

One wording correction changed Windows App SDK 2.3.1 from an inaccurate “2.0 servicing line” phrase to “2.x line.”

### Scope

The original whole-product specification is too large for one implementation plan. The accepted decomposition remains:

1. repository/toolchain and pure-domain bootstrap;
2. domain/storage/search;
3. clipboard fixtures/capture;
4. high-fidelity replay;
5. focus-safe overlay;
6. Quick Paste;
7. profiles/rules;
8. manager;
9. security hardening/fuzzing;
10. packaging/compatibility/benchmarks/release.

Phase 0 itself contains no implementation beyond Git/document governance.

### Ambiguity resolution

Explicitly resolved:

- Windows 11-only support;
- release floor tied to servicing rather than a permanently fixed old build;
- agent as sole DB owner;
- WAL requires evidence;
- DirectComposition/Direct2D/DirectWrite is the initial overlay choice with prototype review trigger;
- sensitive payload storage is off by default but encrypted narrow opt-in is allowed;
- ordinary unencrypted secret retention is not offered;
- previous clipboard restoration is off by default;
- public source-code license is not assumed;
- `Pastral` remains provisional.

## 6. Deviations from the original Phase 0 file list

### Added `PRODUCT.md` and `DESIGN.md`

The UI/UX design governance required one durable source for product truth and one for visual-system truth. These files consolidate facts already approved in the specification and do not add feature scope.

`PRODUCT.md` uses the explicit platform value `windows` even though the design-skill schema enumerates web/iOS/Android/adaptive. Recording the true Windows-only platform was judged safer than misclassifying the product.

### Final brand assets not created

Phase 0 defines geometry, variants, scaling, production, licensing, and clearance gates but does not create SVG/ICO/Store assets. Final asset work waits for formal name clearance and a separate reviewed visual-production slice.

### CI and build scaffold not created

The repository initialization plan defines them, but adding Cargo/CMake/WinUI/workflow files would begin implementation/toolchain bootstrap and is intentionally the next independent slice.

### No `LICENSE`

No license was selected by the owner. The repository states that redistribution is not implied. Choosing a public license is a deliberate future decision, not a generated default.

## 7. Remaining risks and decisions

Implementation evidence is still required for:

- COM apartment and delayed-render behavior;
- exact capture retry/coalescing timing;
- DirectComposition versus Windows.UI.Composition prototype outcome;
- actual agent dependency footprint and working set;
- SQLite journal/pragmas and two-part blob commit recovery;
- authenticated-encryption library/algorithm;
- IPC schema serialization format and handshake implementation;
- AppContainer/restricted-token worker feasibility;
- multi-executable MSIX startup/update/uninstall behavior;
- final Windows/SDK/App SDK versions at bootstrap/release;
- application compatibility and clipboard restoration heuristics;
- all performance, accessibility, security, and fidelity targets;
- final name clearance, logo assets, package identity, signing, and license.

## 8. Historical readiness decision

At commit `3445122`, Phase 0 was considered ready to be committed as a documentation/governance baseline. The later adversarial audit found additional architecture/security/factual gaps and supersedes this conclusion for implementation readiness.

The next highest-value step is a separate, test-first repository/toolchain and pure-domain bootstrap plan. It must verify the native Windows toolchain and domain invariants without implementing clipboard capture, storage, IPC, overlay, or UI prematurely.
