# ADR 0015: Hybrid Win32/OLE capture and replay ownership on a dedicated clipboard-platform STA

**Status:** Accepted
**Date:** 2026-08-04

## Context

Clipboard owners are untrusted and can expose delayed-render callbacks, marshaled `IDataObject` implementations, streams, handles, and malformed formats. `WM_CLIPBOARDUPDATE` must remain responsive, while `EnumFormatEtc`, `GetData`, `GetClipboardData`, or a rendering callback can block or re-enter. COM call cancellation is optional and cannot provide a universal hard timeout.

The Phase 0 single-STA wording did not adequately separate foreign clipboard calls from hotkeys, tray, overlay, session notifications, IPC health, and capture supervision.

## Decision

Use a hybrid capture model inside `pastral-agent.exe`:

1. A **control/overlay message thread** receives clipboard notifications, snapshots cheap source/foreground state, owns tray/hotkey/session messages and overlay HWNDs, and never calls foreign `IDataObject` methods.
2. A **dedicated clipboard platform STA** initialized with `OleInitialize` owns all foreign clipboard/OLE objects/media plus Pastral `OleSetClipboard` replay-object publication/lifetime, and has its own message pump/private helper HWND.
3. The control thread posts bounded `ClipboardObservation` and paste-publication intents to the clipboard-platform STA. The queue/state machine is bounded and tracks latest-state pressure without pretending every intermediate clipboard state remains observable.
4. The clipboard-platform STA first inspects Win32 clipboard formats/privacy flags and captures reviewed standard formats through adapters.
5. OLE `IDataObject` enumeration and `GetData` are supplemental and used only for adapters requiring `FORMATETC`, `lindex`, `IStream`, virtual-file, or richer medium semantics.
6. Foreign `STGMEDIUM` values are copied into Pastral-owned bytes/handles/staging files and released on the clipboard-platform STA with the correct ownership semantics. Foreign COM interfaces do not cross into storage/UI code. Pastral replay callbacks use only prevalidated owned memory or immutable pre-opened resources and never synchronously call SQLite, IPC, rules, or UI.
7. `CoEnableCallCancellation`/`CoCancelCall` may be prototyped around eligible marshaled calls. It is defense in depth, not the capture deadline contract.
8. A supervisor records clipboard-apartment progress. If the apartment is stuck beyond the health threshold, Pastral enters a visible degraded capture/paste state; it does not terminate the thread unsafely or spawn unbounded replacements.
9. If fixtures show that a blocked clipboard apartment cannot be recovered acceptably, a separate short-lived capture-broker process or replay-apartment split is designed through a new ADR before broad OLE/custom-format support.
10. A serialized storage executor receives only Pastral-owned, length-validated capture results.

## Consequences

Positive:

- hostile/delayed clipboard calls cannot directly freeze overlay, tray, hotkeys, IPC supervision, or session handling;
- COM apartment ownership and medium release are explicit;
- common formats avoid unnecessary OLE complexity;
- difficult OLE formats remain possible through adapters and evidence gates.

Costs:

- another resident thread and cross-thread capture/replay state machine;
- source snapshot and actual capture can occur at slightly different times;
- a blocked in-process Win32/COM call may still require degraded capture/paste operation or agent restart;
- some rapidly replaced intermediate clipboard states cannot be recovered.

## Alternatives considered

- **All capture on the control STA:** rejected because foreign calls can freeze the entire resident UI/control surface.
- **OLE-first for every format:** rejected as an unconditional rule because enumeration/get-data can invoke untrusted COM code when Win32 standard-format capture is sufficient.
- **Capture broker process for every copy:** strongest isolation, but process-launch latency/energy and clipboard-state races require prototype evidence before becoming the default.
- **Terminate stuck threads:** rejected as unsafe for COM, locks, memory ownership, and process integrity.

## Review triggers

- clipboard-platform STA hangs, replay callbacks block, or recovery fails under malicious fixtures;
- virtual-file/custom-format fidelity requires broader OLE use;
- thread/working-set budget is missed;
- process-launch benchmarks make an on-demand broker practical;
- apartment/device-loss interaction with overlay proves unreliable;
- Windows clipboard API behavior changes on the supported baseline.
