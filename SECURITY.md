# Security Policy

## Supported versions

Security fixes are applied to the default branch (`main`). Pin releases and module CI to a known-good commit or tag when possible.

## Reporting a vulnerability

Please **do not** file a public GitHub issue for security vulnerabilities.

Email **security@syntax-labs.fr** with:

- A short description of the issue
- Steps to reproduce or a proof of concept
- Affected crate / CLI command if known
- Your preferred contact method for follow-up

We aim to acknowledge reports within **5 business days**.

## Scope (examples)

In scope:

- Remote code execution or sandbox escape via Wasm host functions
- Privilege escalation through capability checks
- Credential leakage in CLI publish / registry auth flows
- Supply-chain issues in published crates or OCI artifacts produced by this repo

Out of scope:

- Denial of service that requires already-compromised host privileges
- Issues in third-party dependencies with no realistic Portaki-specific impact (report upstream when appropriate)
- Social engineering or physical attacks

## Prefer responsible disclosure

Give us a reasonable window to ship a fix before public disclosure. We will coordinate a timeline with you when needed.
