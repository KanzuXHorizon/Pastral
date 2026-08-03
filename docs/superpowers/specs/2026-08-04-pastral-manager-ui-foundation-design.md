# Pastral Manager UI Foundation Design

**Date:** 2026-08-04
**Status:** Approved for autonomous implementation by the owner's instruction to continue researching and completing the project comprehensively
**Phase:** 3A

## 1. Decision

Build the first executable Pastral manager vertical slice as an unpackaged C++20/C++/WinRT WinUI 3 application using the supported Visual Studio `.vcxproj` + MSBuild + XAML toolchain, pinned to Windows App SDK `2.3.1` stable.

The slice delivers a professional adaptive manager shell, shared native design resources, Home and History surfaces, explicit operational states, keyboard/accessibility contracts, and build/CI evidence. It does not pretend that IPC or live clipboard history exists.

## 2. Why this slice comes next

The repository already has domain, storage, and Win32 clipboard foundations, but no user-visible executable. The manager shell is the lowest-risk UI slice because it can establish:

- native build authority and dependency pinning;
- reusable theme, typography, spacing, status, navigation, and clip-row language;
- light/dark/high-contrast compatible structure;
- adaptive manager information architecture;
- honest disconnected, loading, empty, error, paused, and synthetic-preview states;
- an interface boundary for the future versioned IPC view model.

Quick Paste and the passive overlay remain independent later specifications because they have materially different focus, activation, latency, windowing, and privacy constraints.

## 3. Approaches considered

### A. Manager shell plus Home/History vertical slice — selected

Create a real WinUI 3 executable with reusable resources, adaptive navigation, visible system health, Home, History, a detail pane, and explicit state handling.

Benefits:

- proves the native toolchain and XAML compiler;
- creates actual reusable UI architecture rather than screenshots;
- exposes accessibility, localization, density, and state problems early;
- is directly reusable when IPC arrives.

Cost:

- requires careful project/template alignment and Windows App SDK runtime setup;
- cannot display live user data yet.

### B. Design-system/control library only

Would reduce initial scope but produce no end-to-end user surface and leave shell/navigation/state integration unproven. Rejected as insufficiently vertical.

### C. Quick Paste first

Would produce a visible feature sooner but mixes cold-start, destination capture, focus restoration, activation, IPC, search cancellation, and paste safety before the manager/toolchain foundation exists. Rejected for this phase.

## 4. Authoritative constraints

- C++20, C++/WinRT, WinUI 3, Windows App SDK `2.3.1` stable.
- `.vcxproj`/MSBuild/XAML is authoritative; no CMake Windows App SDK dependency.
- `Pastral.slnx` is the solution authority.
- Windows 11 x64 first; compile against installed Windows SDK `10.0.26100.0` for this machine and preserve runtime floor policy separately.
- Manager runs on demand and never opens SQLite/blob files directly.
- No Electron, Tauri, WebView primary UI, .NET manager, or custom UI toolkit.
- No account, cloud, telemetry, mandatory AI, OCR, semantic search, or fake integration tiles.
- No functional Private profile before encrypted storage/non-indexing/lock/recovery gates exist.
- No packaging project in this slice; `.wapproj` remains a packaging-phase deliverable.
- No production claims based on synthetic preview content.

## 5. Build and repository structure

Create:

```text
Pastral.slnx
Directory.Build.props
Directory.Packages.props
apps/manager/Pastral.Manager/
  Pastral.Manager.vcxproj
  Pastral.Manager.vcxproj.filters
  packages.config or PackageReference entries as required by the supported template path
  pch.h
  pch.cpp
  App.xaml
  App.xaml.h
  App.xaml.cpp
  MainWindow.xaml
  MainWindow.xaml.h
  MainWindow.xaml.cpp
  Themes/PastralTheme.xaml
  Pages/HomePage.xaml
  Pages/HomePage.xaml.h
  Pages/HomePage.xaml.cpp
  Pages/HistoryPage.xaml
  Pages/HistoryPage.xaml.h
  Pages/HistoryPage.xaml.cpp
  ViewModels/ManagerState.h
  ViewModels/ManagerState.cpp
  Assets/ (only assets actually required by the build)
```

