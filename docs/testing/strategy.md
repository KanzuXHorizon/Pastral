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
- blob identity/reference counting, `sha256-raw-v1` byte-domain semantics, and protection-domain isolation;
- opaque UUIDv4 generation/serialization, persistent installation-local `capture_order`, and deterministic time/order behavior across wall-clock rollback and agent restart;
- transformation provenance;
- envelope parser and migration helpers;
- redaction/log-field allowlists.

Unit tests run cross-platform only where the module has no Windows contract. Windows-specific logic uses Windows CI.

### Integration tests

- SQLite repositories and constraints;
- FTS query semantics/ranking/filter composition;
- schema migration from every supported version;
- internal SQLite BLOB and external-file backends behind one `BlobStore`, threshold-policy selection, `sha256-raw-v1` backend independence, staging/finalization, transactional migration, reference counting, and reconciliation;
- rollback-journal/WAL selected configuration;
- encryption round trip/tamper/key rotation;
- fixed 36-byte IPC frame parsing with explicit bulk sequence, Protobuf Edition 2024 control-schema parsing/post-parse validation, DTO-domain conversion, selected generator/generated-code/runtime compatibility checks, official-kernel versus credible wire-compatible Rust-runtime footprint/build evidence, negotiation, cross-user/session peer validation, first-instance anti-squatting, authorization classes, pagination, cancellation, and same-user limitation assertions;
- agent-worker job validation;
- profile switching, immutable capture-profile preservation, Private protection domain, and retention boundaries;
- import/export and diagnostic redaction.

### Native clipboard fixtures

Build separate fixture producer and consumer executables. Producer can advertise:

- Unicode and ANSI text;
- HTML Clipboard Format and RTF;
- URLs;
- CF_HDROP file lists;
- DIB, DIBV5, bitmap, encoded PNG;
- multiple simultaneous formats;
- registered formats whose runtime numeric ID changes while the persisted exact name remains stable;
- unknown custom formats that remain metadata-only/unsupported;
- `HGLOBAL`, `IStream`, `STGMEDIUM` ownership/`pUnkForRelease`, `lindex`, and justified media;
- delayed rendering;
- malformed sizes/media;
- owner exit;
- contention and rapid updates, sequence unavailable/zero, unsigned-wrap abstraction, queued duplicate notifications, unobservable intermediate-state pressure, and final-current-state capture;
- valid, stale, forged, and malformed Pastral origin markers;
- history/cloud exclusion formats.

Consumer records offered format order, requested `FORMATETC`/`TYMED`, exact bytes, asynchronous reads, and clipboard ownership without leaking data into ordinary logs.

### End-to-end tests

Critical flows:

1. fixture copy → capture → restart → search → replay → fixture verification;
2. original versus plain/derived representation;
3. source/profile/type/time filters;
4. passive overlay foreground/focus preservation;
5. Quick Paste cold/warm manager-process activation, focus restoration, destination/integrity revalidation, ordinary paste, and elevated/UIPI clipboard-only/manual fallback;
6. denylist, source-owned hard deny with no durable row, pause, hidden sensitive skip with coarse 24-hour audit, and Private-profile encryption/non-indexing;
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

- fuzz clipboard/custom-format adapters, HTML/RTF/image metadata, query parser, fixed IPC frame parser, protobuf parser/validator, DTO-domain conversion, bulk-transfer state machine, import, and encryption envelope;
- oversized/truncated/decompression/path traversal/reparse cases;
- named-pipe explicit DACL, remote rejection, first-instance squatting, cross-user/session access, PID/token/session checks, bounded impersonation/revert, and documented same-user residual boundary;
- handshake replay, invalid/unknown frame kind/flags/sequence/length/UUID, unknown Protobuf action/enum, missing explicit presence, duplicate keys, capability mismatch, adjacent-version golden fixtures, and unauthorized operations;
- clipboard-platform STA blocking/re-entrancy/cancellation-refusal and replay-callback isolation while the control/overlay thread remains responsive;
- worker no-network/limits/escape checks;
- tampered internal/external blobs, backend/reference mismatches, database, and encrypted envelopes;
- wrong-user DPAPI access and checks that documentation does not present user-scope DPAPI as a same-user malware boundary;
- secret canaries across DB, FTS, SQLite freelists/journal/WAL where testable, blobs, logs, overlay, UI Automation, diagnostics, export, and crash artifacts;
- whole-message and chunked AEAD authentication-before-release, chunk reorder/duplicate/truncate/splice cases;
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
- C++ build with project warnings as errors plus exact selected Protobuf generator/generated-code/runtime verification when IPC exists;
- native manager/UI tests where environment supports them;
- migration and storage recovery tests;
- clipboard fixture smoke tests on Windows;
- dependency advisory and license checks;
- secret scan and static analysis;
- fuzz smoke corpus for frame/Protobuf/domain/bulk-transfer and other hostile parsers;
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
