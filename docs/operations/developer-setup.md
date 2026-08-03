# Phase 1 Windows developer setup

## Scope

These commands build only the pure Rust `pastral-domain` workspace slice. They do not install or build WinUI, Windows App SDK, Protobuf, SQLite, packaging, or native clipboard code.

## Required environment

- Windows PowerShell 5.1 or newer.
- Git.
- rustup.
- Rust `1.97.1-x86_64-pc-windows-msvc` with `rustfmt`, `clippy`, and target `x86_64-pc-windows-msvc`.
- Visual Studio 2022 Build Tools with the MSVC x64 compiler/linker toolset.

Windows SDK `10.0.28000.2526` is deferred to a future native slice. The Phase 1 verifier reports installed SDKs but does not fail merely because that exact deferred SDK is absent.

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

Run the full Phase 1 quality gate:

```powershell
.\eng\build.ps1 -Task All
```

Individual tasks are available when developing a focused change:

```powershell
.\eng\build.ps1 -Task Verify
.\eng\build.ps1 -Task Format
.\eng\build.ps1 -Task Check
.\eng\build.ps1 -Task Test
.\eng\build.ps1 -Task Clippy
.\eng\build.ps1 -Task Doc
```

The script stops at the first failure and preserves the command exit code.

## Direct CI-equivalent commands

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
cargo tree --workspace
```

## Dependency rationale

- `uuid = 1.18.1`: UUIDv4 generation through the crate's operating-system CSPRNG path, binary UUID storage, validation, and canonical hyphenated textual boundaries.
- `sha2 = 0.10.9`: portable SHA-256 implementation for the explicitly versioned `sha256-raw-v1` logical-byte digest.

Both dependencies disable default features. `uuid` enables only `std` and `v4`; `sha2` uses its no-default-features configuration. `Cargo.lock` is committed. The domain dependency tree must not contain Windows bindings, async runtimes, serialization, databases, network clients, UI libraries, or logging backends.
