# Pastral Read-Only History IPC Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bounded storage History/Search pages and expose them through a separate authenticated feature-gated `serve-read` agent command without changing the current Health-only manager contract.

**Architecture:** A storage-owned read model queries only `clip_events` and `search_documents`, computes `has_more` with `limit + 1`, and truncates previews safely. The existing feature-gated agent IPC server gains a distinct read-authorized mode that maps storage rows to the already-versioned Protobuf DTOs while preserving `serve-health` behavior.

**Tech Stack:** Rust 1.97.1, rusqlite 0.40.1 with bundled SQLite/FTS5, existing Protobuf 4.35.0-release DTOs, authenticated Windows named pipes, PowerShell verification.

## Global Constraints

- Work directly on the explicitly authorized `main` checkout while preserving concurrent changes.
- Do not reset, clean, stash, restore, or stage unrelated files.
- Existing `serve-health` remains Health-only.
- `serve-read` authorizes only Health, HistoryPage, and Search.
- No direct blob payload reads, schema migration, network, async runtime, or manager UI/ABI changes.
- Limits are exact: page `1..=100`, preview at most 4,096 UTF-8 bytes, connections `1..=16`.
- Source label remains `None`; pinned remains `false`; missing projections are unavailable.
- Default `pastral-agent` remains Protobuf/transport-free.

---

### Task 1: Storage read-only History and Search pages

**Files:**
- Create: `crates/storage/src/read_model.rs`
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/search.rs`
- Test: `crates/storage/src/read_model.rs`

**Interfaces:**
- Produces:
  - `ClipListItem`
  - `ClipPage`
  - `Storage::history_page(before_capture_order: Option<CaptureOrder>, limit: usize)`
  - `Storage::search_page(query: &str, limit: usize)`
- Consumes the existing schema, `search::literal_query`, typed ID decoding, and domain value types.

- [x] **Step 1: Add failing read-model tests**

Add tests that commit synthetic clips and assert:

```rust
let page = storage.history_page(None, 2).unwrap();
assert_eq!(page.items().len(), 2);
assert!(page.has_more());
assert!(page.items()[0].capture_order() > page.items()[1].capture_order());
```

Also cover exclusive cursors, unavailable rows without search projections, literal search ordering, zero/101 limit rejection, and a multibyte preview truncated to at most 4,096 bytes on a character boundary.

- [x] **Step 2: Run RED**

```powershell
cargo test --locked -p pastral-storage read_model -- --nocapture
```

Expected: FAIL because the module/types/APIs do not exist.

- [x] **Step 3: Implement bounded value types and UTF-8 truncation**

Create private fields with getters. Validate `1..=100`. Implement:

```rust
fn truncate_preview(value: String) -> String {
    if value.len() <= 4096 { return value; }
    let mut end = 4096;
    while !value.is_char_boundary(end) { end -= 1; }
    value[..end].to_owned()
}
```

- [x] **Step 4: Implement History query**

Use one query over `clip_events LEFT JOIN search_documents`, cursor predicate `(?1 IS NULL OR e.capture_order < ?1)`, deterministic order, and `LIMIT limit + 1`. Remove the extra row and set `has_more`.

- [x] **Step 5: Implement Search query**

Expose `search::literal_query` as `pub(crate)`, join FTS -> search_documents -> clip_events, select the bounded preview body, deterministic order, and `LIMIT limit + 1`.

- [x] **Step 6: Run GREEN and storage regression**

```powershell
cargo test --locked -p pastral-storage --all-targets
cargo clippy --locked -p pastral-storage --all-targets -- -D warnings
```

- [x] **Step 7: Commit**

```bash
git add crates/storage/src/read_model.rs crates/storage/src/lib.rs crates/storage/src/search.rs
git commit -m "feat: add bounded storage history pages"
```

---

### Task 2: Authenticated read-only agent command

**Files:**
- Modify: `crates/ipc-win/src/handshake.rs`
- Modify: `crates/ipc-win/src/lib.rs`
- Test: `crates/ipc-win/tests/handshake.rs`
- Modify: `apps/agent/src/ipc_cli.rs`
- Modify: `apps/agent/src/ipc_health.rs`
- Modify: `apps/agent/src/ipc_main.rs`
- Modify: `apps/agent/src/lib.rs`
- Test: `apps/agent/tests/ipc_cli.rs`
- Create: `apps/agent/tests/ipc_read.rs`
- Preserve: `apps/agent/tests/ipc_health.rs`

**Interfaces:**
- Produces:
  - `AgentIpcCommand::ServeRead { data_root, max_connections }`
  - `serve_read(config: HealthServerConfig, output: &mut impl Write)`
- Consumes `Storage::history_page`, `Storage::search_page`, existing DTO encoding, authentication, and Health snapshot loading.

- [x] **Step 1: Add failing CLI tests**

Accept exactly:

```text
serve-read --data-root C:\temp\root --max-connections 3
```

Reject missing root/value, zero, values above 16, duplicates, unknown flags, and positional arguments. Preserve all `serve-health` tests.

- [x] **Step 2: Add failing authenticated read tests**

Create a disposable root with three synthetic ordinary text clips. Start `serve_read` for three connections and assert:

1. Health response is real and content-free.
2. History returns newest-first preview metadata with `has_more`.
3. Search returns only literal matches and does not echo the query in diagnostics.

Also assert `serve-health` continues to reject History.

- [x] **Step 3: Run RED**

```powershell
cargo test --locked -p pastral-agent --features ipc-health --test ipc_cli
cargo test --locked -p pastral-agent --features ipc-health --test ipc_read
```

- [x] **Step 4: Add `ServeRead` parsing and usage**

Use the same strict `--data-root` and `--max-connections` bounds as `ServeHealth`. Update `ipc_main.rs` dispatch without adding another binary or dependency.

- [x] **Step 5: Generalize one-request serving**

Keep the current secure binding/handshake. Parameterize authorization as `HealthOnly` or `ReadOnly`. `HealthOnly` rejects History/Search. `ReadOnly` maps:

```rust
RequestDto::HistoryPage(request) => ResponseDto::HistoryPage(...)
RequestDto::Search(request) => ResponseDto::Search(...)
```

Use `ProtocolErrorDto` with no developer detail for request/resource/internal failures.

- [x] **Step 6: Map storage rows truthfully**

`Some(preview)` maps to Text. `None` maps to Unavailable with empty preview. Always use `source_label = None` and `pinned = false`.

- [x] **Step 7: Run GREEN and isolation gates**

```powershell
cargo test --locked -p pastral-agent --all-targets
cargo test --locked -p pastral-agent --features ipc-health --all-targets
cargo clippy --locked -p pastral-agent --all-targets --all-features -- -D warnings
cargo tree --locked -p pastral-agent
cargo tree --locked -p pastral-agent --features ipc-health
```

Expected: default graph remains Protobuf/transport-free.

- [x] **Step 8: Commit**

```bash
git add apps/agent/src apps/agent/tests
 git commit -m "feat: serve read-only history over IPC"
