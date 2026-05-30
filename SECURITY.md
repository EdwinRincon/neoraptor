# Security Policy

## Supported Versions

Neoraptor is currently in early development. Security fixes are applied to the
latest version on `master` only.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

Neoraptor is an offensive security tool. We take security bugs in the engine
itself very seriously to ensure it cannot be weaponised against unintended
targets or expose sensitive operator data.

**Please do NOT open a public GitHub issue for security vulnerabilities.**

Instead, use GitHub's private vulnerability reporting:

1. Go to the [Security tab](https://github.com/EdwinRincon/neoraptor/security)
2. Click **"Report a vulnerability"**
3. Fill in the advisory form with:
   - A clear description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested mitigations (optional)

## Response Timeline

| Step                        | Target time |
| --------------------------- | ----------- |
| Acknowledgement             | 3 days      |
| Initial assessment          | 7 days      |
| Fix or mitigation published | 30 days     |

## Scope

In-scope vulnerabilities include:

- Sandbox escape or container breakout in the execution plane
- Unauthorised access to the control plane API
- Injection attacks against the LLM provider pipeline
- Credential or secret leakage through logs or artifacts
- Supply-chain attacks via workspace dependencies

Out of scope:

- Vulnerabilities in third-party infrastructure not under this project
- Issues already tracked publicly in dependency advisories (report upstream)

## Disclosure Policy

We follow coordinated disclosure. Once a fix is available and deployed we will
publish a GitHub Security Advisory crediting the reporter (unless anonymity is
requested).
