# Quick Paste

## 1. Job

Quick Paste is the fastest keyboard-first path from “I copied this before” to a deliberate paste or copy action. It is not a reduced manager; it prioritizes search, selection, representation, and destination safety.

## 1.1 Host and lifecycle

Quick Paste is a dedicated top-level window/activation mode inside the single-instance, on-demand `pastral-manager.exe` WinUI 3 process. The global hotkey remains owned by the resident agent, which sends a versioned content-free activation request and destination snapshot to the manager.

- Warm means the manager process and minimal Quick Paste services are already initialized.
- Cold includes process, Windows App SDK/XAML, localization/theme/accessibility, IPC, and first-window activation.
- Cold and warm are measured separately.
- The full manager shell, navigation, previews, and unrelated services load lazily.
- Closing Quick Paste does not justify keeping the manager resident indefinitely; any warm-retention window is bounded, configurable, and measured.
- A separate picker executable requires a new ADR and evidence that it improves cold start without unacceptable accessibility, packaging, security, or duplication cost.

## 2. Invocation and destination

- Open through a configurable global hotkey, tray command, or explicit first-party command.
- Record the current foreground window/process/control confidence before activation.
- Open on the intended foreground monitor and within its work area.
- Focus the search field by default unless the user configured recent-item navigation first.
- Before paste dispatch, revalidate the intended destination, process lifetime identity, session, and available integrity evidence. Cancel rather than paste into an unrelated or uncertain foreground window.
- Close/cancel returns focus only when the prior destination remains valid and Windows permits restoration.
- If focus restoration or synthetic input is blocked/uncertain, including a higher-integrity destination, leave the selected data on the clipboard and show a concise manual-paste instruction.
- Do not request `uiAccess`, elevation, a service, or unsupported focus hacks.

## 3. Default structure

```text
┌ Current profile · Capture state · Representation/paste mode ┐
│ Search history…                                              │
├ Results ───────────────────────────────┬ Optional preview ───┤
│ Selected item                           │ Safe preview/details │
│ Recent/search results                   │ Representations       │
│                                         │ Primary actions        │
├ Query/filter status · result count · shortcut help ──────────┤
```

Responsive modes:

- **Wide:** list plus optional detail/preview pane.
- **Standard:** results dominate; compact details in lower/side region.
- **Narrow/text-scaled:** one pane with drill-in details; search and selected paste action remain visible.
- **Compact list/grid:** user choice; all use virtualized data and equivalent keyboard semantics.

## 4. Header

Show only decision-relevant context:

- current profile;
- capture paused/locked state;
- default paste mode/representation when not obvious;
- optional destination application if it affects compatibility.

Profile switching from Quick Paste is explicit and shows privacy/retention implications. Auto-switching displays an explanation and can be undone.

## 5. Search

Supported syntax includes:

- text terms and exact phrases;
- `type:`;
- `app:`/`source:`;
- `domain:`;
- `profile:`;
- `project:`;
- `tag:`/collection filters;
- `before:`/`after:`;
- `has:` representation;
- `fidelity:`;
- `pinned:`;
- `ext:` for file references;
- sensitivity visibility according to policy.

Examples:

```text
type:code app:vscode project:keyina
type:image ocr:"MSB4236"
domain:learn.microsoft.com after:yesterday
profile:development "clipboard owner"
source:edge has:html
fidelity:full pinned:true
type:file ext:pdf
before:2026-08-01 tag:research
```

Rules:

- plain typing remains useful without syntax;
- parse into typed AST and parameterized FTS/SQL;
- invalid filters display an editable explanation and continue searching valid plain terms where unambiguous;
- obsolete requests cancel while typing;
- first page arrives incrementally;
- selection remains stable as results update;
- sensitive snippets/suggestions remain absent.

OCR query fields appear only after OCR exists and is enabled.

## 6. Result item

Visible by default:

- privacy-safe content preview or type placeholder;
- source app/site/project according to privacy policy;
- relative timestamp;
- type/representation summary;
- profile indicator;
- pin state;
- selected state.

Conditional details:

