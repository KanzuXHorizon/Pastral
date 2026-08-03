# ADR 0004: Immutable multi-representation clipboard event model

**Status:** Accepted
**Date:** 2026-08-03

## Context

One copy operation can expose Unicode text, HTML, RTF, images, URLs, file lists, Shell/OLE metadata, and application-private formats simultaneously. Creating one history item per format loses their relationship; storing only a preferred format loses fidelity.

## Decision

Model each meaningful clipboard update as one immutable `ClipEvent` containing multiple `ClipRepresentation` records.

Capture prefers short-lived OLE `IDataObject` enumeration and supplements it with Win32 format enumeration where required. Each representation records format, medium, capture state, size, storage reference, fidelity, and safety notes.

Transformations create versioned derived representations with parent links and never mutate originals.

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
- exact event boundaries need sequence/source/coalescing tests.

## Alternatives considered

- **One record per format:** simpler storage but poor UX and broken provenance.
- **Preferred-format-only history:** low storage complexity but incompatible with fidelity mission.
- **Serialize foreign IDataObject directly:** unsafe, process-lifetime-dependent, and not portable across restart.

## Review triggers

- fixture evidence shows a required clipboard scenario cannot map to the model;
- a format needs live process callbacks or references that require a distinct state;
- event coalescing loses meaningful copy occurrences;
- schema cost materially harms search or retention performance.
