# ADR 0002: C++/WinRT and WinUI 3 manager

**Status:** Accepted
**Date:** 2026-08-03
**Amended:** 2026-08-04 — select the supported Visual Studio/MSBuild C++ WinUI project path; do not make experimental Windows App SDK CMake integration a release dependency.

## Context

The manager requires native Windows navigation, virtualization, settings, dialogs, keyboard behavior, UI Automation, high contrast, text scaling, localization, touch, and modern shell materials. Rebuilding these capabilities in a custom Rust UI would divert effort from clipboard correctness.

## Decision

Implement `pastral-manager.exe` in C++20 using C++/WinRT and WinUI 3 from Windows App SDK stable.

Initial version decision: Windows App SDK 2.3.1 stable, revalidated during repository bootstrap.

Build-system decision:

- use a Visual Studio C++ WinUI 3 `.vcxproj` generated from or kept structurally aligned with the current stable Windows App SDK template;
- use MSBuild/XAML compiler/NuGet integration as the authoritative manager build path, organized in `Pastral.slnx`; legacy `.sln` fallback requires recorded tooling evidence;
- use a Windows Application Packaging Project (`.wapproj`) for the multi-executable MSIX under ADR 0009;
- do not require Windows App SDK CMake integration while Microsoft labels it experimental;
- a top-level PowerShell build orchestrator may call Cargo and MSBuild with pinned discovery/tool versions; it does not replace either native build graph.

The manager:

- runs only on user request;
- communicates through versioned IPC;
- never opens SQLite or blob files directly;
- contains presentation and interaction logic, not duplicated domain/rule/storage logic;
- uses native controls and UI Automation patterns wherever possible.

## Consequences

Positive:

- modern Windows-native controls and design language;
- stronger accessibility baseline than a custom control toolkit;
- supported deployment and servicing path.

Costs:

- MSBuild/NuGet/Windows App SDK/XAML tooling alongside Cargo and a cross-toolchain orchestration script;
- IPC view-model mapping;
- Windows App SDK startup and packaging must be measured.

## Alternatives considered

- **All-Rust custom Win32 UI:** lower language count but excessive UI, accessibility, localization, and virtualization burden.
- **WPF/C# manager:** mature and accessible but introduces .NET and a different visual/runtime direction.
- **WebView/Electron/Tauri:** rejected as primary UI.

## Review triggers

- WinUI 3 cannot meet required startup/virtualization/accessibility behavior;
- a stable Windows App SDK update fixes a blocking issue or introduces regression;
- packaging or CI architecture requires a supported alternative;
- Windows App SDK CMake support becomes stable and demonstrates a simpler reproducible path without weakening XAML/package tooling;
- manager working set/startup materially violates budgets with no mitigation.
