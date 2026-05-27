# Ventus — Claude Context File

> **Read `RULES.md` before writing any code.** It defines naming, comments, wording, structure, and IPC conventions for this project. All code must match that style.

Rust desktop browser. Frameless window, Zen Browser-inspired UI, AI sidebar.
Stack: WRY 0.38.2, TAO 0.28, WebView2 (Windows), SQLite (rusqlite), Tokio.

---

## Architecture: Two-Layer WebView System

The most critical architectural concept. Two types of WebViews co-exist as Win32 child HWNDs:

**Chrome WebView** — full-window overlay (0,0,win_w,win_h), always on top via `SetWindowPos(HWND_TOP)`.
- Contains all UI: toolbar, sidebar, AI sidebar, settings modal, new-tab page, overlays.
- `SetWindowRgn` clips it to only the UI regions (toolbar strip + sidebar column + optional AI panel).
- Outside the clip region: mouse events and painting fall through to the content WebViews below.

**Content WebViews** — one per tab, positioned below chrome in Z-order.
- Rect is `(content_x, toolbar_h, content_w, content_h)`.
- In auto-hide-pinned mode: `content_x = sidebar_w`. In all other modes: `content_x = 0`.
- Managed by `build_content_webview()` in `main.rs`.

**Key constraint**: `WM_MOUSELEAVE` never fires when cursor moves from chrome clip region into content area (content is a sibling HWND, not the chrome's client). The content WebView's `mousemove` IPC is used instead to signal Rust to close the sidebar.

---

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Event loop, WebView creation, layout engine, Win32 clip/Z management |
| `src/app.rs` | `AppState`, all `ChromeCommand` handlers, `TabAction` dispatch |
| `src/ui/chrome.rs` | Entire browser UI as one HTML/CSS/JS string (`chrome_html()`) |
| `src/ui/events.rs` | `ChromeCommand` enum (JS→Rust IPC), `AppEvent` enum (Rust→JS+internal) |
| `src/updater.rs` | GitHub release checker + .bat-based self-update |
| `src/browser/tab_manager.rs` | Tab + workspace state |
| `src/config.rs` | `AppSettings`, `SidebarMode` enum |
| `src/storage/` | SQLite: history, bookmarks, downloads, settings, search engines |

---

## Layout Engine (`main.rs`)

`AppLayout::calculate()` computes all pixel values from `AppState` + `LayoutConfig`.

```
LayoutConfig:
  sidebar_expanded_w = 240
  sidebar_collapsed_w = 52
  toolbar_h = 44
  ai_sidebar_w = 340
  min_content_w = 200
```

Sidebar width rules:
- `is_auto_hide && sidebar_pinned` → `min(240, win_w - 200)` (solid, pushes content)
- `is_auto_hide && !sidebar_pinned` → `0` (overlay only, content full width)
- `!is_auto_hide && collapsed` → `52`
- `!is_auto_hide && !collapsed` → `min(240, win_w - 200)`

`clip_sidebar_w` (for `SetWindowRgn`) = `sidebar_w` in auto-hide, else `0` (sidebar is already an overlay in non-auto-hide, clip only needs toolbar).

Wait — actually re-check: in non-auto-hide mode `#sidebar` is `position:fixed` with `transform:translateX(0)`. The chrome clip needs to include the sidebar. So `clip_sidebar_w = sidebar_w` for non-auto-hide too. Verify in `AppLayout::calculate` before assuming.

---

## AppState Fields (src/app.rs)

```rust
pub struct AppState {
    pub tab_manager: TabManager,
    pub downloads: DownloadManager,
    pub settings: AppSettings,           // persisted in SQLite
    pub conn: Connection,
    pub ai_messages: Vec<ChatMessage>,
    pub current_ai_provider: String,
    pub sidebar_collapsed: bool,
    pub ai_sidebar_open: bool,
    pub chrome_overlay_open: bool,       // true = settings/onboarding open, clip = full window
    pub sidebar_auto_hide_open: bool,    // true = sidebar peeking (auto-hide mode)
    pub sidebar_pinned: bool,            // true = sidebar click-pinned (solid, content pushed)
    pub suggestion_overlay_rect: Option<ChromeClipRect>,
    pub pending_update_url: Option<String>,
}
```

---

## TabAction Flow

`handle_app_event_inner()` in `app.rs` returns `Option<TabAction>`.
Main event loop dispatches:

- `SyncViews` → `apply_layout()` full (content WebView bounds + chrome clip)
- `SyncClipOnly` → only updates chrome clip region, content WebViews untouched
- `Create { tab_id, url }` → `build_content_webview()` + `apply_layout()`
- `Remove(id)` → drop content WebView from HashMap + `apply_layout()`
- `ContentNavigate(url)` → call `navigate()` on active content WebView
- `ContentScript(js)` → `evaluate_script()` on active content WebView

---

## Chrome Clip Region (`set_chrome_clip_region` in main.rs)

```rust
fn set_chrome_clip_region(hwnd, window_w, window_h, sidebar_w, toolbar_h, ai_sidebar_w, ai_open, overlay_open)
```

- `overlay_open=true` → `SetWindowRgn(full_window, bRedraw=true)` — settings/onboarding open
- `overlay_open=false` → toolbar rect (full width) OR-combined with sidebar rect, optionally AI panel rect

**Known issue (OPEN)**: When `overlay_open=true`, `bRedraw=true` forces chrome repaint → dark `body{background:var(--bg)}` covers content WebView momentarily before settings overlay renders. Fix: change `bRedraw` to `false` for overlay case, OR add `.with_transparent(true)` to chrome WebViewBuilder + make `body{background:transparent}`.

---

## Sidebar Auto-Hide System

**JS side** (`chrome.rs`):
- `sidebarPeeking` / `sidebarPinned` — module-level state vars
- `showFloatingSidebar()` → adds `.sidebar-floating-open` class → CSS transition slides sidebar in → sends `SidebarPeek{visible:true, pinned: sidebarPinned}`
- `_doHideSidebar()` → removes class → 220ms delay → sends `SidebarPeek{visible:false, pinned:false}`
- Click on toggle btn while peeking → upgrades to pinned: `sidebarPinned=true` → sends `SidebarPeek{visible:true, pinned:true}`
- `window.__neura.closeSidebar()` — called by Rust when content WebView detects mousemove; respects `sidebarPinned`

**Rust side** (`app.rs`):
- `SidebarPeek{visible, pinned}` → updates `sidebar_auto_hide_open` + `sidebar_pinned`; if pinned state changed → `SyncViews`, else → `SyncClipOnly`
- `SidebarAutoClose` → calls `closeSidebar()` JS if `sidebar_auto_hide_open && !sidebar_pinned`

**Content WebView** (`content_initialization_script`):
- Throttled `mousemove` at 100ms sends `{"cmd":"sidebar_auto_close"}` to Rust

---

## IPC Protocol

**JS → Rust**: `window.ipc.postMessage(JSON.stringify({cmd: "snake_case_cmd", ...fields}))`

The `send()` helper auto-converts PascalCase → snake_case:
```js
function send(cmd, data={}) {
  const sc = cmd.replace(/([A-Z])/g, (m, c, i) => (i > 0 ? '_' : '') + c.toLowerCase());
  window.ipc.postMessage(JSON.stringify({cmd: sc, ...data}));
}
```

Serde deserializes via `#[serde(tag = "cmd", rename_all = "snake_case")]` on `ChromeCommand`.

**Rust → JS**: `chrome.evaluate_script("window.__neura && window.__neura.METHOD(args)")` or `push_state_to_chrome()`.

`push_state_to_chrome()` → `chrome_state_json()` → `window.__neura.setState(json)` → `render()`.

---

## chrome.rs JS Interface (`window.__neura`)

Key methods called by Rust:
- `setState(s)` — merges into `state`, calls `render()`
- `setLayout(sidebarW, toolbarH, aiW)` — updates CSS vars
- `appendAiChunk(text, done)` — streaming AI response
- `setUrl(url, title)` — updates address bar + newtab check
- `setHistory(items)` — updates `state.history` + `renderHistory()`
- `setBookmarked(v)` — bookmark icon state
- `updateNavState(back, fwd, loading)` — nav button state
- `closeSidebar()` — auto-close sidebar if unpinned
- `setUpdateState({status, version, notes, error, received, total})` — update UI
- `showOnboarding()` — opens onboarding modal
- `showError(msg)` / `showSuccess(msg)` — toasts

---

## New Tab Page

The new tab page (`neura://newtab` or `about:blank`) is rendered in the CHROME WebView via `#newtab-placeholder` div (NOT a separate WebView). `checkNewtabPlaceholder(url)` shows/hides it.

`#newtab-shortcuts` div exists but **is not populated** — `populateNewtabShortcuts()` function was NEVER implemented. This is a known open bug.

---

## Open Bugs / Pending Work

### 1. New Tab Shortcuts (NOT IMPLEMENTED)
`#newtab-shortcuts` div in `chrome.rs` HTML is always empty. Need to add `populateNewtabShortcuts()` that:
- Uses `state.bookmarks` if populated, else hardcoded defaults (Google, YouTube, GitHub, Reddit, etc.)
- Renders `.newtab-shortcut` divs with colored letter-icon squares
- Called from `checkNewtabPlaceholder()` when showing the new tab

### 2. Address Bar Omnibox (NOT IMPLEMENTED)
`#url-input` only handles Enter (navigate) and Escape (blur). No suggestion dropdown.
Need:
- `#url-suggestions` fixed-position dropdown (add to `<body>`)
- CSS: `.url-suggestion`, `.url-suggestion.highlighted`
- On focus: `send('GetHistory', {q: ''})` to prefetch recent history
- `oninput="filterOmnibox(this.value)"` handler
- `showOmniboxSuggestions(q)` — filters `state.history`, adds "Search X for..." option
- Keyboard: ArrowUp/Down navigate, Enter selects, Escape closes
- `onmousedown` on suggestions with `e.preventDefault()` (prevents blur before selection)
- Update `__neura.setHistory` to also refresh open omnibox
- `handleUrlKey()` needs ArrowUp/Down handling before current Enter/Escape logic

### 3. Content Becomes Black on Settings Open / Sidebar Peek
Root cause: `SetWindowRgn(..., bRedraw=true)` forces chrome repaint → opaque `body{background:var(--bg)}` covers content WebView.

Two-part fix (NOT YET APPLIED):
1. Change `bRedraw` from `true` to `false` in the `overlay_open=true` branch of `set_chrome_clip_region`
2. (Optional deeper fix) Add `.with_transparent(true)` to chrome `WebViewBuilder::new_as_child` + change `body{background: transparent}` in chrome CSS. WRY 0.38 supports `with_transparent` via `put_DefaultBackgroundColor`.

---

## Updater (src/updater.rs)

```rust
const GITHUB_OWNER: &str = "neura-spheres";
const GITHUB_REPO: &str = "Ventus";
pub const CURRENT_VERSION: &str = crate::version::APP_VERSION;
```

- `check_latest()` → GitHub API `/releases/latest` → semver compare → finds `.exe` asset
- `download_update(url, on_progress)` → streams to `%TEMP%\ventus-update.exe`
- `apply_update(new_exe)` → writes `%TEMP%\neura-update.bat` → cmd /c (CREATE_NO_WINDOW) → process::exit(0)

The bat script loops `tasklist` until the old PID exits, then `copy /y`, then `start`.

---

## Settings Persistence

Settings saved via `SaveSettings{key, value}` command. Key handlers in `app.rs`:
- `"sidebar_mode"` → `state.sidebar_pinned = false` + `SyncViews`
- `"theme"` → `SyncViews`
- Various others → `None` (JS only)

`AppSettings` serialized to/from JSON in SQLite via `settings_store::set/get`.

---

## Fixed Bugs (Previous Sessions)

1. **Black screen on startup**: `neura://` custom protocol failed silently. Fixed: use `with_html(new_tab_html())` for neura pages.
2. **Sidebar empty**: `state_json()` used camelCase keys; JS expects snake_case. Fixed all keys.
3. **Initial state timing**: `evaluate_script` ran before chrome page loaded. Fixed: `with_initialization_script` + `__neura_pending_state` pattern.
4. **Nav events not tracked**: Content WebViews had no page load handlers. Fixed: added `with_on_page_load_handler`.
5. **New tab IPC cmd case**: `new_tab.rs` sent `Navigate` (PascalCase), serde needs `navigate`. Fixed.
6. **`RevealFile` broken**: `.args(["/select,", &path])` passes two args. Fixed: `.arg(format!("/select,{}", path))`.
7. **`OpenFile` broken**: Fixed: `cmd /c start "" {path}`.
8. **Sidebar stuck open**: `WM_MOUSELEAVE` never fires at clip boundary. Fixed: content WebView mousemove IPC → `SidebarAutoClose` command.
9. **Sidebar pin → content resize**: Added `sidebar_pinned` to Rust state. `SidebarPeek` carries `pinned` field. `AppLayout::calculate` adjusts `content_x` and `content_w` when pinned.

---

## Build

```powershell
cargo build           # dev
cargo build --release # release (lto=true, strip=true, opt-level=3)
```

Binary: `target/release/ventus.exe`
Version: `config.yaml`
Data dir: `%APPDATA%\ventus\` (via `directories` crate)
