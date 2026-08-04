# Pastral Privacy Admission Foundation Implementation Plan

> **Required workflow:** execute task-by-task with TDD and fresh verification before each commit.

**Goal:** enforce source-owned hard-deny controls, fail-closed source policy, and narrow high-confidence secret skipping before ordinary Unicode text is persisted.

**Architecture:** extend the existing clipboard session with bounded control/owner observation, keep policy and secret classification in Windows-binding-free `pastral-agent-core`, return explicit admission outcomes through the coordinator, and persist only content-free sensitive-skip audits.

**Tech stack:** Rust 1.97.1, Edition 2024, pinned `windows-sys 0.61.2`, SQLite/FTS5, Windows PowerShell verification.

## Global constraints

- No automated test may read, replace, or publish the user's clipboard.
- Unsafe remains restricted to `crates/clipboard-win/src/sys.rs`.
- Hard-deny signals are evaluated before payload acquisition.
- Full process paths, process IDs, titles, and clipboard content are never persisted or printed.
- `pastral-agent-core`, domain, and storage remain Windows-binding free.
- No new third-party dependency.
- Agent remains manual/diagnostic and manager remains disconnected.

---

## Task 1: Clipboard history controls and owner observation

**Files:**
- Create: `crates/clipboard-win/src/history_controls.rs`
- Create: `crates/clipboard-win/src/owner.rs`
- Modify: `crates/clipboard-win/src/error.rs`
- Modify: `crates/clipboard-win/src/lib.rs`
- Modify: `crates/clipboard-win/src/session.rs`
- Modify: `crates/clipboard-win/src/sys.rs`

### Step 1: Add RED tests

Add pure tests for:

- absent controls;
- exclude-format availability;
- DWORD zero/one parsing;
- malformed length and unsupported DWORD value;
- owner-observation public type behavior.

Run:

```powershell
cargo test --locked -p pastral-clipboard-win history_controls -- --nocapture
```

Expected: compile failure because the new types/functions do not exist.

### Step 2: Implement bounded control parsing

Add exact constants:

```text
ExcludeClipboardContentFromMonitorProcessing
CanIncludeInClipboardHistory
CanUploadToCloudClipboard
```

`ClipboardSession::history_controls()` registers runtime IDs, checks availability, and reads exact four-byte DWORD values for the latter two controls. Invalid available control data returns a dedicated error and is never treated as permission.

### Step 3: Implement owner process observation

Inside `sys.rs` only:

- call `GetClipboardOwner`;
- obtain PID through `GetWindowThreadProcessId`;
- open with `PROCESS_QUERY_LIMITED_INFORMATION`;
- query a maximum 32,768 UTF-16 unit image path;
- close every opened handle;
- return `Unavailable` for absent/inaccessible/exited owners.

Expose `ClipboardSession::owner_process()`.

### Step 4: Verify and commit

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-clipboard-win --all-targets
cargo clippy --locked -p pastral-clipboard-win --all-targets -- -D warnings
.\eng\verify-source-policy.ps1
git diff --check
```

Commit:

```text
feat: inspect clipboard privacy controls
```

---

## Task 2: Windows-free source and secret admission policy

**Files:**
- Create: `crates/agent-core/src/privacy.rs`
- Modify: `crates/agent-core/src/lib.rs`
- Modify: `crates/agent-core/src/ports.rs`
- Modify: `crates/agent-core/src/coordinator.rs`
- Modify: `crates/agent-core/tests/coordinator.rs`
- Create: `crates/agent-core/tests/privacy.rs`

### Step 1: Add RED policy tests

Test:

- unresolved source denied when configured;
- resolved unlisted process allowed;
- exact case-insensitive denied executable rejected;
- path-like deny entries rejected;
- duplicate deny entries normalized;
- all private-key envelope markers detected;
- text over 1 MiB returns detector-limit classification;
- ordinary text allowed;
- hard-denied source performs no sink call;
- policy-denied source performs no sink call;
- sensitive sink result maps to `CaptureOutcome::SensitiveSkipped`.

### Step 2: Implement source policy and detector

Add:

- `SourceConfidence`;
- `SourceObservation`;
- `SourceAdmissionPolicy`;
- `SourceAdmissionDecision`;
- `SensitiveClass`;
- `detect_high_confidence_secret`.

Executable entries are basename-only, <=260 UTF-8 bytes, exact case-insensitive matches.

### Step 3: Extend ports/outcomes

Add:

```rust
CaptureSourceError::HardDenied
CaptureSourceError::PolicyDenied
CaptureSinkOutcome::{Stored, SensitiveSkipped}
CaptureOutcome::{HardDenied, PolicyDenied, SensitiveSkipped}
```

Hard/policy deny marks sequence handled and performs no sink call. Storage failure remains retryable.

### Step 4: Verify and commit

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-agent-core --all-targets
cargo clippy --locked -p pastral-agent-core --all-targets -- -D warnings
.\eng\verify-dependencies.ps1
```

