# Performance budgets

**Status:** Initial engineering budgets; revise only with measured evidence and an explicit rationale.
**Reference scope:** x64, supported Windows 11 baseline, release configuration, content logging disabled.

## 1. Measurement principles

- Budgets are gates, not marketing claims.
- Report median, p95, p99, maximum, sample count, warm/cold state, hardware, Windows build, power mode, storage state, and dataset.
- Separate synchronous capture-critical time from deferred persistence/enrichment.
- Compare against a no-Pastral baseline and previous accepted build.
- A single fast run is not evidence.
- Do not optimize through working-set trimming APIs, busy waits, hidden polling, disabled safety checks, or lossy changes to originals.

## 2. Idle agent

| Metric | Initial budget |
|---|---|
| CPU, no events for 10 minutes | Effectively zero; target average under 0.1% of one logical processor, no periodic spikes attributable to polling |
| Clipboard polling | Zero |
| Periodic wakeups | None except explicit scheduled maintenance with documented cadence |
| Private working set after steady initialization | Target under 25 MB where technically achievable |
| Handle/thread growth during 8-hour idle | No unbounded growth; stable after initialization |
| Disk reads/writes during idle | Zero except structured log rotation/config/update explicitly triggered |
| Network | Zero for core processes |
| GPU activity | Zero without visible overlay/UI |

A miss on the 25 MB target is not hidden. Record component/dependency contribution and decide whether benefit justifies cost.

## 3. Clipboard capture

### Ordinary Unicode text, warm state

| Stage | Budget |
|---|---|
| Control-thread notification handling and observation handoff | p95 under 1 ms; no foreign clipboard/OLE call or wait |
| Capture-queue wait | reported separately; target p95 under 2 ms without pressure |
| Win32 clipboard acquisition excluding external contention | p95 under 3 ms |
| Synchronous capture-critical path on clipboard-platform STA | target p95 under 10 ms for ordinary text |
| Clipboard-open/foreign-object hold duration | measured explicitly and minimized; owner/call delay reported separately |
| Pastral-owned capture result queued to storage | p95 under 20 ms for ordinary text |
| Durable lightweight metadata/payload persistence | p95 under 50 ms for ordinary text, off control and clipboard-platform apartments after ownership transfer |
| Overlay view-model readiness after successful durable capture | p95 under 20 ms from commit acknowledgement |

External clipboard contention, delayed rendering, foreign COM/stream time, cancellation attempts, and capture-queue pressure are reported separately. They are not hidden inside the normal-path budget.

### Larger/common content

- 1 MB text/HTML: no UI/message-thread stall over the validated capture boundary.
- 10 MB encoded image: stream/copy with bounded memory; no decode on capture path.
- 100 MB supported stream/reference scenario: no duplicate full-size buffers; capture policy may degrade or refuse according to configured limits.
- 100 rapid replacements: no unbounded queue or source-copy interference; capture the final current state under the defined fixture; report possible unobservable intermediate-state loss without inventing an exact count.

### Capture-health and blocked-owner budget

- While a fixture blocks the clipboard-platform STA, control-thread tray/hotkey/session handling remains responsive and passive overlay never reports a false successful capture or paste availability.
- Observation queue remains bounded; latest-state pressure and dropped/coalesced observations are counted without payload.
- Soft deadline, cancellation attempt, degraded-state transition, and explicit restart recovery are timed separately.
- No unsafe thread termination and no unbounded capture-thread creation.
- A reproducible non-recoverable stuck owner triggers capture-broker architecture review rather than a relaxed correctness claim.

## 4. Overlay

| Metric | Initial budget |
|---|---|
| Successful capture to first visible warm frame | p95 under 50 ms |
| Entry/exit duration | 120–180 ms unless reduced motion/battery policy disables it |
| CPU during compositor animation | No continuous application-side frame loop; attributable CPU measured |
| Post-dismiss resource state | GPU surfaces/caches bounded and reusable; no growth per notification |
| Focus/foreground changes | Zero; correctness gate overrides latency |

Under battery saver, reduced motion, transparency disabled, RDP, or device loss, a simpler/instant solid presentation is acceptable.

## 5. Quick Paste

| Metric | Initial budget |
|---|---|
| Warm invocation to interactive first frame | p95 under 80 ms when the single-instance manager UI process is already alive |
| Cold invocation | Measured separately from process activation through interactive first frame; no unsupported target until the WinUI prototype exists |
| First recent results | p95 under 30 ms at 100,000 ordinary text records |
| Keystroke to updated first results | p95 under 30 ms at 100,000 records, obsolete queries cancelled |
| Selection stability | No unexpected selection jump during incremental update |
| Memory | Does not load full history or image payloads; bounded result/page/preview cache |

## 6. Search

Datasets: 1k, 10k, 100k, and 1M metadata records with realistic text lengths, sources, tags, profiles, dates, duplicate occurrences, and FTS terms.

Required measurements:

