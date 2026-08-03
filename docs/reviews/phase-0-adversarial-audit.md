# Phase 0 adversarial audit and hardening record

**Date:** 2026-08-04
**Reviewed baseline:** commit `34451221a2d604fd693161e8422cbfaefbdc1220`
**Scope:** factual freshness, clipboard/OLE correctness, process/thread boundaries, durable identities, source confidence, IPC security, encryption, Quick Paste lifecycle, paste dispatch, privacy, performance, tests, packaging assumptions, and brand risk.

## Conclusion

The Phase 0 baseline was a useful design boundary, but it was not sufficiently precise to begin native clipboard implementation safely. This audit found several architectural gaps and two factual/wording errors that could otherwise produce incorrect code or overstated security guarantees.

Phase 0.1 corrects those issues without adding feature code. The original Phase 0 consistency review remains a historical record of what was checked at commit `3445122`; this document supersedes its readiness conclusion.

## Severity definitions

- **Critical:** likely to create a false security boundary or irreversible disclosure/corruption assumption.
- **High:** likely to cause hangs, lost data, wrong replay, privacy errors, or an implementation split later.
- **Medium:** materially weakens testability, UX correctness, or release accuracy.
- **Low:** clarity/governance issue without immediate runtime risk.

## Findings

### F-01 — Same-user IPC security was overstated

**Severity:** Critical

**Affected baseline:** threat-model objective, ADR 0008, encryption key hierarchy, incident severity language.

**Problem:** A pipe DACL using the current logon SID can isolate normal client access to one logon session, while runtime token checks verify the expected user/session. Adding a broad user-SID allow ACE alongside it would weaken that isolation because allow ACEs are additive. Even with the correct logon-SID DACL, arbitrary processes already running in that same unlocked session share the user's authority, and a DPAPI user-scope installation secret is normally available to code under those credentials. Therefore DACL + DPAPI challenge-response cannot honestly promise confidentiality from a fully compromised same-user process.

**Correction:**

- Make cross-user/cross-session isolation the strong OS boundary.
- Treat process identity, binary/package checks, nonces, and installation-secret binding as anti-confusion, stale-client, replay, and defense-in-depth controls.
- State that same-user malware is outside a strong confidentiality boundary for ordinary history.
- Require explicit foreground user intent and later Windows Hello freshness for sensitive reveal/export/destructive operations.

**Residual risk:** A malicious process already running as the user can often inspect the user's files, invoke user-scope DPAPI, inject into allowed processes, or observe the unlocked desktop. Pastral cannot convert an unlocked same-user environment into a secure enclave.

### F-02 — Foreign OLE calls could block the agent control thread

**Severity:** High

**Affected baseline:** process model, clipboard lifecycle, threat T2, performance capture path.

**Problem:** The baseline allowed one STA-capable message thread to own clipboard/OLE and overlay/tray work. `IDataObject::EnumFormatEtc`, `GetData`, delayed rendering, and Win32 rendering callbacks can block or re-enter. COM call cancellation is opt-in, may degrade performance, and is not guaranteed to be supported or honored by the server. Running these calls on the control/overlay thread could freeze hotkeys, tray, overlay, session handling, and capture health.

**Correction:**

- Keep the control/overlay message thread free of foreign `IDataObject` calls.
- Use a dedicated capture STA with its own message pump.
- Use Win32 standard-format capture as the immediate baseline and OLE only for adapters that require `FORMATETC`, `lindex`, or `IStream` semantics.
- Treat COM call cancellation as best effort.
- Add a watchdog/degraded-state policy and a review trigger for a separate capture broker if a stuck capture STA cannot be recovered safely.

### F-03 — Clipboard notifications are not durable copy-event identities

**Severity:** High

**Affected baseline:** coalescing, event identity, performance burst claims.

**Problem:** `WM_CLIPBOARDUPDATE` carries no sequence or source identity. `GetClipboardSequenceNumber` returns the current sequence for the window station, can return zero when access is unavailable, and delayed rendering affects increments. If multiple updates occur before processing, the listener can observe only the latest state. Arithmetic gaps cannot be equated to user copy count, and 32-bit wrap must not be treated as corruption.