Commit:

```text
feat: add privacy admission policy
```

---

## Task 3: Content-free sensitive audit and admission-aware adapters

**Files:**
- Modify: `crates/storage/src/repository.rs`
- Modify: `apps/agent/src/platform.rs`
- Modify: `apps/agent/src/storage_sink.rs`
- Modify: `apps/agent/tests/adapters.rs`

### Step 1: Add RED tests

Test:

- private-key fixture returns `SensitiveSkipped`;
- no clip/search result is created;
- audit count increases by one;
- ordinary text still stores;
- detector-over-limit content skips;
- hard/policy source decisions are made before Unicode acquisition using fake policy/control inputs where possible.

### Step 2: Add content-free audit count

Expose:

```rust
Storage::capture_audit_event_count() -> Result<u64, StorageError>
```

The API returns only a count.

### Step 3: Integrate sink detector

Before digest/representation creation:

- detect private-key material or detector-limit overflow;
- insert `SensitiveItemSkipped` audit with no capture order;
- return `CaptureSinkOutcome::SensitiveSkipped`;
- never create a clip, digest, blob, FTS row, or preview.

### Step 4: Integrate Windows source admission

`WindowsClipboardSource` receives `SourceAdmissionPolicy`.

Per attempt:

1. open clipboard;
2. inspect history controls;
3. return `HardDenied` for exclude or include-history false;
4. observe owner process basename;
5. evaluate source policy and return `PolicyDenied` when denied;
6. only then acquire Unicode text.

Any malformed available history control fails closed as `HardDenied`.

### Step 5: Verify and commit

```powershell
cargo fmt --all -- --check
cargo test --locked -p pastral-storage --all-targets
cargo test --locked -p pastral-agent --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
.\eng\verify-source-policy.ps1
```

Commit:

```text
feat: enforce privacy before persistence
```

---

## Task 4: Strict privacy config and runtime integration

**Files:**
- Create: `apps/agent/src/privacy_config.rs`
- Modify: `apps/agent/src/lib.rs`
- Modify: `apps/agent/src/runtime.rs`
- Create: `apps/agent/tests/privacy_config.rs`
- Modify: `eng/verify-agent.ps1`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Create: `docs/reviews/phase-3c-privacy-admission-foundation-verification.md`

### Step 1: Add RED configuration tests

Test:

- first creation writes exact version/defaults;
- reopen is stable;
- default unresolved-source deny is true;
- default baseline deny entries exist;
- user-added exact basenames parse case-insensitively;
- malformed/unknown/missing/path-like fields fail closed and are not overwritten.

### Step 2: Implement strict config

Create `privacy-policy.txt` atomically using the same staging discipline as identity creation.

Defaults:

```text
version=1
deny_unresolved_source=true
deny_process=1password.exe
deny_process=bitwarden.exe
deny_process=keepass.exe
deny_process=keepassxc.exe
```

### Step 3: Wire runtime

All clipboard-reading commands load privacy config and construct `WindowsClipboardSource::new(policy)`. `health-check` validates the config without opening the clipboard and reports only `privacy-policy=ok`.

Output adds content-free classes:

```text
capture-outcome=hard-denied
capture-outcome=policy-denied
capture-outcome=sensitive-skipped
```

### Step 4: Extend verification

Static/smoke verifier requires the privacy config, hard-deny format names, detector bound, and health marker. Smoke remains health-check only.

### Step 5: Fresh aggregate verification

```powershell
.\eng\verify-agent.ps1 -Mode All
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
cargo tree --locked --workspace
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
git diff --check
git fsck --no-progress --no-dangling
git show --check HEAD
```

### Step 6: Documentation/evidence and commits

Document exact enforcement and exclusions. Do not claim private-browser detection, comprehensive secret classification, or release-ready unattended capture.

Implementation commit:

```text
feat: integrate agent privacy policy
```

Evidence commit:

```text
docs: record privacy admission foundation evidence
```

Final expectation: all gates pass and working tree is clean.