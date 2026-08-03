# ADR 0003: DirectComposition, Direct2D, and DirectWrite overlay

**Status:** Accepted with prototype validation
**Date:** 2026-08-03

## Context

The copy overlay must appear quickly, animate smoothly, respect DPI/accessibility/power states, and never activate or steal keyboard focus. Loading the manager UI framework into the resident agent would increase footprint and couple passive UI to Windows App SDK lifetime.

## Decision

Implement the initial overlay inside `pastral-agent.exe` using:

- a top-level Win32 HWND;
- `WS_EX_NOACTIVATE` and tool-window/taskbar exclusion styles;
- `SW_SHOWNOACTIVATE` and `SWP_NOACTIVATE`;
- `WM_MOUSEACTIVATE` returning a no-activate result;
- DirectComposition for compositor-driven opacity/translation;
- Direct2D for 2D content;
- DirectWrite with OS-provided Segoe UI Variable;
- Per-Monitor V2 DPI awareness;
- solid/software fallback for transparency disabled, RDP, battery saver, or device loss.

The passive overlay has no keyboard focus. Explicit interaction transitions to a separately modeled focused panel or Quick Paste surface.

## Consequences

Positive:

- no WinUI runtime in the agent;
- compositor-driven animation without continuous CPU frame loops;
- precise HWND activation behavior and per-monitor placement.

Costs:

- custom rendering and device-loss handling;
- accessibility exposure for an interactive expansion requires deliberate UI Automation implementation;
- DirectComposition documentation recommends evaluating newer visual-layer APIs.

## Alternatives considered

- **Windows.UI.Composition:** viable; prototype before final implementation if Rust/Win32 hosting is simpler and footprint-neutral.
- **WinUI 3 overlay:** easier controls but loads larger UI stack into agent or adds another process.
- **Layered GDI window:** simple fallback but weaker animation/text/rendering path.

## Review triggers

- prototype shows Windows.UI.Composition is more reliable or materially smaller;
- no-focus or Alt+Tab/taskbar tests fail with the chosen HWND model;
- RDP/device-loss behavior is unacceptable;
- first-visible-frame or idle-resource budgets cannot be met.
