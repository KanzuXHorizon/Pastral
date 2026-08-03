# Pastral Manager UI Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify the first professional native Pastral manager executable with reusable WinUI design resources, an adaptive shell, honest operational states, and production-structured Home and History surfaces.

**Architecture:** Add one unpackaged x64 C++20/C++/WinRT WinUI 3 manager `.vcxproj` under `apps/manager`, organized by a root `Pastral.slnx`. The manager consumes presentation-only snapshots from an internal provider abstraction; Release defaults to disconnected/no-data while Debug can expose clearly labeled bounded synthetic data. No manager code reads SQLite or duplicates Rust domain/storage logic.

**Tech Stack:** Windows App SDK `2.3.1`, Microsoft.Windows.CppWinRT `3.0.260715.1`, C++20, C++/WinRT, WinUI 3 XAML, Visual Studio 2022 Build Tools 17.14/v143, Windows SDK `10.0.26100.0`, MSBuild, PowerShell 5.1, GitHub Actions `windows-2025`.

## Global Constraints

- Windows 11 x64 first.
- Use `.vcxproj`/MSBuild/XAML; do not add CMake or vcpkg.
- `Pastral.slnx` is solution authority; direct `.vcxproj` build is an allowed local fallback only if the installed MSBuild cannot consume `.slnx`.
- Manager is unpackaged in this slice with `WindowsPackageType=None`; no `.wapproj`, package identity, installer, or signing files.
- Manager never opens SQLite/blob files and contains no Rust/storage dependency.
- Windows App SDK and C++/WinRT package versions are pinned centrally.
- Release builds contain no synthetic clip data.
- Debug synthetic data is bounded, non-sensitive, and visibly announced as synthetic.
- Use Windows semantic brushes, native controls, Segoe UI Variable through the OS, and Mica with system fallback.
- Brand violet/cyan is limited to identity/provenance accents and never replaces semantic state colors.
- Every page implements loading, disconnected, empty/no-results, paused/mismatch/error, and content states as applicable.
- Keyboard, UI Automation names/headings, high contrast, long strings, narrow layouts, and reduced-motion-compatible structure are required from the first slice.
- No `TODO`, `TBD`, dead navigation, fake integrations, productivity scores, unsupported Private-profile functionality, or fabricated product claims.

---

### Task 1: Native manager verification contract

**Files:**
- Create: `eng/verify-native-manager.ps1`
- Modify: `eng/build.ps1`
- Modify: `eng/verify-source-policy.ps1`

**Interfaces:**
- Produces: `eng/verify-native-manager.ps1` with `-Mode Static|Build|Smoke|All`.
- Produces: `eng/build.ps1 -Task Manager` and `-Task NativePolicy`.
- Consumes later: `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj` and its XAML/source graph.

- [ ] **Step 1: Write the failing native-manager verifier**

The script resolves repository root and validates these exact paths:

```powershell
$required = @(
    'Pastral.slnx',
    'Directory.Build.props',
    'Directory.Packages.props',
    'apps/manager/Pastral.Manager/Pastral.Manager.vcxproj',
    'apps/manager/Pastral.Manager/App.xaml',
    'apps/manager/Pastral.Manager/MainWindow.xaml',
    'apps/manager/Pastral.Manager/Themes/PastralTheme.xaml',
    'apps/manager/Pastral.Manager/Pages/HomePage.xaml',
    'apps/manager/Pastral.Manager/Pages/HistoryPage.xaml'
)
```

Static checks must fail on:

- missing required file;
- `WebView`, `Electron`, `Tauri`, `sqlite`, `rusqlite`, `ATTACH DATABASE`, network APIs, or hard-coded database paths in manager product code;
- `TODO`, `TBD`, `FIXME`, or dead `NavigationViewItem` tags;
- synthetic clip strings outside a `#if defined(_DEBUG)` guarded provider implementation;
- missing `AutomationProperties.Name` on search/results/status/primary actions;
- missing `AutomationProperties.HeadingLevel` on Home/History page titles;
- page-level hard-coded `Background="#..."` or `Foreground="#..."` values;
- missing `WindowsPackageType=None`, `UseWinUI=true`, x64 configuration, C++20, warnings-as-errors, or exact PackageReferences.

