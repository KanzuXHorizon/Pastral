# Plan 009: Complete the manager V1 information architecture and workflows

> **Executor instructions:** Use an isolated worktree. Add a navigation destination only when its provider-backed workflow and state matrix are implemented in the same slice. Hidden unavailable destinations are preferable to dead placeholders.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- apps/manager/Pastral.Manager docs/ux/manager.md docs/ux/accessibility.md DESIGN.md eng/verify-native-manager.ps1`

## Status

- **Priority:** P1
- **Effort:** XL
- **Risk:** MED
- **Depends on:** Plans 003, 004, 006 and 008
- **Category:** UI/UX, product workflows, accessibility, architecture
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

The manager currently provides Home and History. V1 also needs organization, source policy, profile/rule management, storage/recovery, privacy, settings and diagnostics. These pages must not become a generic dashboard or a collection of disabled cards; each destination should complete a real task through authenticated provider contracts and preserve Pastral’s native, calm, privacy-explicit design.

## Information architecture

Implement and reveal in this order:

1. Home
2. History and clip detail
3. Collections
4. Sources
5. Profiles
6. Rules
7. Storage
8. Privacy
9. Settings
10. Diagnostics
11. About

Paste Queue is visible only when sequential-paste behavior is genuinely implemented. Integrations stays hidden until a real reviewed integration exists. Group lower-frequency administration destinations without changing stable names/deep-link identity.

## Shared page architecture

- Every page uses a typed async provider; no direct storage/clipboard access.
- One page state model represents loading, ready, empty, filtered-empty, disconnected, mismatch, permission/policy denied, partial, operation progress, success and error/retry.
- Navigation preserves query/selection/scroll context where safe and clears protected content on lock.
- Shared components cover page header, status banner, empty/recovery state, virtualized data row, detail pane, destructive-scope confirmation, inline progress and accessible result count.
- Static/dynamic copy uses Plan 004’s resource boundary.
- System semantic brushes and native controls dominate; brand gradient remains rare.

## Scope

**In scope**

- navigation shell/deep links/context restoration
- Home actionability and onboarding state
- full clip detail and safe actions backed by completed services
- Collections, Sources, Profiles, Rules, Storage, Privacy, Settings, Diagnostics, About
- shared native design-system components/tokens and UIA tests
- accessibility, localization, responsive and state-matrix coverage

**Out of scope**

- Fake integrations, OCR/AI/semantic-search controls.
- Pages backed only by local sample data in Release.
- Direct manager database/blob/clipboard access.
- Arbitrary raw JSON as normal settings/rule UX.
- Productivity scores, streaks or invented savings/security metrics.

## Program slices

### Slice 1: Extract shared manager shell/page primitives

From the proven Home/History patterns, create focused reusable controls/styles for:

- `ManagerPageHeader` with Level1 heading/subtitle/action slot;
- `OperationalStatusRegion` using native InfoBar semantics;
- `StatePresenter` for loading/empty/error/denied/retry;
- `AdaptiveMasterDetail` for wide/two-pane and narrow/drill-in;
- `DestructiveScopeDialog` that states item/payload/storage impact and undo policy;
- `InlineOperationProgress` with cancel;
- compact metadata rows/badges that use icon+text and system colors.

Do not create a generic mega-control or duplicate existing WinUI controls. Add component-level UIA/theme/scale tests.

### Slice 2: Finish Home as an actionable launch surface

Replace engineering-preview emphasis with real actionable state:

- current profile/capture/lock/startup state;
- recent privacy-safe clips;
- pinned collections when implemented;
- storage/retention/low-disk status;
- unresolved rule conflicts/failed jobs only when actionable;
- short skippable onboarding until completed.

Actions deep-link to the owning page and preserve context. Do not expose unsupported Private or fabricated analytics.

**Verify:** first run, healthy, paused, locked, low disk, disconnected, conflict and completed-onboarding modes.

### Slice 3: Complete clip detail and actions

Expand History detail with authorized sections:

- safe preview;
- source/confidence/privacy state;
- occurrences;
- profile;
- representations/original-derived graph;
- fidelity/compatibility notes;
- sizes/hashes according to sensitivity;
- tags/collections/notes;
- related/duplicates;
- rule matches/transformations;
- retention/encryption policy;
- paste/copy/pin/tag/export/delete/recovery actions only when available.

Use progressive disclosure. At narrow widths, detail is a separate pane/page with Back/focus restoration. Developer raw metadata remains opt-in and cannot bypass authorization.

### Slice 4: Build Collections

Implement manual collections and smart collections over the typed query model. Membership references clips without duplicating payloads. Support create/rename/delete, add/remove, pinned collections, smart-query simulation and container-versus-content deletion scope. Multi-select shows selected count and unavailable/protected exclusions.

**Verify:** empty/large/manual/smart/pinned/deleted-source/unavailable/private-locked cases.

### Slice 5: Build Sources

Group by reliable application/package and optional site/project evidence only when confidence/policy permits. Show capture count/storage/time range/policy/profile behavior and explicit “do not capture this source” action. Privacy options can hide/delete title/domain/path metadata with prospective/retroactive scope clearly stated.

Do not infer domains/projects from weak window titles as fact.

### Slice 6: Integrate Profiles and Rules workflows

Use Plan 008’s services. Provide profile list/detail/switch/auto-switch explanation and rule list/editor/simulation/conflict/audit/rollback. Keep Private gated and lock-aware. Show exact action consequences and privacy precedence. Equal-priority destructive conflicts cannot be saved.

### Slice 7: Build Storage and recovery

Show total usage/quota, originals/derived/previews/index/private categories, age distribution and pinned exclusions. Provide cleanup simulation, data path, integrity check, backup/restore/import and maintenance with progress/cancel. Low-disk state explains capture impact. Never imply physical secure erasure.

Use agent-owned operations and bounded progress events; manager never scans directories itself.

### Slice 8: Build Privacy

Centralize capture pause/exclusions, password-manager/private-browser policy, sensitive detectors and per-class behavior, metadata retention, preview/search/index policy, Private encryption/lock state, session/screen-share/fullscreen behavior, diagnostics/export privacy and destructive clear flows. Every setting states what is stored, what remains metadata and whether change is prospective or retroactive.

High-risk changes require consequence preview and explicit scope, not repeated generic confirmation dialogs.

### Slice 9: Build Settings

Create searchable settings grouped by user behavior:

- Quick Paste/hotkey;
- overlay/tray;
- capture/retention defaults;
- appearance/accessibility/notifications;
- startup/lifecycle;
- import/export settings separately from history;
- advanced/developer compatibility/diagnostics.

Support reset per section and validation/conflict feedback. Do not require editing JSON.

### Slice 10: Build Diagnostics and About

Diagnostics is content-free by default and shows agent/manager/protocol/schema versions, process/queue/listener/hotkey/tray/overlay state, storage integrity/size/journal, last result codes/timings, compatibility profile versions and network-silence state. Diagnostic bundle preview/export must enumerate/redact exact fields before writing.

About shows factual version/build/channel/license/notices/privacy/security/support/name status and links/actions through safe shell activation. No fabricated update status.

### Slice 11: Add onboarding and global recovery routing

Implement a short, skippable onboarding flow:

1. local storage and default sensitive skip;
2. Quick Paste hotkey/conflict;
3. overlay mode/placement;
4. retention/quota and default exclusions;
5. synthetic demonstration without personal content.

Global status routes agent mismatch, database recovery, low disk and locked profile to one authoritative workflow without duplicating state banners across every page.

### Slice 12: Polish navigation and visual system

After functionality passes:

- use compact/expanded NavigationView based on window size;
- add stable icons/access keys where appropriate;
- preserve page context/back-forward semantics;
- reduce excessive independent cards; use spacing/dividers for hierarchy;
- introduce signal-trail motif only for provenance/relationship/brand moments;
- validate light/dark/high contrast/transparency off/reduced motion.

Do not replace native controls/window chrome without measured parity.

## Verification commands

| Purpose | Command | Expected |
|---|---|---|
| Native build/UIA | `powershell -File eng/verify-native-manager.ps1 -Mode All` | all page flows PASS |
| Bridge/provider | focused authenticated operation probes | PASS |
| Resource parity | Plan 004 gate | PASS |
| Full repository | `powershell -File eng/build.ps1 -Task Full` | exit 0 |

## Test plan

For every page:

- loading/cancel, first-run empty, filtered empty, ready, partial, disconnected/restarting, protocol mismatch, policy denied, low disk/locked/corrupt/partial operation;
- keyboard traversal, visible focus, access keys/context menu, screen-reader names/roles/states/live regions;
- wide/narrow/300% scaling/long VI strings/high contrast/reduced motion/transparency off/RDP;
- provider stale completion/cancellation and page navigation lifecycle;
- destructive scope/undo and protected/pinned/private exclusions;
- no sensitive content in unauthorized UIA, thumbnail, cache or diagnostic state.

Cross-page scenarios:

- Home status → owning page → resolution → Home refresh;
- History clip → collection/source/profile/rule context and Back;
- low disk → cleanup simulation → maintenance → recovery;
- Private lock → hidden content everywhere → unlock/reveal → relock clears it;
- diagnostic bundle preview/export contains no content canary.

## Done criteria

- [ ] Every visible navigation destination completes a real provider-backed task.
- [ ] Unimplemented future destinations remain hidden, not dead.
- [ ] Home is actionable and does not display gamified/fabricated metrics.
- [ ] Clip detail and all V1 administration workflows are complete and scope-safe.
- [ ] Shared components reduce duplication without replacing native behavior.
- [ ] All pages pass full state, keyboard, UIA, localization and adaptivity matrices.
- [ ] Manager never opens storage/blob/clipboard directly and never reveals protected data without service authorization.
- [ ] Full repository verification passes.

## STOP conditions

Stop and report if:

- A page has no implemented provider/service contract; keep it hidden.
- A shared component would become a generic control framework larger than the pages it serves.
- A destructive workflow cannot state exact affected scope and recovery/undo.
- Source/project/domain identity lacks reliable evidence.
- Diagnostics or About would need network access or content-bearing logs without separate review.
- A UI feature attempts to enable Private before Plan 008 gates.

## Maintenance notes

Each future manager destination must declare user job, provider contract, state matrix, accessibility/localization tests and release evidence before navigation is exposed. Keep task hierarchy and native conventions stronger than decorative branding.