**Correction:**

- Introduce transient `ClipboardObservation` records separate from durable clips.
- Compare sequence values for equality and transaction ownership; never use them as globally unique IDs.
- Record coalescing/gap diagnostics without claiming a precise number of missed copies.
- Require the final current clipboard state to be captured under burst fixtures while acknowledging that intermediate states may be unobservable.

### F-04 — Registered clipboard numeric IDs were treated as durable

**Severity:** High

**Affected baseline:** paste lifecycle and implicit data model.

**Problem:** Values returned by `RegisterClipboardFormat` are runtime/system registration identifiers, not a stable serialized product identity. Persisting only a numeric ID can replay the wrong meaning after restart or on another system state.

**Correction:** Persist standard clipboard formats by defined standard ID and registered formats by exact registered name. Re-register names at replay time and retain runtime numeric IDs only as transient diagnostics.

### F-05 — No normative domain identity/time/hash model

**Severity:** High

**Affected baseline:** `ClipEvent` cardinality, skipped/failed captures, migrations, IPC contracts, deduplication, query dates, audit records.

**Problem:** The glossary did not define whether a `ClipEvent` could contain zero representations, how denied/failed observations differ from captured clips, which IDs are stable, how wall clock and monotonic time are used, or which digest and protection domain define content addressing.

**Correction:**

- Successful `ClipEvent` requires at least one `ClipRepresentation`.
- Denied, failed, coalesced, and sensitive-skip outcomes use `CaptureAuditEvent`.
- Use CSPRNG-generated opaque 128-bit public IDs; SQLite row IDs are not protocol identities.
- Persist UTC microseconds; use monotonic clocks only for durations/deadlines.
- Use versioned SHA-256 for ordinary raw blob content addressing.
- Use random identifiers and no plaintext digest/deduplication by default for Private/sensitive payloads.

### F-06 — Clipboard format safety policy was too broad

**Severity:** High

**Affected baseline:** custom registered formats, `CF_BITMAP`, shell/OLE formats, `STGMEDIUM` ownership.

**Problem:** “Opaque safe serialization” was not defined. Raw custom bytes may contain process-local pointers, handles, serialized objects, or callbacks. `CF_BITMAP` is a GDI handle, not durable bytes. Virtual files require `FileGroupDescriptor`/`FileContents` plus `lindex` and medium handling. Blind replay is unsafe.

**Correction:** Add an adapter capability matrix. Unknown custom formats are metadata-only/unsupported by default. Store registered names, copy/normalize only through reviewed adapters, release `STGMEDIUM` on its owning apartment, and label handle/reference limitations honestly.

### F-07 — Source attribution lacked a normative confidence model

**Severity:** High

**Affected baseline:** source-aware rules/search/privacy.

**Problem:** Clipboard owner, foreground window at notification time, and an explicit browser/IDE integration are different evidence classes. Window titles are not a reliable domain/project source and can leak private document names.

**Correction:** Store source claims with evidence type and confidence. Do not infer domain/project from title text by default. Missing/ambiguous owner remains `Unknown`, not guessed.

### F-08 — Quick Paste had no process host

**Severity:** High

**Affected baseline:** four-executable architecture, WinUI decision, warm-open target.

**Problem:** Quick Paste was specified as WinUI-like UI but not assigned to an executable or lifecycle. A separate resident UI would violate the one-resident-process principle, while a cold WinUI launch cannot be assumed to meet the warm target.

**Correction:** Host Quick Paste as an activation mode/window in the single-instance, on-demand `pastral-manager.exe`. “Warm” means the UI process is already alive. Cold activation is measured separately. A dedicated lightweight executable is added only after evidence and a new ADR.

### F-09 — Elevated destination/UIPI behavior was missing

**Severity:** High

**Affected baseline:** paste transaction, compatibility matrix, threat T12.

**Problem:** `SendInput` is subject to UIPI and can inject only into equal/lower integrity processes. Failure does not reliably identify UIPI through `GetLastError`. Foreground restoration is also restricted by Windows policy.

