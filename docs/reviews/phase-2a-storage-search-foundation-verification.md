# Phase 2A storage and lexical-search foundation verification

**Date:** 2026-08-04
**Baseline commit:** `6d5f3b6`
**Verified implementation commit before this report:** `99f5525`
**Branch:** `main`

## 1. Environment

- Repository: `F:\Pastral`
- OS: Microsoft Windows NT `10.0.26200.0`
- Shell: Windows PowerShell `5.1.26100.8972`
- Rust host/target: `x86_64-pc-windows-msvc`
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`
- rustup active toolchain: `1.97.1-x86_64-pc-windows-msvc`, selected by `rust-toolchain.toml`; the global default was not changed
- rustfmt: `1.9.0-stable (8bab26f4f6 2026-07-14)`
- Clippy: `0.1.97 (8bab26f4f6 2026-07-14)`
- Visual Studio: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`
- MSVC linker: `VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
- Installed Windows SDK: `10.0.26100.0`
- Deferred native-slice Windows SDK `10.0.28000.2526`: not installed; intentionally non-blocking for this Rust storage slice
- rusqlite: `0.40.1`
- libsqlite3-sys: `0.38.1`
- Bundled SQLite runtime: `3.53.2`

## 2. Delivered scope

Phase 2A adds a synchronous, single-owner storage/search foundation while preserving the platform-independent domain boundary.

Implemented:

- `pastral-storage` with `#![forbid(unsafe_code)]`.
- SQLite schema version `1` using strict tables, explicit enum tags, UUID bytes, foreign keys, and bounded busy timeout.
- Required runtime policy:
  - `foreign_keys = ON`;
  - `journal_mode = DELETE`;
  - `synchronous = FULL` (`2`);
  - FTS5 available;
  - newer unknown schema versions rejected.
- Caller-supplied versioned blob-placement policy with no invented production threshold.
- Internal SQLite BLOB storage, including incremental BLOB writes for nonempty payloads.
- Controlled external-file storage with create-new staging, flush, `sync_all`, rename, safe relative locators, and digest-suite-namespaced paths.
- Ordinary-payload deduplication only within the exact protection-domain ID, digest suite, full digest, and raw length.
- Physical readback plus exact-byte comparison before dedup reuse, preventing aliasing to a missing, corrupt, or theoretically colliding existing object.
- Immutable clip-event and representation metadata persistence and reconstruction.
- Domain-level enforcement that every captured representation belongs to the clip event's captured protection domain.
- Read/load integrity checks for event, representation, blob domain, digest suite, digest, length, aggregate fidelity, and configured payload limit.
- Explicit rejection of Sensitive and Private plaintext before payload or search-projection persistence.
- Bounded literal FTS5 search with bound parameters, deterministic ordering, and no payload-derived snippets.
- Content-free `CaptureAuditEvent` persistence with schema inspection tests for forbidden column classes.
- Transactional event/search/representation deletion and blob reference accounting.
- Pending external deletion with retry through reconciliation.
- Bounded reconciliation by actual filesystem/database entry budget, not only returned file count.
- SQLite quick-check, FTS5 integrity-check, metadata relationship checks, search-row mapping checks, reference-count checks, and corruption-reporting tests.
- Lockfile-enforced build, test, Clippy, documentation, and dependency commands.
- Dependency-policy and source-policy PowerShell gates.
- Windows CI, README, changelog, design, plan, and developer setup updates.

Not implemented:

- Win32 clipboard capture, COM/OLE, delayed rendering, format adapters, or clipboard fixtures;
- IPC, named pipes, Protobuf, agent/worker/CLI executables, or multi-process ownership;
- encryption, DPAPI, key envelopes, Sensitive/Private storage, or Private-profile activation;
- backup/restore, import/export, retention/quota, backend migration, annotations, tags, or profiles beyond captured identifiers;
- WinUI/C++, installer, packaging, signing, update delivery, telemetry, OCR, semantic search, AI, or product network access;
- WAL, a production placement threshold, or a benchmark claim.

