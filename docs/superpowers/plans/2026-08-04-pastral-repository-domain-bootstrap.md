# Pastral Repository Domain Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify a reproducible Windows MSVC Rust workspace containing one pure, immutable Pastral domain crate.

**Architecture:** The root workspace pins Rust and dependency versions, PowerShell scripts own Windows verification/build orchestration, and `pastral-domain` is decomposed into focused modules with constructor-enforced invariants. Tests exercise public contracts before implementation, while CI mirrors the fresh local Windows commands.

**Tech Stack:** Rust 1.97.1, Edition 2024, `x86_64-pc-windows-msvc`, `uuid`, `sha2`, Windows PowerShell 5.1, GitHub Actions Windows runner.

## Global Constraints

- Work on `main` from baseline `d4bbb90` with a clean tree.
- Use Rust `1.97.1`, Edition 2024, target `x86_64-pc-windows-msvc`.
- Product code has no Windows, database, filesystem, network, async runtime, UI, logging backend, or serialization dependency.
- Do not add CMake, vcpkg, Protobuf, Windows App SDK, native project, executable, installer, or empty future directory tree.
- Use `#![forbid(unsafe_code)]` and constructor-enforced immutable domain values.
- Do not commit `Start-DevSpace-MCP-Cloudflared.ps1`, add a license, push, open a PR, rewrite, or squash history.

---

### Task 1: Commit the approved design

**Files:**
- Create: `docs/superpowers/specs/2026-08-04-pastral-repository-domain-bootstrap-design.md`

**Interfaces:**
- Consumes: Phase 0 authority documents and approved prompt decisions.
- Produces: Exact module boundaries, public constructors, invariants, errors, dependency rationale, tests, and acceptance criteria used by all later tasks.

- [ ] **Step 1: Write the complete design spec**
- [ ] **Step 2: Scan for `TBD`, `TODO`, placeholders, contradictions, and out-of-scope implementation**
- [ ] **Step 3: Run `git diff --check`**
- [ ] **Step 4: Commit with `docs: specify repository domain bootstrap`**

### Task 2: Pin and verify the Windows Rust workspace

**Files:**
- Create: `Cargo.toml`
- Create: `Cargo.lock`
- Create: `rust-toolchain.toml`
- Create: `eng/verify-toolchain.ps1`
- Create: `eng/build.ps1`

**Interfaces:**
- Produces: Workspace package metadata (`edition = "2024"`, `rust-version = "1.97.1"`), centralized `uuid`/`sha2` dependencies, exact toolchain verification, and task orchestration.

- [ ] **Step 1: Add the pinned toolchain and workspace manifest**
- [ ] **Step 2: Add a PowerShell 5.1 verifier that reports Rust tools, active target, VS/MSVC, installed SDKs, and classifies future SDK requirements**
- [ ] **Step 3: Add a fail-fast build dispatcher for `Verify`, `Format`, `Check`, `Test`, `Clippy`, `Doc`, and `All`**
- [ ] **Step 4: Run `eng/verify-toolchain.ps1` and correct required mismatches**
- [ ] **Step 5: Generate and inspect `Cargo.lock` through the pinned Windows Cargo toolchain**
- [ ] **Step 6: Commit with `build: pin Rust workspace toolchain`**

### Task 3: Add identity, time, order, and format values test-first

**Files:**
- Create: `crates/domain/Cargo.toml`
- Create: `crates/domain/src/lib.rs`
- Create: `crates/domain/src/error.rs`
- Create: `crates/domain/src/id.rs`
- Create: `crates/domain/src/time.rs`
- Create: `crates/domain/src/capture_order.rs`
- Create: `crates/domain/src/format_identity.rs`

**Interfaces:**
- Produces: Typed UUID IDs; `UtcUnixMicros::new(i64)`; `CaptureOrder::new(u64)`; `StandardFormatId`; `RegisteredFormatName::new`; `ClipboardFormatIdentity`.

- [ ] **Step 1: Write focused tests for UUID version/variant/canonical text, non-v4 rejection, distinct typed API use, UTC boundaries, zero order, ordering, and registered-name validation**
- [ ] **Step 2: Run focused tests and record expected compile/test failures as red evidence**
- [ ] **Step 3: Implement the smallest value types and manual `DomainError` variants**
- [ ] **Step 4: Run focused tests until green, then run format/check/Clippy**