`-Mode Build` locates MSBuild with `vswhere.exe` and runs:

```powershell
MSBuild.exe apps/manager/Pastral.Manager/Pastral.Manager.vcxproj `
  /restore /m /p:Configuration=Debug /p:Platform=x64 /p:RestoreLockedMode=true
MSBuild.exe apps/manager/Pastral.Manager/Pastral.Manager.vcxproj `
  /restore /m /p:Configuration=Release /p:Platform=x64 /p:RestoreLockedMode=true
```

`-Mode Smoke` starts the Debug executable, waits up to 15 seconds for a nonzero `MainWindowHandle`, checks it remains alive for two seconds, then closes it through `CloseMainWindow()` with bounded forced cleanup only after the close timeout.

- [ ] **Step 2: Run the verifier to prove red**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\eng\verify-native-manager.ps1 -Mode Static
```

Expected: nonzero exit with missing `Pastral.slnx` and manager project paths.

- [ ] **Step 3: Add build dispatcher entries**

Add exact tasks:

```powershell
'Manager' { Invoke-Step 'Verify native manager' { & "$PSScriptRoot\verify-native-manager.ps1" -Mode All } }
'NativePolicy' { Invoke-Step 'Verify native manager policy' { & "$PSScriptRoot\verify-native-manager.ps1" -Mode Static } }
```

Do not add `Manager` into `All` until the project builds green in Task 7.

Update source policy so native C++ `unsafe` terminology is irrelevant while existing Rust `unsafe` boundary rules remain unchanged. Add native output ignores/checks for `apps/manager/**/Generated Files`, `x64`, `AppPackages`, and `.vs` without weakening secret/path rules.

- [ ] **Step 4: Verify script syntax and commit**

Run:

```powershell
powershell.exe -NoProfile -Command "[void][scriptblock]::Create((Get-Content .\eng\verify-native-manager.ps1 -Raw))"
powershell.exe -NoProfile -Command "[void][scriptblock]::Create((Get-Content .\eng\build.ps1 -Raw))"
```

Commit:

```powershell
git add eng/verify-native-manager.ps1 eng/build.ps1 eng/verify-source-policy.ps1
git commit -m "test: define manager UI verification gates"
```

---

### Task 2: Native build authority and minimal WinUI executable

**Files:**
- Create: `Directory.Build.props`
- Create: `Directory.Packages.props`
- Create: `Pastral.slnx`
- Create: `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj`
- Create: `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj.filters`
- Create: `apps/manager/Pastral.Manager/app.manifest`
- Create: `apps/manager/Pastral.Manager/pch.h`
- Create: `apps/manager/Pastral.Manager/pch.cpp`
- Create: `apps/manager/Pastral.Manager/App.idl`
- Create: `apps/manager/Pastral.Manager/App.xaml`
- Create: `apps/manager/Pastral.Manager/App.xaml.h`
- Create: `apps/manager/Pastral.Manager/App.xaml.cpp`
- Create: `apps/manager/Pastral.Manager/MainWindow.idl`
- Create: `apps/manager/Pastral.Manager/MainWindow.xaml`
- Create: `apps/manager/Pastral.Manager/MainWindow.xaml.h`
- Create: `apps/manager/Pastral.Manager/MainWindow.xaml.cpp`

**Interfaces:**
- Produces: `pastral-manager.exe` x64 Debug/Release.
- Produces: WinUI `App` and `MainWindow` runtime classes.
- Consumes: centrally pinned NuGet package versions.

- [ ] **Step 1: Add central native properties and packages**

`Directory.Build.props` must define:

```xml
<Project>
  <PropertyGroup>
    <WindowsTargetPlatformVersion>10.0.26100.0</WindowsTargetPlatformVersion>
    <WindowsTargetPlatformMinVersion>10.0.26100.0</WindowsTargetPlatformMinVersion>
    <PreferredToolArchitecture>x64</PreferredToolArchitecture>
    <LangVersion>latest</LangVersion>
  </PropertyGroup>
