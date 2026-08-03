# Architecture documentation

Read these documents in this order before implementation:

1. [`overview.md`](overview.md) — system shape, dependencies, executables, storage ownership.
2. [`process-model.md`](process-model.md) — process lifetimes, privileges, activation, session isolation.
3. [`threading-and-com-apartments.md`](threading-and-com-apartments.md) — control thread, capture STA, storage/IPC/worker execution contexts.
4. [`clipboard-event-identity.md`](clipboard-event-identity.md) — notification/sequence semantics, observations, coalescing, self-origin marker, ordering.
5. [`source-context.md`](source-context.md) — source evidence classes, confidence, privacy, rule safety.
6. [`clipboard-format-policy.md`](clipboard-format-policy.md) — adapter support matrix, stable format identity, ownership, limits, replay policy.
7. [`data-model.md`](data-model.md) — immutable capture core, derived content, mutable user state, audit records, IDs/time/digests.
8. [`clipboard-lifecycle.md`](clipboard-lifecycle.md) — capture state machine, policy, persistence, enrichment, retention.
9. [`paste-lifecycle.md`](paste-lifecycle.md) — replay object, publication, destination validation, UIPI/manual fallback, restoration.
10. [`ipc-security-model.md`](ipc-security-model.md) — pipe creation, logon-session isolation, peer validation, handshake, authorization, residual risk.
11. [`data-flow.md`](data-flow.md) — end-to-end sequence and trust-boundary diagrams.

## Normative priority

When documents conflict:

1. a later Accepted ADR or explicit amendment controls;
2. the architecture document named by that ADR controls implementation detail;
3. security/privacy hard-deny requirements control over convenience behavior;
4. implementation plans must be updated before code is written;
5. historical reviews/specs remain evidence, not current authority when marked superseded.

Current hardening decisions:

- ADR 0015: dedicated capture STA and hybrid Win32/OLE capture;
- ADR 0016: UUIDv4 identities, UTC/monotonic time split, persistent installation-local `capture_order`, `sha256-raw-v1`, protection domains;
- ADR 0017: Quick Paste hosted by the on-demand manager process.

## Architecture invariants

- One always-running process by default.
- Control/overlay thread never invokes foreign clipboard/OLE methods.
- A successful `ClipEvent` has at least one captured representation.
- Source-owned hard deny creates no durable record.
- Registered formats persist names, not runtime numeric IDs.
- Manager/CLI/worker never open the primary database directly.
- Named pipes strongly isolate users/logon sessions but do not claim a same-user secure enclave.
- Private/sensitive plaintext is not persistently hashed/deduplicated by default.
- Synthetic paste never bypasses Windows integrity boundaries.
- Every speed/security/fidelity claim requires test evidence.