### Task 4: Add protection and exact-byte digest policy test-first

**Files:**
- Create: `crates/domain/src/protection.rs`
- Create: `crates/domain/src/digest.rs`

**Interfaces:**
- Produces: `ProtectionDomain`; `DigestSuite::Sha256RawV1`; `RawDigest::sha256_raw_v1`; same-domain dedup eligibility.

- [ ] **Step 1: Write tests for stable exact-byte SHA-256, one-byte differences, no Unicode normalization, Sensitive/Private rejection, and cross-domain dedup denial**
- [ ] **Step 2: Run tests to capture red evidence**
- [ ] **Step 3: Implement exact logical-byte hashing and domain validation**
- [ ] **Step 4: Run focused and workspace tests green**

### Task 5: Add immutable representations and events test-first

**Files:**
- Create: `crates/domain/src/fidelity.rs`
- Create: `crates/domain/src/representation.rs`
- Create: `crates/domain/src/clip_event.rs`

**Interfaces:**
- Produces: Versioned `aggregate_fidelity_v1`; immutable `ClipRepresentation::new`; immutable `ClipEvent::new` requiring usable representations.

- [ ] **Step 1: Write tests for fidelity combinations, empty/unusable aggregation, representation digest-domain validation, zero-event rejection, one/many acceptance, unavailable-cardinality rejection, immutable getters, and separate IDs for identical payloads**
- [ ] **Step 2: Run tests to capture red evidence**
- [ ] **Step 3: Implement private-field immutable records and validation**
- [ ] **Step 4: Run focused and workspace tests green**

### Task 6: Add content-free audit and derived provenance test-first

**Files:**
- Create: `crates/domain/src/audit_event.rs`
- Create: `crates/domain/src/derived.rs`

**Interfaces:**
- Produces: `CaptureAuditEvent`, `CaptureOutcome::HardDenied`, `DerivedRepresentationProvenance::new`.

- [ ] **Step 1: Write tests for allowed content-free metadata, hard-deny without durable event, parent preservation, transformation-version validation, digest policy, and unchanged original representation**
- [ ] **Step 2: Run tests to capture red evidence**
- [ ] **Step 3: Implement the minimal immutable types without payload/digest/blob fields on audits**
- [ ] **Step 4: Run all quality gates green**
- [ ] **Step 5: Commit Tasks 3–6 together with `feat: add immutable domain foundation` so no intentionally failing commit is created**

### Task 7: Add Windows CI and developer documentation

**Files:**
- Create: `.github/workflows/rust-ci.yml`
- Create: `docs/operations/developer-setup.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: CI commands matching local verification and exact Windows setup instructions for the Phase 1 slice.

- [ ] **Step 1: Add Windows CI for verifier, format, check, test, Clippy warnings-as-errors, and docs**
- [ ] **Step 2: Document exact install/verification/build commands and deferred native SDK status**
- [ ] **Step 3: Update README and changelog without claiming unrun checks**
- [ ] **Step 4: Validate YAML and run local CI-equivalent commands**
- [ ] **Step 5: Commit with `ci: verify Rust domain bootstrap`**

### Task 8: Record and commit fresh milestone evidence

**Files:**
- Create: `docs/reviews/phase-1-domain-bootstrap-verification.md`

**Interfaces:**
- Produces: Auditable baseline, environment, dependency tree, red/green evidence, commands/results/counts, changed files, exclusions, skips, risks, and unproven claims.

- [ ] **Step 1: Run fresh PowerShell verification and every requested Cargo quality gate**
- [ ] **Step 2: Run `cargo tree`, forbidden-dependency review, diff check, secret-signature scan, scope review, `git fsck`, and `git show --check HEAD`**
- [ ] **Step 3: Write only observed results and explicit residual risks**
- [ ] **Step 4: Run doc consistency and final diff checks**
- [ ] **Step 5: Commit with `docs: record domain bootstrap evidence`**
- [ ] **Step 6: Re-run final status and verify the working tree is clean**
