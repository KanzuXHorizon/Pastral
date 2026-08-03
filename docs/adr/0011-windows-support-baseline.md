# ADR 0011: Windows 11-only support baseline

**Status:** Accepted
**Date:** 2026-08-03

## Context

Supporting Windows 10 would add API fallback, packaging, test-matrix, DPI, composition, accessibility, and lifecycle cost while Pastral is still proving clipboard fidelity and resident performance. Windows 11 releases have time-limited servicing, so a fixed marketing minimum can become unsupported before release.

## Decision

- Support Windows 11 only.
- Use Windows 11 24H2 build 26100 as the MVP engineering and compatibility floor.
- At release-candidate time, require the oldest Windows 11 Home/Pro release still serviced by Microsoft.
- If release occurs after Windows 11 24H2 Home/Pro end of servicing on 2026-10-13, the expected public floor is Windows 11 25H2 build 26200.
- Do not use Windows 11 26H1 as a general baseline because it is a new-device release rather than the universal update path from 24H2/25H2.
- Build against a pinned stable Windows SDK supported by the selected Visual Studio toolchain; guard APIs newer than the runtime floor through capability detection.
- Release x64 first. Keep protocol, storage, serialization, and ABI assumptions architecture-neutral so ARM64 can follow after x64 gates pass.

## Consequences

Positive:

- smaller compatibility matrix and clearer native-API assumptions;
- effort focuses on reliability, security, accessibility, and performance;
- avoids shipping on an unsupported Home/Pro baseline.

Costs:

- Windows 10 users are excluded;
- release minimum may rise during development;
- ARM64 users wait for a later validated build.

## Alternatives considered

- **Windows 10 22H2 and Windows 11 equally:** rejected due to high test/fallback cost and expired Windows 10 Home/Pro lifecycle.
- **Windows 11 25H2 immediately:** safer release horizon but prevents testing useful 24H2 compatibility during development.
- **Latest Windows 11 only:** unnecessarily excludes supported systems and weakens compatibility discipline.

## Review triggers

- release-candidate date and Microsoft servicing table;
- a required API cannot be supported on build 26100;
- Windows App SDK or packaging changes its OS minimum;
- ARM64 readiness review;
- enterprise LTSC support is proposed.
