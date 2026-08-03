# UX design system

`DESIGN.md` is the durable visual authority. This document translates it into native Windows product requirements and component behavior.

## 1. Foundations

### Platform

- Native Windows 11 interaction and accessibility conventions are the default.
- Use WinUI 3 controls in manager/Quick Paste where they meet behavior and performance needs.
- Passive overlay is custom native rendering but must expose equivalent semantics when interactive.
- Custom controls require a written reason, complete keyboard/touch/UI Automation behavior, high-contrast support, and performance evidence.

### Density

Pastral supports three user-configurable density modes:

- **Comfortable:** larger row spacing and previews; touch-friendly.
- **Standard:** default desktop balance.
- **Compact:** more rows without reducing actionable hit regions below accessible targets.

Density changes visual spacing, not font legibility, focus visibility, or action discoverability.

### Layout adaptation

- Use fluid columns and pane modes rather than fixed window sizes.
- Preserve search and primary action during narrow reflow.
- Detail panes collapse into navigation/drill-in instead of becoming unusably thin.
- Mixed DPI and text scaling are treated independently.
- Remember window geometry only when it remains valid for the current monitor topology/work area.

## 2. Semantic color roles

Use Windows theme/system brushes for:

- canvas and layered surfaces;
- primary/secondary/disabled text;
- focus, selection, hover, pressed, and control states;
- errors, warnings, success, and informational messages;
- high contrast.

Brand roles from `DESIGN.md` are limited to:

- logo and onboarding identity;
- provenance/continuity motif;
- restrained empty-state artwork;
- optional profile accent that remains redundant with text/icon.

A component cannot encode profile, sensitivity, fidelity, or rule status through hue alone.

## 3. Typography roles

Use Windows text styles as the source of truth:

- page title;
- section title;
- body/row primary;
- body secondary;
- caption/metadata;
- code/diagnostic monospace.

Rules:

- no fixed clipping at larger text scale;
- use line wrapping for consequential labels;
- tabular numeric alignment for sizes/timings where useful;
- timestamps use locale-aware formatting and expose exact time in details/tooltips/accessibility text;
- technical identifiers are selectable/copyable in diagnostics, not visually dominant elsewhere.

## 4. Core component contracts

### Clip row/card

Required states:

- normal;
- hover;
- keyboard focus;
- selected;
- multi-selected;
- pinned;
- sensitive/hidden;
- unavailable/reference-only;
- corrupt/quarantined;
- pending enrichment;
- paste success/failure transient feedback.

Content hierarchy:

1. privacy-safe preview or type placeholder;
2. source and relative time;
3. content type/representation availability;
4. only decision-relevant state badges;
5. selected paste action when it differs from default.

### Search box

- standard text editing shortcuts remain unchanged;
- structured filters appear as editable tokens only after valid parsing;
- invalid syntax is explained inline without blocking ordinary text search;
- clear-search and history/suggestion controls have accessible names;
- search starts immediately and cancels obsolete requests;
- sensitive suggestions are omitted according to active privacy policy.

### Filter token

- keyboard removable/editable;
- exposes field/operator/value;
- not color-only;
- invalid/unavailable token remains editable and explains the issue;
- long values truncate visually but full sanitized value is accessible where allowed.

### Representation selector

Displays explicit choices such as Original, Preferred, Plain text, HTML, RTF, Image, File path, Filename, OCR text, or derived output.

- Never label an unsupported mode as available.
- Show fidelity/compatibility notes in a secondary description.
- Remembering a choice requires explicit scope selection.

### Profile selector

- always shows current profile name and capture state;
- switch is reversible and announces changed retention/privacy implications;
- auto-switch state is visible with “why” explanation;
- Private profile uses privacy language, not merely a lock color/icon.

### Rule explanation

Contains:

- what happened;
- matched facts;
- rule name/scope/priority;
- action and result;
- undo;
- edit/disable;
- conflict or fallback notes.

### Empty/error states

Every state includes:

- specific condition;
- impact on capture/search/paste;
- safe next action;
- technical details disclosure only when useful;
- no invented reassurance.

## 5. Commands and destructive actions

- Primary task action appears once per surface.
- Destructive actions use clear verbs: Delete clip, Clear unpinned history, Delete profile data.
- Undo is preferred for reversible local changes.
- Permanent deletion, encryption-key deletion, import replacement, and destructive repair require consequence-specific confirmation.
- Bulk operations show scope/count and exclusions before execution.
- Disabled actions explain why through accessible help text, not only a tooltip.

## 6. Navigation

Manager navigation uses stable semantic destinations rather than feature marketing terms:

- Home;
- History;
- Collections;
- Sources;
- Profiles;
- Rules;
- Paste Queue;
- Storage;
- Privacy;
- Integrations;
- Settings;
- Diagnostics;
- About.

Navigation supports keyboard, access keys where appropriate, back/forward history, deep links only after protocol security design, and restored context without reopening sensitive previews.

## 7. Feedback

- Passive overlay confirms capture without focus.
- In-surface teaching tips are rare, dismissible, and do not recur after completion.
- Status bars/inline banners communicate paused capture, low disk, recovery, locked profile, incompatible agent, or database health.
- Toasts/Windows notifications are not used for every copy.
- Progress indicators appear only for operations long enough to justify them and support cancellation where safe.

## 8. Content voice

- Short, literal, calm.
- Prefer “Copied · Text · VS Code” over celebratory language.
- State automation directly: “Tracking removed · Rule: Clean Edge links.”
- State uncertainty: “Paste sent; destination response unknown.”
- Avoid “magic,” “smart AI,” productivity scores, blame, and fear-based secret warnings.
- Privacy copy distinguishes not stored, encrypted, hidden, expired, and unavailable.

## 9. Verification checklist

Every reusable component is verified for:

- keyboard and focus order;
- UI Automation name/role/state/pattern;
- light/dark/high contrast;
- disabled/empty/error/loading/overflow;
- 100–300% DPI and text scaling;
- RTL and long localization;
- touch/pointer behavior;
- reduced motion/transparency disabled;
- virtualization/recycling state correctness;
- sensitive content leakage in visuals and accessibility tree.