</Project>
```

`Directory.Packages.props` must enable central package management and pin:

```xml
<PackageVersion Include="Microsoft.WindowsAppSDK" Version="2.3.1" />
<PackageVersion Include="Microsoft.Windows.CppWinRT" Version="3.0.260715.1" />
```

- [ ] **Step 2: Add solution and project graph**

`Pastral.slnx` contains one project path:

```xml
<Solution>
  <Project Path="apps/manager/Pastral.Manager/Pastral.Manager.vcxproj" />
</Solution>
```

The `.vcxproj` must include only `Debug|x64` and `Release|x64`, and set:

```xml
<RootNamespace>Pastral.Manager</RootNamespace>
<TargetName>pastral-manager</TargetName>
<WindowsPackageType>None</WindowsPackageType>
<AppxPackage>false</AppxPackage>
<AppContainerApplication>false</AppContainerApplication>
<ApplicationType>Windows Store</ApplicationType>
<ApplicationTypeRevision>10.0</ApplicationTypeRevision>
<UseWinUI>true</UseWinUI>
<CppWinRTOptimized>true</CppWinRTOptimized>
<CppWinRTRootNamespaceAutoMerge>true</CppWinRTRootNamespaceAutoMerge>
<WindowsAppSDKSelfContained>false</WindowsAppSDKSelfContained>
<PlatformToolset>v143</PlatformToolset>
<LanguageStandard>stdcpp20</LanguageStandard>
<ConformanceMode>true</ConformanceMode>
<TreatWarningAsError>true</TreatWarningAsError>
```

Use PackageReference entries without local version attributes because versions come from `Directory.Packages.props`.

- [ ] **Step 3: Add minimal C++/WinRT app classes**

`App.idl` defines `runtimeclass App : Microsoft.UI.Xaml.Application` and `MainWindow.idl` defines `runtimeclass MainWindow : Microsoft.UI.Xaml.Window`.

`App.xaml.cpp` contains the standard C++/WinRT `wWinMain` entry point:

```cpp
int __stdcall wWinMain(HINSTANCE, HINSTANCE, PWSTR, int)
{
    winrt::init_apartment(winrt::apartment_type::single_threaded);
    winrt::Microsoft::UI::Xaml::Application::Start(
        [](auto&&) { winrt::make<Pastral::Manager::implementation::App>(); });
}
```

`OnLaunched` creates and activates one `MainWindow` retained by the `App` object.

`MainWindow.xaml` initially contains a `Grid` and `TextBlock` named for automation so XAML compilation and launch can be isolated before the full shell.

- [ ] **Step 4: Restore/build and fix only template-alignment failures**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\eng\verify-native-manager.ps1 -Mode Build
```

Expected: both x64 configurations build. If `.slnx` cannot be consumed by installed MSBuild, record that and keep project-direct builds as the evidence path; do not replace `.slnx` with `.sln` without an ADR change.

