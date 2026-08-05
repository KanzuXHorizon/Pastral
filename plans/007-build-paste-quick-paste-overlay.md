# Plan 007: Build safe paste/replay, Quick Paste, overlay, tray, and hotkeys

> **Executor instructions:** This is a high-risk interaction program. Execute replay before UI convenience. Use fixture destinations and isolated test data. Never use unsupported focus hacks, elevation, `uiAccess`, or a global keyboard hook solely for overlay actions.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- crates/domain crates/clipboard-win crates/ipc-core crates/ipc-schema crates/ipc-win apps/agent apps/manager protocols eng docs/architecture docs/ux docs/adr/0003-overlay-rendering.md docs/adr/0004-clipboard-data-object-model.md docs/adr/0017-quick-paste-hosting.md`

## Status

- **Priority:** P1
- **Effort:** XL
- **Risk:** HIGH
- **Depends on:** Plan 006
- **Category:** replay, security, UI/UX, performance, accessibility
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

The product promise is not merely storing clipboard history; it is finding a prior clip and returning the safest useful representation to the intended destination. No paste/replay, Quick Paste, overlay, tray or hotkey implementation exists today. These capabilities cross clipboard ownership, focus, input, privacy, IPC, package activation and accessibility boundaries, so they must be delivered as measured vertical slices rather than a decorative picker first.

## Architecture

- The resident agent owns global hotkeys, tray, destination snapshot and replay authorization.
- Clipboard-platform STA constructs and publishes a Pastral-owned `IDataObject` with bounded delayed-render lifetime.
- Manager hosts Quick Paste as a dedicated top-level activation/window mode in the single-instance on-demand process; full manager shell loads lazily.
- Quick Paste searches via authenticated provider contracts and requests paste/copy through explicit versioned operations.
- Destination identity is captured before activation and revalidated immediately before input dispatch.
- When focus/input dispatch is blocked or uncertain, leave data on the clipboard and present a truthful manual-paste instruction.
- Passive overlay is a separate non-activating top-level surface controlled by the resident and never enters ordinary keyboard focus.

## Scope

**In scope**

- replay object/adapters for formats completed by Plan 006
- self-update suppression and clipboard publication lifetime
- destination capture/revalidation and safe input dispatch/fallback
- Quick Paste activation, search/results/details/action flow
- passive confirmation and explicit interactive overlay state
- tray, global hotkey registration/conflict handling and settings contracts
- accessibility, privacy, performance and fixture evidence

**Out of scope**

- Automatic background paste without explicit user intent.
- `uiAccess`, elevation, service-based input, focus stealing or unsupported hooks.
- OCR, semantic search, cloud integrations.
- Rules/profiles beyond consuming the active profile/default action; full rule engine is Plan 008.
- Private content until Plan 008 authorization/decryption boundary exists.

## Program slices

### Slice 1: Define replay authorization and result contracts

Add domain/IPC operations with explicit user intent, selected event/representation, action (`Paste`, `CopyOnly`, `PastePlainText`), destination snapshot ID, deadline and expected policy/profile generation. Responses distinguish published, pasted, clipboard-only fallback, cancelled, destination changed, unavailable, denied, incompatible and internal error.

Connection authentication alone must not authorize replay. Correlation and operation IDs are bounded; no raw content appears in control messages.

**Verify:** unauthorized, stale, duplicate/replayed and expired requests fail closed.

### Slice 2: Build Pastral replay data object on the clipboard-platform STA

Implement replay adapters from stored exact representations to Windows clipboard formats. Re-register exact registered-format names per process; never reuse persisted runtime IDs. Offer only validated safe representations and an explicit priority policy. Keep delayed-render object/media alive until ownership changes or a measured timeout; release every medium/COM reference on the STA.

Add self-origin marker/sequence handling so Pastral publication does not create duplicate history while genuine subsequent user copies still capture.

**Verify:** fixture consumers for Notepad/Win32/WPF/WinForms/custom request supported formats; exact bytes/fidelity and lifetime pass.

### Slice 3: Add destination snapshot and focus-safe dispatch

Capture current foreground HWND, PID, process creation identity, session, integrity and monitor before Quick Paste activation. Before dispatch:

- validate process/window still exists and matches identity;
- ensure destination is not Pastral itself unless explicitly copy-only;
- publish replay object first;
- restore/activate only through supported Windows behavior;
- send paste input only when destination/focus state is confirmed;
- cancel or return clipboard-only/manual instruction on uncertainty, elevated/UIPI target or blocked focus.

Do not claim success from `SendInput` return alone; use explicit result semantics and no destructive clipboard restoration by default.

**Verify:** foreground change, PID reuse fixture, elevated target, closed target, focus denial and normal destination cases.

### Slice 4: Implement Quick Paste lifecycle and activation

Add manager single-instance activation routing for Quick Paste and normal manager. Resident hotkey sends a versioned activation request plus content-free destination token. Quick Paste opens on the intended monitor/work area, focuses search after explicit invocation, and restores/cancels safely. Measure cold and warm separately; keep warm retention bounded/configurable.

Load only localization/theme/accessibility/provider essentials before first frame. Full NavigationView/Home services must not block Quick Paste first frame.

**Verify:** one manager process, repeated activation, correct monitor/mixed DPI, cold/warm timing and clean teardown.

### Slice 5: Build Quick Paste search/result/action UI

Follow `docs/ux/quick-paste.md`:

- decision-relevant profile/capture/destination context;
- dominant search field with typed query status;
- virtualized results and optional detail/representation pane;
- visible selected action/representation;
- Enter paste preferred, Shift+Enter plain, Ctrl+Enter copy-only, Alt+Enter details, context menu support;
- stable selection during incremental/cancelled search;
- one-pane narrow/text-scaled behavior;
- loading, empty, no-results, disconnected, locked, unavailable and mismatch states.

Do not show actions not implemented. Do not fabricate compatibility or fidelity claims.

**Verify:** keyboard-only and UIA end-to-end search→select→copy/paste/fallback with seeded fixtures.

### Slice 6: Implement tray and hotkey ownership

Resident creates one notification-area icon with native context menu for Quick Paste, Open Manager, Pause/Resume Capture, status and Exit/Restart as policy permits. Register configurable global hotkey, detect conflicts and offer safe alternatives. Handle Explorer restart, session transitions, DPI/theme and clean shutdown. Do not use a low-level keyboard hook for ordinary hotkey behavior.

**Verify:** hotkey conflict, tray recreation, keyboard menu, Narrator names, pause state and package startup.

### Slice 7: Build passive overlay as non-activating confirmation

Implement Smart/Confirmation-only/Actions-on-modifier/Manual/Off modes from `docs/ux/overlay.md`. Passive state:

- no foreground/active/focused HWND change;
- no taskbar/Alt+Tab entry;
- `WM_MOUSEACTIVATE` does not activate;
- one primary line, privacy-safe source/type/outcome;
- no sensitive payload in view model/UIA/cache/thumbnail;
- source hard-deny suppressed by default;
- bounded coalescing of durable events only;
- solid fallback for transparency-disabled/high contrast/RDP/power state.

Use restrained Windows materials and 120–180 ms opacity/small translation only when motion is permitted.

**Verify:** automated HWND/focus/input invariants across typing, IME, selection, menus and copy bursts.

### Slice 8: Add explicit interactive overlay actions

Only explicit modifier/hotkey/tray/pointer transition creates an activating action panel. Show at most four implemented actions with text labels and visible focus. Number keys are active only while the panel owns focus and labels are visible. Escape closes only this panel and returns to the validated destination when safe.

Reuse replay/action authorization; do not duplicate transformation logic in UI.

**Verify:** keyboard/touch/UIA actions, focus return, destination change, unavailable representation and sensitive suppression.

### Slice 9: Add settings/provider contracts

Expose hotkey, overlay mode/duration/placement/motion/source display/suppression and default paste representation through versioned agent-owned settings. Apply atomically, validate conflicts, provide reset, and state prospective consequences. Manager settings page is completed in Plan 009; this slice supplies tested contracts and minimal tray/Quick Paste control.

### Slice 10: Complete end-to-end compatibility and performance evidence

Run capture→restart→search→replay against required application fixtures for text/HTML/RTF/URL/files/images. Measure:

- replay construction/publication/consumption;
- Quick Paste cold/warm first frame and first result;
- overlay first frame and post-dismiss idle CPU;
- memory/cache bounds and cleanup;
- destination failure/fallback latency.

Record exact hardware, OS/app versions and fidelity outcome. Compatibility profiles are evidence-based and versioned.

## Test plan

- replay exact/preferred/plain/copy-only across supported formats;
- delayed rendering and owner lifetime;
- self-origin suppression without suppressing real copies;
- intended destination revalidation, foreground changes, elevated/UIPI and focus failure;
- single-instance Quick Paste cold/warm activation, monitor/DPI/work area;
- search cancellation/selection stability/incremental pages;
- passive overlay non-activation/input invariants and suppression matrix;
- tray/hotkey conflict/Explorer restart/session transitions;
- keyboard, Narrator, high contrast, reduced motion, transparency off, 300% scaling, RDP;
- no sensitive/private leakage through visuals, UIA, thumbnails, caches or logs.

## Done criteria

- [ ] Supported stored representations replay with truthful fidelity and bounded lifetime.
- [ ] Wrong-target paste is prevented; uncertainty produces clipboard-only/manual fallback without false success.
- [ ] Quick Paste opens only explicitly, uses one manager process and meets accepted cold/warm budgets.
- [ ] Search/results/actions are keyboard/UIA complete and adaptive.
- [ ] Passive overlay never changes foreground/active/focused HWND or intercepts ordinary input.
- [ ] Tray/hotkeys survive conflicts/session/Explorer lifecycle.
- [ ] Sensitive content is absent from unauthorized presentation/cache/accessibility paths.
- [ ] Fixture compatibility and performance reports exist; full gates pass.

## STOP conditions

Stop and review if:

- A destination requires elevation, `uiAccess`, service input or unsupported focus manipulation.
- Replay ownership cannot be kept on the clipboard-platform STA.
- A format would execute/render active untrusted content.
- Passive overlay changes focus/input in any tested source application.
- Quick Paste startup target can only be met by keeping an unbounded full manager resident.
- An action exposes Private/sensitive content before Plan 008 authorization.
- Clipboard restoration is proposed as default without consumer/lifetime evidence.

## Maintenance notes

Every new representation needs both capture and replay fixture evidence. Quick Paste and overlay are clients of shared search/action/replay contracts; never fork business rules into UI code. Re-run focus, accessibility and privacy matrices after any window-style or activation change.
