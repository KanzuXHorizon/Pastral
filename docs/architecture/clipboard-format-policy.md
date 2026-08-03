# Clipboard format capture and replay policy

## 1. Principle

A clipboard format is supported only through a reviewed adapter that defines:

- stable identity;
- accepted `FORMATETC`/`TYMED` or Win32 handle form;
- ownership and release rules;
- length/allocation validation;
- durable serialization;
- replay reconstruction;
- fallback relationship;
- parser/isolation requirements;
- fidelity and limitation labels;
- security/fuzz fixtures.

Obtaining bytes does not by itself make a format safe or replayable. Unknown custom formats may contain pointers, handles, process-local object state, executable serialization, callbacks, or external references.

## 2. Stable format identity

### Standard formats

Persist the documented standard `CF_*` numeric ID and canonical symbolic name.

### Registered formats

Persist the exact registered format name as UTF-16/UTF-8 canonical schema text. The numeric value returned by `RegisterClipboardFormat` is runtime-local and is never the durable identity. Replay re-registers the exact name and caches the current runtime ID.

Rules:

- validate name length and encoding;
- preserve exact case/code units for replay while optionally storing a normalized search/display key separately;
- reject empty/unresolvable names from durable replay;
- never compare unrelated registered formats only by numeric ID across process lifetimes;
- runtime numeric IDs may appear only in ephemeral diagnostics with the registered name when available.

## 3. Adapter support matrix

| Format/class | Initial capture | Durable representation | Replay | Fidelity/notes |
|---|---|---|---|---|
| `CF_UNICODETEXT` | Copy bounded `HGLOBAL`; validate even byte length and terminating NUL policy | Preserve exact bounded raw bytes plus validated Unicode view metadata | Recreate `HGLOBAL`; include terminator according to adapter | Primary text original; invalid UTF-16 is isolated/unsupported rather than lossy replacement |
| `CF_TEXT` | Preserve bounded raw bytes and source locale/code-page evidence where available | Raw bytes + conversion provenance | Recreate raw bytes only when provenance is sufficient; also offer Unicode fallback | ANSI conversion is locale-sensitive; derived Unicode is not the same original |
| `CF_OEMTEXT` | Same pattern as ANSI with OEM code-page provenance | Raw bytes + provenance | Adapter-gated | Fallback only unless exact environment semantics are known |
| `CF_LOCALE` | Copy validated `LCID` companion | Metadata bound to text representation | Recreate when valid | Never treated as content alone |
| Registered `HTML Format` | Preserve exact clipboard-format bytes; validate header offsets/lengths before preview/index | Raw original bytes; sanitized preview/index is derived | Replay exact raw bytes plus text fallback | Parser runs bounded; malformed offsets do not cause unsafe slicing |
| Registered `Rich Text Format` | Preserve exact bounded bytes | Raw original bytes; preview/plain text derived | Replay exact bytes plus Unicode fallback where valid | Never execute embedded objects; parser/sanitizer isolated as needed |
| Registered URL formats (`UniformResourceLocatorW`, documented variants) | Preserve raw adapter-defined bytes | Raw original + parsed URL metadata only after validation | Replay registered format plus Unicode text | Do not auto-open or fetch URL |
| `CF_HDROP` | Parse `DROPFILES` and bounded path list safely | Reference list with original path text and existence state; no automatic file-content archive | Recreate path list only after validation/current availability | `ReferenceOnly`; paths may disappear/change; do not follow links/reparse/network locations automatically |
| Shell virtual files (`FileGroupDescriptorW` + `FileContents`) | Required prototype adapter using OLE `lindex` and supported media; release claim waits for exact fixtures | Descriptor + individually bounded captured streams only when policy permits | Adapter-gated | Common OLE scenario but high complexity; no support claim until multi-file/delayed/owner-exit tests pass |
| Registered `Shell IDList Array` (`CFSTR_SHELLIDLIST`) | Validate CIDA offsets/count/PIDL bounds without invoking namespace extensions | Same-machine/session-sensitive reference bytes + metadata only when policy allows | Adapter-gated `ReferenceOnly` | PIDLs may become stale and can refer to shell namespace objects; never auto-bind/open during capture |
| Registered `Preferred DropEffect` / `Performed DropEffect` | Copy validated DWORD companion metadata | Metadata bound to file/drop representation | Recreate only with relevant file/drop set | Never a standalone clip representation |
| Registered `DropDescription` | Validate enum and bounded Unicode fields | Companion presentation metadata only | Recreate only when safe/compatible | Never displayed as trusted source text without sanitization |
| `CF_DIB` / `CF_DIBV5` | Copy bounded global bytes; validate header, dimensions, stride, palette/masks, overflow | Exact raw DIB bytes | Recreate exact HGLOBAL | Decode/thumbnail is derived; malformed dimensions isolated |
| `CF_BITMAP` | Duplicate GDI bitmap only through reviewed adapter while owner is valid | A normalized captured pixel/bitmap representation plus provenance; raw handle is never durable | Recreate a new `HBITMAP` and offer encoded/DIB fallbacks | Cannot claim byte-identical handle fidelity; label common/adapter fidelity honestly |
| Registered `PNG` and reviewed encoded image formats | Preserve exact encoded bounded bytes | Raw encoded bytes | Re-register name and publish exact bytes | Preferred for encoded original; decoder only in worker/preview path |
| `CF_ENHMETAFILE` / `CF_METAFILEPICT` | Deferred until dedicated safe duplication/serialization adapter | None in MVP by default | Unsupported | Handle/object semantics and parser risk require separate design |
| OLE `TYMED_HGLOBAL` | Adapter-specific bounded copy | Adapter-defined bytes | Adapter-defined | `HGLOBAL` is a medium, not a generic safe schema |
| OLE `TYMED_ISTREAM` | Read bounded stream on the clipboard-platform STA to staging with cancellation/limits | Stream bytes only for a named adapter | Adapter-defined stream/HGLOBAL replay | Never read unbounded; stream may block/re-enter |
| OLE `TYMED_ISTORAGE` | Deferred | None by default | Unsupported | Compound-storage parsing requires isolated adapter/fuzzing |
| OLE `TYMED_FILE` | Treat path as untrusted reference; no automatic content read | Metadata/reference only if adapter explicitly supports it | Deferred | Prevent traversal/reparse/network side effects |
| OLE `TYMED_GDI`/`ENHMF`/`MFPICT` | Dedicated adapter only | Never serialize raw handle values | Adapter-dependent | Ownership/release on the clipboard-platform STA |
| Unknown registered/custom format | Enumerate name/medium/size availability only under privacy policy | Metadata descriptor only; no raw durable bytes by default | `UnsafeOrUnsupported` | Allowlist adapter required before capture/replay |
| Application-private format with process-local object/handle | Do not serialize blindly | Metadata-only unsupported descriptor | No | Honest limitation; preserve safe common fallbacks from same clip |

