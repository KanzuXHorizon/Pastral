# Phase 3C Privacy Admission Foundation Verification

**Date:** 2026-08-04

**Baseline:** `12a1680`

**Implementation head:** `ac59102`

**Scope:** Windows clipboard history controls, bounded clipboard-owner observation, exact process admission policy, high-confidence private-key skipping, strict privacy configuration, agent/runtime verification

## 1. Result

Phase 3C is **green for its declared fail-closed admission scope**.

The Phase 3B ordinary Unicode-text path now uses this precedence before persistence:

```text
clipboard opened
  -> source-owned Windows history controls
  -> immediate hard deny when required
  -> bounded clipboard-owner process observation
  -> executable-basename admission policy
  -> bounded CF_UNICODETEXT acquisition
  -> high-confidence secret detector
  -> audit-only skip OR ordinary clip persistence
```

Hard-denied observations never reach owner/payload processing and create no durable row. Policy-denied observations perform no sink work. Private-key material and content exceeding the detector bound create one content-free `SensitiveItemSkipped` audit and no clip, digest, blob, preview, or FTS row.

The agent remains an explicit diagnostic command-line executable. This phase does not enable auto-start, claim reliable private-browser detection, or claim comprehensive secret classification.

## 2. Commits

```text
7d4b1e5 docs: specify privacy admission foundation
c946926 docs: plan privacy admission foundation
0dda0aa feat: inspect clipboard privacy controls
dc263a3 feat: add privacy admission policy
a5a94f2 feat: enforce privacy before persistence
ac59102 feat: integrate agent privacy policy
```

Documentation and this evidence report are committed separately after verification.

## 3. Official Windows behavior used

The implementation follows Microsoft clipboard contracts:

