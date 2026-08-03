# ADR 0006: SQLite metadata, FTS5, and recoverable blob storage

**Status:** Accepted
**Date:** 2026-08-03
**Amended:** 2026-08-04 — define `sha256-raw-v1`, protection-domain boundaries, random Private/sensitive identities, deletion-remnant evaluation, and a benchmark-gated hybrid blob backend after the adversarial audit.

## Context

Pastral needs transactional metadata, structured filtering, full-text search, schema migration, duplicate payload reduction, large streamed payloads, crash recovery, quotas, and offline operation. A file per tiny text/HTML payload can create large file-count, filesystem-block, antivirus, backup, and open/close overhead; forcing large image/stream payloads into ordinary SQL statements can increase copy amplification and complicate bounded streaming. SQLite's own documentation reports that small BLOBs can outperform separate files on tested systems, while larger BLOBs can favor the filesystem. Pastral therefore must measure the crossover on supported Windows hardware rather than assume one backend is universally superior.

## Decision

Use the logical/physical lifecycle defined in [`../architecture/blob-store-lifecycle.md`](../architecture/blob-store-lifecycle.md):

- SQLite for metadata, relationships, audit records, migrations, and structured indexes;
- FTS5 for deterministic lexical search;
- one content-addressed `BlobStore` abstraction whose physical backend is either an internal SQLite BLOB row or an external staged/final file; event/representation tables store references, never duplicate inline payload columns;
- one database owner process: `pastral-agent.exe`;
- prepared statements and typed repository interfaces;
- transactional SQLite BLOB insertion for selected bounded small ordinary payloads, and temporary-file staging followed by durable close and atomic final rename for selected external/large/streaming payloads;
- content-addressed final paths for ordinary non-sensitive payloads using versioned `sha256-raw-v1` over logical raw bytes before optional physical storage encoding, namespaced by compatible ordinary protection domain;
- random identifiers and no persistent plaintext digest/deduplication by default for Private or sensitive encrypted payloads;
- startup and explicit health-check reconciliation for incomplete external staging, unreferenced files, orphaned blob rows, and backend/reference mismatches;
- explicit evaluation of FTS deletion, freelist reuse, `secure_delete`, vacuum strategy, rollback/WAL checkpoint retention, snapshots, and backups without promising forensic erasure.

The storage prototype selects an internal/external threshold and backend policy using Windows x64 measurements with realistic text/HTML/image distributions, 100k–1M metadata rows, Defender/antivirus enabled, warm/cold cache, backup, crash injection, low disk, deletion, export, and migration. The threshold is versioned and may differ by protection domain or payload class, but a blob never silently moves backend without a transactional migration.

WAL is not enabled merely by convention. The storage prototype compares rollback journaling and WAL across crash, suspend, antivirus, backup, low-disk, shutdown, deletion-remnant, and single-owner workloads. The selected journal mode, checkpoint policy, `secure_delete` setting, vacuum strategy, foreign-key mode, synchronization level, and temporary-store policy are recorded with benchmark, privacy, and recovery evidence.

## Consequences

Positive:

- mature transactional metadata and fast local filters/search;
- small ordinary payloads can avoid a file-per-clip explosion when evidence favors SQLite;
- large/streaming payloads can use external files without loading full content into memory;
- duplicate byte storage can be shared without collapsing event history;
- one logical blob contract keeps digest/protection/reference semantics independent of physical backend.

Costs:

- two physical backends and threshold migrations require explicit invariants, tests, and recovery reconciliation;
- external two-part commits require reconciliation, while internal BLOB commits affect database/journal size and checkpoint behavior;
- backup/export must understand SQLite journaling, internal BLOBs, and external-file consistency;
- quota cleanup must preserve shared references and pinned items;
- logical deletion can leave bytes in database pages, FTS structures, journals/WAL, snapshots, or backups until later maintenance/overwrite, so UI language cannot promise secure erasure.

## Alternatives considered

- **All payloads as external files:** simple large-stream path but risks excessive tiny-file/antivirus/filesystem overhead and was rejected without Windows evidence.
- **All payloads as SQLite BLOBs:** simple referential integrity and efficient small-BLOB behavior, but large/streaming payloads, database growth, backup/checkpoint, and write-amplification risks require an external option.
- **RocksDB/LMDB/custom store:** unnecessary operational complexity and weaker built-in relational/FTS tooling for this product.
- **Multiple process database access:** rejected to simplify migrations, locks, encryption, and audit policy.

## Review triggers

- benchmark evidence materially changes the selected internal/external threshold or shows one backend should be removed;
- FTS5 cannot satisfy required query semantics or scale;
- one-owner IPC becomes a measured bottleneck;
- selected journal mode fails recovery or backup acceptance tests;
- encrypted metadata requirements justify a separate private-profile database;
- deletion-remnant testing shows the selected FTS/journal/vacuum or backend policy is unacceptable;
- internal/external migration cannot preserve atomicity, deduplication, or privacy semantics.