## 3. SQLite runtime evidence

The focused test below was freshly run against `99f5525`:

```powershell
cargo test --locked -p pastral-storage schema::tests::schema_creation_and_reopen_preserve_version_and_pragmas -- --exact --nocapture
```

Observed output:

```text
SQLite runtime: 3.53.2
1 passed; 0 failed
```

The test also verifies:

- schema version `1`;
- journal mode `delete`;
- synchronous value `2` (`FULL`);
- foreign keys enabled;
- FTS5 enabled;
- reopening does not recreate or reinterpret the schema.

Rollback journal is intentionally selected. The pinned bundled runtime is SQLite `3.53.2`, while SQLite `3.53.3` contains a later WAL-reset corruption fix. Phase 2A therefore makes no WAL durability or performance claim and requires an explicit dependency update plus crash/backup/antivirus/low-disk evidence before a journal-mode change.

## 4. Dependency tree

Fresh `cargo tree --locked --workspace` output contains:

```text
pastral-domain v0.1.0
├── sha2 v0.10.9
│   ├── cfg-if v1.0.4
│   ├── cpufeatures v0.2.17
│   └── digest v0.10.7
│       ├── block-buffer v0.10.4
│       │   └── generic-array v0.14.7
│       │       └── typenum v1.20.1
│       │       [build-dependencies]
│       │       └── version_check v0.9.5
│       └── crypto-common v0.1.7
└── uuid v1.18.1
    └── getrandom v0.3.4
        └── cfg-if v1.0.4

pastral-storage v0.1.0
├── pastral-domain v0.1.0
├── rusqlite v0.40.1
│   ├── bitflags v2.13.1
│   ├── fallible-iterator v0.3.0
│   ├── fallible-streaming-iterator v0.1.9
│   ├── libsqlite3-sys v0.38.1
│   │   [build-dependencies]
│   │   ├── cc v1.4.0
│   │   │   ├── find-msvc-tools v0.1.9
│   │   │   └── shlex v2.0.1
│   │   ├── pkg-config v0.3.33
│   │   └── vcpkg v0.2.15
│   └── smallvec v1.15.2
└── uuid v1.18.1
```

`cc`, `pkg-config`, and `vcpkg` are build helpers reachable through the bundled SQLite source build. No repository vcpkg manifest or machine-installed vcpkg dependency was added.

The dependency gate found no `tokio`, `serde`, `chrono`, `time`, `sqlx`, `prost`, `protobuf`, `tracing`, `log`, `reqwest`, `hyper`, `windows`, `windows-sys`, Windows App SDK, WinUI, or other prohibited current-foundation package.

## 5. Test inventory and fresh results

Fresh workspace test result:

- `pastral-domain`: 26 passed, 0 failed, 0 ignored.
- `pastral-storage`: 30 passed, 0 failed, 0 ignored.
- Total: **56 passed, 0 failed, 0 ignored**.

Storage coverage includes:

- schema creation/reopen and newer-schema rejection;
- exact bundled SQLite runtime and pragma verification;
- internal/external exact-byte round trips, including empty payloads;
- payload mapping cardinality, unknown/duplicate IDs, length mismatch, and digest mismatch;
- same-domain ordinary dedup without event merging;
- cross-domain dedup isolation;
- corrupt/missing existing physical blob rejection before dedup reuse;
- external staging cleanup after failed reuse;
- Sensitive/Private plaintext rejection before persistence;
- literal handling of `OR`, `NOT`, quotes, `*`, and parentheses;
- explicit-only indexing and deterministic capture-order search results;
- FTS mapping cleanup when deleting a clip;
- content-free audit schema/API;
- shared internal reference deletion;
- external deletion retry;
- stale staging and unreferenced-object cleanup;
- missing external object reporting without metadata destruction;
- true reconciliation entry-budget truncation, including directory traversal;
- SQLite/FTS integrity success;
- metadata/search-mapping corruption detection;
- blob metadata, aggregate-fidelity, protection-domain, length, and configured-limit tamper rejection on read/load.

