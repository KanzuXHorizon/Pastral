# Compatibility matrix

This matrix defines required environments and behaviors. Exact application versions and results are recorded in test reports, not guessed in advance.

## 1. Windows and architecture

| Dimension | Required coverage |
|---|---|
| OS floor | Windows 11 24H2 build 26100 during MVP engineering |
| Current serviced release | Latest serviced Windows 11 Home/Pro release at test time |
| Release floor | Oldest Home/Pro release still serviced at release candidate |
| Architecture | x64 first; ARM64 added only after separate readiness gate |
| Session | Console, lock/unlock, fast user switch, RDP where supported |
| Power | AC, battery, battery saver, suspend/resume |

## 2. Display and input

| Dimension | Required values/states |
|---|---|
| DPI | 100%, 125%, 150%, 175%, 200%, 250%, 300% |
| Displays | Single; dual mixed-DPI; monitor removed/added; negative coordinates; portrait |
| Work area | Taskbar each supported edge, auto-hide, scaled work area |
| Theme | Light, dark, high contrast/contrast themes |
| Transparency | Enabled and disabled |
| Motion | Normal and reduced-motion policy |
| Input | Keyboard, mouse, touch where hardware exists, pen smoke test |
| Direction | LTR and RTL layout/content |
| Text | Default, long localization, text scaling |

## 3. Clipboard source applications

Minimum source coverage:

- Pastral fixture producer;
- Notepad;
- File Explorer;
- Edge;
- Chrome;
- Firefox;
- VS Code;
- Visual Studio;
- Microsoft Word, Excel, and PowerPoint;
- Windows Terminal, PowerShell, and Command Prompt;
- Snipping Tool;
- Paint or another native image editor;
- Discord;
- Slack;
- representative Win32 edit, WinForms, WPF, and custom OLE controls.

For every source, record:

- available formats/media/order;
- delayed rendering and owner lifetime;
- source context confidence;
- clipboard exclusion flags;
- capture result/fidelity;
- contention or known incompatibility;
- application/Windows version and date.

## 4. Paste destinations

Minimum destination coverage:

| Destination | Original/common formats | Plain text | Image | File/list/path | Async consumption/restoration |
|---|---:|---:|---:|---:|---:|
| Fixture consumer | Required exact verification | Required | Required | Required | Required |
| Notepad | Text fallback | Required | N/A | Path text only | Observe |
| Explorer | File-oriented | N/A | Clipboard image where supported | Required | Required |
| Word | HTML/RTF/text | Required | Required | Required where supported | Required |
| Excel | HTML/RTF/text/table | Required | Required | Paths where supported | Required |
| PowerPoint | Rich/text | Required | Required | Required where supported | Required |
| Edge/Chrome/Firefox | Rich/text/URL | Required | Required where editor supports | Paths/files where supported | Observe |
| VS Code/Visual Studio | Text/code formats | Required | Editor-dependent | Paths/files | Observe |
| Terminal/PowerShell/CMD | Text | Required | N/A | Path/name | Input safety required |
| Discord/Slack | Rich/text | Required | Required | Required | Async read required |
| Win32/WinForms/WPF fixtures | Exact control behavior | Required | As supported | As supported | Required |

A cell marked “observe” requires documented behavior before enabling clipboard restoration by default. Restoration remains off unless evidence is strong.

## 5. Clipboard format fixtures

Required combinations:

- `CF_UNICODETEXT`;
- `CF_TEXT` fallback and code-page edge cases;
- HTML Clipboard Format with valid and malformed offsets;
- RTF;
- URL formats plus text fallback;
- `CF_HDROP` with Unicode file paths, unavailable targets, reparse points, and network paths;
- `FileGroupDescriptorW` + indexed `FileContents` virtual files with multiple files, delayed streams, cancellation, and owner exit;
- `Shell IDList Array`/CIDA valid and malformed offsets/PIDLs as `ReferenceOnly` without automatic namespace binding;
- `Preferred DropEffect`, `Performed DropEffect`, and `DropDescription` as validated companion metadata rather than standalone clips;
- DIB and DIBV5;
- bitmap handle scenario;
- encoded PNG registered format;
- simultaneous text + HTML + RTF + URL;
- simultaneous image + encoded image + text;
- registered format replayed by exact name after the runtime numeric ID changes;
- unknown custom format retained as metadata-only/unsupported rather than blindly replayed;
- `HGLOBAL`, `IStream`, `lindex`, `STGMEDIUM`/`pUnkForRelease`, and supported media variants;
- delayed rendering, blocked/non-cooperative owner, re-entrancy, cancellation refusal, and owner exit;
- empty clipboard;
- sequence unavailable/zero and unsigned-wrap abstraction;
- rapid repeated notifications/replacements with final-state capture and honest intermediate-state-loss diagnostics;
- valid, stale, forged, and malformed Pastral origin markers;
- clipboard busy/locked;
- malformed lengths and unsupported media;
- `ExcludeClipboardContentFromMonitorProcessing`;
- `CanIncludeInClipboardHistory=0`;
- `CanUploadToCloudClipboard=0`.

