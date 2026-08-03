# Glossary

## CaptureAuditEvent

A durable content-free policy, failure, integrity, or health outcome. It is distinct from a successfully captured clip. Source-owned hard-deny observations create no durable audit event.

## Capture order

A strictly increasing installation-local sequence assigned by the serialized storage owner in the same transaction that commits a durable clip or ordered audit record. It preserves local capture order across agent restarts and wall-clock changes, but is not a public ID, timestamp, content property, or authorization value.

## Capture-critical path

The bounded work executed from clipboard notification until Pastral has safely copied the required immediate representations or decided to skip them. It excludes OCR, syntax highlighting, semantic indexing, large preview generation, and other enrichment.

## ClipboardObservation

A transient attempt to inspect the current clipboard after a notification or explicit recovery action. It carries sequence/source/policy evidence for capture orchestration but is not a durable copy event and does not remain in ordinary history.

## ClipEvent

One successful durable logical copy occurrence identified by Pastral. It includes stable opaque identity, timestamp/order, privacy-filtered source context, active profile/policy version, and one or more `ClipRepresentation` records. Multiple formats captured from one current clipboard state belong to the same event. A zero-representation outcome is an audit/failure record, not a `ClipEvent`.

## ClipRepresentation

One immutable preserved or referenced format within a `ClipEvent`, including durable format identity, adapter/version, medium evidence, byte length, content/blob reference, digest policy, protection domain, fidelity, safety state, and capture notes. Standard formats persist defined IDs; registered formats persist exact names rather than runtime-local numeric IDs.

## Copy occurrence

A meaningful successful capture occurrence with its own event identity, time/order, source evidence, and profile context. Identical ordinary payload bytes may share blob storage but never collapse event history. Pastral does not claim that every intermediate state is observable when the clipboard changes faster than it can be acquired.

## Digest suite

A versioned algorithm and exact byte-domain definition used for ordinary blob identity. The initial suite is `sha256-raw-v1`: SHA-256 over the exact logical raw representation payload bytes before optional storage compression/container encoding. It is not a user-facing entity ID, excludes normalized text and format descriptors, and is not persisted for Private/sensitive plaintext by default.

## DerivedRepresentation

A new representation produced from an original or another derived representation. It records parent clip/representation, transformation identifier and version, parameters, creation time, deterministic output hash where appropriate, and fidelity notes.

## Fidelity

An honest statement about preservation/replay quality:

- `FullFidelity`: all representations required by the tested scenario were safely preserved and replayed byte-for-byte or through an equivalent required medium.
- `CommonFormatsPreserved`: common interoperable formats were preserved, but private/ephemeral formats were not.
- `FallbackOnly`: only a fallback such as Unicode/plain text is usable.
- `ReferenceOnly`: Pastral stored a reference that may no longer resolve, such as an unavailable external resource.
- `Unavailable`: advertised data could not be retrieved.
- `UnsafeOrUnsupported`: capture or replay was intentionally refused.

Fidelity is scoped to known formats and tested destinations; it is not a universal guarantee.

## Hard deny

A capture prohibition that ordinary user settings and convenience rules cannot override. Clipboard-owner exclusion signals and core safety invariants use hard-deny semantics.

## Immutable original

Captured representation bytes and immutable capture-core metadata that transformations or later user organization cannot modify. Tags, notes, pin state, collection membership, current placement, and paste statistics are separate mutable records. Deletion and retention may remove the original according to explicit policy, but an action cannot rewrite it in place.

## Opaque domain ID

A UUIDv4 generated from a cryptographically secure RNG, stored as 16 bytes and encoded as canonical lower-case hyphenated text at textual boundaries. It carries no timestamp, source, database row number, content hash, process ID, clipboard sequence, registered-format number, authorization, or ordering semantics.

## PasteTransaction

A bounded operation that reconstructs a data object, publishes safe formats to the clipboard, optionally sends paste to an explicitly selected destination, monitors result heuristics, retains object lifetime as required, and records metadata-only outcome.

## Profile

A separate clipboard context defining retention, cleanup target, capture allow/deny policy, sensitive-data behavior, indexing, transformations, paste defaults, visual accent, encryption, and optional switching rules. Every clip retains its immutable capture-profile snapshot even if later organization changes.

## Protection domain

A storage/privacy boundary controlling encryption, digest, deduplication, indexing, export, and key policy. Ordinary profiles may share a compatible ordinary deduplication domain; Private/sensitive content does not cross or deduplicate with ordinary domains by default.

## Quick Paste

The explicitly invoked keyboard-first picker used to search, inspect, select a representation/paste mode, and paste or copy a history item.

## Representation set

The complete collection of formats and media advertised for one logical clipboard event.

## Rule

A versioned deterministic mapping from explicit match conditions and scope to one or more allowed actions. Rules include priority, enabled state, origin, audit history, match count, last result, and rollback information.

## RuleMatch

A replayable explanation of which rule predicates matched an event or paste context, which higher-priority privacy rules applied, and why an action was selected or rejected.

## SensitiveItemSkipped

A hidden `CaptureAuditEvent` created by default when high-confidence sensitive-content policy suppresses storage. It contains a broad policy/detector class, active profile, coarse timestamp, and no preview, value, content hash, OCR text, token fragment, title, path, domain, or reconstructable metadata. Default retention is 24 hours and can be disabled or shortened.

## SourceContext

Privacy-filtered source evidence with explicit provenance and confidence. Clipboard-owner evidence, foreground-window snapshot, package/process identity, explicit integration, user assignment, and heuristic inference are distinct. Unknown or conflicting source remains unknown/low confidence; arbitrary window titles are not treated as domain/project proof.

## Stable clipboard format identity

For standard formats, the documented fixed `CF_*` ID. For registered formats, the exact registered name. Numeric values returned by `RegisterClipboardFormat` are runtime-local and never durable format identities.

## Time semantics

Persisted event times use UTC Unix microseconds for civil time. Durable local ordering uses installation-local `capture_order`; the per-agent observation ordinal is origin evidence only and can restart with the process. Monotonic high-resolution clocks are used only for durations, retries, deadlines, and benchmarks. Relative-date queries resolve in the user's current timezone and compile to explicit UTC ranges.

## Transformation

A pure or isolated operation that creates a derived representation, such as plain-text extraction, URL tracking removal, image-to-PNG, OCR, filename extraction, or code-block formatting.

## Worker

The on-demand, resource-bounded process that handles expensive or hostile parsing/enrichment. It has no network access by default and cannot own the primary database.