- [ ] **Step 5: Launch minimal executable and commit**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\eng\verify-native-manager.ps1 -Mode Smoke
```

Expected: nonzero window handle and clean bounded close.

Commit:

```powershell
git add Directory.Build.props Directory.Packages.props Pastral.slnx apps/manager/Pastral.Manager
git commit -m "build: add WinUI manager project"
```

---

### Task 3: Shared design resources and adaptive manager shell

**Files:**
- Create: `apps/manager/Pastral.Manager/Themes/PastralTheme.xaml`
- Modify: `apps/manager/Pastral.Manager/App.xaml`
- Modify: `apps/manager/Pastral.Manager/MainWindow.xaml`
- Modify: `apps/manager/Pastral.Manager/MainWindow.xaml.h`
- Modify: `apps/manager/Pastral.Manager/MainWindow.xaml.cpp`
- Create: `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw`

**Interfaces:**
- Produces resource keys: `PastralBrandPrimaryBrush`, `PastralBrandSecondaryBrush`, `PastralSectionSpacing`, `PastralContentMaxWidth`, `PastralCompactCornerRadius`, `PastralCardStyle`, `PastralPageHeaderStyle`, `PastralMetadataTextStyle`.
- Produces navigation tags: `home`, `history`, `settings`, `diagnostics`, `about` only when destination exists; initially only `home` and `history` are enabled/visible.
- Produces `NavigateTo(std::wstring_view tag)`.

- [ ] **Step 1: Extend static verifier for shell contracts**

Require:

- `MicaBackdrop` in `MainWindow.xaml`;
- `TitleBar`, `NavigationView`, `InfoBar`, and `Frame` names;
- Home/History navigation items only;
- system theme resources in normal surface/background states;
- brand colors only in `PastralTheme.xaml`;
- accessible names for title bar, navigation, global status, and content frame.

Run Static mode and confirm it fails for missing shell resources.

- [ ] **Step 2: Add resource dictionary**

Define brand colors exactly once in `PastralTheme.xaml`:

```xml
<Color x:Key="PastralAuroraVioletColor">#725CFF</Color>
<Color x:Key="PastralSignalCyanColor">#2ED3FF</Color>
```

Expose brushes and shared styles while using `CardBackgroundFillColorDefaultBrush`, `TextFillColorPrimaryBrush`, `TextFillColorSecondaryBrush`, `ControlStrokeColorDefaultBrush`, and system status resources for operative UI.

Do not override global Button/ListView/NavigationView styles.

- [ ] **Step 3: Build the shell**

`MainWindow.xaml` uses:

```xml
<Window.SystemBackdrop>
  <MicaBackdrop Kind="Base" />
</Window.SystemBackdrop>
```

Root rows:

1. `TitleBar` with app title and compact synthetic/disconnected state subtitle.
2. `NavigationView` containing a top operational `InfoBar` and `Frame` content.

Use `PaneDisplayMode="Auto"`, no back button until navigation history exists, and system-handled caption buttons. Do not create custom window chrome outside the WinUI `TitleBar` control.

- [ ] **Step 4: Implement deterministic navigation**

On load, select Home. `SelectionChanged` maps exact tags to `xaml_typename<HomePage>()` or `xaml_typename<HistoryPage>()`. Unknown tags keep the current page and show the global error InfoBar.

- [ ] **Step 5: Add initial English resources and build**

Move shell-visible strings to `Strings/en-US/Resources.resw` with `x:Uid` or resource loader access. Include app title, navigation names, status titles, and error text.

Run:

```powershell
.\eng\verify-native-manager.ps1 -Mode Static
.\eng\verify-native-manager.ps1 -Mode Build
```

Commit:

```powershell
git add apps/manager/Pastral.Manager/App.xaml apps/manager/Pastral.Manager/MainWindow.* apps/manager/Pastral.Manager/Themes apps/manager/Pastral.Manager/Strings
git commit -m "feat: add native manager shell and design system"
```

---

### Task 4: Presentation state model and honest provider boundary

**Files:**
- Create: `apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.idl`
- Create: `apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.h`
- Create: `apps/manager/Pastral.Manager/ViewModels/ClipPreviewViewModel.cpp`
- Create: `apps/manager/Pastral.Manager/ViewModels/ManagerState.h`
- Create: `apps/manager/Pastral.Manager/ViewModels/ManagerState.cpp`
- Create: `apps/manager/Pastral.Manager/Services/IManagerDataProvider.h`
- Create: `apps/manager/Pastral.Manager/Services/ManagerDataProvider.h`
- Create: `apps/manager/Pastral.Manager/Services/ManagerDataProvider.cpp`
- Modify: `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj`
- Modify: `apps/manager/Pastral.Manager/Pastral.Manager.vcxproj.filters`

**Interfaces:**
- Produces `Pastral::Manager::ClipPreviewViewModel` with read-only WinRT properties.
- Produces plain C++ `ConnectionState`, `ManagerSnapshot`, and `IManagerDataProvider::LoadSnapshot()`.
- Produces `std::shared_ptr<IManagerDataProvider> CreateManagerDataProvider()`.

- [ ] **Step 1: Add verifier checks for Release-data honesty**

Static verifier must scan `Services/ManagerDataProvider.cpp` and require:

```cpp
#if defined(_DEBUG)
```

around every synthetic `ClipPreviewData` literal. The `#else` path must return a disconnected snapshot with an empty clip vector. It must fail if synthetic preview strings appear in any other product file.

