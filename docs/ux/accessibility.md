# Accessibility requirements

## 1. Standard

Pastral targets WCAG 2.2 AA principles where applicable to native desktop UI and follows Microsoft Windows accessibility guidance and UI Automation contracts. Passing automated checks alone is insufficient; core flows require manual assistive-technology validation.

## 2. Core commitments

- Every core task is keyboard operable.
- Focus order follows task and visual order.
- Focus is always visible in focused surfaces.
- Passive overlay never steals focus.
- UI Automation exposes accurate name, role, state, value, position, and supported patterns.
- Information never depends on color, shape, animation, sound, hover, or pointer precision alone.
- Text/content scales and reflows at 100–300% DPI and Windows text scaling.
- High contrast/contrast themes and transparency disabled are first-class modes.
- Reduced-motion and battery/RDP fallback remove nonessential motion.
- Sensitive/locked content remains absent from the accessibility tree, not merely visually obscured.

## 3. Keyboard

### Global behavior

- Global hotkeys are configurable and conflict-checked.
- Registration failure is announced and shown in settings.
- No global interception of Escape, number keys, arrows, Tab, or ordinary text.
- IME, dead keys, AltGr, language switching, sticky/filter keys, and On-Screen Keyboard must continue to work.

### Focused surfaces

- Tab/Shift+Tab traverses logical regions.
- Arrow keys follow native list/grid/menu semantics.
- Enter/Space activation follows control role.
- Escape closes only the current owned layer or returns one navigation level.
- Context Menu key/Shift+F10 exposes item commands.
- Search preserves standard select/copy/paste/undo/redo/word-navigation shortcuts.
- Shortcut customization detects reserved/conflicting combinations and provides reset.

## 4. UI Automation

### Clip result

Expose:

- privacy-safe item name or type placeholder;
- source/profile/time according to policy;
- selected/focused/pinned/locked/unavailable states;
- item position and collection size when available;
- invoke/select/toggle patterns appropriate to actual behavior;
- representation/paste action as separate controls when selectable.

Do not concatenate every badge into an unreadable announcement. Use descriptions/help text for secondary detail.

### Search and filters

- Search uses the text/value pattern and clear accessible label.
- Result count announcements are polite/coalesced and do not fire on every internal page update.
- Filter tokens expose field/operator/value and remove/edit actions.
- Syntax errors identify range/token and correction without blocking plain search unnecessarily.

### Rules

- Human-readable rule summary is the accessible name.
- Enabled/priority/scope/conflict/match state is structured, not encoded in color.
- Simulation separates matched predicates from action outcome.

### Overlay

- Passive confirmation is not focusable.
- Optional announcement is concise and rate-limited.
- No content is announced for sensitive skip/locked preview beyond safe policy status.
- When synthetic paste is blocked or uncertain, including a higher-integrity destination, announce that the item was copied to the clipboard and that the user should paste manually; do not announce payload content.
- Explicit interactive panel exposes a normal dialog/flyout structure and focus trap only while open.

## 5. Vision and contrast

- Validate text, icons, focus indicators, selection, borders, and state against theme/system contrast requirements.
- In contrast themes, use system brushes and preserve native forced-color behavior rather than retaining brand backgrounds.
- Selected and focused states remain distinguishable from each other.
- Disabled state remains readable and is supplemented by explanation.
- Small metadata text cannot be the sole location of essential information.
- Zoom/magnification does not cause overlay to cover its intended target unnecessarily.

## 6. Text scaling and localization

- Prefer native layout measurement and text styles over fixed dimensions.
- Consequential labels wrap; controls grow/reflow.
- At large scale, multi-column surfaces become stacked/drill-in.
- Ellipsis is allowed for previews and source names when full safe text is available through details/accessibility; destructive/action labels do not truncate into ambiguity.
- Support long translations, plural rules, locale dates/numbers/byte sizes, Unicode source names, and mixed-direction content.
- RTL mirrors navigation/layout and directional icons where semantic; code, paths, and hashes preserve correct text direction handling.

## 7. Motion, transparency, and sound

- Reduced motion removes translation and nonessential transitions.
- No looping passive animation.
- Transparency disabled uses solid system surfaces with sufficient separation.
- Device loss/RDP/battery saver has a stable nonanimated fallback.
- Sound is off by default and never the sole confirmation.
- Optional sound respects system volume/accessibility settings and has visual/text equivalent.

## 8. Touch, pen, and motor accessibility

- Interactive targets meet accessible size/spacing in touch mode.
- Dense mode may reduce visual padding but preserves effective hit regions.
- Avoid actions requiring drag, hover, double-click, press-and-hold, or precise pointer as the only path.
- Drag-and-drop has menu/keyboard alternatives.
- Time-limited passive overlay duration is configurable; critical actions do not disappear before the user can operate them because those actions require explicit interactive mode.

## 9. Cognitive accessibility

- Use literal labels and stable placement.
- Explain automation with what/why/rule/undo/edit.
- Avoid broad hidden scope, magic language, gamification, and unnecessary alerts.
- Separate original from derived content clearly.
- Show reversible action/undo near outcome.
- Errors state impact and next safe action.
- Settings state defaults and whether changes are prospective or retroactive.

## 10. Privacy and accessibility

- Hidden sensitive text is not present in offscreen accessibility nodes, live regions, tooltips, cached view models, task thumbnails, or copied automation properties.
- Window display-affinity/capture exclusion is optional defense in depth and never substitutes for removing hidden content from UI Automation and all rendered/cached view models.
- Unlock/reveal is explicit and state changes are announced without reading the secret automatically.
- Source title/domain/path obeys the same redaction policy in automation properties as on screen.
- Diagnostic technical text is selectable but excludes content.
- Display-affinity/screen-capture exclusion is never the primary privacy control and is not announced as guaranteed protection; accessibility-safe hidden-state behavior remains correct when the API is unavailable or ignored.

## 11. Test matrix

Automated:

- Accessibility Insights/UI Automation inspection;
- keyboard traversal/focus assertions;
- name/role/state/pattern checks;
- contrast and theme smoke checks;
- text scale/DPI layout snapshots using synthetic data;
- virtualization recycling privacy/state tests;
- no sensitive canaries in automation tree/live-region events.

Manual release candidate:

- Narrator: onboarding, Quick Paste search/paste, clip details, rules, privacy, storage, diagnostics;
- keyboard-only full core workflows;
- Magnifier and 300% DPI/text scaling;
- On-Screen Keyboard and IME;
- contrast themes/light/dark;
- reduced motion/transparency disabled;
- touch where hardware exists;
- RDP and screen-capture/display-affinity fallback behavior;
- RTL and long localization;
- copy overlay while selecting/typing/menu open.

## 12. Accessibility defect policy

Block release for:

- inaccessible core action;
- focus theft/loss or keyboard trap;
- hidden sensitive content exposed to UI Automation;
- critical text unreadable in contrast themes;
- core layout unusable at supported DPI/text scale;
- motion that ignores reduced-motion policy and causes functional disruption;
- selection/focus/state impossible to distinguish.

Other defects require severity, affected flow, workaround, owner, and deadline; they are not silently waived.
