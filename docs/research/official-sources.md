# Official-source research report

**Retrieved:** 2026-08-03
**Scope:** Phase 0 architectural decisions.
**Source policy:** Prefer Microsoft Learn, Rust project publications, SQLite project documentation, and primary project documentation. Blogs and community posts are not architectural authorities.

## 1. Windows support baseline

### Windows 11 servicing

- Microsoft documents an annual Windows 11 feature-update cadence. Home and Pro releases receive 24 months of support; Enterprise and Education receive 36 months.
- Windows 11 24H2 Home/Pro reaches end of servicing on 2026-10-13.
- Windows 11 25H2 Home/Pro reaches end of servicing on 2027-10-12.
- Windows 11 26H1 is intended for new devices and is not an in-place feature update for existing 24H2/25H2 devices.

Sources:

- https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information
- https://learn.microsoft.com/en-us/lifecycle/products/windows-11-home-and-pro

Design consequence:

- Pastral is Windows 11-only.
- MVP engineering uses Windows 11 24H2 build 26100 as its lowest test baseline.
- The release-candidate gate raises the public floor to the oldest Home/Pro release still serviced. A release after 2026-10-13 is expected to require Windows 11 25H2 build 26200.
- Pastral does not target 26H1-specific APIs unless runtime capability detection and a lower-path fallback exist.

### Windows SDK

Microsoft's Windows SDK downloads page lists stable build 10.0.28000.2526, released in July 2026. The release-notes page documents the 28000 SDK line and API changes.

Sources:

- https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads
- https://learn.microsoft.com/en-us/windows/apps/windows-sdk/release-notes

Design consequence:

- Build with a pinned stable SDK supported by the selected Visual Studio toolchain.
- Declaring a newer SDK does not authorize calling newer APIs unconditionally; the manifest minimum and runtime capability checks remain authoritative.

## 2. Windows App SDK and manager UI

Microsoft lists Windows App SDK 2.3.1, released 2026-07-16, as the current stable release in the 2.x line. Stable is the supported production channel; Preview and Experimental are unsupported for production.

Sources:

- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/downloads
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/stable-channel
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-notes/windows-app-sdk-2-0
- https://learn.microsoft.com/en-us/windows/apps/winui/winui3/
- https://learn.microsoft.com/en-us/windows/apps/develop/platform/
- https://github.com/microsoft/WindowsAppSDK/releases
- https://learn.microsoft.com/en-us/visualstudio/msbuild/solution-filters
- https://learn.microsoft.com/en-us/visualstudio/ide/projects-and-solutions-options-dialog-box

Design consequence:

- Pin Windows App SDK 2.3.1 stable for the first manager bootstrap.
- Use C++/WinRT and WinUI 3 only in the on-demand manager.
- Use the supported Visual Studio C++ WinUI project/MSBuild/XAML toolchain as the authoritative manager build. Microsoft's Windows App SDK release notes describe C++ CMake consumption as Experimental, so Pastral does not make it a stable release dependency.
- Use a separate Windows Application Packaging Project for the four-executable MSIX because single-project MSIX supports only one executable.
- Use `Pastral.slnx` as the planned manager solution format because supported current MSBuild/Visual Studio toolchains recognize it; a fallback to legacy `.sln` requires recorded manager/packaging-tool evidence.
- Do not load Windows App SDK into the always-running Rust agent solely for UI convenience.
- Re-evaluate the pinned patch release and supported project template/toolchain during repository bootstrap and before release; upgrading requires build, UI, accessibility, startup, and packaging verification.

Additional packaging source:

- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/single-project-msix

## 3. Rust toolchain

The Rust project published Rust 1.97.1 on 2026-07-16 as a point release correcting an LLVM miscompilation risk.

Source:

- https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/

Design consequence:

- Pin Rust 1.97.1 stable and Edition 2024 for the initial Cargo workspace.
- Dependency and compiler updates are deliberate changes with CI and benchmark evidence, not floating local state.

## 4. Clipboard eventing and sequence identity

`AddClipboardFormatListener` registers an HWND for `WM_CLIPBOARDUPDATE`; the message carries no clipboard event identity. `GetClipboardSequenceNumber` returns a 32-bit serial for the current window station, returns zero without `WINSTA_ACCESSCLIPBOARD`, increments when content changes or is emptied, and does not increment for delayed rendering until changes are rendered.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-addclipboardformatlistener
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardsequencenumber

