# Product scope

## Supported platform

- Windows 11 only.
- x64 first.
- MVP engineering floor: Windows 11 24H2, build 26100.
- Release-candidate floor: oldest Windows 11 Home/Pro release still serviced by Microsoft; expected Windows 11 25H2, build 26200, after 2026-10-13.
- ARM64 begins after x64 correctness, security, accessibility, performance, packaging, and compatibility gates pass.

## Phase 0: foundation

Documentation and repository governance only:

- official-source research;
- competitor/gap analysis;
- product scope and non-goals;
- architecture and diagrams;
- ADRs;
- threat/privacy/encryption models;
- performance and benchmark methodology;
- UX/accessibility foundation;
- testing strategy;
- brand direction/name research;
- repository initialization and release plans.

No feature code is permitted in Phase 0.

## MVP 1: reliable foundation

- event-driven clipboard listener;
- clipboard sequence handling and self-update suppression;
- bounded contention retry;
- capture of Unicode text, ANSI fallback, HTML Clipboard Format, RTF, common images, URLs, and file lists;
- logical `ClipEvent` with multiple `ClipRepresentation` records;
- immutable originals and fidelity states;
- source process/window metadata under privacy policy;
- SQLite metadata and content-addressed blobs;
- exact/lexical search and structured filters;
- basic CLI status/search/health;
- pause/resume, denylist, retention, and quota;
- fixture producer/consumer and crash-recovery tests.

## MVP 2: replay and interaction

- high-fidelity replay for supported common formats;
- original, preferred, and plain-text paste modes;
- copy-only mode without synthetic paste;
- focus-safe confirmation overlay;
- overlay modes and suppression policy;
- Quick Paste keyboard-first UI hosted as an activation/window mode in the single-instance on-demand manager process;
- type/source/time filters;
- pin and delete;
- compatibility and performance evidence.

## MVP 3: profiles and deterministic actions

- General, Development, Study, Work, and Design profiles;
- Private profile only after mandatory encrypted payload storage, random blob IDs, non-indexing, hidden-preview, ordinary-export exclusion, lock/key-clear, and recovery gates pass;
- explicit profile switching;
- up to four context actions;
- derived representations with provenance;
- deterministic versioned rules;
- narrow remember-choice scopes;
- simulation, conflict handling, audit history, explanation, pause, edit, delete, and rollback;
- duplicate stacks without event loss.

## MVP 4: full manager

- WinUI 3 manager shell;
- Home, History, Collections, Sources, Profiles, Rules, Paste Queue, Storage, Privacy, Settings, Diagnostics, and About;
- virtualized history;
- clip details and representation inspector;
- backup/export/import;
- accessibility validation;
- installer/update/uninstall and data-retention controls.

## Later work

- OCR worker;
- browser extension and reliable private-mode/domain signals;
- IDE/project integrations;
- version chains and copy sessions;
- sequential paste queues and multi-item workflows;
- Windows Hello-backed private vault UX;
- optional local semantic search;
- optional encrypted device sync.

Every later item requires a separate design and privacy/security review.

## Non-goals for first release

- Windows 10 support;
- cloud account or mandatory sign-in;
- remote AI or mandatory local model;
- cloud sync;
- team collaboration;
- arbitrary scripting engine;
- plugin marketplace or third-party in-process plugins;
- AI rewriting assistant;
- universal capture/replay of every private application format;
- running OCR or semantic models continuously;
- clipboard polling;
- destructive normalization of originals;
- silently broad learned rules;
- working-set trimming as an optimization strategy;
- embedding Chromium, WebView, Electron, or Tauri as primary UI;
- installer, update, or sync behavior that is not separately tested and documented.

## Default policy decisions

### Retention

- 90 days.
- 5 GB automatic-cleanup target for ordinary unpinned history, not an absolute cap.
- Pinned/protected clips are exempt from automatic retention deletion and may exceed the target with visible warnings.
- A separately measured low-disk reserve may suspend new payload capture while preserving copy behavior and diagnostic state; pinned content is not silently deleted.

### Sensitive content

- Highly confident secret payloads are not stored by default.
- A hidden `SensitiveItemSkipped` audit record is created by default for 24 hours with broad detector/policy class, active profile, and coarse timestamp only; users may disable or shorten it.
- It contains no preview, content hash, OCR, snippet, precise source, title, path, domain, payload size, value structure, or reconstructable value.
- Encrypted sensitive retention is opt-in and narrowly scoped by profile, application, content class, and duration.
- Sensitive items never enter FTS, OCR, semantic indexing, duplicate stacks, preview caches, passive overlays, or ordinary exports.
- Windows Hello remains a later user-presence layer and is not a substitute for Private-profile encryption.

### Source exclusions

- Known password managers are denied by default.
- Reliably identified private browser contexts are denied by default.
- Clipboard-owner history-exclusion signals are hard deny, cannot be overridden by ordinary settings, and create no durable clip/audit row.
- Private-browser detection without an explicit integration is best effort and must remain confidence-labeled rather than claimed as universal.

## Scope-change rule

Changes that add a resident process, network access, public protocol, executable code extension, database owner, new sensitive-data path, supported operating system, or broad background job require an ADR and threat-model update before implementation.