**Correction:** Pastral remains standard user, does not request `uiAccess`, elevation, or a service, and does not use focus-stealing hacks. If restoration/injection is blocked or uncertain, Pastral leaves data on the clipboard and asks for manual paste.

### F-10 — Hard-deny source signals could still create durable audit rows

**Severity:** High

**Affected baseline:** clipboard lifecycle state machine and privacy precedence.

**Problem:** The baseline allowed a denied capture to create an optional metadata-only event. For source-owned history-exclusion formats, even durable metadata can contradict the product's stronger promise to honor the source's request.

**Correction:** Source-owned hard deny produces no durable clip or audit row. Only ephemeral counters/health information may exist. User/app deny and high-confidence secret skips have separately defined metadata behavior.

### F-11 — Sensitive-skip metadata was underspecified

**Severity:** Medium

**Affected baseline:** user-selected default A, privacy model, search/history UX.

**Problem:** “Optional metadata-only” did not specify default state, source precision, timestamp precision, or retention.

**Correction:** High-confidence secret skip creates a hidden `SensitiveItemSkipped` audit record by default containing policy/detector class, active profile, coarse timestamp, and no payload/hash/snippet/OCR/title/path/domain. Default retention is 24 hours. Users can disable these records or shorten retention.

### F-12 — Private profile guarantees were ambiguous

**Severity:** High

**Affected baseline:** MVP 3 Private profile versus later Windows Hello vault UX.

**Problem:** A built-in `Private` profile was listed before a clear mandatory encryption/indexing contract. Windows Hello was deferred, but that does not justify an unencrypted Private profile.

**Correction:** The built-in Private profile is unavailable until encrypted payload storage exists. From introduction, it uses encrypted payloads, random blob IDs, hidden previews, no ordinary FTS/OCR/semantic indexing, no ordinary export, and key clearing on lock according to policy. Windows Hello adds user-presence verification later; it is not the base encryption primitive.

### F-13 — Encrypted streaming could expose unauthenticated plaintext

**Severity:** High

**Affected baseline:** encryption read path.

**Problem:** A whole-message AEAD tag verified only at the end cannot safely stream plaintext to a consumer before authentication. “Where the API permits” was too weak.

**Correction:** Either buffer the complete plaintext within a strict bound before release, or use independently authenticated chunks. A chunk is not released until its tag verifies; AAD binds object identity, chunk index/count, lengths, and envelope version.

### F-14 — Default quota semantics were contradictory

**Severity:** Medium

**Affected baseline:** “5 GB quota” and pinned exemption.

**Problem:** If pinned items are never auto-deleted, 5 GB cannot be a hard storage cap.

**Correction:** Define 5 GB as the automatic-cleanup target for ordinary unpinned history. Pinned/protected data may exceed it with visible warnings. A separate low-disk reserve pauses new payload capture; Pastral never silently deletes pinned content to meet the target.

### F-15 — SQLite/FTS deletion remnants needed explicit treatment

**Severity:** Medium

**Affected baseline:** deletion/privacy and journal-mode evaluation.

**Problem:** Deleted text may remain in freelists, rollback journals, WAL, snapshots, or backups. Physical overwrite on SSD was mentioned, but FTS/journal-specific policy and tests were not.

**Correction:** Evaluate `secure_delete`, incremental/auto vacuum, checkpointing, journal retention, and export/backup copies with measured trade-offs. UI deletion language remains logical deletion, never guaranteed forensic erasure.

### F-16 — Windows SDK pin was stale on the research date

**Severity:** Medium

**Affected baseline:** research report, initialization plan, Phase 0 plan.

**Problem:** Phase 0 pinned Windows SDK `10.0.28000.2270` from June 2026, but Microsoft listed stable `10.0.28000.2526` for July 2026 by the 2026-08-03/04 research date.

**Correction:** Update the bootstrap pin to `10.0.28000.2526`. Runtime minimum remains build 26100 and newer APIs still require availability checks/fallbacks.

