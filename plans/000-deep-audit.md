# Pastral Deep Audit — V1 Readiness

**Audit date:** 2026-08-05  
**Reconciled at:** commit `49be43c` (audit began at `36641f2`)  
**Scope:** repository architecture, correctness, security/privacy, performance, tests, dependencies, DX, documentation, product direction, native UI/UX, accessibility, localization, packaging, and release readiness.  
**Method:** source/document review, git/history inspection, fresh Windows build/test gates, focused reproduction, live WinUI/UI Automation inspection, screenshot review, and current primary-platform research.

## Executive verdict

Pastral has an unusually disciplined engineering foundation for an early preview: typed domain invariants, bounded clipboard/IPC/storage APIs, authenticated same-session named pipes, fail-closed privacy admission, native WinUI, exact toolchain pins, and substantial focused tests. During the audit, four commits completed the default resident decision, fixed the footprint/dependency-policy mismatch, and produced a verified development MSIX foundation. It is still not a V1 product. The aggregate agent smoke contract remains inconsistent, one core UI list is visibly blank pending an uncommitted concurrent repair, large parts of the promised user journey do not exist, and packaging is not yet a production delivery pipeline.

The best route remains incremental vertical delivery, not a rewrite: finish the remaining verification contracts, verify/land the manager binding/accessibility repair, harden resident/package lifecycle, then build capture/search/replay/privacy and manager workflows in dependency order.

## Audit health

### Engineering health

| Dimension | Score | Evidence-based assessment |
|---|---:|---|
| Correctness and verification | 2/4 | Strong focused tests and repaired admission/dependency gates, but agent aggregate smoke still hangs and native standalone smoke can test a stale artifact. |
| Security and privacy | 3/4 | Strong fail-closed boundaries and no advisory findings; encryption, Private, fuzzing, adjacent-version and release-signing gates remain open. |
| Performance and scalability | 2/4 | Explicit budgets and bounded IPC exist; 100k UI/search evidence, resident combined-load measurement, incremental list loading, and replay benchmarks are missing. |
| Architecture and maintainability | 3/4 | Clear process/provider/domain boundaries; several hotspot files are large and future subsystems are unimplemented rather than incorrectly entangled. |
| Delivery and operations | 2/4 | A tracked, verified development MSIX foundation exists; packaging remains absent from CI and production signing/update/upgrade evidence is incomplete. |
| **Total** | **11/20** | **Acceptable foundation; significant work required before V1.** |

### Native UI/UX health

| Dimension | Score | Key finding |
|---|---:|---|
| Accessibility | 2/4 | History/Home clip rows expose empty text and names because classic binding is not bindable; smoke misses the failure. |
| Performance | 2/4 | Async provider and bounded pages are good; nested scrolling, fixed list height, no incremental loading, and no 100k proof remain. |
| Appearance and theming | 3/4 | Native semantic brushes, Mica and restrained brand system; current composition is still card-heavy and engineering-preview oriented. |
| Platform conformance | 3/4 | Native controls and window behavior; narrow History topology and nested scroll violate expected desktop task flow. |
| Adaptivity | 1/4 | `NarrowLayout` has no trigger, so narrow screenshots retain a crushed two-column layout. |
| **Total** | **11/20** | **Acceptable visual foundation, blocked by functional and adaptive defects.** |

## Prioritized findings

