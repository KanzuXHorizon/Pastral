# Pastral Design System

## Direction contract

**THESIS:** Pastral visualizes clipboard memory as a precise signal trail, refusing the category default of a generic stack of floating glass cards.
**OWN WORLD:** Native Windows surfaces, disciplined information density, clipped ribbon paths, restrained violet/cyan continuity marks, and strong system-state typography.
**STORY:** Copy disappears into a trustworthy indexed trail; source, representation, rule, and privacy remain legible; paste returns intentionally.
**FIRST VIEWPORT:** Quick Paste opens as a compact command surface with search first, recent results immediately visible, source/type/profile encoded redundantly, and the selected representation/paste action explicit.
**FORM:** Operate-mode native utility; task hierarchy and familiar Windows affordances outrank decorative expression.

## 1. Visual authority

Pastral is an incomplete but binding brand system:

- Windows 11-native behavior and platform conventions are authoritative.
- The provisional violet/cyan mark and ribbon-continuity idea are authoritative at brand moments.
- Supplied concept boards establish desired compactness, dark-mode polish, and visible source/profile organization, but their generated text, icons, glass intensity, and component arrangements are not implementation authority.
- System settings, accessibility, power state, and focus safety outrank visual effects.

## 2. Scene and color strategy

Pastral is used for long periods beside editors, documents, browsers, and terminals under varied ambient light. It must support both Windows light and dark modes rather than forcing a permanent dark product shell.

Color strategy: **Restrained**.

- Neutral surfaces and Windows semantic brushes carry the application.
- User accent color carries ordinary selection, focus, and primary interaction where Windows guidance expects it.
- Aurora Violet and Signal Cyan identify Pastral and continuity/provenance; they do not replace semantic system colors.
- The brand gradient may appear in the logo, onboarding illustration, empty-state trace, and rare selected continuity path—not as a background wash for every card.

Brand colors:

| Role | Value | Use |
|---|---|---|
| Aurora Violet | `#725CFF` | Brand mark, provenance accent, limited focus-independent emphasis |
| Signal Cyan | `#2ED3FF` | Brand mark, linked/continuation endpoint, limited information accent |
| Night Ink | `#0B1020` | Brand artwork dark field, not a mandatory app background |
| Frost | `#F7F8FC` | Brand artwork light field, not a hard-coded system surface |
| Success Mint | `#35D49A` | Success only, always paired with text/icon |
| Warning Amber | `#F2B84B` | Warning/attention only, always paired with text/icon |

Never hard-code brand colors where a Windows system brush provides semantic state. Contrast is validated in every theme and contrast mode.

## 3. Material and depth

- Main manager shell may use Mica where supported; use a solid system-color fallback.
- Acrylic is limited to transient, light-dismiss surfaces such as an explicitly interactive flyout.
- Passive overlay uses restrained translucency only when it remains readable and power-safe; solid is the normal fallback, not a degraded error.
- Depth uses one restrained shadow/elevation layer. Avoid nested glowing panels, neon edges, and multiple blurry shadows.
- Dividers and spacing establish most hierarchy; containers are used only when they group an actionable concept.

## 4. Typography

- Use OS-provided Segoe UI Variable.
- Do not redistribute fonts.
- Prefer Windows text styles and user text-scaling behavior over custom fixed pixel sizes.
- Use sentence case for labels and headings.
- Reserve monospaced text for code, hashes, format identifiers, and diagnostics; use an OS-available monospaced fallback and never apply it decoratively.
- Important state follows a three-level hierarchy: task title, primary content/source, secondary metadata.
- Avoid all-caps tracked micro-labels and oversized marketing typography in operative surfaces.

## 5. Geometry and spacing

- Base spatial rhythm follows Windows control metrics and a 4-DIP subgrid.
- Use 8, 12, 16, 24, and 32 DIP as common spacing steps; exact control metrics follow WinUI/Windows guidance and text scaling.
- Compact floating surfaces use 8–12 DIP visual corner radius according to size.
- Main content regions use restrained rounding; do not turn every row into an independent pill/card.
- Minimum pointer/touch target follows Windows accessibility guidance; compact desktop density must not reduce operable hit areas.
- Alignment is anchored to content baselines and columns, not decorative symmetry.

