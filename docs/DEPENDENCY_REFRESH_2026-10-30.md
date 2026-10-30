# Dependency refresh: 2026-10-30

This update refreshes compatible Cargo dependencies, upgrades document and networking libraries, and replaces the bundled adblocker with the official uBlock Origin Lite release. It does not migrate Ventus's native window and WebView bindings to new major API generations.

## Updated components

| Component | Previous | Updated |
| --- | --- | --- |
| Installed Rust stable | 1.93.1 | 1.99.1 |
| uBlock Origin Lite | 2026.820.1159 | 2026.1026.1812 |
| rusqlite | 0.31.0 | 0.41.1 |
| reqwest | 0.12.28 | 0.13.5 |
| thiserror (direct dependency) | 1.0.69 | 2.0.21 |
| directories | 5.0.1 | 6.0.0 |
| rfd | 0.14.1 | 0.17.2 |
| roxmltree | 0.20.0 | 0.21.2 |
| pdf-extract | 0.10.0 | 0.12.1 |
| lopdf | 0.38.0 | 0.42.0 |
| zip | 2.4.2 | 9.1.3 |
| Wry's cfg_aliases build helper | 0.1.1 | 0.2.2 |

`Cargo.lock` also contains compatible transitive updates. Reqwest explicitly retains native TLS and enables the form/query features needed by account authentication, Gemini, and instant answers.

## Adblocker provenance and startup

- Official release: <https://github.com/uBlockOrigin/uBOL-home/releases/tag/2026.1026.1812>.
- Asset: `uBOLite_2026.1026.1812.chromium.zip`.
- SHA-256: `d3788eac9f0a44c67f3745a43c3fd446ef33d56b1121f469b19e0fa0a836c2b5`.
- All 928 bundled files match the downloaded release byte for byte. Existing upstream whitespace is retained.
- All bundled JSON parses, and the manifest's 104 referenced resources exist.
- Runtime extension files now refresh after profile cleanup and before the first WebView2 environment starts. This prevents loading the previous extension before its files are updated.
- Regression coverage checks rule replacement, removal of obsolete files, unreadable manifests, and domain-boundary matching. The bundled-list smoke test no longer assumes an upstream list will permanently block a particular real website.

## Security audit and remaining work

The current official RustSec database reports zero security vulnerabilities in the updated lockfile. The PDF update addresses RUSTSEC-2026-0187; the ZIP update addresses RUSTSEC-2026-0367, RUSTSEC-2026-0368, and RUSTSEC-2026-0369.

There are eight unmaintained-package warnings and two soundness warnings across the cross-platform dependency graph. `cargo tree --target x86_64-pc-windows-msvc` confirms that the flagged `glib` and `rand 0.7.3` soundness dependencies are absent from the Windows build. The Windows graph still contains unmaintained `instant 0.1.13` through Tao and `ttf-parser 0.25.1` through the current PDF library.

Wry 0.38.2, Tao 0.28.1, the patched WebView2 bindings, Windows API aliases, keyring, and cryptographic API generations remain on their existing compatible lines. Upgrading those requires preserving Ventus's shared-environment, extension, cookie, navigation, and Windows handle behavior. The browser engine is the independently serviced installed WebView2 Runtime, not the Wry crate version.

The installed WebView2 Runtime is 156.0.4314.50. Automatic approval review blocked the Microsoft installer command with the reason "blocked by policy", so this session did not apply a system runtime update.

## Verification

- `cargo fmt --check`, `cargo check --locked`, `cargo test --locked`, and `cargo build --locked` passed on Rust 1.99.1. All 142 unit tests passed.
- Embedded chrome JavaScript syntax and the manifest/resource/JSON checks passed.
- The live WebView2 service worker reported uBOL version `2026.1026.1812` and all six default rulesets enabled.
- A real image request to an advertising host was rejected with Chromium's `net::ERR_BLOCKED_BY_CLIENT`. Using a real resource also avoids mistaking a content blocker's JavaScript fetch shim for a network result.
- HTTPS content rendered, the address bar existed, and settings opened and closed through the established JS/Rust IPC path.
- Ventus was restarted with `--restore-session` after verification to remove the temporary local debugging endpoint.
- Native click/z-order behavior, every website, downloads against external servers, authenticated cloud sync, and paid AI-provider calls were not exhaustively tested. A successful build and these checks are not a guarantee of perfect operation.

No deployment or release publication is part of this update. The app version remains unchanged.