## 4. `FORMATETC` normalization

Persist only a safe normalized descriptor:

- stable format identity;
- `dwAspect` when supported and validated;
- `lindex` when semantically required, such as virtual-file content;
- supported `tymed` bitset and selected acquired medium;
- adapter/version and source ordinal/priority where observable.

Do not serialize `FORMATETC.ptd` pointer values. A target-device structure is copied only through a specific reviewed adapter with bounded length and stable schema; otherwise the representation is unsupported for exact replay.

A source advertising multiple media for one format may yield multiple acquisition candidates, but Pastral stores one logical representation with adapter evidence or explicit variants rather than duplicate history cards.

## 5. Medium ownership

- Every successful OLE `GetData` result is released according to `STGMEDIUM` rules, normally through `ReleaseStgMedium`, on the clipboard-platform STA.
- Honor `pUnkForRelease`; do not manually free the medium behind it.
- Never persist raw pointer, handle value, COM interface pointer, or `pUnkForRelease` identity.
- Duplicate/copy data before release using the adapter's documented ownership method.
- Win32 `GetClipboardData` handles remain clipboard-owned; Pastral locks/copies promptly and never frees the source handle.
- GDI/meta handles require documented duplication; integer handle values are not content.
- RAII wrappers encode ownership state and every unsafe boundary has invariants/safety comments.

## 6. Size and parsing policy

Limits are per adapter and aggregate per event:

- validate all offsets, counts, dimensions, strides, and additions/multiplications before allocation;
- cap raw capture size, stream bytes, item count, path length, virtual-file count, and output size;
- distinguish capture preservation from decode/parser limits;
- preserve encoded bytes without decoding where possible;
- run HTML/RTF/image/compound/custom parsing after capture in a restricted worker where risk/complexity warrants;
- reject decompression bombs and pathological metadata without deleting safe sibling representations.

A representation that exceeds policy is recorded as `Unavailable`/`UnsafeOrUnsupported` metadata only if the event has another successful representation and policy permits descriptor retention.

## 7. Format priority and fallbacks

- Preserve source enumeration/adapter priority as evidence, but replay priority is a versioned compatibility policy.
- Original/preferred mode offers all safe compatible representations, not just one selected format.
- Plain-text mode offers only the selected derived Unicode representation and explicitly justified companions.
- Rich formats include validated Unicode fallback where semantically correct.
- Do not synthesize a richer format and label it original.
- Destination compatibility profiles can suppress a format proven harmful, with explanation and versioned rule; stored originals remain unchanged.

## 8. Private origin marker

Pastral registers one private origin format for replay self-suppression. Its payload contains only version, random transaction ID, and instance/session binding. It contains no clip ID, source, content hash, profile name, or sensitive state.

The marker:

- is untrusted when received;
- must validate against active/recent transaction state;
- is not exported as user content;
- is not a security credential;
- is omitted when compatibility evidence shows a destination mishandles unknown formats, with self-suppression falling back to ownership/timing checks.

## 9. Custom-format onboarding gate

Adding capture/replay support for a custom format requires:

1. documented producer/consumer use cases and stable registered name;
2. exact byte/object schema or official format documentation;
3. ownership/lifetime model;
4. size/security analysis;
5. malformed/fuzz corpus;
6. capture and replay fixture producer/consumer;
7. compatibility/fidelity label;
8. privacy classification;
9. dependency/license review;
10. ADR or adapter decision record when the format expands trust/process boundaries.

No user setting enables blind “store every custom format” replay.

## 10. Required tests

- registered format obtains different runtime IDs across simulated registrations but replays by name;
- invalid/missing registered name;
- text terminators, odd lengths, invalid UTF-16, ANSI/OEM/locale cases;
- malformed HTML offsets and RTF payloads;
- `CF_HDROP` count/offset/path traversal/reparse/network paths;
- DIB dimension/stride/palette/mask overflow;
- `CF_BITMAP` owner exit and duplication failure;
- PNG exact-byte round trip;
- virtual-file multiple `lindex`, delayed streams, partial reads, owner exit;
- `pUnkForRelease` and every supported `TYMED` release path;
- unknown custom pointer-like payload remains unsupported;
- aggregate size limit with safe sibling format preserved;
- replay consumer verifies exact offered names/order/bytes/fallbacks.