Design consequence:

- Use an event-driven message-only or hidden window.
- No polling.
- Sequence number is current-state/equality evidence, not a durable event identifier or reliable copy count.
- Handle zero, unsigned wrap, duplicate queued notifications, and delayed rendering explicitly.
- Windows provides no historical queue of intermediate clipboard contents, so burst handling prioritizes the final current state and reports possible unobservable intermediates honestly.
- Self-generated suppression requires a validated private origin marker plus ownership/timing evidence; sequence equality alone is insufficient.

## 5. OLE clipboard and untrusted data

`OleGetClipboard` retrieves an `IDataObject`; Microsoft explicitly warns that clipboard data is not trusted and that returned data objects should be held only briefly. `IDataObject::EnumFormatEtc` and `GetData` expose multiple formats and storage media. `OleSetClipboard` publishes an `IDataObject` using delayed rendering; `OleFlushClipboard` can render supported formats so they survive source-process exit.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-olegetclipboard
- https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-olesetclipboard
- https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-oleflushclipboard
- https://learn.microsoft.com/en-us/windows/win32/com/data-transfer-interfaces

Design consequence:

- Use reviewed Win32 adapters for common formats and supplemental OLE access where `FORMATETC`, `lindex`, `IStream`, virtual-file, or richer medium semantics are required.
- Copy data out promptly; never retain a foreign data object longer than required or move raw foreign COM interfaces/media outside the capture STA.
- Preserve `FORMATETC`, medium type, and fidelity notes where safe.
- Complex/custom formats are hostile input and may be isolated in the worker.
- Replay object lifetime is part of the paste transaction and cannot end immediately after `OleSetClipboard`.

### Registered clipboard format identity

`RegisterClipboardFormatW` registers a format by name and returns an identifier in the registered range. `GetClipboardFormatNameW` retrieves the name for a registered identifier. Multiple applications registering the same case-insensitive name receive the same current registered-format identity.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerclipboardformatw
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardformatnamew
- https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats

Design consequence:

- Persist standard formats by defined standard ID and registered formats by exact registered name.
- Re-register the exact name at replay time; never use a persisted runtime numeric registered-format value as durable identity.
- Unknown custom formats require reviewed adapters and are metadata-only/unsupported by default.

## 6. Clipboard-owner privacy signals

Windows recognizes registered formats that affect history and cloud processing:

- `ExcludeClipboardContentFromMonitorProcessing` excludes the complete clipboard item from history and synchronization.
- `CanIncludeInClipboardHistory` with serialized DWORD zero excludes the item from history.
- `CanUploadToCloudClipboard` with serialized DWORD zero excludes synchronization.

Source:

- https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats

Design consequence:

- Pastral treats the first two history-exclusion signals as hard-deny capture policy.
- Normal settings do not override a source application's explicit request not to be monitored.
- The third signal is preserved as privacy metadata and prevents any future optional sync path, but core Pastral is network-silent regardless.

## 7. COM cancellation and clipboard-call limits

Microsoft documents `CoEnableCallCancellation`/`CoCancelCall` for eligible synchronous marshaled COM calls, but cancellation is opt-in, can materially reduce call performance, and depends on a cancel object/server cooperation. It is not a universal timeout for every clipboard, Win32, in-process, or non-cooperating call.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coenablecallcancellation
- https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cocancelcall
- https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-codisablecallcancellation

Design consequence:

- Foreign `IDataObject`/delayed-render calls do not run on the agent control/overlay thread.
- A dedicated capture STA owns foreign OLE objects and their release.
- Call cancellation is evaluated as defense in depth, never as proof of a hard deadline.
- A permanently blocked capture apartment requires a visible degraded state and a broker/process-isolation review rather than unsafe thread termination.

## 8. Focus-safe overlay primitives

Windows documents:

- `WS_EX_NOACTIVATE` for top-level windows that should not become foreground on click;
- `SW_SHOWNOACTIVATE` and `SWP_NOACTIVATE` for showing/positioning without activation;
- `WM_MOUSEACTIVATE`, whose default processing can activate a top-level window;
- tool-window behavior for taskbar/Alt+Tab exclusion, subject to owner/style verification.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos
- https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mouseactivate
- https://learn.microsoft.com/en-us/windows/win32/inputdev/about-mouse-input

