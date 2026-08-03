# Process model

## Security and lifetime table

| Process | Normal lifetime | Privilege | Network | Primary data access | Crash boundary |
|---|---|---|---|---|---|
| `pastral-agent.exe` | User session | Standard user | None by default | Exclusive DB/migration owner; blob owner | Core availability boundary |
| `pastral-worker.exe` | Per job/batch | Restricted/sandboxed | Denied by default | Explicit staged input/output only | Hostile parser/codec boundary |
| `pastral-manager.exe` | User invoked | Standard user | None for core | IPC only | UI boundary |
| `pastral-cli.exe` | Per command | Standard user | None for core | IPC only | Administrative client boundary |

Pastral does not install a Windows service for the MVP. Clipboard access and foreground context are user-session concerns, and a service would expand privilege and session-isolation complexity.

## pastral-agent.exe

### Startup

1. Verify single instance for the current user and logon session.
2. Initialize structured metadata-only logging.
3. Resolve app data paths and permissions.
4. Open or recover SQLite/blob storage.
5. Run schema migration under an exclusive migration gate.
6. Start the dedicated clipboard platform STA (capture + replay ownership) and wait for its readiness handshake.
7. Initialize the control/overlay message thread and create its message-only/hidden window.
8. Register clipboard listener, hotkeys, session/power notifications, and tray icon.
9. Start named-pipe listeners with explicit security and peer-validation policy.
10. Enter the event-driven message/wait loop.

### Shutdown

1. Stop accepting new manager/CLI requests.
2. Cancel or drain bounded capture-independent work.
3. Terminate worker jobs through job-object lifetime controls.
4. Resolve clipboard ownership: keep/flush replay data only according to paste transaction policy.
5. Checkpoint/close storage according to configured journaling mode.
6. Remove hotkeys, clipboard listener, tray icon, and overlay windows.
7. Zero sensitive key buffers where practical and release DPAPI-unwrapped keys.

Unexpected termination is assumed; recovery cannot depend on graceful shutdown.

### Thread model

Accepted boundary:

- one control/overlay Win32 message thread for listener HWND, tray, hotkeys, session/power notifications, and native overlay; it never invokes foreign clipboard `IDataObject` methods;
- one dedicated clipboard platform STA initialized with `OleInitialize`, its own message pump, and exclusive ownership of foreign clipboard/OLE interfaces/media plus Pastral replay data-object publication/lifetime;
- one serialized storage executor/context;
- a bounded background work pool for lightweight post-capture tasks;
- overlapped named-pipe I/O and worker completion notifications;
- no periodic timer except narrowly scheduled retention/maintenance jobs with documented wake behavior.

The control thread posts bounded `ClipboardObservation` and paste-publication intents to the clipboard STA and returns immediately. The STA copies validated foreign data into Pastral-owned buffers/handles/staging files before storage handoff and serves Pastral delayed-render callbacks only from prevalidated owned resources—never SQLite/IPC/business queries. COM cancellation is best effort; a stuck clipboard STA creates a visible degraded capture/paste state rather than unsafe thread termination or unbounded replacement threads.

Thread count and executor choice are benchmarked. Idle thread existence is acceptable; periodic wakeups are not. Detailed ownership is defined in `threading-and-com-apartments.md`.

## pastral-worker.exe

### Launch contract

The agent creates a job object before or immediately around process launch and configures:

- kill-on-job-close;
- active-process limit;
- per-process/job memory limits appropriate to job class;
- wall-clock timeout in the agent;
- output-size limit;
- child-process prohibition unless a specific job class requires one;
- restricted token or supported Windows sandbox/AppContainer policy;
- no network capability by default;
- explicit inherited handles only.

### Data contract

The worker receives:

- protocol version;
- job identifier and type;
- read-only input handle or isolated staged path;
- exact input byte length and expected hash;
- output handle/path with quota;
- parser limits and deadline.

The worker returns:

- status code;
- validated output descriptors;
- output hashes and lengths;
- non-sensitive diagnostics;
- no arbitrary serialized object graph.

Agent validates all worker output before persistence.

## pastral-manager.exe

### Activation

- Single-instance on-demand UI process started by Quick Paste hotkey activation, tray, Start menu, settings link, URI/protocol only after protocol design, or CLI request.
- Hosts both the full manager shell and Quick Paste as distinct windows/activation modes in the same WinUI 3 process.
- `Warm Quick Paste` means this UI process is already running; cold process activation is measured and reported separately.
- Connects to the current session's agent pipe.
- Performs version/capability negotiation.
- Retrieves paged view models and subscribes only to necessary change notifications.
- Does not start a private copy of the database or agent business logic.

### Failure behavior

- If agent is unavailable, manager displays a recoverable service state and offers an explicit safe restart action.
- IPC timeout never blocks the XAML UI thread.
- Reconnect is bounded and user-visible; no tight retry loops.

## pastral-cli.exe

The CLI uses the same protocol and capability negotiation as manager. Ordinary content-returning commands require an explicit flag and avoid printing payloads by default. Private/sensitive plaintext cannot be retrieved solely by a CLI flag; it requires a separately authorized user-intent flow and may be unavailable to noninteractive use.

Exit codes are stable and documented for automation:

- success;
- invalid invocation;
- agent unavailable;
- authorization/protocol failure;
- operation rejected by policy;
- storage integrity failure;
- partial export/recovery result.

## Session and identity isolation

- Named-pipe name includes a random installation/protocol component and current logon-session scope; the name is not treated as a secret.
- First server creation uses anti-squatting and remote-client rejection flags, an explicit least-privilege logon-SID DACL, and peer token-user/logon-SID/session/PID validation. A broad user-SID allow ACE is not added because it would defeat same-account cross-session isolation.
- Cross-session manager/CLI connections are rejected even for the same account unless a future ADR explicitly supports them.
- Fast user switching creates independent agent instances and data access according to per-user app-data paths.
- DACL, peer identity, and user-scope DPAPI strongly separate users/sessions and reduce stale/replay/confusion risk, but do not claim confidentiality from fully compromised code running as the same unlocked user.
- Sensitive reveal/export/destructive operations require explicit foreground user intent and profile authorization freshness beyond basic pipe connection.

## COM apartment decision

ADR 0015 selects a dedicated clipboard platform STA for capture and replay ownership. The prototype still measures Win32-only versus supplemental OLE adapters, eligible COM cancellation overhead, replay callback latency, and whether a separate capture broker or replay apartment is required for non-cooperative owners.

Fixtures cover delayed rendering, OLE private formats, owner exit, re-entrancy, blocked calls, contention, shutdown, medium ownership, and control-thread responsiveness. Raw foreign COM interfaces or `STGMEDIUM` values never leave the capture apartment.