Run Static mode and confirm red before the provider exists.

- [ ] **Step 2: Define view-model types**

`ClipPreviewViewModel.idl` defines these properties:

```idl
String Id{ get; };
String SafePreview{ get; };
String Source{ get; };
String RelativeTime{ get; };
String TypeLabel{ get; };
String Profile{ get; };
String RepresentationSummary{ get; };
String AutomationName{ get; };
Boolean Pinned{ get; };
Boolean Unavailable{ get; };
```

The implementation is immutable after construction and contains no payload bytes.

- [ ] **Step 3: Define snapshot/provider boundary**

Use exact state enum:

```cpp
enum class ConnectionState {
    Loading,
    Connected,
    Disconnected,
    CapturePaused,
    ProtocolMismatch,
    Error,
};
```

`ManagerSnapshot` contains status title/detail, active profile, storage summary text, `std::vector<ClipPreviewData>`, and `bool synthetic`.

- [ ] **Step 4: Implement bounded Debug provider and empty Release provider**

Debug returns six synthetic clips covering text, code, URL, image, pinned, and unavailable states. All examples are generic developer/test content and the snapshot sets `synthetic=true`.

Release returns:

```cpp
connection = ConnectionState::Disconnected;
statusTitle = L"Pastral agent is not connected";
statusDetail = L"History and paste actions will become available after the versioned local IPC service is implemented.";
clips = {};
synthetic = false;
```

- [ ] **Step 5: Build both configurations and commit**

Run Debug and Release builds so preprocessor separation is compiled in both paths.

Commit:

```powershell
git add apps/manager/Pastral.Manager/ViewModels apps/manager/Pastral.Manager/Services apps/manager/Pastral.Manager/Pastral.Manager.vcxproj*
git commit -m "feat: add manager presentation state boundary"
```

---

### Task 5: Home page with operational and empty states

**Files:**
- Create: `apps/manager/Pastral.Manager/Pages/HomePage.idl`
- Create: `apps/manager/Pastral.Manager/Pages/HomePage.xaml`
- Create: `apps/manager/Pastral.Manager/Pages/HomePage.xaml.h`
- Create: `apps/manager/Pastral.Manager/Pages/HomePage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw`
- Modify: project/filter files.

**Interfaces:**
- Produces Home page properties: `RecentClips`, `StatusTitle`, `StatusDetail`, `ActiveProfile`, `StorageSummary`, `IsSynthetic`, `HasRecentClips`, `IsDisconnected`.
- Consumes `CreateManagerDataProvider()`.

- [ ] **Step 1: Add Home static acceptance checks**

Require in Home XAML:

- page heading with `HeadingLevel="Level1"`;
- operational state region with accessible name;
- current profile label;
- capture/connection status;
- primary recovery button with accessible name;
- recent clips list with accessible name;
- explicit empty/disconnected panel;
- no productivity score/chart words.

Run Static mode and confirm red.

- [ ] **Step 2: Implement page state population**

On construction, load one snapshot and map clip structs to an `IObservableVector<ClipPreviewViewModel>`. Set UI state properties before calling `Bindings().Update()`.

No asynchronous fake delay is added. Loading state exists in XAML and will be used by the future IPC provider; this synchronous provider transitions immediately.

- [ ] **Step 3: Build professional Home composition**

Use a `ScrollViewer` and max-width content column with:

- page title/status text;
- operational state card with profile, capture, agent state, and one recovery action;
- recent clips section using a `ListView` with shared row DataTemplate;
- storage/retention section using honest synthetic/unavailable text;
- getting-started section visible only when disconnected.

Use `InfoBadge`/icons only as redundant support. Every state includes text.

