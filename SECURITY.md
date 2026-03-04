# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.3.x   | Yes       |
| < 0.3   | No        |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Send a report to **security@aether-lang.dev** with:

- A description of the vulnerability and its potential impact
- Steps to reproduce (minimal `.ae` / `.as` program if applicable)
- Any suggested mitigations or patches

You will receive an acknowledgment within 48 hours and a status update within 7 days.

## Scope

The following are in-scope for security reports:

- **Code injection** via guest language sandboxing bypass (Python/JS/Shell execution)
- **Safety level bypass** — executing L3/L4 nodes without the required approval level
- **Path traversal** in `fs_read` / `fs_write` intent execution
- **Denial of service** via malformed `.ae` input to the parser or executor
- **Sensitive data exposure** in `output.ae.json` execution logs

The following are out-of-scope:

- Vulnerabilities in the user's own Anthropic API key handling (bring-your-own-key model)
- Issues in guest language runtimes themselves (Python, Node.js) — report those upstream
- Rate limiting or abuse prevention on the Community REST API (`aether-api`)

## Disclosure Policy

We follow coordinated disclosure. Once a fix is ready, we will:
1. Publish a patched release
2. Credit the reporter in the release notes (unless anonymity is requested)
3. File a CVE if applicable

## Security Model Notes

Aether's safety tier system (L0–L4) is a **sandboxing policy**, not a security boundary against a malicious `.ae` author. It is designed to prevent accidental side effects in AI-generated programs, not to sandbox untrusted user code. Do not run untrusted `.ae` files from unknown sources with elevated safety levels.
