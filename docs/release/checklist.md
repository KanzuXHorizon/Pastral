# Release checklist

A release is blocked until every required item is checked with artifact-specific evidence. A checked architecture document is not a substitute for a passing test or measurement.

## 1. Release identity

- [ ] Version, commit, tag, build number, package version, architecture, Windows minimum, and channel agree.
- [ ] Release commit is reviewed and protected according to repository policy.
- [ ] `CHANGELOG.md` describes user-visible changes, security/privacy changes, migrations, and known limitations.
- [ ] Provisional name has passed the formal clearance gate or release uses an approved replacement.
- [ ] Package identity, publisher, display name, executable aliases, URI schemes, data paths, and named-pipe identity are finalized and migration-safe.
- [ ] Public license and third-party notices are approved; no redistribution occurs without them.

## 2. Toolchain and dependency evidence

- [ ] Rust, Windows SDK, Windows App SDK, Visual Studio/C++ toolset, CMake/generator, vcpkg baseline, SQLite, and packaging tool versions are recorded.
- [ ] Lockfiles/manifests match the released source.
- [ ] Dependency advisory scan passes or accepted residual risk is documented.
- [ ] License/source review passes.
- [ ] Secret scan passes.
- [ ] Static analysis and compiler warnings-as-errors pass for project code.
- [ ] SBOM/provenance artifacts are generated when the release process adopts them.
- [ ] Reproducibility metadata and build artifact hashes are retained.

## 3. Build and test gates

- [ ] Rust formatting passes.
- [ ] Rust release build passes.
- [ ] Rust unit/integration/doc tests pass.
- [ ] Clippy passes with warnings denied.
- [ ] C++/WinRT manager release build passes with project warnings as errors.
- [ ] Native UI/unit tests pass.
- [ ] Migration tests pass from every supported prior schema.
- [ ] Clipboard fixture suite passes for required formats/media.
- [ ] End-to-end capture → restart → search → replay passes.
- [ ] IPC serialization/auth/authorization/cancellation tests pass.
- [ ] Worker restriction and no-network tests pass.
- [ ] Security fuzz smoke tests pass; longer campaign results are reviewed.
- [ ] Packaging validation and clean-install smoke pass.
- [ ] Required CI contains no unexplained retry-only success or hidden flaky test.

Record exact commands, pass/fail/skip counts, environment, and artifact hashes.

## 4. Windows support and compatibility

- [ ] Release minimum equals the oldest Windows 11 Home/Pro release still serviced by Microsoft unless a narrower justified support policy is documented.
- [ ] Latest serviced Windows 11 release passes.
- [ ] x64 compatibility matrix is current; ARM64 is absent unless its full gate passes.
- [ ] Runtime capability fallback tests cover APIs newer than the minimum.
- [ ] Console, lock/unlock, suspend/resume, fast user switch, and RDP policy pass.
- [ ] Mixed-DPI/multi-monitor/taskbar/work-area/topology tests pass.
- [ ] Explorer, Notepad, Office, browsers, IDEs, terminals, Discord/Slack, and Win32/WinForms/WPF fixtures have current results.
- [ ] Known compatibility profiles include tested version/date and do not overclaim support.

## 5. Clipboard capture

- [ ] No clipboard polling exists.
- [ ] Listener registration/recovery passes across startup/session transitions.
- [ ] Sequence/coalescing/self-generated-update tests pass.
- [ ] Clipboard contention retries are bounded and measured.
- [ ] Foreign `IDataObject` lifetime is short and tested.
- [ ] Unicode, ANSI fallback, HTML, RTF, URL, file list, DIB/DIBV5/bitmap/encoded image, multiple simultaneous formats, custom/unsupported formats pass expected fidelity outcomes.
- [ ] Delayed rendering, owner exit, rapid updates, malformed lengths/media, large streams, and empty clipboard are contained.
- [ ] Source context obeys redaction/privacy policy.
- [ ] Hard-deny history formats are never captured.
- [ ] Copy continues successfully when Pastral capture/storage/worker fails.

## 6. Storage, search, and recovery

