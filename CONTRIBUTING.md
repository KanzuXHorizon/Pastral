# Contributing to Pastral

Pastral handles sensitive, untrusted clipboard data and runs continuously in the Windows user session. Changes are reviewed for correctness, privacy, security, accessibility, performance, compatibility, and recoverability—not only visible behavior.

## Current phase

The repository is in Phase 0. Do not add feature code, dependencies, package identities, public registries, signing configuration, network integrations, or generated brand assets until the relevant design/implementation plan is approved.

## Before changing behavior

1. Read `PRODUCT.md`, `DESIGN.md`, the relevant architecture document, and applicable ADRs.
2. Identify user-visible behavior and exact acceptance criteria.
3. Review privacy/security and data-migration impact.
4. Write or update a focused design/plan for multi-file or high-risk work.
5. Use test-first development for behavior and bug fixes where reproducible.
6. Keep the vertical slice small and independently verifiable.

Architectural changes require an ADR update before implementation when they add or change:

- a resident process;
- storage owner/schema/encryption;
- public or local protocol;
- network behavior;
- supported OS/architecture;
- parser/codec/sandbox boundary;
- scripting/plugins/integrations;
- package/update/signing behavior;
- sensitive-data collection, indexing, export, or retention.

## Test-first workflow

For behavior that can regress:

1. write a focused failing test or fixture reproducing the desired behavior;
2. run it and confirm it fails for the intended reason;
3. make the smallest correct change;
4. run targeted tests;
5. refactor while green;
6. run the full relevant suite;
7. record benchmark impact for capture, storage, search, overlay, paste, startup, or resident dependencies.

Documentation-only and mechanical generated-file changes use an explicit observable check instead of artificial tests.

## Security and privacy

- Treat clipboard formats, HTML, images, paths, imports, IPC, databases, blobs, and worker output as untrusted.
- Never commit or test with real clipboard history or secrets.
- Never add payload fields to normal logs.
- Do not weaken hard-deny, sensitive skip, encryption, authorization, or destination-validation policy for convenience.
- Network access is prohibited in core components unless a separate accepted design exists.
- Use the private reporting process in `SECURITY.md` for vulnerabilities.

## Accessibility and UX

A UI change includes:

- keyboard and focus behavior;
- UI Automation name/role/state/pattern;
- light, dark, and high contrast;
- text scaling and 100–300% DPI;
- RTL and long localization;
- reduced motion and transparency disabled;
- loading, empty, error, denied, locked, overflow, and recovery states;
- sensitive-content behavior in visuals and accessibility tree;
- screenshots using synthetic data when visuals change.

Passive overlay changes additionally prove foreground/focus preservation and no ordinary-key interception.

## Performance

Do not describe a change as fast, lightweight, optimized, lossless, or efficient without evidence from `docs/performance/benchmark-methodology.md`.

Prohibited shortcuts:

- clipboard polling;
- periodic full-history scans;
- busy waits;
- working-set trimming APIs as an optimization claim;
- disabling safety checks;
- hidden lossy transformation of originals;
- unbounded caches, queues, retries, or parser allocations.

## Code expectations after bootstrap

- Rust stable toolchain and lockfile are pinned.
- Rust warnings/Clippy warnings are denied for project code.
- Every `unsafe` block states invariants and ownership/lifetime assumptions.
- Win32/COM/HANDLE/HGLOBAL/HWND/stream/resource ownership uses RAII wrappers.
- C++20 project warnings are errors; raw ownership is avoided.
- Manager does not duplicate domain logic or open SQLite directly.
- IPC/schema/storage migrations are versioned and tested.
- Dependencies are minimal in the agent and receive advisory/license review.

## Commit and pull-request discipline

- Small coherent commits.
- Conventional commit style unless a later repository ADR replaces it.
- No unrelated refactors.
- Do not rewrite history to hide security/design decisions.
- A pull request states behavior, tests and exact commands, risks, screenshots for UI changes, performance impact, migration/rollback, documentation, and remaining limitations.
- Required failing CI blocks merge. Flaky tests require a tracked owner and removal deadline.

Suggested commit types:

- `feat:` behavior;
- `fix:` defect;
- `docs:` documentation/ADR;
- `test:` tests/fixtures;
- `perf:` measured performance work;
- `refactor:` behavior-preserving restructuring;
- `build:` toolchain/dependency/build;
- `ci:` workflows;
- `security:` security hardening/remediation.

## Generated files and local state

Do not commit:

- tunnel/DevSpace state;
- developer logs or diagnostics;
- clipboard databases/blobs/exports;
- credentials, signing keys, certificates, tokens, DPAPI material;
- build outputs, package artifacts, fuzz crashes, benchmarks containing private data;
- user-specific IDE settings unless intentionally shared and reviewed.

## Definition of done

A feature or fix is complete only when:

- behavior and failure states are specified;
- focused and relevant full tests pass;
- security/privacy implications are resolved;
- accessibility is validated;
- performance is measured where relevant;
- migrations and rollback/recovery exist where needed;
- documentation and ADRs match implementation;
- final diff contains no accidental scope, secret, or generated private artifact;
- exact verification results and remaining risk are reported.
