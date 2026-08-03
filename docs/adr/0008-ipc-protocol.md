# ADR 0008: Versioned authenticated local named-pipe IPC

**Status:** Accepted
**Date:** 2026-08-03

## Context

Manager and CLI require local access to history, settings, rules, diagnostics, and commands without opening the database. The transport must isolate users and logon sessions, enforce bounded messages, evolve compatibly, and remain lightweight in the resident agent.

## Decision

Use Windows named pipes with:

- an explicit security descriptor; never the default descriptor;
- access restricted to the current user SID and current logon SID, with SYSTEM included only when a concrete packaging/update requirement justifies it;
- a per-installation unguessable component and per-session scope in the pipe name;
- protocol major/minor version negotiation and capability flags;
- length-prefixed messages with strict maximum sizes;
- a schema format selected during protocol bootstrap that rejects unknown dangerous variants and never deserializes executable/arbitrary object types;
- challenge-response bound to a per-installation secret protected by DPAPI, in addition to OS ACLs;
- correlation IDs, deadlines, cancellation, pagination, and bounded subscriptions;
- peer identity/session validation through Windows pipe APIs;
- replay-resistant handshake nonces and short-lived connection state;
- metadata-only security logging.

Manager and CLI use the same public protocol contracts. Worker job transport is separate and narrower.

## Consequences

Positive:

- Windows-native local transport with strong user/session ACLs;
- supports request/response, incremental results, and notifications;
- database and business logic remain centralized;
- protocol can evolve independently of UI implementation language.

Costs:

- protocol schemas, compatibility tests, and cancellation semantics are required;
- named-pipe ACL construction and impersonation checks contain security-sensitive Win32 code;
- challenge secret lifecycle must be protected and rotated safely.

## Alternatives considered

- **Loopback HTTP/gRPC:** larger resident dependency and attack/listening surface; unnecessary for local-only core.
- **COM local server:** rich Windows integration but more registration/versioning complexity and broader activation behavior.
- **Shared memory:** efficient for bulk transfer but poor as the sole authenticated control protocol; may be added for measured large-preview needs.
- **Direct database access:** rejected because it bypasses policy, migrations, encryption, and audit boundaries.

## Review triggers

- schema/serialization library selection;
- fuzzing reveals ambiguity or resource exhaustion;
- packaged app identity changes pipe ACL or activation constraints;
- large payload transfer becomes a measured bottleneck;
- cross-session or service communication is proposed.
