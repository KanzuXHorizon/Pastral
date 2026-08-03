# Clipboard observation and event identity

## 1. Terms

### ClipboardObservation

A transient attempt to inspect the current clipboard after a notification or explicit health/recovery action. It is not a durable copy event and contains no payload after handoff completes.

Fields include:

- opaque observation ID;
- current `GetClipboardSequenceNumber` value and whether it was zero;
- monotonic observation time;
- wall-clock UTC time;
- notification generation/order within the agent;
- active Pastral paste transaction marker, if any;
- source/foreground evidence snapshot with confidence;
- active session/profile/policy version.

### ClipEvent

A durable successful capture. It requires at least one `ClipRepresentation` that passed capture and storage policy.

### CaptureAuditEvent

A durable content-free outcome such as high-confidence secret skipped, user deny, capture failure, integrity quarantine, or storage degradation. Source-owned hard-deny observations do not create a durable audit event.

## 2. Windows sequence semantics

- `WM_CLIPBOARDUPDATE` carries no event identifier and both message parameters are zero.
- `GetClipboardSequenceNumber` is a 32-bit serial for the current window station.
- Zero means the sequence is unavailable in the current access context; it is not a valid durable event ID.
- Compare sequence values primarily for equality/current ownership. Do not assume arithmetic distance equals the number of user copies.
- Handle wrap as normal unsigned serial behavior; do not treat a lower value as database rollback or corruption.
- Delayed rendering can affect when the sequence increments.
- Emptying and republishing the clipboard may generate multiple changes for one user operation.
- Multiple notifications queued before processing can all observe the same latest sequence/current clipboard state.

## 3. Coalescing rules

A notification may be coalesced when:

- the observed current sequence equals a successfully captured/denied current-state observation and no source/policy transition needs a separate audit record;
- it is proven to originate from the active Pastral paste transaction through an origin marker plus clipboard ownership/current-object checks;
- multiple queued observations point to the same current state before capture starts.

Do not coalesce merely because payload hashes match. Re-copying identical content is a meaningful event with new time/source/profile context once it is actually observed and captured.

## 4. Self-generated update suppression

Sequence equality alone is insufficient under races. Pastral replay data objects include a private registered origin format containing:

- protocol/magic version;
- random paste transaction ID;
- instance/session binding;
- no clip ID, payload hash, source, or sensitive metadata.

The agent suppresses a capture only when the marker validates against an active/recent transaction and ownership/sequence timing is consistent. Unknown or malformed origin markers are ignored as untrusted data, not treated as proof.

The private marker name is centralized and rebrandable. It is not a secret or authorization mechanism.

## 5. Burst behavior and honest loss accounting

Windows does not provide a historical queue of clipboard contents. If applications replace the clipboard faster than Pastral can acquire each state, intermediate states may be permanently unobservable.

Required behavior:

- bound the observation/capture queue;
- prioritize capturing the current final state;
- never block or slow the source copy operation;
- increment a metadata-only pressure/coalescing counter;
- record `possible_intermediate_state_loss` without claiming a precise number of copies;
- suppress false success overlays for states that were never durably captured;
- expose capture-health diagnostics and benchmark evidence.

The acceptance criterion is “no unbounded queue and final current state captured under the defined fixture,” not “every rapid copy is always preserved.”

## 6. Ordering and time

Durable local capture order uses a strictly increasing installation-local `capture_order` assigned by the serialized storage owner in the same transaction as the durable record. It is not a public ID, timestamp, or authorization value.

- `observation_ordinal` is scoped to one agent process and retained only as origin/diagnostic evidence.
- Default history/pagination order is `capture_order`, then stable opaque event ID.
- Alternate civil-time views sort by `observed_at_utc_us`, then `capture_order`, then event ID and label clock anomalies honestly.
- Import/merge assigns new local capture-order values while retaining original UTC/provenance; only a verified whole-vault restore preserves the original order domain.

Monotonic timestamps are used only for durations/deadlines and are not persisted as cross-restart wall time.

Clock changes can make wall-clock timestamps non-monotonic. The UI displays civil time without rewriting stored original timestamps or pretending that a clock rollback changed actual local capture order.

## 7. Failure outcomes

- **Hard deny:** no durable clip/audit event; optional in-memory aggregate counter only.
- **Sensitive skip:** hidden content-free `SensitiveItemSkipped` audit according to retention policy.
- **User/application deny:** optional content-free policy audit if enabled.
- **Clipboard unavailable:** content-free failure audit with sequence availability, attempts, duration bucket, and error code.
- **Possible sequence pressure:** aggregate diagnostic; no invented clip placeholder.
- **Partial capture:** durable `ClipEvent` only when at least one representation succeeded; unavailable formats carry fidelity/reason metadata.

## 8. Required fixtures

- duplicate notifications with one current state;
- identical content copied twice from same and different sources;
- sequence value zero;
- wrap from high unsigned values to low values in the abstraction tests;
- delayed rendering with sequence behavior;
- rapid replacements where only final state is available;
- Pastral replay marker valid, stale, forged, and malformed;
- Pastral replay followed immediately by an external copy;
- clipboard empty transitions;
- fast user switching/window-station isolation.