- fidelity when not full/common or when representation choice matters;
- sensitivity/locked state without revealing content;
- duplicate stack count;
- rule/derived marker;
- unavailable file/reference warning.

Source icon is supportive, not the only source identification.

## 7. Selection and actions

Default keys:

- Enter: paste preferred representation;
- Shift+Enter: paste plain text;
- Ctrl+Enter: copy without pasting;
- Alt+Enter: details;
- Ctrl+P: pin;
- Delete: configured delete flow;
- Tab/Shift+Tab: move focus regions;
- context menu key/Shift+F10: item actions.

The selected primary action is visible in text where ambiguity exists. Representation selector exposes only available safe options and compatibility notes.

Multi-select supports:

- add to collection/tag;
- delete according to policy;
- export;
- queue for sequential paste;
- copy/paste multiple items only where semantics are explicit.

No silent concatenation of unrelated clips.

## 8. Preview and details

Preview is optional and lazy:

- text/HTML/RTF rendered through a sanitized non-executing path;
- images decode only when selected and bounded;
- files display metadata/reference state, not automatic content opening;
- custom formats show safe metadata or developer inspector only;
- sensitive/private content remains absent from ordinary view models until authorized;
- accessibility tree, thumbnails, cached previews, and background subscriptions exclude hidden content;
- window display-affinity/capture exclusion may be used as defense in depth but is never represented as guaranteed protection or DRM.

Details include representations, source, occurrences, transformations, fidelity, size, tags, collections, rule matches, and security policy as permitted.

## 9. Duplicate stacks and versions

Identical payloads may collapse visually while preserving occurrences.

A stack exposes:

- most recent occurrence by default;
- source/time/profile count summary;
- expand to individual occurrences;
- delete one occurrence versus all references clearly separated.

Version relationships are not inferred broadly in early MVP. Explicit parent/derived relationships are always shown; similarity-based version chains wait for their later design.

## 10. Loading, empty, and error states

- **Initial empty:** explain capture state and hotkey; no fake sample history unless clearly labeled demonstration mode.
- **No results:** show active query/filter summary and one clear filter reset action.
- **Search unavailable:** agent/storage reason and retry; do not open DB directly.
- **Profile locked:** unlock action without content hints.
- **Low disk/capture paused:** persistent concise banner.
- **Item corrupt/unavailable:** keep row metadata, disable unsafe paste, offer details/recovery.
- **Agent/protocol mismatch:** update/restart guidance and safe exit.

## 11. Performance

- Warm invocation target: p95 under 80 ms when the manager process/minimal Quick Paste services are already initialized.
- Cold activation is measured separately and receives a budget only after the bootstrap prototype; it is never hidden inside a warm claim.
- First results target: p95 under 30 ms at 100,000 ordinary text records on reference hardware after the search connection is ready.
- Do not load all history or image blobs.
- Virtualize/recycle rows without stale accessibility or privacy state.
- Preview/cache sizes are bounded and cleared on lock/profile changes.
- Search cancellation cannot starve capture or storage operations.

## 12. Accessibility

- Search, results, preview, actions, and status have explicit landmarks/automation structure.
- Announce result count changes without excessive chatter.
- Result name combines safe preview/type, source, time, and relevant state.
- Selection and focus are distinct.
- Grid/list mode exposes correct patterns and positions.
- Keyboard flow works at 300% DPI/text scaling and in RTL.
- High contrast and hidden-sensitive states remain understandable without icons/color.

## 13. Acceptance criteria

- Opens only on explicit invocation and on correct monitor.
- Standard search editing/IME remains intact.
- Search/filter examples parse and produce expected deterministic IDs.
- Obsolete queries cancel and selection does not jump.
- Destination revalidation prevents wrong-target paste; elevated/UIPI or uncertain dispatch falls back to clipboard-only/manual paste without false success.
- Original/plain/copy-only modes behave distinctly.
- Sensitive/private content never leaks through preview, snippet, accessibility, thumbnail, or cache.
- Keyboard-only, Narrator, high contrast, mixed DPI, RDP, long localization, and touch mode pass.
- Warm and cold activation, first frame, first result, and retained-process cost are measured separately, not inferred.
