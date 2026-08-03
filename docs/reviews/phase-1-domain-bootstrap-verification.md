# Phase 1 domain bootstrap verification

**Date:** 2026-08-04
**Baseline commit:** `d4bbb90`
**Verified implementation commit before this report:** `fbb0bcb`
**Branch:** `main`

## Environment

- OS: Microsoft Windows NT `10.0.26200.0`
- Shell: Windows PowerShell `5.1.26100.8972`
- Repository: `F:\Pastral`
- Rust host/target: `x86_64-pc-windows-msvc`
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)`
- rustup: active `1.97.1-x86_64-pc-windows-msvc`, selected by `rust-toolchain.toml`; global default was not changed
- rustfmt: `1.9.0-stable (8bab26f4f6 2026-07-14)`
- Clippy: `0.1.97 (8bab26f4f6 2026-07-14)`
- Visual Studio: `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`
- MSVC linker: `VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
- Installed Windows SDK: `10.0.26100.0`
- Deferred native-slice Windows SDK `10.0.28000.2526`: not installed; intentionally non-blocking for this pure-domain slice

## Delivered scope

- Pinned Rust workspace/toolchain with Edition 2024 and `rust-version = "1.97.1"`.
- PowerShell 5.1-compatible toolchain verification and fail-fast build task dispatcher.
- One pure crate, `pastral-domain`, with `#![forbid(unsafe_code)]`.
- Typed UUIDv4 IDs, validated UTC Unix microseconds, strictly positive capture order, stable clipboard format identity, protection domains, `sha256-raw-v1`, immutable clip representations/events, content-free audit events, fidelity aggregation, and immutable derived provenance.
- Windows GitHub Actions workflow and exact developer setup documentation.

No Win32 clipboard, COM/OLE, SQLite/FTS5, BlobStore backend, IPC, Protobuf, executable, async runtime, filesystem/network product code, WinUI/C++, packaging, logging backend, OCR, semantic search, AI, serde, CMake, or vcpkg implementation was added.

## Dependency tree

Fresh `cargo tree --workspace` output:

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
│           ├── generic-array v0.14.7
│           └── typenum v1.20.1
└── uuid v1.18.1
    └── getrandom v0.3.4
        └── cfg-if v1.0.4
```

A product-code and dependency scan found no `windows`, `windows-sys`, `tokio`, `serde`, `chrono`, `time`, `rusqlite`, `sqlx`, `prost`, `protobuf`, `tracing`, network client, WinUI, or named-pipe dependency/API.

## TDD red/green evidence

### Red

Before the first successful domain build, this command was run:

```powershell
cargo test --workspace --all-targets
```

It failed during compilation with seven typed-ID errors because the initial implementation attempted to construct a nonexistent `uuid::Error::InvalidVersion`, plus one `const` evaluation error in protection-domain equality. This proved that the UUID textual boundary and dedup policy implementation were not yet valid. The failure was not committed.

A later quality-gate run reached Clippy and failed with one `clippy::manual_contains` warning in fidelity aggregation under `-D warnings`. The implementation was changed to `contains`, then the full gate was rerun.

### Green

After the minimal corrections, `cargo test --workspace --all-targets` passed all 25 unit tests. Fresh final runs through both `eng/build.ps1 -Task All` and the direct CI-equivalent commands again passed all 25 tests with zero failures, ignored tests, measured tests, or filtered tests.

## Fresh commands and results

All commands below were run from Windows PowerShell against `fbb0bcb` before writing this report:

| Command | Result |
|---|---|
| `git status --short` | PASS; no output before final verification |
| `eng/verify-toolchain.ps1` | PASS; required Rust/MSVC state matched, deferred SDK reported non-blocking |
| `eng/build.ps1 -Task All` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets` | PASS |
| `cargo test --workspace --all-targets` | PASS; 25 passed, 0 failed, 0 ignored |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo doc --workspace --no-deps` | PASS |
| `cargo tree --workspace` | PASS; dependency tree recorded above |
| `git diff --check` | PASS |
| secret-signature scan for AWS/GitHub/private-key signatures | PASS; no hits |
| product-code forbidden-scope scan | PASS; no hits |
| `git fsck --no-progress --no-dangling` | PASS |
| `git show --check HEAD` | PASS |

The direct Cargo checks were intentionally run again after `eng/build.ps1 -Task All` to match the requested final checklist exactly rather than relying only on wrapper output.

## Files changed from baseline

Twenty-five files were added or modified before this report:

- Root workspace: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`.
- Build tooling: `eng/build.ps1`, `eng/verify-toolchain.ps1`.
- Domain crate: `crates/domain/Cargo.toml` and 13 focused Rust source modules.
- CI: `.github/workflows/rust-ci.yml`.
- Documentation: design spec, implementation plan, developer setup, `README.md`, and `CHANGELOG.md`.

`Start-DevSpace-MCP-Cloudflared.ps1` remains untracked/ignored and was not committed. No `LICENSE` was added.

## Local commits

- `f257eed docs: specify repository domain bootstrap`
- `9458ea8 feat: add immutable domain foundation`
- `fbb0bcb ci: verify Rust domain bootstrap`
- Final evidence commit follows this report.

No remote push, pull request, history rewrite, or squash was performed.

## Skipped checks

- GitHub-hosted `windows-2025` CI was not executed remotely because the user explicitly prohibited pushing or creating a PR. The workflow was reviewed locally and its commands were run on the actual Windows machine.
- No native Windows SDK `10.0.28000.2526`, Windows App SDK, MSBuild project, UI, package, signing, clipboard fixture, database, IPC, performance, accessibility, or installer check was run because those systems are outside Phase 1 and do not exist in this repository slice.
- No dependency advisory/license policy tool was added or run because the approved slice required a small CI workflow and did not authorize extra tooling/dependencies; dependency versions and graph were inspected directly.
- No compile-fail framework was added. Distinct typed APIs are enforced by Rust signatures, while ordinary unit tests cover runtime identity properties without adding `trybuild`.

## Residual risks and unproven claims

- The GitHub Actions workflow is not proven on the hosted runner until a future authorized remote run.
- UUIDv4 generation relies on `uuid`/`getrandom` and the operating system CSPRNG path; this slice verifies UUID version/variant and dependency selection, not OS entropy quality.
- SHA-256 correctness relies on the reviewed `sha2` dependency; tests prove deterministic exact-byte behavior and policy boundaries, not an independent cryptographic implementation audit.
- The chosen UTC range is constructor-tested at boundaries but has not yet been exercised through a persistence or IPC schema because those are deliberately absent.
- Immutable-original guarantees are established through private fields, shared-reference getters, owned construction, and absence of mutation APIs; Rust cannot prove that future code will never add a mutation API, so this remains a review/CI contract.
- No Windows clipboard fidelity, storage durability, IPC compatibility, UI behavior, packaging, security isolation, or performance claim is made by Phase 1.
