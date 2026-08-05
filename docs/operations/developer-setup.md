# Windows developer setup

## Current implemented scope

These commands build and test the implemented foundation:

- `pastral-domain` — platform-independent immutable domain types and invariants;
- `pastral-storage` — synchronous single-owner SQLite/FTS5 metadata, ordinary internal/external blob persistence, literal lexical search, content-free capture audit storage, deletion, integrity checks, and bounded reconciliation;
- `pastral-clipboard-win` — Windows-only listener/session/format/HGLOBAL/Unicode-text platform boundary that never writes to the user's clipboard in automated tests;
- `pastral-agent-core` — Windows-binding-free capture coordinator with bounded retry, transient-sequence duplicate suppression, exact text/byte preservation, and explicit terminal outcomes;
- `pastral-ipc-core` — serializer-neutral safe Rust framing, incremental byte-stream decoding, handshake/in-flight/bulk state, and bounded validated control DTOs;
- `pastral-ipc-schema` — Edition 2024 control schema using exact official Protocol Buffers Rust runtime/codegen and conversion into `pastral-ipc-core` DTOs;
- `pastral-ipc-auth` — Windows-binding-free mutual HMAC transcript authentication, zeroized secret/proof material, and bounded replay cache;
- `pastral-ipc-win` — Windows DPAPI identity, token/session peer validation, logon-SID-only DACLs, overlapped named-pipe stream, and authenticated handshake boundary;
- `pastral-ipc-probe.exe` — deterministic content-free Release measurement executable for fragmented/coalesced schema round trips;
- `pastral-ipc-transport-probe.exe` — content-free parent/server-child executable that verifies a real authenticated cross-process Health exchange;
- `pastral-agent-ipc-probe.exe` — measured parent/baseline-child/server-child admission executable that serves a real agent-owned Health snapshot and enforces Release binary/private/working-set ceilings;
- `pastral-agent.exe` — diagnostic resident-agent executable with explicit health-check, one-shot current capture, and event-driven listen commands for ordinary `CF_UNICODETEXT`;
- `pastral-manager.exe` — unpackaged C++20/C++/WinRT WinUI 3 manager with Home and History UI, adaptive layout, localization, accessibility landmarks, Debug-only synthetic preview data, and an empty disconnected Release provider.

The repository contains the Rust framing/schema layer, authenticated cross-process named-pipe transport, and a measured real-agent Health admission path. It still does not contain C++ schema/client parity, production agent/manager transport linkage, COM/OLE capture/replay, manager live data, reliable private-browser detection, publisher verification, comprehensive secret classification, encrypted profiles, Quick Paste, passive overlay, packaging, installer, telemetry runtime, OCR, semantic search, or AI product code. The default agent is not auto-started and remains transport/Protobuf-free; the admission executable is evidence rather than an alternate production owner. The manager never opens SQLite or blob storage directly.

## Required environment

- Windows PowerShell 5.1 or newer.
- Git.
- rustup.
- Rust `1.97.1-x86_64-pc-windows-msvc` with `rustfmt`, `clippy`, and target `x86_64-pc-windows-msvc`.
- Exact Protocol Buffers compiler `libprotoc 35.0`, available as `protoc.exe` on `PATH`, through `PROTOC`, or from the user-scoped Winget `Google.Protobuf` 35.0 package.
- Visual Studio 2022 version 17.14 or newer with:
  - MSVC x64 v143 compiler/linker;
  - `Microsoft.VisualStudio.Workload.UniversalBuildTools`;
  - `Microsoft.VisualStudio.ComponentGroup.WindowsAppDevelopment.VC.BuildTools` (C++ WinUI app development build tools);
  - C++ UWP/WinUI x64 v143 platform toolset.
- Windows SDK `10.0.26100.0` for the current native manager build.
- Windows App Runtime `Microsoft.WindowsAppRuntime.2` x64 version `2.3.1.0` for local manager smoke testing.

Windows SDK `10.0.28000.2526` remains deferred to packaging/release validation. It is not required for the current unpackaged manager slice. The default toolchain verifier reports native availability without failing Rust-only work; pass `-RequireNativeManager` to enforce native build prerequisites.

