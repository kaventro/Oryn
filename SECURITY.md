# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 1.x (latest) | Yes |

## Reporting a Vulnerability

If you discover a security vulnerability in Oryn, **please do not open a public issue**.

Instead, report it privately:

1. **GitHub Security Advisories** (preferred) — go to the [Security tab](https://github.com/kaventro/Oryn/security/advisories) and click **"Report a vulnerability"**.
2. **Email** — contact the maintainers directly at the email listed in the GitHub profile.

We will acknowledge receipt within 48 hours and aim to release a fix within 7 days for critical issues.

## Security Model

Oryn is a desktop application built with [Tauri 2](https://v2.tauri.app). Its security relies on:

- **Content Security Policy (CSP)** — enforced in both production and development builds via `tauri.conf.json`.
- **Local bundled frontend** — the webview renderer loads only local bundled assets without external scripts or stylesheets.
- **HTML escaping** — user-controlled strings (file paths, error messages) are escaped before DOM insertion.
- **Path & Archive validation** — file operations and archive extraction validate output paths, reject symlink parent traversal, and bound entry counts and extraction sizes. File writes go through a temp file + atomic rename so a destination symlink is replaced rather than written through. These checks are best-effort against a *local, single-user* threat model; they do not claim to close every filesystem-level TOCTOU race an attacker with concurrent write access to a target directory could contrive.

## Scope

The following are considered in-scope for security reports:

- Path traversal or file-system escape bugs
- IPC permission bypass
- CSP bypass or XSS via crafted filenames
- Arbitrary code execution from user-controlled input
- Credential or sensitive data leaks

Out of scope:

- Issues that require physical access to the machine
- Social engineering
- Denial of service on the local machine (the app already has full local file access by design)
