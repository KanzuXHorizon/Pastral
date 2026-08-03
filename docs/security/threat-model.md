# Threat model

**Date:** 2026-08-03
**Scope:** Pastral MVP architecture on Windows 11, standard-user context.
**Method:** Asset/trust-boundary analysis informed by STRIDE. This model defines required controls and tests; it does not claim complete security.

## 1. Security objectives

1. Never interfere with the source application's successful copy operation.
2. Prevent other local users, logon sessions, remote pipe clients, stale/wrong components, and accidental clients from reading or controlling Pastral through supported interfaces. Do not claim a strong confidentiality boundary against a fully compromised process already running as the same unlocked Windows user.
3. Keep clipboard payloads out of logs, passive notifications, crash diagnostics, update traffic, and network communication.
4. Treat every clipboard representation, parser input, file path, database file, IPC message, import, and backup as untrusted.
5. Preserve stored-original integrity and transformation provenance.
6. Detect corruption/tampering and fail safe for storage and replay.
7. Limit the effect of worker/parser compromise.
8. Respect source-owner history exclusions and user deny/privacy policy before convenience rules.

## 2. Assets

### Highest sensitivity

- captured clipboard payloads;
- sensitive/private-profile encryption keys and decrypted buffers;
- source titles, URLs/domains, paths, project names, and application identity;
- searchable text and OCR/derived text;
- exported backups;
- rule actions capable of changing pasted content;
- update/signing identity.

### Integrity/availability assets

- immutable original representations;
- database schema and migrations;
- transformation provenance;
- rule priority/scope/audit history;
- retention and deletion state;
- executable/package authenticity;
- agent availability and clipboard listener reliability.

## 3. Adversaries

- malformed or malicious clipboard owner in the same user session;
- compromised application publishing hostile formats, streams, handles, paths, HTML, images, or delayed-render callbacks;
- another local process running as the same user, including malware that may share the user's file/DPAPI authority;
- another logged-on session or different local user;
- malicious/corrupt import, backup, database, blob, or settings file;
- malicious update/package or compromised signing pipeline;
- accidental user configuration that broadens capture or retention;
- physical attacker with access to unlocked session or offline disk;
- parser/library vulnerability;
- non-malicious faults: power loss, low disk, antivirus interference, crash, suspend, device loss, partial update.

Out of scope as a guaranteed defense:

- an attacker with administrator/SYSTEM control of the running machine;
- kernel compromise;
- screen capture/keylogging in the same interactive session;
- secure deletion guarantees on SSDs and backup media;
- recovery of DPAPI-protected data after destructive account credential/profile loss;
- strong confidentiality from a fully compromised process already running as the same unlocked user, which may access user files, invoke user-scope DPAPI, inspect the desktop, or inject into peer processes according to Windows policy.

## 4. Trust boundaries

1. **Windows clipboard/OLE boundary:** foreign `IDataObject`, `FORMATETC`, `STGMEDIUM`, registered formats, handles, streams, and delayed-render behavior enter agent-controlled code.
2. **Agent/worker boundary:** bounded input leaves the trusted core; output is untrusted until validated.
3. **Agent/manager/CLI boundary:** local clients cross named-pipe authentication and schema validation.
4. **Agent/storage boundary:** disk state may be corrupt, stale, replaced, or partially committed.
5. **Encryption boundary:** plaintext exists transiently in agent/worker memory after policy authorization.
6. **Package/update boundary:** executable trust changes across installation/update.
7. **Export/import boundary:** user-selected external files may disclose content or contain hostile structures.

## 5. Threats and controls

### T1 — Oversized or malformed clipboard allocation

**Threat:** Length integer overflow, decompression bomb, invalid DIB/RTF/HTML length, infinite stream, or hostile custom format exhausts memory/CPU or crashes the agent.

**Controls:**

- validate arithmetic before allocation;
- per-format and aggregate capture limits;
- bounded stream reads and deadlines;
- preserve encoded bytes without decoding on capture path;
- isolate complex parsing in worker;
- worker memory/time/output quotas and job object;
- fuzz format adapters and malformed fixtures;
- skip with `UnsafeOrUnsupported` rather than trust.

