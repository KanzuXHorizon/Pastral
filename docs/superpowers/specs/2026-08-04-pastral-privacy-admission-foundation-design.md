# Pastral Privacy Admission Foundation Design

**Date:** 2026-08-04

**Phase:** 3C

**Status:** Approved by continuation directive and the existing privacy-model precedence

## 1. Goal

Add a fail-closed privacy admission boundary before the Phase 3B diagnostic agent can persist ordinary Unicode text.

This slice must enforce source-owned hard-deny clipboard formats, observe the clipboard owner process without persisting source paths, apply a configurable exact process denylist, and skip high-confidence private-key material before any digest, blob, search projection, or preview is created.

The agent remains diagnostic/manual. This phase does not enable auto-start or claim complete password-manager/private-browser coverage.

## 2. Official Windows authority

Microsoft documents these registered clipboard formats:

- `ExcludeClipboardContentFromMonitorProcessing`: any data in this format prevents monitor processing, clipboard history, and cloud synchronization;
- `CanIncludeInClipboardHistory`: serialized DWORD `0` prevents local clipboard history, while `1` explicitly permits it;
- `CanUploadToCloudClipboard`: serialized DWORD `0` prevents cloud synchronization but does not prevent local history.

Microsoft also documents that `GetClipboardOwner` may return no owner even while clipboard data remains available. Therefore owner observation is confidence-bearing evidence, not guaranteed identity.

Primary references:

- Microsoft Learn, Clipboard Formats: <https://learn.microsoft.com/windows/win32/dataxchg/clipboard-formats>
- Microsoft Learn, RegisterClipboardFormatW: <https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-registerclipboardformatw>
- Microsoft Learn, GetClipboardOwner: <https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getclipboardowner>
- Microsoft Learn, GetWindowThreadProcessId: <https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid>
- Microsoft Learn, QueryFullProcessImageNameW: <https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew>

## 3. Admission precedence

For the implemented ordinary Unicode-text path:

1. source-owned hard-deny clipboard formats;
2. source observation availability and exact process deny policy;
3. bounded Unicode acquisition/validation;
4. high-confidence secret classification;
5. ordinary storage and indexing.

No lower-precedence setting can override a source-owned hard deny.

## 4. Source-owned clipboard controls

Add `ClipboardHistoryControls` to `pastral-clipboard-win`:

```rust
pub struct ClipboardHistoryControls {
    exclude_from_monitor_processing: bool,
    include_in_history: Option<bool>,
    upload_to_cloud: Option<bool>,
}
```

The session registers the exact Microsoft format names and inspects their availability while the clipboard is open.

Rules:

- `ExcludeClipboardContentFromMonitorProcessing` present: hard deny without reading payload and without durable audit.
- `CanIncludeInClipboardHistory` present with DWORD `0`: hard deny without reading payload and without durable audit.
- `CanIncludeInClipboardHistory` present with DWORD `1`: no hard-deny signal; other Pastral policy still applies.
- `CanUploadToCloudClipboard` DWORD `0`: retain only as an ephemeral local-only hint. It does not deny local Pastral history because Microsoft documents that it does not affect local clipboard history.
- malformed DWORD data, unsupported values, or inability to validate an available control: fail closed for that observation.

Registered runtime IDs are never persisted.

## 5. Clipboard owner observation

Add a bounded `ClipboardOwnerObservation`:

```rust
pub enum ClipboardOwnerObservation {
    Unavailable,
    ProcessImage(PathBuf),
}
```

The reviewed Win32 `sys.rs` boundary performs:

1. `GetClipboardOwner`;
2. `GetWindowThreadProcessId`;
3. `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`;
4. `QueryFullProcessImageNameW` into a bounded UTF-16 buffer;
5. `CloseHandle` on every opened-handle path.

No process is injected, suspended, modified, or inspected beyond the executable image path. Product policy immediately reduces the observation to the final executable basename. Full paths and process IDs are not stored, indexed, logged, or emitted by the diagnostic executable.

If the owner is absent, inaccessible, exits during observation, or has no valid basename, the observation is `Unavailable`.

## 6. Source admission policy

Add Windows-binding-free types to `pastral-agent-core`:

```rust
pub enum SourceConfidence {
    Unavailable,
    ProcessImage,
}

pub struct SourceObservation {
    executable_name: Option<String>,
    confidence: SourceConfidence,
}

pub struct SourceAdmissionPolicy {
    deny_unresolved_source: bool,
    denied_executable_names: Vec<String>,
}

pub enum SourceAdmissionDecision {
    Allow,
    DenyUnresolved,
    DenyExecutable,
}
```

Executable matching is exact and case-insensitive after basename extraction. Paths, wildcard patterns, regular expressions, publisher claims, and titles are excluded from this slice.

Default diagnostic policy:

```text
version=1
deny_unresolved_source=true
deny_process=1password.exe
deny_process=bitwarden.exe
deny_process=keepass.exe
deny_process=keepassxc.exe
```

