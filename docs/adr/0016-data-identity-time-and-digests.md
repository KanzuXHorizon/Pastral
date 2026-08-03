# ADR 0016: Durable data identity, time, and digest semantics

**Status:** Accepted
**Date:** 2026-08-04

## Context

Clipboard sequence numbers, Windows process IDs, SQLite row IDs, registered clipboard numeric IDs, and wall-clock timestamps are not suitable as stable cross-process/public identities. Pastral also needs deterministic ordering, content-addressed ordinary blobs, privacy-preserving sensitive storage, migration stability, and protocol-safe identifiers.

The Phase 0 model did not define whether failed/skipped captures were clips, how IDs are generated, which clock is authoritative, or which digest/protection domain controls deduplication.

## Decision

### Record types

- `ClipboardObservation` is transient notification/current-state evidence.
- `ClipEvent` is a durable successful capture and contains at least one `ClipRepresentation`.
- `CaptureAuditEvent` is a content-free durable policy/failure/integrity record where policy permits.
- Source-owned hard-deny observations create neither `ClipEvent` nor durable `CaptureAuditEvent`.

### Public IDs

- Every durable domain entity exposed through storage, IPC, export, or logs uses an opaque UUIDv4 generated from a cryptographically secure RNG (122 random bits after required version/variant bits).
- IDs are stored canonically as 16 bytes internally and lower-case hyphenated UUID text only at textual boundaries.
- UUIDs carry no ordering, time, namespace, content, source, authorization, or trust semantics.
- SQLite integer row IDs may exist solely as local query/index implementation details and never become protocol/export identities.
- Windows PIDs, HWNDs, clipboard sequence numbers, runtime clipboard format numbers, hashes, filenames, and timestamps are not entity IDs.

### Time

- Persist wall-clock timestamps as signed UTC Unix microseconds with a documented supported range.
- Record monotonic high-resolution ticks only transiently for durations, retries, deadlines, and benchmark trace correlation.
- Preserve a per-agent `observation_ordinal` as transient/origin evidence, but do not use it as the durable cross-restart order.
- The serialized storage owner assigns a strictly increasing installation-local `capture_order` in the same transaction that commits each durable `ClipEvent` or ordered audit record. It is never a public/entity ID and carries no wall-time meaning.
- Resolve user query terms such as `today` and `yesterday` using the user's timezone at query time, then compile to explicit UTC ranges.
- Store original UTC times; timezone/display changes do not rewrite history.

### Ordinary digests and deduplication

- Ordinary raw payload blobs use digest suite `sha256-raw-v1`: SHA-256 over the exact logical raw representation payload bytes before any optional storage compression/container encoding, without text normalization or format-descriptor bytes.
- Blob metadata separately binds raw byte length, representation identity, protection domain, format descriptor, and physical storage-encoding version. Reads decode any storage container and verify the resulting raw bytes against the digest.
- Deduplication occurs only within the same ordinary protection domain and only after byte length plus full digest match. Implementations must tolerate theoretical digest collision by verifying existing blob metadata and, when ambiguity is detected, store separately rather than alias.
- A derived representation computes its own digest over its own exact output bytes and retains transformation provenance.
- Digest suite/version is persisted so future migration can coexist rather than silently reinterpret old hashes.

### Sensitive and Private protection domains

- Sensitive/Private plaintext is not hashed for storage identity, duplicate detection, search, or filenames by default.
- Encrypted payloads use random blob IDs and per-payload or bounded-group DEKs according to the encryption design.
- No cross-profile or cross-protection-domain deduplication is performed.
- Ciphertext/authentication metadata may use implementation integrity checks that do not expose plaintext equality; these are not public content IDs.

### Clipboard format identity

- Standard formats use their documented fixed standard IDs.
- Registered formats persist the exact registered name as the stable identity and obtain a runtime numeric ID through registration on each process/system as needed.
- Runtime registered numeric IDs are transient diagnostic values only.

## Consequences

Positive:

- stable IPC/export/migration identity independent of database layout and Windows runtime identifiers;
- deterministic installation-local ordering across process restarts and wall-clock changes;
- ordinary byte deduplication has explicit semantics;
- Private/sensitive content avoids plaintext equality leakage;
- registered-format replay survives process restart and runtime ID changes.

Costs:

- additional opaque IDs and indexes;
- no chronological sorting from IDs alone;
- restoring/merging another vault must preserve original time/provenance while assigning new local `capture_order` values unless performing a verified whole-vault restore;
- Private/sensitive duplicate storage may consume more disk;
- future digest-suite migration requires explicit coexistence/reconciliation.

## Alternatives considered

- **ULID/UUIDv7/time-ordered ID:** rejected for the baseline because embedded time can leak timing and IDs still require separate order semantics under clock changes.
- **SQLite row ID as public ID:** rejected due to database coupling, enumeration, import/merge conflicts, and migration constraints.
- **BLAKE3 default:** technically credible, but SHA-256 has sufficient platform/library availability and interoperability for a non-adversarial content-address key; performance can be revisited with measurements.
- **Plaintext keyed hash for sensitive dedupe:** deferred because key compromise or repeated-query behavior can expose equality and the storage saving is not required for MVP correctness.
- **Normalized text hash:** rejected for original identity because normalization would merge byte-distinct representations and violate immutable-original semantics.

## Review triggers

- measured SHA-256 cost threatens capture/storage budgets;
- cross-device encrypted sync or portable backups require mergeable IDs;
- a private-profile storage design accepts equality leakage for a documented benefit;
- digest collision/corruption handling proves inadequate;
- timestamp range/precision or `capture_order` exhaustion/restore semantics conflict with SQLite or IPC format;
- UUIDv4 interoperability, collision handling, or storage representation proves inadequate.
