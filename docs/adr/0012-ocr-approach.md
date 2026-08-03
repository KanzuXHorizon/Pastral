# ADR 0012: OCR is an optional on-demand worker capability

**Status:** Deferred implementation; boundary accepted
**Date:** 2026-08-03

## Context

OCR can make screenshots and scanned images searchable, but image decoding and recognition increase attack surface, CPU, memory, disk, battery use, language-pack complexity, and privacy exposure. Pastral must remain useful with OCR disabled.

## Decision

Defer OCR until capture, storage, replay, Quick Paste, profiles, and worker isolation are proven.

When implemented:

- OCR runs only in `pastral-worker.exe`;
- it is disabled by default for Private profiles and sensitive items;
- no resident OCR engine or model is loaded;
- jobs are user-initiated or policy-enabled after durable original capture;
- decoded dimensions, input bytes, decompression ratio, time, memory, output length, and language set are bounded;
- OCR output is a derived representation with engine/version/language provenance;
- original images are never recompressed or replaced;
- users can delete/regenerate OCR independently;
- language packs and offline availability are explicit;
- remote OCR is outside this ADR and remains prohibited by default.

Initial implementation research compares Windows OCR/Windows AI APIs available on the supported baseline with a bundled/offline engine. Selection requires official support, redistribution/license review, security maintenance, language coverage, quality, footprint, and benchmark evidence.

## Consequences

Positive:

- core MVP avoids a large resident and parser burden;
- OCR failures cannot damage originals;
- privacy and resource behavior are profile-controlled and auditable.

Costs:

- image-text search is unavailable in early MVPs;
- later engine choice may affect installer size and worker sandbox permissions;
- language quality varies and requires fixtures.

## Alternatives considered

- **OCR in agent:** rejected due to footprint and crash/security boundary.
- **Mandatory cloud OCR:** rejected by local-first and network-silent principles.
- **OCR every image automatically:** rejected due to battery, privacy, and storage cost.

## Review triggers

- worker sandbox and image fixtures pass;
- lexical search and original image capture/replay are complete;
- supported Windows baseline exposes an appropriate stable offline API;
- product research identifies concrete OCR user demand and language requirements.
