# ADR 0008: Versioned authenticated local named-pipe IPC

**Status:** Accepted
**Date:** 2026-08-03
**Amended:** 2026-08-04 — correct same-user threat claims, use logon-SID-first ACLs, add anti-squatting/remote rejection, peer validation, and user-intent authorization after the Phase 0 adversarial audit.

## Context

Manager and CLI require local access to history, settings, rules, diagnostics, and commands without opening the database. The transport must isolate users and logon sessions, enforce bounded messages, evolve compatibly, and remain lightweight in the resident agent.

## Decision

Use Windows named pipes with the detailed model in `../architecture/ipc-security-model.md`:

- an explicit least-privilege security descriptor; never the default descriptor;
- normal client access granted through a least-privilege current logon-SID ACE, with token user SID/logon SID/session validated at runtime; do not add a broad user-SID allow ACE that would defeat same-account cross-session isolation; SYSTEM is included only when a concrete packaging/update requirement justifies it;
- `FILE_FLAG_FIRST_PIPE_INSTANCE` on initial server creation and `PIPE_REJECT_REMOTE_CLIENTS`;
- a per-installation random component and per-session scope in the pipe name; the name is not treated as a secret;
- protocol major/minor version negotiation and capability flags;
- length-prefixed messages with strict maximum sizes;
- the bounded 36-byte framing, Protobuf Edition 2024 control-schema prototype, and sequenced bulk-transfer state machine proposed by ADR 0018; never deserialize executable/arbitrary object types or put large clipboard payloads in control messages; final resident runtime adoption remains gated by footprint/build/security evidence;
- instance/transcript-bound challenge-response using a per-installation secret protected by user-scope DPAPI, for stale/wrong-client and replay/confusion resistance rather than a claimed barrier against fully compromised same-user processes;
- correlation IDs, deadlines, cancellation, pagination, and bounded subscriptions;
- server and client PID/token/user/logon-session validation through Windows pipe APIs, with bounded impersonation and guaranteed `RevertToSelf`;
- replay-resistant handshake nonces and short-lived connection state;
- metadata-only security logging.

Manager and CLI use the same public protocol contracts. Worker job transport is separate and narrower.

The strong boundary is cross-user, cross-session, and remote-client isolation. A process fully compromised under the same unlocked user is outside a strong confidentiality boundary; sensitive reveal/export/destructive operations additionally require explicit foreground user intent and future user-presence freshness where configured.

## Consequences

Positive:

- Windows-native local transport with strong user/session ACLs;
- supports request/response, incremental results, and notifications;
- database and business logic remain centralized;
- protocol can evolve independently of UI implementation language.

Costs:

- protocol schemas, compatibility tests, and cancellation semantics are required;
- named-pipe ACL construction and impersonation checks contain security-sensitive Win32 code;
- challenge secret lifecycle must be protected and rotated safely;
- executable/PID/package evidence remains defense in depth and cannot prove safety after same-user process compromise.

## Alternatives considered

- **Loopback HTTP/gRPC:** larger resident dependency and attack/listening surface; unnecessary for local-only core.
- **COM local server:** rich Windows integration but more registration/versioning complexity and broader activation behavior.
- **Shared memory:** efficient for bulk transfer but poor as the sole authenticated control protocol; may be added for measured large-preview needs.
- **Direct database access:** rejected because it bypasses policy, migrations, encryption, and audit boundaries.

## Review triggers

- selected Protobuf generator/runtime footprint, build integration, security, or reproducibility fails ADR 0018 acceptance gates;
- framing/schema/parser fuzzing, resource limits, or adjacent-version compatibility reveals ambiguity;
- packaged app identity changes pipe ACL or activation constraints;
- large payload transfer becomes a measured bottleneck;
- cross-session or service communication is proposed;
- a sensitive/private operation needs a stronger user-presence contract.