## 6. Signature motif: the signal trail

The durable Pastral motif is a short geometric trail that connects source, preserved representations, transformation, and paste outcome.

Rules:

- Use it for provenance, relationship, progress, and brand—not as arbitrary ornament.
- It may appear as a clipped ribbon path, paired brackets, or two interlocking rounded planes.
- A trail has a clear start and end and never obscures text.
- Motion, when enabled, is a single directional reveal or handoff; no looping pulse, bounce, particle field, or “AI” sparkle.
- In monochrome/high contrast, shape and stroke communicate the same relationship.

## 7. Iconography

- Use Windows/Fluent system icons for common actions when available.
- Create custom icons only for Pastral-specific concepts such as representation set, fidelity, derived representation, and rule provenance.
- Custom icons use simple geometry, consistent optical weight, and 16/20/24 DIP masters.
- Never rely on icon alone for destructive, privacy, fidelity, or automation state.
- Tray glyph is a highly simplified monochrome `P`/loop signal with no text and no gradient.

## 8. Component language

### Search and command surfaces

- Search is the dominant control in Quick Paste and History.
- Query chips are editable structured filters, not decorative pills.
- Primary action is explicit as text when consequence or representation matters.
- Keyboard shortcuts appear only where active and customizable.

### History rows

One row expresses:

- content preview or privacy-safe placeholder;
- source and time;
- type/representation summary;
- profile;
- pin/sensitivity/fidelity state when relevant;
- selected paste action.

Rows remain scannable without every badge visible. Secondary metadata appears on selection, details, or when it changes the decision.

### State badges

- Use shape/icon + concise text.
- Fidelity badges are neutral/informational, not success scores.
- Sensitivity badges reveal policy state, never content.
- Profile accents are redundant with names/icons and remain subtle.

### Rules

Human-readable sentence is primary. Priority, scope, last match, audit, and conflict state form a secondary structured region. Simulation results visually separate matched facts from action consequences.

## 9. Motion

- Entry/exit for transient surfaces: 120–180 ms using opacity and small translation only.
- No bounce or excessive spring.
- Passive overlay does not loop.
- Reduced-motion policy removes translation and may use an immediate opacity/state change.
- Battery saver/RDP/device-loss may remove all animation.
- Motion never delays access to content or indicates the only copy-success signal.

## 10. Focus and interaction

- Passive overlay is not in the keyboard focus order.
- Focus visuals use Windows conventions and user accent/system brushes.
- Quick Paste takes focus only after explicit invocation and restores/cancels safely.
- Ordinary number keys and Escape remain untouched unless an explicit interactive surface owns them.
- Hover is supplementary; every action works with keyboard and touch where supported.
- Destructive operations use policy-based confirmation/undo rather than repeated modal friction.

## 11. Accessibility and adaptation

- High contrast replaces brand/material brushes with system colors while preserving iconography and hierarchy.
- Transparency disabled uses solid surfaces.
- 100–300% DPI and text scaling may move from multi-column to stacked detail layouts.
- RTL mirrors topology and directional icons only where semantics require it.
- Long localized strings wrap or reflow; labels are not truncated where consequence would become ambiguous.
- Screen readers announce task, item position, content type, source policy, selected representation/action, and state—never hidden sensitive content.

## 12. Anti-patterns

Do not ship:

- generic dark dashboard with every region in a glowing glass card;
- violet/cyan gradient on all controls;
- excessive pills, badges, and icon-only actions;
- unreadable tiny secondary text;
- fake productivity scores, streaks, or gamification;
- AI sparkles or unexplained magic language;
- decorative clipboard-board/paperclip metaphors;
- generated placeholder labels presented as final copy;
- custom title bars or controls that weaken native accessibility without a measured reason;
- animation or transparency that survives against reduced-motion, high-contrast, battery, or RDP policy.

## 13. Durable versus surface-specific decisions

This file owns shared color, material, typography, geometry, motif, icon, motion, focus, and accessibility rules. Overlay, Quick Paste, and manager information architecture live in `docs/ux/` and may specialize layout without changing the shared world. Any durable visual-system change updates this file and receives accessibility/performance review.
