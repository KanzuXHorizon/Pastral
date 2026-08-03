# ADR 0007: Authenticated payload encryption with DPAPI-protected root keys

**Status:** Accepted for sensitive/private storage
**Date:** 2026-08-03
**Amended:** 2026-08-04 — clarify DPAPI's same-user limitation, random sensitive identity, and authentication-before-plaintext-release requirements after the Phase 0 adversarial audit.

## Context

Clipboard history can contain credentials and confidential material. Pastral defaults to not storing highly confident secrets, but users may explicitly enable narrow encrypted retention and private profiles. Encryption must protect payload confidentiality and integrity without inventing cryptography or requiring an account.

## Decision

Use a versioned envelope design:

1. Generate root and data-encryption keys with a Windows CSPRNG.
2. Protect the installation/profile root key using user-scope Windows DPAPI; do not use machine scope. This protects under the Windows account model but is not represented as a same-user malware boundary.
3. Encrypt sensitive payloads with a vetted authenticated-encryption implementation selected during security-module bootstrap.
4. Use independent random nonces according to the chosen algorithm's requirements.
5. Bind envelope version, clip/representation identity, profile identity, content length, and—when chunked—chunk index/count and object identity as authenticated associated data.
6. Use random blob identifiers and no persistent plaintext digest/deduplication by default for Private/sensitive payloads. A keyed equality scheme requires a later explicit privacy decision and migration design.
7. Version algorithm, key identifier, nonce, associated-data schema, and ciphertext length in the envelope.
8. Support key rotation by rewrapping root keys when possible and background re-encryption only through an explicit, interruptible migration.
9. Never release unauthenticated plaintext: whole-message AEAD buffers within a strict bound until final authentication, or each independently authenticated chunk verifies before release.
10. Zero plaintext/key buffers where practical while documenting compiler, allocator, paging, crash-dump, and GPU limitations.

Sensitive data remains excluded from FTS, OCR, semantic indexing, duplicate stacks, passive previews, and normal diagnostic exports.

## Consequences

Positive:

- local key protection integrates with the Windows user account;
- authenticated encryption detects tampering;
- algorithm and key rotation can evolve through envelope versions;
- default random blob identity avoids persistent plaintext equality leakage between sensitive/Private payloads.

Costs:

- DPAPI-bound keys are not automatically portable to another device/account;
- administrative credential reset and profile corruption can make data unrecoverable;
- metadata not separately encrypted may still expose time, size bucket, source class, and policy state;
- secure deletion on modern storage cannot be guaranteed by overwriting files;
- code already running as the same unlocked user may be able to invoke user-scope DPAPI or observe decrypted content, so encryption is not a secure enclave.

## Alternatives considered

- **DPAPI directly on every large payload:** simple but couples all data to one API envelope and can be inefficient for streaming/rotation.
- **Machine-scope DPAPI:** rejected because other users on the machine may decrypt.
- **User password-derived key only:** increases lockout, KDF, recovery, and UX risk; may be added only through a separate vault ADR.
- **Custom cryptography:** prohibited.

## Review triggers

- authenticated-encryption library selection or vulnerability;
- Windows Hello/private-vault design;
- portable encrypted backup requirements;
- private metadata needs stronger confidentiality;
- key compromise, rotation, authentication-before-release, or recovery testing exposes a design flaw;
- a proposal introduces plaintext/keyed equality indexes or cross-protection-domain deduplication.
