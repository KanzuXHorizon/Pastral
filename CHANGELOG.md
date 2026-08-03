# Changelog

All notable project changes will be documented here.

The format follows the intent of Keep a Changelog. Releases will use semantic versioning only after executable/package versioning is defined.

## Unreleased

### Added

- Phase 1 reproducible Rust 1.97.1/Edition 2024 workspace and Windows MSVC toolchain verification/build scripts.
- Pure `pastral-domain` crate with typed UUIDv4 identities, UTC Unix microseconds, capture order, stable clipboard format identity, protection-domain digest policy, immutable clip/audit/derived records, and fidelity aggregation.
- Windows Rust CI and exact Phase 1 developer setup instructions.
- Phase 0 product vision, scope, personas, and glossary.
- Official-source research and competitor/gap analysis.
- Foundation architecture and clipboard/paste lifecycle designs.
- Initial ADR set for Rust, WinUI 3, overlay rendering, data model, process boundaries, storage, encryption, IPC, packaging, updates, Windows baseline, OCR, semantic search, and plugins.
- Threat, privacy, encryption, and incident-response models.
- Performance budgets and benchmark methodology.
- Testing strategy, compatibility matrix, and test-data policy.
- UX design system, interaction, overlay, Quick Paste, manager, and accessibility specifications.
- Provisional brand guidelines and name-clearance research.
- Repository initialization and release planning.
- Phase 0.1 adversarial architecture audit and hardening plan.
- ADRs for dedicated clipboard-platform STA/hybrid Win32-OLE capture/replay ownership, durable UUID/time/order/digest semantics, and Quick Paste hosting.
- Normative data model, clipboard observation identity, format-adapter policy, source-confidence model, threading/COM ownership, and IPC security model.
- Stable manager build authority based on Visual Studio/MSBuild/XAML with `Pastral.slnx`; multi-executable packaging remains a separate `.wapproj`, while experimental Windows App SDK CMake support is not a release prerequisite.
- Proposed ADR 0018 for Protobuf Edition 2024 control schemas, an explicit 36-byte bounded named-pipe frame, sequenced bulk transfer, and measured resident-runtime acceptance gates.
- Content-addressed `BlobStore` contract with benchmark-selected internal SQLite BLOB and external-file backends; event/representation rows store references, and backend choice does not change digest/protection semantics.

### Security

- Defined local-first/network-silent default.
- Defined hard-deny clipboard-source policy.
- Defined default non-storage of highly confident secrets.
- Defined explicit logon-session named-pipe ACL, anti-squatting/remote rejection, peer validation, operation authorization, and restricted-worker boundaries.
- Corrected same-user threat claims: user-scope DPAPI and pipe authentication are defense in depth, not a secure enclave against fully compromised same-user code.
- Defined source-owned hard-deny as no durable record, hidden 24-hour sensitive-skip audit defaults, mandatory encrypted Private-profile gates, authenticated-before-release decryption, and best-effort display-affinity limits.

### Known limitations

- No executable application, clipboard integration, storage, IPC, native UI, or packaging implementation exists yet.
- No installer, package identity, signing pipeline, or public update channel exists.
- No final logo/brand assets or legal name clearance exists.
- Performance, fidelity, security, compatibility, and accessibility budgets are design targets pending implementation evidence.
- Extremely rapid clipboard replacements may make intermediate states unobservable; Pastral targets bounded behavior and final-current-state capture rather than claiming complete event history.
- `Pastral` remains an internal codename; similarity to the clipboard product `Pastry` raises the public-brand clearance threshold.
