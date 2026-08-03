# Manager application UX

## 1. Purpose

The manager is the deliberate workspace for inspecting, organizing, configuring, recovering, and explaining Pastral. It does not compete with Quick Paste for fast retrieval and does not run continuously.

## 2. Information architecture

Primary navigation:

1. Home
2. History
3. Collections
4. Sources
5. Profiles
6. Rules
7. Paste Queue
8. Storage
9. Privacy
10. Integrations
11. Settings
12. Diagnostics
13. About

Navigation may group lower-frequency administration destinations visually, but names and deep-link identity remain stable.

## 3. Shell

- WinUI 3 desktop shell using native title bar/navigation behavior unless a custom title bar proves accessibility and window-management parity.
- Mica where supported; solid system fallback.
- Navigation supports compact/expanded modes, keyboard, access keys where appropriate, back/forward history, and restored page context.
- Global search/settings entry does not replace History search semantics.
- Persistent status surfaces show current profile, capture state, locked/private state, low disk, recovery, or agent mismatch when relevant.

## 4. Home

Show actionable state, not a productivity dashboard:

- current profile and capture state;
- recent clips using privacy-safe previews;
- pinned collections;
- storage usage/retention status;
- unresolved rule conflicts or failed jobs only when action is useful;
- onboarding/permission steps until completed.

Do not show gamified counts, streaks, productivity scores, invented savings, or alarming security statistics.

## 5. History

### Layout

- virtualized timeline/list as default;
- compact/list/card user choice;
- date grouping with sticky headers where accessible;
- source/profile/type grouping on demand;
- optional detail pane;
- advanced query/filter builder that maps to the same typed search model as Quick Paste.

### Actions

- paste/copy selected representation;
- pin/favorite;
- tag/add to collection;
- inspect representations/provenance;
- delete occurrence/stack/item according to scope;
- export;
- create narrow rule from an action;
- open source/file/URL only through explicit safe action and confirmation policy.

### Bulk behavior

Show selected count, total affected payload/storage, pinned/sensitive exclusions, and whether operation targets occurrences, shared payloads, derived representations, or originals.

## 6. Clip detail

Sections:

- safe preview;
- source and confidence/privacy state;
- copy occurrences;
- optional paste occurrences;
- profile;
- representation set;
- original/derived graph;
- fidelity and compatibility notes;
- sizes and hashes according to sensitivity policy;
- tags/collections/notes;
- related/duplicate clips;
- rule matches and transformations;
- retention/encryption/security policy;
- export/delete/recovery controls.

Developer mode may inspect raw format metadata and hashes but never bypass sensitive authorization or render untrusted content directly.

## 7. Collections

- manual collections;
- smart collections defined by typed queries;
- pinned collections on Home/Quick Paste;
- collection membership does not duplicate payloads;
- smart collection preview/simulation before save;
- collection deletion distinguishes container from clips.

## 8. Sources

Browse by:

- application/package;
- website/domain when reliable and permitted;
- project/workspace;
- window class or broad source type in advanced mode.

Each source shows capture count/storage/time range, policy, profile behavior, and “do not capture this source” action. Privacy settings can hide titles/domains or delete source metadata/history.

## 9. Profiles

Profile detail covers:

- retention/quota;
- capture allow/deny;
- sensitive policy;
- OCR/indexing policy;
- default actions/paste preferences;
- accent/name/icon;
- encryption/lock/auto-lock;
- auto-switch rules;
- storage usage;
- import/export/reset.

Auto-switching is disabled by default except explicit user rules. Simulation explains what profile would activate for a selected context.

Private profile emphasizes behavior in text:

- encrypted payloads;
- hidden previews;
- no OCR/semantic indexing by default;
- no network integrations;
- unlock/auto-lock/session-lock options;
- shorter retention options.

## 10. Rules

### List

- human-readable summary;
- enabled state;
- priority and scope;
- action;
- match count/last match/result;
- conflict/warning state;
- origin: explicit, imported, or user-enabled learned recommendation.

### Editor

1. Name and enabled state.
2. Match facts with exact scope.
3. Action and representation consequences.
4. Priority and conflict resolution.
5. Simulation against selected/synthetic events.
6. Review of privacy/deny rules that outrank it.
7. Save with schema/version audit entry.