- [ ] Selected SQLite journal/pragmas have crash/backup/AV/low-disk evidence.
- [ ] Blob staging/finalization/reconciliation passes forced termination at each phase.
- [ ] Content-addressed dedup preserves every meaningful occurrence.
- [ ] Sensitive blob identifiers do not reveal plaintext equality.
- [ ] Integrity check and quarantine preserve unaffected data.
- [ ] Low-disk behavior pauses/degrades capture safely and recovers.
- [ ] Retention default is 90 days and 5 GB; pinned exclusion is tested.
- [ ] Cleanup is incremental/cancellable and does not freeze capture/UI.
- [ ] FTS/structured query correctness passes at 1k, 10k, 100k, and release-target scale.
- [ ] Sensitive/excluded content is absent from FTS, suggestions, snippets, OCR, semantic indexes, and duplicate stacks.
- [ ] Backup/restore/export/import round trips and corruption handling pass according to documented scope.

## 7. Paste and replay

- [ ] Replay data object offers the expected safe format set and order.
- [ ] Original, preferred, plain text, derived, copy-only, file/path/name, image, and supported queue modes pass.
- [ ] Delayed-render object lifetime works for required destinations.
- [ ] Self-generated clipboard updates do not create duplicate history.
- [ ] Intended destination is revalidated before synthetic input.
- [ ] Foreground changes cancel wrong-target paste.
- [ ] Failed dispatch leaves a safe manual-paste path where possible.
- [ ] Previous clipboard restoration remains off by default and passes ownership/sequence/async-consumer tests when enabled.
- [ ] Paste failure never changes stored original.
- [ ] Logs contain only metadata/result codes.

## 8. Overlay and Quick Paste

- [ ] Passive overlay never changes foreground, active, or focused HWND.
- [ ] Passive overlay does not appear in taskbar/Alt+Tab.
- [ ] Click does not activate passive overlay.
- [ ] Ordinary typing, IME, number keys, Escape, menus, drag selection, and shortcuts remain unaffected.
- [ ] Placement passes mixed DPI, work area, taskbar, pointer/caret policy, and topology changes.
- [ ] Fullscreen/game/presentation/screen-share/RDP/password/sensitive suppression follows settings.
- [ ] Reduced motion, transparency disabled, battery saver, RDP, and device-loss fallback pass.
- [ ] Copy bursts coalesce without losing event semantics or looping distraction.
- [ ] Quick Paste opens only by explicit invocation and revalidates destination.
- [ ] Search editing, keyboard navigation, selection stability, representation choice, and cancel/focus restoration pass.
- [ ] Sensitive content is absent from visual, UI Automation, task thumbnail, and cache surfaces.

## 9. Profiles, rules, and automation

- [ ] Built-in profile defaults and migrations pass.
- [ ] Profile retention/privacy/index/encryption behavior is enforced by service logic, not UI only.
- [ ] Auto-switch is visible, reversible, and disabled by default unless explicit rule exists.
- [ ] Rule priority, specificity, privacy precedence, conflicts, and simulation pass.
- [ ] No broad scope is silently selected.
- [ ] Remember-choice consent/repeated-choice recommendation behavior passes.
- [ ] Every automatic action exposes what/why/rule/undo/edit/disable.
- [ ] Audit history, import/export, pause, reorder, duplicate, rollback, match count, last result pass.
- [ ] Equal-priority destructive conflicts cannot execute arbitrarily.

## 10. Privacy and security

- [ ] Threat model is updated for released features/dependencies.
- [ ] Default core processes make no network connection in controlled release tests.
- [ ] Password-manager and reliable private-browser exclusions pass.
- [ ] High-confidence synthetic secrets are not stored by default.
- [ ] `SensitiveItemSkipped` contains no value/hash/snippet/OCR/reconstructable content.
- [ ] Opt-in encrypted sensitive retention is narrow, explicit, and excluded from normal indexes/previews/exports.
- [ ] DPAPI user-scope key protection, wrong-user failure, envelope test vectors, tamper detection, and rotation/recovery pass.
- [ ] Named-pipe DACL, user/logon-session isolation, client identity, challenge replay, message limits, and authorization pass.
- [ ] Worker token/sandbox/job/no-network/time/memory/output/child-process limits pass.
- [ ] Path traversal, reparse, decompression, oversized, malformed parser/import tests pass.
- [ ] Secret canaries are absent from logs, dumps, diagnostics, screenshots, exports, CI artifacts, and accessibility tree.
- [ ] Security contact/advisory channel is active and monitored.

