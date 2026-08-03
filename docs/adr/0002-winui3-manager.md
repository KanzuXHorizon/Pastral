# ADR 0002: C++/WinRT and WinUI 3 manager

**Status:** Accepted
**Date:** 2026-08-03

## Context

The manager requires native Windows navigation, virtualization, settings, dialogs, keyboard behavior, UI Automation, high contrast, text scaling, localization, touch, and modern shell materials. Rebuilding these capabilities in a custom Rust UI would divert effort from clipboard correctness.

## Decision

Implement `pastral-manager.exe` in C++20 using C++/WinRT and WinUI 3 from Windows App SDK stable.

Initial version decision: Windows App SDK 2.3.1 stable, revalidated during repository bootstrap.

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

- CMake/MSBuild/vcpkg/Windows App SDK integration alongside Cargo;
- IPC view-model mapping;
- Windows App SDK startup and packaging must be measured.

## Alternatives considered

- **All-Rust custom Win32 UI:** lower language count but excessive UI, accessibility, localization, and virtualization burden.
- **WPF/C# manager:** mature and accessible but introduces .NET and a different visual/runtime direction.
- **WebView/Electron/Tauri:** rejected as primary UI.

## Review triggers

- WinUI 3 cannot meet required startup/virtualization/accessibility behavior;
- a stable Windows App SDK update fixes a blocking issue or introduces regression;
- packaging architecture requires a supported alternative;
- manager working set/startup materially violates budgets with no mitigation.
