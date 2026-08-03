# ADR 0014: Plugin and scripting systems are deferred

**Status:** Deferred
**Date:** 2026-08-03

## Context

Clipboard tools benefit from custom transformations and integrations, but arbitrary scripts or in-process plugins can read sensitive history, execute commands, access the network, destabilize the agent, bypass policy, and create an unbounded compatibility surface. Core user outcomes can be served first through built-in versioned transformations and deterministic rules.

## Decision

Do not ship a scripting engine, plugin marketplace, arbitrary command execution, or third-party in-process extensions in the first release.

First-release extensibility is limited to:

- built-in transformations with versioned provenance;
- deterministic rule conditions/actions from an allowlisted schema;
- documented import/export formats that never execute code;
- authenticated IPC for first-party manager/CLI;
- separately installed browser/IDE integrations only after dedicated threat models and permissions UX.

Any future extension system must be out-of-process, capability-based, deny network/history access by default, use signed/versioned manifests, enforce resource limits, expose every permission, support revocation, and keep the agent's database/private keys inaccessible.

## Consequences

Positive:

- smaller attack surface and dependency set;
- predictable rules and support behavior;
- no arbitrary code inside the resident process;
- easier privacy explanations and diagnostics.

Costs:

- expert users cannot automate every niche workflow initially;
- Pastral may need more built-in transformations;
- integrations require first-party development or a later secure host.

## Alternatives considered

- **Embedded JavaScript/Lua:** powerful but introduces a resident interpreter, sandbox complexity, and content-access risk.
- **Shell command actions:** rejected because copied content could become command input and policy is difficult to enforce.
- **Native DLL plugins:** rejected due to shared-process trust and ABI/update risk.
- **WebAssembly immediately:** stronger sandbox potential but still requires capability, resource, signing, and UX design not justified before core maturity.

## Review triggers

- core rules and transformations are stable;
- repeated validated workflows cannot be expressed safely;
- a capability model and out-of-process host have separate accepted ADRs;
- security review and fuzzing infrastructure are mature;
- extension signing, revocation, compatibility, and incident response are defined.
