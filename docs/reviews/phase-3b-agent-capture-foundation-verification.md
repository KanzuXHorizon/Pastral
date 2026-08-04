# Phase 3B Agent Capture Foundation Verification

**Date:** 2026-08-04  
**Baseline:** `b96a003`  
**Implementation head:** `d2fc792`  
**Scope:** transactional capture-order assignment, deterministic capture coordinator, Windows clipboard/storage adapters, diagnostic resident-agent executable, local/CI verification integration

## 1. Result

Phase 3B is **green for its declared scope**.

Pastral now has a real Windows agent executable and an end-to-end ordinary Unicode-text persistence path:

```text
WM_CLIPBOARDUPDATE notification
  -> transient sequence evidence
  -> bounded capture coordinator
  -> short-lived ClipboardSession
  -> exact CF_UNICODETEXT logical UTF-16LE bytes
  -> immutable domain representation
  -> storage-assigned capture order
  -> SQLite metadata/blob + optional FTS5 projection
```

The automated verification path does not read, replace, or publish clipboard content. It builds both agent configurations and runs `health-check` only against a disposable data root.

This phase does **not** establish release readiness, default resident capture, manager live connectivity, broad clipboard fidelity, or sensitive-data safety. Those remain blocked by the exclusions listed below.

## 2. Commits

```text
1faad55 docs: specify agent capture foundation
da2bd2f docs: plan agent capture foundation
c03ddc5 feat: assign capture order in storage
4f6e0a8 feat: add deterministic capture coordinator
d2f71f6 feat: add Windows capture adapters
d2fc792 feat: add diagnostic resident agent
```

Documentation/evidence changes are committed separately after implementation verification.

## 3. Environment

Verified on:

- OS: Microsoft Windows 11 Pro
- OS version/build family: `10.0.26200`
- Windows PowerShell: `5.1.26100.8972`
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `1.97.1 (c980f4866 2026-06-30)`
- Host/target: `x86_64-pc-windows-msvc`
- Visual Studio Build Tools: `17.14.37516.0`
- MSBuild: `17.14.51.32402`
- MSVC toolset: `14.44.35207`
- Windows SDK: `10.0.26100.0`
- Windows App Runtime: `Microsoft.WindowsAppRuntime.2` x64 `2.3.1.0`
- Deferred release SDK `10.0.28000.2526`: not installed and not required for this unpackaged slice

No global toolchain or SDK installation was changed during this phase.

## 4. Implemented components

### 4.1 Storage-owned capture order

`pastral-storage` now exposes:

- `NewClipCommit` without a caller-provided `CaptureOrder`;
- `Storage::commit_new_clip`;
- `AssignedCommitReceipt`.

Assignment occurs inside `TransactionBehavior::Immediate`. The next order is derived from the maximum ordered durable record across both clip events and audit events:

```sql
SELECT COALESCE(MAX(capture_order), 0)
FROM (
    SELECT capture_order FROM clip_events
    UNION ALL
    SELECT capture_order FROM capture_audit_events WHERE capture_order IS NOT NULL
)
```

The new commit path and existing explicit-order path share the same validated metadata/blob/search writer. This avoids two subtly different persistence implementations.

Verified behaviors:

- first new capture receives order `1`;
- next new capture receives order `2`;
- order remains stable after storage reopen;
- a failed capture does not consume an order;
- an explicit imported/test event at order `1` causes the next new capture to receive `2` rather than collide or be silently reassigned;
- existing deduplication, external-file, reconciliation, integrity, FTS, audit, and deletion tests remain green.

### 4.2 Windows-binding-free coordinator

`pastral-agent-core` introduces:

- `CaptureSequence`;
- `CapturedText`;
- `TextCaptureRequest`;
- `StoredCapture`;
- `CaptureConfig`;
- `CaptureCoordinator`;
- `CaptureOutcome`;
- `CaptureSource`, `CaptureSink`, `Clock`, and `Sleeper` ports.

The dependency policy proves this crate remains Windows-binding free.

Default diagnostic retry schedule:

