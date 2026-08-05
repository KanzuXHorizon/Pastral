# Phase 3I — Production Resident Agent Verification

**Date:** 2026-08-05  
**Scope:** one production resident process combining clipboard capture, storage ownership, authenticated Health/History/Search IPC, bounded shutdown, and startup entry behavior

## Result

**PASS for the implemented resident lifecycle.** This report does not claim paste replay, mutations, encryption, bulk transfer, update handoff, or public release readiness.

## Implemented boundary

```text
pastral-agent.exe [no arguments | run]
  -> resolve local data root
  -> preflight identity/privacy/storage/Health
  -> start authenticated read IPC thread
  -> start clipboard listener/capture loop
  -> shared atomic stop
  -> join both subsystems before exit
```

- `pastral-agent.exe` is the only resident process.
- `%LOCALAPPDATA%\Pastral` is the default root; explicit diagnostic roots must be local absolute non-UNC paths.
- The manager remains an IPC client and never opens storage.
- `pastral-agent-ipc.exe` remains diagnostic-only and is excluded from packaging.

## Failure containment

- A client authentication, protocol, disconnect, or malformed-request failure is contained to that connection.
- Connect timeout is a polling boundary, not a resident fatal error.
- The timed-out first pipe handle is dropped before creating the next `FILE_FLAG_FIRST_PIPE_INSTANCE` handle.
- One preflight Health/storage open completes before capture and IPC split, preventing concurrent first-open/schema initialization races.
- Fatal identity, privacy, storage, listener, transport-material, pipe-security, or first-instance failures terminate the resident process content-free.
- Test-only event/connection limits request coordinated shutdown; production omits the limits.

## Test evidence

Focused additions:

- `resident_root.rs`: explicit/default root resolution and relative/UNC fail-closed behavior;
- `resident_ipc.rs`: stop-before-start, idle timeout survival, rejected-client containment, valid Health after rejection, bounded shutdown, content-free markers;
- `resident.rs`: simultaneous clipboard listener plus authenticated Health/History/Search and clean join.

The resident lifecycle test passed 20 consecutive serialized executions after the preflight race fix. The complete feature-enabled agent suite passed, including adapter, CLI, Health, IPC Health/read, privacy, resident, and runtime tests. `cargo fmt --all -- --check` and Clippy with warnings denied passed.

## Release evidence

Representative resident process after two seconds idle:

| Metric | Evidence |
|---|---:|
| Release binary | 2,484,224 bytes |
| Working set | 12,775,424 bytes |
| Private memory | 3,252,224 bytes |
| Handles | 171 |
| Private-memory budget | 25 MiB maximum — passed |

These are machine-specific development measurements, not a universal SLA.

## Commits

- `1742630 feat: add production resident agent command`
- `a8e75b9 feat: add resilient resident read server`
- `5016edd feat: unify capture and read IPC resident lifecycle`
- `36641f2 fix: keep resident IPC alive across idle timeouts`
- `766e298 feat: default agent startup to resident mode`

## Remaining gates

- normal user-initiated graceful shutdown independent of test bounds;
- update handoff and adjacent-version resident/manager overlap;
- suspend/resume, session transitions, logoff/shutdown, crash recovery, and duplicate-instance UX;
- paste, mutation, bulk transfer, Private-profile encryption, retention and maintenance;
- authoritative startup/idle/clipboard-pressure benchmarks on release hardware.