**Tests:** malformed lengths, huge `IStream`, decompression ratios, DIB dimensions/stride, cyclic/invalid serialized structures, cancellation, worker kill.

### T2 — Delayed-render or foreign clipboard call hangs/re-enters

**Threat:** `IDataObject::EnumFormatEtc/GetData`, `IStream`, `GetClipboardData`, a delayed-render callback, or release callback blocks, re-enters messages, exits, changes clipboard, or returns inconsistent media.

**Controls:**

- control/overlay thread never invokes foreign clipboard/OLE methods;
- dedicated capture STA owns foreign interfaces/media and has its own message pump;
- bounded observation queue and soft capture deadlines;
- eligible COM call cancellation is prototyped as best effort, never relied on as a universal hard timeout;
- sequence/current-state recheck where safe;
- short-lived foreign object references and no foreign interfaces crossing into storage/UI;
- watchdog-visible degraded/paused capture state rather than `TerminateThread` or unbounded replacement threads;
- separate capture-broker review if non-cooperative fixtures cannot be recovered acceptably;
- fixture producer for delayed rendering, blocked Win32/OLE calls, owner exit, cancellation refusal, and re-entrancy.

### T3 — Hard-deny or exclusion bypass

**Threat:** Rules/settings capture content despite application/history exclusion signals or password-manager/private-context policy.

**Controls:**

- policy precedence encoded as a pure tested invariant;
- `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0` evaluated before payload reads where technically possible;
- deny/privacy rules outrank all transformations and learned rules;
- no normal UI override;
- source-owned hard deny creates no durable clip or audit row; only ephemeral aggregate health counters are permitted;
- other policy skips may contain a policy ID but never content according to the privacy model.

### T4 — Secret detector false negative

**Threat:** Token/password/private key reaches storage, preview, log, FTS, or overlay.

**Controls:**

- application/context deny rules;
- conservative high-confidence detectors before durable persistence;
- content-free logs and passive overlays independently of detector;
- explicit sensitive flags propagated to storage/search/export;
- synthetic secret corpus and mutation tests;
- users can deny apps/profiles and pause capture.

**Residual risk:** No detector can identify every secret. UI and documentation must state this.

### T5 — Secret detector false positive

**Threat:** Ordinary content is silently discarded.

**Controls:**

- destructive skip only at high confidence or hard policy;
- hidden content-free `SensitiveItemSkipped` audit by default with coarse metadata and 24-hour retention; user may disable/shorten it;
- user-visible policy explanation without value;
- configurable narrow encrypted retention for intentional use;
- detector version/audit and false-positive fixtures.

### T6 — Named-pipe unauthorized access, squatting, or spoofing

**Threat:** Another user/session, remote client, stale/wrong component, or accidental client reads history, changes rules, issues paste/delete/export, creates the first pipe instance, or impersonates the agent/client. A fully compromised same-user process may also attempt these operations and is not treated as a strongly isolated confidentiality principal.

**Controls:**

- explicit least-privilege DACL granting normal client access to the current logon SID; do not add a broad user-SID allow ACE that would defeat same-account cross-session isolation; SYSTEM only when justified;
- runtime validation of token user SID, enabled logon SID, and session ID;
- `FILE_FLAG_FIRST_PIPE_INSTANCE` and `PIPE_REJECT_REMOTE_CLIENTS` where supported;
- validate client PID, session, token user, integrity, and package/signature evidence where useful;
- bounded impersonation only for peer checks, followed by prompt revert;
- DPAPI-protected installation-secret challenge, nonce, and transcript/instance binding for anti-confusion/replay defense—not as a same-user malware guarantee;
- protocol/version/capability negotiation;
- message and response-size limits;
- authorization per operation and explicit foreground user intent for sensitive reveal/export/destructive actions;
- no content-returning defaults in CLI; private/sensitive output requires separate authorization;
- IPC fuzzing, first-instance squatting, remote, cross-user/session, stale-client, replay, and same-user residual-risk tests/documentation.

### T7 — IPC resource exhaustion

**Threat:** Same-user client opens many connections, streams requests, or triggers expensive searches/exports.

**Controls:**