The initial development executable is unpackaged (`WindowsPackageType=None`) so the manager code can be built and launched before the multi-executable packaging slice. It remains structurally compatible with later `.wapproj` packaging.

## 6. Visual direction

Inherit `DESIGN.md` without replacing it.

### Thesis

The manager presents clipboard memory as a precise native signal trail, not a generic dark dashboard made of glowing cards.

### Material

- Mica on the primary window with system fallback.
- Native Windows semantic brushes for canvas, text, selection, focus, status, errors, and high contrast.
- Aurora Violet and Signal Cyan only for the Pastral identity/provenance trail and restrained brand moments.
- One restrained elevation layer; hierarchy comes primarily from spacing, alignment, dividers, and typography.
- Segoe UI Variable through OS defaults; no redistributed fonts.

### Shell composition

- system-integrated TitleBar control;
- adaptive left NavigationView;
- global operational InfoBar below the title/navigation chrome;
- page Frame for content;
- bottom navigation contains Settings, Diagnostics, and About rather than mixing all destinations into the primary task group.

The initial enabled destinations are Home and History. Future destinations are omitted rather than shown as dead navigation.

## 7. Manager state boundary

Define a presentation-only state model with no storage implementation knowledge:

```cpp
enum class ConnectionState {
    Loading,
    Connected,
    Disconnected,
    CapturePaused,
    ProtocolMismatch,
    Error,
};

struct ClipPreview {
    std::wstring id;
    std::wstring safePreview;
    std::wstring source;
    std::wstring relativeTime;
    std::wstring typeLabel;
    std::wstring profile;
    std::wstring representationSummary;
    bool pinned;
    bool unavailable;
};

struct ManagerSnapshot {
    ConnectionState connection;
    std::wstring statusTitle;
    std::wstring statusDetail;
    std::vector<ClipPreview> recentClips;
    bool synthetic;
};
```

The UI consumes a provider interface returning `ManagerSnapshot` values. In this phase:

- Release provider returns `Disconnected` with no clips and a clear statement that the agent/IPC is unavailable.
- Debug provider can return bounded synthetic previews to exercise real layouts and states.
- Synthetic state is permanently visible through an InfoBar and accessibility name.
- No sample data path exists in Release builds.

The future IPC provider must map protocol DTOs into these presentation types without changing page contracts.

## 8. Home page

### Job

Answer: “Is Pastral healthy, what profile/capture state is active, and what useful action should I take?”

### Layout

1. Page title and concise status sentence.
2. Operational state panel:
   - active profile;
   - capture state;
   - connection/agent state;
   - primary recovery action when disconnected.
3. Recent clips section:
   - privacy-safe preview;
   - source and time;
   - type/profile state;
   - compact pin/unavailable indicators.
4. Storage/retention summary as an honest unavailable or bounded synthetic preview.
5. Getting-started actions only while the live backend is unavailable.

Do not show productivity metrics, streaks, security scores, fabricated space savings, or charts without real data.

## 9. History page

### Primary structure

- dominant search box;
- compact filter/action row;
- virtualizable result list region;
- optional details pane;
- persistent selected primary action text;
- adaptive one-pane mode below the standard width threshold.

### Clip row hierarchy

1. safe preview or content-type placeholder;
2. source and relative time;
3. type/representation summary;
4. profile;
5. only decision-relevant state such as pinned or unavailable.

No icon-only destructive action. Hover-only behavior is supplementary.

### Detail pane

For the selected synthetic item, show:

- preview placeholder;
- source/time/profile;
- representations;
- fidelity/availability note;
- disabled or explanatory Paste/Copy actions because no agent/IPC exists.

### Search behavior in this phase

The page provides production UI states but does not implement a fake backend query language. In Debug synthetic mode, local case-insensitive filtering over the bounded synthetic list may demonstrate selection/no-results behavior. Release mode remains disconnected.

## 10. Required states

Both pages must deliberately render:

