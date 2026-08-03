# Threat model

**Date:** 2026-08-03
**Scope:** Pastral MVP architecture on Windows 11, standard-user context.
**Method:** Asset/trust-boundary analysis informed by STRIDE. This model defines required controls and tests; it does not claim complete security.

## 1. Security objectives

1. Never interfere with the source application's successful copy operation.
2. Prevent unauthorized local users/sessions/processes from reading or controlling Pastral through supported interfaces.
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
- another local process running as the same user;
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
- recovery of DPAPI-protected data after destructive account credential/profile loss.

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

### T2 — Delayed-render callback hangs or re-enters

**Threat:** Foreign `IDataObject::GetData` blocks, re-enters messages, exits, changes clipboard, or returns inconsistent media.

**Controls:**

- minimal operations on the STA/message thread;
- bounded capture deadline and attempt policy;
- sequence recheck where safe;
- short-lived foreign object reference;
- cancellation/degradation rather than blocking copy;
- fixture producer for delayed rendering, owner exit, and re-entrancy.

### T3 — Hard-deny or exclusion bypass

**Threat:** Rules/settings capture content despite application/history exclusion signals or password-manager/private-context policy.

**Controls:**

- policy precedence encoded as a pure tested invariant;
- `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0` evaluated before payload reads where technically possible;
- deny/privacy rules outrank all transformations and learned rules;
- no normal UI override;
- audit metadata contains policy ID, not content.

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
- optional content-free `SensitiveItemSkipped` event;
- user-visible policy explanation without value;
- configurable narrow encrypted retention for intentional use;
- detector version/audit and false-positive fixtures.

### T6 — Named-pipe unauthorized access or spoofing

**Threat:** Another process reads history, changes rules, issues paste/delete/export, or impersonates the agent.

**Controls:**

- explicit DACL for current user and logon SID;
- validate client PID/token/session;
- DPAPI-protected installation secret challenge-response;
- nonce and transcript binding;
- protocol/version/capability negotiation;
- message and response-size limits;
- authorization per operation;
- no content-returning defaults in CLI;
- IPC fuzzing and cross-user/session tests.

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

### T9 — Database/blob tampering or corruption

**Threat:** Modified metadata points to wrong blob, ciphertext is replaced, partial commit leaks or loses data, malicious local file triggers parser.

**Controls:**

- authenticated encryption for sensitive blobs;
- hashes and length checks for ordinary blobs;
- schema constraints and foreign keys where appropriate;
- staging/atomic rename/reconciliation;
- integrity check tooling;
- never execute/open content automatically;
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

### T12 — Paste to wrong destination

**Threat:** Foreground changes after Quick Paste selection and synthetic paste enters another application.

**Controls:**

- snapshot intended destination process/window/control where safely available;
- revalidate foreground identity immediately before input;
- cancel on unexpected change;
- leave data on clipboard for manual paste after dispatch failure;
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

### T15 — Log/crash/diagnostic leakage

**Threat:** Payload, secret fragment, source title/path, or encryption key reaches logs, dumps, screenshots, support bundles, or telemetry.

**Controls:**

- structured allowlist logging rather than regex-only redaction;
- content fields absent from logging types;
- bounded rotating files and privacy tiers;
- release crash-dump policy excludes sensitive memory where feasible;
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

### T17 — Offline data theft

**Threat:** Attacker reads ordinary history from disk while Windows user is logged out or disk is mounted elsewhere.

**Controls:**

- document that ordinary profile payloads are not encrypted by default unless policy changes;
- recommend BitLocker/device encryption;
- encrypted Private/sensitive profiles;
- optional metadata encryption evaluation;
- easy retention/delete controls.

**Residual risk:** DPAPI alone does not encrypt ordinary unprotected history.

## 6. Security test mapping

| Boundary | Required evidence |
|---|---|
| Clipboard/OLE | fixture producer/consumer, malformed formats, contention, delayed rendering, owner exit, sequence storms |
| Policy | hard-deny precedence, secret corpus, denylist, profile switching, no value in skipped metadata |
| IPC | ACL/cross-session tests, handshake replay, schema fuzz, oversized messages, authorization matrix |
| Worker | restricted-token capabilities, no network, memory/time/process limits, parser fuzz, kill/recovery |
| Storage | migration paths, crash injection, low disk, hash/envelope tamper, orphan reconciliation, integrity check |
| Paste | wrong destination, async read, intervening copy, restoration skip, corrupt blob, compatibility matrix |
| Overlay | foreground/focus preservation, Alt+Tab/taskbar, fullscreen, DPI, RDP, reduced motion |
| Logging/export | secret canaries, path/title redaction, diagnostic levels, crash artifact inspection |
| Update/package | signature/publisher validation, tampered package rejection, migration rollback/recovery |

## 7. Review cadence

Update this threat model before merging any change that adds network access, a parser/codec, another resident process, scripting/plugins, browser/IDE integration, cloud/device sync, semantic indexing, public protocol, new encryption scheme, privileged component, or data export path.
