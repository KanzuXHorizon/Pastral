# Pastral MSIX packaging

This directory contains only reviewed packaging source files. Generated binaries, package assets, certificates, staging roots, and final `.msix` artifacts are written under ignored `target/` and `artifacts/` directories.

## Package model

- `pastral-manager.exe` is the visible packaged WinUI 3 application.
- `pastral-agent.exe` is the only resident process and is registered through a desktop startup task.
- `pastral-manager-ipc-bridge.dll` remains beside the manager executable.
- `pastral-agent-ipc.exe` is diagnostic-only and must never be staged.
- The package depends on the x64 Microsoft Windows App Runtime 2.3 and Microsoft VCLibs UWP Desktop framework packages.

## Development installer

Run from a PowerShell prompt:

```powershell
.\eng\build-msix.ps1 -CreateDevelopmentCertificate
```

The command builds Release binaries, stages an exact allowlist, generates brand assets, creates and validates the MSIX, signs it with a local development certificate, verifies extraction parity, and writes distributable development artifacts to `artifacts/`. `makeappx.exe` and `signtool.exe` are resolved only from the exact Windows SDK version pinned by `Directory.Build.props`; a newer installed SDK is never selected implicitly. The verification report records the SDK version and both tool hashes.

The exported `.cer` contains only the public key. The temporary `.pfx` and its password remain under ignored `target/package/signing/` and are deleted after signing unless explicitly preserved for local debugging.

A public release requires an organization-owned trusted signing identity and timestamping. The development certificate is not a public distribution credential.
