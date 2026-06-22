# Ventus Agent Context

Read this before changing the code. Ventus is a Rust desktop browser for Windows, built with Tao, Wry, WebView2, SQLite, and a custom HTML/CSS/JS chrome.

## Project Shape

Ventus is not a normal web app. It is a native Rust process that owns a frameless desktop window and several WebView2 child windows.

- `src/main.rs` is only the binary entry point. It calls `runtime::run()`.
- `src/runtime/` owns process startup, the Tao event loop, WebView creation, Win32 layout/clipping, downloads, update downloads, and dispatching `TabAction`s.
- `src/app.rs` owns `AppState`, most browser command handling, state serialization for the chrome UI, settings persistence, history/bookmark/download updates, and navigation decisions.
- `src/ui/chrome.rs` builds the chrome document and owns its Rust tests. `src/ui/chrome.html` contains the browser interface, CSS, and JavaScript compiled into the executable.
- `src/ui/events.rs` defines the IPC contract between JS and Rust with `ChromeCommand` and `AppEvent`.
- `src/browser/` contains tab, workspace, navigation, search-engine, downloads, and split-view models.
- `src/storage/` contains SQLite setup, migrations, repositories, keychain access, and settings serialization.
- `src/ai/` contains the AI provider abstraction and OpenAI, OpenRouter, Anthropic, and Ollama adapters.
- `src/updater.rs` checks GitHub releases for `neura-spheres/Ventus`, downloads a Windows `.exe`, and applies it through a temporary batch file.
- `public/ventus.png` is the logo source used in the chrome and app icon generation.
- `build.rs` creates `assets/logo.ico` from the PNG and attaches it as the Windows resource icon.

Some local checkouts may include `RULES.md` with stricter project style notes. If it is present, read it before coding. Otherwise, follow the conventions in this file and the surrounding source.

## Runtime Source Map

The runtime fragments use `include!` from `src/runtime/mod.rs`. This keeps them in one Rust module scope, so the refactor does not change private visibility or event-loop captures.

| File | Owns |
| --- | --- |
| `src/runtime/mod.rs` | Runtime setup, shared imports/constants, window and WebContext creation |
| `src/runtime/event_loop.rs` | Tao event callback and `TabAction` execution |
| `src/runtime/startup.rs` | Single-instance startup, relaunch, profile locks, shutdown, process failure |
| `src/runtime/permissions.rs` | Live WebView2 permission policy and permission callbacks |
| `src/runtime/downloads.rs` | WebView2 download lifecycle and accelerated downloads |
| `src/runtime/native_events.rs` | Navigation, popup windows, new-window handling, keyboard accelerators |
| `src/runtime/webviews.rs` | Content WebView creation, browser arguments, settings load, secure DNS setup |
| `src/runtime/scripts.rs` | Content and privacy initialization scripts injected into pages |
| `src/runtime/feed.rs` | Trends and feed refresh helpers |
| `src/runtime/layout.rs` | `AppLayout`, clipping, z-order, content bounds, load watches, cookie snapshots |
| `src/runtime/services.rs` | Session saves, cloud sync, notification icons, stale-event guards |
| `src/runtime/assistant.rs` | AI agent execution, account auth, profile work, AI streaming |
| `src/runtime/search_answers.rs` | Currency, market, Wikipedia, and instant-answer helpers |
| `src/runtime/system.rs` | Memory limits, image saving, and clipboard integration |

## WebView Architecture

The app uses a layered WebView model.

The chrome WebView is created first in `src/runtime/mod.rs` with `WebViewBuilder::new_as_child(&window)`, `with_transparent(true)`, and `with_html(chrome_html())`. It covers the whole window. It renders the toolbar, address bar, sidebar, settings modal, onboarding modal, new-tab page, suggestions, download panel, AI sidebar, and window controls.

Content WebViews are created per tab by `build_content_webview()`. They use a shared `wry::WebContext` rooted under the app data directory so cookies, local storage, and cache persist across tabs and restarts. Normal web pages get content WebViews. `neura://` pages are chrome-rendered and should not keep an active content WebView for that tab.

On Windows, the chrome WebView is clipped with `SetWindowRgn` so only UI regions receive pointer events. The content WebViews sit behind or above the chrome depending on state:

- Settings, onboarding, suggestions that need ownership, and `neura://` pages require chrome to own the relevant content area.
- Normal web pages must remain interactable. Content HWNDs are raised above chrome with `sync_content_z_order()` when chrome should not own content.
- `AppLayout::calculate()` is the single source of truth for toolbar height, sidebar width, AI width, content bounds, and the chrome clip region.
- `SyncViews` means content bounds may change. `SyncClipOnly` means only the chrome clip region changes.

