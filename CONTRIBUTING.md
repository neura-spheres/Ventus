# Contributing

Thanks for helping improve Ventus. This project is a desktop browser, so small UI changes can affect Rust, JavaScript, CSS, WebView2, and Windows window behavior at the same time. Keep changes focused and describe what you tested.

## Before You Start

- Read `README.md` for setup and project status.
- Read `AGENTS.md` for architecture context.
- Read `RULES.md` if it is present in your checkout.
- Check existing issues or discussions before starting larger work.

## Development Setup

Install:

- Rust stable
- Git
- Microsoft Edge WebView2 Runtime
- Windows 10 or Windows 11

Run:

```powershell
cargo run
```

Check:

```powershell
cargo fmt
cargo check
cargo build
```

## Code Style

- Keep changes small and direct.
- Use existing patterns before adding new abstractions.
- Rust uses `snake_case`; JavaScript uses `camelCase`; CSS uses `kebab-case`.
- Add new JS-to-Rust commands in `src/ui/events.rs` and handle them in `src/app.rs`.
- Keep WebView lifecycle and native window work in `src/main.rs`.
- Do not commit secrets, API keys, local database files, downloaded files, logs, or build outputs.

## Browser UI Work

The browser UI is embedded in `src/ui/chrome.rs`. If you edit JavaScript there, run:

```powershell
@'
const fs = require('fs');
const s = fs.readFileSync('src/ui/chrome.rs','utf8');
const m = s.match(/<script>([\s\S]*)<\/script>/);
if (!m) throw new Error('script block not found');
new Function(m[1]);
console.log('chrome inline script syntax OK');
'@ | node -
```

Layout, overlay, and clickability bugs often involve `src/main.rs` too. Check the Win32 clip and z-order logic before assuming the issue is only CSS.

## Pull Request Checklist

Before opening a pull request:

- Run `cargo fmt`.
- Run `cargo check`.
- Run `cargo build` for changes touching startup, WebViews, downloads, updates, or Windows APIs.
- Run the inline JS syntax check after `src/ui/chrome.rs` changes.
- Test the user-facing flow manually.
- Update docs when behavior, setup, or contributor workflow changes.

In the pull request description, include:

- What changed
- Why it changed
- How you tested it
- Screenshots or short clips for visual UI changes
- Any known limitations

## Issue Labels

Suggested labels for maintainers:

- `bug`
- `feature`
- `docs`
- `good first issue`
- `help wanted`
- `windows`
- `webview`
- `ai`
- `storage`
- `ui`