```text
attempt 1: immediate
attempt 2: +5 ms
attempt 3: +15 ms
attempt 4: +35 ms
```

Only transient `Busy` failures are retried. The coordinator:

- suppresses an already successfully handled nonzero sequence;
- treats unsupported/no-text as terminal;
- performs no sink commit after retry exhaustion;
- leaves the same sequence retryable after a storage failure;
- preserves exact logical UTF-16LE bytes;
- does not normalize precomposed/decomposed text;
- contains no async runtime, network stack, logging framework, serialization framework, or Windows binding.

### 4.3 Windows adapters

`apps/agent` includes:

- `WindowsClipboardSource` backed by existing `ClipboardSession` and bounded `CapturedUnicodeText`;
- `StorageCaptureSink` backed by `Storage::commit_new_clip`;
- `SystemClock` and `ThreadSleeper`;
- `AgentIdentity` content-free installation identity;
- `DiagnosticStoragePolicy` using internal SQLite BLOB placement.

The adapter maps:

- clipboard-open contention to retryable `Busy`;
- invalid/oversized/misaligned Unicode or HGLOBAL data to `InvalidData`;
- other platform failures to terminal `PlatformFailure`.

No unsafe block was added. The reviewed unsafe boundary remains `crates/clipboard-win/src/sys.rs`.

### 4.4 Identity file

The selected data root contains `agent-identity.txt` with exactly:

```text
version=1
profile_id=<canonical UUIDv4>
ordinary_domain_id=<canonical UUIDv4>
```

Verified behaviors:

- first open creates the file;
- reopen returns the same IDs;
- staging uses create-new, write, sync, and rename semantics;
- malformed existing identity data fails closed;
- malformed data is not silently replaced;
- no clipboard text, preview, hash, key, or secret is stored in this file.

### 4.5 Ordinary Unicode-text persistence

The sink creates one representation with:

- standard clipboard format ID `13` (`CF_UNICODETEXT`);
- exact logical UTF-16LE bytes including the first aligned NUL terminator;
- `FullFidelity` for that captured representation;
- `sha256-raw-v1` within the ordinary protection domain;
- FTS projection only when decoded text contains a non-whitespace character.

Verified behaviors:

- exact bytes round-trip;
- decomposed text is not normalized;
- non-empty text is searchable;
- empty text persists without an FTS document;
- capture order increases through the storage-owned path.

### 4.6 Diagnostic executable

`pastral-agent.exe` requires one explicit command:

```text
health-check --data-root <path>
capture-current --data-root <path>
listen --data-root <path> [--max-events <positive-integer>]
```

Running without a command returns a nonzero exit code and usage text. Unknown commands, unknown flags, duplicate flags, zero event limits, missing values, and positional extras fail closed.

Command boundaries:

- `health-check` creates/opens identity and storage, then verifies SQLite, FTS, metadata, and search mappings. It does not open the clipboard.
- `capture-current` explicitly performs one bounded current-clipboard capture attempt.
- `listen` explicitly starts the event-driven listener. It does not poll and can be bounded by terminal outcome count for diagnostics.

There is no default command, auto-start registration, Windows service registration, scheduled task, installer mutation, network access, or telemetry.

## 5. TDD evidence

### 5.1 Storage RED

Before production API implementation, focused storage tests failed with:

```text
unresolved import crate::NewClipCommit
no method named commit_new_clip found for struct Storage
```

After implementation, all three new focused behaviors and the full storage suite passed.

### 5.2 Coordinator RED

The integration suite was added before coordinator types. It failed with unresolved imports for:

```text
CaptureConfig
CaptureCoordinator
CaptureOutcome
CaptureSequence
CaptureSource
CaptureSink
Clock
Sleeper
CapturedText
TextCaptureRequest
StoredCapture
```

After implementation, all eight coordinator tests passed.

### 5.3 Adapter RED

Adapter tests initially failed because these product boundaries did not exist:

```text
AgentIdentity
DiagnosticStoragePolicy
StorageCaptureSink
diagnostic_storage_limits
```

