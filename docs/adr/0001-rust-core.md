# ADR 0001: Rust core for resident and data-processing components

**Status:** Accepted
**Date:** 2026-08-03

## Context

Pastral requires a small always-running Windows process that crosses Win32, COM/OLE, clipboard memory, SQLite, cryptography, IPC, and hostile parser boundaries. Resource use, ownership, and crash safety matter more than rapid cross-platform UI delivery.

## Decision

Implement `pastral-agent.exe`, `pastral-worker.exe`, `pastral-cli.exe`, and shared domain/storage/search/security modules in Rust.

Initial toolchain:

- Rust 1.97.1 stable;
- Edition 2024;
- Cargo workspace;
- `windows` for typed Win32/COM wrappers where practical and `windows-sys` for narrow low-level surfaces when measurements justify it;
- isolated `unsafe` modules with safety comments and RAII handle wrappers;
- no continuously running general async runtime unless benchmarked against native/overlapped alternatives.

## Consequences

Positive:

- memory-safe domain and parser orchestration;
- strong ownership model for handles, HGLOBAL, COM references, streams, and buffers;
- efficient static binaries without a mandatory managed runtime;
- shared testable logic for agent, worker, and CLI.

Costs:

- Windows COM/OLE integration requires carefully reviewed unsafe boundaries;
- two-language repository because manager uses C++/WinRT;
- dependency selection must avoid inflating resident working set.

## Alternatives considered

- **All C++/WinRT:** simplest Windows interop but larger memory-safety blast radius for hostile data and storage code.
- **C#/.NET agent:** productive but introduces a managed runtime and less deterministic resident footprint.
- **Tauri/Electron:** rejected by product requirements and resident/native goals.

## Review triggers

- Rust Windows bindings cannot support required OLE behavior reliably;
- measured Rust agent footprint materially exceeds a comparable C++ prototype;
- a dependency introduces a mandatory runtime or unacceptable security/maintenance risk;
- supported Rust stable toolchain changes before bootstrap or release.