This layering is fragile. When fixing black content, unclickable pages, sidebar peek issues, or broken overlays, check both the CSS state in `src/ui/chrome.html` and the Win32 clipping and z-order logic in `src/runtime/layout.rs`.

## Runtime Flow

Startup flow:

1. `main()` calls `runtime::run()`, which initializes logging and Tokio.
2. The app data directory comes from `utils::platform::data_dir()`, using `ProjectDirs::from("com", "neura", "NeuraBrowser")`.
3. SQLite opens at `neura.db`; migrations run; built-in search engines are seeded.
4. `AppSettings` loads from the `app_settings` row in the `settings` table.
5. A frameless Tao window is created.
6. The chrome WebView is created from `chrome_html()`.
7. `AppState::new()` creates a default workspace and a `neura://newtab` tab.
8. A content WebView is created only if the initial active URL is not `neura://`.
9. `apply_layout()` syncs chrome bounds, chrome clip, content bounds, and HWND ordering.
10. When the chrome WebView finishes loading, `ChromeReady` pushes the serialized app state into JS.

Most user actions are:

1. JS calls `send("PascalCaseCommand", data)`.
2. `send()` converts the command to `snake_case`.
3. Rust deserializes it as `ChromeCommand` using `#[serde(tag = "cmd", rename_all = "snake_case")]`.
4. `handle_app_event_inner()` calls `handle_chrome_command()`.
5. The handler mutates `AppState`, persists data when needed, pushes state to JS, and may return a `TabAction`.
6. `src/runtime/event_loop.rs` executes the `TabAction` because the runtime owns the actual WebViews and window.

## IPC Contract

Do not bypass the established IPC path.

From JS:

```js
send('Navigate', {url})
```

This posts:

```json
{"cmd":"navigate","url":"..."}
```

New UI commands should be added to `ChromeCommand` in `src/ui/events.rs` and handled in `src/app.rs`. Keep direct WebView operations in `src/runtime/` by returning a `TabAction`; do not make `app.rs` own WebView lifecycle work.

Rust-to-JS calls go through the `window.__neura` object in `src/ui/chrome.html`. If Rust needs a new UI update method, add it there and call it with `chrome.evaluate_script(...)`.

## UI State

`AppState::chrome_state_json()` is the main state payload for JS. It includes active tabs, workspaces, active URL/title, nav state, settings, search engines, bookmarks, recent history, and downloads.

`window.__neura.setState(s)` merges state into the JS `state` object and calls `render()`. `render()` updates workspaces, tabs, address bar, search settings, bookmarks, downloads, new-tab shortcuts, theme, sidebar mode, and settings fields.

Important UI areas in `src/ui/chrome.html`:

- Address bar and omnibox suggestions use `#url-input`, `#url-suggestions`, `GetHistory`, `SuggestionOverlay`, and `syncSuggestionOverlay()`.
- New tab page is rendered by chrome through `#newtab-placeholder`, not by a content WebView.
- Bookmarks/history/downloads list rows use data attributes and `handleDelegatedListClick()`. Keep this pattern so action buttons remain clickable without inline JSON escaping problems.
- Settings modal uses `OpenSettings` and `CloseSettings` to tell Rust whether chrome should own the content area.
- The download mini-panel also expands the chrome clip with `SuggestionOverlay` so it can receive clicks above web content.

## Navigation

Navigation is resolved in `app.rs`:

- `navigate_current_tab()` clears suggestion clipping, resolves the input, marks the active tab loading, starts the progress bar, pushes state, and returns `TabAction::ContentNavigate`.
- `resolve_navigation_url()` checks search shortcuts first, then delegates to `browser::navigation::resolve_input()`.
- `resolve_input()` handles empty input, `neura://`, full HTTP/HTTPS/file URLs, dotted domains, localhost addresses, and search queries.
- Built-in search engines live in `browser::search_engine::SearchEngine::builtin_engines()` and are seeded into SQLite.

Content navigation events come back from `src/runtime/native_events.rs` and the content initialization script in `src/runtime/scripts.rs`. Metadata and favicon updates come from `content_initialization_script()`.

History stores cleaned URLs through `utils::url::clean_tracking_url()`. Do not save `neura://` pages into history.

## Persistence

SQLite schema is declared in `src/storage/migrations.rs`.

Important tables:

- `settings`
- `workspaces`
- `tabs`
- `bookmarks`
- `bookmark_folders`
- `history`
- `downloads`
- `search_engines`
- `ai_chat_sessions`
- `ai_chat_messages`
- `ai_providers`
- `keyboard_shortcuts`

