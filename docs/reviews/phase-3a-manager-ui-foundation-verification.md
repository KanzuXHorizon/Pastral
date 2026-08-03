# Phase 3A native manager UI foundation verification

**Date:** 2026-08-04

**Repository:** `F:\Pastral`

**Branch:** `main`

**Baseline commit:** `0b5c86d`

**Evidence commit before this report:** `a65b845`

**Scope:** native C++/WinRT WinUI 3 manager bootstrap, design system, shell, Home, History, presentation boundary, local verification, and CI definitions

## 1. Outcome

Phase 3A establishes a buildable and launchable native manager UI foundation without pretending that the resident agent, IPC, live clipboard history, or paste replay already exist.

Verified outcomes:

- unpackaged x64 C++20/C++/WinRT WinUI 3 manager project;
- Windows App SDK `2.3.1` and Microsoft.Windows.CppWinRT `3.0.260715.1` pinned through central package management;
- locked NuGet restore through `packages.lock.json` version `2`;
- Debug and Release x64 MSBuild builds;
- Mica/TitleBar/NavigationView native shell;
- shared system-brush-based visual resources with restrained Pastral brand colors;
- localized Home and History surfaces;
- immutable privacy-safe presentation view model;
- provider boundary that does not open SQLite, blobs, clipboard handles, network, or IPC;
- six clearly labeled Debug-only synthetic preview records;
- empty disconnected Release provider;
- Home operational, retry, summary, recent-item, synthetic, and empty states;
- adaptive History search, list, selection details, unavailable state, no-results state, and disabled-action explanations;
- UI Automation navigation, filtering, detail update, no-results, and clean-close verification;
- Rust foundation quality gates remain green.

The current manager is a UI and presentation-contract foundation, not an end-to-end clipboard application.

## 2. Environment evidence

### Operating system and shell

- OS: Microsoft Windows 11 Pro
- OS version: `10.0.26200`
- OS build: `26200`
- Shell: Windows PowerShell Desktop
- PowerShell: `5.1.26100.8972`

### Rust

- rustc: `1.97.1 (8bab26f4f 2026-07-14)`
- cargo: `1.97.1 (c980f4866 2026-06-30)`
- active toolchain: `1.97.1-x86_64-pc-windows-msvc`
- target: `x86_64-pc-windows-msvc`
- rustfmt: `1.9.0-stable`
- Clippy: `0.1.97`

### Native manager build

- Visual Studio Build Tools: `17.14.37516.0`
- MSBuild: `17.14.51.32402`
- MSVC toolset: `14.44.35207`
- MSVC host/target: `Hostx64\x64`
- C++ WinUI/UWP x64 platform toolset: v143
- Windows SDK used: `10.0.26100.0`
- Windows App SDK: `2.3.1`
- Microsoft.Windows.CppWinRT: `3.0.260715.1`
- Windows App Runtime smoke dependency: `Microsoft.WindowsAppRuntime.2` x64 `2.3.1.0`
- deferred packaging/release SDK `10.0.28000.2526`: not installed and not required by this unpackaged slice

### Visual Studio components added for this slice

The existing Build Tools installation required the official C++ WinUI/UWP build payloads. The selected component/workload authority is:

- `Microsoft.VisualStudio.Workload.UniversalBuildTools`;
- `Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools`;
- transitive C++ UWP x64 v143 build packages and Windows build targets.

The repository does not vendor Visual Studio payloads and does not add CMake or a vcpkg manifest.

## 3. Architecture and data-boundary evidence

### Manager process boundary

The manager:

- runs as an unpackaged on-demand WinUI process;
- does not open SQLite or blob files;
- does not call the clipboard platform directly;
- does not contain network clients;
- does not contain an IPC implementation yet;
- receives data only through `IManagerDataProvider` and immutable presentation records.

### Provider behavior

Debug branch:

- contains exactly six bounded generic records;
- uses IDs prefixed with `synthetic-clip-`;
- labels the data as synthetic on Home and History;
- does not read clipboard or storage data.

Release branch:

- sets `ConnectionState::Disconnected`;
- returns no clips;
- sets `snapshot.synthetic = false`;
- keeps history/search/paste unavailable rather than fabricating live state.

