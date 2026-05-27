# Ventus

Ventus is a focused desktop browser with AI built in. Built with Rust, Tao, Wry, and Microsoft Edge WebView2, it has a custom frameless browser chrome, local SQLite storage, workspaces, bookmarks, history, downloads, and an AI sidebar.

The project is early but usable. The goal is a clean, fast, practical browser that stays small enough for contributors to understand.

## Features

- Chromium-based browsing through Microsoft Edge WebView2
- Custom frameless desktop browser shell
- Tabs, tab search, pinned tabs, and workspace navigation
- Address bar search with built-in search shortcuts
- New tab experience with feed and quick-link shortcuts
- Bookmarks, history, and downloads stored locally
- Download panel with open and show-in-folder actions
- AI sidebar with OpenAI, OpenRouter, Anthropic, and Ollama provider support
- Local settings stored in SQLite
- API keys stored in the OS keychain
- Windows app icon and GUI release build

## Status

Ventus is in active development and is not meant to replace a daily driver yet. Expect missing browser features, rough edges, and platform-specific behavior while the project matures.

Current priority areas:

- Stable browsing and tab behavior
- Reliable WebView layout and click handling
- Download, history, and bookmark polish
- Safer AI sidebar workflows
- Better contributor documentation and release process

## Tech Stack

- Rust 2021
- Tao for the native window and event loop
- Wry for WebView2 integration
- Microsoft Edge WebView2 on Windows
- SQLite through `rusqlite`
- Tokio for async tasks
- Reqwest for update checks and AI provider requests
- Serde for state and settings serialization

## Requirements

- Windows 10 or Windows 11
- Rust stable
- Microsoft Edge WebView2 Runtime
- Git

Most Windows 10 and Windows 11 machines already include WebView2. If the app opens a blank window or fails to start, install the WebView2 Runtime from Microsoft.

## Getting Started

Clone the repository:

```powershell
git clone https://github.com/neura-spheres/Ventus.git
cd Ventus
```

Run the development build:

```powershell
cargo run
```

Build a release executable:

```powershell
cargo build --release
```

Run the release executable:

```powershell
.\target\release\ventus.exe
```

## Useful Commands

```powershell
cargo fmt
cargo check
cargo build
cargo build --release
```

If the debug executable is already running and blocks a rebuild:

```powershell
$p = Get-Process ventus -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq "$PWD\target\debug\ventus.exe" }
if ($p) { $p | Stop-Process -Force }
cargo build
```

## Project Structure

```text
src/
  ai/          AI provider and prompt logic
  browser/     Tabs, navigation, downloads, search engines, and workspaces
  storage/     SQLite migrations, repositories, keychain, and settings storage
  ui/          Browser chrome, events, shortcuts, assets, and theme code
  utils/       Logging, platform paths, and URL helpers

assets/        Generated app icon assets
public/        Source logo and public static assets
AGENTS.md      Architecture notes for coding agents and maintainers
Cargo.toml     Rust package configuration
config.yaml    App version source
build.rs       Windows icon build helper
```

## Architecture Notes

Ventus uses a two-layer WebView model:

- A transparent chrome WebView renders the toolbar, sidebar, settings modal, new tab page, suggestions, download panel, and AI sidebar.
- Per-tab content WebViews render normal web pages.

On Windows, the chrome WebView is clipped with Win32 regions so browser UI can sit above content while normal pages remain clickable. This is the most sensitive part of the app. Before changing layout, sidebar, overlay, settings, or web-content interaction behavior, read `AGENTS.md`.

## AI Providers

The AI sidebar supports:

- OpenAI
- OpenRouter
- Anthropic
- Ollama

API keys are entered in the app settings and stored in the OS keychain. Do not commit API keys, `.env` files, or local secrets.

## Data Storage

Local app data is stored through the platform data directory returned by the `directories` crate. The main database file is `neura.db`.

Stored locally:

- Settings
- Bookmarks
- History
- Downloads
- Search engines

Stored in the OS keychain:

- AI provider API keys

## Contributing

Contributions are welcome. Start with:

- `CONTRIBUTING.md` for setup, workflow, and pull request expectations
- `COLLABORATORS.md` for maintainer and collaborator expectations
- `CODE_OF_CONDUCT.md` for community behavior
- `SECURITY.md` for vulnerability reports
- `AGENTS.md` for architecture context

Please keep changes focused. Browser shell bugs often cross Rust, JavaScript, CSS, WebView2, and Win32 behavior, so describe what you tested in every pull request.

## License

Ventus is released under the MIT License. See `LICENSE.md`.