Repositories are in `src/storage/repositories.rs`. Settings are generic JSON values in `settings_store.rs`; the full `AppSettings` struct is also persisted under `app_settings`. When changing settings, preserve that full-struct persistence so changes survive restart.

API keys are not stored in SQLite. They use the OS keychain via `src/storage/keychain.rs` with service name `ventus`.

Downloads are loaded on startup with `repositories::list_downloads(&conn, 100)`. Starting and completing a download both save to SQLite. If a completed download already has `local_path`, do not overwrite it with an empty path.

## Downloads

Downloads are attached to content WebViews from `src/runtime/downloads.rs`. Content WebViews are built in `src/runtime/webviews.rs`.

- `with_download_started_handler()` chooses the final download path.
- `download_dir_from_settings()` uses `settings.downloads.default_folder` when set; otherwise it falls back to the OS Downloads directory.
- `unique_download_path()` avoids overwriting existing files.
- `DownloadStarted` creates a download record with `local_path`.
- `DownloadCompleted` updates status and completion time.
- The UI can open files with `OpenFile` and reveal them with `RevealFile`.

On Windows, file reveal is sensitive to quoting. Explorer expects `/select,"C:\path\file.ext"`. Be careful with spaces, commas, and Rust argument escaping.

## Layout And Clickability

Most severe UI bugs in this app come from the relationship between CSS, Win32 regions, and WebView HWND z-order.

Rules for layout work:

- Only `AppLayout::calculate()` should compute browser geometry.
- Use `apply_layout()` after changes that alter content bounds.
- Use `SyncClipOnly` for transient overlay/peek changes that should not move web content.
- Keep chrome transparent unless a specific UI surface must paint.
- For normal pages, web content must be able to receive pointer events.
- For settings, onboarding, new-tab, suggestions, and floating panels, chrome must own the area that needs clicks.
- When a floating chrome panel overlaps web content, send `SuggestionOverlay` with its bounds so the clip region includes it.
- Avoid calling Win32 region and child-window APIs outside `set_chrome_clip_region()`, `move_child_window()`, `bring_hwnd_to_top()`, and the existing HWND discovery helpers.

Auto-hide sidebar behavior depends on both JS and content IPC. The content initialization script sends `sidebar_auto_close` on mouse movement because `WM_MOUSELEAVE` is not reliable across clipped sibling HWNDs.

## AI Sidebar

AI provider selection is configured through `AppSettings.ai`.

- `ai::build_provider()` reads the selected provider and looks up keys from the keychain.
- OpenAI and OpenRouter share `OpenAiProvider`.
- Anthropic uses `/v1/messages`.
- Ollama targets `http://localhost:11434`.
- Chat requests are sent from `handle_ai_message()` in `src/runtime/assistant.rs` so async streaming can use Tokio and send `AppEvent::AiChunk` back to the UI.

The current page context is basic title and URL text. The richer page-text extraction flow is only partially represented by events and prompt helpers.

## Branding

The app is named Ventus. The about/settings branding uses the local logo data URL from `ui::assets::logo_data_url()`.

## Build And Verification

Common commands:

```powershell
cargo fmt
cargo check
cargo build
cargo build --release
```

For changes inside the embedded JS in `src/ui/chrome.html`, validate the script syntax:

```powershell
@'
const fs = require('fs');
const s = fs.readFileSync('src/ui/chrome.html','utf8');
const m = s.match(/<script>([\s\S]*)<\/script>/);
if (!m) throw new Error('script block not found');
new Function(m[1]);
console.log('chrome inline script syntax OK');
'@ | node -
```

If `cargo build` cannot write the executable because the app is running, stop only this checkout's debug executable:

```powershell
$p = Get-Process ventus -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq 'C:\Projects\NeuraSearch\target\debug\ventus.exe' }
if ($p) { $p | Stop-Process -Force }
cargo build
```

Current builds may emit warnings from unused or partially wired modules. Treat new warnings in touched code as actionable, but do not start broad warning cleanup unless asked.

## Practical Checklist For Future Changes

Before editing:

- Read `RULES.md` if it exists in your checkout.
- Check `git status --short`; do not revert user changes.
- For UI bugs, inspect `src/ui/chrome.html`, `src/runtime/event_loop.rs`, and `src/runtime/layout.rs`.
- For persistence bugs, inspect `src/app.rs`, `src/storage/repositories.rs`, and the relevant settings path.

After editing:

- Run `cargo fmt` for Rust changes.
- Run the Node inline-script syntax check after `chrome.html` JS changes.
- Run `cargo check` for code changes.
- Run `cargo build` when touching startup, WebView creation, Windows APIs, downloads, or update behavior.
- Manually reason through whether chrome owns content, content owns clicks, or a floating panel needs `SuggestionOverlay`.