- per-client and global connection limits;
- request deadlines, pagination, cancellation, and quotas;
- bounded subscriptions and outbound queues;
- cheap authentication before expensive parsing;
- rate limits for destructive/expensive operations;
- abandon blocked clients without blocking capture.

### T8 — Worker escape or excessive access

**Threat:** Parser exploit reads all history, keys, network, or launches child processes.

**Controls:**

- restricted token/AppContainer feasibility;
- job kill-on-close, process/memory/time limits;
- no network capability;
- explicit inherited handles only;
- staged job directory isolated from main storage;
- no DB/root-key access;
- validated output and hashes;
- worker binary signature/version check before launch.

### T9 — Database/blob tampering, deletion remnants, or corruption

**Threat:** Modified metadata points to wrong blob, ciphertext is replaced, partial commit leaks or loses data, malicious local file triggers parser.

**Controls:**

- authenticated encryption for sensitive blobs;
- hashes and length checks for ordinary blobs;
- schema constraints and foreign keys where appropriate;
- staging/atomic rename/reconciliation;
- integrity check tooling;
- never execute/open content automatically;
- evaluate SQLite `secure_delete`, freelist/vacuum, rollback journal/WAL checkpoint/retention, snapshots, and backup copies; describe deletion as logical rather than guaranteed forensic erasure;
- quarantine corruption and continue with unaffected data;
- crash/power-loss and tampered-blob tests.

### T10 — Path traversal, reparse points, and unsafe file lists

**Threat:** Clipboard/import paths escape controlled directories, access devices/network shares, or trick worker/output writes.

**Controls:**

- treat copied file paths as references, not files to open automatically;
- canonicalize controlled output paths and reject traversal;
- create files with safe flags and inspect reparse points where relevant;
- use handles rather than trusting repeated path resolution;
- worker output restricted to pre-opened handles/staging root;
- export never follows arbitrary links without explicit user action.

### T11 — Rule confusion or unsafe automatic action

**Threat:** Broad/inferred rule transforms or pastes sensitive/wrong content unexpectedly.

**Controls:**

- explicit user consent or user-enabled repeated-choice threshold;
- narrow scope choice; no silent broadening;
- privacy/deny rules outrank transformations;
- deterministic priority and specificity;
- equal-priority destructive conflict requires resolution;
- simulation before save;
- audit, explanation, undo, pause, rollback, and versioned migrations.

### T12 — Paste to wrong or higher-integrity destination

**Threat:** Foreground changes after Quick Paste selection, focus restoration fails, or UIPI blocks input into a higher-integrity application while Pastral mistakenly reports success or sends input elsewhere.

**Controls:**

- snapshot intended destination process/window/control/integrity evidence where safely available;
- revalidate foreground identity immediately before input;
- remain standard user; do not request `uiAccess`, elevation, a service, or focus-stealing bypasses;
- cancel on unexpected change or uncertain restoration/injection;
- treat `SendInput` result as dispatch evidence, not proof of destination consumption, and do not assume UIPI is diagnosable through `GetLastError`;
- leave data on clipboard and prompt for manual paste when injection is blocked/uncertain;
- compatibility profiles and bounded transaction;
- no secret payload logging.

### T13 — Clipboard restoration destroys newer content

**Threat:** Pastral restores an old clipboard after another application/user copied something.

**Controls:**

- restoration off by default;
- verify Pastral still owns current clipboard;
- sequence/ownership checks;
- conservative destination-read heuristic and timeout;
- skip restoration on uncertainty;
- fixture tests for asynchronous consumers and intervening copies.

### T14 — Overlay focus/input interference

**Threat:** Overlay activates, steals focus, intercepts typing/Escape/numbers, appears in taskbar/Alt+Tab, or covers sensitive UI.

**Controls:**

- `WS_EX_NOACTIVATE`, no-activate show/position, `WM_MOUSEACTIVATE` handling;
- passive overlay has no keyboard hooks/actions;
- explicit interaction mode only;
- foreground/focus assertions in E2E tests;
- fullscreen/password/screen-share suppression;
- taskbar/Alt+Tab/multi-monitor/DPI tests;
- privacy-safe content and placement.

### T15 — Log/crash/diagnostic/accessibility/capture leakage