| # | Finding | Category | Impact | Effort | Fix risk | Confidence | Evidence |
|---:|---|---|---|---:|---|---|---|
| 1 | Finish the remaining fresh-artifact verification contracts | correctness/tests/DX | P0: agent aggregate hangs and native smoke can test stale artifacts | S–M | LOW–MED | HIGH | `eng/verify-agent.ps1` no-argument smoke; `eng/verify-native-manager.ps1:407-418,543`; reproduced failures |
| 2 | Make clip rows visible and accessible | UI/a11y/correctness | P0: History’s primary content is blank; screen readers receive empty rows | S | LOW | HIGH | `ClipPreviewViewModel.idl`; classic bindings in Home/History; live UIA found six empty ListItems and empty Text names |
| 3 | Repair History adaptive topology and scrolling | UI/adaptivity/perf | P1: narrow window is cramped; nested scrolling obstructs long-history operation | L | MED | HIGH | `HistoryPage.xaml:9,20-42,225-226`; `history-narrow.png` |
| 4 | Complete localization of runtime states | i18n/a11y/UI | P1: Vietnamese users receive mixed English/VI and English-only live/error states | M | MED | HIGH | only 25 keys/locale; hard-coded strings in `HistoryPage.xaml.cpp`, `HomePage.xaml.cpp`, `ManagerDataProvider.cpp` |
| 5 | Preserve the reconciled resident dependency/footprint contract | architecture/perf/DX | P1: future default-feature drift could invalidate resident budgets | S ongoing | MED | HIGH | commits `76b17c3` and `49be43c`; dependency/admission gates now PASS |
| 6 | Promote development MSIX into reproducible production delivery | release/security/DX | P1: package exists but lacks CI, exact SDK selection, protected production signing and upgrade coverage | L | HIGH | HIGH | commits `de24298`–`49be43c`; no packaging workflow; SDK tool chosen by highest path |
| 7 | Complete MVP1 representation capture/search/retention/recovery | direction/correctness | P1: product currently captures ordinary Unicode text only and lacks core storage policy UX | XL | HIGH | HIGH | `README.md`, `docs/product/scope.md`, source search and current adapter tests |
| 8 | Build replay, Quick Paste, overlay, tray and hotkeys | direction/security/UX | P1: “find and paste” primary product promise is unavailable | XL | HIGH | HIGH | no implementation in `apps/`/`crates/`; detailed accepted UX/ADR docs exist |
| 9 | Implement encryption, Private, profiles and rules | security/direction | P1: privacy-sensitive workflow and deterministic automation are intentionally unavailable | XL | HIGH | HIGH | storage rejects protected payloads; no encryption/profile/rule services; product gates explicit |
| 10 | Complete manager V1 workflows | direction/UI | P1: Home/History are the only real pages; organization, policy and recovery are inaccessible | XL | MED | HIGH | `docs/ux/manager.md` IA versus `Pages/` and navigation implementation |
| 11 | Add fuzzing, adjacent-version, compatibility and release-scale evidence | security/tests/perf | P0 release gate: parser, update, 100k, accessibility and compatibility claims remain unproven | XL | HIGH | HIGH | ADR 0018 Proposed; no fuzz target; release checklist open |
| 12 | Correct stale product/status documentation | docs/DX | P2: contributors and users receive wrong capability state | S | LOW | HIGH | README says bridge Health-only while `ManagerIpcBridge.h` exposes History/Search and live gate passes |

## Detailed findings

### [P0] Two verification contracts remain inconsistent after resident reconciliation

**Resolved during audit**

- Commit `76b17c3` replaced the invalid ordered binary subtraction with magnitude (`abs_diff`), added matching boundary tests, and reconciled dependency/admission policy with the default resident IPC graph.
- Focused metrics/cross-process tests, `verify-dependencies.ps1`, and `verify-agent-ipc-admission.ps1 -Mode All` now pass. The measured Release binary delta was 8,704 bytes; admission ceilings passed.

**Remaining evidence**

1. Commit `766e298` makes no-argument invocation start the resident, while `eng/verify-agent.ps1` still invokes the binary without arguments and waits for a nonzero exit. `-Mode All` consistently reaches successful Debug/Release builds and then hangs in smoke.
2. Fresh native manager code displays singular `1 item`, but smoke still searches for `1 items`.
3. `Resolve-DebugExecutable` can fall back to `apps/manager/Pastral.Manager/x64/Debug/pastral-manager.exe`, so standalone smoke can pass against a stale artifact while `-Mode All` tests fresh isolated output.

**Impact**

Agent aggregate verification can hang indefinitely, and a green standalone native smoke does not necessarily prove the current checkout. This still blocks trustworthy release evidence.

**Recommendation**

Finish Plan 001 by making agent smoke bounded and consistent with resident startup, then make native smoke artifact-exact and assert semantic/localized state rather than stale English copy.

### [P0] History and Home clip templates do not bind C++/WinRT row properties

**Evidence**

- At audit time, `ClipPreviewViewModel.idl` declared a runtime class without bindable metadata. A concurrent uncommitted change now adds `[bindable, default_interface]`; it still requires a successful fresh build/UIA verification and review before the finding is closed.
- Home/History templates use classic `{Binding SafePreview}`, `{Binding Source}`, and related properties.
- Screenshots show blank result-row bodies.
- Live UI Automation found six History `ListItem` elements, all with empty names; all Text descendants also had empty names.
- Detail pane still updates because code-behind reads the selected view model directly, allowing the existing smoke test to miss the blank row.

**Impact**

