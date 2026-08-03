# Encryption and key-management design

**Status:** Architecture requirement; concrete library/algorithm selection occurs in the security-module design with official documentation and independent review.

## 1. Scope

Encryption applies to:

- opt-in sensitive clip payloads;
- Private-profile payloads according to profile policy;
- installation secrets used for IPC authentication;
- portable backups only after a separate portable-key design.

Ordinary profile content is local but is not represented as encrypted by default in Phase 0. Device-level encryption such as BitLocker remains recommended.

## 2. Key hierarchy

```text
Windows user credentials
└─ DPAPI-protected Installation Root Key (IRK)
   ├─ Profile Key Encryption Key (PKEK) per encrypted profile
   │  └─ Data Encryption Key (DEK) per payload or bounded payload group
   └─ IPC Authentication Secret
```

Requirements:

- keys originate from a cryptographically secure Windows RNG;
- DPAPI uses user scope, not machine scope;
- background DPAPI calls forbid UI prompts;
- profile/DEK derivation or wrapping uses a reviewed standard construction;
- compromise of one DEK must not expose unrelated payloads;
- key identifiers are random/non-sensitive and versioned;
- plaintext keys are never written to logs, crash diagnostics, database text fields, or command output.

## 3. Envelope format

Each encrypted blob uses a strict versioned binary envelope containing:

- magic and envelope version;
- algorithm suite identifier;
- key identifier/version;
- nonce length and nonce;
- authenticated associated-data schema version;
- plaintext length with validated upper bound;
- ciphertext length with validated upper bound;
- ciphertext and authentication tag or algorithm-defined combined output.

Authenticated associated data should bind, where stable:

- installation/profile identifier;
- clip and representation identifiers;
- representation format identifier;
- transformation version for derived encrypted content;
- declared plaintext length;
- envelope version.

Parsing validates all arithmetic before allocation and rejects unknown versions/suites safely.

## 4. Algorithm selection criteria

The implementation plan must select a vetted, maintained authenticated-encryption implementation that supports:

- streaming or bounded-memory processing for large payloads;
- misuse-resistant nonce handling or an enforceable unique-nonce design;
- constant-time authentication behavior in the library;
- Windows x64 and later ARM64;
- stable Rust support and security advisory process;
- clear license and supply-chain review;
- test vectors and fuzzable envelope parsing.

Do not implement cryptographic primitives locally.

## 5. Write path

1. Authorize the sensitive/private retention policy.
2. Generate or obtain the correct DEK.
3. Create a random staging filename in a restricted directory.
4. Encrypt while streaming from captured memory/input stream to staging output.
5. Compute only permitted ciphertext/integrity metadata; do not persist plaintext hashes for sensitive clips.
6. Flush/close according to durability policy.
7. Atomically move to a random/keyed final identifier.
8. Commit envelope/blob reference and policy metadata in SQLite.
9. Zero transient plaintext/key buffers where practical.
10. Recovery removes incomplete staging files without attempting to expose partial plaintext.

## 6. Read path

1. Authenticate and authorize profile/item access.
2. Load and validate envelope header/lengths.
3. Resolve and unwrap the required key.
4. Decrypt into a bounded protected buffer or stream directly to the consumer.
5. Verify authentication before presenting usable output where the API permits.
6. Avoid caching plaintext beyond the operation.
7. Zero/release plaintext and key material after use.
8. On authentication failure, quarantine metadata/blob association and report a content-free integrity error.

## 7. Key rotation

Rotation types:

- **DPAPI rewrap:** protect the same IRK again after credential/security-policy changes where safe.
- **Profile key rotation:** create a new PKEK version; new writes use it while old keys remain for reads.
- **Payload re-encryption:** interruptible migration to a new DEK/algorithm suite.

Rules:

- never delete an old key until every referenced envelope is migrated or intentionally deleted;
- migration records progress transactionally and resumes after crash;
- preserve immutable content semantics: ciphertext may change, plaintext original identity does not;
- benchmark and power-failure test large migrations;
- provide explicit recovery/export guidance before irreversible rotation.

## 8. Locking and Windows Hello

DPAPI user-scope protection does not mean the application is locked while the Windows session is unlocked.

Private-profile UX may later require Windows Hello/user verification before revealing or pasting. That capability needs a separate design covering:

- verification freshness;
- fallback/recovery;
- session lock/unlock;
- headless agent behavior;
- accessibility;
- credential provider and platform API limitations.

Until then, encrypted sensitive retention can be configured to clear decrypted keys on session lock and require explicit profile unlock through the manager/Quick Paste flow.

## 9. Metadata exposure

Unless a separate encrypted-metadata design is accepted, an attacker reading ordinary database files may learn:

- event time or coarse time bucket;
- encrypted payload size or size bucket;
- profile/key identifier;
- source class if retained;
- retention/pinned state;
- that sensitive content was stored or skipped.

The UI must describe this honestly. Sensitive profiles should minimize or encrypt metadata when later requirements justify the complexity.

## 10. Backup and recovery

- Raw DPAPI-bound encrypted blobs may be unusable after moving to another Windows user/device.
- Copying the database and blobs is not a complete portable backup guarantee.
- Portable encrypted backup requires an export-specific key hierarchy, strong KDF or platform credential, authentication, recovery UX, and separate threat model.
- Administrative password reset or damaged Windows profile can make DPAPI material unrecoverable.
- Pastral must never imply vendor recovery exists when no server-held recovery key exists.

## 11. Verification

Required tests:

- official algorithm test vectors;
- envelope round trip and version rejection;
- nonce uniqueness/handling;
- associated-data mismatch;
- bit-flipped header/ciphertext/tag;
- truncated and oversized fields;
- wrong user/DPAPI context;
- key rotation interrupted at every persisted phase;
- crash/power interruption during encrypted write;
- log and diagnostic canary scan;
- plaintext absence in ordinary database/blob filenames and FTS;
- memory inspection during development to minimize avoidable copies.
