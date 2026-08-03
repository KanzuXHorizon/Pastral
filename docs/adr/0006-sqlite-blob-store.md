# ADR 0006: SQLite metadata, FTS5, and recoverable blob storage

**Status:** Accepted
**Date:** 2026-08-03
**Amended:** 2026-08-04 — define `sha256-raw-v1`, protection-domain boundaries, random Private/sensitive identities, and deletion-remnant evaluation after the Phase 0 adversarial audit.

## Context

Pastral needs transactional metadata, structured filtering, full-text search, schema migration, duplicate payload reduction, large streamed payloads, crash recovery, quotas, and offline operation. Storing every payload in SQLite would increase copy amplification and complicate large image/file handling; storing all metadata as loose files would weaken query and migration behavior.

## Decision

Use:

- SQLite for metadata, relationships, audit records, migrations, and structured indexes;
- FTS5 for deterministic lexical search;
- a separate blob directory for captured and derived payloads;
- one database owner process: `pastral-agent.exe`;
- prepared statements and typed repository interfaces;
- temporary-file staging followed by durable close and atomic final rename where supported;
- content-addressed final paths for ordinary non-sensitive payloads using versioned `sha256-raw-v1` over logical raw bytes before optional physical storage encoding, namespaced by compatible ordinary protection domain;
- random identifiers and no persistent plaintext digest/deduplication by default for Private or sensitive encrypted payloads;
- startup and explicit health-check reconciliation for incomplete staging and unreferenced blobs;
- explicit evaluation of FTS deletion, freelist reuse, `secure_delete`, vacuum strategy, rollback/WAL checkpoint retention, snapshots, and backups without promising forensic erasure.

WAL is not enabled merely by convention. The storage prototype compares rollback journaling and WAL across crash, suspend, antivirus, backup, low-disk, shutdown, deletion-remnant, and single-owner workloads. The selected journal mode, checkpoint policy, `secure_delete` setting, vacuum strategy, foreign-key mode, synchronization level, and temporary-store policy are recorded with benchmark, privacy, and recovery evidence.

## Consequences

Positive:

- mature transactional metadata and fast local filters/search;
- large payload streaming without loading full history into memory;
- duplicate byte storage can be shared without collapsing event history;
- storage recovery can reason separately about metadata and blobs.

Costs:

- two-part commit requires explicit recovery reconciliation;
- backup/export must understand SQLite journaling and blob consistency;
- quota cleanup must preserve shared references and pinned items;
- logical deletion can leave bytes in database pages, FTS structures, journals/WAL, snapshots, or backups until later maintenance/overwrite, so UI language cannot promise secure erasure.

## Alternatives considered

- **All payloads as SQLite BLOBs:** simpler referential integrity but less suitable for large streaming payloads and file-level recovery.
- **RocksDB/LMDB/custom store:** unnecessary operational complexity and weaker built-in relational/FTS tooling for this product.
- **Multiple process database access:** rejected to simplify migrations, locks, encryption, and audit policy.

## Review triggers

- benchmark evidence shows blob files are slower or less reliable than SQLite BLOBs for target workloads;
- FTS5 cannot satisfy required query semantics or scale;
- one-owner IPC becomes a measured bottleneck;
- selected journal mode fails recovery or backup acceptance tests;
- encrypted metadata requirements justify a separate private-profile database;
- deletion-remnant testing shows the selected FTS/journal/vacuum policy is unacceptable.
