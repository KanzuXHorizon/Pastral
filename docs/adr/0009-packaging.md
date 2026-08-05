# ADR 0009: Signed multi-executable MSIX as the primary distribution

**Status:** Accepted — deterministic x64 development MSIX, package activation, startup declaration, signing, extraction parity, and live registration smoke pass; public trusted signing and update/rollback qualification remain release gates
**Date:** 2026-08-05

## Context

Pastral has one resident Rust agent and one on-demand C++/WinRT manager. The manager requires package identity, localized resources, branded assets, and a Windows App SDK framework dependency. The agent must start at sign-in without becoming a service or introducing a second resident storage owner. Package signing, repair, update, and uninstall boundaries must remain explicit.

The earlier planning model listed four prospective executables. The implemented product boundary is intentionally smaller:

- `pastral-agent.exe` is the only resident process and the sole capture/storage/IPC owner;
- `pastral-manager.exe` is the visible on-demand WinUI 3 application;
- `pastral-manager-ipc-bridge.dll` is deployed beside the manager;
- diagnostic probes and `pastral-agent-ipc.exe` are never packaged.

## Decision

Use a signed x64 MSIX as the primary Windows distribution path.

- Package the resident agent, native manager, Rust bridge, XAML/PRI/WinMD resources, and reproducibly generated brand assets together.
- Build the manager specifically as a packaged framework-dependent Windows App SDK application. The package manifest declares `Microsoft.WindowsAppRuntime.2` version `2.3.1.0` and `Microsoft.VCLibs.140.00.UWPDesktop` version `14.0.33728.0` as x64 framework dependencies.
- Declare the manager as `packagedClassicApp` at `mediumIL` with `runFullTrust`.
- Declare `pastral-agent.exe` through one `windows.startupTask` named `PastralAgentStartup`. The no-argument agent entry point runs resident mode and uses `%LOCALAPPDATA%\Pastral`.
- Keep startup user-controllable. Windows may require the user to launch the manager once, and users can disable the startup task through Windows startup settings.
- Do not package a service, driver, updater daemon, diagnostic IPC executable, symbols, libraries, logs, or signing private key.
- Generate package assets and stage an exact allowlist under ignored `target/package/` output.
- Create development packages with a temporary self-signed certificate only when explicitly requested. Export only the public `.cer`; delete the generated `.pfx` after signing unless a local preservation switch is explicitly supplied.
- Public distribution requires an organization-controlled trusted signing identity and timestamping outside the repository.
- Prepare Store/WinGet metadata only after package identity, trusted signing, update, repair, uninstall-data behavior, and rollback qualification are stable.

## Development package evidence — 2026-08-05

The deterministic pipeline under `eng/` produced and verified:

- package: `Pastral-0.1.0.0-x64.msix`;
- identity: `Pastral.Development`;
- publisher: `CN=Pastral Development`;
- package size: `1,713,595` bytes;
- package SHA-256: `cabbed1dcb25dfcf2a5ebc21c0618540d8a34fc752ddf07e52427a0dfa6b02eb`;
- resident agent: `2,484,224` bytes;
- packaged manager: `606,208` bytes;
- manager bridge: `443,392` bytes.

`MakeAppx pack` validated the manifest and created the 17-file package. SignTool signed and verified one SHA-256 signature with zero warnings and zero errors while the development certificate was temporarily trusted. MakeAppx unpack plus per-file SHA-256 comparison proved extraction parity. The development private key was removed after signing.

Because the automation session was not elevated, signed installation could not temporarily import the public certificate into `LocalMachine\TrustedPeople`. The elevated install/uninstall script is present and fails clearly when not run as Administrator. A non-elevated loose-registration smoke nevertheless proved the same staged payload and manifest through Windows AppModel: x64 registration, Start Apps activation, startup-task declaration, manager window creation, authenticated resident Health/History, Connected UI state, cleanup, package removal, and restoration of any pre-existing `%LOCALAPPDATA%\Pastral` data root.

## Consequences

Positive:

- one package identity for the visible app, resident startup, resources, and assets;
- one resident storage owner with no service or elevation requirement at runtime;
- deterministic package contents and reproducible brand assets;
- explicit framework dependencies rather than bundling unused Windows App SDK bootstrap/WebView payloads;
- development signing never exposes a private key in distributable artifacts.

Costs and remaining gates:

- public users cannot trust the development certificate automatically;
- a trusted/timestamped production signature is still required;
- elevated signed install, same-version reinstall, upgrade, repair, interrupted update, rollback, and machine-wide policy scenarios require dedicated release evidence;
- startup enable/disable state and first-launch consent must be tested across supported Windows configurations;
- portable users are not served by this ADR.

## Alternatives considered

- **Single-project MSIX:** not used because Pastral deliberately stages multiple executables and an independently built Rust DLL through a reviewed package boundary.
- **Traditional MSI/EXE first:** flexible, but it would make Pastral own more identity, update, repair, startup, and rollback machinery.
- **Portable ZIP first:** rejected because it weakens startup, identity, signing, dependency, update, and data-location semantics for a resident utility.
- **Store-only:** rejected because local development, enterprise, and offline side-loading still need a signed package path.
- **Package diagnostic IPC separately:** rejected because it creates an unnecessary second executable surface and undermines the single-resident-process model.

## Review triggers

- Windows App SDK framework/package requirements change;
- MSIX startup-task or full-trust behavior no longer supports the resident model;
- Store or trusted-signing identity is selected;
- package update/rollback qualification fails;
- enterprise or portable demand justifies another packaging ADR;
- package footprint or startup measurements exceed accepted budgets.
