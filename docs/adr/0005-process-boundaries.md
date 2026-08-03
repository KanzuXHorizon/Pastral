# ADR 0005: One resident agent with on-demand worker, manager, and CLI

**Status:** Accepted
**Date:** 2026-08-03

## Context

Pastral needs continuous clipboard event handling but expensive parsing, full UI, OCR, and diagnostics should not remain resident. Complex formats must not share the core availability boundary.

## Decision

Use four executables:

- `pastral-agent.exe`: only resident process and sole database owner;
- `pastral-worker.exe`: launched only for bounded expensive/hostile jobs;
- `pastral-manager.exe`: on-demand UI through IPC;
- `pastral-cli.exe`: on-demand administration through IPC.

The MVP does not install a Windows service. Overlay remains inside the agent unless benchmark/security evidence requires separation.

## Consequences

Positive:

- small idle process set;
- worker/parser crashes isolated;
- manager startup and faults do not affect capture;
- one storage/migration authority.

Costs:

- versioned IPC and process lifecycle complexity;
- worker sandbox/staging design;
- multi-executable packaging.

## Alternatives considered

- **Single monolithic process:** simple communication but full UI/parser footprint and shared crash boundary.
- **Separate resident overlay process:** stronger isolation but another always-running process and IPC latency.
- **Windows service plus user UI:** unnecessary privilege/session complexity for clipboard behavior.

## Review triggers

- overlay faults threaten agent stability despite tests;
- worker launch latency makes required enrichment unusable;
- a service is required by a separately approved multi-session capability;
- database ownership blocks a measured use case that cannot be solved through IPC.