- [ ] **Step 4: Add light/dark/high-contrast-safe resources and copy**

All status surfaces use theme brushes and `InfoBarSeverity`; no brand hue communicates health/error state.

- [ ] **Step 5: Build, static verify, and commit**

Commit:

```powershell
git add apps/manager/Pastral.Manager/Pages/HomePage.* apps/manager/Pastral.Manager/Strings apps/manager/Pastral.Manager/Pastral.Manager.vcxproj*
git commit -m "feat: add professional manager Home states"
```

---

### Task 6: Adaptive History page and selection details

**Files:**
- Create: `apps/manager/Pastral.Manager/Pages/HistoryPage.idl`
- Create: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml`
- Create: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.h`
- Create: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml.cpp`
- Modify: `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw`
- Modify: project/filter files.

**Interfaces:**
- Produces `Items`, `SelectedItem`, `Query`, `HasResults`, `IsDisconnected`, `IsSynthetic`, `IsDetailPaneVisible`.
- Produces handlers: `SearchBox_TextChanged`, `ResultsList_SelectionChanged`, `ClearFilters_Click`, `Retry_Click`.

- [ ] **Step 1: Add History static acceptance checks**

Require:

- Level1 page heading;
- named search box, result list, result-count live region, detail region, primary action, and no-results panel;
- `VisualStateManager` with `WideLayout` and `NarrowLayout` using an adaptive trigger;
- search remains visible in both layouts;
- disabled Paste/Copy buttons include explanatory accessible help text;
- result DataTemplate includes safe preview, source/time, type/representation, profile, pinned/unavailable text.

Run Static mode and confirm red.

- [ ] **Step 2: Implement bounded local Debug filtering**

Keep an immutable in-memory copy of the provider's bounded Debug clip list. `TextChanged` performs case-insensitive matching over safe preview, source, type, profile, and representation summary. It never parses fake production query syntax.

Release disconnected state has no items and no local search behavior beyond preserving entered text.

- [ ] **Step 3: Implement selection and details**

The first result is selected on initial Debug load. Selection updates the detail pane and its automation name. Paste and Copy actions remain disabled with text explaining that local IPC is unavailable.

Unavailable items display an explicit warning and cannot become actionable.

- [ ] **Step 4: Implement adaptive topology**

Wide mode uses a two-column `Grid` with result list and detail pane. Narrow mode moves detail below results or hides it behind an explicit Details action while retaining search and the selected primary action label. Use native NavigationView adaptation plus page-level `AdaptiveTrigger MinWindowWidth="920"`.

- [ ] **Step 5: Verify no-results, disconnected, synthetic, and selected states**

Add code-path assertions to the native verifier by inspecting required named state elements and Debug/Release provider branches. Build both configurations and launch Debug smoke.

- [ ] **Step 6: Commit**

```powershell
git add apps/manager/Pastral.Manager/Pages/HistoryPage.* apps/manager/Pastral.Manager/Strings apps/manager/Pastral.Manager/Pastral.Manager.vcxproj*
git commit -m "feat: add adaptive History and details states"
```

---

### Task 7: Native CI, complete build orchestration, and documentation

**Files:**
- Create: `.github/workflows/native-ui-ci.yml`
- Modify: `eng/build.ps1`
- Modify: `eng/verify-toolchain.ps1`
- Modify: `.gitignore`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`

**Interfaces:**
- Produces full local `-Task Manager` and static `-Task NativePolicy` gates.
- Produces Windows CI for restore/build/static checks.

- [ ] **Step 1: Extend toolchain verification**

Report, without changing global installs:

- Visual Studio Build Tools version and path;
- MSBuild version;
- v143 toolset;
- Windows SDK `10.0.26100.0` presence;
- Windows App Runtime `Microsoft.WindowsAppRuntime.2` version `2.3.1.0` for local smoke;
- C++ WinUI workload/template absence as a documented non-blocker because the project uses pinned NuGet/MSBuild inputs.

Required build prerequisites fail the native manager task; missing runtime fails Smoke only, not compile.

