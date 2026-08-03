# Repository initialization plan

**Status:** Phase 0.2-refined bootstrap plan; no feature scaffold is created by this document.
**Repository root:** `F:\Pastral`

## 1. Current state

- Git repository initialized on `main`.
- Existing local `Start-DevSpace-MCP-Cloudflared.ps1` remains at repository root but is ignored; it is not product source and must not be committed because it is machine/workflow-specific.
- Phase 0 foundation and Phase 0.1 adversarial hardening documentation/governance exist.
- ADR 0015–0017 define capture threading, durable identity/time/digests, and Quick Paste hosting; ADR 0018 proposes bounded IPC framing/Protobuf schemas while keeping the resident runtime unselected pending evidence.
- Normative architecture now includes dedicated clipboard-platform STA ownership, transient observation versus durable clip/audit records, stable clipboard-format names, source confidence, format adapters, IPC limits, UIPI fallback, Private/sensitive protection domains, and a benchmark-gated internal-SQLite-BLOB/external-file `BlobStore` policy.
- No Cargo workspace, Visual Studio/MSBuild WinUI project, packaging project, installer, CI workflow, executable, database, or runtime data exists.
- No public license has been selected; `LICENSE` remains intentionally absent.

## 2. Bootstrap principles

- Bootstrap is its own reviewed vertical slice.
- Pin toolchain/dependency versions in repository files; do not depend on unrecorded global defaults.
- Windows-native build and tests run from a supported Windows developer environment, not WSL alone.
- Keep the agent dependency graph minimal from the first commit.
- Build only a pure domain skeleton and toolchain validation first; clipboard feature work receives its own design/plan.
- The first domain slice must encode UUIDv4 IDs, UTC microsecond civil time, persistent installation-local `capture_order`, `ClipEvent` nonempty representation cardinality, `CaptureAuditEvent` separation, protection domains, and `sha256-raw-v1` semantics before database or Win32 work.
- No package identity, signing certificate, Store reservation, public crate/package publication, or network integration during bootstrap.
- Generated user data, logs, signing material, tunnel state, package output, and fuzz output remain ignored.

## 3. Initial pinned baseline

Revalidate all versions against official sources on the bootstrap date. Initial decisions:

- Rust stable `1.97.1`, Edition 2024.
- Windows SDK `10.0.28000.2526`, while runtime support floor remains Windows 11 build 26100 and newer API calls require capability checks.
- Windows App SDK `2.3.1` stable for the manager prototype.
- C++20.
- x64 build presets first.
- SQLite compiled with FTS5 and pinned through a reviewed source/package path.
- Protocol Buffers v35.0 release train is the initial ADR 0018 prototype candidate, revalidated before the IPC slice. Edition 2024 schemas and exact supported generator/generated-code/runtime matching are required; the official Rust kernel path and a credible wire-compatible Rust alternative are measured before any resident runtime is accepted. No Protobuf dependency is added to the pure-domain bootstrap.
- `windows`/`windows-sys` versions selected and locked after API-surface/dependency review.
- Visual Studio/MSVC/MSBuild component versions, Windows App SDK templates/NuGet packages, formatter/static-analysis tools, and CI runner images are pinned/recorded when their slice is introduced. vcpkg is added only for an actual reviewed C++ dependency. Experimental Windows App SDK CMake support is not a release prerequisite.

A version being current does not make it automatically acceptable; build support, security, license, footprint, and compatibility are checked. Revalidate the Windows SDK/App SDK/Rust pins on the actual bootstrap date rather than carrying this research snapshot forward mechanically.

## 4. Target repository structure

