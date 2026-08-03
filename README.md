# Pastral

> Copy once. Find anything. Paste perfectly.

Pastral is a provisional Windows 11-native clipboard intelligence and history platform. It is designed as a private, searchable, source-aware memory of copy events, with multiple preserved representations, high-fidelity replay, focus-safe confirmation, profiles, deterministic rules, and explicit privacy controls.

“Paste perfectly” is a product ambition, not a claim that every application-private clipboard format can be captured or replayed losslessly.

## Project status

**Phase 0 — architecture and product foundation.**

This repository currently contains research, product scope, architecture decisions, threat/privacy models, performance budgets, UX specifications, testing strategy, and repository/release plans. Clipboard feature implementation has intentionally not started.

Feature work begins only after the Phase 0 consistency review passes and the first vertical-slice implementation plan is approved.

## Confirmed direction

- Windows 11 only; x64 first.
- Rust 1.97.1/Edition 2024 planned for agent, worker, CLI, and core modules.
- C++20, C++/WinRT, WinUI 3, and Windows App SDK 2.3.1 stable planned for the on-demand manager.
- One small event-driven `pastral-agent.exe` owns clipboard orchestration and storage.
- `pastral-worker.exe` runs only for bounded expensive or hostile work.
- SQLite + FTS5 metadata with recoverable blob storage.
- Native focus-safe overlay using Win32 and a compositor/Direct2D/DirectWrite path subject to prototype evidence.
- Local-first and network-silent core.
- No clipboard polling, mandatory AI, Electron, Tauri, or embedded browser primary UI.

Versions are pinned when the repository bootstrap implementation begins and are revalidated against official support before release.

## Privacy defaults

- Ordinary history retention: 90 days and 5 GB.
- Pinned clips are exempt from automatic retention deletion.
- Known password managers and reliably identified private-browser contexts are excluded by default.
- Highly confident secrets are not stored by default.
- Optional `SensitiveItemSkipped` records contain no preview, content hash, OCR, snippet, or reconstructable value.
- Source-owned clipboard history-exclusion signals are hard deny.
- Encrypted sensitive retention, when implemented, is explicit and narrowly scoped.

See [`docs/security/privacy-model.md`](docs/security/privacy-model.md) and [`docs/security/threat-model.md`](docs/security/threat-model.md).

## Documentation map

- [`PRODUCT.md`](PRODUCT.md) — durable product truth
- [`DESIGN.md`](DESIGN.md) — durable visual system
- [`docs/product/`](docs/product/) — vision, scope, personas, glossary
- [`docs/research/`](docs/research/) — official sources and competitor analysis
- [`docs/architecture/`](docs/architecture/) — process and data lifecycles
- [`docs/adr/`](docs/adr/) — architecture decisions
- [`docs/security/`](docs/security/) — threat, privacy, encryption, incident response
- [`docs/performance/`](docs/performance/) — budgets and benchmark method
- [`docs/testing/`](docs/testing/) — test strategy and compatibility matrix
- [`docs/ux/`](docs/ux/) — interaction, overlay, Quick Paste, manager, accessibility
- [`docs/brand/`](docs/brand/) — provisional brand and name research
- [`docs/operations/repository-initialization.md`](docs/operations/repository-initialization.md) — bootstrap sequence
- [`docs/release/checklist.md`](docs/release/checklist.md) — release evidence gates

## Development state

No Cargo, CMake, WinUI, installer, or executable scaffold exists yet. The next engineering slice will bootstrap the pinned toolchains and a small pure-domain Rust crate with tests, without implementing clipboard capture prematurely.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md), [`SECURITY.md`](SECURITY.md), and [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before opening work.

## License

No public source-code license has been selected. Copyright remains with the project owner unless and until a `LICENSE` file is deliberately added. Do not assume permission to redistribute or publish packages from this repository.

## Brand status

`Pastral` is a provisional working name. Preliminary research is recorded in [`docs/brand/name-clearance.md`](docs/brand/name-clearance.md); it is not legal clearance.