Focused integrity commands were also freshly run:

```powershell
cargo test --locked -p pastral-storage maintenance::tests::sqlite_and_fts_integrity_checks_pass_after_operations -- --exact --nocapture
cargo test --locked -p pastral-storage maintenance::tests::integrity_check_reports_metadata_and_search_mapping_corruption -- --exact --nocapture
```

Both passed independently.

## 6. TDD red/green evidence

### Recorded red evidence

The first storage test command was run before a Rust target existed:

```powershell
cargo test -p pastral-storage --all-targets
```

It failed with `no targets specified in the manifest`. This established the initial red state before `src/lib.rs`, schema, and storage contracts were implemented. The failure was not committed.

During implementation, additional quality-gate failures were observed and corrected before commits:

- compilation failures for incomplete UUID/storage API integration;
- rustfmt differences across newly added modules;
- Clippy failures for unused/test-only symbols and `manual_contains`;
- Clippy `type_complexity` failure for a reconciliation return type;
- a missing digest-suite namespace in external object keys identified during final scope review;
- integrity-review gaps identified for bounded allocation, blob/representation/event metadata agreement, exact-byte dedup reuse, FTS mapping validation, and actual directory-entry reconciliation limits.

Each correction was followed by the focused test and then workspace gates before its green commit.

### Green evidence

The final fresh runs against `99f5525` passed:

- `eng/build.ps1 -Task All`;
- `eng/build.ps1 -Task Storage`;
- direct format/check/test/Clippy/doc commands using `--locked`;
- dependency policy;
- source policy;
- focused SQLite runtime and integrity tests;
- Git integrity checks.

Strict per-test red output was not separately captured for every behavior test. Several tests and their minimal implementation were developed within one eventual green batch, while the observed compile/format/Clippy failures and focused green reruns were recorded. The repository contains no intentionally failing commit.

## 7. Fresh commands and results

All commands below were run against `99f5525` before writing this report.

| Command | Result |
|---|---|
| `git status --short` | PASS; no tracked or untracked changes before report creation |
| `git branch --show-current` | `main` |
| `git rev-parse --short HEAD` | `99f5525` |
| PowerShell/OS version query | PowerShell `5.1.26100.8972`; Windows NT `10.0.26200.0` |
| `eng/verify-toolchain.ps1` | PASS |
| `eng/build.ps1 -Task All` | PASS; 56 tests, Clippy/doc/dependency/source policy all passed |
| `eng/build.ps1 -Task Storage` | PASS; 30 storage tests |
| `eng/build.ps1 -Task SourcePolicy` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --locked --workspace --all-targets` | PASS |
| `cargo test --locked --workspace --all-targets` | PASS; 56 passed, 0 failed, 0 ignored |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo doc --locked --workspace --no-deps` | PASS |
| `cargo tree --locked --workspace` | PASS; tree recorded above |
| `eng/verify-dependencies.ps1` | PASS |
| `eng/verify-source-policy.ps1` | PASS |
| focused SQLite runtime/schema/pragma test | PASS; SQLite `3.53.2` |
| focused SQLite/FTS integrity success test | PASS |
| focused metadata/search corruption-detection test | PASS |
| `git diff --check` | PASS |
| `git fsck --no-progress --no-dangling` | PASS |
| `git show --check HEAD` | PASS |

A failed Bash attempt to combine PowerShell-specific `$LASTEXITCODE` syntax was discarded before PowerShell execution. It produced no repository changes and was replaced by independent commands with explicit exit codes.

## 8. Source and secret policy evidence

`eng/verify-source-policy.ps1` freshly passed. It checks tracked text/source and rejects:

- common AWS/GitHub token and private-key signatures;
- tracked `.pfx`, `.p12`, `.pem`, `.key`, `.env*`, `target/`, and the machine-local `Start-DevSpace-MCP-Cloudflared.ps1`;
- unsafe blocks/functions/traits/impls/extern boundaries in product crates;
- `std::net` and process-spawning APIs in product crates;
- SQLite extension loading, `ATTACH DATABASE`, and WAL activation in product crates.

