# Data flow

## Capture and enrichment flow

```mermaid
sequenceDiagram
    participant Src as Source application
    participant Win as Windows clipboard
    participant Agent as pastral-agent.exe
    participant Policy as Capture policy
    participant Store as SQLite/blob store
    participant Worker as pastral-worker.exe
    participant Overlay as Passive overlay

    Src->>Win: Publish clipboard IDataObject/formats
    Win-->>Agent: WM_CLIPBOARDUPDATE
    Agent->>Win: GetClipboardSequenceNumber
    Agent->>Agent: Coalesce duplicate/self-generated sequence
    Agent->>Win: Bounded OleGetClipboard/OpenClipboard retry
    Win-->>Agent: Short-lived IDataObject / formats
    Agent->>Policy: Source + format set + lightweight sensitivity signals
    alt Hard deny or high-confidence secret
        Policy-->>Agent: Skip payload
        Agent->>Store: Optional metadata-only SensitiveItemSkipped
        Agent->>Overlay: Privacy-safe suppressed/skipped status when enabled
    else Capture allowed
        Agent->>Agent: Capture common safe representations
        Agent->>Store: Stage blobs + metadata transaction
        Store-->>Agent: Durable clip ID / recovery token
        Agent->>Overlay: Confirmation view model
        opt Expensive or hostile enrichment enabled
            Agent->>Worker: Bounded staged input job
            Worker-->>Agent: Validated descriptor + output hash
            Agent->>Store: Commit derived representation
        end
    end
    Agent->>Win: Release foreign clipboard/data object promptly
```

## Query flow

```mermaid
sequenceDiagram
    participant UI as manager/Quick Paste/CLI
    participant IPC as Authenticated named pipe
    participant Agent as pastral-agent.exe
    participant Search as Typed query compiler
    participant DB as SQLite + FTS5

    UI->>IPC: Versioned request + cursor/page limits
    IPC->>Agent: Authenticated validated message
    Agent->>Search: Parse query syntax to typed AST
    Search->>Search: Validate fields, operators, ranges, privacy policy
    Search->>DB: Parameterized structured + FTS query
    DB-->>Search: Bounded rows and continuation cursor
    Search-->>Agent: Privacy-filtered view models
    Agent-->>IPC: Correlated response
    IPC-->>UI: Incremental results
```

## Rule evaluation flow

```mermaid
flowchart TD
    E[Clip or paste context] --> P[Privacy and deny rules]
    P -->|Denied| D[Skip/reject with explanation]
    P -->|Allowed| X[Exact user rules]
    X --> S[Specificity + explicit priority]
    S --> C{Conflict?}
    C -->|Equal destructive actions| R[Require user resolution]
    C -->|No| A[Produce action intent]
    A --> U[Execute bounded action]
    U --> L[Audit metadata + result]
    L --> O[Optional brief explanation/undo]
```

## Blob commit flow

Ordinary payload commit is designed around recoverable staging:

1. create an unpredictable temporary file in the blob staging directory;
2. stream bytes while enforcing size limits and computing the selected hash;
3. flush and close the staging handle according to durability policy;
4. derive the final content-addressed path for non-sensitive data;
5. atomically rename when possible, accepting an existing identical blob;
6. commit metadata references in SQLite;
7. recovery reconciliation removes orphan staging files and identifies unreferenced final blobs after grace periods.

Sensitive payload flow differs:

- encrypt before durable final placement;
- use random blob identifiers or keyed hashes;
- store versioned envelope metadata;
- never use plaintext equality as a filename or public index;
- do not create searchable preview or derivative metadata without explicit policy.

## Trust boundaries

```mermaid
flowchart LR
    Foreign[Foreign applications / clipboard owners]
    Agent[Trusted agent core]
    Worker[Restricted worker]
    UI[Manager and CLI]
    Disk[User storage]

    Foreign -->|Untrusted IDataObject, formats, paths, bytes| Agent
    Agent -->|Length-limited explicit jobs| Worker
    Worker -->|Untrusted until validated| Agent
    UI -->|Untrusted local IPC messages until authenticated/validated| Agent
    Agent -->|Encrypted or policy-filtered durable state| Disk
    Disk -->|Potentially corrupt/tampered files| Agent
```

Every arrow crossing into the agent is validated. Same-user origin is not equivalent to trusted input.

## Data minimization

- Source titles and paths are redacted or omitted according to profile policy.
- Logs contain identifiers, durations, sizes, format IDs, and result codes, not payloads.
- Search snippets are generated only from indexed non-sensitive data and obey preview policy.
- Paste occurrence tracking is optional and stores metadata, not destination document content.
- Diagnostic bundles sanitize usernames, paths, titles, domains, package identities, and clip IDs according to export level.
