# Privacy model

**Date:** 2026-08-03

## 1. Default posture

Pastral is local-first and network-silent by default:

- no account;
- no cloud sync;
- no remote AI;
- no content telemetry;
- no content-bearing logs;
- no automatic URL opening;
- no browser/IDE integration until separately enabled;
- no OCR or semantic indexing for sensitive/private content by default.

Core capture, search, organization, and paste continue to work offline.

## 2. Data categories

### Payload content

Text, HTML, RTF, image bytes, file lists, URLs, registered-format bytes, and derived representations. This is the most sensitive class.

### Source context

Process/package identity, window class, privacy-filtered title, domain, project/workspace, profile, monitor/session, and confidence. Source context may be sensitive even when payload is ordinary.

### Operational metadata

Timestamps, format identifiers, size buckets, fidelity, retention state, policy/rule IDs, match/result codes, and correlation IDs.

### Security material

DPAPI-wrapped root keys, encryption key identifiers, nonces, authenticated envelope metadata, IPC installation secret, package/signing identity.

### Diagnostics

Durations, counters, error codes, component versions, health state, schema version, and redacted environment information.

## 3. Collection minimization

- Capture only after an actual clipboard update event.
- Do not poll the clipboard.
- Do not read payload after hard-deny policy is known, where format ordering and Windows behavior permit.
- Store source title/path/domain only according to profile policy.
- Paste occurrence tracking is optional.
- Do not store destination document text or infer surrounding document content.
- Do not index data not needed for the enabled search features.
- Derived representations require an enabled action/module and retain provenance.

## 4. Capture exclusions

Policy precedence:

1. source-owned hard-deny clipboard formats;
2. core safety restrictions;
3. application/package/window/private-context deny policy;
4. sensitive-content policy;
5. profile allow/deny policy;
6. retention/indexing policy;
7. convenience transformations and learned rules.

Default exclusions:

- known password managers;
- reliably identified Incognito/InPrivate/Private contexts;
- user-defined denied processes/packages/window patterns;
- high-confidence secrets;
- fullscreen/screen-sharing contexts only when the user enables capture suppression; overlay suppression is broader by default.

`ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0` are hard-deny signals. Ordinary settings cannot override them.

## 5. Sensitive-content handling

Conservative detectors cover:

- passwords where context is strong;
- OTP and recovery codes;
- API/access tokens;
- private keys;
- authentication cookies;
- secret connection strings;
- credit-card-like values with validation and context.

Default high-confidence result:

- payload is not stored;
- no plaintext or keyed/plain hash is stored;
- no preview, snippet, OCR, FTS entry, semantic vector, duplicate relationship, thumbnail, or passive overlay content is created;
- optional `SensitiveItemSkipped` records broad detector class and policy outcome only, without reconstructable value.

Uncertain detection does not silently destroy data. It may mark content as potentially sensitive, hide previews, shorten retention, or ask through explicit settings depending on profile policy.

## 6. Opt-in sensitive retention

A user may intentionally enable encrypted retention with a narrow scope:

- profile;
- exact application/package;
- sensitive class;
- duration such as 15 seconds, 1 minute, until lock, or custom retention;
- view/paste unlock requirements.

The product does not offer ordinary unencrypted secret retention.

Encrypted sensitive clips:

- use authenticated encryption and non-equality-revealing blob identifiers;
- remain excluded from normal search indexes and previews;
- are not included in ordinary exports or diagnostic bundles;
- require explicit reveal/paste authorization according to profile settings;
- can be deleted immediately as a class;
- expire independently of ordinary 90-day retention.

## 7. Retention and quota

Default ordinary history:

- 90 days;
- 5 GB;
- pinned clips are exempt from automatic age/quota deletion but remain visible in storage reporting.

Cleanup order is policy-driven and explainable. Regenerable derived data and old unpinned items are removed before protected/pinned originals. Low disk can pause payload capture without blocking user copy.

Deletion semantics:

- remove metadata references transactionally;
- delete shared blobs only when no reference remains;
- delete encryption keys/envelopes for encrypted items;
- acknowledge that physical overwrite on SSDs, snapshots, backups, and filesystem journals cannot be guaranteed;
- expose backup/export copies as separate user responsibility.

## 8. Search and preview privacy

- Sensitive items do not enter FTS, typo index, OCR, semantic index, or suggestions.
- Hidden-preview policy applies consistently to cards, overlay, Quick Paste, manager, task switcher thumbnails, notifications, and screen-capture behavior where supported.
- Search snippets are generated from allowed indexed text only.
- Source/domain suggestions obey the active profile and privacy settings.
- Lock state removes decrypted content and protected previews from view models.

## 9. Logs, telemetry, and crash artifacts

Release logs use typed allowlisted fields. Clipboard payload fields do not exist in the normal logging API.

Allowed examples:

- operation/correlation ID;
- format numeric/name identifier;
- size bucket rather than exact size where privacy requires;
- duration and result code;
- component/schema/protocol version;
- redacted source class according to diagnostics level.

Disallowed:

- clipboard text/bytes;
- image pixels;
- secret fragments or masked values that still reveal structure;
- full window titles, URLs, file paths, usernames, project names by default;
- encryption keys, nonces paired with plaintext, IPC secrets;
- arbitrary exception object dumps containing payload buffers.

No content telemetry is introduced. Optional usage telemetry, if ever proposed, requires a separate design and must be off by default.

## 10. Network behavior

Core release network policy:

- no outbound sockets from agent/worker/manager/CLI during core operation;
- worker has no network capability by default;
- Store/OS package update traffic is outside Pastral content processing;
- optional future integrations use explicit permissions, destinations, content disclosure, and per-feature network indicators;
- network-silence tests run in release validation.

## 11. Backup, export, and import

- Ordinary export requires explicit user action, destination, scope, and format.
- Sensitive/private content is excluded unless separately unlocked and selected.
- Exports warn that destination files may be unencrypted and indexed/backed up by other software.
- Encrypted backup format, if added, uses an independent portable-key design rather than assuming DPAPI portability.
- Imports are untrusted, size-limited, schema-validated, previewed, and never execute code.

## 12. User controls

Users can:

- pause for a duration, until restart, until profile change, or manually;
- deny the current app;
- delete one clip, a profile, a source, a time range, all sensitive history, or all history;
- inspect data location, storage usage, retention, indexing, and encryption state;
- see why capture or an action was suppressed/applied without exposing content;
- export settings/rules separately from payload history;
- reset sections and disable learned behavior.

## 13. Privacy review triggers

Re-review before adding OCR, semantic search, browser/IDE integration, cloud/device sync, telemetry, crash upload, shared/team features, scripting/plugins, remote APIs, private metadata indexing, or a new export format.
