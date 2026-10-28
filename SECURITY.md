# Security

## Supported scope

Security fixes are applied to the current `main` branch and the most recent release. Both surfaces are in scope: the desktop app (`nook`) and the CLI (`nook-cli`). Older releases are not patched retroactively; update to current `main`.

## Reporting a vulnerability

Nook is pointed at databases people care about. Credential handling (OS keychain on macOS, `secret-tool` on Linux, profile file on Windows) and the SQL surface (every identifier quoted, every literal escaped, every row count bounded) are the two areas where a bug has real consequences.

If you find a vulnerability, **report it privately** through [GitHub Security Advisories](https://github.com/kasfulk/nook/security/advisories/new) ("Report a vulnerability" on the repository page). Do not open a public issue for a security bug.

## What to include

- The affected component (driver, engine, UI, MCP, control channel, keychain backend).
- Steps to reproduce, or a proof-of-concept script.
- The Nook version or `main` commit where you observed it.
- What you expected versus what happened.

## Disclosure

Do not publish the vulnerability — in a blog post, on social media, in a talk — before a fix is available. Coordinate the disclosure timeline through the advisory. If you are unsure whether something counts, ask first through the advisory.

## Response

The maintainer will acknowledge the report within a few days and work with you on a fix. Response time is best-effort; Nook is a small project. If the report is valid, a fix lands on `main`, a release is cut, and the advisory is published with credit to the reporter unless they ask to remain anonymous.

## What is not in scope

- Vulnerabilities in third-party dependencies that Nook does not control. These should be reported upstream; Dependabot tracks updates.
- Issues that require physical access to an unlocked machine.
- Misuse of the tool against databases the operator does not own or have permission to access.
