# Source context and confidence

## 1. Purpose

Source-aware organization and rules are useful only when Pastral distinguishes direct platform evidence from inference. Source metadata can also reveal private document names, websites, projects, and activity patterns, so collection is minimized and every claim carries provenance/confidence.

## 2. Evidence classes

| Evidence | Meaning | Default confidence | Notes |
|---|---|---|---|
| Clipboard owner HWND/process | Window/process currently owning the clipboard when queried | High for process attribution when valid; not proof of user-visible source | May be NULL; OLE may use an internal owner window; data can remain after owner exit |
| Foreground HWND/process at notification | App/window active when Pastral received the update | Medium | A broker, automation, background copy, focus race, or delayed notification can make it wrong |
| Package identity/signature metadata | Identity of a resolved source process | High for process/package identity | Does not prove which document/tab produced content |
| Window class | Native control/top-level class | Medium | Useful for compatibility; can be generic/spoofed |
| Window title | Privacy-sensitive label supplied by a process | Low/medium | Disabled or redacted according to policy; never trusted as domain/project proof |
| Explicit browser/IDE integration | Signed/versioned first-party integration message bound to process/session | High within integration contract | Requires opt-in, permission, protocol validation, and separate threat/privacy review |
| User tag/profile/project assignment | Explicit user metadata | High for organization | Does not retroactively prove technical source |
| Heuristic inference | Conservative classification from multiple weak signals | Low | Must be labeled inferred and cannot drive privacy/destructive policy by itself |

`GetOpenClipboardWindow` is contention diagnostics only and is never source attribution.

## 3. SourceContext model

A durable source context contains only enabled fields:

- `source_process_id_at_observation` as transient evidence, not a durable identity by itself;
- executable/package stable identity and version/signature class where policy allows;
- owner/foreground evidence flags;
- window class;
- redacted or hashed title token only when enabled;
- integration-provided domain/project/workspace with integration ID/version;
- confidence enum: `ConfirmedIntegration`, `OwnerEvidence`, `ForegroundHeuristic`, `UserAssigned`, `Unknown`;
- observation and resolution timestamps;
- privacy policy version and redaction result.

Process IDs are reused by Windows. Persist process/package identity and evidence time; never use PID alone as a stable source key.

## 4. Collection rules

- Query cheap owner/foreground/process facts on the control thread without opening files or scanning UI trees.
- Resolve executable/package/signature metadata asynchronously and cache by process lifetime identity with invalidation.
- Do not read neighboring document text, browser accessibility trees, UI Automation content, or command lines for ordinary source attribution.
- Do not parse arbitrary window titles into domains/projects by default.
- Do not store full executable paths, command lines, usernames, titles, URLs, or project paths in release logs.
- A profile may omit source title/domain/project entirely while still storing broad application identity.
- Private/sensitive skips omit title/path/domain and use coarse source metadata only if the selected policy permits it.

## 5. Rule safety

- Privacy exclusions may use exact process/package allow/deny rules configured by the user or reviewed built-in deny entries.
- A low-confidence foreground/window-title inference cannot override a hard deny, reveal sensitive content, broaden retention, or trigger destructive transformation.
- Domain/project rules require explicit integration evidence or user assignment.
- When source evidence conflicts, store the conflict and use the lower safe confidence; do not choose arbitrarily.
- Rule explanations name the evidence class: for example, “matched source process owner” or “matched user-assigned project,” not simply “from this website.”

## 6. Browser private mode

There is no universal Win32 clipboard API that reliably reports every browser's private/incognito state.

- Before browser integration, private-mode suppression is limited to reviewed process/window signals and is labeled best effort.
- Do not claim a private context was detected when evidence is ambiguous.
- A browser extension/integration must be explicitly allowed in private mode by the user and must disclose exactly which context signal it sends.
- Failure/absence of integration falls back to ordinary conservative sensitive detection and user deny policy; it does not silently assert safety.

## 7. Source icons and display

- Resolve icons from trusted package/executable metadata without executing source-controlled code.
- Cache bounded icon derivatives, not arbitrary source paths.
- Text labels accompany icons; icon alone never communicates source.
- Unknown source displays `Unknown source` rather than a guessed application.
- Redacted source displays `Source hidden by privacy policy`.

## 8. Tests

- owner HWND valid, NULL, destroyed, internal OLE window, and PID reuse;
- foreground changes between notification and capture;
- background copy while another app is foreground;
- packaged and unpackaged processes;
- title privacy disabled/enabled/redacted;
- forged title/domain-looking title;
- integration valid, stale, wrong process/session, malformed, and revoked;
- rule conflict between owner, foreground, user assignment, and privacy deny;
- private-mode unknown state remains honest;
- source icon extraction handles malformed/unavailable binaries safely.
