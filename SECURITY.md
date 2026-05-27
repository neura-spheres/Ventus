# Security Policy

Security reports are welcome. Please do not open a public issue for vulnerabilities that could put users at risk.

## Supported Versions

Ventus is currently pre-1.0. Security fixes target the current main branch unless a stable release line is announced later.

## Reporting A Vulnerability

Please report security issues privately through one of these channels:

- GitHub Security Advisories for this repository, if enabled
- A private message or email to the Ventus maintainers

Include:

- A clear description of the issue
- Steps to reproduce
- Affected operating system and app version or commit
- Any logs or screenshots that help explain the issue
- Whether the issue exposes local data, browser content, downloads, API keys, or update behavior

## Security-Sensitive Areas

Treat these areas carefully:

- API key storage and keychain access
- Downloads and file opening
- Update download and install flow
- WebView IPC between content, chrome, and Rust
- Navigation handling and custom `neura://` pages
- History, bookmarks, and local SQLite data
- AI features that may send page context to providers

## Disclosure

Please give maintainers reasonable time to investigate and release a fix before public disclosure.