The list is a conservative baseline, not exhaustive password-manager detection. Users/developers may add exact basenames. Invalid configuration fails closed and is not silently replaced.

## 7. Secret admission

Add a bounded high-confidence detector to `pastral-agent-core`.

Implemented secret class:

```rust
pub enum SensitiveClass {
    PrivateKeyMaterial,
    DetectorLimitExceeded,
}
```

The detector scans at most 1 MiB of decoded text. It detects exact private-key envelope markers:

- PKCS#8 private key;
- encrypted PKCS#8 private key;
- RSA/EC/DSA private key;
- OpenSSH private key;
- PGP private key block.

If decoded text exceeds the detector bound, the slice skips it as `DetectorLimitExceeded` rather than persisting content that was not fully classified.

This phase intentionally does not hard-skip generic tokens, JWTs, credit-card-like values, OTPs, connection strings, or `password=` assignments because reliable context/validation is not yet implemented and false-positive destruction would be unacceptable.

## 8. Coordinator and sink outcomes

Extend source outcomes:

```rust
CaptureSourceError::HardDenied
CaptureSourceError::PolicyDenied
```

Extend coordinator outcomes:

```rust
CaptureOutcome::HardDenied
CaptureOutcome::PolicyDenied
CaptureOutcome::SensitiveSkipped
```

Hard-deny and policy-deny outcomes mark the transient sequence handled and perform no payload sink call.

Change the sink port to return:

```rust
pub enum CaptureSinkOutcome {
    Stored(StoredCapture),
    SensitiveSkipped,
}
```

A storage failure remains retryable for the same transient sequence.

## 9. Sensitive skip persistence

`StorageCaptureSink` runs the detector before digest or representation creation.

For `SensitiveSkipped`:

- no clip event;
- no representation;
- no payload blob;
- no raw/plain/keyed hash;
- no FTS row;
- no preview or derived data;
- insert one content-free `CaptureAuditEvent` with:
  - `CaptureAuditKind::SensitiveItemSkipped`;
  - coarse/current UTC observation already supplied by the coordinator;
  - `capture_order=None`;
  - active profile ID;
  - `AuditResultCode::Skipped`.

Source-owned hard deny is stricter and inserts no audit event.

Add a bounded content-free storage audit-count query only for verification/health summaries. It must not expose content or source metadata.

## 10. Agent privacy configuration

Create `privacy-policy.txt` under the chosen agent data root.

Strict format:

```text
version=1
deny_unresolved_source=true
deny_process=<executable basename>
...
```

Rules:

- version must be exactly `1`;
- `deny_unresolved_source` must be `true` or `false`;
- process entries must be basenames only, nonempty, bounded to 260 UTF-8 bytes, and contain no slash/backslash/colon;
- matching is case-insensitive;
- duplicate entries are removed deterministically;
- unknown fields, malformed values, or missing required fields fail closed;
- first creation uses staging/create-new/sync/rename behavior;
- invalid existing configuration is never overwritten.

## 11. Automated-test safety

Automated tests continue to avoid the user's clipboard.

- Clipboard control DWORD parsing uses synthetic byte slices.
- Source-policy tests use synthetic basename observations.
- Coordinator tests use fake sources/sinks.
- Secret detector tests use synthetic fixtures.
- Storage sink tests use temporary roots and synthetic text.
- `verify-agent` smoke still runs only `health-check`.

Actual `capture-current` and `listen` remain explicit manual commands and are not invoked in CI or aggregate gates.

## 12. Verification gates

Phase 3C is complete only when:

- hard-deny controls are checked before Unicode payload capture;
- malformed available history controls fail closed;
- owner process observation uses bounded buffers and closes handles;
- unresolved owner is denied by default agent policy;
- default exact denylist and user additions work case-insensitively;
- private-key material and detector-over-limit content produce audit-only `SensitiveSkipped` outcomes;
- hard deny produces no sink/audit work;
- 91 existing tests remain green and new tests pass;
- dependency/source policy remain green;
- agent Debug/Release/health-check pass;
- manager Debug/Release/UI Automation regression passes;
- Git integrity and clean tree pass;
- hosted CI execution is not claimed unless observed.

## 13. Explicit exclusions

Not implemented in Phase 3C:

- reliable Incognito/InPrivate/private-window detection;
- publisher/signature/package-family verification;
- browser extension integration;
- OTP/recovery-code/token/cookie/connection-string/card detectors;
- user UI for source policy;
- retention/expiry cleanup for sensitive audit rows;
- source attribution persistence;
- per-source encrypted sensitive opt-in;
- self-origin replay markers;
- IPC, Quick Paste, overlay, replay, packaging, or auto-start.

## 14. Completion statement

Phase 3C proves a fail-closed admission foundation for source-owned deny signals, owner-process exact deny policy, and a narrow high-confidence secret class. It does not prove comprehensive privacy classification or authorize unattended resident capture.