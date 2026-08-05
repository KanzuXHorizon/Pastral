# Pastral V1 Implementation Roadmap

Generated from a deep repository audit on 2026-08-05 and now being executed directly on the operator-approved `main` checkout.
Execute plans in dependency order. Each executor must read the whole assigned plan, preserve concurrent work, run every verification gate, and stop instead of improvising when a STOP condition is met.

This roadmap treats “complete” as a production-ready Windows 11 x64 V1: reliable capture, searchable durable history, safe replay, Quick Paste and overlay interaction, privacy/encryption boundaries, full manager workflows, accessibility/localization, signed packaging, upgrade/uninstall, measured performance, and release evidence. Visual polish alone is not completion, and backend-only success is not completion when the user-facing task remains unavailable.

## Audit baseline

- Execution baseline: `1d611ec` plus current reviewed working-tree changes.
- Commits `766e298`–`49be43c` completed the resident default, footprint/dependency-policy reconciliation, and verified development MSIX foundation.
- Commit `a6100b6` restored visible/bindable History rows and UI Automation row evidence.
- Commit `1d611ec` rebuilt History responsive behavior.
- `eng/verify-agent.ps1 -Mode All` now validates no-argument resident startup with a disposable LocalAppData root and bounded cleanup; it passes without leaving a process.
- Native verifier now requires isolated fresh outputs; final full native evidence is waiting for the concurrent localization refactor to finish compiling.
- Exact Windows SDK package-tool selection and package CI staging are in progress under Plan 005.
- IPC prototype, authenticated transport, manager IPC bridge, source policy, Clippy, documentation, cargo advisory scan, agent admission and dependency policy passed in this audit/reconciliation.

## Execution order and status

| Plan | Title | Priority | Effort | Depends on | Status |
|---|---|---:|---:|---|---|
| 001 | Restore one trustworthy verification baseline | P0 | M | — | IN PROGRESS |
| 002 | Restore visible and accessible manager clip rows | P0 | S | 001 | DONE (`a6100b6`) |
| 003 | Rebuild History as a responsive virtualized workspace | P1 | L | 002 | DONE (`1d611ec`) |
| 004 | Complete manager localization and state-copy infrastructure | P1 | M | 002 | IN PROGRESS |
| 005 | Finish resident lifecycle and reproducible MSIX delivery | P1 | L | 001 | IN PROGRESS |
| 006 | Complete capture, storage, search, retention, and recovery MVP | P1 | XL | 001, 005 | READY |
| 007 | Build safe paste/replay, Quick Paste, overlay, tray, and hotkeys | P1 | XL | 006 | READY |
| 008 | Build encryption, Private profile, profiles, and deterministic rules | P1 | XL | 006, 007 | READY |
| 009 | Complete the manager V1 information architecture and workflows | P1 | XL | 003, 004, 006, 008 | READY |
| 010 | Close compatibility, security, accessibility, performance, and release gates | P0 release gate | XL | 005–009 | READY |

Status values: `READY`, `IN PROGRESS`, `DONE`, `BLOCKED: <reason>`, `REJECTED: <reason>`.

## Milestones

### Milestone A — Trust the build

Plans 001–002. Complete the remaining verifier/CLI and fresh native-smoke contracts, then verify the concurrent binding repair so Home/History rows are visible, keyboard-usable, and represented correctly to UI Automation.

### Milestone B — Professional manager foundation

Plans 003–004. History owns the viewport without nested scrolling, narrow layouts are usable, state transitions are clear, and every English/Vietnamese user-visible string is resource-backed.

### Milestone C — Installable reliable clipboard memory

Plans 005–006. One production resident lifecycle is packaged and supervised; common Windows clipboard representations are captured durably; search, retention, quota, low-disk, integrity, backup, and recovery operate end-to-end.

### Milestone D — Retrieval and replay

Plan 007. The user can invoke Quick Paste, find a clip, choose a safe representation, and paste or fall back to copy-only without wrong-target behavior. The passive overlay remains non-activating and privacy-safe.

### Milestone E — Privacy automation and full management

Plans 008–009. Encryption and lock/recovery gates make Private usable; profiles and deterministic rules are explainable and reversible; all V1 manager destinations expose real workflows rather than placeholders.

### Milestone F — Release evidence

Plan 010. Supported Windows versions, accessibility modes, 100k history, IPC compatibility, fuzzing, package install/upgrade/uninstall, signing/update, documentation, and artifact provenance all pass against the exact release candidate.

## Dependency notes

- Plan 001 precedes all risky implementation because the current aggregate gate can fail for unrelated stale assumptions and standalone native smoke can test the wrong executable.
- Plan 002 precedes manager redesign because the current classic bindings do not expose row content; layout work on invisible rows would be unverifiable.
- Plan 005 precedes long-lived production features because capture, IPC, hotkeys, and package activation require one authoritative resident lifecycle and data root.
- Plan 006 precedes paste/Quick Paste because replay and retrieval must consume a stable representation/search contract.
- Plan 008 follows replay because rules and profiles must express real capture/replay consequences, not speculative actions.
- Plan 009 follows the domain work so manager pages are backed by real provider contracts rather than dead navigation.
- Plan 010 is a release gate, not a cleanup bucket; every preceding plan must add focused evidence, and Plan 010 integrates it on one release artifact.

## Cross-cutting constraints

- Windows 11, x64 first; do not introduce Windows 10 support or a primary web runtime.
- Rust `1.97.1`, Edition 2024, Windows App SDK `2.3.1`, C++20/C++/WinRT, WinUI 3.
- Core remains local-first and network-silent; no mandatory AI, OCR, account, or cloud.
- Manager never opens SQLite, FTS, clipboard APIs, or blob storage directly.
- Passive overlay never steals focus or intercepts ordinary typing.
- Highly confident secrets are not stored by default.
- Private profile remains unavailable until encryption, non-indexing, lock, key clearing, and recovery pass.
- Use system semantic brushes, Segoe UI Variable, native controls, and the restrained Pastral signal-trail motif from `DESIGN.md`.
- Every user-visible capability needs loading, empty, error, denied, disabled, success, overflow, narrow, high-contrast, text-scaling, keyboard, and screen-reader behavior.
- Never weaken a security/privacy gate merely to make a UI or test pass.

## Findings considered and rejected

- Replacing WinUI 3 with Electron/Tauri: rejected; it conflicts with the product contract and would discard working native accessibility, package, and IPC foundations.
- Rewriting IPC in JSON/HTTP: rejected; bounded framing, authenticated named pipes, schema conversion, and extensive tests already provide a stronger local boundary.
- Adding AI/OCR/semantic search to V1: rejected; product scope explicitly defers them and current value depends on reliable capture/replay first.
- Shipping all future manager destinations as disabled placeholders: rejected; unavailable navigation should remain hidden until a real provider-backed workflow exists.
- Performing a visual rebrand before fixing row binding, responsive topology, and core workflows: rejected; it would polish an unusable surface and increase rework.

## Audit report

The evidence, scores, prioritized findings, verification transcript summary, and release-gap matrix are in [`000-deep-audit.md`](000-deep-audit.md).
