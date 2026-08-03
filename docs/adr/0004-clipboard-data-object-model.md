# ADR 0004: Immutable multi-representation clipboard event model

**Status:** Accepted
**Date:** 2026-08-03
**Amended:** 2026-08-04 — separate notification observations/audit outcomes from successful clips, persist registered-format names, and require reviewed adapters after the Phase 0 adversarial audit.

## Context

One copy operation can expose Unicode text, HTML, RTF, images, URLs, file lists, Shell/OLE metadata, and application-private formats simultaneously. Creating one history item per format loses their relationship; storing only a preferred format loses fidelity.

## Decision

Model each successfully captured current clipboard state as one immutable `ClipEvent` containing one or more `ClipRepresentation` records. A notification/attempt is a transient `ClipboardObservation`; denied, failed, skipped, or degraded outcomes use content-free `CaptureAuditEvent` records where policy permits. Source-owned hard deny creates neither a clip nor a durable audit row.

Capture uses reviewed Win32 adapters for common standard/registered formats and supplemental short-lived OLE `IDataObject` access only where `FORMATETC`, `lindex`, `IStream`, virtual-file, or richer medium semantics are required. Each representation records durable format identity, adapter/version, medium evidence, capture state, size, storage reference, protection/digest policy, fidelity, and safety notes.

Standard formats persist defined IDs. Registered formats persist their exact registered names and are re-registered during replay; runtime numeric registered-format IDs are transient only. Unknown custom formats are not blindly serialized or replayed.

Transformations create versioned derived representations with parent links and never mutate originals. Detailed invariants are defined in `../architecture/data-model.md` and adapter policy in `../architecture/clipboard-format-policy.md`.

Aggregate and per-representation fidelity use:

- `FullFidelity`;
- `CommonFormatsPreserved`;
- `FallbackOnly`;
- `ReferenceOnly`;
- `Unavailable`;
- `UnsafeOrUnsupported`.

## Consequences

Positive:

- rich and fallback formats remain connected;
- replay can construct destination-appropriate data objects;
- transformations are auditable and reversible;
- limitations are explicit.

Costs:

- more complex schema, capture fixtures, and replay logic;
- application-private formats require safe adapter policies;
- event observation/current-state boundaries need sequence/source/coalescing tests without claiming every rapid intermediate copy is observable;
- registered-name and adapter migrations must remain backward compatible.

## Alternatives considered

- **One record per format:** simpler storage but poor UX and broken provenance.
- **Preferred-format-only history:** low storage complexity but incompatible with fidelity mission.
- **Serialize foreign IDataObject directly:** unsafe, process-lifetime-dependent, and not portable across restart.

## Review triggers

- fixture evidence shows a required clipboard scenario cannot map to the model;
- a format needs live process callbacks or references that require a distinct state;
- event coalescing loses meaningful copy occurrences;
- schema cost materially harms search or retention performance.
