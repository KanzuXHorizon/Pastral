# Testing strategy

## 1. Principles

- Behavior and bug fixes use test-first development when reproducible.
- Tests assert observable contracts, not private implementation details.
- Clipboard bytes, source context, focus state, policy decisions, and recovery outcomes are verified at the lowest reliable layer and again in critical end-to-end paths.
- Real secrets and personal clipboard histories are prohibited test data.
- Flaky tests are not hidden. Quarantine requires an issue, owner, reason, reproduction evidence, and removal deadline.
- Windows/API/version assumptions are validated on supported builds, not inferred from compilation.

## 2. Test layers

### Unit tests

Pure deterministic tests for:

- domain invariants and immutable-original rules;
- representation/fidelity aggregation;
- classification and sensitivity decisions;
- privacy/deny precedence;
- rule priority, specificity, conflict, simulation, audit, and rollback;
- query parser and typed AST;
- retention/quota selection;
- blob identity/reference counting;
- transformation provenance;
- envelope parser and migration helpers;
- redaction/log-field allowlists.

Unit tests run cross-platform only where the module has no Windows contract. Windows-specific logic uses Windows CI.

### Integration tests

- SQLite repositories and constraints;
- FTS query semantics/ranking/filter composition;
- schema migration from every supported version;
- blob staging/finalization/reconciliation;
- rollback-journal/WAL selected configuration;
- encryption round trip/tamper/key rotation;
- IPC serialization, negotiation, authorization, pagination, cancellation;
- agent-worker job validation;
- profile switching and retention boundaries;
- import/export and diagnostic redaction.

### Native clipboard fixtures

Build separate fixture producer and consumer executables. Producer can advertise:

- Unicode and ANSI text;
- HTML Clipboard Format and RTF;
- URLs;
- CF_HDROP file lists;
- DIB, DIBV5, bitmap, encoded PNG;
- multiple simultaneous formats;
- custom registered formats;
- `HGLOBAL`, `IStream`, and justified media;
- delayed rendering;
- malformed sizes/media;
- owner exit;
- contention and rapid updates;
- history/cloud exclusion formats.

Consumer records offered format order, requested `FORMATETC`/`TYMED`, exact bytes, asynchronous reads, and clipboard ownership without leaking data into ordinary logs.

### End-to-end tests

Critical flows:

1. fixture copy → capture → restart → search → replay → fixture verification;
2. original versus plain/derived representation;
3. source/profile/type/time filters;
4. passive overlay foreground/focus preservation;
5. Quick Paste invocation, focus restoration, destination revalidation, and paste;
6. denylist, hard-deny format, pause, and sensitive skip;
7. explicit narrow rule creation → later match → explanation → undo/pause/delete;
8. crash during staged write and recovery;
9. corrupt/unsupported format containment;
10. install/update/uninstall with chosen data-retention options.

Use isolated Windows user profiles or desktops where practical. Never run destructive tests against a developer's ordinary history store.

### UI and accessibility tests

Automated checks:

- keyboard traversal, focus order, focus restoration;
- accessible names, roles, states, patterns, and notifications;
- standard search-box editing shortcuts;
- virtualization and selection stability;
- loading, empty, error, denied, locked, offline, low-disk, and recovery states;
- light/dark/high contrast;
- text scaling and 100–300% DPI;
- multiple monitors and work areas;
- RTL and long localized strings;
- reduced motion and transparency disabled;
- touch target geometry and pointer/touch behavior.

Manual release checks use Narrator, Magnifier, On-Screen Keyboard, keyboard-only navigation, contrast themes, text scaling, touch where available, RDP, and screen sharing.

### Performance tests

Follow `docs/performance/benchmark-methodology.md` for idle, capture, overlay, Quick Paste, search, paste, storage, startup, profile switch, and maintenance. CI runs smoke thresholds; release hardware produces authoritative reports.

### Security tests

- fuzz clipboard/custom-format adapters, HTML/RTF/image metadata, query parser, IPC, import, and encryption envelope;
- oversized/truncated/decompression/path traversal/reparse cases;
- named-pipe ACL and cross-user/session access;
- handshake replay and unauthorized operations;
- worker no-network/limits/escape checks;
- tampered blobs/database/encrypted envelopes;
- wrong-user DPAPI access;
- secret canaries across DB, FTS, blobs, logs, overlay, diagnostics, export, crash artifacts;
- hard-deny and denylist precedence;
- malicious/tampered package rejection.

### Packaging and release tests

- clean install and first run;
- startup registration enable/disable;
- repair;
- same-version and upgrade install;
- migration interruption/recovery;
- publisher/signature/architecture validation;
- uninstall with keep/delete-data options as documented;
- side-load and Store-equivalent package behavior;
- app execution aliases and process activation;
- no unexpected network traffic;
- data remains usable across supported update paths.

## 3. Test ownership by module

| Module | Primary tests |
|---|---|
| domain | Unit/property invariants |
| clipboard-win/formats | Native fixtures, malformed input, COM/handle lifetime |
| capture | Sequence/coalescing/retry/policy/persistence E2E |
| paste | IDataObject consumer fixtures and destination matrix |
| storage/search | Integration, migration, recovery, scale benchmarks |
| security | Detector, envelope, IPC ACL/auth, redaction, fuzz |
| rules/profiles | Pure replayable unit and integration tests |
| overlay | HWND activation/focus, rendering/DPI/power states |
| manager/Quick Paste | UI Automation, keyboard, virtualization, accessibility |
| packaging | Installation/update/uninstall and signing |

## 4. CI quality gates

Required lanes before release:

- formatting and repository policy;
- Rust build/tests and Clippy warnings denied;
- C++ build with project warnings as errors;
- native manager/UI tests where environment supports them;
- migration and storage recovery tests;
- clipboard fixture smoke tests on Windows;
- dependency advisory and license checks;
- secret scan and static analysis;
- fuzz smoke corpus;
- package validation and clean-install smoke;
- documentation link/consistency checks;
- performance smoke without noisy user-facing claims.

A required failing lane blocks release. Retry is allowed only to diagnose infrastructure; passing by retry does not erase flakiness evidence.

## 5. Test result reporting

Every milestone report includes:

- exact commands executed;
- tested commit/artifact hash;
- Windows/hardware/environment;
- pass/fail/skip counts;
- skipped tests and reason;
- benchmark values and comparison;
- discovered defects and remaining risks;
- deviations from specification.

Do not report a test as passed unless it was freshly run against the stated artifact.
