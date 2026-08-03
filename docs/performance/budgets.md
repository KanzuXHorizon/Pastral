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
| Notification to sequence/coalescing decision | p95 under 1 ms |
| Clipboard/OLE acquisition excluding external contention | p95 under 3 ms |
| Synchronous capture-critical path | target p95 under 10 ms |
| Clipboard/data-object release | as early as safely possible; measured duration reported |
| Durable lightweight metadata/payload persistence | p95 under 50 ms for ordinary text, off message thread where possible |
| Overlay view-model readiness after successful capture | p95 under 20 ms |

External clipboard contention is reported separately by attempt count and total bounded retry duration.

### Larger/common content

- 1 MB text/HTML: no UI/message-thread stall over the validated capture boundary.
- 10 MB encoded image: stream/copy with bounded memory; no decode on capture path.
- 100 MB supported stream/reference scenario: no duplicate full-size buffers; capture policy may degrade or refuse according to configured limits.
- 100 rapid updates: no unbounded queue, no source-copy interference, deterministic coalescing, and no lost final event under fixture semantics.

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
| Warm invocation to interactive first frame | p95 under 80 ms |
| Cold invocation | Measured and budgeted after manager/surface architecture prototype; no unsupported claim |
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

## 8. Storage and maintenance

- Startup recovery scans only staging/reference state needed for safety; no full payload read.
- Retention/quota cleanup is incremental, cancellable, and yields to capture/search.
- Delete shared blob only after reference check.
- Low-disk detection avoids repeated failing writes.
- Migration benchmarks include largest supported test database and interruption at persisted phases.
- Integrity check may be long-running but must expose progress/cancellation and never freeze the UI.

## 9. Startup and lifecycle

Measure:

- agent cold startup to clipboard listener ready;
- warm startup;
- database recovery/migration variants;
- manager/Quick Paste cold and warm activation;
- profile switch;
- session unlock/resume;
- shutdown with/without active paste ownership;
- update first run.

No startup target is claimed until a representative executable exists. Initial goal: agent listener readiness fast enough not to miss normal post-login copies, with exact budget derived from prototype traces.

## 10. Regression gates

A performance-sensitive change fails review when, on the same controlled benchmark:

- p95 capture/overlay/Quick Paste/search regresses more than 10% and exceeds noise confidence without accepted rationale;
- idle introduces periodic CPU/disk/network activity;
- agent steady working set grows more than 2 MB or 10%, whichever is larger, without dependency attribution and approval;
- large-payload peak memory introduces an additional full-size copy;
- correctness/privacy/security behavior is weakened to gain speed.

CI smoke tests use wider thresholds for noisy virtual hardware. Manual release benchmarks are authoritative for user-facing claims.