A binary scan produced:

```text
DEBUG_SYNTHETIC_MARKER=True
RELEASE_SYNTHETIC_MARKER=False
SYNTHETIC_BINARY_SEPARATION=PASS
```

### Presentation model

`ClipPreviewViewModel` exposes read-only properties for:

- ID;
- privacy-safe preview;
- source;
- relative time;
- type;
- profile;
- representation summary;
- accessibility name;
- textual state summary;
- pinned/unavailable state.

It does not expose payload bytes, blob references, file-system storage paths, SQLite row IDs, authorization tokens, clipboard handles, or mutable business operations.

## 4. UI and UX evidence

### Shared system

Implemented visual and interaction authority:

- Windows system/theme brushes for operative states;
- Mica backdrop with native fallback behavior supplied by WinUI;
- native `TitleBar` and `NavigationView`;
- Segoe UI Variable through Windows controls, with no redistributed font files;
- Aurora Violet `#725CFF` and Signal Cyan `#2ED3FF` limited to named brand resources;
- system focus, selection, warning, disabled, card, divider, and text resources;
- semantic headings and explicit Automation names;
- no gradient-filled application shell, decorative glass-card stack, productivity score, AI sparkle, or icon-only destructive action.

The Impeccable mechanical detector was run once after the UI implementation and returned zero findings for:

- `MainWindow.xaml`;
- `HomePage.xaml`;
- `HistoryPage.xaml`;
- `PastralTheme.xaml`.

### Home

Verified structure includes:

- Level 1 page heading;
- connection/status InfoBar;
- Debug synthetic disclosure;
- retry action;
- capture/profile/local-data summaries;
- recent clips list with safe preview/source/time/type/profile/representation metadata;
- honest Release empty state.

### History

Verified structure includes:

- Level 1 heading and polite result-count live region;
- search over presentation-safe fields only;
- adaptive wide and narrow layouts at a `920` effective-pixel trigger;
- results list and selected-detail region;
- source, type, representations, profile, and textual state details;
- unavailable-item warning;
- disabled Paste and Copy actions with accessible HelpText;
- disconnected, empty, query-no-results, and synthetic states.

### Runtime UI Automation

The Debug manager was launched and tested through Windows UI Automation. The test:

1. resolved the top-level HWND;
2. located and selected the History navigation item;
3. found the Search, results list, selected-detail, and availability landmarks;
4. entered `Terminal` through `ValuePattern`;
5. observed the live result count `1 items`;
6. confirmed details updated to `Windows Terminal · 8 min ago`;
7. entered a non-matching query;
8. observed `No matching clips`;
9. closed the window and verified process exit within five seconds.

Fresh result:

```text
Manager UI Automation History navigation, filtering, selection, and no-results states: PASS
Native manager runtime smoke: PASS
```

## 5. Red/green evidence

No intentionally failing state was committed. Red evidence was collected locally, followed by the minimum corrective change and fresh green verification.

### Repository/project bootstrap

Red:

- static verifier failed while `Pastral.slnx`, the manager project, XAML authority, and native package pins did not exist.

Green:

- static policy passes with the project, central package pins, locked restore, and solution authority present.

### Native toolchain

Red:

- initial Build Tools installation lacked the C++ WinUI/UWP x64 application-type platform toolset;
- MSBuild reported missing Windows Store/UWP v143 targets.

Green:

- official Visual Studio Installer components were installed;
- `verify-toolchain.ps1 -RequireNativeManager` detects MSBuild, MSVC x64, UWP/WinUI x64 v143, and SDK `10.0.26100.0`.

### C++/WinRT/XAML bootstrap

Red findings included:

- `TargetPlatformVersion` evaluated as `7.0` before the UWP toolset was present;
- native NuGet target/RuntimeIdentifier mismatch;
- invalid import of nonexistent `Microsoft.UI.Xaml.idl`;
- `CppWinRTEnabled=true` forced the package into a wrong non-package executable path;
- `GetCurrentTime` Win32 macro collision;
- App declared as a public runtime class instead of template-aligned namespace-only IDL;
- duplicate manually defined and XAML-generated `wWinMain`;
- generated-page include path omitted the `Pages` subdirectory;
- parallel C++/MIDL tool tasks produced PDB, tracker, and compiler-crash races;
- App referenced a theme file before that file existed.