Equal-priority destructive conflicts cannot be saved unresolved. Rule JSON is not required for normal editing.

### Audit

Show creation, edits, reorder, matches, results, undo/rollback, and actor/process version without content payload.

## 11. Paste Queue

Later-slice surface prepared in IA:

- ordered items and selected representation;
- target/sequence policy;
- remove/reorder/pause/clear;
- progress and per-item result;
- privacy state and destination revalidation;
- no automatic background pasting without explicit user action.

Before implementation, unavailable navigation may be hidden rather than shipping a dead placeholder.

## 12. Storage

Show:

- total usage and quota;
- metadata, originals, derived data, previews/indexes, encrypted/private data categories;
- age distribution and pinned exclusions;
- cleanup simulation;
- data path;
- integrity check, backup, restore/import, recovery, and vacuum/maintenance actions;
- low-disk state and capture impact.

Cleanup never implies physical secure erasure. Long operations show progress/cancel and yield to capture.

## 13. Privacy

Centralize:

- capture pause and default exclusions;
- denied applications/packages/window patterns;
- private browser/password manager policy;
- sensitive detectors and per-class behavior;
- source title/domain/path retention;
- preview/search/indexing policy;
- private profiles and encryption state;
- session lock/screen-share/fullscreen behavior;
- logs/diagnostics/export privacy;
- delete sensitive/all history.

Every setting states what is stored, what remains metadata, and whether change applies prospectively or retroactively.

## 14. Integrations

Hidden or minimal until real integrations exist. Each future integration requires:

- permission scope;
- data read/sent;
- network destination;
- private/sensitive behavior;
- disable/revoke/delete controls;
- health and version;
- separate threat model.

Do not show fake upcoming provider tiles as functional settings.

## 15. Settings

- searchable settings;
- grouped by behavior rather than implementation modules;
- concise explanation and default;
- reset per section;
- import/export settings separately from history;
- advanced/developer section for technical formats, diagnostics, and compatibility profiles;
- no required JSON editing;
- changes with dangerous broad scope show simulation/consequence.

## 16. Diagnostics

Show content-free health:

- agent/worker/manager/protocol/schema versions;
- process uptime/working set/handles/threads;
- listener/hotkey/tray/overlay state;
- database integrity/journal/size;
- queue/job failures;
- last capture/paste result codes and timings without payload;
- compatibility profile versions;
- network-silence state;
- sanitized diagnostic bundle preview/export.

Developer diagnostics are opt-in, bounded, and cannot expose content by switching a single casual toggle.

## 17. Onboarding

Onboarding is short and skippable:

1. Explain local storage and default sensitive skip.
2. Choose/confirm Quick Paste hotkey and detect conflicts.
3. Choose calm overlay mode/placement or Off.
4. Confirm retention/quota and password-manager/private-context defaults.
5. Run a synthetic copy/paste demonstration without storing personal content.

Avoid account creation, marketing carousel, forced animation, or broad permission prompts before their feature is used.

## 18. States and recovery

Every page covers:

- loading and cancellation;
- empty/new user;
- no search result;
- capture paused;
- profile locked;
- agent unavailable/restarting;
- protocol/version mismatch;
- database migration/recovery;
- low disk/quota;
- permission/policy denied;
- corrupt/quarantined item;
- partial operation;
- offline/network feature disabled where applicable.

## 19. Accessibility and performance

- virtualized controls must recycle UI Automation and privacy state correctly;
- UI thread never blocks on IPC/database/preview decode;
- keyboard order follows visual/task order;
- panes expose landmarks/headings;
- selection, focus, checked, expanded, locked, and unavailable states are distinct;
- layout survives text scaling, 300% DPI, RTL, and long localization;
- preview decoding/caches are lazy and bounded;
- screen-capture thumbnails do not reveal locked/private content where supported.

## 20. Manager acceptance gates

- all navigation and critical actions keyboard/screen-reader operable;
- no direct DB/blob access;
- full state matrix represented;
- virtualized 100k history remains responsive;
- privacy policy consistent across preview, search, accessibility, export, and diagnostics;
- rule simulation/conflict/audit workflows complete;
- storage cleanup and destructive scope understandable;
- light/dark/high contrast, mixed DPI, text scaling, RTL, reduced motion, touch, RDP pass.