After implementation, identity and storage-adapter tests passed without opening the clipboard.

### 5.4 CLI RED

CLI tests initially failed because `AgentCommand`, `CliError`, and `parse_arguments` did not exist. After implementation, all accepted/rejected forms passed.

### 5.5 Health-check RED

Runtime smoke test initially failed because `run_command` did not exist. After implementation, it verified storage creation and all content-free health markers.

## 6. Test results

Fresh workspace test total:

| Package | Tests | Result |
|---|---:|---|
| `pastral-agent` | 7 | PASS |
| `pastral-agent-core` | 8 | PASS |
| `pastral-clipboard-win` | 17 | PASS |
| `pastral-domain` | 26 | PASS |
| `pastral-storage` | 33 | PASS |
| **Total** | **91** | **PASS** |

Zero tests failed, were ignored, or were filtered in the aggregate workspace run.

Agent test coverage includes:

- identity create/reopen;
- malformed identity fail-closed behavior;
- exact Unicode storage and search;
- CLI accepted/rejected forms;
- health-check integrity output;
- immediate coordinator success;
- duplicate suppression;
- exact retry delays;
- retry exhaustion;
- no-representation state;
- storage-failure retryability;
- empty text;
- normalization preservation.

## 7. Build and runtime evidence

### 7.1 Agent builds

```text
Agent Debug and Release builds: PASS
```

Commands:

```powershell
cargo build --locked -p pastral-agent
cargo build --locked -p pastral-agent --release
```

### 7.2 Real health-check smoke

A disposable data root produced:

```text
data-root=C:\Users\KanzuWakazaki\AppData\Local\Temp\pastral-agent-evidence-<random>
agent-health=ok
storage-schema=1
sqlite-integrity=ok
fts-integrity=ok
metadata-integrity=ok
search-mapping-integrity=ok
EXIT_CODE=0
```

The root was removed after the command.

The verifier also confirmed that running the executable without a command returns nonzero.

### 7.3 Clipboard automation safety

Automated gates run only:

```text
pastral-agent.exe health-check --data-root <disposable-root>
```

They do not invoke:

```text
capture-current
listen
```

The static verifier rejects direct automation invocations of those clipboard-reading commands. Unit/integration tests use fakes or synthetic exact bytes and never mutate the system clipboard.

### 7.4 Manager regression

The existing native manager remained green after the agent implementation:

```text
Native manager Debug and Release builds: PASS
Manager UI Automation History navigation, filtering, selection, and no-results states: PASS
Native manager runtime smoke: PASS
```

Observed smoke HWND in this phase: `15668022`.

The manager remains disconnected by design; no direct storage coupling was introduced.

## 8. Aggregate gates

The following passed:

```powershell
.\eng\build.ps1 -Task Agent
.\eng\build.ps1 -Task All
.\eng\build.ps1 -Task Full
.\eng\build.ps1 -Task Manager
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
```

`Full` now means:

1. enforce Rust and native-manager toolchains;
2. formatting/check/test/Clippy/docs;
3. dependency and source policy;
4. agent static, Debug/Release, and health-check smoke;
5. native manager static and Debug/Release build.

Interactive manager UI Automation remains in the separate `Manager` task.

## 9. Policy findings and fixes

### 9.1 Rust workflow repair

The existing `.github/workflows/rust-ci.yml` contained a malformed path entry and duplicate `pull_request` key. Phase 3B replaced it with valid `push` and `pull_request` triggers and added paths for:

- `apps/agent/**`;
- `crates/agent-core/**`;
- `crates/clipboard-win/**`.

The workflow now runs agent verification on `windows-2025`.

Local YAML parse verification passed. A hosted GitHub Actions run was not observed in this session and is not claimed.

### 9.2 PowerShell exit-state repair

The smoke verifier deliberately checks that a missing command exits nonzero. Windows PowerShell 5.1 initially surfaced stderr as `NativeCommandError`, and the expected native exit code remained in `$LASTEXITCODE`, causing the parent build dispatcher to report failure after a successful smoke.

The harness now:

