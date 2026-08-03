# Copy confirmation and fast-action overlay

## 1. Purpose

The overlay confirms capture and, only when useful, points to fast actions without changing foreground focus, keyboard focus, text selection, menus, or typing.

The passive overlay is not a miniature manager and is not a notification stream.

## 2. Modes

### Smart — default

- brief confirmation for successful capture or a meaningful policy result;
- trusted remembered rules show result, not chooser;
- actions are offered only for unknown/low-confidence patterns;
- suppress in configured fullscreen, games, presentations, screen sharing, remote sessions, denied apps/profiles, or sensitive fields/contexts;
- coalesce rapid repeated copies.

### Confirmation only

A compact type/source/status pill with no actions.

### Actions on modifier

Passive confirmation behaves normally; explicit action interaction opens only when the configured modifier condition is present.

### Manual

No automatic overlay. User invokes the action panel through a hotkey/tray command.

### Off

No overlay; capture continues according to profile policy.

## 3. Passive state

Example content:

- `Copied · Text · VS Code`
- `Copied · Image · Snipping Tool`
- `Saved to Development`
- `Tracking removed · Original preserved`
- `Sensitive content not saved`
- `Capture paused`
- `3 items added to Paste Queue`

Rules:

- one primary line; optional secondary explanation only when it changes user understanding;
- no raw preview for potentially sensitive content;
- source omitted according to privacy setting;
- no more than one decision-relevant badge/icon;
- no buttons that imply ordinary keyboard focus in passive state;
- no progress animation that loops after capture.

## 4. Explicit action state

The user enters an interactive action panel through an explicit hotkey/modifier/tray/pointer transition defined by prototype evidence. The passive HWND itself does not become activated accidentally.

Show at most four primary actions.

URL example:

1. Keep original
2. Remove tracking
3. Copy title + URL
4. Save to collection

Code example:

1. Keep rich
2. Plain text
3. Code block
4. Pin

Image example:

1. Keep original
2. Save as PNG
3. OCR text
4. Compress copy

Only implemented/available actions appear. OCR is absent until its module exists. Action consequences and derived-original relationship are explicit.

## 5. Focus and input

Passive overlay assertions:

- foreground window unchanged;
- active window unchanged;
- focused HWND unchanged;
- no taskbar button;
- no Alt+Tab entry;
- `WM_MOUSEACTIVATE` does not activate;
- ordinary number keys, Escape, arrows, Tab, and typing remain with source application;
- no global keyboard hook is installed solely for overlay actions.

Interactive panel assertions:

- entered only through explicit user intent;
- visible focus indicator;
- number keys 1–4 active only when labels are visible and panel owns focus;
- Escape closes only the owned panel;
- closing returns focus to the validated prior destination where safe;
- screen reader announces panel title, actions, selection, and scope follow-up without sensitive content.

## 6. Placement

User options:

- near pointer;
- near caret only when reliable and privacy-safe;
- active monitor bottom center;
- active monitor bottom right;
- custom anchored position.

Placement algorithm:

1. identify intended active monitor from foreground window, caret/pointer policy, and current monitor topology;
2. obtain work area and Per-Monitor V2 scale;
3. compute passive surface size after localized/text-scaled layout;
4. avoid pointer hotspot and reliably known selected/caret region;
5. avoid taskbar and screen edges;
6. avoid Windows notification region where practical;
7. suppress/relocate rather than cover password fields or sensitive surfaces when detectable;
8. clamp to work area;
9. remember custom anchor by stable monitor configuration with safe fallback after topology changes.

Do not chase the pointer continuously. Placement is computed once per presentation unless DPI/topology changes.

## 7. Layout and visual treatment

### Confirmation pill

- compact horizontal form;
- icon/type marker, short text, optional source/rule phrase;
- 8–12 DIP visual corner radius;
- restrained single shadow;
- sufficient solid fallback contrast;
- width bounded; long source names elide after preserving outcome.

### Action card

- confirmation header plus up to four actions;
- actions use text labels by default; icon-only option is not the default;
- scope follow-up is a separate minimal step, not another dense row;
- no nested cards/glowing borders;
- Pastral gradient appears only in logo/short provenance trail, not entire background.

## 8. Timing and motion

Defaults subject to usability tests:

- passive confirmation duration: approximately 1.5–2.5 seconds depending on content and accessibility setting;
- rule/sensitive/paused messages may remain slightly longer;
- enter/exit: 120–180 ms opacity + small translation;
- copy bursts update/coalesce without restarting distracting animation indefinitely;
- reduced motion: no translation; immediate or short opacity transition;
- battery saver/RDP/device loss/transparency disabled: solid, simple, potentially unanimated fallback.

A user can configure duration or disable animation/sound. No sound by default.

## 9. Coalescing

Rapid events are grouped when they share compatible outcome and profile:

- repeated same payload/source may update count;
- mixed types use a generic count summary rather than flashing every card;
- policy errors/sensitive skips are not combined with ordinary success in a misleading way;
- final captured event remains available in history even when overlay groups confirmations;
- coalescing never changes capture event semantics.

## 10. Suppression

Default overlay suppression is broader than capture suppression.

Suppress or use a minimal safe state for:

- exclusive/fullscreen games;
- presentations and video fullscreen;
- screen sharing/recording when configured or reliably detected;
- Windows lock/UAC secure desktop;
- RDP according to setting;
- denied applications/profiles;
- password or sensitive contexts;
- rapid system transitions and unsupported display state;
- user Do Not Disturb preference if integrated through a later design.

Capture may continue unless separate policy denies it.

## 11. Accessibility

- Passive confirmation is not keyboard focusable.
- Announcement is configurable, concise, coalesced, and never reads sensitive content.
- Interactive state exposes UI Automation roles/patterns and predictable tab/order.
- High contrast uses system brushes and no transparency-dependent separation.
- Text scaling can convert a pill to a taller compact card.
- Touch targets meet accessible size in interactive state.
- Meaning does not depend on violet/cyan or animation.

## 12. Settings

- enabled/mode;
- duration;
- placement/monitor behavior;
- maximum actions;
- labels versus icons;
- animation and sound;
- source/preview/rule explanation display;
- excluded apps/profiles;
- fullscreen/screen-share/RDP suppression;
- notification coalescing;
- sensitive preview policy;
- announcement behavior;
- reset to calm defaults.

## 13. Acceptance tests

- foreground/active/focused HWND unchanged across copy sources;
- no Alt+Tab/taskbar presence;
- passive click no activation;
- typing, IME, menus, drag selection, number keys, and Escape unaffected;
- placement correct at 100–300% DPI and mixed monitors;
- taskbar/work-area and topology changes;
- fullscreen/screen-share/RDP/transparency/reduced-motion/battery fallback;
- copy burst coalescing;
- content-free sensitive state;
- device loss and agent shutdown cleanup;
- first warm frame budget and zero post-dismiss CPU animation loop.
