# Phase 3H IPC Schema Mutation Corpus Verification

## Result

**GREEN for the deterministic parser-hardening corpus introduced by `088ef34`.**

The IPC schema package now exercises every current control decoder against reproducible malformed and forward-compatible wire mutations. The corpus is test-only: it changes no production decoder, runtime dependency, wire schema, frame limit, agent footprint, or authorization behavior.

This evidence does **not** replace coverage-guided fuzzing, allocation/recursion instrumentation, generated C++ parity, or adjacent-version compatibility fixtures. Those remain open ADR 0018 acceptance gates.

## Commit

```text
088ef34 test: harden IPC schema decoders with mutation corpus
```

Added file:

```text
crates/ipc-schema/tests/mutation_corpus.rs
```

## Decoder families covered

The corpus starts from valid encoded DTOs and invokes the same public decode functions used by the authenticated transport and manager bridge:

- `ServerHello`;
- `ClientHello`;
- `ServerAccepted`;
- every request variant: Health, HistoryPage with and without cursor, and Search;
- every response variant: Health, HistoryPage, Search, and ProtocolError;
- standalone ProtocolError;
- BulkEnd.

Successful decodes continue through the existing DTO constructors, so page counts, preview bytes, query bytes/terms, IDs, timestamps, capabilities, required-field presence, and other domain bounds remain enforced after parsing.

## Deterministic mutation set

For each valid encoded seed, the test performs:

1. every prefix truncation from zero bytes to one byte before the complete message;
2. every single-bit flip at every encoded byte position;
3. an appended unknown varint field;
4. an appended unknown length-delimited field;
5. a malformed truncated unknown length-delimited field;
6. an input one byte above the default 256 KiB control-body ceiling.

Every invocation is wrapped with `catch_unwind`. A parser panic fails the test with only mutation length metadata; no payload content is printed.

Expected compatibility behavior is explicit:

- complete valid seeds round-trip exactly;
- well-formed unknown fields are ignored and the decoded DTO remains equal to the original;
- malformed/truncated unknown-field data is rejected;
- oversized bodies are rejected before schema conversion;
- arbitrary truncations and bit flips may reject or decode to another valid bounded DTO, but must never panic.

The test uses a fixed mutation algorithm and no random dependency, making failures reproducible locally and in CI.

## Focused verification

Fresh Windows x64 results:

```text
cargo test --locked -p pastral-ipc-schema --all-targets
12 passed, 0 failed

cargo clippy --locked -p pastral-ipc-schema --all-targets -- -D warnings
PASS
```

The mutation corpus itself completed in approximately 0.10 seconds in the clean verification worktree.

## Clean aggregate verification

A clean managed worktree based exactly on `088ef34` passed:

```text
.\eng\build.ps1 -Task All
```

Verified gates:

- Rust 1.97.1 MSVC toolchain pin;
- `cargo fmt --all -- --check`;
- `cargo check --locked --workspace --all-targets`;
- `cargo test --locked --workspace --all-targets` — **239 passed, 0 failed**;
- Clippy across all targets/features with `-D warnings`;
- workspace documentation build;
- dependency policy;
- source policy.

The aggregate increased from 238 to 239 solely through the new mutation-corpus test.

## Security and scope notes

- No new crate or build dependency was added.
- The default resident agent remains Protobuf- and transport-free.
- No process, filesystem, network, clipboard, storage, or user-data access occurs in the corpus.
- No production DTO gained `Debug`; content-bearing DTO privacy surfaces remain unchanged.
- Unknown-field compatibility is verified only for the current Rust generated runtime and current schema.
- Unknown enum values remain fail-closed through existing DTO conversion tests.

## Remaining ADR 0018 parser gates

Still required before ADR 0018 can be accepted:

- coverage-guided fuzzing with saved regression corpus;
- parser peak-allocation and recursion-limit evidence at boundaries;
- exact generated C++ malformed-input and unknown-field parity;
- adjacent supported minor-version fixtures and capability compatibility cases;
- bulk-transfer mutation/fuzz coverage for gap, duplicate, reorder, overflow, cancel, disconnect, and staging cleanup.

## Conclusion

The deterministic Rust control-schema mutation corpus is admitted as fast CI regression protection. It materially strengthens panic-safety and unknown-field evidence while keeping the larger fuzzing and cross-language compatibility gates explicit and unclaimed.
