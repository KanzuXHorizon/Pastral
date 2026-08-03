# Security and privacy incident response

## 1. Purpose

This document defines the minimum response for vulnerabilities or incidents involving clipboard content, keys, parsers, IPC, updates, storage integrity, or privacy policy. It does not replace an organization-specific legal or regulatory plan.

## 2. Severity

### Critical

- remote or low-interaction code execution through clipboard/parser/update paths;
- unauthorized bulk history/key disclosure;
- malicious signed update or signing-key compromise;
- hard-deny/privacy policy systematically bypassed;
- active exploitation with no practical mitigation.

### High

- IPC allows a client outside the documented user/logon-session boundary, or a same-user client performs sensitive/private/destructive operations without the required explicit user-intent authorization;
- sandbox escape from worker into broad history/key access;
- repeatable secret leakage into logs, diagnostics, or passive UI;
- irreversible migration corruption affecting many users;
- encrypted blob authentication/key-isolation failure.

### Medium

- limited metadata disclosure;
- denial of service requiring restart without data compromise;
- narrow parser crash contained to worker;
- configuration or UI behavior that can cause unintended retention with clear mitigation.

### Low

- hardening issue with no demonstrated confidentiality, integrity, or availability impact;
- inaccurate documentation or diagnostics that does not expose content.

## 3. Intake and evidence preservation

- Accept reports through the private channel documented in `SECURITY.md` after repository hosting is configured.
- Acknowledge without requesting real clipboard payloads.
- Ask reporters to use synthetic fixtures, hashes, metadata-only traces, and isolated reproduction steps.
- Assign incident ID, severity, affected versions/components, and owner.
- Preserve package hashes, signatures, build provenance, dependency lockfiles, sanitized logs, migration state, and reproduction fixtures.
- Do not upload user databases, blobs, keys, dumps, or screenshots to public issue trackers.

## 4. Immediate containment

Select the narrowest safe action:

- disable affected optional parser/feature through configuration or signed update;
- stop launching the worker job class;
- suppress unsafe format capture while preserving basic copy/history behavior;
- disable update channel or revoke package/certificate according to platform process;
- pause migrations or exports;
- advise users to pause capture/delete affected data only when evidence supports it;
- rotate IPC or profile keys when compromise scope requires it, while stating that IPC-secret rotation does not remediate a still-compromised same-user session.

Do not silently delete user history as a general containment technique.

## 5. Investigation

Answer:

- which versions, architectures, Windows builds, profiles, formats, and settings are affected;
- whether content, metadata, keys, rules, updates, or availability were impacted;
- whether exploitation crossed the worker, cross-user/logon-session IPC, storage, package, or user-intent authorization boundary, versus operating within the documented same-user residual-risk model;
- whether logs/dumps/backups created secondary copies;
- whether hard-deny or sensitive policy failed;
- whether evidence indicates active exploitation;
- whether data can be recovered without weakening confidentiality.

Use synthetic reproduction and test-first fixes. Preserve the earliest incorrect boundary and regression fixture.

## 6. Remediation

A security fix includes:

- focused failing regression test;
- smallest correct code/configuration change;
- adjacent boundary tests;
- dependency/toolchain review;
- threat-model and ADR update when assumptions changed;
- migration/recovery path when stored data is affected;
- performance/accessibility validation when mitigation changes user behavior;
- signed package and provenance verification.

No release claim is made until fresh verification runs complete.

## 7. Communication

Advisory content:

- affected and fixed versions;
- clear impact without exposing exploit details prematurely;
- prerequisites and realistic attack boundary;
- temporary mitigation;
- update/recovery/deletion steps;
- whether clipboard payload, metadata, or keys may have been exposed;
- known residual risks;
- credit and coordinated-disclosure timeline when appropriate.

Never include real user content, secret fragments, private paths, or identifying source titles.

## 8. Incident-specific playbooks

### Content in logs/diagnostics

1. Stop generation/upload of affected artifact.
2. Identify all local and remote copies/retention locations.
3. ship typed logging fix and canary tests;
4. instruct deletion/rotation appropriate to exposed content class;
5. inspect crash/support pipelines for secondary copies.

### Key compromise

1. Determine key scope: IPC secret, DEK, profile key, root key, signing key.
2. Revoke/rotate the narrowest affected key first.
3. Prevent reuse/replay.
4. Re-encrypt only data still trustworthy and recoverable.
5. State clearly when confidentiality cannot be restored retroactively.

### Vulnerable parser/worker

1. Disable the parser/job class.
2. keep original encoded payload handling only if safe;
3. update dependency/parser and fuzz corpus;
4. verify sandbox, job limits, and no-network policy;
5. re-enable only after malformed-input and compatibility tests pass.

### Malicious or invalid update

1. Stop distribution and update checks.
2. validate publisher certificate and provenance;
3. revoke or replace compromised signing material through platform procedures;
4. publish known-good hashes/package identity;
5. assess whether installed binary accessed clipboard data/keys;
6. provide clean reinstall and data-integrity guidance.

### Database/blob corruption

1. make a read-only evidence copy with user consent;
2. stop writes/migrations;
3. run integrity and reference reconciliation in isolated mode;
4. recover unaffected events without trusting corrupt payloads;
5. quarantine tampered encrypted blobs;
6. avoid overwriting the only recoverable copy.

## 9. Closure

An incident closes only when:

- root cause and affected boundary are documented;
- fix and regression tests pass;
- release/update is verified and signed;
- user guidance is published where needed;
- threat model, tests, docs, and release checklist are updated;
- residual risk and follow-up owner/deadline are recorded;
- lessons are converted into prevent/detect/respond improvements.