- loading;
- connected with data;
- connected empty;
- no search results;
- disconnected agent/IPC;
- capture paused;
- protocol/version mismatch;
- recoverable error;
- unavailable/corrupt item;
- narrow/text-scaled layout.

State copy always states condition, impact, and next safe action. Disabled controls include accessible explanatory text.

## 11. Adaptive behavior

- Default desktop shell uses expanded left NavigationView where width permits.
- Compact mode uses NavigationView's native adaptive behavior.
- History uses two panes at wide widths and one pane with details drill-in or lower region at narrow widths.
- Search and the primary selected action never disappear during reflow.
- Pages avoid fixed heights for consequential text.
- Long localized strings wrap.
- The UI remains usable at 300% text scaling and mixed DPI; exact live evidence is deferred until UI automation/manual test infrastructure exists.

## 12. Accessibility

- Every page has a programmatic page heading.
- Navigation, status, search, results, selected item, detail region, and actions have explicit accessible names.
- Result rows expose preview/type, source, time, profile, position, selected state, pinned/unavailable state.
- Focus and selection remain visually distinct.
- Keyboard order follows title/status → search/actions → results → detail actions.
- High contrast relies on system resources; brand colors are never the sole meaning.
- Synthetic preview status is announced and cannot be mistaken for live user data.
- Hidden sensitive content is not represented in the view model or accessibility tree.

## 13. Localization and content

- Strings are placed in `.resw` resources from the first slice.
- Initial resource language is `en-US`; no claim of Vietnamese or broad localization support is made until resources and layout tests exist.
- Voice is short, literal, calm, and explicit about unavailable functionality.
- Synthetic clip previews use clearly non-sensitive developer examples and are labeled synthetic at the surface level rather than in every row.

## 14. Error handling

- App startup exceptions are caught at the application boundary and produce a content-free diagnostic result.
- Navigation failures retain the current page and display an InfoBar rather than closing the app.
- Release disconnected state is normal, not a crash.
- UI commands that require IPC remain disabled with explanation.
- No exception text containing future payload data is shown directly.

## 15. Verification

### Build gates

- NuGet restore through MSBuild.
- x64 Debug and Release compile.
- XAML compile succeeds.
- warnings treated as errors for project code where template/tool-generated warnings permit.
- no CMake/vcpkg dependency.
- `Pastral.slnx` is accepted by the available solution/MSBuild tooling; if current MSBuild cannot build `.slnx`, build the `.vcxproj` directly while keeping `.slnx` as solution authority and record the limitation.

### Static UI gates

PowerShell checks verify:

- required pages/resources exist;
- no hard-coded product canvas/background colors in page XAML;
- no WebView/Electron/Tauri references;
- no SQLite/storage imports in manager code;
- no unlabelled Debug synthetic provider in Release paths;
- required accessibility names/headings/status states exist;
- no placeholder `TODO`, `TBD`, or dead navigation entries.

### Runtime smoke

When the runtime/toolchain permits:

- launch x64 Debug manager;
- verify process creates a nonzero main window handle;
- verify it remains responsive during a bounded smoke interval;
- close it cleanly;
- record runtime prerequisites and skipped visual/manual assertions.

## 16. Commit sequence

1. `docs: specify manager UI foundation`
2. `build: add WinUI manager project`
3. `feat: add manager shell and design resources`
4. `feat: add Home and History states`
5. `ci: verify native manager UI`
6. `docs: record manager UI evidence`

Commits may combine adjacent test-first changes when splitting would leave `main` intentionally broken.

## 17. Non-goals and residual risks

Not implemented in this phase:

- IPC, live agent connection, real history/search, paste/copy dispatch;
- Quick Paste window/activation;
- passive overlay;
- packaging/signing/installer;
- full navigation destination set;
- Private profile functionality;
- automated UIA/Narrator screenshot testing;
- performance budgets and startup optimization beyond measurement scaffolding;
- final logo/store assets;
- production localization beyond resource-ready structure.

A successful Phase 3A proves the native manager build and reusable UI contracts. It does not prove end-to-end clipboard operation or release-ready accessibility.