Green:

- Debug and Release build with locked restore;
- generated entry point is the sole entry-point authority;
- App and MainWindow follow the generated C++/WinRT/XAML contracts;
- `UseMultiToolTask=false`, `MultiProcessorCompilation=false`, `/m:1`, and `/nr:false` provide deterministic generation on the verified toolchain;
- the manager launches and closes cleanly.

### Presentation boundary

Red:

- static policy failed because immutable view model, snapshot, interface, and provider files were absent.

Green:

- Debug and Release provider branches compile;
- source policy confirms synthetic markers exist only in the provider source;
- binary scan confirms Release exclusion.

### Home

Red:

- static policy failed because Home lacked operational-state region, retry, synthetic disclosure, recent list, empty state, and safe bindings.

Green:

- static policy, Debug/Release build, and runtime smoke pass.

### History

Red:

- static policy failed because History lacked named search/results/details/no-results regions and handlers.

Green:

- static policy passes;
- Debug/Release build passes;
- UI Automation navigation, filtering, detail update, and no-results behavior pass.

### Rust formatting

Red:

- the first `Full` run found two rustfmt layout differences in test-only `clipboard-win` helpers.

Green:

- exact rustfmt output was applied;
- the subsequent `Full` run passed formatting, check, tests, Clippy, and docs.

## 6. Fresh commands and results

The following were run from Windows PowerShell against commit `a65b845` before this report was created.

### Rust-only aggregate

```powershell
.\eng\build.ps1 -Task All
```

Result: PASS.

### Rust plus native build aggregate

```powershell
.\eng\build.ps1 -Task Full
```

Result: PASS.

This includes:

- enforced Rust and native manager toolchain verification;
- `cargo fmt --all -- --check`;
- `cargo check --locked --workspace --all-targets`;
- `cargo test --locked --workspace --all-targets`;
- Clippy with `-D warnings`;
- Rust docs;
- dependency policy;
- repository source policy;
- native manager static policy;
- Debug and Release x64 manager builds.

### Interactive manager aggregate

```powershell
.\eng\build.ps1 -Task Manager
```

Result: PASS.

This includes:

- native static policy;
- locked Debug build;
- locked Release build;
- Windows App Runtime launch;
- top-level HWND creation;
- History UI Automation scenario;
- clean process shutdown.

### Rust test counts

```text
pastral-clipboard-win  17 passed
pastral-domain         26 passed
pastral-storage        30 passed
--------------------------------
Total                  73 passed
Failed                  0
Ignored                 0
```

### Native configurations

```text
Debug|x64   PASS
Release|x64 PASS
```

### Dependency graph

```powershell
cargo tree --locked --workspace
```

Result: PASS. The Rust graph remains limited to the reviewed domain/storage/clipboard dependencies. The manager NuGet graph is separate and locked through `packages.lock.json`.

### Source and secret policy

```powershell
.\eng\verify-source-policy.ps1
.\eng\verify-native-manager.ps1 -Mode Static
```

Result: PASS.

## 7. Files changed

`48` tracked files changed between `0b5c86d` and `a65b845`.

Primary groups:

- solution and package authority:
  - `Pastral.slnx`;
  - `Directory.Build.props`;
  - `Directory.Packages.props`;
  - manager `.vcxproj`, filters, manifest, NuGet lock;
- application/XAML bootstrap:
  - App, MainWindow, PCH, IDL;
- design system and localization:
  - `Themes/PastralTheme.xaml`;
  - `Strings/en-US/Resources.resw`;
- pages:
  - Home Page IDL/XAML/header/source;
  - History Page IDL/XAML/header/source;
- presentation boundary:
  - manager state;
  - immutable clip preview view model;
  - provider interface and implementation;
- quality gates:
  - toolchain verifier;
  - native manager verifier;
  - build orchestrator;
  - source policy;
  - Rust and native UI workflows;
