# Pastral Resident Agent and MSIX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build one supervised resident Pastral agent and produce a verified, signed Windows 11 x64 development MSIX installer containing the resident agent and native manager.

**Architecture:** Extend the feature-enabled `pastral-agent.exe` with a production `run` command that supervises clipboard capture and authenticated read IPC in one process. Package only that resident executable, the WinUI manager, and its Rust bridge into a deterministic MSIX staged and verified by PowerShell scripts; local development signing exports only the public certificate.

**Tech Stack:** Rust/MSVC, C++/WinRT WinUI 3, Windows named pipes, MSIX manifest, MakeAppx, SignTool, PowerShell, UI Automation.

## Global Constraints

- Work directly on the existing `main` checkout and preserve unrelated changes.
- Windows 11 x64 only.
- Exactly one resident production process: `pastral-agent.exe`.
- The manager must not open SQLite/blob storage directly.
- No service, driver, elevation, network capability, WebView addition, or public portable ZIP release.
- No private signing key may be committed, copied into release artifacts, or exposed in logs.
- Every implementation task follows RED → GREEN → refactor and ends with a focused commit.

---

### Task 1: Production resident command and data-root resolution

**Files:**
- Modify: `apps/agent/src/cli.rs`
- Modify: `apps/agent/src/runtime.rs`
- Modify: `apps/agent/src/lib.rs`
- Modify: `apps/agent/Cargo.toml`
- Modify: `apps/agent/tests/cli.rs`
- Create: `apps/agent/tests/resident_root.rs`

**Interfaces:**
- Produces: `AgentCommand::Run { data_root: Option<PathBuf>, max_events: Option<NonZeroUsize>, max_connections: Option<NonZeroUsize> }`.
- Produces: `resolve_resident_data_root(explicit: Option<PathBuf>) -> Result<PathBuf, AgentRuntimeError>`.
- Keeps diagnostic commands and their explicit roots unchanged.

- [ ] Add CLI tests accepting `run`, optional `--data-root`, and bounded diagnostic limits while rejecting duplicates, zero values, and flags on other commands.
- [ ] Run `cargo test --locked -p pastral-agent --features ipc-health --test cli --test resident_root` and confirm RED because `Run` and root resolution do not exist.
- [ ] Implement strict parsing and `%LOCALAPPDATA%\Pastral` resolution without creating directories during parsing.
- [ ] Run focused tests, formatting, and Clippy; commit `feat: add production resident agent command`.

### Task 2: Stop-aware authenticated resident IPC loop

**Files:**
- Modify: `apps/agent/src/ipc_health.rs`
- Modify: `apps/agent/src/lib.rs`
- Modify: `apps/agent/tests/ipc_read.rs`
- Create: `apps/agent/tests/resident_ipc.rs`

**Interfaces:**
- Produces: `ResidentReadServerConfig` with data root, connect timeout, operation timeout, and optional connection limit.
- Produces: `serve_read_until_stopped(config, Arc<AtomicBool>, output) -> Result<HealthServerSummary, AgentIpcError>`.
- Per-client authentication/protocol/timeout failures are contained; initialization/pipe-publication failures remain fatal.

- [ ] Add tests proving stop-before-start, bounded connection shutdown, a rejected client followed by a valid Health request, and no content-bearing diagnostics.
- [ ] Run focused tests and confirm RED because the resident loop is absent.
- [ ] Refactor shared pipe setup and implement timeout polling plus per-client containment.
- [ ] Run all feature-enabled agent IPC tests and Clippy; commit `feat: add resilient resident read server`.

### Task 3: Supervise capture and IPC in one process

**Files:**
- Modify: `apps/agent/src/runtime.rs`
- Modify: `apps/agent/src/main.rs`
- Modify: `apps/agent/tests/runtime.rs`
- Create: `apps/agent/tests/resident.rs`
- Modify: `eng/verify-agent.ps1` or the repository’s current agent verifier discovered during implementation.

**Interfaces:**
- Produces: `run_resident(data_root, max_events, max_connections, output)` used by `AgentCommand::Run`.
- One shared `Arc<AtomicBool>` stops both capture and IPC.
- Production invocation `pastral-agent.exe run` has no diagnostic limit.