- temporarily allows the intentional invalid invocation;
- records and asserts its exit code;
- restores `ErrorActionPreference`;
- resets native exit state after the assertion.

`-Task Agent` subsequently passed end-to-end.

### 9.3 Source-policy expansion

`eng/verify-source-policy.ps1` previously applied Rust product-source checks only under `crates/`. It now covers `apps/agent/` as well, while keeping unsafe code allowed only in the exact reviewed clipboard `sys.rs` boundary.

### 9.4 Dependency-policy expansion

The dependency verifier now proves:

- `pastral-agent-core`, domain, and storage contain no Windows binding packages;
- `pastral-agent` and `pastral-clipboard-win` use only pinned `windows-sys 0.61.2` and `windows-link 0.2.1`;
- no async, serialization, alternate database, IPC runtime, logger, network client, or high-level Windows binding was introduced.

## 10. Explicit exclusions and skipped checks

Not implemented or not claimed:

- default resident/auto-start capture;
- source application attribution;
- password-manager exclusion;
- private-browser exclusion;
- source-owned clipboard-history exclusion signal handling;
- secret classification or sensitive-skip audit generation;
- encrypted Sensitive/Private retention;
- COM/OLE `IDataObject` capture;
- HTML, RTF, images, file lists, virtual files, or registered application-private formats;
- owner-change race matrix across real third-party applications;
- replay/paste engine;
- origin markers and self-capture suppression;
- Quick Paste or passive overlay;
- versioned named-pipe IPC;
- manager live history/search/status;
- graceful Ctrl+C control protocol;
- process supervision or crash restart;
- retention/quota/background maintenance;
- backup/restore/import/export;
- MSIX, installer, signing, update, or auto-start registration;
- startup, idle CPU, working-set, capture-latency, and throughput benchmarks;
- hostile clipboard-provider fuzzing;
- AV/EDR, low-disk, abrupt-power-loss, RDP, multi-session, lock/unlock, and Fast User Switching matrix;
- hosted CI execution evidence.

`capture-current` and `listen` were compiled but intentionally not invoked during automated verification. Therefore this report does not claim real-user clipboard interoperability evidence for the new executable adapter.

## 11. Residual risks

1. **Privacy policy is not enforced at runtime yet.** Ordinary Unicode text can be explicitly captured without source/private-context or secret exclusion. The agent must not become a default auto-start process before that slice is complete.
2. **No IPC ownership contract is implemented.** Manager remains synthetic/disconnected and cannot observe agent state.
3. **Sequence evidence is transient.** Win32 sequence numbers are used only for duplicate suppression and are not durable event identities.
4. **Only one format is captured.** `CF_UNICODETEXT` success does not imply rich-format or application-private fidelity.
5. **No self-origin suppression exists.** Replay is excluded, but origin markers are required before paste publication begins.
6. **Termination is basic.** An unbounded listener currently relies on external process termination; a versioned control channel is still needed.
7. **Diagnostic limits are safe engineering bounds, not benchmark-selected production defaults.**
8. **Internal SQLite BLOB placement is fixed for this agent slice.** Production placement remains benchmark-gated.

## 12. Next recommended slice

Before enabling default resident capture, implement **Phase 3C: source/privacy admission policy**:

1. capture-source observation with confidence/provenance;
2. source-owned hard-deny signals;
3. password-manager and private-browser fail-closed policy;
4. bounded high-confidence secret detection;
5. content-free skipped-event audit semantics;
6. self-origin marker contract groundwork;
7. real-application compatibility tests that use explicitly controlled fixtures.

After privacy admission is verified, complete ADR 0018 runtime/frame prototypes and implement bounded authenticated per-user IPC so the manager can consume live privacy-safe snapshots without opening storage directly.

## 13. Completion statement

Phase 3B proves that Pastral can coordinate and persist bounded ordinary Unicode-text captures through a real Windows agent executable and existing storage/native boundaries, while keeping automated verification clipboard-safe.

It does not prove that Pastral is a complete clipboard manager or ready for unattended daily use.