- recent/no-query page;
- exact phrase;
- prefix;
- structured-only filter;
- FTS plus source/profile/type/date filters;
- no-result query;
- common term with high result count;
- typo fallback;
- cancellation under rapid typing;
- sensitive/excluded rows absent.

Initial target: first result page under 30 ms at 100k on reference hardware. A separate 1M target is established only after schema/index prototype evidence.

## 7. Paste/replay

- Build ordinary text data object: target p95 under 10 ms warm.
- Publish clipboard: report external contention separately.
- Synthetic paste dispatch: correctness and destination safety before speed.
- Large replay: no unnecessary duplicate full buffers; memory peak reported.
- Delayed-render object lifetime: bounded without breaking tested destinations.
- Copy-only mode: no focus change.
- Elevated/UIPI or uncertain destination fallback adds no unbounded retry; data publication completes and manual-paste guidance appears without false success.

## 8. IPC protocol

| Metric | Initial budget/gate |
|---|---|
| Agent idle impact after linking a candidate Protobuf runtime/schema | Included in the under-25 MB target; official Rust-kernel and credible wire-compatible alternative deltas are reported separately before selection |
| Control frame body | Hard maximum 256 KiB; typical requests remain far smaller |
| Bulk chunk body | Hard maximum 1 MiB per frame; one active bulk transfer per connection initially; logical total is operation/policy-specific |
| In-flight control requests | Maximum 16 per connection and 64 globally with backpressure |
| Fixed-header parse | No body-buffer allocation before validated header length/kind/sequence/state |
| Selected Protobuf parser allocation | Peak/aggregate allocation at 256 KiB malicious and valid limits is measured and bounded; runtime recursion/total-byte controls enabled where available |
| Typical control request parse + post-parse validation + domain conversion | target p95 under 1 ms warm on reference hardware after prototype |
| Idle wakeups/network | No additional periodic wakeup; no gRPC/HTTP/Tokio runtime solely for IPC |
| Bulk transfer memory | Bounded by chunk/window/staging policy; never whole-payload buffering by default |

Generator, generated code, and runtime artifacts are exact-matched according to the selected language/toolchain support policy. ADR 0018 remains Proposed until the official Rust-kernel path and at least one credible wire-compatible Rust alternative are measured. If no candidate meets the resident-agent/build/security gates, the framing/schema runtime choice is revised rather than hiding the cost inside generic IPC memory.

## 9. Storage and maintenance

- Startup recovery scans only staging/reference/backend state needed for safety; no full payload read.
- Internal SQLite BLOB versus external-file placement uses a versioned, benchmark-selected threshold/policy; neither backend is declared universally faster.
- Threshold evidence includes Windows/Defender warm/cold cache, realistic payload distributions, file/database size/count, 100k–1M histories, durable writes, random reads, preview access, backup/export, cleanup, and migration.
- Retention/quota cleanup is incremental, cancellable, and yields to capture/search.
- Delete shared blob only after reference check; backend migration preserves `sha256-raw-v1`, protection domain, reference count, and crash recovery.
- The 5 GB value is an automatic-cleanup target for ordinary unpinned history, not a hard cap when pinned/protected data exceeds it.
- Low-disk detection uses a separately benchmarked reserve/hysteresis policy, avoids repeated failing writes, pauses new payload capture before filesystem exhaustion, and never silently deletes pinned data.
- Migration benchmarks include largest supported test database and interruption at persisted phases.
- Integrity check may be long-running but must expose progress/cancellation and never freeze the UI.

## 10. Startup and lifecycle

Measure:

- agent cold startup to control listener ready and clipboard-platform STA ready as separate points;
- warm startup;
- database recovery/migration variants;
- manager/Quick Paste cold process activation, warm window activation, bounded warm-lifetime cost, and idle teardown;
- profile switch;
- session unlock/resume;
- shutdown with/without active paste ownership;
- update first run.

No startup target is claimed until a representative executable exists. Initial goal: agent listener readiness fast enough not to miss normal post-login copies, with exact budget derived from prototype traces.

## 11. Regression gates

A performance-sensitive change fails review when, on the same controlled benchmark:

- p95 capture/overlay/Quick Paste/search regresses more than 10% and exceeds noise confidence without accepted rationale;
- idle introduces periodic CPU/disk/network activity;
- agent steady working set grows more than 2 MB or 10%, whichever is larger, without dependency attribution and approval;
- large-payload peak memory introduces an additional full-size copy;
- frame/Protobuf/domain validation or bulk transfer exceeds accepted limits, introduces unbounded allocation/backpressure, or adds periodic wakeups;
- a storage threshold/backend change regresses capture/search/preview/backup/recovery or creates unacceptable tiny-file/database growth without accepted evidence;
- correctness/privacy/security behavior is weakened to gain speed.

CI smoke tests use wider thresholds for noisy virtual hardware. Manual release benchmarks are authoritative for user-facing claims.