- documentation:
  - design specification;
  - implementation plan;
  - README;
  - changelog;
  - developer setup;
- two rustfmt-only changes in `clipboard-win` test helpers.

No `x64`, `obj`, generated XAML/C++ projection, `.vs`, package output, credential, private key, or local launcher file is tracked.

## 8. Commits

```text
a65b845 docs: document native manager foundation
362df5b ci: integrate native manager quality gates
a8fda8f style: align clipboard foundation formatting
0eb9fef feat: add adaptive History and details states
04303e4 feat: add professional manager Home states
f7a1e7d feat: add manager presentation state boundary
f4d5050 feat: add native manager shell and design system
7c1f3e5 build: add WinUI manager project
d2651d5 test: define manager UI verification gates
a389bb0 docs: plan manager UI foundation
7e87e41 docs: specify manager UI foundation
```

## 9. Scope exclusions

This phase does not implement:

- resident capture agent;
- process IPC or Protobuf;
- live storage/search connection;
- direct manager database access;
- COM/OLE/delayed-rendering capture;
- Quick Paste;
- passive copy overlay;
- paste/replay engine;
- global hotkeys or tray;
- encryption or usable Private profile;
- rules/profiles/collections/settings/diagnostics pages;
- packaging, MSIX, installer, signing, update, or release channel;
- OCR, semantic search, AI, or networking.

Unavailable primary navigation destinations are not shown as dead placeholders.

## 10. Skipped checks and reasons

The following are not marked PASS:

- **Hosted GitHub Actions execution:** workflow definitions were added, but no remote push/PR was made, so hosted-runner installation/build behavior is not proven.
- **Actionlint or equivalent workflow semantic validation:** no repository-pinned workflow linter exists. Local PowerShell/native commands used by the workflows passed, but GitHub expression/runner behavior remains remote evidence.
- **Formal Narrator testing:** UI Automation landmarks and interaction patterns passed, but Narrator speech order and verbosity were not manually audited.
- **High contrast/contrast themes:** system brushes are used, but no formal visual comparison was captured.
- **100–300% text scaling and mixed DPI:** adaptive layout exists, but the full matrix was not manually executed.
- **RTL and long localization:** resources and wrapping exist, but only `en-US` content was built and launched.
- **Touch, pen, RDP, transparency-disabled, reduced-motion, and battery-saver states:** not exercised in this slice.
- **Screenshot before/after comparison:** no production UI baseline existed and no matched screenshot review was recorded. Runtime structure was verified through live WinUI launch and UI Automation.
- **Startup, first-frame, working-set, and virtualization benchmarks:** no performance claim is made.
- **Packaging/signing/Store identity:** intentionally excluded.
- **Live agent/storage/clipboard flow:** impossible until IPC and agent slices exist.

## 11. Residual risks

- Windows App SDK `2.3.1` pulls a broader transitive NuGet graph than the manager currently uses, including WebView2 and AI-related metadata packages. Product source does not use these capabilities, and the source policy rejects WebView/product-network code, but package-footprint reduction has not yet been benchmarked or selected.
- The verified Build Tools combination required serial C++/MIDL execution to avoid generation/PDB races. This is deterministic but slower; a future Visual Studio/CppWinRT servicing update should be tested before relaxing it.
- Debug synthetic data validates common text, code, link, image, pinned, and unavailable presentation states, but it does not represent adversarial maximum-length localization or all future clipboard formats.
- Disabled Paste/Copy buttons communicate the current boundary, but real authorization, destination validation, and replay behavior remain unproven.
- Accessibility names and UIA behavior can regress when virtualization/live IPC is introduced; these tests must evolve with that slice.
- The public product name remains provisional and lacks legal clearance.

## 12. What this report does not prove

This report does not prove that Pastral captures clipboard contents, persists live history, searches the real database through IPC, restores focus, pastes into applications, protects Private-profile content, meets startup/resource budgets, passes the full accessibility matrix, or is package/release ready.

It proves that the selected native manager technology and build path are reproducible on the recorded local Windows environment, that the initial professional Home/History UI behaves through its current presentation contract, that Release does not embed synthetic history markers, and that all prior Rust foundations remain green.
