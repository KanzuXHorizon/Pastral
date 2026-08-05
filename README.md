# Pastral

> Copy once. Find anything. Paste perfectly.

[![Rust foundation CI](https://github.com/KanzuXHorizon/Pastral/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/KanzuXHorizon/Pastral/actions/workflows/rust-ci.yml)
[![Native manager UI CI](https://github.com/KanzuXHorizon/Pastral/actions/workflows/native-ui-ci.yml/badge.svg)](https://github.com/KanzuXHorizon/Pastral/actions/workflows/native-ui-ci.yml)
![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4)
![Architecture](https://img.shields.io/badge/architecture-x64-5C2D91)
![Rust](https://img.shields.io/badge/Rust-1.97.1-000000)
![Windows App SDK](https://img.shields.io/badge/Windows%20App%20SDK-2.3.1-0078D4)

Pastral is a Windows 11-native clipboard intelligence and history platform designed around local ownership, source awareness, high-fidelity capture, deterministic behavior, and explicit privacy boundaries.

The project is currently an engineering preview. The ordinary-text capture/storage foundation, single resident agent lifecycle, authenticated read-only IPC, live native Manager History/Search path, and signed development MSIX workflow are implemented and verified. Rich clipboard formats, paste/replay, encryption, Quick Paste, full manager administration surfaces, production signing, and update delivery remain in development.

> “Paste perfectly” is a product ambition, not a claim that every application-private clipboard format can be captured or replayed losslessly.

## Current milestone

**Phase 3J foundation — one production resident agent, live native History/Search, and a verified signed development MSIX.**

The current repository includes:

- a pinned Rust `1.97.1` / Edition 2024 workspace;
- domain, storage, clipboard, privacy, IPC framing, authentication, schema, and Windows transport crates;
- one production resident clipboard agent with bounded ordinary `CF_UNICODETEXT` capture, authenticated read IPC, and a per-data-root kernel instance guard acquired before storage ownership;
- SQLite + FTS5 storage with internal and external blob placement support;
- authenticated same-user/session named-pipe transport using DPAPI-protected installation material;
- a resident authenticated boundary for Health, paged History, and literal Search using bounded preview metadata only;
- a versioned caller-owned Rust C ABI bridge for manager Health, History, and Search state;
- an unpackaged C++20/C++/WinRT WinUI 3 manager using Windows App SDK `2.3.1`;
- English and Vietnamese manager resources;
- a signed development MSIX containing only the manager, resident agent, bridge, resources, and reviewed assets;
- Windows CI, policy checks, focused probes, runtime smoke tests, package verification, and explicit performance ceilings.

## Capability matrix

| Area | Status | Notes |
| --- | --- | --- |
| Domain and event model | Implemented | Typed IDs, fidelity, protection domains, source metadata, deterministic ordering |
| SQLite + FTS5 storage | Implemented foundation | Ordinary payloads only; WAL and final placement threshold remain evidence-gated |
| Windows clipboard listener | Implemented foundation | Event-driven, no polling; currently ordinary `CF_UNICODETEXT` capture |
| Source privacy admission | Implemented foundation | Honors Windows history-exclusion controls and exact executable deny policy |
| Sensitive-content admission | Partial | High-confidence private-key material and detector overflow are skipped |
| Authenticated local IPC | Implemented foundation | Bounded framing, peer/session evidence, HMAC authentication, replay defense |
| Manager Health connection | Implemented | Live content-free Health through a versioned Rust bridge |
| Native Manager UI | Implemented foundation | Home and History shell, adaptive states, accessibility, English/Vietnamese resources |
| Live History/Search | Implemented foundation | Resident agent, caller-owned C ABI, asynchronous provider, and native UI render bounded safe previews |
| Paste/replay engine | Not implemented | Format fidelity and focus-safe confirmation remain separate milestones |
| Encryption and Private profile | Not implemented | Private profile remains unavailable until encryption/recovery gates pass |
| Installer, signing, updates | Partial | Signed development MSIX and install/uninstall verification exist; protected production signing and update delivery remain open |

## Architecture

```text
Windows clipboard
       │
       ▼
pastral-clipboard-win
       │ reviewed native boundary
       ▼
pastral-agent-core ── privacy admission ── pastral-storage
       │                                      │
       │                                      └─ SQLite + FTS5 + BlobStore
       ▼
pastral-agent
       │ authenticated named pipe
       ▼
pastral-manager-ipc-bridge.dll
       │ fixed-size versioned C ABI
       ▼
Pastral.Manager.exe (C++/WinRT + WinUI 3)
```

The manager never opens SQLite, FTS, clipboard APIs, or blob storage directly. All live state crosses an explicit provider boundary. The production resident agent owns capture, storage, and authenticated Health/History/Search serving. The Rust bridge maps bounded caller-owned result buffers into immutable native presentation records, and the WinUI provider performs reads off the XAML thread with stale-result rejection. Paste still requires separate authorization, payload transfer, destination validation, and privacy review.

## Privacy and security posture

Pastral is local-first and the core is network-silent.

Current protections include:

- no clipboard polling;
- hard deny for `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0`;
- fail-closed behavior when the clipboard owner cannot be resolved;
- a baseline exact case-insensitive denylist for `1password.exe`, `bitwarden.exe`, `keepass.exe`, and `keepassxc.exe`;
- pre-persistence rejection of high-confidence private-key envelopes and text beyond the current 1 MiB detector bound;
- content-free `SensitiveItemSkipped` audit records;
- same-logon-SID named-pipe ACLs;
- kernel peer PID/session evidence;
- DPAPI-protected per-installation authentication material;
- mutual HMAC authentication and replay rejection;
- bounded reads, writes, frame sizes, connection counts, retries, and timeouts;
- no synthetic history in Release builds.

These controls do not create a secure enclave against fully compromised code already running as the same unlocked Windows user. See [`docs/security/privacy-model.md`](docs/security/privacy-model.md) and [`docs/security/threat-model.md`](docs/security/threat-model.md).

## Development prerequisites

- Windows 11 x64
- Visual Studio 2022 Build Tools with MSVC x64 and C++ WinUI/UWP build tools
- Windows SDK `10.0.26100.0`
- Windows App Runtime `2.3.1` x64 for local manager launch
- Rust `1.97.1-x86_64-pc-windows-msvc` with `rustfmt` and `clippy`
- PowerShell 5.1+
- `protoc 35.0` for the focused IPC schema gate

The exact setup procedure is documented in [`docs/operations/developer-setup.md`](docs/operations/developer-setup.md).

## Build and verify

Run commands from Windows PowerShell at the repository root.

```powershell
# Rust foundation checks
.\eng\build.ps1 -Task All

# Authenticated IPC and schema probes
.\eng\build.ps1 -Task IpcPrototype
.\eng\build.ps1 -Task IpcTransport

# Resident agent and measured IPC admission
.\eng\build.ps1 -Task Agent
.\eng\build.ps1 -Task AgentIpcAdmission

# Native manager Health/History/Search bridge
.\eng\build.ps1 -Task ManagerIpcBridge

# Native manager static policy and Debug/Release build
.\eng\build.ps1 -Task ManagerBuild

# Native manager UI Automation smoke
.\eng\build.ps1 -Task Manager

# Aggregate Rust + IPC + agent + bridge + native build gates
.\eng\build.ps1 -Task Full
```

Automated aggregate and CI smoke gates do not invoke `capture-current` or `listen`; they do not read from or write to the user's clipboard.

## Repository layout

```text
apps/
  agent/                    Production resident capture and authenticated read agent
  agent-ipc-probe/          Measured cross-process Health/read admission probe
  ipc-probe/                Deterministic framing/schema probe
  ipc-transport-probe/      Authenticated Windows transport probe
  manager/Pastral.Manager/  Native C++/WinRT WinUI 3 manager
crates/
  agent-core/               Clipboard orchestration and admission coordination
  clipboard-win/            Reviewed Windows clipboard boundary
  domain/                   Platform-independent domain model
  ipc-auth/                 HMAC transcript and replay protection
  ipc-core/                 Framing, connection state, and bounded DTO rules
  ipc-schema/               Protobuf Edition 2024 control schema bindings
  ipc-win/                  Named-pipe, DPAPI, token, DACL, and timeout boundary
  manager-ipc-bridge/       Versioned Rust C ABI used by the manager
  storage/                  SQLite, FTS5, BlobStore, retention, and maintenance
eng/                        Build, policy, smoke, dependency, and CI-equivalent scripts
docs/                       Product, architecture, ADR, security, UX, testing, and release evidence
protocols/                  Versioned IPC schema sources
```

## Engineering principles

- Windows-native interaction and accessibility first.
- Local ownership and explicit privacy boundaries.
- No Electron, Tauri, embedded browser, or mandatory AI in the primary product.
- No clipboard polling.
- No direct storage access from the manager.
- Fail closed at privacy and authentication boundaries.
- Preserve exact clipboard bytes where the format contract permits it.
- Make performance and memory claims only from measured gates.
- Keep unsupported capabilities visibly unavailable rather than fabricating data or state.

## Roadmap

The next major engineering slices are:

1. restore one canonical full verification baseline and complete manager localization/responsive behavior;
2. richer Win32/OLE clipboard format acquisition, retention, quota, backup, and recovery;
3. focus-safe paste/replay, Quick Paste, overlay, tray, and global-hotkey interaction;
4. authenticated encryption, key lifecycle, lock/recovery, Private profile, profiles, and deterministic rules;
5. complete manager administration surfaces for sources, collections, storage, privacy, settings, diagnostics, and about;
6. parser/schema fuzzing, adjacent-version/update fixtures, release-scale performance and accessibility evidence;
7. protected production signing, update delivery, rollback, and final release provenance.

Detailed scope and acceptance gates live in [`PRODUCT.md`](PRODUCT.md), [`DESIGN.md`](DESIGN.md), [`docs/adr/`](docs/adr/), and [`docs/release/checklist.md`](docs/release/checklist.md).

## Documentation

- [`PRODUCT.md`](PRODUCT.md) — durable product truth
- [`DESIGN.md`](DESIGN.md) — visual and interaction system
- [`docs/architecture/`](docs/architecture/) — process, storage, IPC, capture, and paste design
- [`docs/adr/`](docs/adr/) — architecture decisions
- [`docs/security/`](docs/security/) — privacy, threat model, encryption, and incident response
- [`docs/performance/`](docs/performance/) — budgets and benchmark methodology
- [`docs/testing/`](docs/testing/) — test strategy and compatibility matrix
- [`docs/ux/`](docs/ux/) — manager, Quick Paste, overlay, and accessibility
- [`docs/operations/`](docs/operations/) — setup and repository operation
- [`CHANGELOG.md`](CHANGELOG.md) — implemented milestones and known limitations

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md), [`SECURITY.md`](SECURITY.md), and [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before opening work.

## License

No public source-code license has been selected. Copyright remains with the project owner unless and until a `LICENSE` file is deliberately added. Public visibility does not grant permission to redistribute source code or publish derived packages.

## Brand status

`Pastral` remains a provisional project name. Preliminary name research, including the similarly named clipboard product `Pastry`, is documented in [`docs/brand/name-clearance.md`](docs/brand/name-clearance.md). This is not legal clearance.
