# Architecture decision records

ADRs capture decisions that materially affect architecture, security, privacy, compatibility, deployment, or long-term maintenance.

## Status values

- **Proposed:** under review; implementation must not depend on it.
- **Accepted:** current direction.
- **Superseded:** replaced by a later ADR, which must be linked.
- **Deferred:** intentionally postponed with a defined review trigger.
- **Rejected:** considered and not selected.

## Required sections

Every ADR includes:

1. Status
2. Date
3. Context
4. Decision
5. Consequences
6. Alternatives considered
7. Review triggers

## Index

- [0001 — Rust core](0001-rust-core.md)
- [0002 — WinUI 3 manager](0002-winui3-manager.md)
- [0003 — Overlay rendering](0003-overlay-rendering.md)
- [0004 — Clipboard data-object model](0004-clipboard-data-object-model.md)
- [0005 — Process boundaries](0005-process-boundaries.md)
- [0006 — SQLite and blob store](0006-sqlite-blob-store.md)
- [0007 — Encryption and key storage](0007-encryption-key-storage.md)
- [0008 — IPC protocol](0008-ipc-protocol.md)
- [0009 — Packaging](0009-packaging.md)
- [0010 — Update strategy](0010-update-strategy.md)
- [0011 — Windows support baseline](0011-windows-support-baseline.md)
- [0012 — OCR approach](0012-ocr-approach.md)
- [0013 — Semantic search deferred](0013-semantic-search-deferred.md)
- [0014 — Plugin system deferred](0014-plugin-system-deferred.md)
- [0015 — Dedicated capture STA and hybrid Win32/OLE capture](0015-clipboard-capture-threading.md)
- [0016 — Durable identity, time, and digest semantics](0016-data-identity-time-and-digests.md)
- [0017 — Quick Paste hosting](0017-quick-paste-hosting.md)

An ADR change is committed with the implementation or plan it governs. Historical decisions are not rewritten to conceal prior reasoning; corrections use a new ADR or an explicit amendment note.
