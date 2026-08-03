# ADR 0013: Semantic search is deferred and optional

**Status:** Deferred
**Date:** 2026-08-03

## Context

Semantic search may help users recall content without exact words, but it adds model/runtime footprint, indexing cost, privacy considerations, language quality variance, storage migrations, and ranking opacity. Pastral's primary search must remain exact, deterministic, and fast.

## Decision

Do not implement semantic search in the first release.

Complete and benchmark first:

- SQLite FTS5 lexical search;
- exact phrase, prefix, structured filters, dates, tags, source, profile, type, fidelity, and representation filters;
- typo-tolerant lexical fallback with deterministic bounds;
- incremental/cancellable result delivery at 1,000,000 metadata records;
- privacy-safe snippets and sensitive-data exclusion.

A later semantic module must:

- be optional and local by default;
- run indexing in the on-demand worker;
- exclude sensitive/private-profile content unless explicitly enabled;
- expose model/version, indexed fields, storage usage, and deletion/rebuild controls;
- combine with lexical/structured results without replacing exact matches;
- provide an explanation such as matched terms, semantic similarity, or source/filter constraints;
- remain removable without making history unusable.

## Consequences

Positive:

- smaller MVP scope and resident footprint;
- deterministic search remains the reliable foundation;
- avoids premature model/vendor and privacy commitments.

Costs:

- fuzzy conceptual recall is limited in the first release;
- later indexing requires migration and benchmark work;
- users may compare Pastral unfavorably with AI-first products before this module exists.

## Alternatives considered

- **Remote embeddings/search:** rejected by local-first defaults and content privacy.
- **Bundled local model in first release:** rejected due to footprint and unproven value.
- **Semantic-only ranking:** rejected because exact/structured queries must be predictable.

## Review triggers

- lexical search meets all performance and quality gates;
- worker infrastructure and private-index policies are mature;
- reference hardware can index without unacceptable battery/CPU/storage cost;
- user research demonstrates unmet conceptual-recall needs.
