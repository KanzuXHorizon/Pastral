# Windows developer setup

## Current implemented scope

These commands build and test the implemented foundation:

- `pastral-domain` — platform-independent immutable domain types and invariants;
- `pastral-storage` — synchronous single-owner SQLite/FTS5 metadata, ordinary internal/external blob persistence, literal lexical search, content-free capture audit storage, deletion, integrity checks, and bounded reconciliation;
- `pastral-clipboard-win` — Windows-only listener/session/format/HGLOBAL/Unicode-text platform boundary that never writes to the user's clipboard in automated tests;
- `pastral-manager.exe` — unpackaged C++20/C++/WinRT WinUI 3 manager with Home and History UI, adaptive layout, localization, accessibility landmarks, Debug-only synthetic preview data, and an empty disconnected Release provider.

The repository still does not contain the resident capture agent, COM/OLE capture/replay, versioned IPC, encrypted profiles, Quick Paste, passive overlay, packaging, installer, telemetry runtime, OCR, semantic search, or AI product code. The manager never opens SQLite or blob storage directly.

## Required environment

- Windows PowerShell 5.1 or newer.
- Git.
- rustup.
- Rust `1.97.1-x86_64-pc-windows-msvc` with `rustfmt`, `clippy`, and target `x86_64-pc-windows-msvc`.
- Visual Studio 2022 version 17.14 or newer with:
  - MSVC x64 v143 compiler/linker;
  - `Microsoft.VisualStudio.Workload.UniversalBuildTools`;
  - `Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools` (C++ WinUI app development build tools);
  - C++ UWP/WinUI x64 v143 platform toolset.
- Windows SDK `10.0.26100.0` for the current native manager build.
- Windows App Runtime `Microsoft.WindowsAppRuntime.2` x64 version `2.3.1.0` for local manager smoke testing.

Windows SDK `10.0.28000.2526` remains deferred to packaging/release validation. It is not required for the current unpackaged manager slice. The default toolchain verifier reports native availability without failing Rust-only work; pass `-RequireNativeManager` to enforce native build prerequisites.

SQLite does not need a separate machine installation. `rusqlite 0.40.1` is pinned with the `bundled` and `blob` features, and `libsqlite3-sys` builds the reviewed bundled SQLite source through the MSVC toolchain. The dependency graph includes build-helper crates such as `cc`, `pkg-config`, and `vcpkg`; no external vcpkg installation or repository vcpkg manifest is required.

## Install the native manager build tools

Run Visual Studio Installer elevated when the required components are absent:

```powershell
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$installPath = (& $vswhere -latest -products * -property installationPath).Trim()
$setup = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\setup.exe'

& $setup modify `
  --installPath $installPath `
  --add Microsoft.VisualStudio.Workload.UniversalBuildTools `
  --add Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools `
  --quiet --norestart --wait
```

The native workflow uses the supported Visual Studio/MSBuild/XAML path. It does not introduce CMake or a repository vcpkg manifest.

Install Windows App Runtime `2.3.1` x64 through the official Windows App SDK runtime installer before local Smoke. Compile/static CI does not require an interactive runtime session.

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

The command reports exact Rust tools plus Visual Studio, MSBuild, MSVC, C++ WinUI/UWP x64 toolset, installed Windows SDKs, Windows App Runtime, and the deferred release SDK. A Rust mismatch returns a nonzero exit code.

Enforce native manager build prerequisites explicitly:

```powershell
.\eng\verify-toolchain.ps1 -RequireNativeManager
```

## Build and test

Run the Rust foundation quality gate without requiring WinUI tooling:

```powershell
.\eng\build.ps1 -Task All
```

Run Rust plus native manager static/build gates:

```powershell
.\eng\build.ps1 -Task Full
```

Run native static/build plus interactive runtime/UI Automation smoke:

```powershell
.\eng\build.ps1 -Task Manager
```

Individual tasks:

```powershell
.\eng\build.ps1 -Task Verify
.\eng\build.ps1 -Task Format
.\eng\build.ps1 -Task Check
.\eng\build.ps1 -Task Test
.\eng\build.ps1 -Task Storage
.\eng\build.ps1 -Task Clipboard
.\eng\build.ps1 -Task NativePolicy
.\eng\build.ps1 -Task ManagerBuild
.\eng\build.ps1 -Task Manager
.\eng\build.ps1 -Task Clippy
.\eng\build.ps1 -Task Doc
.\eng\build.ps1 -Task Dependencies
.\eng\build.ps1 -Task SourcePolicy
```

The script stops at the first failure and preserves the failing command's exit code. `Storage` and `Clipboard` are focused crate tasks; `Test` already covers every Rust workspace crate. `ManagerBuild` compiles Debug and Release without launching UI. `Manager` additionally verifies window creation, History navigation, filtering, selection details, no-results state, and clean close through UI Automation.

## Direct CI-equivalent commands

```powershell
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo test --locked -p pastral-storage --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
cargo tree --locked --workspace

.\eng\verify-native-manager.ps1 -Mode Static
.\eng\verify-native-manager.ps1 -Mode Build
.\eng\verify-native-manager.ps1 -Mode Smoke
```

Native restore is locked by `apps/manager/Pastral.Manager/packages.lock.json`. The project pins Windows App SDK `2.3.1` and Microsoft.Windows.CppWinRT `3.0.260715.1` through `Directory.Packages.props`. The current toolchain requires single-tool MSBuild execution for reliable C++/WinRT/XAML generation (`UseMultiToolTask=false`, `MultiProcessorCompilation=false`); the verifier uses `/m:1 /nr:false`.

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

`Cargo.lock` is committed and every compiling/testing/documentation/dependency gate uses `--locked`. `eng/verify-dependencies.ps1` rejects async runtimes, serialization frameworks, alternate database stacks, Protobuf/IPC runtimes, logging backends, network clients, Windows bindings, and UI dependencies from the current foundation graph. `eng/verify-source-policy.ps1` rejects common secret/private-key signatures, credential/key files, build output, the machine-local launcher, unsafe blocks, network/process APIs, SQLite extension loading, database attachment, and WAL activation in current product source.

## Native manager data policy

- Debug builds expose exactly six generic synthetic records to exercise text, code, link, image, pinned, and unavailable states. Every relevant surface labels them as synthetic and they are not read from the clipboard or storage.
- Release builds compile a separate disconnected provider branch with no synthetic records.
- Paste and Copy actions remain disabled with accessible explanations until versioned local IPC, destination validation, and replay are implemented.
- Home and History consume immutable privacy-safe presentation metadata only. No payload bytes, SQLite handles, blob paths, network clients, or storage APIs enter the manager.

## Current storage and manager limitations

- Only ordinary protection-domain payloads are accepted. Sensitive and Private plaintext is rejected before any payload or search projection is persisted.
- Blob placement is selected by a caller-supplied versioned policy. No benchmark-selected production threshold exists yet.
- Inputs are bounded owned byte buffers; Win32/OLE streaming acquisition is not implemented.
- No encryption, backup/restore, import/export, backend migration, retention/quota engine, multi-process ownership, or background maintenance exists.
- FTS search is bounded literal lexical matching with deterministic ordering; no snippets, typo correction, semantic ranking, or sensitive indexing is provided.
- The manager's Debug filtering is bounded presentation-only matching over synthetic safe metadata; it is not the production typed query/FTS pipeline.
- Formal Narrator, high-contrast, 300% text/DPI, RTL, touch, RDP, packaging, signing, startup-performance, and memory budgets still require dedicated evidence.
