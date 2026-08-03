# ADR 0006: SQLite metadata, FTS5, and recoverable blob storage

**Status:** Accepted
**Date:** 2026-08-03

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
- content-addressed final paths for ordinary non-sensitive payloads;
- random identifiers or keyed hashes for sensitive encrypted payloads;
- startup and explicit health-check reconciliation for incomplete staging and unreferenced blobs.

WAL is not enabled merely by convention. The storage prototype compares rollback journaling and WAL across crash, suspend, antivirus, backup, low-disk, shutdown, and single-owner workloads. The selected mode and pragmas are recorded with benchmark and recovery evidence.

## Consequences

Positive:

- mature transactional metadata and fast local filters/search;
- large payload streaming without loading full history into memory;
- duplicate byte storage can be shared without collapsing event history;
- storage recovery can reason separately about metadata and blobs.

Costs:

- two-part commit requires explicit recovery reconciliation;
- backup/export must understand SQLite journaling and blob consistency;
- quota cleanup must preserve shared references and pinned items.

## Alternatives considered

- **All payloads as SQLite BLOBs:** simpler referential integrity but less suitable for large streaming payloads and file-level recovery.
- **RocksDB/LMDB/custom store:** unnecessary operational complexity and weaker built-in relational/FTS tooling for this product.
- **Multiple process database access:** rejected to simplify migrations, locks, encryption, and audit policy.

## Review triggers

- benchmark evidence shows blob files are slower or less reliable than SQLite BLOBs for target workloads;
- FTS5 cannot satisfy required query semantics or scale;
- one-owner IPC becomes a measured bottleneck;
- selected journal mode fails recovery or backup acceptance tests;
- encrypted metadata requirements justify a separate private-profile database.