## 6. Source and paste-boundary compatibility

Required scenarios:

- clipboard owner and foreground application agree;
- owner absent/destroyed/internal OLE window;
- background copy while another app is foreground;
- foreground changes before capture and before paste;
- process ID reuse and package/executable identity resolution;
- window title resembles a URL/project but does not become high-confidence domain/project evidence;
- explicit integration valid/stale/wrong process/session;
- standard-user destination and elevated/higher-integrity destination;
- `SendInput` blocked/uncertain path leaves data on clipboard and requests manual paste without false success;
- Quick Paste cold manager activation, warm activation, bounded retained lifetime, and crash/restart.

## 7. Overlay compatibility

For every display/session state verify:

- foreground HWND unchanged;
- keyboard focus unchanged;
- active window unchanged;
- no taskbar button;
- no Alt+Tab entry;
- ordinary number/Escape keys unaffected;
- click does not activate passive overlay;
- work-area/taskbar/DPI placement correct;
- fullscreen/presentation/game/screen-share suppression policy;
- password/sensitive preview suppression;
- RDP, transparency-disabled, reduced-motion, and device-loss fallback;
- coalescing during copy bursts.

## 8. Accessibility compatibility

Manual and automated coverage:

- Narrator;
- Magnifier;
- On-Screen Keyboard;
- keyboard-only and switch-like sequential navigation;
- contrast themes;
- text scaling and 300% DPI;
- touch target size;
- UI Automation Inspect tooling;
- RTL and long localized strings;
- passive overlay announcement policy;
- sensitive/locked states without content leakage;
- accessible manual-paste fallback after blocked/uncertain synthetic input;
- display-affinity disabled/unsupported while hidden content remains absent from UI Automation/view models.

## 9. IPC protocol compatibility

Required Rust/C++ cross-language cases:

- exact 36-byte frame header at arbitrary alignment, including explicit bulk frame sequence;
- byte-mode fragmentation/coalescing: every header/body split point, one byte per read, multiple complete/partial frames in one read, short writes/reads, and no dependence on a `WriteFile` or Windows message boundary;
- invalid magic, framing/protocol major, kind, reserved flags, UUID use, body length, truncation, slow/disconnected peer;
- hello order, challenge replay, wrong instance/session, capability/minor negotiation;
- Protobuf Edition 2024 explicit presence, zero/unknown security-critical enums, unknown fields/actions, deleted-field reservation, duplicate key records, nesting/string/repeated limits, and body fields that cannot override header correlation or negotiated connection capabilities;
- exact generator/generated-code/runtime compatibility enforced for whichever Rust/C++ implementation passes ADR 0018 acceptance; the official v35.0 release-train Rust-kernel path and at least one credible wire-compatible Rust alternative are compared rather than assumed;
- adjacent supported client/server version golden bytes in both directions;
- DTO-domain conversion rejects parser-valid but invariant-invalid input;
- bulk open/chunk/end/cancel/disconnect, duplicate/gap/reorder/wrong connection/direction/sequence/count/length/digest, low disk, expired authorization, and staging cleanup;
- sensitive/Private transfer creates no plaintext digest/equality metadata or content-bearing error/log;
- agent idle/binary/working-set delta with protocol runtime linked;
- manager built through supported `.vcxproj`/MSBuild/XAML path and multi-executable `.wapproj`, not experimental Windows App SDK CMake support.

## 10. Packaging compatibility

- clean standard-user install;
- packaged process activation for all executables;
- startup registration enabled/disabled;
- update with manager open, agent running, worker active, and clipboard owned;
- rollback/recovery after interrupted migration;
- repair;
- uninstall with documented data choice;
- Store/framework dependency unavailable or damaged;
- offline side-load where dependencies are supplied;
- code-signing/publisher verification;
- no Windows 10 installation claim.

## 11. Result states

Every tested cell records one of:

- `Pass`;
- `PassWithDocumentedFallback`;
- `KnownLimitation`;
- `BlockedByExternalIssue`;
- `Fail`;
- `NotApplicable`.

No empty cell is treated as supported. A compatibility profile is enabled only from recorded evidence, not application-name assumptions.
