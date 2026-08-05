# Phase 3J — MSIX Development Installer Verification

**Date:** 2026-08-05  
**Scope:** deterministic x64 staging, packaged WinUI build, multi-executable manifest, brand assets, development signing, extraction parity, package registration, Start Apps activation, startup declaration, live IPC, and cleanup

## Result

**PASS for a signed development MSIX artifact and non-elevated package-registration/runtime smoke.** Elevated signed installation remains an environment gate because the automation session was not Administrator. Public release signing, timestamping, update, repair, and rollback remain open.

## Artifact

| Property | Value |
|---|---|
| Package | `Pastral-0.1.0.0-x64.msix` |
| Identity | `Pastral.Development` |
| Publisher | `CN=Pastral Development` |
| Architecture | x64 |
| Size | 1,713,595 bytes |
| SHA-256 | `cabbed1dcb25dfcf2a5ebc21c0618540d8a34fc752ddf07e52427a0dfa6b02eb` |
| Public certificate | `Pastral-Development.cer` |
| Private key | generated temporarily, not distributed, deleted after signing |

The package contains 17 staged files. Diagnostic IPC executables, probes, symbols, libraries, logs, private keys, Windows App SDK bootstrap payload, and unused WebView files are excluded.

## Package architecture

- visible application: `pastral-manager.exe`;
- resident startup executable: `pastral-agent.exe`;
- native bridge: `pastral-manager-ipc-bridge.dll`;
- resources: PRI, WinMD, XBF page/theme output;
- assets: reproducibly generated Store, square, wide, and splash PNGs using Pastral Aurora Violet and Signal Cyan;
- dependencies: Windows App Runtime 2.3.1.0 x64 and Microsoft VCLibs UWP Desktop;
- trust/runtime: `packagedClassicApp`, `mediumIL`, `runFullTrust`;
- startup: `PastralAgentStartup` targeting only `pastral-agent.exe`.

## Packaged-manager root-cause correction

The first staged manager was compiled with `WindowsPackageType=None`, which inserted bootstrap/dynamic-dependency auto-initializers intended for unpackaged applications. Direct `CreateProcess` worked, but AppModel activation created a process that exited before a window appeared.

A controlled comparison built the same manager with packaged mode and disabled unpackaged auto-initializers. Replacing only the executable made Start Apps activation create a stable window. The staging pipeline now always passes packaged Windows App SDK properties and excludes the no-longer-used bootstrap/WebView files.

## Build/signature evidence

- `MakeAppx pack /h SHA256` validated the rendered manifest and created the package.
- SignTool created one SHA-256 primary signature.
- Verification while the development certificate was temporarily trusted reported one successful signature, zero warnings, and zero errors.
- The package is intentionally not timestamped because this is a disposable development certificate.
- MakeAppx unpack succeeded; every staged payload file was found after extraction with the same SHA-256 hash.
- Exact manifest assertions passed for identity, x64 architecture, framework dependencies, manager runtime/trust level, startup task, and `runFullTrust`.

## Runtime smoke evidence

Because AppX deployment validates signing trust in the Local Computer context, the non-elevated automation session could not import the development certificate into `LocalMachine\TrustedPeople`. `test-msix-install.ps1` now requires an elevated PowerShell session and performs temporary machine trust, signed installation, activation, live IPC, uninstall, and trust cleanup.

The available non-elevated smoke used Windows-supported loose package registration against the exact staged payload:

1. reject foreign Pastral processes;
2. back up any existing `%LOCALAPPDATA%\Pastral` root by same-volume rename;
3. register `AppxManifest.xml`;
4. verify x64 identity, package location, manager runtime/trust, and enabled startup task;
5. start the staged resident agent with a two-connection bound;
6. activate `PastralManager` through `shell:AppsFolder`;
7. verify a top-level window, `Pastral agent is connected`, and `0 items` through UI Automation;
8. close the manager and require bounded resident shutdown;
9. unregister the package, remove smoke data, and restore any prior data root.

Result: `Pastral packaged registration, Start Apps activation, live IPC, and cleanup smoke: PASS`.

## Reproduction

```powershell
.\eng\build-msix.ps1 -CreateDevelopmentCertificate
.\eng\test-msix-registration.ps1 `
  -StagingDirectory .\target\package\Pastral_0.1.0.0_x64
```

Run the signed install/uninstall gate from an Administrator PowerShell:

```powershell
.\eng\test-msix-install.ps1 `
  -PackagePath .\artifacts\Pastral-0.1.0.0-x64.msix `
  -CertificatePath .\artifacts\Pastral-Development.cer
```

## Remaining public-release gates

- trusted organization/Store/Azure Artifact Signing identity and timestamp;
- elevated signed install execution evidence on clean supported machines;
- same-version handling, upgrade, downgrade rejection, repair, interrupted update, rollback, and uninstall-data policy;
- startup enable/disable and first-launch behavior across local, managed, and policy-restricted systems;
- Store/WinGet metadata and certification;
- formal accessibility, high-contrast, text scaling, RDP, startup, and footprint release matrix.
