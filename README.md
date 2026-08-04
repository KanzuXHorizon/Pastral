# Pastral

> Copy once. Find anything. Paste perfectly.

Pastral is a provisional Windows 11-native clipboard intelligence and history platform. It is designed as a private, searchable, source-aware memory of copy events, with multiple preserved representations, high-fidelity replay, focus-safe confirmation, profiles, deterministic rules, and explicit privacy controls.

“Paste perfectly” is a product ambition, not a claim that every application-private clipboard format can be captured or replayed losslessly.

## Project status

**Phase 3D — native manager, privacy-admitted diagnostic agent, and measured IPC framing/schema prototype.**

The repository now contains a reproducible Rust `1.97.1`/Edition 2024 workspace, pure `pastral-domain`, `pastral-agent-core`, and `pastral-ipc-core` crates, synchronous `pastral-storage`, the Windows-only `pastral-clipboard-win` boundary, an isolated official Protobuf Edition 2024 schema prototype, a diagnostic `pastral-agent.exe`, and an unpackaged C++20/C++/WinRT WinUI 3 manager built with Windows App SDK `2.3.1`.

The agent can perform an explicit storage health check, one explicit current-clipboard capture, or event-driven listening for bounded ordinary `CF_UNICODETEXT` capture. Before payload acquisition it honors Windows source-owned history exclusion controls, observes the clipboard-owner executable basename, and applies a strict exact-match deny policy that fails closed when the source cannot be resolved. Before digest/blob/index creation it skips high-confidence private-key material and detector-over-limit text, creating only a content-free sensitive-skip audit. Storage assigns durable capture order inside an immediate transaction, and the coordinator provides deterministic duplicate suppression and bounded retry without an async runtime. Automated tests and aggregate smoke gates run `health-check` only; they never invoke clipboard-reading commands.

The manager provides a native Mica/NavigationView shell, localized Home and History surfaces, adaptive layout, accessible landmarks, explicit disconnected/empty states, and a provider boundary that prevents direct SQLite or blob access. Debug builds expose six bounded, clearly labeled synthetic preview records. Release builds contain no synthetic history and remain honestly disconnected until versioned local IPC is implemented.

ADR 0018 remains Proposed. The Rust 36-byte framing/state/schema prototype now passes exact-toolchain, malformed-input, size, and latency gates, but C++ parity, fuzzing, authenticated named-pipe transport, and resident-agent linkage evidence remain required before acceptance.

## Confirmed direction

- Windows 11 only; x64 first.
- Rust 1.97.1/Edition 2024 is pinned for the workspace; `pastral-domain` remains platform-independent, `pastral-storage` owns persistence, and `pastral-clipboard-win` isolates the first reviewed unsafe/native boundary.
- C++20, C++/WinRT, WinUI 3, and Windows App SDK 2.3.1 stable are pinned for the on-demand manager; Debug and Release x64 builds are verified through MSBuild/XAML.
- One small event-driven `pastral-agent.exe` owns clipboard orchestration and storage, with a responsive control/overlay thread and a dedicated clipboard-platform STA for foreign capture objects/media and Pastral replay-object publication/lifetime.
- `pastral-worker.exe` runs only for bounded expensive or hostile work.
- SQLite + FTS5 metadata with one logical `BlobStore`; Phase 2A implements both internal SQLite BLOB and controlled external-file placement behind a caller-supplied versioned policy, without inventing a production threshold before Windows benchmarks.
- Native focus-safe overlay using Win32 and a compositor/Direct2D/DirectWrite path subject to prototype evidence.
- Local-first and network-silent core.
- No clipboard polling, mandatory AI, Electron, Tauri, or embedded browser primary UI.

Versions are pinned when the repository bootstrap implementation begins and are revalidated against official support before release.

## Privacy defaults

- Ordinary history retention: 90 days with a 5 GB automatic-cleanup target for unpinned history.
- Pinned/protected clips are exempt from automatic retention deletion and may exceed that target with visible warnings.
- The current diagnostic agent denies unresolved clipboard owners by default and ships an exact case-insensitive baseline denylist for `1password.exe`, `bitwarden.exe`, `keepass.exe`, and `keepassxc.exe`; this is not claimed as exhaustive password-manager or private-browser detection.
- Windows `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory=0` controls are hard deny before owner/payload processing.
- High-confidence private-key envelope material and content exceeding the current 1 MiB detector bound are not stored by default.
- Hidden `SensitiveItemSkipped` audit records are enabled by default for 24 hours and contain only broad policy/detector class, active profile, and coarse time—no preview, content hash, OCR, snippet, precise source, size, structure, or reconstructable value.
- Source-owned clipboard history-exclusion signals are hard deny and create no durable clip/audit row.
- Encrypted sensitive retention, when implemented, is explicit and narrowly scoped.
- The built-in Private profile is unavailable until mandatory encryption, random blob identity, non-indexing, lock, and recovery gates pass.
- Named-pipe ACLs and user-scope DPAPI strongly separate users/sessions but are not claimed as a secure enclave against fully compromised code already running as the same unlocked user.
- ADR 0018 prototypes Protobuf Edition 2024 control schemas with a bounded 36-byte frame and sequenced bulk transfer. Exact official `4.35.0-release` Rust runtime/codegen plus `protoc 35.0` pass the isolated Rust prototype, but the agent remains protobuf-free until transport/security/resident-footprint gates pass.

