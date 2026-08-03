# Glossary

## Capture-critical path

The bounded work executed from clipboard notification until Pastral has safely copied the required immediate representations or decided to skip them. It excludes OCR, syntax highlighting, semantic indexing, large preview generation, and other enrichment.

## ClipEvent

One logical copy occurrence identified by Pastral. It includes timestamp, source context, active profile, capture policy result, sequence metadata, and zero or more `ClipRepresentation` records. Multiple formats advertised by one clipboard update belong to the same event.

## ClipRepresentation

One preserved or referenced format within a `ClipEvent`, including format identity, medium, byte length, content/blob reference, hash strategy, fidelity, safety state, and capture notes.

## Copy occurrence

A meaningful observation that a payload was copied at a time and source. Identical payloads may share blob storage but retain separate occurrences.

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

Captured representation bytes and original metadata that transformations cannot modify. Deletion and retention may remove the original according to explicit policy, but an action cannot rewrite it in place.

## PasteTransaction

A bounded operation that reconstructs a data object, publishes safe formats to the clipboard, optionally sends paste to an explicitly selected destination, monitors result heuristics, retains object lifetime as required, and records metadata-only outcome.

## Profile

A separate clipboard context defining retention, quota, capture allow/deny policy, sensitive-data behavior, indexing, transformations, paste defaults, visual accent, encryption, and optional switching rules.

## Quick Paste

The explicitly invoked keyboard-first picker used to search, inspect, select a representation/paste mode, and paste or copy a history item.

## Representation set

The complete collection of formats and media advertised for one logical clipboard event.

## Rule

A versioned deterministic mapping from explicit match conditions and scope to one or more allowed actions. Rules include priority, enabled state, origin, audit history, match count, last result, and rollback information.

## RuleMatch

A replayable explanation of which rule predicates matched an event or paste context, which higher-priority privacy rules applied, and why an action was selected or rejected.

## SensitiveItemSkipped

An optional metadata-only event indicating that capture was suppressed by sensitive-content policy. It contains no preview, value, content hash, OCR text, token fragment, reconstructable metadata, or secret-bearing log fields.

## SourceContext

Privacy-filtered information about where a copy originated, potentially including process identity, package identity, window class, redacted title, browser/domain signal, project/workspace, session, monitor, and confidence.

## Transformation

A pure or isolated operation that creates a derived representation, such as plain-text extraction, URL tracking removal, image-to-PNG, OCR, filename extraction, or code-block formatting.

## Worker

The on-demand, resource-bounded process that handles expensive or hostile parsing/enrichment. It has no network access by default and cannot own the primary database.