Design consequence:

- The overlay uses multiple independent no-activation defenses and tests foreground HWND, focused HWND, active HWND, taskbar presence, and Alt+Tab presence.
- Mouse click behavior is verified; styles alone are not accepted as proof.
- Interactive keyboard focus requires an explicit user transition into a separate interaction state.

### Window display-affinity limitation

`SetWindowDisplayAffinity` can restrict a top-level window from certain public OS capture paths, but Microsoft explicitly states that it is not a security/DRM guarantee and cannot prevent every capture method.

Source:

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity

Design consequence:

- `WDA_EXCLUDEFROMCAPTURE` may be used as defense in depth for protected UI where supported.
- The primary control is keeping hidden content out of view models, UI Automation, thumbnails, and caches.
- UI/docs never promise that display affinity defeats every recorder, camera, same-user malware, RDP path, or unsupported compositor environment.

## 9. Overlay rendering and DPI

Direct2D provides hardware-accelerated 2D rendering with a software rasterizer fallback and integrates with DirectWrite. DirectComposition provides compositor-driven transforms, effects, and animations independent of the UI thread. Microsoft recommends Direct2D/DirectWrite for 2D graphics/text and DirectComposition for bitmap composition. Per-Monitor V2 supplies window-tree DPI notifications and requires responding to DPI changes.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-overview
- https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-and-directwrite
- https://learn.microsoft.com/en-us/windows/win32/directcomp/directcomposition-portal
- https://learn.microsoft.com/en-us/windows/win32/getting-started-with-directx-graphics
- https://learn.microsoft.com/en-us/windows/win32/hidpi/dpi-awareness-context
- https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged-beforeparent

Design consequence:

- Initial overlay ADR selects DirectComposition + Direct2D + DirectWrite.
- Animation is compositor-driven; no continuous CPU frame loop.
- A solid/software fallback exists for transparency-disabled, RDP, device-loss, and low-power conditions.
- Per-Monitor V2 is declared in manifests and tested across monitor transitions.

## 10. Accessibility

Microsoft's Windows accessibility guidance identifies UI Automation, keyboard support, readable contrast, high-contrast themes, text scaling, and manual assistive-technology validation as core requirements. Store accessibility claims require end-to-end validation with Narrator, Magnifier, On-Screen Keyboard, contrast themes, and DPI settings.

Sources:

- https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-overview
- https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-checklist
- https://learn.microsoft.com/en-us/windows/apps/develop/accessibility
- https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-in-the-store
- https://learn.microsoft.com/en-us/windows/win32/winauto/windows-automation-api-overview

Design consequence:

- Manager uses native controls and explicit automation names/roles/states where defaults are insufficient.
- Quick Paste and any focused overlay panel expose stable focus order and keyboard behavior.
- The passive confirmation overlay does not announce sensitive content; announcement behavior is configurable and rate-limited.
- Accessibility is verified continuously rather than postponed to release QA.

## 11. SQLite, FTS5, and journaling

SQLite documents FTS5 as its full-text virtual table module. SQLite transactions are atomic and durable across application, OS, and power interruption when used correctly. WAL adds `-wal` and `-shm` files and shared-memory behavior; its operational trade-offs differ from rollback journaling.

Sources:

- https://www.sqlite.org/fts5.html
- https://www.sqlite.org/transactional.html
- https://www.sqlite.org/atomiccommit.html
- https://www.sqlite.org/wal.html
- https://www.sqlite.org/tempfiles.html

Design consequence:

- FTS5 is the lexical-search foundation.
- Structured filters remain ordinary indexed columns.
- One agent process owns the database connection pool and schema changes.
- WAL is an evidence-based configuration decision, not a default copied from common recipes.
- Backup and diagnostic tools must treat sidecar files correctly when WAL is enabled.

## 12. Encryption and key protection

`CryptProtectData` normally protects data to the same user credentials on the same machine and adds an integrity check. The machine-wide flag permits any user on that machine to decrypt and is inappropriate for Pastral's user-private root key. Microsoft notes recovery limitations when credentials are administratively reset.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata
- https://learn.microsoft.com/en-us/windows/win32/seccrypto/example-c-program-using-cryptprotectdata