See [`docs/security/privacy-model.md`](docs/security/privacy-model.md) and [`docs/security/threat-model.md`](docs/security/threat-model.md).

## Documentation map

- [`PRODUCT.md`](PRODUCT.md) — durable product truth
- [`DESIGN.md`](DESIGN.md) — durable visual system
- [`docs/product/`](docs/product/) — vision, scope, personas, glossary
- [`docs/research/`](docs/research/) — official sources and competitor analysis
- [`docs/architecture/`](docs/architecture/) — process/threading, observation/event identity, source confidence, format policy, data model, IPC, and paste/capture lifecycles
- [`docs/adr/`](docs/adr/) — architecture decisions
- [`docs/security/`](docs/security/) — threat, privacy, encryption, incident response
- [`docs/performance/`](docs/performance/) — budgets and benchmark method
- [`docs/testing/`](docs/testing/) — test strategy and compatibility matrix
- [`docs/ux/`](docs/ux/) — interaction, overlay, Quick Paste, manager, accessibility
- [`docs/brand/`](docs/brand/) — provisional brand and name research
- [`docs/operations/repository-initialization.md`](docs/operations/repository-initialization.md) — bootstrap sequence
- [`docs/release/checklist.md`](docs/release/checklist.md) — release evidence gates

## Development state

The implemented foundation includes `crates/domain`, `crates/storage`, `crates/clipboard-win`, `crates/agent-core`, `crates/ipc-core`, the isolated `crates/ipc-schema` prototype, the deterministic `apps/ipc-probe`, the diagnostic resident agent under `apps/agent`, the native manager under `apps/manager/Pastral.Manager`, pinned Cargo/NuGet inputs, Windows CI, and PowerShell toolchain/build/dependency/source-policy verification.

From Windows PowerShell:

- `.\eng\build.ps1 -Task All` runs the Rust foundation gates only.
- `.\eng\build.ps1 -Task IpcPrototype` verifies exact `protoc 35.0`, runs 44 focused IPC tests, builds the Release probe, and executes 10,000 content-free deterministic round trips.
- `.\eng\build.ps1 -Task Agent` builds Debug/Release agent binaries and runs a disposable, content-free `health-check` smoke.
- `.\eng\build.ps1 -Task Full` runs Rust gates, the IPC prototype, the agent build/smoke gate, plus native static policy and Debug/Release manager builds.
- `.\eng\build.ps1 -Task Manager` additionally launches the Debug manager, navigates to History through UI Automation, exercises filtering/selection/no-results states, and verifies clean shutdown.

Exact setup and current limitations are in [`docs/operations/developer-setup.md`](docs/operations/developer-setup.md).

Only ordinary payload storage is enabled. Sensitive and Private plaintext is rejected before persistence or indexing because authenticated encryption has not been implemented. The SQLite foundation currently uses rollback journal `DELETE` with `synchronous=FULL`; WAL and a production internal/external placement threshold remain evidence-gated.

The WinUI manager project and native UI foundation use the supported `.vcxproj`/MSBuild/XAML path rather than experimental Windows App SDK CMake integration. The diagnostic resident agent, ordinary Unicode-text persistence path, Windows history-control hard deny, exact owner-process policy, narrow private-key detector, bounded IPC frame/state core, and Edition 2024 schema prototype now exist. The agent is not registered for auto-start, does not link Protobuf, and is not connected to the manager. Packaging, signing, installer, authenticated named-pipe transport, C++ schema/client parity, COM/OLE formats, reliable private-browser detection, publisher verification, comprehensive secret classification, encryption, Quick Paste, passive overlay, and live manager history/search/paste remain unimplemented. Automated clipboard tests do not write to or read from the user's clipboard, and the manager does not open storage directly.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md), [`SECURITY.md`](SECURITY.md), and [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before opening work.

## License

No public source-code license has been selected. Copyright remains with the project owner unless and until a `LICENSE` file is deliberately added. Do not assume permission to redistribute or publish packages from this repository.

## Brand status

`Pastral` is an internal provisional codename. Preliminary research—including a confusingly similar clipboard product named `Pastry`—is recorded in [`docs/brand/name-clearance.md`](docs/brand/name-clearance.md); it is not legal clearance.