```

---

### Task 3: Verification, policy, and evidence

**Files:**
- Modify only if required by failing exact gates: `eng/verify-dependencies.ps1`, `eng/verify-source-policy.ps1`, `.github/workflows/rust-ci.yml`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/operations/developer-setup.md`
- Create: `docs/reviews/phase-3h-read-only-history-ipc-verification.md`

**Interfaces:**
- Produces a dedicated evidence record and keeps current build/policy dispatch truthful.

- [x] **Step 1: Run aggregate Rust gates**

```powershell
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --workspace --no-deps
```

- [x] **Step 2: Run focused project gates**

```powershell
.\eng\verify-dependencies.ps1
.\eng\verify-source-policy.ps1
.\eng\verify-manager-ipc-bridge.ps1 -Mode Static
```

- [x] **Step 3: Update product truth**

State that the authenticated agent backend can serve bounded History/Search previews, while manager ABI/UI, production resident lifecycle, paste, and payload transfer remain unavailable.

- [x] **Step 4: Write verification report**

Record commits, storage query contract, pagination/cursor tests, UTF-8 bound, authenticated negative tests, test counts, dependency isolation, and remaining Phase 3H gates.

- [x] **Step 5: Commit evidence**

```bash
git add README.md CHANGELOG.md docs eng .github
 git commit -m "docs: record read-only history IPC evidence"
```

- [x] **Step 6: Final integrity**

```powershell
git diff --check
git show --check --oneline --stat HEAD
git fsck --no-dangling
git status --short --branch
```

Do not claim a clean tree if concurrent agent-owned changes remain.