Official source:

- https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads

### F-17 — Name-clearance risk omitted a close same-category product

**Severity:** Medium

**Affected baseline:** provisional-name report.

**Problem:** `Pastry` currently markets itself as a clipboard-history/search product. Although not an exact match, it is phonetically and visually close to `Pastral` in the same product category.

**Correction:** Raise rebrand risk and require confusing-similarity review before logo/package/store/public beta work. This is not a legal infringement conclusion.

Observed source:

- https://www.getpastry.app/

### F-18 — Screen-capture exclusion could be interpreted as a guarantee

**Severity:** Medium

**Affected baseline:** privacy/UX requirements.

**Problem:** Windows display-affinity APIs are defense in depth and do not prevent every capture path, camera, same-user malware, or unsupported environment.

**Correction:** Keep sensitive content out of the view model, accessibility tree, previews, and caches as the primary control. Window capture exclusion is optional best effort and never described as DRM.

### F-19 — Per-process observation ordinal was not a durable cross-restart order

**Severity:** High

**Affected baseline/hardening draft:** event ordering, pagination, import/restore, clock-rollback behavior.

**Problem:** A per-agent observation ordinal can restart at process launch. Combining it with wall-clock time and event ID does not preserve true installation-local capture order when the clock moves backward or equal timestamps/ordinals occur across restarts.

**Correction:** The serialized storage owner assigns a strictly increasing installation-local `capture_order` in the same transaction as each durable clip/ordered audit record. UUID remains the stable public identity, UTC microseconds remain civil time, monotonic clocks remain duration-only, and the per-process ordinal remains origin evidence. Import/merge receives new local order values while retaining original time/provenance; only a verified whole-vault restore preserves the order domain.

**Residual risk:** `capture_order` is local, not globally mergeable or meaningful across independent vaults/devices. Future sync requires a separate ordering/conflict design.

### F-20 — Stable WinUI manager build was coupled to experimental CMake support

**Severity:** High

**Affected baseline:** ADR 0002, root repository structure, toolchain bootstrap, CI/release checklist.

**Problem:** The baseline proposed root CMake authority for the C++/WinRT WinUI 3 manager. Microsoft's stable Windows App SDK path uses the Visual Studio C++ WinUI project/MSBuild/XAML toolchain, while the Windows App SDK release stream describes C++ CMake consumption as Experimental. Making experimental CMake integration a production prerequisite would weaken reproducibility/support and complicate XAML/MSIX tooling.

**Correction:**

- use a stable Visual Studio C++ WinUI `.vcxproj` with MSBuild/NuGet/XAML as the authoritative manager build;
- use a Windows Application Packaging Project (`.wapproj`) for the four-executable MSIX because single-project MSIX supports only one executable;
- orchestrate Cargo + MSBuild through pinned PowerShell/CI scripts;
- keep CMake only for a future isolated native library if that library independently needs it and the choice is justified;
- remove root CMake skeleton from the initial bootstrap acceptance criteria.

Official sources:

- https://learn.microsoft.com/en-us/windows/apps/develop/platform/
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/single-project-msix
- https://github.com/microsoft/WindowsAppSDK/releases

### F-21 — IPC serialization/framing remained unspecified

**Severity:** High

**Affected baseline:** ADR 0008, protocol directory plan, Rust/C++ dependency/toolchain plan, fuzz/compatibility gates.

**Problem:** “Select a schema during bootstrap” left core wire compatibility, framing limits, large-payload behavior, parser/runtime dependencies, presence/default semantics, and schema evolution unresolved. Implementers could choose incompatible Rust/C++ serializers or put clipboard payloads into unbounded control messages.

**Correction:** ADR 0018 proposes a fixed 36-byte bounded Pastral frame with explicit bulk sequence, Protobuf Edition 2024 control schemas, and a separate authorized raw bulk-chunk state machine. Protocol Buffers v35.0 is the current prototype release-train candidate, but the resident Rust runtime is not accepted until official-kernel and credible wire-compatible alternatives are measured for binary/working-set/build/security impact. Large clipboard payloads never live in ordinary control messages. Security-critical enums/presence are validated after parsing, deleted fields are reserved, and DTOs do not become domain/storage models.

