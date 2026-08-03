# Pastral

> Copy once. Find anything. Paste perfectly.

Pastral is a provisional Windows 11-native clipboard intelligence and history platform. It is designed as a private, searchable, source-aware memory of copy events, with multiple preserved representations, high-fidelity replay, focus-safe confirmation, profiles, deterministic rules, and explicit privacy controls.

“Paste perfectly” is a product ambition, not a claim that every application-private clipboard format can be captured or replayed losslessly.

## Project status

**Phase 1 — repository/toolchain and pure Rust domain bootstrap.**

The repository now contains a reproducible Rust `1.97.1`/Edition 2024 workspace, Windows PowerShell verification/build scripts, a pure `pastral-domain` crate, unit tests for its invariants, and Windows CI for this slice. Clipboard feature implementation has intentionally not started.

ADR 0018 remains Proposed and must pass its own runtime evidence gates before the later IPC implementation slice.

## Confirmed direction

- Windows 11 only; x64 first.
- Rust 1.97.1/Edition 2024 is pinned for the workspace; Phase 1 contains only the pure `pastral-domain` crate.
- C++20, C++/WinRT, WinUI 3, and Windows App SDK 2.3.1 stable planned for the on-demand manager.
- One small event-driven `pastral-agent.exe` owns clipboard orchestration and storage, with a responsive control/overlay thread and a dedicated clipboard-platform STA for foreign capture objects/media and Pastral replay-object publication/lifetime.
- `pastral-worker.exe` runs only for bounded expensive or hostile work.
- SQLite + FTS5 metadata with one content-addressed `BlobStore`; internal SQLite BLOB versus external-file placement is selected by Windows benchmark, not assumed globally.
- Native focus-safe overlay using Win32 and a compositor/Direct2D/DirectWrite path subject to prototype evidence.
- Local-first and network-silent core.
- No clipboard polling, mandatory AI, Electron, Tauri, or embedded browser primary UI.

Versions are pinned when the repository bootstrap implementation begins and are revalidated against official support before release.

## Privacy defaults

- Ordinary history retention: 90 days with a 5 GB automatic-cleanup target for unpinned history.
- Pinned/protected clips are exempt from automatic retention deletion and may exceed that target with visible warnings.
- Known password managers and reliably identified private-browser contexts are excluded by default.
- Highly confident secrets are not stored by default.
- Hidden `SensitiveItemSkipped` audit records are enabled by default for 24 hours and contain only broad policy/detector class, active profile, and coarse time—no preview, content hash, OCR, snippet, precise source, size, structure, or reconstructable value.
- Source-owned clipboard history-exclusion signals are hard deny and create no durable clip/audit row.
- Encrypted sensitive retention, when implemented, is explicit and narrowly scoped.
- The built-in Private profile is unavailable until mandatory encryption, random blob identity, non-indexing, lock, and recovery gates pass.
- Named-pipe ACLs and user-scope DPAPI strongly separate users/sessions but are not claimed as a secure enclave against fully compromised code already running as the same unlocked user.
- ADR 0018 proposes Protobuf Edition 2024 control schemas with a bounded 36-byte frame and sequenced bulk transfer; the resident Rust runtime remains unselected until footprint/build/security prototypes pass.

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

Phase 1 provides `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `eng/verify-toolchain.ps1`, `eng/build.ps1`, and `crates/domain`. Run `.\eng\build.ps1 -Task All` from Windows PowerShell; exact setup commands are in [`docs/operations/developer-setup.md`](docs/operations/developer-setup.md).

No Visual Studio/MSBuild WinUI project, packaging project, installer, executable, clipboard capture, database, IPC, or native UI implementation exists yet. The later manager uses the supported `.vcxproj`/MSBuild/XAML path rather than experimental Windows App SDK CMake integration.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md), [`SECURITY.md`](SECURITY.md), and [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before opening work.

## License

No public source-code license has been selected. Copyright remains with the project owner unless and until a `LICENSE` file is deliberately added. Do not assume permission to redistribute or publish packages from this repository.

## Brand status

`Pastral` is an internal provisional codename. Preliminary research—including a confusingly similar clipboard product named `Pastry`—is recorded in [`docs/brand/name-clearance.md`](docs/brand/name-clearance.md); it is not legal clearance.