SQLite does not need a separate machine installation. `rusqlite 0.40.1` is pinned with the `bundled` and `blob` features, and `libsqlite3-sys` builds the reviewed bundled SQLite source through the MSVC toolchain. The dependency graph includes build-helper crates such as `cc`, `pkg-config`, and `vcpkg`; no external vcpkg installation or repository vcpkg manifest is required.

## Install the exact Protocol Buffers compiler

The IPC schema prototype pins official Rust runtime/codegen `4.35.0-release` and requires exact `libprotoc 35.0`. A user-scoped portable install on Windows:

```powershell
winget install `
  --id Google.Protobuf `
  --version 35.0 `
  --exact `
  --scope user `
  --silent `
  --accept-package-agreements `
  --accept-source-agreements `
  --disable-interactivity
```

Verify:

```powershell
protoc --version
# libprotoc 35.0
```

When the current shell has not refreshed `PATH`, `crates/ipc-schema/build.rs` resolves the user-scoped Winget package directly. CI downloads only the official `protoc-35.0-win64.zip` asset and verifies SHA-256 `d1cede9e308cc3eb072392af1c02ccae4bdd3d2f374ec2970dbd8cdfdaa91363` before use. The Cargo build script never downloads tools.

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

Verify the bounded IPC framing/schema prototype and run 10,000 Release round trips:

```powershell
.\eng\build.ps1 -Task IpcPrototype
```

Verify the authenticated Windows transport and real parent/server-child Health exchange:

```powershell
.\eng\build.ps1 -Task IpcTransport
```

Verify shared agent Health truth, real authenticated Health serving, and Release footprint admission:

```powershell
.\eng\build.ps1 -Task AgentIpcAdmission
```

Build and smoke the diagnostic agent without reading the clipboard:

```powershell
.\eng\build.ps1 -Task Agent
```

Verify the live manager Health bridge end to end:

```powershell
.\eng\build.ps1 -Task ManagerIpcBridge
```

Run Rust, agent health-check smoke, the live manager Health bridge, and native manager static/build gates:

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
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task IpcTransport
.\eng\build.ps1 -Task AgentIpcAdmission
.\eng\build.ps1 -Task AgentPolicy
.\eng\build.ps1 -Task Agent
.\eng\build.ps1 -Task ManagerIpcBridge
.\eng\build.ps1 -Task NativePolicy
.\eng\build.ps1 -Task ManagerBuild
.\eng\build.ps1 -Task Manager
.\eng\build.ps1 -Task Clippy
.\eng\build.ps1 -Task Doc
.\eng\build.ps1 -Task Dependencies
.\eng\build.ps1 -Task SourcePolicy
```

The script stops at the first failure and preserves the failing command's exit code. `Storage` and `Clipboard` are focused crate tasks; `Test` covers every Rust workspace crate. `IpcPrototype` verifies exact `protoc 35.0`, runs 44 focused IPC tests, builds the Release probe, and completes 10,000 content-free round trips. `IpcTransport` runs static security checks, 38 focused authentication/transport/probe tests, builds the Release transport probe, and completes a content-free authenticated Health exchange between distinct processes. `AgentIpcAdmission` runs 16 focused shared-Health/process-memory/admission tests, builds the default agent and admission executable in Release, verifies a real agent-owned Health exchange, and fails if server private usage exceeds 25 MiB, private delta exceeds 8 MiB, working-set delta exceeds 12 MiB, or binary delta exceeds 6 MiB. `AgentPolicy` is static-only. `Agent` compiles Debug/Release and runs only a disposable `health-check`; it never invokes `capture-current` or `listen`. `ManagerIpcBridge` runs 35 focused feature-enabled agent and bridge tests, Clippy, exact export/dependency checks, a native C++ ABI/Health probe, and a Release Connected-to-Disconnected UI Automation smoke. Both bridge and native manager verifiers use unique ignored `target\verification\...` output/intermediate roots so concurrent verification cannot corrupt normal `x64` build artifacts. `ManagerBuild` compiles Debug and Release without launching UI. `Manager` additionally verifies window creation, History navigation, filtering, selection details, no-results state, and clean close through UI Automation.

## Direct CI-equivalent commands

```powershell
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo test --locked -p pastral-storage --all-targets
cargo test --locked -p pastral-agent --features ipc-health --test ipc_read
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
cargo tree --locked --workspace