**Threat:** Payload, secret fragment, source title/path, or encryption key reaches logs, dumps, screenshots, accessibility trees, cached view models, support bundles, or telemetry.

**Controls:**

- structured allowlist logging rather than regex-only redaction;
- content fields absent from logging types;
- bounded rotating files and privacy tiers;
- release crash-dump policy excludes sensitive memory where feasible;
- hidden content is absent from view models, UI Automation, thumbnails, and caches; window display-affinity exclusion is defense in depth only;
- diagnostic bundle preview and explicit user consent;
- secret canary tests across logs and exports;
- no content telemetry.

### T16 — Malicious update or supply-chain compromise

**Threat:** Attacker ships signed/unsigned altered binaries or dependency compromise reaches clipboard data.

**Controls:**

- signed MSIX and publisher verification;
- protected signing keys and least-privilege CI;
- pinned dependencies/toolchains and lockfiles;
- dependency/license/advisory/secret scans;
- reproducibility metadata and artifact provenance;
- no unsigned in-app execution;
- staged update and incident rollback plan.

### T17 — Offline or same-user data theft

**Threat:** Attacker reads ordinary history from disk while Windows user is logged out/offline, or a malicious process running as the same unlocked user accesses files or user-scope DPAPI material.

**Controls:**

- document that ordinary profile payloads are not encrypted by default and are not protected from fully compromised same-user processes;
- recommend BitLocker/device encryption;
- encrypted Private/sensitive profiles;
- optional metadata encryption evaluation;
- easy retention/delete controls.

**Residual risk:** DPAPI protects selected keys against offline/cross-user access under its Windows account model; it is not a secure enclave or a same-user malware boundary. Ordinary unprotected history remains readable to an attacker with file access.

### T18 — Clipboard event/format/source identity confusion

**Threat:** Pastral treats a notification or sequence as a unique copy event, persists a runtime-local registered-format number, guesses source/domain/project, or suppresses an external copy as self-generated. This can lose events, replay the wrong format, apply the wrong rule, or violate privacy policy.

**Controls:**

- transient `ClipboardObservation` separate from successful `ClipEvent` and content-free `CaptureAuditEvent`;
- successful clip requires at least one representation;
- sequence values are equality/current-state evidence only; handle zero, wrap, delayed rendering, and unobservable intermediate states;
- self-suppression requires a validated private transaction marker plus ownership/timing evidence, not sequence alone;
- standard formats persist defined IDs; registered formats persist exact names and are re-registered at replay;
- source context stores evidence type/confidence and never infers domain/project from arbitrary titles by default;
- tests for sequence zero/wrap/gaps, forged/stale markers, registered ID changes, owner/foreground conflict, and PID reuse.

## 6. Security test mapping

| Boundary | Required evidence |
|---|---|
| Clipboard/OLE | fixture producer/consumer, malformed formats, contention, delayed/blocking rendering, cancellation refusal, owner exit, sequence zero/wrap/pressure, registered-name replay, control-thread responsiveness |
| Policy | hard-deny precedence, secret corpus, denylist, profile switching, no value in skipped metadata |
| IPC | DACL/remote/cross-user/session tests, first-instance squatting, PID/token/session checks, handshake replay/stale client, same-user limitation assertions, schema fuzz, oversized messages, authorization/user-intent matrix |
| Worker | restricted-token capabilities, no network, memory/time/process limits, parser fuzz, kill/recovery |
| Storage | migration paths, crash injection, low disk, hash/envelope tamper, orphan reconciliation, integrity check |
| Paste | wrong destination, async read, intervening copy, restoration skip, corrupt blob, compatibility matrix |
| Overlay | foreground/focus preservation, Alt+Tab/taskbar, fullscreen, DPI, RDP, reduced motion |
| Logging/export | secret canaries, path/title redaction, diagnostic levels, crash artifact inspection |
| Update/package | signature/publisher validation, tampered package rejection, migration rollback/recovery |

## 7. Review cadence

Update this threat model before merging any change that adds network access, a parser/codec, another resident process, scripting/plugins, browser/IDE integration, cloud/device sync, semantic indexing, public protocol, new encryption scheme, privileged component, or data export path.
