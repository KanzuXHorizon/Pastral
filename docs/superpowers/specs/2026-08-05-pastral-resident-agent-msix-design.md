# Pastral Resident Agent and MSIX Design

**Date:** 2026-08-05
**Status:** Approved for implementation by the project owner’s request to complete the project and create an installer.

## Goal

Produce an installable Windows 11 x64 Pastral package whose normal startup path launches one resident `pastral-agent.exe` process that owns clipboard capture, storage, and authenticated read IPC, while `pastral-manager.exe` remains an on-demand WinUI 3 client.

## Non-goals

This slice does not add paste replay, Quick Paste, mutations, semantic search, sync, import/export, Private-profile encryption, automatic updater service, or public code signing. A locally trusted development certificate may sign the installer artifact; its private key must never be committed or distributed.

## Process model

- `pastral-agent.exe run` is the only resident process.
- The resident process starts the clipboard listener and authenticated `Health`, `HistoryPage`, and `Search` named-pipe endpoint in one supervised lifecycle.
- `pastral-agent-ipc.exe` remains a bounded diagnostic/test executable only and is not packaged.
- `pastral-manager.exe` and `pastral-manager-ipc-bridge.dll` are packaged together and connect to the resident agent through the existing authenticated protocol.
- The resident command defaults its data root to `%LOCALAPPDATA%\Pastral`; diagnostic commands retain explicit `--data-root` support.

## Lifecycle and failure policy

- Capture and IPC run concurrently inside one process.
- One shared atomic stop signal coordinates shutdown.
- The IPC listener uses a bounded connect timeout so it can observe stop requests without busy waiting.
- Authentication/protocol failures from one client are contained to that connection and do not terminate the resident process.
- Failure to initialize identity, privacy policy, storage, clipboard listener, transport material, pipe security, or the first pipe instance is fatal and content-free.
- If either supervised subsystem terminates unexpectedly, it requests shutdown of the other and the process exits nonzero.
- Test-only connection/event limits use the existing CLI bounds to make lifecycle tests deterministic. Production `run` omits both limits.

## IPC authorization

The resident endpoint advertises `Health`, `HistoryPage`, and `Search`. Clients may authenticate an allowed subset. The transcript binds the server-offered set and client-accepted subset, preventing capability injection or downgrade ambiguity.

## Storage ownership

Both capture and read operations remain inside `pastral-agent.exe`. The manager never opens SQLite or blob storage. Read requests use bounded storage handles owned by the resident process and return preview metadata only.

## Packaging

- Format: x64 MSIX, framework-dependent Windows App SDK.
- Contents: manager executable/resources, Rust bridge DLL, resident agent executable, generated package assets, manifest.
- Manifest applications:
  - one visible full-trust desktop application for `pastral-manager.exe`;
  - one `desktop6:StartupTask` that launches `pastral-agent.exe run` at sign-in.
- No service, driver, elevation, broad filesystem capability, network capability, or packaged diagnostic IPC executable.
- Package identity/publisher/version are deterministic inputs to the packaging script.
- Package assets are generated reproducibly from the Pastral violet/cyan brand tokens into the ignored staging directory.

## Signing and installer artifacts

- Release/public signing remains external and requires an organization-owned certificate.
- The repository packaging command can create a local self-signed development certificate when explicitly requested, sign the MSIX, export only the public `.cer`, and keep/delete the private `.pfx` outside version control.
- Final distributable development artifacts are:
  - signed `.msix`;
  - public `.cer` used to trust the development publisher;
  - SHA-256 checksum file;
  - package-content and signature verification report.

## Verification gates

1. CLI tests prove `run`, default root behavior, strict flag handling, and diagnostic bounds.
2. Resident tests prove simultaneous capture/IPC startup, authenticated Health/History/Search, bounded shutdown, and resilience after a rejected client.
3. Existing agent, IPC, bridge, manager static/build/UIA, formatting, Clippy, and diff gates remain green.
4. Packaging verification proves exact file allowlist, no diagnostic binary, valid manifest, successful `MakeAppx validate`, valid Authenticode signature, and package extraction parity.
5. Install smoke on the current machine proves install, manager launch, startup-task registration visibility, package removal, and no residual installed package. Enabling startup may still require the Windows user’s consent according to OS policy.

## Documentation updates

- ADR 0018 records production resident lifecycle/footprint evidence but remains Proposed for unrelated open C++ parser, fuzzing, bulk, and compatibility gates.
- ADR 0009 records the concrete package layout and development-signing path.
- Threat model records startup-task, package-signing, resident IPC, and update/rollback boundaries.
- Release checklist clearly distinguishes gates proven by this development installer from public-release gates that still require organization signing and broader hardware/accessibility/update evidence.
