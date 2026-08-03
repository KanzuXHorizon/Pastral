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
6. Initialize the chosen COM apartment.
7. Create the message-only/hidden window.
8. Register clipboard listener, hotkeys, session/power notifications, and tray icon.
9. Start authenticated named-pipe listeners.
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

Initial design:

- one STA-capable Windows message thread for clipboard/OLE/UI-shell integration;
- one serialized storage executor/context;
- a bounded background work pool for lightweight post-capture tasks;
- overlapped named-pipe I/O and worker completion notifications;
- no periodic timer except narrowly scheduled retention/maintenance jobs with documented wake behavior.

Thread count and executor choice are benchmarked. Idle thread existence is acceptable; periodic wakeups are not.

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

- Started by tray, Start menu, settings link, URI/protocol only after protocol design, or CLI request.
- Connects to the current session's agent pipe.
- Performs version/capability negotiation.
- Retrieves paged view models and subscribes only to necessary change notifications.
- Does not start a private copy of the database or agent business logic.

### Failure behavior

- If agent is unavailable, manager displays a recoverable service state and offers an explicit safe restart action.
- IPC timeout never blocks the XAML UI thread.
- Reconnect is bounded and user-visible; no tight retry loops.

## pastral-cli.exe

The CLI uses the same protocol and capability negotiation as manager. Content-returning commands require an explicit flag and avoid printing to terminals by default when content is marked sensitive.

Exit codes are stable and documented for automation:

- success;
- invalid invocation;
- agent unavailable;
- authorization/protocol failure;
- operation rejected by policy;
- storage integrity failure;
- partial export/recovery result.

## Session and identity isolation

- Named-pipe name includes an unguessable installation/protocol component and current logon-session scope.
- DACL includes current-user SID and current logon SID; broad Everyone/anonymous access is prohibited.
- Cross-session manager/CLI connections are rejected even for the same account unless a future ADR explicitly supports them.
- Fast user switching creates independent agent instances and data access according to per-user app-data paths.

## COM apartment decision gate

The clipboard platform prototype must compare:

1. STA message thread using `OleInitialize`;
2. isolated clipboard STA with captured data transferred to storage work;
3. any MTA helper used only where APIs permit.

Fixtures must cover delayed rendering, OLE private formats, owner exit, contention, and shutdown. The final apartment decision is recorded before capture implementation.
