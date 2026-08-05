# Plan 008: Build encryption, Private profile, profiles, and deterministic rules

> **Executor instructions:** Use an isolated worktree. Encryption/key lifecycle is a security boundary and must be implemented before enabling Private. Do not create an unencrypted “Private” placeholder or persist plaintext digests/previews/indexes for protected content.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- crates/domain crates/storage crates/agent-core crates/ipc-core crates/ipc-schema apps/agent apps/manager protocols eng docs/security docs/architecture docs/ux docs/adr/0007-encryption-key-storage.md`

## Status

- **Priority:** P1
- **Effort:** XL
- **Risk:** HIGH
- **Depends on:** Plans 006 and 007
- **Category:** security, privacy, profiles, rules, product direction
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

Pastral targets privacy-sensitive users and promises deterministic, inspectable behavior. Current storage intentionally rejects protected payloads; Private, profile enforcement and rule automation are not implemented. V1 must add authenticated encryption, key/lock/recovery lifecycle and non-indexing first, then build profiles and rules whose capture/replay consequences are enforced by the agent rather than merely displayed by the manager.

## Security contract

- Private payloads are encrypted before durable persistence with authenticated, versioned envelopes.
- Private blob identity is random and non-equality-revealing; no persistent plaintext digest or default plaintext deduplication.
- Authentication is verified before plaintext release.
- Keys are never sent over ordinary IPC, logged, exported in diagnostics or retained after lock/session policy clears them.
- UI authorization/user presence is distinct from storage encryption; Windows Hello may later unlock/wrap but is not the encryption primitive.
- Private content is absent from ordinary FTS, OCR, semantic indexes, duplicate stacks, previews, thumbnails, UIA, suggestions and ordinary exports.
- Same-user compromise remains residual risk; do not call this a secure enclave.

## Target architecture

- Storage encryption module owns envelope format and streaming authenticated encryption/decryption.
- Agent key service owns installation/profile key wrapping, unlock state, auto-lock/session-lock and rotation/recovery.
- Profile service owns versioned capture/retention/index/replay policy; agent enforces it at every boundary.
- Rule engine consumes immutable context facts and emits deterministic explainable actions. Privacy hard deny always outranks ordinary rules.
- Manager consumes redacted provider views and explicit unlock/authorization operations; it never receives hidden content until authorized.

## Scope

**In scope**

- encrypted Private/sensitive storage and key lifecycle
- lock/unlock/auto-lock/session-lock/rotation/recovery
- versioned built-in/user profiles and enforcement
- deterministic rules, conflict resolution, simulation, audit and rollback
- narrow remember-choice recommendations with explicit consent
- manager/provider contracts for secure management

**Out of scope**

- Cloud/device sync, account recovery service, team key sharing.
- Treating Windows Hello as direct data encryption.
- AI-learned opaque automation.
- Arbitrary scripts/plugins.
- OCR/semantic indexing of Private content.

## Program slices

### Slice 1: Freeze the encrypted-envelope and key hierarchy design

Review ADR 0007 and threat model against implemented storage/blob backends. Define exact versioned envelope fields, nonce uniqueness, associated data (event/representation/profile/domain/version), chunk ordering, final authentication, size ceilings and algorithm agility. Choose only a well-reviewed authenticated encryption implementation supported by the pinned toolchain; document dependency/supply-chain/footprint impact.

Define key hierarchy:

- random installation/profile data key;
- user-scope protected wrapping material with explicit entropy/context;
- separate recovery/rotation metadata;
- no key bytes in ordinary structs with Debug formatting;
- zeroization and bounded in-memory lifetime.

**Verify:** published test vectors, wrong key/AAD, bit flips, reorder/duplicate/truncate/splice and nonce-reuse prevention tests.

### Slice 2: Implement streaming encrypted blob storage

Add encrypted blob writer/reader integrated with internal/external placement without plaintext staging on disk. Generate random blob IDs independent of content. Authenticate metadata and chunks before exposing plaintext to consumers. Crash before final commit leaves no referenced partial blob; reconciliation safely removes staging.

Ordinary dedup remains unchanged; Private/sensitive storage cannot use plaintext digest or cross-event equality by default.

**Verify:** both backends, empty/large/chunked payloads, forced termination at each phase, corruption and authentication-before-release tests.

### Slice 3: Build key service and lock lifecycle

Implement explicit states: unavailable/uninitialized, locked, unlocking, unlocked, rotation/recovery required, error. Cover:

- first initialization and recovery material ceremony;
- unlock with user-scope protection and optional later presence provider;
- inactivity auto-lock, Windows session lock, sign-out and process shutdown clearing;
- key rotation with restart-safe journal;
- recovery verification before accepting Private capture;
- wrong-user and malformed material fail closed.

Key operations run in the resident security boundary and expose content-free status over authorized IPC.

**Verify:** process/session/wrong-user tests, memory zeroization assertions where observable, restart during rotation/recovery, no plaintext/key logs.

### Slice 4: Enable Private only after all mandatory gates

Add the built-in Private profile as unavailable configuration until encrypted storage, random identity, non-indexing, hidden view model, lock/key-clear and recovery tests all pass. Once enabled:

- no ordinary FTS/preview/duplicate/export;
- shorter configurable retention and immediate clear;
- hidden row placeholders until explicit authorized unlock/reveal;
- no network integrations;
- replay requires current authorization and destination revalidation;
- lock immediately clears manager/Quick Paste/overlay caches and subscriptions.

**Verify:** canary content never appears in DB plaintext, FTS, freelist/journal policy samples, logs, UIA, thumbnails, screenshots or diagnostics.

### Slice 5: Define versioned profile policy and built-ins

Implement `General`, `Development`, `Study`, `Work`, `Design`, and gated `Private` with stable IDs, schema versions and explicit defaults for:

- capture allow/deny and source resolution;
- retention/quota;
- sensitive policy;
- indexing/preview/export;
- default replay representation/action;
- overlay behavior;
- encryption/lock;
- auto-switch disabled by default unless explicit rule exists.

Profiles are agent-owned and enforced during capture/search/reveal/replay/export/cleanup. Manager color/icon/name are presentation fields, never sole policy indicators.

**Verify:** migration/golden fixtures and enforcement tests at every boundary.

### Slice 6: Implement deterministic rule facts/actions

Define a closed versioned rule model with bounded predicates over reliable facts (source executable/package, broad content/representation type, active profile, explicit user tags/project evidence when available) and actions (deny, choose profile, select representation, transform through registered safe action, pin/tag where implemented). Rules cannot override source hard deny or hidden sensitive policy.

Evaluation order: privacy precedence → enabled scope → priority → specificity → stable rule ID. Equal-priority destructive conflicts are errors and cannot execute.

**Verify:** property/golden tests for order independence, conflict, unknown versions/enums, bounds and privacy precedence.

### Slice 7: Add simulation, explanation and content-free audit

Every evaluation returns matched facts, rejected facts, selected action, overridden lower-priority rules and consequences without hidden payload. Simulation uses selected/synthetic events and cannot mutate state. Audit records create/edit/reorder/enable/match/result/undo/rollback with actor/process/schema version and no payload.

**Verify:** simulation equals real evaluation for identical facts; audit contains no content canary.

### Slice 8: Add remember-choice recommendations safely

Observe repeated explicit choices only as bounded content-free context/action counts. Offer a narrow recommendation after a documented threshold; never silently create/enable a broad rule. Show exact scope/action/privacy consequences and allow edit/decline/disable. Do not use ML or retain payload snippets.

**Verify:** no recommendation on sensitive/private/hard-denied events; consent and scope tests pass.

### Slice 9: Expose secure IPC/provider operations

Add versioned operations for profile list/detail/update/switch, key state/unlock/lock/rotate/recover, rule CRUD/order/simulate/audit and authorized Private reveal/replay. Sensitive operations require explicit user-intent token/presence as designed, not connection alone. Return redacted DTOs while locked.

Use bounded caller-owned buffers or authorized bulk staging; never send keys or unrestricted payloads.

**Verify:** unauthorized/stale/replayed/locked/mismatch cases fail closed; adjacent-version fixtures pass.

### Slice 10: Build Manager profile/rule/privacy workflows

Implement provider-backed pages/forms following `docs/ux/manager.md`:

- Profiles list/detail with consequences and gated Private explanation;
- lock/unlock/recovery/rotation flows with clear destructive boundaries;
- Rules list/editor/simulation/conflict/audit/rollback;
- auto-switch explanation/undo;
- accessibility/localization/long-copy/high-contrast states;
- no JSON editing required for normal users.

Plan 009 integrates these pages into the complete IA; this slice owns domain-correct workflows.

## Test plan

- encryption vectors, corruption/reorder/truncation/splice/wrong key/AAD;
- internal/external encrypted crash recovery and random identity;
- DPAPI/user/session lock/unlock/rotation/recovery/wrong user;
- plaintext canary scans across files/DB/FTS/logs/UIA/screenshots/diagnostics;
- profile migration and enforcement at capture/search/replay/export/cleanup;
- rule precedence/conflict/simulation/audit/rollback;
- remember-choice consent and privacy exclusion;
- authorized IPC and stale/replay/locked behavior;
- full keyboard/Narrator/reduced-motion/300% scaling secure workflows.

## Done criteria

- [ ] Private payloads are authenticated-encrypted from first persistence and use random non-equality-revealing identity.
- [ ] No Private plaintext digest/index/preview/cache/export exists by default.
- [ ] Lock/session/shutdown clears keys and protected presentation state.
- [ ] Recovery and rotation are restart-safe and tested before Private activation.
- [ ] Profiles are versioned and enforced by service logic at every relevant boundary.
- [ ] Rules are deterministic, explainable, reversible and cannot override privacy precedence.
- [ ] Sensitive operations require explicit authorization beyond connection authentication.
- [ ] Canary and full baseline/security tests pass.

## STOP conditions

Stop and obtain security review if:

- Algorithm/envelope/key hierarchy is not fully specified before implementation.
- Any path releases plaintext before authentication succeeds.
- Private storage requires persistent plaintext digest, ordinary dedup, FTS or preview cache.
- Key bytes cross ordinary IPC or appear in logs/dumps/debug formatting.
- A rule can override source hard deny, sensitive skip or lock state.
- Recovery cannot be tested without risking real user material; use fixtures only.
- A third-party crypto dependency lacks current maintenance/advisory/license review.

## Maintenance notes

Every new Private representation, export, integration, preview or automation requires threat-model review and canary tests. Rule schema changes need version fixtures and deterministic ordering proof. UI state must be derived from service authorization, never local visibility toggles alone.