Official sources:

- https://protobuf.dev/
- https://protobuf.dev/reference/rust/
- https://protobuf.dev/reference/cpp/cpp-generated/
- https://protobuf.dev/support/cross-version-runtime-guarantee/
- https://github.com/protocolbuffers/protobuf/releases

### F-22 — Forcing every payload into a separate file was not evidence-based

**Severity:** High

**Affected baseline/amendment draft:** ADR 0006, data model, storage/recovery benchmarks, quota/backup behavior.

**Problem:** A file-per-payload design simplifies large-stream staging but can create hundreds of thousands of tiny files, filesystem-block waste, open/close cost, antivirus scans, backup overhead, and expensive cleanup for ordinary text histories. Conversely, forcing large images/streams into SQLite can inflate database/journal/checkpoint costs. SQLite's official guidance shows a workload-dependent crossover and supports incremental BLOB I/O, so neither universal backend is justified without Windows evidence.

**Correction:** Use one content-addressed `BlobStore` contract with internal SQLite BLOB and external-file physical backends. Event/representation rows hold references, not duplicated payload columns. A versioned threshold/policy is selected through Windows 11 x64 benchmarks with realistic payload distributions, 100k–1M records, Defender enabled, warm/cold cache, crash, backup, low disk, deletion, and migration tests. Ordinary digest/protection/deduplication semantics remain independent of backend; Private/sensitive policy remains separately protected.

Official sources:

- https://www.sqlite.org/fasterthanfs.html
- https://www.sqlite.org/intern-v-extern-blob.html
- https://www.sqlite.org/appfileformat.html
- https://www.sqlite.org/c3ref/blob.html

**Residual risk:** A hybrid backend adds migration/recovery complexity and can still produce poor file/database behavior if the threshold is chosen from synthetic benchmarks that do not match user histories. Threshold changes require transactional migration and release evidence.

## Decisions retained after audit

The audit did not reverse:

- Windows 11-only, x64-first scope;
- Rust resident core and C++/WinRT WinUI manager;
- one resident agent and agent-owned database;
- immutable originals and derived provenance;
- SQLite + FTS5 + content-addressed `BlobStore` direction with benchmark-selected internal/external physical backends;
- local-first/network-silent core;
- deterministic rules before ML;
- DirectComposition/Direct2D/DirectWrite overlay prototype;
- signed multi-executable MSIX as the initial release path;
- deferred OCR, semantic search, plugins, sync, ARM64, and portable public distribution.

## Verification evidence

### Phase 0.1 historical evidence

Commit `2aec1c7` was verified with 67 Markdown files, 17 ADRs, and findings F-01 through F-20; local links/fences, ADR structure, contradiction/version scope, secret-signature scan, whitespace, documentation-only scope, ignored launcher, and absent `NUL` artifact passed. That evidence applies to the exact committed tree and is not retroactively expanded to cover later ADR 0018 or F-21/F-22.

### Phase 0.2 evidence

ADR 0018, F-21, F-22, the hybrid `BlobStore`, and the follow-up build/runtime refinements are verified separately in [`phase-0-2-ipc-storage-verification.md`](phase-0-2-ipc-storage-verification.md). That file records fresh commands and results for the exact follow-up commit candidate.

Documentation checks establish consistency and repository scope only. They do not validate runtime behavior, security, performance, accessibility, packaging, or clipboard compatibility.

## Readiness gate

Repository/toolchain bootstrap may begin only after:

1. ADR 0015–0017 are accepted; ADR 0018 framing/schema prototype requirements are recorded with its runtime acceptance gate; ADR 0006 uses a benchmark-gated hybrid blob backend rather than a universal file/SQLite assumption;
2. all affected baseline documents are corrected;
3. the new security limitations are reflected in tests and user-facing policy;
4. the documentation consistency and link checks pass;
5. a separate hardening commit is created with a clean working tree.
