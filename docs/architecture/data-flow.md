# Data flow

## Capture and enrichment flow

```mermaid
sequenceDiagram
    participant Src as Source application
    participant Win as Windows clipboard
    participant Control as Agent control/overlay thread
    participant Capture as Agent clipboard-platform STA
    participant Policy as Capture policy
    participant Store as SQLite + BlobStore
    participant Worker as pastral-worker.exe
    participant Overlay as Passive overlay

    Src->>Win: Publish clipboard IDataObject/formats
    Win-->>Control: WM_CLIPBOARDUPDATE
    Control->>Win: GetClipboardSequenceNumber
    Control->>Control: Snapshot transaction/source evidence
    Control->>Capture: Post bounded ClipboardObservation
    Control-->>Win: Return from window procedure
    Capture->>Win: Bounded OpenClipboard retry
    Win-->>Capture: Standard/registered format set
    Capture->>Policy: Privacy flags + source evidence + safe lightweight signals
    alt Source-owned hard deny
        Policy-->>Capture: Hard deny
        Capture-->>Control: Ephemeral suppressed state; no durable row
    else High-confidence secret
        Policy-->>Capture: Skip payload
        Capture->>Store: Hidden content-free SensitiveItemSkipped (24h default)
        Capture-->>Control: Privacy-safe skipped status when enabled
    else Capture allowed
        Capture->>Capture: Capture reviewed Win32 adapters
        opt Adapter requires OLE semantics
            Capture->>Win: Short-lived OleGetClipboard / FORMATETC request
            Win-->>Capture: Foreign IDataObject / STGMEDIUM
        end
        Capture->>Store: Owned payload + metadata transaction intent
        Store-->>Capture: Durable clip ID / recovery token
        Capture-->>Control: Immutable confirmation view model
        Control->>Overlay: Show focus-safe confirmation
        opt Expensive or hostile enrichment enabled
            Capture->>Worker: Bounded staged input job
            Worker-->>Capture: Validated descriptor + output hash
            Capture->>Store: Commit derived representation
        end
    end
    Capture->>Win: Release foreign clipboard/data object on clipboard-platform STA
```

## Query flow

```mermaid
sequenceDiagram
    participant UI as manager/Quick Paste/CLI
    participant IPC as Session-isolated named pipe
    participant Agent as pastral-agent.exe
    participant Search as Typed query compiler
    participant DB as SQLite + FTS5

    UI->>IPC: Versioned request + cursor/page limits
    IPC->>Agent: Peer/session checked + framed validated message
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

The normative lifecycle is defined in [`blob-store-lifecycle.md`](blob-store-lifecycle.md). Ordinary payloads share one logical content-addressed identity while a versioned benchmark policy selects the physical backend.

### Internal SQLite BLOB

1. validate the bounded owned payload and compute `sha256-raw-v1`;
2. begin the SQLite storage transaction and look up an existing compatible logical blob;
3. insert the internal BLOB object when absent and attach representation/reference metadata;
4. commit atomically; rollback cannot expose an incomplete referenced internal object.

### External file

1. create an unpredictable staging file under the controlled data root;
2. stream bytes while enforcing limits/reserve and computing `sha256-raw-v1`;
3. flush/close, deduplicate against an existing compatible logical blob, or atomically finalize to a controlled object key;
4. commit the external locator plus metadata references in SQLite;
5. reconciliation removes stale staging/orphan candidates only after grace/recheck.

Threshold changes do not rewrite blobs on the capture path. Backend migration copies/verifies a candidate, transactionally switches the authoritative locator, then removes the old location after reader/recovery safety.

Sensitive payload flow differs:

- encrypt before durable final placement;
- use random blob identifiers and no persistent plaintext digest/deduplication by default;
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
    UI -->|Untrusted local IPC messages until peer/session/schema/operation checks| Agent
    Agent -->|Encrypted or policy-filtered durable state| Disk
    Disk -->|Potentially corrupt/tampered files| Agent
```

Every arrow crossing into the agent is validated. Same-user origin is not equivalent to trusted input, and the pipe protocol is not claimed as a strong confidentiality boundary against a fully compromised same-user process.

## Data minimization

- Source titles and paths are redacted or omitted according to profile policy.
- Logs contain opaque identifiers, durations, size buckets, standard format IDs or registered-format names according to redaction policy, and result codes—not payloads or unstable runtime registered-format IDs.
- Search snippets are generated only from indexed non-sensitive data and obey preview policy.
- Paste occurrence tracking is optional and stores metadata, not destination document content.
- Diagnostic bundles sanitize usernames, paths, titles, domains, package identities, and clip IDs according to export level.
