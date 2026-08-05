# Plan 003: Rebuild History as a responsive virtualized workspace

> **Executor instructions:** Use an isolated worktree. Preserve the provider/security boundary and the incumbent design system. This is a structural UI plan, not a brand replacement.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- apps/manager/Pastral.Manager/Pages/HistoryPage.* apps/manager/Pastral.Manager/Themes apps/manager/Pastral.Manager/ViewModels apps/manager/Pastral.Manager/Services eng/verify-native-manager.ps1 docs/ux/manager.md DESIGN.md`

## Status

- **Priority:** P1
- **Effort:** L
- **Risk:** MED
- **Depends on:** Plan 002
- **Category:** UI, accessibility, performance, architecture
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

History is the manager’s primary operative surface. Its narrow setters never activate, and an outer ScrollViewer plus a fixed-height ListView creates nested scrolling and prevents the result list from owning the viewport. A V1 implementation must remain scannable at narrow widths and high text scaling, keep search and actions visible, and support bounded incremental history without loading all records.

## Current state

- `HistoryPage.xaml` wraps the full page in `ScrollViewer`.
- `NarrowLayout` has no trigger; `WideLayout` activates at 920 DIP.
- Default `HistoryColumns` is `3* / 2*`, so widths below 920 retain two columns.
- `HistoryResultsList` has `MaxHeight="560"`; the page and list can both scroll.
- Provider/bridge supports one bounded page and `hasMore`, but no cursor/incremental-loading UI contract.
- Details are always present in the visual tree, including on narrow layouts.
- `DESIGN.md` requires an operate-mode native utility, restrained containers, strong task hierarchy, adaptive stacked details, keyboard/screen-reader support and no generic glowing-card dashboard.

## Target interaction

- Header/status/search remain visible at the top of the page.
- Results own the remaining viewport and virtualized scroll.
- Wide (>= 920 effective DIP): list and detail pane side-by-side.
- Standard: list dominates; detail opens as an in-page overlay/pane or lower region without two competing scroll surfaces.
- Narrow/text-scaled: one pane. Selecting a row opens details with a clear Back action; search state and selection survive return.
- Pagination is cursor-based, cancellable and stable. Reaching a threshold requests the next page; loading/error/retry is represented inside the results region.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Native static/build | `powershell -File eng/verify-native-manager.ps1 -Mode Build` | PASS |
| UI Automation | `powershell -File eng/verify-native-manager.ps1 -Mode All` | responsive flow PASS |
| Bridge/live read | `powershell -File eng/verify-manager-ipc-bridge.ps1 -Mode All` | PASS |
| Full baseline | `powershell -File eng/build.ps1 -Task Full` | exit 0 |

## Scope

**In scope**

- `Pages/HistoryPage.xaml`, `.h`, `.cpp`
- focused History view-model/provider contracts needed for paging/selection
- `Themes/PastralTheme.xaml` only for reusable layout/state styles
- `Strings/en-US/Resources.resw`, `Strings/vi-VN/Resources.resw` for new controls/states
- native verifier/UIA and synthetic/live seeded test data
- `docs/ux/manager.md` only when implementation clarifies the accepted History pattern

**Out of scope**

- Paste implementation, collections, rules, full clip inspector, or new storage query syntax.
- Rebranding, custom title bar/window chrome, arbitrary animations.
- Direct manager database access.
- Loading unbounded history or image payloads.

## Steps

### Step 1: Add failing adaptive and scroll-topology tests

Extend native static/UIA verification to assert:

- `NarrowLayout` has a `MinWindowWidth="0"` trigger or is the explicit base state.
- At a narrow test width, detail content is not squeezed beside results; a row selection transitions to one-pane detail and Back restores results/search/selection.
- At a wide width, list and details are side-by-side.
- Only the results ListView owns the primary vertical scroll presenter; there is no page-level ScrollViewer around it.
- Search remains reachable and focus order follows header → search → results → details/actions.

Use explicit window sizing and UIA states rather than screenshot-only assertions.

**Verify:** fresh UIA test fails against current narrow topology.

### Step 2: Replace the outer scrolling page with a viewport grid

Use a root Grid with auto-sized header/status/command rows and a final star-sized content row. Move result empty/loading/list states into the star region. Remove `MaxHeight="560"`; set the ListView to stretch and rely on its own ScrollViewer/virtualization.

Preserve the max content width only where it does not prevent results from using available height. Avoid wrapping the ListView in Border padding that reduces hit area or creates card-inside-card visual weight.

**Verify:** build passes; UIA can scroll the list while header/search remain present.

### Step 3: Make narrow state authoritative

Define narrow/base column widths and positions explicitly, then apply wide setters only at the threshold. For narrow mode, choose one of these native patterns:

- recommended: one result pane plus a detail pane that replaces it in the same grid, with Back and focus restoration;
- acceptable: stacked details only when content height remains bounded and results own scrolling.

At 300% text scaling, use the one-pane pattern regardless of physical width if effective layout space is insufficient.

**Verify:** tests at narrow, 640, 920 and wide widths; no horizontal clipping of consequential labels/actions.

### Step 4: Separate History presentation state from control mutation

Introduce a focused state model for:

- connection state;
- query text and parsed/plain status;
- loading initial/search/more;
- result items and `hasMore`;
- selected item ID;
- detail visibility mode;
- operation error/retry.

Keep the existing generation-based stale-response rejection. Render state in one method or small state-specific methods so loading/error/empty/result transitions cannot leave stale controls visible.

**Verify:** focused native tests or static probe covers every state transition and confirms selection is retained only when its ID remains present.

### Step 5: Add bounded cursor-based incremental loading

Extend the manager provider/bridge use without changing storage ownership:

- history requests pass the last `capture_order` cursor;
- append a next page only when query/generation matches;
- reject duplicate IDs/order regressions;
- cap retained rows according to a documented UI budget or support safe recycling;
- search either supports its own cursor or explicitly replaces the current page until backend pagination is added;
- cancellation/unload prevents callbacks from mutating the page.

Trigger loading before the final visible item using ListView incremental-loading patterns or a deterministic threshold. Show an inline progress/error/retry row, not a modal dialog.

**Verify:** seeded server with > page limit loads multiple pages in exact newest-first order, with no duplicates and stable selection.

### Step 6: Refine hierarchy and row density within DESIGN.md

After structure is correct, refine:

- search as the dominant control;
- results as rows separated by rhythm/dividers rather than independent heavy cards;
- source/type/profile/state encoded redundantly but only decision-relevant metadata shown by default;
- selected representation/action visible only when real capability exists;
- details grouped by task, not by decorative boxes;
- Windows focus/selection visuals and system accent retained.

Do not add gradients to normal controls or oversized marketing copy.

**Verify:** light, dark and high-contrast screenshots plus UIA focus-state checks.

### Step 7: Cover error, empty, overflow and lifecycle states

Test initial empty, no results, loading, load-more, agent restart, protocol mismatch, unavailable row, partial page, very long source/preview/localized text, resize during search, page unload/reload and rapid query replacement. Every state must provide one clear recovery action where recovery is possible.

**Verify:** UIA scenario suite passes with deterministic seeded provider modes.

### Step 8: Run full verification and capture evidence

Run bridge, native and full gates. Capture matched wide/narrow/light/dark evidence using synthetic data only. Inspect the final accessibility tree for non-empty rows, headings, selected state, pane labels and Back action.

## Test plan

- Narrow base state at 480–919 effective DIP.
- Wide state at >=920 DIP.
- 300% text scaling one-pane behavior.
- List owns vertical scroll; header/search remain available.
- Keyboard navigation, Home/End/PageUp/PageDown, context menu key and Back.
- Initial, search, load-more, no-results, disconnected, protocol mismatch and retry states.
- 250 ms debounce and stale-query cancellation.
- 250+ seeded rows across pages; no duplicate/order/selection regression.
- Long English and Vietnamese text; RTL topology smoke even if RTL is not a release locale yet.

## Done criteria

- [ ] Narrow layout is reachable and never displays crushed list/detail columns.
- [ ] History has one primary vertical scroll owner.
- [ ] Results fill available height and remain virtualized.
- [ ] Multi-page seeded history loads in deterministic bounded pages.
- [ ] Selection/search/focus survive adaptive transitions correctly.
- [ ] All loading/empty/error/partial states are accessible and recoverable.
- [ ] Light/dark/high-contrast and text-scaling evidence exists.
- [ ] Full baseline passes with no direct storage access.

## STOP conditions

Stop and report if:

- Paging requires breaking the fixed C ABI without a separately versioned additive contract.
- Effective text scaling cannot be detected reliably with the chosen layout technique; preserve a safe one-pane base instead of guessing.
- List virtualization is lost because the items panel or outer layout measures it with infinite height.
- A proposed details pattern removes keyboard focus restoration or exposes sensitive content.

## Maintenance notes

History should remain a viewport-owned command surface. Future filters, bulk selection and paste actions must extend the same state model rather than adding nested scroll regions or permanent side panels that break narrow operation.
