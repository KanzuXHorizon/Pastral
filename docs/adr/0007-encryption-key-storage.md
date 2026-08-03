# ADR 0007: Authenticated payload encryption with DPAPI-protected root keys

**Status:** Accepted for sensitive/private storage
**Date:** 2026-08-03

## Context

Clipboard history can contain credentials and confidential material. Pastral defaults to not storing highly confident secrets, but users may explicitly enable narrow encrypted retention and private profiles. Encryption must protect payload confidentiality and integrity without inventing cryptography or requiring an account.

## Decision

Use a versioned envelope design:

1. Generate root and data-encryption keys with a Windows CSPRNG.
2. Protect the installation/profile root key using user-scope Windows DPAPI; do not use machine scope.
3. Encrypt sensitive payloads with a vetted authenticated-encryption implementation selected during security-module bootstrap.
4. Use independent random nonces according to the chosen algorithm's requirements.
5. Bind envelope version, clip/representation identity, profile identity, and content length as authenticated associated data where appropriate.
6. Store random blob identifiers or keyed hashes so filenames and ordinary indexes do not reveal plaintext equality.
7. Version algorithm, key identifier, nonce, associated-data schema, and ciphertext length in the envelope.
8. Support key rotation by rewrapping root keys when possible and background re-encryption only through an explicit, interruptible migration.
9. Zero plaintext/key buffers where practical while documenting compiler, allocator, paging, crash-dump, and GPU limitations.

Sensitive data remains excluded from FTS, OCR, semantic indexing, duplicate stacks, passive previews, and normal diagnostic exports.

## Consequences

Positive:

- local key protection integrates with the Windows user account;
- authenticated encryption detects tampering;
- algorithm and key rotation can evolve through envelope versions;
- equality leakage is reduced for sensitive blobs.

Costs:

- DPAPI-bound keys are not automatically portable to another device/account;
- administrative credential reset and profile corruption can make data unrecoverable;
- metadata not separately encrypted may still expose time, size bucket, source class, and policy state;
- secure deletion on modern storage cannot be guaranteed by overwriting files.

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
- key compromise, rotation, or recovery testing exposes a design flaw.