## 11. Accessibility and localization

- [ ] All core tasks are keyboard operable with visible focus.
- [ ] UI Automation names, roles, states, patterns, positions, and live regions pass inspection.
- [ ] Narrator core-flow manual test passes.
- [ ] Magnifier, On-Screen Keyboard, IME, sticky/filter-key interactions pass.
- [ ] Light, dark, and contrast themes pass.
- [ ] Text scaling and 100–300% DPI pass without losing consequential controls/content.
- [ ] Reduced motion/transparency disabled pass.
- [ ] RTL and longest supported localized strings pass.
- [ ] Touch target and drag alternatives pass where touch is supported.
- [ ] Sensitive/locked content is absent from UI Automation and hidden/cached views.
- [ ] Accessibility claims in Store/docs match actual tested scope.

## 12. Performance evidence

- [ ] Reference hardware and environment are recorded.
- [ ] Idle 10-minute and 8-hour soak results pass or deviations are approved/documented.
- [ ] No periodic clipboard polling/full scan/network/GPU activity at idle.
- [ ] Agent private working set and dependency attribution are reported.
- [ ] Ordinary capture critical path and durable persistence are reported against budgets.
- [ ] Overlay first frame and CPU animation behavior are reported.
- [ ] Quick Paste warm/cold activation and first results are reported.
- [ ] Search at 1k/10k/100k/release-target scale is reported.
- [ ] Large text/HTML/image/stream memory peak and I/O are reported.
- [ ] Paste construction/publication/destination consumption is reported.
- [ ] Cleanup, migration, integrity, startup, profile switch, suspend/resume are reported.
- [ ] Regressions versus previous accepted release are explained.
- [ ] Marketing/store copy includes only measured claims.

## 13. Package, signing, update, uninstall

- [ ] Multi-executable MSIX manifest contains only required capabilities/declarations.
- [ ] Package identity/publisher/architecture/version are correct.
- [ ] Release package is signed through approved protected process.
- [ ] Signature and publisher verification pass on clean supported systems.
- [ ] Framework-dependent prerequisites and repair/offline behavior are documented/tested.
- [ ] Clean install, first run, startup enable/disable, repair, same-version handling, upgrade, interrupted update, and rollback/recovery pass.
- [ ] Agent/manager/worker/CLI activation and protocol overlap during update pass.
- [ ] Database migration is restart-safe and package rollback limitations are documented.
- [ ] Uninstall behavior clearly offers/implements keep or delete data according to package limitations and documentation.
- [ ] WinGet manifest is absent unless signing/install/update/uninstall behavior is stable and validated.
- [ ] Store assets, declarations, privacy/security/accessibility statements, and screenshots use factual synthetic content.

## 14. Documentation and support

- [ ] README, setup, user guide, privacy, security, backup/restore, diagnostics, recovery, accessibility, shortcuts, settings, known limitations, and release notes match the artifact.
- [ ] ADRs and architecture match implementation.
- [ ] Every public module/unsafe boundary has meaningful documentation.
- [ ] Compatibility matrix and benchmark report are dated/current.
- [ ] Diagnostic bundle creation/preview/redaction/export is documented.
- [ ] Support procedures never request real secrets/history by default.
- [ ] Data location, retention, encryption limits, and deletion/backup limitations are clear.
- [ ] No unresolved placeholder or fabricated claim remains.

## 15. Final artifact review

- [ ] Fresh full required CI passes on release commit.
- [ ] Final diff is reviewed for unrelated changes, duplication, generated files, secrets, signing material, logs, user data, and accidental network code.
- [ ] Release package installed from the exact published artifact passes smoke tests.
- [ ] Artifact hashes/signatures/provenance are verified after upload.
- [ ] Known limitations and residual risks are accepted by the release owner.
- [ ] Rollback/containment path and incident owner are ready.
- [ ] Release approval is recorded with exact evidence locations.