- [ ] **Step 2: Add native CI**

Workflow triggers on manager, native build props/packages, `Pastral.slnx`, `eng/**`, and its own workflow. On `windows-2025`:

```powershell
./eng/verify-toolchain.ps1
./eng/verify-native-manager.ps1 -Mode Static
./eng/verify-native-manager.ps1 -Mode Build
./eng/verify-source-policy.ps1
```

Do not run interactive Smoke on hosted CI.

- [ ] **Step 3: Complete build dispatcher**

Keep Rust `All` behavior stable. Add a new `Full` task that runs Rust `All` then native manager Static+Build; do not silently make existing Rust-only `All` unexpectedly require Windows App SDK packages on unrelated changes.

- [ ] **Step 4: Update documentation honestly**

README and developer setup state:

- Phase 3A manager exists;
- Release manager is disconnected until IPC exists;
- Debug synthetic data is labeled and excluded from Release;
- exact packages/toolchain/build commands;
- unpackaged runtime prerequisite;
- Quick Paste, overlay, IPC, packaging, live history/search/paste remain absent.

- [ ] **Step 5: Run gates and commit**

```powershell
.\eng\verify-native-manager.ps1 -Mode All
.\eng\build.ps1 -Task Full
.\eng\verify-source-policy.ps1
git diff --check
```

Commit:

```powershell
git add .github/workflows/native-ui-ci.yml eng .gitignore README.md CHANGELOG.md docs/operations/developer-setup.md
git commit -m "ci: verify native manager UI foundation"
```

---

### Task 8: Visual/static quality pass and milestone evidence

**Files:**
- Modify only files with demonstrated issues from the quality pass.
- Create: `docs/reviews/phase-3a-manager-ui-foundation-verification.md`

**Interfaces:**
- Produces final evidence tied to the exact implementation commit.

- [ ] **Step 1: Load Impeccable craft floor before UI edits**

Read `C:/Users/KanzuWakazaki/.agents/skills/impeccable/reference/craft-floor.md`. Review every XAML surface against `PRODUCT.md`, `DESIGN.md`, and the Phase 3A design spec.

- [ ] **Step 2: Run the Impeccable detector once**

Run after all UI changes:

```powershell
node C:\Users\KanzuWakazaki\.agents\skills\impeccable\scripts\detect.mjs --json `
  apps/manager/Pastral.Manager/MainWindow.xaml `
  apps/manager/Pastral.Manager/Themes/PastralTheme.xaml `
  apps/manager/Pastral.Manager/Pages/HomePage.xaml `
  apps/manager/Pastral.Manager/Pages/HistoryPage.xaml
```

Apply only findings that match native WinUI semantics and the established design authority. Record tool limitations if the detector assumes web/CSS.

- [ ] **Step 3: Run fresh final verification**

```powershell
git status --short
.\eng\verify-toolchain.ps1
.\eng\verify-native-manager.ps1 -Mode Static
.\eng\verify-native-manager.ps1 -Mode Build
.\eng\verify-native-manager.ps1 -Mode Smoke
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
git diff --check
git fsck --no-progress --no-dangling
git show --check HEAD
```

- [ ] **Step 4: Write evidence report**

Record:

- baseline/final commits;
- OS, shell, Visual Studio/MSBuild/v143/Windows SDK/Windows App Runtime;
- package versions;
- Debug/Release build results;
- runtime smoke result and window handle evidence;
- Rust test count and native static gate count;
- enabled pages/states;
- synthetic Release exclusion evidence;
- accessibility/static design checks;
- exact skipped checks: UIA automation, Narrator, screenshot comparison, 300% DPI, high contrast, RDP, touch, packaging, IPC, live data, Quick Paste, overlay;
- residual risks and next slice recommendation.

- [ ] **Step 5: Commit evidence and final smoke**

```powershell
git add docs/reviews/phase-3a-manager-ui-foundation-verification.md
git commit -m "docs: record manager UI foundation evidence"
.\eng\verify-native-manager.ps1 -Mode All
git status --short
git show --check HEAD
```
