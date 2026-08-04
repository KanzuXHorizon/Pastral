# Changelog

All notable project changes will be documented here.

The format follows the intent of Keep a Changelog. Releases will use semantic versioning only after executable/package versioning is defined.

## Unreleased

### Added

- Phase 1 reproducible Rust 1.97.1/Edition 2024 workspace and Windows MSVC toolchain verification/build scripts.
- Pure `pastral-domain` crate with typed UUIDv4 identities, UTC Unix microseconds, capture order, stable clipboard format identity, protection-domain digest policy, immutable clip/audit/derived records, and fidelity aggregation.
- Windows Rust CI and exact developer setup instructions for the domain and storage foundation.
- `pastral-storage` crate with schema-versioned SQLite metadata, FTS5 literal lexical search, internal SQLite BLOB and controlled external-file placement, ordinary protection-domain deduplication, exact-byte integrity verification, content-free capture audit persistence, deletion, and bounded reconciliation.
- Lockfile-enforced PowerShell dependency and source-policy verification that rejects async, serialization, alternate database, IPC, logging, network, unauthorized Windows-binding, and UI dependencies, plus common secret/private-key signatures, unsafe code outside the reviewed native boundary, product-network/process APIs, machine-local launchers, build output, SQLite extension loading, database attachment, and WAL activation in the current foundation.
- Windows-only `pastral-clipboard-win` foundation with a message-only clipboard listener, bounded/coalescing notifications, transient sequence evidence, thread-bound read sessions, ordered runtime-format enumeration, stable registered-name identity, bounded HGLOBAL copying, and exact validated `CF_UNICODETEXT` extraction.
- Listener shutdown fallback through the listener thread message queue, isolated and documented Win32 unsafe code in one module, and automated native tests that never write to the user's clipboard.
- C++20/C++/WinRT WinUI 3 manager bootstrap with pinned Windows App SDK `2.3.1`, C++/WinRT `3.0.260715.1`, locked NuGet restore, x64 Debug/Release MSBuild verification, and an unpackaged Windows App Runtime `2.3.1` smoke path.
- Native manager design system and shell using Mica, TitleBar, NavigationView, system semantic brushes, localized resources, adaptive layout, and explicit accessibility names/headings.
- Immutable manager presentation-state/provider boundary: six clearly labeled Debug-only synthetic preview records and an empty disconnected Release provider that never opens SQLite or blob storage directly.
- Professional Home operational, disconnected, synthetic, recent-item, and empty states plus adaptive History search, list, selection details, unavailable-item warnings, disabled-action explanations, and no-results recovery.
- Native static policy, Debug/Release builds, runtime window lifecycle checks, and UI Automation coverage for History navigation, search filtering, selection details, no-results state, and clean shutdown.
- Storage-owned transactional capture-order assignment for new resident captures, while preserving the explicit-order import/test path.
- Windows-binding-free `pastral-agent-core` coordinator with deterministic duplicate suppression, bounded `[0, 5, 15, 35]` millisecond retry, terminal outcome modeling, exact UTF-16 preservation, and retryable storage failures.
- Diagnostic `pastral-agent.exe` with explicit `health-check`, `capture-current`, and `listen` commands, content-free identity persistence, ordinary `CF_UNICODETEXT` storage/FTS mapping, and Debug/Release plus health-check smoke verification.
- Agent static/dependency/source policies and Windows CI coverage that never invoke clipboard-reading commands automatically.
- Clipboard privacy-control inspection for `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`, and `CanUploadToCloudClipboard`, with malformed available controls failing closed.
- Bounded clipboard-owner process observation using limited query access, immediate basename reduction, exact case-insensitive source deny policy, and default unresolved-source denial.
- Strict atomic `privacy-policy.txt` configuration with a conservative baseline denylist for 1Password, Bitwarden, KeePass, and KeePassXC executable basenames.
- A 1 MiB bounded high-confidence private-key detector and audit-only `SensitiveItemSkipped` handling that creates no clip, digest, blob, preview, or FTS row.
- Pure-Rust `pastral-ipc-core` with an exact 36-byte little-endian frame codec, allocation-after-validation incremental byte-stream decoder, handshake/in-flight/bulk state machine, and serializer-neutral bounded control DTOs.
- Edition 2024 `pastral.ipc.v1` schema prototype generated with exact official `protoc 35.0`, `protobuf 4.35.0-release`, and `protobuf-codegen 4.35.0-release`, with generated DTOs isolated behind validated conversion.
- A content-free Release `pastral-ipc-probe` and PowerShell gate covering 44 focused IPC tests plus 10,000 fragmented/coalesced schema round trips, exact schema digest, binary size, latency, and decoder-capacity evidence.
- Dependency/source/CI policy that keeps Protobuf out of the resident agent and core product crates, verifies exact official package versions, and hashes the official Windows protoc archive before CI use.
- Pure `pastral-ipc-auth` mutual HMAC-SHA256 transcript authentication with role-separated proofs, constant-time verification, exact 32-byte secret/proof material, canonical capability binding, zeroization, and a bounded 1,024-entry replay cache.
- Windows-only `pastral-ipc-win` transport identity with system CSPRNG, user-scope noninteractive DPAPI protection, strict versioned secret/identity files, no-replace atomic publication, canonical session-scoped pipe names, and concurrency convergence tests.
- Kernel-derived peer identity validation for user SID, enabled logon SID, session ID, integrity RID, and PID using bounded token queries and reviewed handle/SID ownership.
- Protected logon-SID-only named-pipe DACLs, first-instance anti-squatting, remote-client rejection, identification-only client SQOS, byte-mode overlapped I/O, bounded timeout/cancel-drain behavior, peer PID/session queries, and framed stream exchange.
- Mutual authenticated ServerHello/ClientHello/ServerAccepted handshake and content-free cross-process `pastral-ipc-transport-probe` that performs one authenticated Health request/response between distinct processes.
- `IpcTransport` PowerShell/CI gate covering static security policy, 36 focused auth/transport/probe tests, Release cross-process smoke, PID/session evidence, content-free output, and exact dependency/source boundaries.
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

- The native manager, privacy-admitted diagnostic resident-agent ordinary Unicode-text capture path, Rust framing/schema layer, and authenticated cross-process Windows named-pipe transport exist. The transport remains diagnostic-only: C++ parity, fuzzing, adjacent-version fixtures, agent/manager linkage, auto-start, and live manager data do not yet exist. COM/OLE formats, reliable private-browser detection, publisher verification, comprehensive secret classification, encryption, Quick Paste, passive overlay, and packaging also remain incomplete.
- Storage accepts ordinary payloads only; Sensitive and Private plaintext is rejected until authenticated encryption exists.
- SQLite currently uses rollback journal `DELETE` with `synchronous=FULL`; WAL and the production internal/external placement threshold remain benchmark and crash-evidence gated.
- The current manager is unpackaged and requires Windows App Runtime `2.3.1` x64 for local launch; no installer, package identity, signing pipeline, or public update channel exists.
- No final logo/brand assets or legal name clearance exists.
- Performance, fidelity, security, compatibility, and accessibility budgets are design targets pending implementation evidence.
- Extremely rapid clipboard replacements may make intermediate states unobservable; Pastral targets bounded behavior and final-current-state capture rather than claiming complete event history.
- `Pastral` remains an internal codename; similarity to the clipboard product `Pastry` raises the public-brand clearance threshold.