The core History task—scan and choose copied content—is unusable visually and through assistive technology. Empty row names also make selection ambiguous for keyboard/screen-reader users.

**Recommendation**

Use the smallest native repair first: mark the runtime class bindable or replace classic binding with compiled binding supported by the project’s C++/WinRT setup. Extend UIA tests to assert each row’s accessible name and visible child text, not only selection side effects.

### [P1] History’s narrow state is unreachable

**Evidence**

- `NarrowLayout` contains setters but no `AdaptiveTrigger`.
- `WideLayout` activates at 920 DIP; below that, default column definitions remain `3*` and `2*`.
- `history-narrow.png` shows list and details squeezed side-by-side.

**Impact**

Narrow windows, high text scaling and split-screen usage lose readable content and operable action space. This conflicts with the project’s 100–300% DPI/text-scaling commitment.

**Recommendation**

Make narrow the explicit base state, wide the triggered enhancement, and use a one-pane selection/drill-in or stacked detail flow that preserves search and primary action visibility.

### [P1] Nested scrolling and fixed list height undermine scalable History

**Evidence**

- A page-level `ScrollViewer` wraps History.
- `ListView` supplies its own scroll presenter and is constrained by `MaxHeight="560"`.
- The provider returns only the first bounded page; no incremental-load contract exists.

**Impact**

Wheel/keyboard navigation becomes ambiguous, available window height is wasted, and the documented “virtualized 100k history remains responsive” gate cannot be satisfied by the current topology.

**Recommendation**

Use a fixed header/command area and a star-sized content row. Let ListView own the result viewport and virtualization. Add cursor-based incremental loading with cancellation, stable selection, and bounded page retention.

### [P1] Localization is decorative rather than complete

**Evidence**

Both `en-US` and `vi-VN` contain only 25 keys. Major static headings are localized, but live connection states, errors, empty states, counts, type/profile labels, representation summaries, and UIA names are assembled from English literals in C++.

**Impact**

Vietnamese presentation changes language during normal use, and localized UI Automation announcements are incomplete. Manual singular/plural formatting is also incorrect for scalable localization.

**Recommendation**

Introduce a typed manager string provider backed by PRI resources; resource every user-facing dynamic message and semantic label; centralize count/relative-time formatting and test both languages with long strings.

### [P1] Preserve the newly accepted resident runtime contract

**Evidence**

Commits `766e298` and `76b17c3` establish no-argument resident startup, accept authenticated IPC in the default resident graph, update ADR 0018/dependency policy, and record fresh footprint evidence. Focused admission/dependency gates pass.

**Impact**

The immediate architecture drift is resolved, but future feature/default changes could silently invalidate the measured resident footprint or reintroduce policy contradictions.

**Recommendation**

Keep exact resident dependencies and combined capture+IPC footprint as required gates. Plan 001 now owns only the remaining CLI smoke/artifact authority; Plan 005 hardens package lifecycle and continuous package evidence.

### [P1] Packaging is promising but not yet a release system

**Evidence**

Commits `de24298`–`49be43c` track the MSIX manifest/staging/build/registration/install scripts and record a verified development installer hash. No GitHub workflow watches `packaging/**` or runs package layout/install tests. `Resolve-SdkTool` still chooses the lexicographically highest installed SDK tool rather than the pinned release SDK. Development signing is implemented; protected production signing/update/rollback is not.

**Impact**

A package can differ by machine, regress without CI, or pass source builds while failing install/activation. Release signing secrets and update behavior do not yet have an auditable boundary.

**Recommendation**

Plan 005 establishes tracked package authority, exact SDK selection, unsigned layout CI, isolated signed development smoke, protected release signing, startup-task behavior, upgrade/migration/uninstall checks, and artifact hashes.

### [P1] The product is a strong foundation, not yet the promised workflow

The current implementation provides ordinary Unicode capture, bounded local storage/search, authenticated History/Search previews and a native manager foundation. It does not implement common rich formats, replay/paste, Quick Paste, overlay, tray/hotkeys, encryption/Private, profiles/rules, or most manager administration workflows. These are not “polish”; they define the V1 user value and are decomposed in Plans 006–009.

### [P2] Capability documentation is stale

README and CHANGELOG still describe the manager bridge as Health-only and WinUI History/Search mapping as unavailable. The implementation and `verify-manager-ipc-bridge -Mode All` prove the opposite. Update capability matrices and known limitations whenever Plan 001 establishes a truthful baseline.

## Positive findings to preserve

