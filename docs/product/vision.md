# Product vision

## Mission

Pastral is a private, native Windows clipboard memory that preserves what the user copied, makes it easy to find later, and pastes the best safe representation the destination can accept.

> Copy once. Find anything. Paste perfectly.

“Perfectly” is an aspiration measured per format and destination, not a promise of universal losslessness.

## User jobs

Users hire Pastral to:

1. recover something copied minutes, days, or weeks ago;
2. find clips by content, source application, website, project, profile, type, time, or tag;
3. paste rich content, images, files, HTML, RTF, or plain text intentionally;
4. avoid repeatedly cleaning URLs, formatting code, or choosing the same paste action;
5. separate work, study, development, design, general, and private clipboard contexts;
6. understand why an automatic action occurred and reverse it;
7. keep clipboard content local and exclude sensitive applications or content;
8. receive quiet copy confirmation without losing focus or interrupting typing.

## Product principles

### Original data is immutable

Every transformation creates a derived representation with provenance. The captured original is never destructively rewritten.

### Copy cannot fail because Pastral failed

Pastral observes clipboard changes after the source application completes its copy operation. Contention, parsing, disk, database, or worker failure degrades capture but never blocks the source copy command.

### Local-first and network-silent

Core operation requires no account or network. Release builds make no network request unless the user explicitly enables a separately documented feature.

### Deterministic intelligence first

Classification and learned behavior use inspectable rules. No model is required for capture, search, organization, or paste.

### Quiet by default

The always-running agent is event-driven. The overlay is brief, optional, non-activating, and suppressed in disruptive contexts.

### Honest fidelity

Pastral reports what it preserved and where fallback occurred. Application-private formats may be unsafe, ephemeral, or impossible to replay.

### Progressive complexity

Quick Paste is fast and simple by default. Detailed representations, rules, security policy, diagnostics, and advanced filters remain available without dominating ordinary use.

## Measurable success

A production candidate succeeds only when evidence shows:

- ordinary text copy remains subjectively instant and meets the capture budget;
- idle agent CPU is effectively zero without events;
- passive overlay never changes foreground or keyboard focus;
- supported common formats survive restart and replay into representative destinations;
- original bytes remain unchanged after transformations;
- Quick Paste and search meet reference-hardware latency budgets;
- excluded applications and hard-deny clipboard signals are respected;
- highly confident secret fixtures do not reach storage, logs, notifications, previews, or indexes by default;
- crash-interrupted writes recover without losing committed history or exposing partial blobs;
- keyboard, UI Automation, high contrast, text scaling, reduced motion, and 100–300% DPI work in core flows;
- installation, update, uninstall, retention, backup, and recovery behavior are documented and tested.

## Product character

Pastral should feel like a careful Windows component:

- immediate;
- calm;
- native;
- accessible;
- conservative with CPU, memory, disk, GPU, and battery;
- explicit about privacy and automation;
- powerful through structure rather than visual noise;
- useful even with OCR, semantic search, integrations, network access, and animation disabled.