Design consequence:

- Generate independent random vault/data-encryption keys.
- Wrap root key material with user-scope DPAPI and `CRYPTPROTECT_UI_FORBIDDEN` for background operations.
- Use a versioned authenticated-encryption envelope for payloads; DPAPI is key protection, not the entire blob format.
- Backup/export documentation must state that DPAPI-bound data is not automatically portable.

## 13. Worker isolation

Windows job objects manage process trees and support resource limits. Restricted tokens disable privileges/SIDs. AppContainer provides file, network, process, device, credential, and window isolation, with access granted only through capabilities or explicit policy.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject
- https://learn.microsoft.com/en-us/windows/win32/secauthz/restricted-tokens
- https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-createrestrictedtoken
- https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation
- https://learn.microsoft.com/en-us/windows/win32/secauthz/createprocessinsandbox

Design consequence:

- Worker launch begins with a job object, kill-on-close, time/memory/output limits, a restricted token or supported sandbox API, and no network capability.
- AppContainer/LPAC feasibility must be prototyped against required file/IPC/COM operations before it is promised.
- The worker receives explicit input handles or staged files rather than broad data-directory access.

## 14. IPC security

Microsoft states that the default named-pipe security descriptor grants read access to Everyone and anonymous users. A pipe can be protected with an explicit security descriptor; the logon SID can prevent access from other terminal-services sessions.

Source:

- https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights

Design consequence:

- Never use a null/default pipe security descriptor.
- Grant normal client access through the current logon SID and narrowly justified SYSTEM access. Do not add a broad current-user allow ACE merely alongside the logon SID: allow ACEs are additive, so that would permit another logon session of the same account. Validate token user SID, logon SID, and session at runtime.
- Use individual access rights rather than broad `GENERIC_WRITE` where it could permit pipe-instance creation.
- Reject remote clients, prevent first-instance squatting, and validate peer token/session/PID where supported.
- Add protocol versioning, message-size limits, instance-bound handshake, request correlation, timeouts, and strict schemas.
- State explicitly that a user-scope DACL and DPAPI secret do not create a strong boundary against a fully compromised process already running as that user.

Additional sources:

- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipea
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid
- https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-client-impersonation

## 15. Paste input and integrity levels

Microsoft documents that `SendInput` is subject to User Interface Privilege Isolation (UIPI): input can be injected only into applications at an equal or lower integrity level, and a zero return does not reliably identify UIPI as the cause.

Source:

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput

Design consequence:

- Pastral stays at standard-user integrity and does not request `uiAccess`, elevation, or a service to bypass this restriction.
- Elevated or otherwise blocked destinations fall back to copy-only/manual paste with an honest explanation.
- Focus restoration and input dispatch are validated independently; uncertainty cancels synthetic input.

## 16. Clipboard owner and source evidence

`GetClipboardOwner` returns the current owner window where one exists, but clipboard data can remain when no owner exists. `GetOpenClipboardWindow` reports the window that currently has the clipboard open, which is contention evidence rather than source attribution.

Sources:

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardowner
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getopenclipboardwindow

Design consequence:

- Clipboard owner, foreground window, and explicit integration are distinct evidence classes.
- Missing or brokered ownership is reported as unknown/low confidence.
- Domain/project values are not inferred from arbitrary window-title text by default.

## 17. Packaging and deployment

Microsoft documents framework-dependent and self-contained Windows App SDK deployment, MSIX framework packages, dynamic dependencies, and the limitation that single-project MSIX supports only a single executable. Pastral contains multiple executables.

Sources:

- https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/deploy-packaged-apps
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/deploy-unpackaged-apps
- https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/framework-packages/framework-packages-overview
- https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/single-project-msix

Design consequence:

- Use a multi-executable packaging project rather than single-project MSIX.
- Primary public path is signed packaged MSIX with framework-dependent Windows App SDK for Store/managed deployment.
- Produce self-contained unpackaged builds only for controlled CI/diagnostic scenarios until portable update and data-location semantics are designed.

## 18. Research limits

- Documentation describes APIs and support contracts; it does not prove Pastral meets latency, focus, fidelity, security, or accessibility goals.
- All critical assumptions require fixture tests and measurements on supported Windows 11 builds.
- Name and trademark research is preliminary and cannot provide legal clearance.
