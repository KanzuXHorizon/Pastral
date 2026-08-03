# Security policy

## Supported versions

Pastral has not released executable software. No version is currently supported for production use.

After the first release, this file will list supported release lines, security-update expectations, and end-of-support dates.

## Reporting a vulnerability

Do not open a public issue for a vulnerability or attach clipboard data, databases, blobs, keys, logs, dumps, screenshots, or private paths to a public report.

Until repository hosting is configured, report privately to the project owner through the same private channel used to receive this repository. After hosting is configured, use the repository host's private security-advisory mechanism. A dedicated security contact may be added only after it is active and monitored.

Include, using synthetic data:

- affected commit/version and Windows build;
- component and boundary: clipboard, worker, IPC, storage, paste, overlay, package/update, export/import;
- prerequisites and expected impact;
- minimal reproduction steps;
- fixture or hash that contains no real secret/private content;
- whether exploitation crosses user/session/process boundaries;
- proposed mitigation if known.

Do not send real passwords, API keys, private keys, OTPs, cookies, customer data, or personal clipboard history. See [`docs/testing/test-data-policy.md`](docs/testing/test-data-policy.md).

## Response expectations

The project will aim to:

- acknowledge a complete private report promptly;
- assign severity and scope;
- reproduce with synthetic fixtures;
- coordinate remediation and disclosure;
- credit the reporter when requested and appropriate;
- publish affected/fixed versions and mitigation when a release exists.

Exact response-time guarantees are not published before a maintained security team/contact exists.

## Security architecture

Primary documents:

- [`docs/security/threat-model.md`](docs/security/threat-model.md)
- [`docs/security/privacy-model.md`](docs/security/privacy-model.md)
- [`docs/security/encryption.md`](docs/security/encryption.md)
- [`docs/security/incident-response.md`](docs/security/incident-response.md)

## Disclosure principles

- Protect users before publishing exploit detail.
- Do not conceal affected versions, realistic impact, or residual risk.
- Do not claim a vulnerability is fixed until regression and adjacent-boundary tests pass on the released artifact.
- Signing, update, migration, and rollback evidence are part of remediation for package/data incidents.

## Scope reminders

The design does not guarantee protection against an attacker already running as administrator/SYSTEM, kernel compromise, same-session screen/key capture, physical access to an unlocked session, or physical secure deletion from SSDs/backups. These limits do not excuse preventable weaknesses in Pastral's own boundaries.
