# ADR 0010: Signed staged updates with schema compatibility and rollback evidence

**Status:** Accepted at policy level; mechanism selected during packaging implementation
**Date:** 2026-08-03

## Context

A resident clipboard utility processes sensitive data and owns a persistent schema. Updates are a security boundary and can corrupt history or prevent startup if package, migration, or protocol compatibility is wrong. Core functionality must remain network-silent unless update checking is explicitly enabled or provided by a trusted package channel.

## Decision

- Prefer Microsoft Store/MSIX-managed updates for Store installations.
- For signed side-loaded MSIX, select a supported signed update mechanism during packaging implementation; no custom updater executes before threat modeling and test coverage.
- Never download or execute an update in the core release without explicit user/admin policy.
- Verify publisher identity, package signature, version monotonicity, architecture, and package family before installation.
- Separate package update from user-data migration. Agent performs versioned, restart-safe migrations after package activation.
- Maintain protocol compatibility across one rolling upgrade boundary where old/new process overlap is possible.
- Back up migration-critical metadata before irreversible schema changes and document recovery.
- Use staged release rings only after an opt-in update channel exists; no clipboard content enters update telemetry.

## Consequences

Positive:

- trusted platform update path where available;
- no unauthenticated custom bootstrapper;
- schema and process compatibility are treated as release gates.

Costs:

- update behavior differs by Store and side-loaded channels;
- rollback of package binaries does not automatically roll back migrated data;
- migration compatibility requires additional fixtures and release artifacts.

## Alternatives considered

- **Always-running custom updater:** rejected due to resident footprint and supply-chain attack surface.
- **In-app unsigned download:** prohibited.
- **No updates:** unacceptable for a security-sensitive parser and Windows integration product.

## Review triggers

- Store/enterprise distribution decision;
- selected side-load update technology;
- first non-additive database migration;
- publisher certificate rotation;
- update incident or rollback failure.
