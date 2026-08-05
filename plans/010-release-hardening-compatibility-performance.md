# Plan 010: Close compatibility, security, accessibility, performance, and release gates

> **Executor instructions:** This plan integrates evidence from the exact release candidate; it must not become a place to hide incomplete feature work. Run destructive/install/security tests only in disposable environments. Do not change release claims to match weak evidence—improve evidence or narrow the claim.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- .github eng packaging apps crates protocols docs README.md CHANGELOG.md SECURITY.md Cargo.toml Cargo.lock Directory.* Pastral.slnx`

## Status

- **Priority:** P0 release gate
- **Effort:** XL
- **Risk:** HIGH
- **Depends on:** Plans 005–009
- **Category:** release, security, compatibility, accessibility, performance, documentation
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

Passing unit tests is not sufficient for a Windows clipboard resident that handles private data, owns delayed-render media, injects paste input, spans package updates and promises native accessibility. V1 requires evidence on the exact signed artifact across supported Windows/app versions, malformed IPC/clipboard/storage inputs, long-running idle/capture loads, mixed display/session states, install/upgrade/uninstall and full user journeys. This plan converts the existing comprehensive release checklist into required, reproducible gates and a factual release dossier.

## Release baseline decision

Before testing, update the Windows support decision from current lifecycle evidence. The repository’s engineering floor is Windows 11 24H2/build 26100; V1 should test the oldest Windows 11 Home/Pro release still serviced on the intended release date. Record exact edition/build/patch. Treat device-specific branches as compatibility targets, not automatically as the minimum. Do not use an “expected future version” statement after that version has shipped.

Pin:

- Rust `1.97.1` and lockfile;
- Windows SDK selected by Plan 005;
- Windows App SDK `2.3.1` unless a separately planned upgrade lands with full regression evidence;
- C++/WinRT `3.0.260715.1`;
- Protobuf toolchain/runtime exact versions from ADR 0018;
- x64 architecture and exact package identity/publisher/channel/version.

## Evidence architecture

Create a release-evidence root keyed by version+commit+artifact hash. Every gate writes a small machine-readable result with command, environment, start/end, pass/fail/skip reason, counts and artifact hash. Never include clipboard payload, key material, private path/domain/title or real user history. A release report aggregates existing results; it does not rerun arbitrary commands or infer success from file existence.

## Scope

**In scope**

- required CI and release-candidate orchestration
- parser/IPC/storage/import fuzzing and mutation corpora
- adjacent-version/protocol/package/storage compatibility
- Windows/app/format compatibility matrix
- accessibility/localization/adaptivity/manual evidence
- performance/footprint/energy/idle/scale benchmarks
- security/privacy/network-silence/canary review
- exact package signing/install/upgrade/uninstall/update evidence
- documentation, notices, SBOM/provenance, final release report

**Out of scope**

- Adding unfinished product features during release hardening.
- Broadening claims/support to untested Windows versions, architectures or apps.
- Public upload/Store submission without owner approval, name/license/signing clearance.
- Collecting telemetry or real clipboard data to prove performance.

## Program slices

### Slice 1: Convert the release checklist into executable ownership

Map every checkbox in `docs/release/checklist.md` to:

- automated required gate;
- manual scripted test with evidence template;
- explicit not-applicable reason approved by release owner;
- blocker owned by a prior plan.

No unchecked required item may be hidden in prose. Generate a coverage report that fails when a required item has no owner/result.

**Verify:** release checklist coverage is 100% classified; unresolved blockers stop the release pipeline.

### Slice 2: Add continuous fuzzing and parser hardening

Create isolated fuzz/mutation targets for:

- 36-byte IPC header/stream decoder/state machine;
- Protobuf parse + post-parse DTO conversion for every operation;
- handshake/capability/unknown enum/presence paths;
- storage metadata/archive/import decoders and path handling;
- HTML/RTF/image/file-list/clipboard adapters;
- encrypted envelope/chunk ordering when Plan 008 lands.

Use deterministic bounded mutation smoke on every PR and longer sanitizer/fuzz campaigns on schedule/release candidate. Store only corpus hashes/crash metadata; no secret content. Every found crash becomes a minimized regression fixture.

**Verify:** zero crashes/panics/unchecked growth under required smoke iterations and completed release campaign duration.

### Slice 3: Prove adjacent-version and rolling update compatibility

Maintain golden fixtures and real process matrices for:

- current manager ↔ current resident;
- current ↔ previous supported protocol minor in both directions;
- unsupported major mismatch;
- schema unknown fields/enums/capabilities;
- package N → N+1 while processes are running;
- storage migration from every supported prior version;
- interrupted migration/update/restart;
- recovery/rollback limitations.

Use immutable test artifacts or reproducible tagged builds. Never claim indefinite compatibility; define the exact overlap window.

**Verify:** matrix report records every pair/result and blocks release on an unclassified mismatch.

### Slice 4: Run the Windows and application compatibility matrix

Test minimum supported and latest serviced Windows 11 x64, plus any materially different current device branch. Cover Home/Pro as applicable, clean user, standard user, multiple sessions, lock/unlock, suspend/resume, fast user switch, RDP, mixed DPI/monitors/taskbars and Explorer restart.

Application fixtures include at minimum Explorer, Notepad, Office, major browsers, Visual Studio/VS Code, terminals, Win32, WPF, WinForms and representative custom clipboard owners/consumers. Record exact versions and per-format capture/replay fidelity, destination fallback and known limitations.

**Verify:** all required rows have dated results; unsupported/partial behavior is reflected in compatibility profiles and release notes.

### Slice 5: Complete accessibility and localization evidence

Automated UIA checks cover names, roles, patterns, positions, selection/focus, live regions and protected-content absence. Manual scripted passes cover Narrator, keyboard-only, Magnifier, On-Screen Keyboard, IME, sticky/filter keys, light/dark/contrast themes, 100–300% DPI/text scaling, reduced motion, transparency off, touch alternatives, long VI text and RTL reflow smoke.

Test Home, History, Quick Paste, overlay interactive state, profiles/rules/privacy/storage/recovery and package first-run flows. Passive overlay remains out of focus order and announcement behavior is configurable/coalesced.

**Verify:** no P0/P1 accessibility defect; every manual result includes OS/build, assistive-tech version, steps and evidence reference.

### Slice 6: Measure performance, memory, energy and scale

Use recorded reference hardware and controlled synthetic data. Measure at minimum:

- resident startup, 10-minute and 8-hour idle, private/working set, handles/threads, CPU/GPU/disk/network;
- simultaneous capture+IPC+maintenance and clipboard contention;
- ordinary capture critical path and durable persistence;
- large text/HTML/image/stream peak memory and I/O;
- search/history at 1k/10k/100k/release-target scale;
- manager startup/navigation/scroll and 100k virtualized rows;
- Quick Paste cold/warm first frame/first result, bounded warm lifetime and teardown;
- overlay first frame and zero post-dismiss animation loop;
- replay construction/publication/destination consumption/fallback;
- cleanup, migration, integrity, backup/restore and profile/lock transitions.

Define p50/p95/max budgets before measurement; do not tune by deleting correctness or privacy checks. Compare with last accepted evidence and explain regressions.

**Verify:** all release budgets pass or an explicit scope/claim change is approved.

### Slice 7: Execute security and privacy release review

Update threat/privacy models for all released features/dependencies. Run:

- dependency advisory/license/source and secret scans;
- unsafe boundary review;
- named-pipe ACL/anti-squatting/peer/session/replay/authorization limits;
- source hard-deny and high-confidence secret canaries;
- encrypted Private corruption/wrong-user/lock/cache/UIA tests;
- path/reparse/archive/decompression/malformed parser tests;
- network-silence observation for core processes;
- logs/dumps/diagnostics/screenshots/exports/CI-artifact canary scan;
- package capabilities/publisher/signature review.

Document residual same-user/display-capture/deletion-remnant risks accurately. Security contact/advisory procedures must be active before public release.

**Verify:** no unaccepted high/critical finding and no canary leak.

### Slice 8: Validate the exact signed package lifecycle

Build one release candidate, record its hash, sign through the approved protected process and use that same artifact for:

- signature/publisher/provenance verification;
- clean install/first run/startup registration;
- manager/agent/Quick Paste/tray/overlay activation;
- repair/same-version/upgrade/interruption/protocol overlap;
- storage migration/backup/recovery;
- uninstall keep/delete behavior;
- post-upload hash/signature verification.

Do not rebuild after test and call it the same candidate. Keep signing secrets out of commands/logs/artifacts.

**Verify:** disposable clean systems pass and final artifact hash matches the tested/published hash.

### Slice 9: Finish release documentation and legal/product truth

Update README, changelog, user guide, privacy/security, backup/restore, diagnostics, recovery, accessibility, shortcuts/settings, compatibility, benchmark report, known limitations, install/update/uninstall and support procedures. Resolve or explicitly block on:

- public product name/trademark clearance;
- source/public license and third-party notices;
- package identity/publisher/channel;
- factual Store/marketing copy and synthetic screenshots;
- support/security contact.

“Paste perfectly” remains aspirational and must not be represented as universal fidelity. No AI/cloud/account capability may be implied.

**Verify:** stale-capability/placeholder/fabricated-claim scan passes and docs match the exact artifact.

### Slice 10: Produce final release dossier and approval

Aggregate:

- version/commit/tag/build/package/artifact hashes;
- toolchain/dependency/SBOM/provenance;
- automated/manual test counts and environments;
- compatibility/accessibility/performance/security/package reports;
- migrations and rollback/containment path;
- known limitations/residual risks;
- final diff review and owner approval.

The release command refuses publication without required approvals and exact artifact verification. Publishing/upload remains an explicit owner action.

## Verification commands

The executor must consolidate canonical commands in `eng/build.ps1` or a release orchestrator without duplicating logic. Minimum final sequence:

```powershell
.\eng\build.ps1 -Task Full
# package staging/layout/signature/install gates from Plan 005
# release-scale fuzz/compatibility/a11y/performance/security gates
# exact artifact dossier generation
```

Every command must return nonzero on missing evidence, hidden skip or artifact-hash mismatch.

## Test plan

- PR deterministic fuzz/mutation and scheduled long campaigns.
- Protocol/schema/package/storage adjacent-version matrix.
- Minimum/latest Windows and representative destination/source apps.
- Full UIA plus manual Narrator/keyboard/contrast/scale/RDP/IME matrix.
- 1k/10k/100k/release-scale synthetic datasets.
- idle/soak/suspend/resume/session/Explorer lifecycle.
- security canaries in storage/log/UIA/cache/export/diagnostics/artifacts.
- exact signed clean install/upgrade/interruption/uninstall/reinstall.
- final artifact hash/provenance after publication staging.

## Done criteria

- [ ] Every required release-checklist item has passing evidence or approved not-applicable status.
- [ ] Required fuzz campaigns produce no unresolved crash/panic/resource-exhaustion finding.
- [ ] Adjacent-version and storage migration window is explicit and passes.
- [ ] Supported Windows/app/format matrix is current and honest.
- [ ] No P0/P1 accessibility issue remains; localization/adaptivity matrix passes.
- [ ] Resident, search, Quick Paste, overlay, replay, maintenance and package performance budgets pass.
- [ ] Security/privacy review has no unaccepted high/critical issue and no canary leak.
- [ ] The exact signed artifact passes clean install/upgrade/uninstall and retains its verified hash.
- [ ] Name/license/publisher/support/documentation approvals are recorded.
- [ ] Release dossier and rollback/containment owner are ready.

## STOP conditions

Stop the release if:

- Any required evidence is missing, stale, tests a different artifact or contains an unexplained skip.
- A P0/P1 correctness/accessibility/privacy/security issue remains.
- A parser/fuzz crash, plaintext/key/canary leak or unintended network connection is found.
- Minimum supported Windows or required application fixture fails without narrowed public support.
- Package publisher/signature/hash differs from the tested artifact.
- Migration can make supported rollback/recovery impossible without explicit owner acceptance.
- Public name, license, signing authority or security contact is not approved.

## Maintenance notes

Archive release evidence immutably by artifact hash and compare future candidates against the prior accepted baseline. A release checklist item only stays automated if its command verifies behavior, not source text alone. Do not let scheduled/manual evidence age beyond the compatibility versions it claims.
