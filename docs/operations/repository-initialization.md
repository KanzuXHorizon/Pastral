# Repository initialization plan

**Status:** Phase 0 plan; no feature scaffold is created by this document.
**Repository root:** `F:\Pastral`

## 1. Current state

- Git repository initialized on `main`.
- Existing local `Start-DevSpace-MCP-Cloudflared.ps1` remains at repository root but is ignored; it is not product source and must not be committed because it is machine/workflow-specific.
- Phase 0 documentation and governance exist.
- No Cargo workspace, CMake project, WinUI application, installer, CI workflow, executable, database, or runtime data exists.
- No public license has been selected; `LICENSE` remains intentionally absent.

## 2. Bootstrap principles

- Bootstrap is its own reviewed vertical slice.
- Pin toolchain/dependency versions in repository files; do not depend on unrecorded global defaults.
- Windows-native build and tests run from a supported Windows developer environment, not WSL alone.
- Keep the agent dependency graph minimal from the first commit.
- Build only a pure domain skeleton and toolchain validation first; clipboard feature work receives its own design/plan.
- No package identity, signing certificate, Store reservation, public crate/package publication, or network integration during bootstrap.
- Generated user data, logs, signing material, tunnel state, package output, and fuzz output remain ignored.

## 3. Initial pinned baseline

Revalidate all versions against official sources on the bootstrap date. Initial decisions:

- Rust stable `1.97.1`, Edition 2024.
- Windows SDK `10.0.28000.2270`, while runtime support floor remains Windows 11 build 26100 and newer API calls require capability checks.
- Windows App SDK `2.3.1` stable for the manager prototype.
- C++20.
- x64 build presets first.
- SQLite compiled with FTS5 and pinned through a reviewed source/package path.
- `windows`/`windows-sys` versions selected and locked after API-surface/dependency review.
- CMake, Ninja/MSBuild generator, vcpkg baseline, formatter/static-analysis tools, and CI runner images are pinned in the bootstrap change after verifying mutual support.

A version being current does not make it automatically acceptable; build support, security, license, footprint, and compatibility are checked.

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
├─ CMakeLists.txt
├─ CMakePresets.json
├─ vcpkg.json
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
│  ├─ ipc-schema/
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
- root CMake project and x64 presets that configure even before the manager is added.
- pinned vcpkg manifest/baseline only for dependencies actually used.
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
- CMake and selected generator;
- Windows App SDK tooling/templates or NuGet/CMake integration used by manager;
- vcpkg at pinned baseline if used;
- Windows package/signing test tools without production private keys;
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
- x64 Debug/Release presets;
- native unit test target;
- UI Automation/accessibility smoke surface using synthetic data;
- no direct SQLite/blob linkage;
- an IPC client abstraction with a fake/in-memory test transport before real pipe integration;
- packaging-compatible app identity separated from provisional display brand.

## 9. CI sequence

### Phase A — bootstrap

- repository formatting/line-ending checks;
- Rust format/build/test/Clippy/doc;
- CMake configure/build smoke when native project exists;
- dependency advisory/license/secret scan;
- documentation link/consistency checks.

### Phase B — platform modules

- Windows clipboard fixtures;
- SQLite/migration tests;
- IPC tests/fuzz smoke;
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
4. `build: add Windows native configure skeleton`
5. `ci: verify foundation toolchains and domain`
6. `docs: add verified developer setup`

Tests and implementation may be combined into coherent test-first commits when separating them would leave an intentionally failing main branch.

## 13. Bootstrap acceptance criteria

- Windows native setup command reports the exact toolchain versions.
- Fresh clone/configure works from documented commands.
- Domain tests demonstrate failure before implementation in development evidence and pass in final commit.
- Rust format/build/test/Clippy/doc checks pass.
- CMake configure smoke passes when its skeleton is added.
- No clipboard, database, network, UI, package, or worker feature is accidentally implemented.
- Dependency graph is documented and small.
- No local launcher, secret, build output, user data, or signing material is tracked.
- Final diff and milestone report match the approved slice.

## 14. Next design after bootstrap

The next independent specification covers domain model, SQLite/blob storage, and lexical search together only if their schema contracts can be reviewed coherently. Clipboard capture remains a separate specification built around the native fixture producer/consumer and COM apartment evidence.
