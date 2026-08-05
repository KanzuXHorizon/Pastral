# Plan 005: Finish resident lifecycle and reproducible MSIX delivery

> **Executor instructions:** Use an isolated worktree based on the owner’s current resident/MSIX branch or reconcile that work first. Packaging, startup, signing, upgrade and data-path changes are high risk. Do not install packages or trust certificates outside disposable test environments without explicit operator authority.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- apps/agent apps/manager packaging eng .github docs/adr/0005-process-boundaries.md docs/adr/0009-packaging.md docs/adr/0010-update-strategy.md docs/operations docs/release`

## Status

- **Priority:** P1
- **Effort:** L
- **Risk:** HIGH
- **Depends on:** Plan 001
- **Category:** lifecycle, packaging, security, release, DX
- **Reconciled at:** commit `49be43c`, 2026-08-05

## Why this matters

Pastral requires one authoritative resident clipboard owner, authenticated manager connectivity and an install/update/uninstall lifecycle that preserves user data and protocol compatibility. The repository now contains a committed, verified development MSIX foundation, but it remains absent from CI and is not yet a reproducible production release pipeline. Shipping without this plan risks duplicate agents, stale binaries, unsafe startup behavior, broken upgrades and unverifiable signing.

## Current state

- Commits `de24298`–`49be43c` added `packaging/Pastral/AppxManifest.xml.in`, staging/build/registration/install/layout scripts, package assets, documentation and a recorded development installer hash.
- Manifest declares one manager application and a `windows.startupTask` for `pastral-agent.exe` with `Enabled="true"`.
- Stage script builds Release manager and agent, copies XBF/PRI/bridge, generates assets and renders identity/version/publisher.
- Build script packs and development-signs an MSIX, temporarily trusts the certificate, verifies/extracts parity and writes a checksum/report.
- Install smoke verifies shell activation, starts a bounded resident, connects the manager and uninstalls.
- CI still does not watch `packaging/**`, build an MSIX, verify package layout, or run install/upgrade tests.
- `Resolve-SdkTool` searches all installed SDKs and selects the lexicographically highest x64 tool rather than the explicitly supported SDK.
- Production signing, timestamping, protected credential handling, update feed/channel and rollback/migration evidence are absent.
- Product architecture requires no second storage owner and one interactive-session resident.

## Target lifecycle

1. Package install deploys one manager, one resident agent, one bridge and localized resources.
2. First explicit manager launch registers/enables the startup task according to Windows policy and shows its real state; user disablement is respected.
3. Agent is single-instance per logon session, owns capture/storage and serves authenticated IPC.
4. Manager is single-instance/on-demand and activates Home or Quick Paste modes without becoming an unbounded resident.
5. Upgrade supports an explicitly tested adjacent protocol/schema window, quiesces/restarts safely and never rolls storage backward unsafely.
6. Uninstall offers/implements the documented keep/delete-data behavior; package removal alone does not claim secure erasure.
7. Development signing and production signing are separate trust domains.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Layout/stage | `powershell -File eng/stage-msix.ps1 -Version 0.1.0.0 -StagingDirectory <temp>` | exact staged layout PASS |
| Development package | `powershell -File eng/build-msix.ps1 -Version 0.1.0.0 -CreateDevelopmentCertificate -OutputDirectory <temp>` | signed package/checksum/report |
| Install smoke | elevated disposable VM: `powershell -File eng/test-msix-install.ps1 ...` | install/activation/live IPC/uninstall PASS |
| Source/dependency/full | repository canonical gates | all PASS |

## Scope

**In scope**

- resident startup/single-instance/lifecycle code and tests
- package manifest, staging/layout/assets/build/install/upgrade/uninstall scripts
- package/version/identity/publisher configuration
- CI package validation and artifact retention
- packaging/update/process-boundary ADRs and operational docs
- manager surface needed to display/control startup state

**Out of scope**

- Public Store submission or production certificate provisioning without operator credentials.
- Auto-update implementation that downloads from an unapproved network endpoint.
- Changing storage schema solely for packaging.
- Services, elevation, `uiAccess` or machine-wide resident processes.
- Embedding secrets in repository/workflow logs.

## Steps

### Step 1: Reconcile and review the in-progress resident/MSIX work

Open every dirty/untracked file and compare it with ADRs 0005/0009/0010 and the approved resident design docs. Preserve user work; do not recreate scripts from stale plans. Create a file inventory and confirm which files are intended source versus generated package output.

**Verify:** `git status --short` contains no package artifact, PFX, CER, installed-state dump or personal data intended for commit.

### Step 2: Prove the resident single-instance and shutdown contract

Add process-level tests for:

- first resident owns capture and named-pipe first instance;
- second launch activates/acknowledges the existing resident or exits with a stable code, never creates another storage owner;
- session/logon separation;
- manager disconnect/reconnect does not stop capture;
- session lock/unlock, suspend/resume and bounded shutdown;
- package update requests graceful quiescence and has a hard timeout/cleanup path;
- crash/restart preserves storage integrity and re-registers clipboard listener.

Use explicit process IDs/roots and cleanup guards. Automated tests must not read/write the user clipboard unless running a dedicated fixture environment.

**Verify:** focused resident process tests pass repeatedly and leave no process/file lock.

### Step 3: Pin all package build authorities

Make stage/build scripts select exact supported tools:

- Windows SDK version from repository configuration, not highest installed path;
- Windows App SDK and C++/WinRT from locked NuGet restore;
- Rust toolchain from `rust-toolchain.toml`;
- package version derived from one release-version authority with explicit four-component validation;
- architecture fixed to x64 until ARM64 gates exist.

Fail when the exact SDK tool is absent. Record tool hashes/versions in the verification report.

**Verify:** a machine with multiple SDK versions still uses the configured version; removing that version causes a clear fail-closed error.

### Step 4: Harden manifest and startup-task behavior

Validate package identity, publisher, executable paths, capabilities, framework dependencies, localization resources, visual assets and startup extension against the staged payload. Add a manager startup-control surface that reads actual task state and distinguishes enabled, disabled-by-user, disabled-by-policy and unavailable. Never silently re-enable a user-disabled task.

Confirm `Enabled=true` semantics through clean install and first-launch tests; do not assume the resident starts before the application has registered the task.

**Verify:** manifest schema/layout gate, first-launch task registration, sign-out/sign-in startup and user-disable persistence pass in a disposable VM.

### Step 5: Separate development and production signing

Keep self-signed certificate creation explicitly development-only. For production:

- use a protected signing environment/service chosen by the owner;
- avoid passing reusable signing passwords in command lines or logs;
- timestamp signatures using the approved service when available;
- verify publisher/chain/signature after packaging and after upload;
- publish hashes/provenance without private key material.

Scripts should accept an already signed artifact or invoke a CI signing adapter, not embed provider-specific secrets. Document manual/offline fallback and rotation/incident procedures.

**Verify:** development build works without production credentials; release workflow refuses unsigned/mismatched publisher artifacts; secret scan finds no signing material.

### Step 6: Add package CI tiers

Add required pull-request jobs for:

- staging and manifest/layout validation;
- exact payload and dependency inspection;
- unsigned/development-signed pack validation where runner policy permits;
- package extraction parity and artifact hash report.

Add scheduled/self-hosted disposable-VM jobs for elevated install, shell activation, startup after sign-in, live manager IPC, repair, uninstall and cleanup. Include `packaging/**` and all packaging scripts in workflow path filters.

**Verify:** workflow syntax passes; local equivalents pass; CI artifact contains only MSIX/report/checksum/public certificate when development signing is requested.

### Step 7: Implement upgrade and rollback-safe behavior

Create adjacent-version fixtures for agent/manager/protocol/package/storage combinations. Test:

- N → N+1 upgrade with running manager/agent;
- interrupted install/update;
- newer manager with older resident and inverse within supported overlap;
- unsupported mismatch gives clear UI and no data mutation;
- restart-safe storage migration;
- package rollback when storage has not migrated irreversibly;
- documented recovery when rollback cannot restore an older storage reader.

Never promise package rollback equals database rollback.

**Verify:** disposable VM matrix passes with exact artifact versions recorded.

### Step 8: Define uninstall and data retention

Choose and implement documented behavior for package removal:

- default keep user history for reinstall, with an explicit in-app “delete data before uninstall” path; or
- explicit uninstaller choice where Windows packaging permits it.

Ensure agent is stopped before deletion, paths are canonical and reparse-safe, pinned/private data scope is clear, failures are recoverable and deletion is not described as guaranteed physical secure erasure.

**Verify:** keep-data reinstall restores safely; delete-data path removes the intended Pastral root only; unrelated sibling files survive.

### Step 9: Produce release-operational documentation

Update developer setup, package build, signing, clean install, upgrade, uninstall, startup task, diagnostics and incident-response docs. Document provisional identity/name limitations and prohibit public distribution until license/name/signing approvals exist.

### Step 10: Run package and full gates on exact artifacts

Build once, hash once, and use the same MSIX for signature verification, install, activation, live IPC, upgrade/uninstall smoke and artifact report. Do not rebuild between tests and publication.

## Test plan

- resident single-instance and session isolation;
- startup registration/user-disable/policy states;
- exact SDK/tool selection;
- manifest/payload/resource/framework parity;
- development signing and production-signing refusal boundaries;
- clean install, first launch, sign-in startup, manager connection;
- repair/same-version/upgrade/interruption/protocol mismatch;
- keep-data/delete-data/uninstall/reinstall;
- no leftover process, package, certificate or temporary data after tests.

## Done criteria

- [ ] Resident lifecycle has one authoritative owner and bounded restart/shutdown behavior.
- [ ] Package source/scripts are tracked; artifacts/credentials are ignored.
- [ ] Package uses exact pinned toolchain and records versions/hashes.
- [ ] Required CI validates staging/layout/package; disposable VM validates install lifecycle.
- [ ] Startup task respects real Windows state and user choice.
- [ ] Development and production signing paths are separated and secret-safe.
- [ ] Adjacent upgrade/mismatch/uninstall/data-retention tests pass.
- [ ] Full repository and package gates pass against the same commit/artifact.

## STOP conditions

Stop and report if:

- Reconciliation would overwrite active uncommitted resident/MSIX work.
- Package identity/publisher/public name is not approved for the intended distribution channel.
- Upgrade requires an irreversible storage migration without a reviewed recovery plan.
- CI cannot safely install/trust packages; move install tests to an isolated VM rather than weakening host security.
- Startup behavior requires elevation, a service or `uiAccess`.
- A signing credential/value appears in source, plan, log or artifact.

## Maintenance notes

Every release-affecting executable, manifest, protocol or data-path change must rerun package layout and install/upgrade evidence. Package version, product version, protocol overlap and storage migration support must be reviewed together.
