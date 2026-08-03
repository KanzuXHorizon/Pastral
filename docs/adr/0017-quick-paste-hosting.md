# ADR 0017: Quick Paste is hosted by the on-demand manager process

**Status:** Accepted
**Date:** 2026-08-04

## Context

Quick Paste is a native focused UI requiring search, virtualization, accessibility, localization, representation selection, profile state, and destination-safe paste behavior. The Phase 0 architecture defined four executables but did not assign Quick Paste to one. Hosting it inside the resident Rust agent would load UI framework/business presentation into the always-running process. Adding a fifth resident UI process would violate the one-resident-process principle. A fresh WinUI process launch also cannot be assumed to meet a warm-open target.

## Decision

Host Quick Paste as a distinct activation mode/window inside the single-instance, on-demand `pastral-manager.exe` WinUI 3 process.

- The agent owns the global hotkey and sends an activation request containing only destination/session/profile context needed for the operation.
- If the manager is absent, Windows/package/process activation starts it and the manager opens directly into Quick Paste rather than constructing the full manager shell first.
- If the manager is already alive, it reuses the process and opens/activates the Quick Paste window on the intended monitor.
- `Warm Quick Paste` means the manager process and required IPC/search client infrastructure are already initialized.
- `Cold Quick Paste` includes process/package activation through the first interactive frame and is measured separately.
- The manager may remain alive after its last window closes only under an explicit bounded warm-lifetime policy selected through energy/memory/latency evidence. It does not become an indefinite second resident process by default.
- Quick Paste and the full manager share UI components and IPC contracts but maintain separate navigation/focus/lifetime state.
- The manager never opens SQLite/blob storage directly; all data remains agent-mediated.
- Sensitive/private plaintext is requested only for the selected authorized operation and cleared on close, lock, disconnect, or authorization expiry.
- If cold-start latency is unacceptable after measured optimization, a separate lightweight native Quick Paste executable may be proposed through a new ADR. It is not introduced as a background resident process without explicit evidence and policy.

## Consequences

Positive:

- preserves one always-running process;
- reuses WinUI 3 accessibility, localization, virtualization, and design-system work;
- avoids duplicating search/paste UI logic across C++ and Rust;
- defines honest cold versus warm performance measurements;
- keeps database/business ownership centralized in the agent.

Costs:

- cold WinUI activation may miss the aspirational warm latency target;
- manager single-instance activation and multi-window state need robust tests;
- a bounded warm-lifetime policy consumes memory after recent use;
- manager crashes affect both Quick Paste and full manager UI, though not capture/storage.

## Alternatives considered

- **Render Quick Paste in the Rust agent:** rejected because it expands resident memory, UI/accessibility complexity, and crash/blast radius.
- **Separate fifth Quick Paste executable immediately:** plausible for latency, but adds another UI host, packaging/IPC/test surface, and duplicated components before measurements justify it.
- **Always-resident manager:** rejected because it violates the quiet/minimal resident model.
- **Overlay expanded into full picker:** rejected because passive no-focus overlay and explicitly focused searchable UI have different activation/accessibility contracts.

## Review triggers

- measured cold activation cannot meet an accepted user-experience budget;
- bounded warm lifetime causes unacceptable memory/energy use;
- WinUI multi-window/single-instance activation is unreliable under packaging;
- accessibility or focus restoration differs materially between Quick Paste and manager shell;
- a lightweight native host prototype demonstrates a clear net benefit;
- another resident process is proposed.