- [ ] Add deterministic lifecycle tests using a disposable root and bounded IPC connection count; prove real authenticated Health/History/Search while the clipboard listener is alive.
- [ ] Confirm RED before implementing supervision.
- [ ] Refactor the capture loop to observe the stop signal every second, run IPC on a named thread, propagate fatal errors, and join cleanly.
- [ ] Build and execute the Release resident smoke; measure binary/private working set with the existing admission tooling and record results.
- [ ] Run agent/workspace focused gates and commit `feat: unify capture and read IPC resident lifecycle`.

### Task 4: Resident architecture and security evidence

**Files:**
- Modify: `docs/adr/0018-ipc-schema-and-framing.md`
- Modify: `docs/adr/0009-packaging.md`
- Modify: `docs/security/threat-model.md`
- Create: `docs/reviews/phase-3i-resident-agent-verification.md`
- Modify: `docs/operations/developer-setup.md`

**Interfaces:**
- Consumes: Task 3 test/build/memory evidence.
- Produces: reviewed package process model, startup boundary, signing policy, and explicit remaining blockers.

- [ ] Record exact commands and measured resident evidence without upgrading unrelated ADR 0018 gates.
- [ ] Document client-failure containment, same-user residual risk, startup-task consent, private-key handling, and uninstall data behavior.
- [ ] Run documentation policy searches and `git diff --check`; commit `docs: verify resident agent lifecycle`.

### Task 5: Deterministic package staging and branded assets

**Files:**
- Create: `packaging/Pastral/AppxManifest.xml.in`
- Create: `packaging/Pastral/README.md`
- Create: `eng/generate-package-assets.ps1`
- Create: `eng/stage-msix.ps1`
- Create: `eng/verify-msix-layout.ps1`
- Modify: `.gitignore`

**Interfaces:**
- Produces staging root under `target/package/Pastral_<version>_x64`.
- Stages exact allowlist: manager files/resources, `pastral-manager-ipc-bridge.dll`, `pastral-agent.exe`, manifest, branded PNG assets.
- Excludes `pastral-agent-ipc.exe`, PDBs, tests, logs, secrets, and development private keys.

- [ ] Add layout verifier first and run it against a deliberately incomplete staging root to confirm RED.
- [ ] Implement reproducible PNG generation from `#725CFF`/`#2ED3FF`, manifest token replacement, Release builds, and exact staging.
- [ ] Validate manifest XML, package file allowlist, executable imports/dependencies, and absence of forbidden artifacts.
- [ ] Run focused staging verification and commit `build: stage deterministic Pastral MSIX`.

### Task 6: Build, sign, inspect, and install development MSIX

**Files:**
- Create: `eng/build-msix.ps1`
- Create: `eng/test-msix-install.ps1`
- Modify: `docs/release/checklist.md`
- Create: `docs/reviews/phase-3j-msix-installer-verification.md`

**Interfaces:**
- Produces: `artifacts/Pastral-<version>-x64.msix`, `Pastral-Development.cer`, checksum, and verification report.
- Development PFX exists only in ignored temporary storage and is deleted after signing unless an explicit local preserve switch is supplied.

- [ ] Run `MakeAppx pack` and `MakeAppx validate`, then unpack and compare the exact staged file/hash set.
- [ ] Create or load a local self-signed certificate whose subject exactly matches the manifest publisher, sign with SHA-256, and verify with SignTool.
- [ ] Install the public certificate into the current-user trusted people store only for the smoke, install the package, launch the manager, verify package/startup-task registration, close it, remove the package, and remove the temporary certificate trust.
- [ ] Generate SHA-256 checksums and a content-free verification report; update the release checklist with proven development-installer gates and remaining public-release gates.
- [ ] Commit `build: produce verified Pastral development installer`.

### Task 7: Full repository release verification

**Files:**
- Review: complete diff and generated artifacts only.

**Interfaces:**
- Consumes all prior tasks.
- Produces final evidence and installer paths.

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run locked agent/IPC/bridge tests and Clippy with warnings denied.
- [ ] Run native manager Static, Debug/Release build, Debug UIA smoke, bridge Probe, and Release live UIA smoke.
- [ ] Run package layout, signature, extraction parity, install/uninstall smoke, checksum, and `git diff --check`.
- [ ] Inspect `git status --short`, installer file hashes/sizes, and recent commits.
- [ ] Use `verification-before-completion` and `finishing-a-development-branch`; report exact completed scope and any remaining public-signing/update/accessibility gates honestly.