.\eng\verify-ipc-prototype.ps1 -Mode Static
.\eng\verify-ipc-prototype.ps1 -Mode Test
.\eng\verify-ipc-prototype.ps1 -Mode Probe

.\eng\verify-ipc-transport.ps1 -Mode Static
.\eng\verify-ipc-transport.ps1 -Mode Test
.\eng\verify-ipc-transport.ps1 -Mode Smoke

.\eng\verify-agent-ipc-admission.ps1 -Mode Static
.\eng\verify-agent-ipc-admission.ps1 -Mode Test
.\eng\verify-agent-ipc-admission.ps1 -Mode Smoke

.\eng\verify-agent.ps1 -Mode Static
.\eng\verify-agent.ps1 -Mode Build
.\eng\verify-agent.ps1 -Mode Smoke

.\eng\verify-manager-ipc-bridge.ps1 -Mode Static
.\eng\verify-manager-ipc-bridge.ps1 -Mode Test
.\eng\verify-manager-ipc-bridge.ps1 -Mode Probe
.\eng\verify-manager-ipc-bridge.ps1 -Mode Live

.\eng\verify-native-manager.ps1 -Mode Static
.\eng\verify-native-manager.ps1 -Mode Build
.\eng\verify-native-manager.ps1 -Mode Smoke
```

Native restore is locked by `apps/manager/Pastral.Manager/packages.lock.json`. The project pins Windows App SDK `2.3.1` and Microsoft.Windows.CppWinRT `3.0.260715.1` through `Directory.Packages.props`. The current toolchain requires single-tool MSBuild execution for reliable C++/WinRT/XAML generation (`UseMultiToolTask=false`, `MultiProcessorCompilation=false`); the verifier uses `/m:1 /nr:false`.

## IPC framing and schema prototype

`pastral-ipc-core` is the durable serializer-neutral layer:

- exact 36-byte little-endian `PSTR` header;
- 256 KiB control/hello/error ceiling and 1 MiB bulk-chunk ceiling;
- allocation only after a complete structurally valid header;
- one in-progress frame per decoder and at most 64 emitted frames per push;
- UUIDv4/RFC 4122 correlation validation;
- mandatory client hello before control frames;
- 16 in-flight requests per connection;
- one explicitly authorized ordered bulk transfer;
- bounded health/history/search/error DTOs using typed domain UUID/time/order invariants.

`pastral-ipc-schema` is an isolated prototype adapter. It uses repository-authoritative Edition 2024 `.proto` input, exact official runtime/codegen `4.35.0-release`, and generated files under Cargo `OUT_DIR`. Generated upb bindings contain their expected native unsafe implementation; only that generated module receives a scoped unsafe/Clippy exception. Handwritten conversion remains `deny(unsafe_code)`, while `pastral-ipc-core` remains `forbid(unsafe_code)`.

A representative Release probe run on the current reference machine reported:

```text
ipc-probe=ok
protobuf-release=4.35.0-release
schema-sha256=2029ac9b19f7eb1644a2c12b3cd570586af9b62c40e130558b63c376676e3077
iterations=10000
round-trips=10000
executable-bytes=380416
average-roundtrip-ns=174042
one-byte-average-ns=1029
coalesced-average-ns=858
max-body-capacity=7869
```

These are prototype measurements, not release SLAs. The default Release agent remains protobuf-free. Authenticated named-pipe transport, logon-SID DACLs, kernel peer/session validation, DPAPI material, replay defense, timeout/cancel-drain, and a cross-process Health exchange pass dedicated gates. Phase 3F also measures real agent Health admission: representative Release evidence reports a 2,142,720-byte default agent, 2,413,568-byte admission executable, 270,848-byte binary delta, 606,208-byte working-set delta, and 53,248-byte private-memory delta, all below the explicit ceilings. Phase 3G connects the feature-gated real Health server to the native manager through a 414,208-byte Release bridge DLL; one representative native run measured connect `72 µs`, authenticated handshake `417 µs`, and Health `2,083 µs`. The Phase 3H foundation adds a separate feature-gated `serve-read` mode that negotiates exactly Health, HistoryPage, and Search, returns at most 100 metadata previews per response, truncates each preview to at most 4,096 UTF-8 bytes, and keeps `serve-health` Health-only. Generated C++ Protobuf parity, manager page-buffer mapping, fuzzing, adjacent-version fixtures, bulk staging, and production default-agent lifecycle integration remain separate gates.

## Diagnostic agent commands and safety boundary

Build the agent first:

```powershell
cargo build --locked -p pastral-agent
```

A safe storage/integrity check that does not open the clipboard:

```powershell
.\target\debug\pastral-agent.exe health-check --data-root "$env:LOCALAPPDATA\PastralDiagnostic"
```

The feature-gated IPC binary can expose a bounded diagnostic read boundary without opening the clipboard:

```powershell
cargo build --locked -p pastral-agent --features ipc-health --bin pastral-agent-ipc
.\target\debug\pastral-agent-ipc.exe serve-health --data-root "$env:LOCALAPPDATA\PastralDiagnostic" --max-connections 1
.\target\debug\pastral-agent-ipc.exe serve-read --data-root "$env:LOCALAPPDATA\PastralDiagnostic" --max-connections 3
```

`serve-health` negotiates and authorizes only Health. `serve-read` negotiates exactly Health, HistoryPage, and Search, accepts one request per authenticated connection, and exits after the configured `1..=16` connection bound. It returns metadata and bounded search projections only; it does not load raw representation/blob payloads. These commands are diagnostic candidates, not auto-started resident services.

The following commands **explicitly read the current user's clipboard** and are therefore never run by automated tests, CI, `-Task All`, or the health-check smoke:

```powershell
.\target\debug\pastral-agent.exe capture-current --data-root "$env:LOCALAPPDATA\PastralDiagnostic"
.\target\debug\pastral-agent.exe listen --data-root "$env:LOCALAPPDATA\PastralDiagnostic" --max-events 10
```

Running the executable without a command fails closed and prints usage. `capture-current` performs one bounded attempt. `listen` uses `AddClipboardFormatListener` notifications and bounded retry rather than polling. Diagnostic output reports only outcome classes, event IDs, and capture order; it never prints clipboard text or a content hash.

The chosen data root contains:

- `agent-identity.txt` with exactly `version`, a UUIDv4 profile ID, and an ordinary protection-domain UUIDv4;
- `privacy-policy.txt` with a strict version, `deny_unresolved_source`, and exact `deny_process` basenames;
- `storage/metadata.sqlite3` plus controlled object/staging directories owned by `pastral-storage`.

Default privacy policy:

```text
version=1
deny_unresolved_source=true
deny_process=1password.exe
deny_process=bitwarden.exe
deny_process=keepass.exe
deny_process=keepassxc.exe
```

Before reading Unicode payload bytes, the agent honors `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0`, then reduces the clipboard-owner process image to an executable basename and applies exact case-insensitive policy. Unresolved owners are denied by default. `CanUploadToCloudClipboard=0` does not deny local Pastral history because it is a cloud-only control.

Before creating a digest, blob, representation, or FTS row, the sink scans at most 1 MiB of decoded text for high-confidence private-key envelopes. Matching or detector-over-limit content creates only a content-free `SensitiveItemSkipped` audit. Generic tokens, JWTs, connection strings, OTPs, card-like numbers, and password assignments are not hard-skipped yet because reliable context validation is not implemented.

Malformed existing identity or privacy-policy content fails closed and is not silently replaced. The current policy stores only admitted ordinary `CF_UNICODETEXT`, uses internal SQLite BLOB placement, and still does not provide reliable private-browser detection, publisher verification, comprehensive secret classification, encryption, auto-start registration, or IPC.

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

`Cargo.lock` is committed and every compiling/testing/documentation/dependency gate uses `--locked`. `eng/verify-dependencies.ps1` pins official Protobuf `4.35.0-release` in the resident agent and reviewed IPC/schema/bridge probes; domain/storage/clipboard/agent-core/ipc-auth/ipc-core remain Protobuf-free. The policy rejects alternate serializers, async runtimes, gRPC/HTTP/network stacks, logging backends, unauthorized Windows bindings, and UI dependencies. `eng/verify-source-policy.ps1` rejects common secret/private-key signatures, credential/key files, build output, the machine-local launcher, unsafe product code outside the reviewed clipboard/native-generated boundaries, network/process/named-pipe APIs, SQLite extension loading, database attachment, and WAL activation in current product source.

## Native manager data policy

- Debug builds expose exactly six generic synthetic records to exercise text, code, link, image, pinned, and unavailable states. Every relevant surface labels them as synthetic and they are not read from the clipboard or storage.
- Release builds compile a separate disconnected provider branch with no synthetic records.
- Paste and Copy actions remain disabled with accessible explanations until versioned local IPC, destination validation, and replay are implemented.
- Home and History consume immutable privacy-safe presentation metadata only. No payload bytes, SQLite handles, blob paths, network clients, or storage APIs enter the manager.

## Current storage, agent, and manager limitations

- Only ordinary protection-domain payloads are accepted. Sensitive and Private plaintext is rejected before any payload or search projection is persisted.
- Blob placement is selected by a caller-supplied versioned policy. No benchmark-selected production threshold exists yet.
- Inputs are bounded owned byte buffers; the agent currently captures only `CF_UNICODETEXT`, while Win32/OLE streaming acquisition and other clipboard formats are not implemented.
- The agent enforces Windows history-control hard deny, unresolved-source fail-closed policy, an exact executable denylist, and a narrow private-key detector. It now has one supervised resident capture/read lifecycle and MSIX startup declaration, but it still lacks durable source attribution, reliable private-browser detection, publisher verification, a comprehensive secret classifier, replay suppression marker, user-initiated graceful shutdown, suspend/update handoff, and crash supervision.
- The Rust IPC frame/state/schema layer, authenticated Windows transport, manager bridge, live History/Search, and resident-agent lifecycle pass their current correctness, security, and footprint gates. Generated C++ schema parity, parser/schema fuzzing, adjacent-version fixtures, mutation/bulk staging, and update handoff remain incomplete.
- No encryption, backup/restore, import/export, backend migration, retention/quota engine, multi-process ownership, or background maintenance exists.
- FTS search is bounded literal lexical matching with deterministic ordering; no snippets, typo correction, semantic ranking, or sensitive indexing is provided.
- Release manager History/Search uses authenticated bounded IPC and literal lexical search. Debug mode retains explicitly disclosed synthetic records for UI verification only.
- Deterministic x64 MSIX staging, development signing, extraction parity, Start Apps activation, startup declaration, live IPC registration smoke, and package cleanup pass. Public trusted/timestamped signing, elevated clean-machine install, update/repair/rollback, Store/WinGet, and the formal Narrator/high-contrast/300% text/DPI/RTL/touch/RDP release matrix remain open.

## Resident and MSIX commands

Package scripts resolve `makeappx.exe` and `signtool.exe` only from the exact `WindowsTargetPlatformVersion` in `Directory.Build.props` (`10.0.26100.0`). They fail closed when that SDK is absent even if a newer SDK is installed. Verify this contract with:

```powershell
.\eng\build.ps1 -Task PackageToolchain
```

The signed-package verification report records the selected SDK version and SHA-256 hashes for both SDK tools.

Build and run the production resident agent with the default `%LOCALAPPDATA%\Pastral` root:

```powershell
cargo build --locked -p pastral-agent --release
.\target\release\pastral-agent.exe
```

Create the signed development installer and public certificate:

```powershell
.\eng\build-msix.ps1 -CreateDevelopmentCertificate
```

Verify the exact staged package through non-elevated Windows package registration, Start Apps activation, live authenticated IPC, cleanup, and data-root restoration:

```powershell
.\eng\test-msix-registration.ps1 `
  -StagingDirectory .\target\package\Pastral_0.1.0.0_x64
```

Run the signed MSIX install/uninstall gate from an Administrator PowerShell because AppX deployment requires the self-signed public certificate in `LocalMachine\TrustedPeople`:

```powershell
.\eng\test-msix-install.ps1 `
  -PackagePath .\artifacts\Pastral-0.1.0.0-x64.msix `
  -CertificatePath .\artifacts\Pastral-Development.cer
```

The development `.pfx` is temporary ignored output and is removed after signing. Do not distribute or commit it.