- `ExcludeClipboardContentFromMonitorProcessing` prevents monitor processing, clipboard history, and cloud synchronization;
- `CanIncludeInClipboardHistory` DWORD `0` excludes the item from local clipboard history;
- `CanUploadToCloudClipboard` DWORD `0` controls cloud upload but does not exclude local history;
- registered clipboard format IDs are runtime values and are not persisted;
- `GetClipboardOwner` may return no owner even while clipboard data remains available;
- `GetWindowThreadProcessId`, `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, and `QueryFullProcessImageNameW` are used only to obtain a bounded executable image path.

Primary references:

- <https://learn.microsoft.com/windows/win32/dataxchg/clipboard-formats>
- <https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-registerclipboardformatw>
- <https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getclipboardowner>
- <https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid>
- <https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew>

## 4. Clipboard control enforcement

`pastral-clipboard-win` now registers and inspects the exact names:

```text
ExcludeClipboardContentFromMonitorProcessing
CanIncludeInClipboardHistory
CanUploadToCloudClipboard
```

Implemented semantics:

| Signal | Result |
|---|---|
| Exclude-monitor format present | Hard deny |
| Include-history DWORD `0` | Hard deny |
| Include-history DWORD `1` | Continue to Pastral policy |
| Cloud-upload DWORD `0` | Local-only hint; not a local Pastral deny |
| Available control with invalid length/value | Fail closed as hard deny |
| Control absent | No decision from that control |

The DWORD parser accepts exactly four bytes and only values `0` or `1`.

Runtime ordering was reviewed and corrected during the slice: hard deny now returns immediately after control inspection, before clipboard-owner observation. This avoids collecting extra metadata after a source-owned exclusion signal.

## 5. Clipboard-owner observation

The reviewed unsafe boundary in `crates/clipboard-win/src/sys.rs` performs:

1. `GetClipboardOwner`;
2. `GetWindowThreadProcessId`;
3. `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`;
4. `QueryFullProcessImageNameW` into a maximum 32,768 UTF-16-unit buffer;
5. `CloseHandle` on every successful open path.

Absent, inaccessible, exited, or unresolvable owners become `ClipboardOwnerObservation::Unavailable`.

The full path and PID are ephemeral. `apps/agent` immediately reduces a valid path to its final executable basename. The full path, PID, window title, source text, and publisher information are not stored, indexed, logged, or printed.

No additional unsafe module was introduced.

## 6. Source admission policy

`pastral-agent-core` now provides:

- `SourceConfidence`;
- `SourceObservation`;
- `SourceAdmissionPolicy`;
- `SourceAdmissionDecision`.

Rules:

- unresolved source can be denied fail closed;
- executable matching is exact and case-insensitive;
- entries are trimmed, normalized to lowercase, sorted, and deduplicated;
- empty names, paths, drive-qualified names, slash/backslash entries, colon entries, and names exceeding 260 UTF-8 bytes are rejected;
- no wildcard, regex, title, package-family, or publisher matching is implied.

Default policy:

```text
version=1
deny_unresolved_source=true
deny_process=1password.exe
deny_process=bitwarden.exe
deny_process=keepass.exe
deny_process=keepassxc.exe
```

This baseline is conservative and intentionally not described as exhaustive password-manager detection.

## 7. High-confidence sensitive detector

The detector scans at most 1 MiB of decoded text.

Detected envelope classes:

- PKCS#8 private key;
- encrypted PKCS#8 private key;
- RSA private key;
- EC private key;
- DSA private key;
- OpenSSH private key;
- PGP private key block.

Outcomes:

```text
PrivateKeyMaterial
DetectorLimitExceeded
```

Text above the bound is skipped rather than partially classified and stored.

Generic tokens, JWTs, cookies, connection strings, OTPs, recovery codes, card-like values, and password assignments are not hard-skipped in this phase because reliable context/validation is not implemented and destructive false positives would be unacceptable.

Detector marker literals are assembled from compile-time pieces so the repository source-policy secret scanner remains unchanged. No broad source-policy exemption was added.

## 8. Audit-only sensitive skip

`StorageCaptureSink` runs the detector before:

- event/representation ID allocation for an ordinary clip;
- SHA-256 digest calculation;
- representation creation;
- blob creation;
- search projection creation;
- FTS insertion.

For sensitive or detector-over-limit text it inserts only:

```text
CaptureAuditKind::SensitiveItemSkipped
capture_order=None
active profile ID
AuditResultCode::Skipped
observation time
```

Verified results:

- sink outcome is `SensitiveSkipped`;
- audit count increases by one per skip;
- search returns no result;
- storage integrity remains green;
- coordinator marks the transient sequence handled;
- no ordinary clip event is returned.

Source-owned hard deny remains stricter: no sink call and no audit row.

## 9. Strict privacy configuration

`PrivacyPolicyConfig::load_or_create` owns `privacy-policy.txt`.

Creation behavior:

- creates the data root when absent;
- writes a create-new staging file;
- flushes and `sync_all`s;
- publishes by rename;
- handles concurrent final-file appearance by parsing the final file;
- removes staging files best effort.

Parsing behavior:

- version must be exactly `1`;
- `deny_unresolved_source` is required exactly once and must be `true` or `false`;
- each `deny_process` must pass basename validation;
- unknown fields, duplicate required fields, missing required fields, invalid values, empty lines, unsupported versions, and path-like process entries fail closed;
- an invalid existing file is not overwritten or repaired silently.

Custom exact names are normalized case-insensitively and deduplicated deterministically.

## 10. Coordinator outcomes

New source errors:

```text
HardDenied
PolicyDenied
```

New sink outcome:

```text
SensitiveSkipped
```

New public capture outcomes:

```text
capture-outcome=hard-denied
capture-outcome=policy-denied
capture-outcome=sensitive-skipped
```

Hard deny and policy deny perform no sink call. Sensitive skip performs one audit-only sink operation. All three are terminal for that transient sequence. Storage failure remains retryable for the same sequence.

## 11. TDD evidence

### 11.1 Clipboard-control RED

Tests were written before types/parser implementation. Compilation failed because these did not exist:

```text
ClipboardHistoryControls
parse_control_dword
ClipboardError::HistoryControlInvalid
```

After implementation, history-control tests passed and the complete clipboard suite increased from 17 to 22 tests.

### 11.2 Core-policy RED

Policy/coordinator tests initially failed because these did not exist:

```text
SourceAdmissionPolicy
SourceObservation
SourceAdmissionDecision
SensitiveClass
CaptureSinkOutcome
CaptureOutcome::HardDenied
CaptureOutcome::PolicyDenied
CaptureOutcome::SensitiveSkipped
```

After implementation, agent-core increased from 8 to 18 tests.

### 11.3 Audit-only RED

Adapter test compilation failed because `Storage::capture_audit_event_count` did not exist. After implementation, private-key and detector-over-limit fixtures produced audit-only outcomes.

### 11.4 Source-admission RED

Pure adapter tests failed because `evaluate_source_admission` did not exist. After implementation they verified hard-deny precedence, unresolved-source denial, exact denied basename matching, and an allowed unlisted basename.

### 11.5 Privacy-config RED

Config tests failed because `PrivacyPolicyConfig` did not exist. After implementation, default creation, custom normalization, and malformed fail-closed behavior passed.

## 12. Test results

Fresh workspace result:

| Package | Tests | Result |
|---|---:|---|
| `pastral-agent` | 14 | PASS |
| `pastral-agent-core` | 18 | PASS |
| `pastral-clipboard-win` | 22 | PASS |
| `pastral-domain` | 26 | PASS |
| `pastral-storage` | 33 | PASS |
| **Total** | **113** | **PASS** |

No failures or ignored tests were reported.

New coverage includes:

- absent/history/cloud control parsing;
- malformed DWORD rejection;
- owner-observation value types;
- unresolved/exact source admission;
- denylist validation and deduplication;
- seven private-key envelopes;
- detector bound;
- hard/policy deny no-sink behavior;
- sensitive terminal outcome;
- audit-only persistence;
- strict privacy config create/reopen/custom/malformed cases;
- health-check privacy validation.

## 13. Real health-check evidence

A disposable root produced:

```text
agent-health=ok
privacy-policy=ok
storage-schema=1
sqlite-integrity=ok
fts-integrity=ok
metadata-integrity=ok
search-mapping-integrity=ok
EXIT_CODE=0
```

Generated policy:

```text
version=1
deny_unresolved_source=true
deny_process=1password.exe
deny_process=bitwarden.exe
deny_process=keepass.exe
deny_process=keepassxc.exe
```

The temporary root was removed after evidence capture.

Automated verification still invokes only:

```text
pastral-agent.exe health-check --data-root <temporary-root>
```

It does not invoke `capture-current` or `listen` and therefore does not read the user's clipboard.

## 14. Aggregate verification

Passed:

```powershell
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
.\eng\verify-agent.ps1 -Mode All
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
```

Agent:

```text
Agent static policy: PASS
Agent Debug and Release builds: PASS
Agent health-check smoke: PASS
```

Manager regression:

```text
Native manager Debug and Release builds: PASS
Manager UI Automation History navigation, filtering, selection, and no-results states: PASS
Native manager runtime smoke: PASS
```

Observed manager smoke HWND: `6951974`.

The manager remains disconnected and no direct storage coupling was introduced.

## 15. Dependency and source policy

Verified boundaries:

- `pastral-agent-core`, domain, and storage remain Windows-binding free;
- agent and clipboard-win use only pinned `windows-sys 0.61.2` / `windows-link 0.2.1`;
- no async runtime, serialization framework, network client, IPC runtime, logger, alternate database, or high-level Windows binding was added;
- unsafe remains restricted to the exact clipboard `sys.rs` boundary;
- secret/key source scanning remains enabled;
- no build output, local launcher, key file, WAL activation, SQLite extension loading, or database attachment was introduced.

## 16. Explicit exclusions and residual risks

Not implemented or not claimed:

- reliable Incognito/InPrivate/private-window detection;
- signed publisher, package-family, AppContainer, browser-profile, or extension-assisted source verification;
- exhaustive password-manager detection;
- generic token/JWT/cookie/OTP/recovery-code/card/connection-string/password detectors;
- encrypted sensitive opt-in;
- sensitive-audit retention cleanup;
- durable source attribution;
- self-origin replay markers;
- live IPC and manager state;
- default auto-start/resident operation;
- graceful control channel/process supervision;
- COM/OLE and rich clipboard formats;
- Quick Paste, overlay, or replay;
- real-application clipboard admission matrix;
- hostile clipboard-provider fuzzing;
- AV/EDR, RDP, multi-session, lock/unlock, low-disk, power-loss, and performance evidence;
- hosted CI run evidence;
- packaging, signing, installer, and update.

Because actual clipboard-reading commands were intentionally not run, this report does not claim real-user interoperability evidence for owner/control inspection across third-party applications.

## 17. Next recommended slice

The next authority slice is **bounded local IPC foundation** after completing the ADR 0018 frame/runtime prototype evidence.

Minimum goals:

1. select and measure the resident serialization/runtime approach;
2. implement authenticated same-user named-pipe control framing with strict bounds;
3. expose content-free health/status and privacy-safe history snapshots;
4. keep the manager unable to open SQLite/blob storage directly;
5. add protocol mismatch, timeout, cancellation, backpressure, and disconnect states;
6. wire manager Release to real disconnected/connected snapshots without enabling paste/replay yet.

Auto-start should remain disabled until IPC, privacy-policy diagnostics, process supervision, and real-application admission tests are green.

## 18. Completion statement

Phase 3C proves a fail-closed privacy-admission foundation for Microsoft source-owned clipboard controls, owner-process exact deny policy, and a narrow high-confidence private-key class.

It does not prove comprehensive privacy classification or authorize unattended daily capture.