# Collaborators

This document explains how maintainers and collaborators should work on Ventus.

## Project Ownership

Ventus is owned by Ventus. The code is open source under the MIT License, while the name, logo, and product identity remain part of Ventus branding.

## Maintainer Responsibilities

Maintainers should:

- Keep the main branch buildable.
- Review changes for user-visible behavior, not only code style.
- Protect local-first privacy expectations.
- Avoid merging broad refactors without a clear user benefit.
- Keep public docs accurate when workflows change.
- Treat download, history, bookmark, WebView layout, update, and AI-provider changes as higher-risk.

## Collaborator Expectations

Collaborators should:

- Work from a branch or fork.
- Keep pull requests focused.
- Explain manual testing clearly.
- Respect existing code patterns.
- Avoid committing local app data, secrets, logs, generated binaries, or personal settings.
- Ask before changing product branding, release flow, storage schema, or security-sensitive behavior.

## Review Standards

Review should prioritize:

- Browser content remains clickable.
- Settings, sidebars, overlays, and popups do not block web content unexpectedly.
- Downloads save to the right place and can be opened or revealed.
- History and bookmarks persist and navigate correctly.
- AI features do not expose secrets or page content unexpectedly.
- Settings survive restart.
- The app still starts cleanly on Windows.

## Release Notes

When preparing a release, include:

- User-visible changes
- Bug fixes
- Known issues
- Build target
- Version number from `config.yaml`
- Link to the commit or tag

## Branding Rules

Official releases should use the Ventus name, logo, and Ventus ownership consistently. Forks should make it clear when they are not official Ventus builds.
