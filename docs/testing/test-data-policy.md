# Test data policy

## 1. Rule

Tests, screenshots, logs, benchmark datasets, fuzz corpora, crash artifacts, CI uploads, and bug reports must use synthetic or explicitly licensed public data. Do not use a developer's real clipboard history, credentials, private documents, customer data, or personal communications.

## 2. Allowed data

- generated text, code, HTML, RTF, URLs, images, and file trees;
- public-domain or permissively licensed samples with attribution when required;
- synthetic names, domains using `.example`, reserved IP ranges, and non-routable endpoints;
- fake tokens/keys intentionally marked invalid;
- deterministic seeded random payloads;
- malformed fixtures created for parser tests;
- minimal metadata-only traces from isolated fixtures.

## 3. Prohibited data

- real passwords, OTPs, API keys, cookies, private keys, recovery codes, connection strings, card/account numbers, or auth headers;
- copied source from private repositories without explicit redistribution rights;
- screenshots containing real user windows, notifications, paths, email, identity, or browser tabs;
- production databases or exported histories;
- real private-browser sessions;
- customer/support files unless a separate authorized secure handling process exists;
- copyrighted documents/images without test redistribution rights.

## 4. Synthetic secret corpus

Secret detectors require realistic structure without active value.

Rules:

- use reserved prefixes and invalid checksums/signatures where possible;
- use `.example` hosts and disabled accounts;
- never generate a token through a real provider;
- mark fixtures with metadata such as `synthetic=true`, detector class, expected confidence, and expected policy;
- include near misses and ordinary-content false-positive cases;
- mutation/fuzz generation must preserve non-operational status;
- a CI secret scanner allowlist is limited to exact fixture paths and patterns, never repository-wide exclusions.

Expected assertions:

- high-confidence synthetic secret payload absent from ordinary DB, blobs, FTS, logs, overlay, diagnostics, and export;
- default hidden `SensitiveItemSkipped` contains only broad detector/policy class, active fixture profile, coarse timestamp, and 24-hour retention metadata—no value/hash/snippet/title/path/domain/size/structure;
- source-owned hard-deny fixture creates no durable clip or audit row;
- uncertain/near-miss behavior matches explicit detector policy;
- detector output includes version/class/confidence but not secret text in logs.

## 5. Clipboard format fixtures

Each fixture directory contains:

- source generator definition;
- expected stable format identity (standard ID or exact registered name), source/runtime numeric ID only as transient evidence, and order where relevant;
- expected exact bytes or `sha256-raw-v1` digest stored safely for ordinary fixtures; Private/sensitive plaintext fixtures do not persist a plaintext digest in simulated product state;
- expected fidelity state;
- expected capture/replay behavior;
- size and parser limits;
- license/source metadata;
- sensitivity classification;
- malformed-case rationale.

Binary fixtures should be minimal. Large benchmark fixtures are generated deterministically during tests or stored outside Git with hashes and retrieval instructions from an approved source.

## 6. Image and OCR fixtures

- create synthetic screenshots/documents with non-personal text;
- include transparent, indexed, grayscale, HDR/large-dimension metadata, invalid headers, truncation, and decompression-ratio cases;
- do not embed EXIF location, real device identity, or private thumbnails;
- OCR expected text is stored separately and may include multiple languages using original/generated content with appropriate rights;
- images remain encoded originals for fidelity tests; derived preview/OCR artifacts are disposable.

## 7. File-list/path fixtures

Use a disposable isolated tree under the test run root:

- Unicode and long names;
- spaces and reserved-character edge cases allowed by Windows;
- directories, empty files, large sparse files where safe;
- broken references;
- reparse/symlink cases only in isolated privileged-capable tests;
- traversal strings and device/network paths represented as strings without accessing real resources.

Tests must not enumerate or copy outside the fixture root unless the test explicitly verifies rejection using handles/path checks.

## 8. Database and migration fixtures

- generated from committed schema builders/migrations;
- contain synthetic clip IDs/content;
- one fixture for every supported prior schema version;
- corruption variants operate on disposable copies;
- encrypted fixtures use test-only keys and cannot be confused with production key formats/locations;
- Private/sensitive equal-plaintext fixtures verify random blob identity and no cross-domain plaintext deduplication;
- encrypted fixtures include whole-message and chunked authentication cases for truncation, reorder, duplicate, cross-object splice, wrong count, and no plaintext release before verification;
- deletion-remnant fixtures inspect disposable SQLite/FTS/freelist/journal/WAL copies according to the selected policy without using real user data;
- fixture creation is deterministic enough to reproduce result hashes where expected.

Do not commit databases copied from a user profile.

## 9. Logs, screenshots, and artifacts

- CI artifacts are treated as potentially public to repository collaborators.
- Release/debug logs use content-free structured fields.
- screenshot tests render synthetic clips and identities; protected-content tests verify hidden payloads are absent from view models/UI Automation even when window capture exclusion is unavailable.
- failure reporters scrub window titles, paths, usernames, domains, profile names, package identities, and clip IDs unless they are fixture values.
- crash dumps are not uploaded automatically; development dumps use synthetic fixture processes and limited retention.
- artifact retention is bounded and documented by workflow.

## 10. Fuzzing

- fuzz corpora begin with minimal synthetic valid/malformed seeds;
- crashes are minimized before storage;
- corpus files are reviewed for accidental secrets/license concerns before commit;
- fuzz output directories are ignored;
- cloud fuzzing, if adopted, receives no user data and uses dedicated test keys;
- reports include hash, target, build, sanitizer/configuration, and reproduction command without content-bearing environment state.

## 11. Manual compatibility testing

When a real third-party application must be tested:

- use dedicated Windows test accounts/logon sessions for cross-user/session IPC cases and a separate standard-user/elevated fixture pair for UIPI fallback;
- copy only synthetic fixture content;
- close unrelated private apps/documents;
- disable or isolate real password managers/private browsing;
- clear the disposable Pastral data store after the run;
- inspect logs/screenshots before sharing;
- record application and Windows version, not personal account details.

## 12. Data incident handling

If real sensitive/private data enters a repository or artifact:

1. stop further distribution and workflow uploads;
2. treat Git history/cache/artifacts/backups as additional copies;
3. rotate affected credentials immediately when applicable;
4. remove/revoke through the hosting platform's incident procedure;
5. document scope without reproducing the value;
6. add a regression control such as a secret scan or safer fixture;
7. follow `docs/security/incident-response.md`.
