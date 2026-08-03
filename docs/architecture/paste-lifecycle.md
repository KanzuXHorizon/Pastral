# Paste lifecycle

## Objectives

A paste transaction reconstructs the safest highest-fidelity representation set requested by the user, publishes it to the Windows clipboard, optionally invokes paste in the intended foreground destination, and preserves stored originals regardless of outcome.

## State model

```mermaid
stateDiagram-v2
    [*] --> Requested
    Requested --> Validating: clip + representation + destination policy
    Validating --> Rejected: unavailable/unsafe/locked/policy denied
    Validating --> Building: allowed
    Building --> Publishing: IDataObject ready
    Building --> Failed: reconstruction error
    Publishing --> ClipboardOwned: OleSetClipboard success
    Publishing --> Failed: clipboard contention/error
    ClipboardOwned --> CopyOnlyComplete: no paste requested
    ClipboardOwned --> Dispatching: paste requested
    Dispatching --> Observing: input sent / destination heuristic active
    Observing --> Complete: success or bounded unknown result
    Observing --> Failed: explicit failure/timeout
    Complete --> Restoring: transactional restore explicitly enabled and safe
    Restoring --> Complete: prior clipboard restored or skipped safely
    CopyOnlyComplete --> [*]
    Complete --> [*]
    Failed --> [*]
    Rejected --> [*]
```

## Request inputs

A transaction specifies:

- clip ID;
- requested representation or `preferred`/`original` policy;
- paste mode: copy only, paste, type Unicode fallback, file/path/name, sequential queue, or derived output;
- destination snapshot and compatibility profile;
- whether previous clipboard restoration is requested;
- user interaction/correlation ID;
- privacy and sensitive-unlock state.

## Validation

Before touching the clipboard:

- verify clip and blob integrity;
- authorize sensitive/private-profile access;
- reject deleted, expired, corrupt, unsupported, or unsafe representations;
- evaluate destination deny/compatibility policy;
- resolve deterministic representation priority;
- ensure file references still exist or report `ReferenceOnly`;
- cap reconstructed aggregate size and delayed-render lifetime;
- preserve a metadata-only result path.

## IDataObject construction

The replay object:

- offers all safe preserved representations needed for original/preferred modes;
- includes interoperable fallbacks such as Unicode text alongside rich formats when valid;
- preserves registered format identifiers and format ordering where destination behavior depends on it;
- owns all memory/stream lifetime through RAII;
- implements only required `IDataObject` operations and validates `FORMATETC`/`TYMED` requests;
- supports delayed rendering where it improves memory behavior and compatibility;
- never exposes worker staging paths or internal encrypted blobs directly.

Derived or plain-text modes publish only the selected derived set plus appropriate fallback metadata; they do not mutate stored originals.

## Clipboard publication

- Use `OleSetClipboard` for OLE-aware multi-format replay.
- Track the resulting sequence as self-generated to prevent duplicate history capture.
- Keep the data object alive until the destination has consumed it or the bounded lifetime policy resolves.
- Use `OleIsCurrentClipboard`/`OleFlushClipboard` only where transaction and shutdown semantics require them.
- Never restore the prior clipboard immediately after synthetic paste; many destinations read asynchronously.

## Destination invocation

Preferred order is compatibility-profile dependent:

1. return focus to the explicitly recorded destination only when Quick Paste had been user-invoked and focus restoration is safe;
2. send the configured paste gesture through documented input APIs;
3. observe bounded heuristics such as clipboard ownership/read patterns and foreground changes;
4. report success, failure, or `UnknownConsumed` honestly;
5. use Unicode typing only as an explicit compatibility fallback, never silently for rich content.

Pastral must not paste into a newly foregrounded unrelated application if destination identity changed after user selection.

## Previous clipboard restoration

Restoration is off by default for ordinary history paste because early restoration can break destinations.

When explicitly enabled:

- capture the prior clipboard through the same safety policy;
- wait for a destination-read heuristic or conservative timeout chosen per compatibility profile;
- verify Pastral still owns the current clipboard before restoration;
- skip restoration if another application/user changed the clipboard;
- publish the previous data object with the same lifetime guarantees;
- record metadata-only result and reason.

No heuristic is described as certain unless destination-specific evidence supports it.

## Failure behavior

- Reconstruction failure leaves the Windows clipboard unchanged where possible.
- Publication failure does not remove or alter stored data.
- Synthetic-input failure leaves the selected item on the clipboard so the user can press paste manually.
- Destination change cancels paste dispatch rather than risking a wrong target.
- Corrupt or tampered encrypted blobs are quarantined and reported without plaintext logging.
- Agent crash during delayed rendering is covered by shutdown/restart and `OleFlushClipboard` compatibility tests.

## Compatibility evidence

Test original, rich, plain, copy-only, and fallback modes against:

- Explorer;
- Notepad;
- Word, Excel, PowerPoint;
- Edge, Chrome, Firefox;
- VS Code and Visual Studio;
- Windows Terminal, PowerShell, Command Prompt;
- Discord and Slack;
- standard Win32 edit controls;
- WinForms and WPF controls;
- Pastral fixture destinations that inspect exact offered formats and bytes.

Each application profile records:

- accepted formats and priority;
- asynchronous-read behavior;
- paste shortcut/input constraints;
- restoration safety;
- known failures and workarounds;
- tested application/Windows version and date.

## Privacy and diagnostics

Logs may contain:

- transaction ID;
- format IDs and size buckets;
- source/destination application identifiers according to redaction policy;
- timings;
- result/error codes;
- compatibility-profile version.

Logs never contain payload bytes, pasted text, image pixels, file contents, private keys, tokens, or secret fragments.
