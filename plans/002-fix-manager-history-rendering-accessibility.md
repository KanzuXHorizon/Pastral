# Plan 002: Restore visible and accessible manager clip rows

> **Executor instructions:** Use an isolated worktree and test first. This plan repairs the existing Home/History presentation contract; it is not permission to redesign either page. Stop if the generated C++/WinRT binding model differs from the current excerpts.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- apps/manager/Pastral.Manager/ViewModels apps/manager/Pastral.Manager/Pages apps/manager/Pastral.Manager/Tests eng/verify-native-manager.ps1 screenshots-actual`

## Status

- **Priority:** P0
- **Effort:** S
- **Risk:** LOW
- **Depends on:** Plan 001
- **Category:** correctness, accessibility, UI
- **Reconciled at:** commit `49be43c`, 2026-08-05

## Why this matters

History’s primary rows exist but render without bound text, and their UI Automation names are empty. The detail pane updates from code-behind, which masks the defect in current smoke coverage. Until this is repaired, the core “find a copied item” task is unusable visually and for screen-reader users, and later layout/polish work cannot be evaluated honestly.

## Current state

- At audit time, `ViewModels/ClipPreviewViewModel.idl` declared `[default_interface] runtimeclass ClipPreviewViewModel` with read-only string properties but no bindable metadata. The current working tree now contains an uncommitted concurrent `[bindable, default_interface]` change.
- `Pages/HomePage.xaml` and `Pages/HistoryPage.xaml` use classic bindings such as `{Binding SafePreview}`, `{Binding Source}`, `{Binding AutomationName}`.
- `HistoryPage.xaml.cpp` manually reads the selected `ClipPreviewViewModel` to populate detail controls, so selection can appear to work despite blank rows.
- Live UIA inspection on the current Debug manager found six `ListItem` descendants and 42 Text descendants, all with empty accessible names.
- Existing screenshots `history-wide.png` and `history-narrow.png` show blank row surfaces. The working tree also contains uncommitted native UIA assertions for six non-empty rows; review and verify them rather than recreating or overwriting them.
- The project uses C++/WinRT, WinUI 3 and generated IDL metadata; match that pattern rather than introducing a second UI framework.

## Recommended implementation

Use the minimal C++/WinRT-compatible contract: add bindable metadata to the runtime class and preserve the existing immutable properties and classic XAML templates. If the pinned toolchain rejects that metadata, switch only these templates to a proven compiled binding pattern and document why. Do not duplicate row values into code-behind or use element-name hacks.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Static/build | `powershell -File eng/verify-native-manager.ps1 -Mode Build` | Debug and Release PASS |
| Fresh UIA smoke | `powershell -File eng/verify-native-manager.ps1 -Mode All` | row assertions and existing flow PASS |
| Full baseline | `powershell -File eng/build.ps1 -Task Full` | exit 0 |

## Scope

**In scope**

- `apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.idl`
- `apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.h`
- `apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.cpp`
- `apps/manager/Pastral.Manager/Pages/HomePage.xaml`
- `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml`
- `apps/manager/Pastral.Manager/Tests/` when adding a focused native/UIA probe
- `eng/verify-native-manager.ps1`
- `screenshots-actual/manager-review/` for matched fresh evidence only

**Out of scope**

- Changing row information architecture, spacing, colors, typography or responsive topology.
- Adding new clip actions or provider fields.
- Reading storage directly from the manager.
- Making sensitive/private content bindable before authorization.

## Steps

### Step 1: Review and preserve the concurrent failing UI Automation assertion

The current working tree already extends the fresh-artifact manager smoke. Review its exact diff, keep it only if it tests the fresh artifact and asserts:

- six synthetic rows in Debug diagnostic mode;
- every row has a non-empty automation name;
- the first row name contains the privacy-safe preview/type/source/time contract;
- visible Text descendants include the expected safe preview and source;
- no row name contains hidden content beyond its existing safe view model.

Add the equivalent Home recent-list assertion or a focused probe that confirms at least one Home row exposes non-empty preview/source text.

First resolve Plan 001’s singular/artifact verifier issue so this test cannot fail for the wrong reason. Do not overwrite concurrent work.

**Verify before accepting the implementation:** run against the parent commit or temporarily remove only the bindable metadata in an isolated test worktree; the row assertion must fail specifically on empty row content.

### Step 2: Make the view model a valid binding source

Add the supported bindable metadata to `ClipPreviewViewModel.idl` while retaining `[default_interface]` and all current immutable properties. Regenerate through the normal MSBuild/XAML pipeline; do not edit generated files under `obj/`.

Expected conceptual shape:

```idl
[bindable, default_interface]
runtimeclass ClipPreviewViewModel
{
    String SafePreview{ get; };
    // existing properties unchanged
}
```

If the toolchain requires separate attribute syntax/order, use the generated metadata accepted by Windows App SDK 2.3.1 and document the exact compiler requirement in the commit.

**Verify:** `powershell -File eng/verify-native-manager.ps1 -Mode Build` → Debug and Release PASS.

### Step 3: Verify all existing binding paths

Inspect Home and History templates for property-name mismatches. Keep one source of truth in the view model; do not introduce duplicate `Tag`, `DataContext` or code-behind population. Confirm these bindings resolve:

- `SafePreview`
- `Source`
- `RelativeTime`
- `TypeLabel`
- `Profile`
- `RepresentationSummary`
- `AutomationName`
- `StateSummary`
- `Pinned`
- `Unavailable`

If a binding is unused, leave it for the detail flow unless removal is independently justified. Ensure `AutomationProperties.Name="{Binding AutomationName}"` lands on the ListItem-accessible subtree.

**Verify:** fresh UIA smoke now reports non-empty row names and text.

### Step 4: Strengthen privacy/accessibility regression coverage

Add assertions that an unavailable row announces its type/state without revealing unavailable payload content beyond the existing safe placeholder. Confirm selection and focus are distinct and the selected row remains navigable with keyboard/UIA patterns.

In Release live mode, seed the disposable read server used by the bridge smoke and assert returned privacy-safe previews bind into rows. Do not rely only on Debug synthetic data.

**Verify:** `powershell -File eng/verify-manager-ipc-bridge.ps1 -Mode All` and native manager fresh smoke → PASS.

### Step 5: Capture matched visual evidence

Build one fresh Debug artifact and capture Home wide, History wide and History narrow using the same synthetic dataset. Replace evidence images only when rows visibly contain expected text and no personal clipboard content. Record window dimensions/DPI in the review note or filename metadata.

**Verify:** manually compare before/after: row text is visible, clipped safely, and detail selection corresponds to the selected row. UIA assertions remain the machine authority.

### Step 6: Run aggregate verification

Run static/build/UIA/bridge/full gates and inspect the final diff for generated files or unrelated layout changes.

**Verify:** `powershell -File eng/build.ps1 -Task Full` → exit 0.

## Test plan

- Debug synthetic Home rows bind visible preview/source.
- Debug synthetic History rows bind all displayed metadata.
- Release live History/Search rows bind from authenticated provider output.
- Every row has a non-empty UI Automation name.
- Unavailable row exposes policy-safe state, not hidden content.
- Selecting the first row updates details and the row itself remains readable.
- Empty/no-results views contain no stale row descendants.

## Done criteria

- [ ] Home and History row text is visible in a fresh build.
- [ ] Every visible row has a meaningful non-empty UIA name.
- [ ] Existing detail selection, search and no-results behavior still passes.
- [ ] Release live-provider rows pass the same binding assertions.
- [ ] No generated `obj/`, `x64/`, personal data or unrelated visual edits are committed.
- [ ] Full verification exits 0.

## STOP conditions

Stop and report if:

- The pinned C++/WinRT compiler does not support the selected bindable metadata and compiled binding would require a broad project migration.
- The fix would expose sensitive/private data absent from the authorized view model.
- Row values are empty before they reach `ClipPreviewViewModel`; that is a provider/data-contract issue and requires a revised plan.
- Fresh UIA cannot distinguish row content because of a platform automation bug; capture a minimal probe before choosing a workaround.

## Maintenance notes

Any new manager row view model used with classic `{Binding}` must be explicitly bindable and must have a UIA smoke assertion for visible text and accessible name. Do not accept detail-pane side effects as proof that the row rendered.
