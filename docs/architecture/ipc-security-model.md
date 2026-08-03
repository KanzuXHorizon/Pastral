# IPC security and authorization model

## 1. Security boundary

Pastral local IPC provides a strong operating-system boundary against:

- other Windows users;
- other logon sessions, including another session of the same account;
- remote named-pipe clients;
- accidental or stale clients that do not speak the current protocol;
- unauthorized operations from clients that fail token/session/capability checks.

It does **not** provide a strong confidentiality boundary against a fully compromised process already executing as the same unlocked Windows user. Such a process may be able to read user files, invoke user-scope DPAPI, automate the desktop, inspect process memory under applicable policy, or call Pastral as the user. IPC hardening reduces attack surface and confusion; it does not create a secure enclave.

## 2. Pipe naming and discovery

- Use a per-installation random public instance component and per-logon-session scope.
- Store discovery metadata in a current-user-only location with restrictive ACL and versioning; the name is not treated as a secret.
- Avoid clipboard content, usernames, profile names, clip IDs, or machine identifiers in pipe names.
- Rotate the instance component during reinstall/security reset with bounded compatibility behavior.
- Clients reject discovery records with unsupported schema, wrong user/session, invalid length, or stale agent process identity.

## 3. Server creation

Every server instance uses:

- `FILE_FLAG_FIRST_PIPE_INSTANCE` for the first creation attempt to detect/suppress name squatting;
- `PIPE_REJECT_REMOTE_CLIENTS`;
- overlapped I/O rather than compatibility `PIPE_NOWAIT`;
- explicit security descriptor, never a null/default descriptor;
- least-privilege allow ACE for the current **logon SID** as the normal client-access principal; SYSTEM only for a documented package/update need;
- no ordinary allow ACE for the broad user SID, because DACL allow entries are additive and a user-SID ACE would let another logon session of the same account bypass logon-SID isolation;
- runtime token checks additionally require the expected token user SID, logon SID, and session ID;
- no broad `Everyone`, `Anonymous`, network, interactive-user, or generic write grant;
- bounded instance count, input/output buffers, message size, and connection lifetime.

Failure to create the first instance is treated as a possible existing/stale/spoofed instance and requires process/token validation before any recovery action.

## 4. Peer validation

After connection and before expensive parsing:

1. retrieve client PID where supported;
2. open/query client process token with minimum rights;
3. validate token user SID, enabled logon SID, session ID, integrity level, app/package identity where policy requires, and expected executable identity for first-party privileged capabilities;
4. optionally impersonate only in a tiny reviewed scope to query the effective client token;
5. always call `RevertToSelf` on every path, including errors/panics;
6. reject protected/ambiguous identity when an operation requires stronger evidence;
7. bind validated peer identity to the connection and revalidate on reconnect, not per claimed message field.

PID and executable path/signature are evidence and anti-confusion controls, not proof against same-user code injection or replacement. The protocol never trusts a client-supplied PID/session/user field.

## 5. Handshake

The handshake provides protocol freshness and instance binding:

- major/minor version and capability negotiation;
- random server/client nonces;
- agent instance/session/installation binding;
- transcript authentication using a user-scope DPAPI-protected installation secret;
- replay cache limited to the active process/session window;
- key/secret version and rotation support;
- no clipboard or source content in handshake material/logs.

This detects stale/wrong installations and simple replay/confusion. Because same-user code can often access user-scope DPAPI material, it is not represented as a same-user malware barrier.

## 6. Message framing and schemas

- Fixed endian/versioned length prefix with strict maximum before allocation.
- Explicit request/response/event discriminants; no arbitrary object deserialization.
- Unknown mandatory variants rejected; optional fields follow negotiated capability rules.
- Checked arithmetic, bounded strings/collections/nesting, deadlines, cancellation, pagination, and backpressure.
- Per-client/global limits for connections, in-flight requests, subscriptions, search cost, export bytes, and destructive operations.
- Incremental results carry query/request IDs so obsolete responses cannot mutate current UI state.
- Payload transfer uses bounded chunks or separately opened handles/shared memory only after a dedicated measured design; no giant unbounded message.

## 7. Authorization classes

### Low risk

Examples: health, capture state, protocol capabilities, redacted counts. Allowed after validated connection.

### Ordinary user data

Examples: paged ordinary history/search, ordinary clip detail, settings/rules. Requires current user/session and active profile policy.

### Sensitive/private reveal or paste

Requires:

- explicit foreground user action in trusted Pastral UI;
- profile/item unlock state and freshness policy;
- no passive subscription delivering plaintext;
- response scoped to one operation with short lifetime;
- future Windows Hello/user-presence verification where configured.

The CLI cannot retrieve sensitive/private payload content by merely passing a flag; a separate authorized flow is required.

### Destructive/high-impact

Examples: delete all, export, key rotation, profile reset, rule import, diagnostic bundle with source metadata. Requires explicit UI confirmation, operation summary, revalidation of active user/session, and audit. Noninteractive automation must use a separately designed capability, never an undocumented bypass.

## 8. Client behavior

- Manager and CLI validate server PID/token/session/package/publisher evidence before trusting content.
- Clients do not fall back to another user's/session's pipe.
- Protocol mismatch fails with safe restart/update guidance.
- Reconnect uses bounded exponential delay and never blocks capture/UI threads.
- Manager view models clear protected content immediately on disconnect, lock, session change, or authorization expiry.

## 9. Same-user residual risk

Documented residuals:

- same-user malware may invoke ordinary APIs as the user;
- current-user history files are accessible according to filesystem/DPAPI/session security policy;
- executable identity can be spoofed after compromise/injection;
- unlocked sensitive content shown to the user may be observable by same-user malware/screen capture;
- user-presence verification reduces accidental/background access but cannot defeat a compromised OS/session.

Mitigation emphasis:

- minimize stored data and retention;
- encrypt Private/sensitive payloads;
- require explicit user intent for reveal/export/destructive actions;
- keep content out of logs, notifications, subscriptions, and diagnostics;
- use Windows session lock and device encryption;
- avoid elevated/service components that would increase blast radius.

## 10. Required tests

- other user and same-user different logon session denied, including a regression proving no broad user-SID allow ACE defeats logon-SID isolation;
- remote client rejected;
- first-instance squatting and stale discovery recovery;
- null/default/broad DACL regression test;
- PID reuse/race and client exit during validation;
- impersonation failure and guaranteed `RevertToSelf`;
- wrong user/session/integrity/package/publisher evidence;
- handshake replay, wrong installation, rotated secret, nonce reuse, transcript tamper;
- oversized/nested/malformed frames and subscription backpressure;
- unauthorized sensitive reveal/export/delete/rule/key operations;
- CLI cannot retrieve Private/sensitive content without authorized flow;
- disconnect/lock clears protected view models;
- documentation test verifies no claim of protection from fully compromised same-user processes.
