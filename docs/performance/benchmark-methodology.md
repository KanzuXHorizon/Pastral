# Benchmark methodology

## 1. Purpose

Produce repeatable evidence for idle footprint, clipboard capture, overlay, Quick Paste, search, paste/replay, storage, startup, and maintenance. Results must be comparable across commits and must not contain clipboard payloads.

## 2. Reference hardware policy

Every report records:

- manufacturer/model or reproducible virtual-machine definition;
- CPU model, physical/logical cores, firmware/power mode;
- RAM capacity/speed;
- storage model/interface, free space, BitLocker state;
- GPU/driver and display topology;
- Windows edition/build/patch level;
- architecture;
- Windows SDK/App SDK/runtime versions;
- antivirus/indexing state;
- battery/AC and battery saver;
- RDP/console session;
- Pastral build profile, commit, package mode, and database schema.

Initial reference tiers:

1. **Minimum practical x64:** supported Windows 11 laptop, 4 cores, 8 GB RAM, mainstream NVMe.
2. **Primary x64:** 6–8 modern cores, 16 GB RAM, mainstream NVMe, integrated GPU.
3. **Stress/high-end:** more cores/RAM for 1M-record and large-payload scaling; not used to hide minimum-tier failures.

The exact machines are recorded when implementation hardware is selected.

## 3. Environment control

Before a comparative run:

- reboot or use a documented clean-state method for cold measurements;
- wait for login/startup stabilization;
- confirm no Windows Update/install/indexing task dominates the system;
- set and record power mode;
- use the same display/DPI topology;
- clear only benchmark-specific caches when measuring cold state;
- preserve normal OS security features unless the scenario explicitly studies them;
- disable debug logging and tracing for user-facing latency runs;
- collect tracing in separate diagnostic runs.

## 4. Build control

- Release/LTO/codegen configuration is pinned and recorded.
- Symbols are retained separately for analysis.
- Dependency lockfiles and compiler versions are included in report metadata.
- Compare the exact signed/package layout intended for release where packaging affects startup.
- Do not compare debug and release builds.

## 5. Sampling and statistics

Default procedure:

- at least 10 warm-up operations excluded from results;
- at least 100 ordinary capture/search/paste samples when automation is reliable;
- at least 30 cold startup samples or a documented lower sample count when reboot cost is material;
- idle observation for 10 minutes minimum and an 8-hour soak for release candidates;
- report median, p90, p95, p99, maximum, standard deviation or robust spread, and failures/timeouts;
- retain raw metadata-only measurements in machine-readable artifacts;
- compare paired runs on the same machine and state.

Outliers are not silently deleted. State the rule and reason for exclusions.

## 6. Time sources and instrumentation

- Use monotonic high-resolution clocks such as QueryPerformanceCounter through a reviewed wrapper.
- Define named trace points for notification received, sequence read, clipboard acquired, first/last immediate representation captured, foreign object released, storage queued/committed, overlay submitted/visible, Quick Paste invoked/interactive, query submitted/first row, data object published, paste dispatched/result.
- Correlation IDs link stages without payload data.
- Development-only ETW/WPR/WPA traces inspect CPU scheduling, disk I/O, page faults, GPU/compositor, process/thread lifetime, and power wakeups.
- Release logging remains much less verbose.

## 7. Idle benchmark

Procedure:

1. Start agent in steady configured state with no manager/worker.
2. Confirm clipboard and UI are inactive.
3. Observe CPU, private working set, commit size, handles, threads, disk I/O, network, GPU, context switches, and wakeups for 10 minutes.
4. Repeat with database sizes 0, 100k, and 1M metadata records.
5. Run 8-hour soak across lock/unlock and monitor power transitions.
6. Attribute any periodic activity to a named subsystem and timer.

Failure examples:

- clipboard polling;
- periodic full scans;
- cache/handle/thread growth;
- network socket activity;
- compositor/GPU activity with no visible UI.

## 8. Capture benchmark

Use a native fixture producer to publish controlled clipboard data:

- short/1 MB Unicode;
- text + HTML + RTF;
- URL formats;
- encoded PNG and DIB/DIBV5;
- file lists;
- custom opaque registered format;
- delayed rendering;
- slow/contended owner;
- malformed/oversized descriptors;
- rapid sequences.

Measure source `SetClipboard`/`OleSetClipboard` completion separately from Pastral observation. Pastral must not be credited or blamed for source-side publication time. Report capture-critical latency, object hold duration, durable commit, memory peak, and dropped/degraded formats.

## 9. Overlay benchmark

- Timestamp successful capture and compositor commit/first presentation using the most reliable available instrumentation.
- Measure warm/cold device state, 100–300% DPI, multi-monitor transition, transparency disabled, reduced motion, battery saver, RDP, and device loss.
- Record foreground/focus correctness as a binary gate for every sample.
- Inspect CPU-side animation work and post-dismiss resource stability.

## 10. Search benchmark

Generate deterministic synthetic datasets with seeded distributions:

- realistic text lengths and term frequencies;
- exact duplicates with separate occurrences;
- source/profile/type/date/tag/fidelity distributions;
- HTML/RTF/OCR-derived searchable text where enabled;
- sensitive/excluded rows that must never appear;
- 1k, 10k, 100k, 1M scales.

Each query class is stored with expected result IDs and ordering rules. Measure parse, SQL/FTS execution, first page, full bounded page, cancellation, and UI presentation separately.

## 11. Paste/replay benchmark

Fixture destinations advertise/inspect supported formats and can emulate synchronous/asynchronous reads. Measure:

- data-object construction;
- clipboard publication;
- destination consumption timing;
- object lifetime;
- memory peak for large formats;
- foreground validation and input dispatch;
- optional restoration result.

Correct bytes/formats and destination safety are prerequisites; fast incorrect paste is a failure.

## 12. Storage and recovery benchmark

Scenarios:

- rollback journal versus WAL prototype;
- clean and dirty shutdown;
- forced termination at each blob/database commit phase;
- low disk and quota crossing;
- antivirus/file lock interference;
- migration from every supported prior schema;
- integrity check and backup/restore;
- retention cleanup with pinned/shared blobs;
- encrypted write/read/rotation.

Use disposable synthetic data only.

## 13. Reporting

Create reports under `docs/performance/reports/YYYY-MM-DD-<commit>-<machine>.md` containing:

- hypothesis/change;
- exact commands and build artifact hashes;
- environment table;
- raw artifact location/hash;
- summary statistics;
- charts without hidden axes or truncated baselines;
- failures and anomalies;
- comparison to budgets and prior accepted commit;
- decision and residual risk.

No report may claim lightweight, instant, lossless, secure, or optimized solely from architecture expectations.
