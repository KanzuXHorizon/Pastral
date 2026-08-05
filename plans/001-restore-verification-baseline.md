# Plan 001: Restore one trustworthy verification baseline

> **Executor instructions:** Work in an isolated worktree. Follow each step and run every verification command. Do not weaken a policy merely to make a gate green. Update `plans/README.md` only when all done criteria pass.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- apps/agent apps/agent-ipc-probe eng .github README.md CHANGELOG.md docs/adr/0018-ipc-schema-and-framing.md`
> Compare the live code with the current-state evidence below. Stop on a material mismatch.

## Status

- **Priority:** P0
- **Effort:** M
- **Risk:** MED
- **Depends on:** none
- **Category:** correctness, tests, architecture, DX, docs
- **Reconciled at:** commit `49be43c`, 2026-08-05

## Why this matters

Pastral still has no single command whose result reliably describes the checkout. Commits through `49be43c` resolved the invalid footprint assumption and dependency/ADR mismatch, but no-argument resident behavior still conflicts with an old agent smoke assertion, and native standalone smoke may test a stale executable. All subsequent work needs one canonical, fresh-artifact baseline before changing high-risk capture, IPC, security, UI, or packaging code.

## Current state

- Commit `76b17c3` changed binary delta to an absolute magnitude, updated tests/policy/ADR, and made focused metrics, dependency and admission gates pass.
- Commit `766e298` makes no-argument parsing start resident `Run`; `eng/verify-agent.ps1` still invokes the agent without arguments and waits for a nonzero exit. `-Mode All` reaches successful builds and then hangs.
- `Pages/HistoryPage.xaml.cpp` uses singular `1 item`; `eng/verify-native-manager.ps1` searches for `1 items`.
- `Resolve-DebugExecutable` can fall back to `apps/manager/Pastral.Manager/x64/Debug/pastral-manager.exe`, so standalone smoke can pass against an old artifact.
- Development MSIX files are now committed and verified, but packaging CI/production delivery remains Plan 005.
- Existing commit convention is conventional commits, e.g. `docs: record final development installer hash`.

## Resolved architectural decision

The repository now accepts authenticated IPC as part of the one production resident agent and records the exact pinned graph and footprint evidence. Preserve that decision: do not restore a second production owner or remove the dependency/footprint ceilings. Remaining work in this plan is verifier authority and documentation consistency.

## Commands you will need

| Purpose | Command | Expected |
|---|---|---|
| Focused metrics tests | `cargo test --locked -p pastral-agent-ipc-probe --test metrics --test cross_process` | all pass |
| Agent tests | `cargo test --locked -p pastral-agent --all-targets --all-features` | all pass |
| Dependency policy | `powershell -File eng/verify-dependencies.ps1` | PASS |
| Admission gate | `powershell -File eng/verify-agent-ipc-admission.ps1 -Mode All` | PASS |
| Manager fresh smoke | `powershell -File eng/verify-native-manager.ps1 -Mode All` | build and UIA smoke PASS |
| Full repository | `powershell -File eng/build.ps1 -Task Full` | exit 0 |

## Scope

**In scope**

- `apps/agent/Cargo.toml`
- `apps/agent/src/cli.rs`
- `apps/agent/tests/cli.rs`
- `apps/agent-ipc-probe/src/metrics.rs`
- `apps/agent-ipc-probe/tests/metrics.rs`
- `apps/agent-ipc-probe/tests/cross_process.rs`
- `eng/verify-agent.ps1`
- `eng/verify-agent-ipc-admission.ps1`
- `eng/verify-dependencies.ps1`
- `eng/verify-native-manager.ps1`
- `eng/build.ps1`
- `.github/workflows/rust-ci.yml`
- `.github/workflows/native-ui-ci.yml`
- `docs/adr/0018-ipc-schema-and-framing.md`
- `README.md`
- `CHANGELOG.md`

**Out of scope**

- Clipboard behavior beyond lifecycle verification.
- IPC wire-schema changes.
- Manager visual redesign.
- Production signing or package installation.
- Relaxing privacy/authentication/size/timeout constraints.

## Steps

### Step 1: Confirm the committed resident startup contract

Commit `766e298` makes no arguments start the production resident and retains explicit command parsing for diagnostic modes. Read the live CLI tests and package startup registration before editing the verifier; do not revert this contract as a shortcut.

**Verify:** `cargo test --locked -p pastral-agent --test cli --test resident_root` → all pass.

### Step 2: Re-verify the committed footprint correction

Commit `76b17c3` defines binary delta as an absolute magnitude and covers smaller/larger binary orderings. Re-run the focused tests and inspect the implementation for zero/overflow handling. Do not replace it unless a new measured semantic requirement exists.

**Verify:** `cargo test --locked -p pastral-agent-ipc-probe --test metrics --test cross_process` → all pass.

### Step 3: Re-verify the accepted resident dependency contract

Confirm the default resident contains only the exact pinned authenticated IPC/Protobuf graph, while domain/storage/clipboard/agent-core/ipc-auth/ipc-core retain their recorded isolation. Confirm ADR 0018 and admission evidence describe the same contract.

**Verify:** `powershell -File eng/verify-dependencies.ps1` and `powershell -File eng/verify-agent-ipc-admission.ps1 -Mode All` → PASS. During reconciliation these gates passed with an 8,704-byte Release binary delta and all admission ceilings satisfied.

### Step 4: Align agent smoke with resident CLI behavior

Replace the unbounded direct no-argument invocation in `verify-agent.ps1` with a bounded test appropriate to the selected contract:

- resident contract: start process with a disposable root/bounds, verify it remains alive and serves expected state, then stop via the supported bound/cleanup path;
- mandatory-command contract: retain the exit-code assertion with an explicit timeout and process cleanup.

Every spawned process must be killed/waited in `finally`. The gate must never hang.

**Verify:** `powershell -File eng/verify-agent.ps1 -Mode All` → PASS within the documented timeout and leaves no Pastral test process.

### Step 5: Make native smoke artifact-exact

Change `verify-native-manager.ps1` so canonical smoke requires an explicit freshly built executable or builds one in the same run. Remove stale `x64\Debug` fallback from CI/release authority; an optional developer-only fallback must be visibly named and never used by `All`.

Update the singular assertion from `1 items` to the actual localized singular contract, preferably by finding a stable automation state rather than English copy. Add a test/assertion that the executable path begins under the unique `target\verification\pastral-native-manager-*` root in `All` mode.

**Verify:** run `powershell -File eng/verify-native-manager.ps1 -Mode All` twice after deliberately leaving an older binary in `x64\Debug`; both runs test the unique fresh artifact and pass.

### Step 6: Make CI call the canonical gates

Ensure Rust CI and native UI CI invoke the same authoritative scripts and do not duplicate contradictory assumptions. Add native runtime smoke where the runner supports interactive desktop UI; otherwise create a clearly separated scheduled/self-hosted UIA job and keep fresh artifact static/build gates required on pull requests.

Include `packaging/**` in path filters only when Plan 005 lands the package gate; do not add an empty trigger with no job.

**Verify:** validate workflow syntax and run local script equivalents. Expected: no workflow command refers to an obsolete Health-only/default-agent premise.

### Step 7: Refresh capability documentation

Update README capability matrix, architecture paragraph, CHANGELOG known limitations, and ADR evidence so they state the implemented live History/Search bridge and the selected resident dependency/lifecycle contract. Preserve honest limitations for paste, encryption, rich formats, Quick Paste, overlay and release delivery.

**Verify:** `grep -RIn "bridge exposes content-free Health only\|manager C ABI/UI integration remains unavailable" README.md CHANGELOG.md docs/adr` → no stale claims.

### Step 8: Run the full baseline from a clean isolated worktree

Run all focused commands, then the full gate. Inspect test-created processes and the final diff.

**Verify:** `powershell -File eng/build.ps1 -Task Full` → exit 0; `git status --short` contains only intended files; no Pastral test process remains.

## Test plan

Add regression coverage for:

- negative, zero, positive and overflowing binary deltas;
- current real binary ordering;
- no-argument resident contract and bounded cleanup;
- default dependency graph exact allow/deny set;
- combined resident capture+IPC footprint;
- native smoke refusing/failing on stale artifact paths;
- localized singular result state without English-copy fragility;
- documentation assertions for current History/Search capability.

## Done criteria

- [ ] Every focused command in the command table exits 0.
- [ ] `eng/build.ps1 -Task Full` exits 0 in a fresh worktree.
- [ ] Admission metrics support both binary-size orderings without hiding positive growth.
- [ ] Agent dependency policy, Cargo defaults, ADR 0018 and package lifecycle describe one consistent production design.
- [ ] Agent smoke cannot hang on no-argument invocation.
- [ ] Native `All` smoke proves the executable path is freshly built and cannot fall back to stale `x64` output.
- [ ] README/CHANGELOG accurately describe live History/Search.
- [ ] No privacy/security ceiling is removed without measured replacement evidence.
- [ ] No files outside scope are modified.

## STOP conditions

Stop and report if:

- The resident/MSIX dirty changes are owned by another active task and cannot be reconciled safely.
- Combined resident footprint exceeds the documented budget and no accepted ADR decision exists to change it.
- The only way to make a gate pass appears to be weakening peer authentication, capability checks, privacy admission, bounds, or content-free output.
- Hosted native UI smoke cannot obtain an interactive desktop; separate that job rather than silently skipping it.
- A verification step fails twice for distinct unexplained reasons.

## Maintenance notes

- Any future default-feature change must update dependency policy and resident footprint evidence in the same change.
- Canonical verification must consume artifact paths, not discover “the newest-looking” binary.
- Reviewers should reject documentation that describes a capability state different from the code/gates.
