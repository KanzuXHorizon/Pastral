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
- Define named trace points for notification received, sequence read, control-thread observation queued/returned, capture-queue dequeue, clipboard acquired, foreign call begin/end/cancel attempt, first/last immediate representation captured, foreign object/medium released, storage queued/committed, overlay submitted/visible, Quick Paste cold process start or warm activation/interactive, query submitted/first row, data object published, focus revalidation, input dispatched/manual fallback, and consumption result.
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
- registered format with stable name and changed runtime numeric ID;
- unknown custom format that must remain metadata-only/unsupported;
- delayed rendering, re-entrancy, blocked/non-cooperative owner, and cancellation refusal;
- slow/contended owner;
- malformed/oversized descriptors;
- sequence unavailable/zero, unsigned-wrap abstraction, duplicate queued notifications, rapid replacements, and forged/stale self-origin markers.

Measure source `SetClipboard`/`OleSetClipboard` completion separately from Pastral observation. Pastral must not be credited or blamed for source-side publication time. Report control-thread handling, capture-queue wait, Win32 clipboard hold, foreign OLE/stream time, cancellation/degraded transition, durable commit, memory peak, final-current-state success, possible unobservable intermediate-state pressure, and dropped/degraded formats. A blocked capture fixture must prove tray/hotkey/session/overlay control responsiveness and bounded queue behavior.

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
- foreground/process/integrity validation, standard destination dispatch, elevated/UIPI blocked path, clipboard-only/manual fallback, and no false consumption claim;
- optional restoration result.

Correct bytes/formats and destination safety are prerequisites; fast incorrect paste is a failure.

## 12. IPC schema and transfer benchmark

Measure with the exact selected Protobuf release-train generator/generated-code/runtime candidates:

- Rust agent binary size and steady-state private working-set delta for the official Rust-kernel path and at least one credible wire-compatible alternative, each linked but idle;
- C++ manager binary/package size using lite control runtime where supported;
- fixed 36-byte frame parse latency and allocation count for minimum, typical, and maximum control bodies;
- protobuf parse, post-parse validation, and DTO-domain conversion latency/allocations;
- Rust-to-C++ and C++-to-Rust serialize/parse golden compatibility;
- malformed, deeply nested, unknown action/enum, missing presence, duplicate-key, and limit rejection cost;
- 256 KiB control and 1 MiB bulk-chunk boundaries;
- bulk streaming throughput, staging disk I/O, cancellation/disconnect cleanup, low-disk response, and peak memory;
- 16 in-flight requests per connection, 64 globally, one active bulk stream per connection, backpressure, and slow-client isolation behavior;
- proof that linking IPC does not add periodic wakeups, gRPC, Tokio, reflection, JSON/TextFormat, or an unbounded allocation pattern.

Compare against a minimal framed protocol stub. Missing the resident/build/security gates keeps ADR 0018 Proposed and triggers a measured runtime/schema alternative decision rather than hiding dependency cost.

## 13. Storage and recovery benchmark

Scenarios:

- internal SQLite BLOB versus external-file backend across realistic payload sizes/distributions (for example 64 B–10 MiB), duplicate ratios, 100k and 1M metadata histories, warm/cold cache, Defender/antivirus enabled, random/sequential reads, durable grouped writes, preview/search access, backup/export, cleanup, and file/database space;
- threshold sweeps and transactional migration between backends, preserving `sha256-raw-v1`, protection domains, reference counts, crash recovery, and no duplicate event semantics;
- incremental SQLite BLOB I/O where applicable versus ordinary SQL retrieval and external streamed I/O;
- rollback journal versus WAL prototype, including FTS/freelist/journal/WAL deletion remnants;
- clean and dirty shutdown;
- forced termination at each internal-BLOB, external staging/rename, reference/database commit, and backend-migration phase;
- low disk and quota crossing;
- antivirus/file lock interference;
- migration from every supported prior schema;
- integrity check and backup/restore;
- retention cleanup with pinned/shared blobs and usage above the 5 GB automatic-cleanup target;
- encrypted write/read/rotation, whole-message authentication-before-release, independently authenticated chunk behavior, and equal Private plaintext producing non-equal storage identity.

Use disposable synthetic data only. Record the selected storage-policy version and threshold with confidence intervals and retain the runner-up; do not promote one backend as universally faster.

## 14. Reporting

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