```text
pastral/
├─ PRODUCT.md
├─ DESIGN.md
├─ README.md
├─ SECURITY.md
├─ CONTRIBUTING.md
├─ CODE_OF_CONDUCT.md
├─ CHANGELOG.md
├─ Cargo.toml
├─ Cargo.lock
├─ rust-toolchain.toml
├─ eng/
│  ├─ build.ps1
│  └─ verify-toolchain.ps1
├─ Directory.Build.props          # added with the native manager slice
├─ Directory.Packages.props       # added with the native manager slice
├─ Pastral.slnx                   # added with the native manager slice; .sln fallback requires recorded tooling evidence
├─ .editorconfig
├─ .gitattributes
├─ .gitignore
├─ .cargo/
│  └─ config.toml
├─ .github/
│  ├─ workflows/
│  │  ├─ rust-ci.yml
│  │  ├─ native-ui-ci.yml
│  │  ├─ security.yml
│  │  ├─ packaging.yml
│  │  └─ release.yml
│  ├─ ISSUE_TEMPLATE/
│  └─ pull_request_template.md
├─ apps/
│  ├─ agent/
│  ├─ worker/
│  ├─ cli/
│  └─ manager/
│     ├─ Pastral.Manager.vcxproj  # added with the manager slice
│     └─ ...
├─ crates/
│  ├─ domain/
│  ├─ clipboard-win/
│  ├─ clipboard-formats/
│  ├─ capture/
│  ├─ paste/
│  ├─ source-context/
│  ├─ classifier/
│  ├─ rules/
│  ├─ profiles/
│  ├─ storage/
│  ├─ search/
│  ├─ security/
│  ├─ ipc/
│  ├─ overlay/
│  ├─ hotkeys/
│  ├─ diagnostics/
│  └─ test-support/
├─ protocols/
│  ├─ ipc-schema/                  # .proto authority added with IPC slice
│  ├─ fixtures/                    # adjacent-version golden bytes
│  └─ versioning.md
├─ database/
│  ├─ migrations/
│  ├─ fixtures/
│  └─ schema.md
├─ tests/
│  ├─ integration/
│  ├─ e2e/
│  ├─ compatibility/
│  ├─ performance/
│  ├─ security/
│  ├─ fuzz/
│  └─ fixtures/
├─ tools/
│  ├─ dev/
│  ├─ benchmarks/
│  ├─ fixture-capture/
│  ├─ diagnostic-viewer/
│  └─ release/
├─ installer/
│  ├─ msix/
│  │  └─ Pastral.Package.wapproj # added with packaging slice
│  ├─ portable/
│  ├─ signing/
│  └─ winget/
├─ assets/
│  ├─ brand/
│  ├─ icons/
│  ├─ store/
│  └─ screenshots/
└─ docs/
```

Directories are created only when a slice has reviewed content. Empty placeholder trees are avoided.

## 5. Bootstrap slice deliverables

### Repository/toolchain

- `rust-toolchain.toml` pinning stable channel/version and required components.
- root Cargo workspace with resolver appropriate to Edition 2024.
- `Cargo.lock` committed.
- no placeholder CMake/native build graph in the pure Rust/domain bootstrap;
- a top-level PowerShell toolchain/build orchestrator that validates and invokes Cargo only for this slice, then later invokes MSBuild after the manager `Pastral.slnx` exists;
- vcpkg manifest/baseline only if a later reviewed C++ dependency actually requires it.
- Windows application manifests/policy templates only when an executable exists.
- reproducible developer environment/version report command.

### First code

Create only `crates/domain` with pure models/invariants needed by later slices:

- strongly typed IDs;
- `Fidelity` enum;
- representation format identity abstraction without Win32 calls;
- immutable `ClipEvent` and `ClipRepresentation` constructors/invariants;
- derived-representation provenance type;
- sensitivity/profile/rule identifiers as value types.

The first code slice does not open the clipboard, database, filesystem, network, pipe, HWND, or COM.

### Tests

- construction and invariant tests written first;
- immutable-original/derived-parent tests;
- fidelity aggregation tests;
- serialization is not added until the IPC/storage schema design selects a format;
- Rust formatting, build, unit tests, Clippy warnings denied, and documentation tests pass.

### Documentation

- developer setup with exact installed components and commands verified on Windows;
- dependency rationale for every root/agent dependency;
- updated architecture only when bootstrap evidence changes a decision;
- milestone report with commands/results/environment.

## 6. Native Windows developer prerequisites

The verified setup guide will include:

- Git;
- rustup with pinned Rust toolchain, rustfmt, Clippy, and Windows MSVC target;
- supported Visual Studio C++ workload and exact individual components;
- selected Windows SDK;
- MSBuild/Visual Studio developer command environment, exact MSVC components, and verified `.slnx` support; use legacy `.sln` only if manager/packaging tooling evidence requires it and record the deviation;
- Windows App SDK stable tooling/templates and NuGet/MSBuild/XAML integration used by manager;
- Windows Application Packaging Project tooling for the multi-executable MSIX;
- vcpkg at a pinned baseline only if used by an actual C++ dependency;
- Windows package/signing test tools without production private keys;
- exact Protocol Buffers v35.0 release-train compiler/schema tools for the prototype plus exact compatible artifacts for each C++/Rust runtime candidate; final resident runtime remains gated by ADR 0018 footprint/build/security evidence;
- Accessibility Insights, Windows SDK inspection tools, WPR/WPA, and debugger for later validation.

The setup command checks versions and exits with actionable mismatches. It never downloads/executes arbitrary tools silently.

## 7. Cargo workspace boundaries