This is a bounded signature/policy scan, not a substitute for a full credential-history scanner, dependency advisory database, malware scanner, or formal static analysis.

## 9. Files changed from baseline

Before this report, `git diff --stat 6d5f3b6..99f5525` reported:

- 29 files added or modified;
- 4,762 insertions;
- 48 deletions.

Major groups:

- Workspace/dependencies: `Cargo.toml`, `Cargo.lock`.
- Domain hardening: five `crates/domain` modules/exports.
- New storage crate: manifest plus 12 focused Rust modules.
- CI/build policy: workflow, `eng/build.ps1`, dependency scan, source scan.
- Documentation: design, plan, developer setup, README, and changelog.

No empty placeholder tree, CMake/vcpkg manifest, executable, installer, `LICENSE`, or machine-local launcher was committed.

## 10. Local commits

- `90dc8f2 docs: specify storage and lexical search foundation`
- `dd0f0eb feat: add storage and lexical search foundation`
- `ac2a741 ci: verify storage foundation`
- `33e829b test: verify bundled SQLite runtime`
- `4668b7b fix: namespace external blobs by digest suite`
- `470d91c fix: harden storage integrity and reconciliation`
- `99f5525 ci: lock and scan foundation inputs`
- Final evidence commit follows this report.

No remote push, pull request, merge, history rewrite, or squash was performed.

## 11. Skipped checks

- GitHub-hosted `windows-2025` CI was not executed remotely because no push or pull request was authorized. The workflow's commands were run locally on the actual Windows machine.
- No dependency vulnerability/advisory service, license-compatibility scanner, SBOM generator, or provenance signer was added or run.
- No crash injection at every staging/write/sync/rename/SQLite-commit boundary was run.
- No real power-loss, low-disk, antivirus-lock, backup-agent, roaming-profile, RDP, or multi-user matrix was run.
- No performance benchmark selected an internal/external placement threshold.
- No WAL/checkpoint benchmark or durability test was run; WAL remains disabled.
- No encryption, DPAPI, backup/restore, import/export, retention/quota, or backend migration test was run because those features do not exist in this slice.
- No Win32 clipboard, COM/OLE, IPC, process, UI, accessibility, packaging, signing, installer, or update test was run because those systems remain outside Phase 2A.
- No fuzzing, property-testing framework, model checking, or formal verification was added.

## 12. Residual risks and unproven claims

- The GitHub Actions workflow remains unproven on the hosted runner until an authorized remote run.
- SQLite `3.53.2` is intentionally pinned and verified, but it predates the SQLite `3.53.3` WAL-reset fix. WAL is disabled; the next SQLite upgrade requires explicit compatibility and recovery evidence.
- External-file finalization and SQLite metadata commit are coordinated but cannot be one cross-filesystem/database atomic primitive. Bounded reconciliation and rollback cleanup are tested, but a complete crash-at-every-instruction matrix is not proven.
- `Storage` is type-level single-owner (`!Send`/`!Sync`), but no cross-process first-instance lock or file lock exists yet.
- The data root is assumed to be application-owned. Same-user malware, hostile reparse points, and filesystem tampering are not claimed to be an enclave boundary.
- Reconciliation intentionally does not hash every ready payload. Reads and dedup reuse verify exact bytes on demand; a future explicit scrub operation may be required.
- The source-policy scan is signature/pattern based and does not inspect historical Git objects or binary files for secrets.
- No production storage limits or internal/external placement threshold have been selected by benchmark evidence.
- No performance, scale, long-running fragmentation, database-vacuum, backup, or retention claim is made.
- Sensitive and Private storage remain unavailable because authenticated encryption and key management are not implemented.
- Windows capture fidelity, IPC security/compatibility, UI behavior, packaging, accessibility, and release readiness remain entirely unproven by Phase 2A.
