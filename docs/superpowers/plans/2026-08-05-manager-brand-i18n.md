# Pastral Manager Brand and Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernize the existing Pastral WinUI 3 manager shell, Home, and History surfaces while adding a coherent brand mark and complete English/Vietnamese static localization.

**Architecture:** Keep the current WinUI 3 NavigationView, Frame navigation, provider boundary, and page code-behind behavior. Add reusable brand/layout resources in the shared theme, localize static XAML through matching `.resw` keys, and refine only existing functional surfaces rather than introducing fake navigation destinations.

**Tech Stack:** C++/WinRT, WinUI 3, XAML ResourceDictionary, `.resw` localization, MSBuild.

## Global Constraints

- Windows 11 only; x64 first.
- Preserve Windows system brushes and user accent for ordinary interactions.
- Reserve Aurora Violet `#725CFF` and Signal Cyan `#2ED3FF` for brand moments.
- Do not add WebView, Electron, Tauri, custom fonts, or redistributed font files.
- Do not bypass the manager provider boundary or read SQLite/blob storage directly.
- Preserve unrelated working-tree changes, especially `eng/*.ps1` files.
- Do not fabricate backend availability or production clipboard history.

---

### Task 1: Shared brand and surface tokens

**Files:**
- Modify: `apps/manager/Pastral.Manager/Themes/PastralTheme.xaml`

**Interfaces:**
- Produces: `PastralBrandMarkStyle`, `PastralHeroPanelStyle`, `PastralSectionHeaderStyle`, shared spacing and compact surface resources.

- [ ] Add reusable brand mark geometry and restrained hero/surface styles using system brushes.
- [ ] Validate XAML resource references are unique and resolve from `App.xaml`.

### Task 2: Localized branded shell

**Files:**
- Modify: `apps/manager/Pastral.Manager/MainWindow.xaml`
- Modify: `apps/manager/Pastral.Manager/Strings/en-US/Resources.resw`
- Create: `apps/manager/Pastral.Manager/Strings/vi-VN/Resources.resw`

**Interfaces:**
- Consumes: shared theme resources from Task 1.
- Produces: localized title bar, navigation labels, accessibility names, and global status copy.

- [ ] Replace the text-only title bar identity with a compact geometric Pastral mark and localized product positioning.
- [ ] Move touched shell strings to matching English and Vietnamese resources.
- [ ] Verify both resource files expose identical key sets.

### Task 3: Home command-center hierarchy

**Files:**
- Modify: `apps/manager/Pastral.Manager/Pages/HomePage.xaml`
- Modify: both manager resource files.

**Interfaces:**
- Consumes: existing named controls used by `HomePage.xaml.cpp`; names and event handlers remain unchanged.
- Produces: localized static Home hierarchy with clearer task, status, privacy, and recent-content regions.

- [ ] Preserve all runtime-bound control names and state transitions.
- [ ] Replace implementation-first headlines with user-task copy.
- [ ] Apply shared hero and section styles without adding fake actions.
- [ ] Add matching English and Vietnamese resources.

### Task 4: History search and detail polish

**Files:**
- Modify: `apps/manager/Pastral.Manager/Pages/HistoryPage.xaml`
- Modify: both manager resource files.

**Interfaces:**
- Consumes: existing named controls and handlers used by `HistoryPage.xaml.cpp`.
- Produces: localized search-first History surface with refined results, empty state, and detail hierarchy.

- [ ] Preserve all state names, named controls, bindings, and handlers.
- [ ] Improve search prominence, row scanability, empty-state language, and details hierarchy.
- [ ] Add matching English and Vietnamese resources.

### Task 5: Verification and scope review

**Files:**
- Review: complete working-tree diff.

**Interfaces:**
- Consumes: all prior tasks.
- Produces: evidence-backed build and localization validation results.

- [ ] Compare `.resw` key sets and reject missing/duplicate keys.
- [ ] Run the focused manager build using the repository solution/project configuration.
- [ ] Run `git diff --check`.
- [ ] Inspect `git status --short` and confirm unrelated `eng/*.ps1` changes were not modified by this work.