Initial members are added one slice at a time. Dependency direction is enforced through workspace organization and review:

```text
domain
  ↑
platform abstractions / classifier / rules / profiles
  ↑
capture / paste / storage / search / security / IPC
  ↑
agent / worker / CLI
```

`domain` has no Windows, SQLite, async runtime, logging backend, or UI dependency.

Workspace policy after bootstrap:

- centralized package version/edition/license metadata;
- centralized dependency versions;
- default features disabled when they expand the agent unnecessarily;
- no wildcard dependency versions;
- lockfile committed;
- advisory/license/source review in CI;
- unsafe code policy documented per crate.

## 8. C++/WinRT manager bootstrap

The manager starts in a later slice after protocol/view-model contracts exist enough to avoid duplicating domain logic.

Its bootstrap includes:

- C++20 warnings as errors for project code;
- C++/WinRT and WinUI 3 stable package pin;
- x64 Debug/Release MSBuild configurations and deterministic solution/project settings;
- native unit test target;
- UI Automation/accessibility smoke surface using synthetic data;
- no direct SQLite/blob linkage;
- an IPC client abstraction with a fake/in-memory test transport before real pipe integration;
- packaging-compatible app identity separated from provisional display brand.

## 9. CI sequence

### Phase A — bootstrap

- repository formatting/line-ending checks;
- Rust format/build/test/Clippy/doc;
- MSBuild restore/build/XAML compile smoke when the native manager project exists;
- dependency advisory/license/secret scan;
- documentation link/consistency checks.

### Phase B — platform modules

- Windows clipboard fixtures;
- SQLite/migration tests;
- fixed 36-byte frame/Protobuf Edition 2024 schema, DTO-domain conversion, exact runtime-match, adjacent-version, sequenced bulk-transfer, footprint, and IPC security/fuzz smoke;
- manager/UI tests;
- worker restriction/no-network tests;
- package validation.

### Phase C — release

- supported Windows/build/architecture matrix;
- performance smoke and manual report artifacts;
- accessibility/manual evidence checklist;
- clean install/update/uninstall;
- signing/provenance/SBOM where selected;
- release artifact hashes and known limitations.

CI artifact retention is bounded and test-data policy applies to every upload.

## 10. Signing and identity separation

Repository may contain:

- public certificates;
- package manifests with non-secret identity placeholders only after identity reservation;
- signing command templates;
- test certificates generated for local disposable use;
- documentation for secure signing service integration.

Repository/ordinary CI never contains:

- production private key/PFX/password;
- Store credentials/tokens;
- certificate export password;
- DPAPI-wrapped user secrets;
- tunnel/API credentials.

Public package identity and brand display name remain centralized and rebrandable.

## 11. Development data isolation

- Every test run uses a disposable data root.
- Developer/default user Pastral data is never used by tests.
- Fixture producer/consumer uses synthetic content only.
- Database/blob/log/trace/dump/export paths are ignored.
- Destructive recovery/migration tests operate on copies.
- A development reset command prints the exact disposable path before deletion and refuses paths outside the test root.

## 12. Commit sequence

Recommended bootstrap commits:

1. `build: pin Rust workspace toolchain`
2. `test: define domain invariants`
3. `feat: add immutable clip domain model`
4. `ci: verify Rust toolchain and domain`
5. `docs: add verified developer setup`

The Visual Studio manager solution/project and `.wapproj` packaging graph are separate later-slice commits, not empty bootstrap placeholders.

Tests and implementation may be combined into coherent test-first commits when separating them would leave an intentionally failing main branch.

## 13. Bootstrap acceptance criteria

- Windows native setup command reports the exact toolchain versions.
- Fresh clone/configure works from documented commands.
- Domain tests demonstrate failure before implementation in development evidence and pass in final commit.
- Rust format/build/test/Clippy/doc checks pass.
- No CMake/WinUI/MSBuild packaging skeleton is added to the pure domain bootstrap; the later manager slice must pass MSBuild restore/build/XAML compile smoke from documented Windows commands.
- No clipboard, database, network, UI, package, or worker feature is accidentally implemented.
- Dependency graph is documented and small.
- No local launcher, secret, build output, user data, or signing material is tracked.
- Final diff and milestone report match the approved slice.

## 14. Next design after bootstrap

The next independent specification covers domain model, the backend-neutral `BlobStore` contract plus SQLite/FTS5 metadata/search, and lexical search together only if their schema contracts can be reviewed coherently. It must benchmark/version internal SQLite BLOB versus external-file placement rather than hardcoding a universal threshold. Clipboard capture remains a separate specification built around the native fixture producer/consumer and COM apartment evidence.