- Domain types reject invalid UUID, time, format, fidelity and protection-domain combinations.
- Storage validates blob metadata, dedup boundaries, cursor ordering, missing payloads, FTS mapping and integrity.
- Clipboard capture is event-driven, bounded and tested without mutating the user clipboard in automated gates.
- Privacy admission honors source-owned hard deny, resolves source conservatively, and skips high-confidence private-key material before persistence.
- IPC framing validates before allocation, authenticates exact capabilities, rejects replay and binds kernel peer/session evidence.
- Manager uses a provider/bridge boundary and does not open storage directly.
- Background manager operations use one worker and stale-generation rejection.
- Native design uses system semantic brushes, Mica, standard controls and a restrained brand palette.
- Debug synthetic data is clearly disclosed; Release excludes it.
- Source-policy, Clippy, docs, IPC prototype, authenticated transport, bridge ABI/live read path and dependency advisory scan provide a strong base for completion.

## Release-gap matrix

| Release area | Current state | V1 blocker | Owning plan |
|---|---|---|---|
| Toolchain/build/test | Partial; several focused gates pass | aggregate baseline inconsistent | 001 |
| Manager core content | Implemented but blank rows | binding/UIA failure | 002 |
| Responsive History | Partial | narrow state and scrolling | 003 |
| Localization | Partial English/VI | dynamic states English-only | 004 |
| Resident/package lifecycle | Resident decision + verified development MSIX foundation | canonical agent smoke, CI, exact SDK, production signing, upgrade | 001, 005 |
| Clipboard formats | Unicode text foundation | rich/image/file/OLE coverage | 006 |
| Storage/search lifecycle | Strong foundation | retention/quota/backup/recovery/scale | 006 |
| Paste/replay | Not implemented | primary task absent | 007 |
| Quick Paste/overlay/tray | Not implemented | explicit retrieval interaction absent | 007 |
| Encryption/Private | Not implemented | privacy-sensitive mode unavailable | 008 |
| Profiles/rules/collections | Not implemented | deterministic organization absent | 008–009 |
| Full manager | Home/History only | policy/admin/recovery workflows absent | 009 |
| Security compatibility | Partial | fuzz/adjacent-version/bulk/update gates | 010 |
| Accessibility matrix | Partial automation | Narrator, contrast, scaling, RDP etc. | 010 |
| Performance evidence | Resident admission and prototype metrics | long soak, 100k, replay, cold/warm and release-scale matrix | 010 |
| Release distribution | Development scripts only | protected signing/update/provenance | 005, 010 |

## Verification transcript summary

| Command | Result |
|---|---|
| initial `eng/build.ps1 -Task Full` at `36641f2` | FAIL at agent IPC admission metric assumption |
| focused admission metrics/cross-process after `76b17c3` | PASS |
| workspace tests excluding the initially failing probe | PASS during initial audit |
| `cargo clippy ... -D warnings` | PASS |
| `cargo doc --workspace --no-deps` | PASS |
| `cargo audit` | no vulnerability reported for 66 dependencies |
| `eng/verify-source-policy.ps1` | PASS |
| `eng/verify-dependencies.ps1` after resident reconciliation | PASS |
| `eng/verify-agent-ipc-admission.ps1 -Mode All` after resident reconciliation | PASS; Release ceilings and Health/History/Search smoke pass |
| `eng/verify-ipc-prototype.ps1 -Mode All` | PASS |
| `eng/verify-ipc-transport.ps1 -Mode All` | PASS |
| `eng/verify-manager-ipc-bridge.ps1 -Mode All` | PASS |
| `eng/verify-agent.ps1 -Mode All` at `49be43c` | static/build PASS, then hangs in no-argument smoke because CLI now starts resident |
| `eng/verify-native-manager.ps1 -Mode All` | fresh build path retains singular/artifact contract issue; a concurrent binding/UIA repair is uncommitted |
| live History UIA tree inspection before concurrent repair | six rows present; row/text accessible names empty |

## Audit limitations

- No production package was installed, certificate store was modified, or external release system was invoked.
- No real personal clipboard content was read by the audit.
- Manual Narrator, contrast-theme, 300% scaling, touch, RDP, mixed-monitor and power-state matrices were not executed; their absence is itself a release gap.
- Development packaging source was committed during the audit and reconciled at `49be43c`; production signing/CI/upgrade behavior was not executed by this audit.
- The audit did not mutate source code or resolve concurrent uncommitted UI changes; it created only files under `plans/`.
