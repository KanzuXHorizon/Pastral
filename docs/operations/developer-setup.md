# Windows developer setup

## Current implemented scope

These commands build and test the Rust foundation currently present in the repository:

- `pastral-domain` — platform-independent immutable domain types and invariants;
- `pastral-storage` — synchronous single-owner SQLite/FTS5 metadata, ordinary internal/external blob persistence, literal lexical search, content-free capture audit storage, deletion, integrity checks, and bounded reconciliation.

The repository still does not contain Win32 clipboard capture, COM/OLE, IPC, executables, WinUI, encryption, packaging, installer, telemetry, OCR, semantic search, or AI product code.

## Required environment

- Windows PowerShell 5.1 or newer.
- Git.
- rustup.
- Rust `1.97.1-x86_64-pc-windows-msvc` with `rustfmt`, `clippy`, and target `x86_64-pc-windows-msvc`.
- Visual Studio 2022 Build Tools with the MSVC x64 compiler/linker toolset.

Windows SDK `10.0.28000.2526` remains deferred to a future native slice. The verifier reports installed SDKs but does not fail the Rust foundation merely because that exact deferred SDK is absent.

SQLite does not need a separate machine installation. `rusqlite 0.40.1` is pinned with the `bundled` and `blob` features, and `libsqlite3-sys` builds the reviewed bundled SQLite source through the MSVC toolchain. The dependency graph includes build-helper crates such as `cc`, `pkg-config`, and `vcpkg`; no external vcpkg installation or repository vcpkg manifest is required.

## Install the pinned Rust toolchain

The repository toolchain file selects the pinned version without changing the global rustup default:

```powershell
rustup toolchain install 1.97.1-x86_64-pc-windows-msvc `
  --profile minimal `
  --component rustfmt `
  --component clippy `
  --target x86_64-pc-windows-msvc
```

## Verify the environment

From the repository root:

```powershell
Set-Location F:\Pastral
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\eng\verify-toolchain.ps1
```

The command reports exact `rustc`, `cargo`, rustup active toolchain, host target, `rustfmt`, Clippy, Visual Studio/MSVC linker evidence, and installed Windows SDK versions. A required current-slice mismatch returns a nonzero exit code.

## Build and test

Run the full foundation quality gate:

```powershell
.\eng\build.ps1 -Task All
```

Individual tasks:

```powershell
.\eng\build.ps1 -Task Verify
.\eng\build.ps1 -Task Format
.\eng\build.ps1 -Task Check
.\eng\build.ps1 -Task Test
.\eng\build.ps1 -Task Storage
.\eng\build.ps1 -Task Clippy
.\eng\build.ps1 -Task Doc
.\eng\build.ps1 -Task Dependencies
```

The script stops at the first failure and preserves the failing command's exit code. `Storage` is a focused storage-crate test task; `Test` already covers every workspace crate.

## Direct CI-equivalent commands

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo test -p pastral-storage --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
.\eng\verify-dependencies.ps1
cargo tree --workspace
```

## SQLite runtime policy for Phase 2A

Every storage open verifies:

- schema version `1`;
- `foreign_keys=ON`;
- `journal_mode=DELETE`;
- `synchronous=FULL`;
- FTS5 availability;
- a bounded busy timeout.

WAL is intentionally not enabled in this slice. The pinned bundled SQLite reports `3.53.2`, while SQLite `3.53.3` includes a WAL-reset corruption fix. A future dependency update plus Windows crash/backup/AV/low-disk benchmarks must pass before Pastral changes journal mode. This rollback-journal selection is a correctness baseline, not a claim that it is the final fastest configuration.

## Storage test data

Storage unit and integration tests create synthetic disposable roots under the current user's temporary directory. They never access a future default Pastral user-data root. Test cleanup is best effort; an interrupted process may leave a directory named `pastral-storage-test-<uuid>` in the system temporary directory, which contains synthetic test data only.

## Dependency rationale

- `uuid = 1.18.1`: typed UUIDv4 generation through the operating-system CSPRNG path, validation, binary UUID storage, and canonical textual boundaries.
- `sha2 = 0.10.9`: SHA-256 implementation for the explicitly versioned `sha256-raw-v1` logical-byte digest.
- `rusqlite = 0.40.1`: reviewed synchronous SQLite wrapper. Only `bundled` and `blob` are enabled; default features and unrelated integration features are disabled.

`Cargo.lock` is committed. `eng/verify-dependencies.ps1` rejects async runtimes, serialization frameworks, alternate database stacks, Protobuf/IPC runtimes, logging backends, network clients, Windows bindings, and UI dependencies from the current foundation graph.

## Current storage limitations

- Only ordinary protection-domain payloads are accepted. Sensitive and Private plaintext is rejected before any payload or search projection is persisted.
- Blob placement is selected by a caller-supplied versioned policy. No benchmark-selected production threshold exists yet.
- Inputs are bounded owned byte buffers; Win32/OLE streaming acquisition is not implemented.
- No encryption, backup/restore, import/export, backend migration, retention/quota engine, multi-process ownership, or background maintenance exists.
- FTS search is bounded literal lexical matching with deterministic ordering; no snippets, typo correction, semantic ranking, or sensitive indexing is provided.
