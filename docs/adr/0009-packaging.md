# ADR 0009: Signed multi-executable MSIX as the primary distribution

**Status:** Accepted for initial release planning
**Date:** 2026-08-03

## Context

Pastral contains four executables, a WinUI 3 manager, startup/tray behavior, app identity, icons, protocol declarations, update requirements, and user data that must survive ordinary updates. Single-project MSIX is not suitable for a multi-executable package.

## Decision

Use a supported multi-executable MSIX packaging project as the primary public distribution path.

- Package `pastral-agent.exe`, `pastral-worker.exe`, `pastral-manager.exe`, and `pastral-cli.exe` together.
- Prefer framework-dependent Windows App SDK packaging for Store and managed deployment.
- Sign release packages with protected code-signing material outside the repository and CI logs.
- Define startup behavior, process activation, app execution aliases, uninstall retention, update migration, and repair behavior explicitly in manifests and documentation.
- Produce unpackaged/self-contained artifacts only for controlled development, CI, fixture testing, and diagnostics until portable deployment has a separate accepted design.
- Prepare WinGet manifests only after package identity, signing, update, uninstall, and silent-install behavior are stable.

## Consequences

Positive:

- supported Windows identity, signing, deployment, update, and clean package registration;
- one package for all executables and assets;
- appropriate path for Microsoft Store and managed environments.

Costs:

- multi-project build and packaging complexity;
- startup/background behavior requires careful package declarations;
- framework dependency availability and repair paths must be tested;
- portable users are not served initially.

## Alternatives considered

- **Single-project MSIX:** rejected because it supports a single executable scenario.
- **Traditional MSI/EXE first:** flexible but places more update, repair, identity, and uninstall work on Pastral.
- **Portable ZIP first:** easy distribution but weak update/signing/startup/data-location semantics for a resident system utility.
- **Store-only:** rejected because development, enterprise, and offline test deployment still need signed side-loading paths.

## Review triggers

- Windows App SDK deployment requirements change;
- MSIX cannot support required startup or IPC behavior;
- enterprise/portable demand justifies another packaging ADR;
- code-signing service or Store identity is selected;
- package footprint/startup benchmarks fail acceptance gates.
