use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;
use wry::WebView;

use crate::adblock::AdBlockEngine;
use crate::browser::downloads::DownloadManager;
use crate::browser::tab_manager::TabManager;
use crate::config::AppSettings;
use crate::storage::{keychain, repositories, settings_store};
use crate::ui::events::{AppEvent, ChromeCommand};

#[derive(Debug, Clone, Copy)]
pub struct ChromeClipRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct AppState {
    pub tab_manager: TabManager,
    pub downloads: DownloadManager,
    pub settings: AppSettings,
    pub conn: Connection,
    pub ai_messages: Vec<crate::ai::ChatMessage>,
    pub current_ai_provider: String,
    pub sidebar_collapsed: bool,
    pub ai_sidebar_open: bool,
    pub chrome_overlay_open: bool,
    pub sidebar_auto_hide_open: bool,
    pub sidebar_pinned: bool,
    pub suggestion_overlay_rect: Option<ChromeClipRect>,
    pub pending_update_url: Option<String>,
    pub pending_update_version: Option<String>,
    pub content_fullscreen: bool,
    pub pending_nav_urls: HashMap<String, String>,
    pub load_recoveries: HashMap<String, u8>,
    pub spotlight_open: bool,
    pub zoom_levels: HashMap<String, f64>,
    /// Pending AI page-tool channels — keyed by call_id.
    /// Agent loop inserts a sender; main loop resolves it when the IPC result arrives.
    pub ai_pending_tools: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>,
    pub ad_block_engine: AdBlockEngine,
    /// Element kill count reported by the init script for the currently active page.
    pub adblock_page_kills: u32,
    /// Cached DB data — refreshed only when the underlying data changes.
    /// Avoids 3 SQLite queries on every push_state_to_chrome call.
    pub cached_search_engines: Vec<crate::browser::search_engine::SearchEngine>,
    pub cached_bookmarks: Vec<repositories::Bookmark>,
    pub cached_history: Vec<repositories::HistoryEntry>,
}

impl AppState {
    pub fn new(conn: Connection, settings: AppSettings) -> Self {
        let ai_provider = settings.ai.default_provider.clone();
        let ad_block_enabled = settings.privacy.ad_blocker_enabled;
        let ad_block_exceptions = settings.privacy.ad_blocker_exceptions.clone();
        let mut downloads = repositories::list_downloads(&conn, 100).unwrap_or_default();
        downloads.reverse();
        let cached_search_engines = repositories::list_search_engines(&conn).unwrap_or_default();
        let cached_bookmarks = repositories::list_bookmarks(&conn).unwrap_or_default();
        let cached_history = repositories::list_history(&conn, 30).unwrap_or_default();
        Self {
            tab_manager: TabManager::new(),
            downloads: DownloadManager::with_downloads(downloads),
            settings,
            conn,
            ai_messages: Vec::new(),
            current_ai_provider: ai_provider,
            sidebar_collapsed: false,
            ai_sidebar_open: false,
            chrome_overlay_open: false,
            sidebar_auto_hide_open: false,
            sidebar_pinned: false,
            suggestion_overlay_rect: None,
            pending_update_url: None,
            pending_update_version: None,
            content_fullscreen: false,
            pending_nav_urls: HashMap::new(),
            load_recoveries: HashMap::new(),
            spotlight_open: false,
            zoom_levels: HashMap::new(),
            ai_pending_tools: Arc::new(Mutex::new(HashMap::new())),
            ad_block_engine: AdBlockEngine::new(ad_block_enabled, &ad_block_exceptions),
            adblock_page_kills: 0,
            cached_search_engines,
            cached_bookmarks,
            cached_history,
        }
    }

    pub fn push_state_to_chrome(&self, chrome: &WebView) {
        let json = self.chrome_state_json();
        let _ = chrome.evaluate_script(&format!(
            "window.__neura && window.__neura.setState({})",
            json
        ));
    }

    pub fn chrome_state_json(&self) -> String {
        let active_tab = self.tab_manager.active_tab();
        let workspace_tab_counts = self.tab_manager.workspace_tab_counts();
        let active_url = active_tab.map(|t| t.url.as_str()).unwrap_or("");
        let is_incognito = self
            .tab_manager
            .active_workspace()
            .map(|w| w.is_incognito)
            .unwrap_or(false);
        let ad_blocker_active = self.settings.privacy.ad_blocker_enabled && !is_incognito;
        let ad_blocker_site_excepted =
            !active_url.is_empty() && self.ad_block_engine.is_site_excepted(active_url);
        serde_json::to_string(&serde_json::json!({
            "tabs": self.tab_manager.active_workspace_tabs(),
            "workspaces": &self.tab_manager.workspaces,
            "workspace_tab_counts": workspace_tab_counts,
            "active_tab_id": self.tab_manager.active_tab_id,
            "active_workspace_id": self.tab_manager.active_workspace_id,
            "active_url": active_url,
            "active_title": active_tab.map(|t| t.title.as_str()).unwrap_or(""),
            "can_go_back": active_tab.map(|t| t.can_go_back).unwrap_or(false),
            "can_go_fwd": active_tab.map(|t| t.can_go_forward).unwrap_or(false),
            "is_loading": active_tab.map(|t| t.status == crate::browser::tab::TabStatus::Loading).unwrap_or(false),
            "settings": &self.settings,
            "search_engines": &self.cached_search_engines,
            "sidebar_collapsed": self.sidebar_collapsed,
            "ai_open": self.ai_sidebar_open,
            "bookmarks": &self.cached_bookmarks,
            "history": &self.cached_history,
            "downloads": &self.downloads.downloads,
            "ai_key_status": {
                "anthropic": keychain::has_api_key("anthropic"),
                "openai": keychain::has_api_key("openai"),
                "gemini": keychain::has_api_key("gemini"),
                "openrouter": keychain::has_api_key("openrouter"),
            },
            "ad_blocker_active": ad_blocker_active,
            "ad_blocker_site_excepted": ad_blocker_site_excepted,
            "ad_blocker_kills": self.adblock_page_kills,
        }))
        .unwrap_or_default()
    }
}

pub enum TabAction {
    Create {
        tab_id: String,
        url: String,
    },
    Remove(String),
    RemoveMany(Vec<String>),
    SyncViews,
    /// Only update the chrome clip region — do not touch content WebView bounds.
    /// Used for auto-hide sidebar peek so the content page is completely undisturbed.
    SyncClipOnly,
    ContentScript(String),
    SetZoom(f64),
    SetZoomFor {
        tab_id: String,
        level: f64,
    },
    SetZoomAll(f64),
    ContentNavigate(String),
    ReloadContent {
        tab_id: String,
        url: String,
    },
    ActivateContent {
        tab_id: String,
        url: String,
        loading: bool,
    },
    /// Destroy the active tab's WebView and recreate it at the same URL so the
    /// updated initialization script (e.g. new ad-block exceptions) takes effect.
    RebuildContent {
        tab_id: String,
        url: String,
    },
    DownloadUpdate(String),
    SetFullscreen(bool),
    ContentScriptOnTab {
        tab_id: String,
        js: String,
    },
}

pub fn handle_chrome_command(
    cmd: ChromeCommand,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    match cmd {
        ChromeCommand::Navigate { url } => navigate_current_tab(url, state, chrome),
        ChromeCommand::NavigateFromOverlay { url } => {
            state.chrome_overlay_open = false;
            state.suggestion_overlay_rect = None;
            navigate_current_tab(url, state, chrome).or(Some(TabAction::SyncViews))
        }
        ChromeCommand::Back => {
            if let Some(tab_id) = state.tab_manager.active_tab_id.clone() {
                if let Some(url) = state.tab_manager.go_back(&tab_id) {
                    clear_transient_chrome(state, chrome);
                    if finish_internal_nav(&tab_id, &url, state, chrome) {
                        return Some(TabAction::ContentNavigate(url));
                    }
                    state.pending_nav_urls.insert(tab_id, url.clone());
                    state.push_state_to_chrome(chrome);
                    return Some(TabAction::ContentNavigate(url));
                }
            }
            Some(TabAction::ContentScript("history.back()".into()))
        }
        ChromeCommand::Forward => {
            if let Some(tab_id) = state.tab_manager.active_tab_id.clone() {
                if let Some(url) = state.tab_manager.go_forward(&tab_id) {
                    clear_transient_chrome(state, chrome);
                    if finish_internal_nav(&tab_id, &url, state, chrome) {
                        return Some(TabAction::ContentNavigate(url));
                    }
                    state.pending_nav_urls.insert(tab_id, url.clone());
                    state.push_state_to_chrome(chrome);
                    return Some(TabAction::ContentNavigate(url));
                }
            }
            Some(TabAction::ContentScript("history.forward()".into()))
        }
        ChromeCommand::Reload => reload_current_tab(state, chrome),
        ChromeCommand::Stop => Some(TabAction::ContentScript("window.stop()".into())),
        ChromeCommand::NewTab => {
            let tab_id;
            let url;
            {
                let tab = state.tab_manager.new_tab(None);
                tab_id = tab.id.clone();
                url = tab.url.clone();
            }
            state.push_state_to_chrome(chrome);
            Some(TabAction::Create { tab_id, url })
        }
        ChromeCommand::CloseTab { id } => {
            state.tab_manager.close_tab(&id);
            state.push_state_to_chrome(chrome);
            Some(TabAction::Remove(id))
        }
        ChromeCommand::SwitchTab { id } => {
            if !state.tab_manager.switch_tab(&id) {
                return None;
            }

            if let Some(tab) = state.tab_manager.tabs.iter().find(|t| t.id == id) {
                let url = serde_json::to_string(&tab.url).unwrap_or_default();
                let title = serde_json::to_string(&tab.title).unwrap_or_default();
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setUrl({}, {})",
                    url, title
                ));
            }

            state.push_state_to_chrome(chrome);
            let Some(tab) = state.tab_manager.get_tab(&id) else {
                return Some(TabAction::SyncViews);
            };
            if tab.is_neura_page() {
                return Some(TabAction::SyncViews);
            }
            Some(TabAction::ActivateContent {
                tab_id: id,
                url: tab.url.clone(),
                loading: tab.status == crate::browser::tab::TabStatus::Loading,
            })
        }
        ChromeCommand::PinTab { id } => {
            let tab = state.tab_manager.get_tab(&id);
            let is_pinned = tab.map(|t| t.pinned).unwrap_or(false);
            state.tab_manager.pin_tab(&id, !is_pinned);
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::UnpinTab { id } => {
            state.tab_manager.pin_tab(&id, false);
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::NewWorkspace {
            name,
            is_incognito,
            icon,
            accent_color,
        } => {
            let id = {
                let ws = state.tab_manager.add_workspace(name, is_incognito, icon);
                if let Some(color) = clean_workspace_color(accent_color) {
                    ws.accent_color = color;
                }
                ws.id.clone()
            };
            state.tab_manager.switch_workspace(&id);
            state.push_state_to_chrome(chrome);
            Some(TabAction::SyncViews)
        }
        ChromeCommand::SwitchWorkspace { id } => {
            state.tab_manager.switch_workspace(&id);
            state.push_state_to_chrome(chrome);
            Some(TabAction::SyncViews)
        }
        ChromeCommand::DeleteWorkspace { id } => {
            let tab_ids: Vec<String> = state
                .tab_manager
                .tabs
                .iter()
                .filter(|tab| tab.workspace_id == id)
                .map(|tab| tab.id.clone())
                .collect();
            if !state.tab_manager.delete_workspace(&id) {
                return None;
            }
            state.push_state_to_chrome(chrome);
            Some(TabAction::RemoveMany(tab_ids))
        }
        ChromeCommand::RenameWorkspace {
            id,
            name,
            icon,
            accent_color,
        } => {
            if let Some(ws) = state.tab_manager.workspaces.iter_mut().find(|w| w.id == id) {
                ws.name = name;
                if let Some(ico) = icon {
                    ws.icon = ico;
                }
                if let Some(color) = clean_workspace_color(accent_color) {
                    ws.accent_color = color;
                }
            }
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::ToggleAiSidebar => {
            state.ai_sidebar_open = !state.ai_sidebar_open;
            state.push_state_to_chrome(chrome);
            Some(TabAction::SyncViews)
        }
        ChromeCommand::AiProviderChange { provider } => {
            state.current_ai_provider = provider.clone();
            state.settings.ai.default_provider = provider.clone();
            if should_swap_ai_model(&state.settings.ai.default_model) {
                state.settings.ai.default_model = ai_model_for(&provider).to_string();
            }
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::AiModelChange { model } => {
            state.settings.ai.default_model = model;
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            None
        }
        ChromeCommand::AiClearChat => {
            state.ai_messages.clear();
            None
        }
        ChromeCommand::BookmarkAdd => {
            if let Some(tab) = state.tab_manager.active_tab() {
                let url = tab.url.clone();
                let title = tab.title.clone();
                match repositories::add_bookmark(&state.conn, &url, &title, None) {
                    Ok(_) => {
                        state.cached_bookmarks =
                            repositories::list_bookmarks(&state.conn).unwrap_or_default();
                        let _ = chrome.evaluate_script(
                            "window.__neura && window.__neura.setBookmarked(true)",
                        );
                        let _ = chrome.evaluate_script(
                            "window.__neura && window.__neura.showSuccess('Bookmark saved')",
                        );
                        state.push_state_to_chrome(chrome);
                    }
                    Err(e) => tracing::warn!("Bookmark add failed: {}", e),
                }
            }
            None
        }
        ChromeCommand::BookmarkRemove { url } => {
            let _ = repositories::remove_bookmark_by_url(&state.conn, &url);
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            let _ = chrome.evaluate_script("window.__neura && window.__neura.setBookmarked(false)");
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::HistoryClear => {
            let _ = repositories::clear_history(&state.conn);
            state.cached_history.clear();
            let _ = chrome
                .evaluate_script("window.__neura && window.__neura.showSuccess('History cleared')");
            let _ = chrome.evaluate_script("window.__neura && window.__neura.setHistory([])");
            None
        }
        ChromeCommand::BrowseDownloadFolder => {
            if let Some(path) = pick_download_folder(&state.settings.downloads.default_folder) {
                handle_save_settings(
                    "download_path".to_string(),
                    serde_json::Value::String(path),
                    state,
                    chrome,
                );
                state.push_state_to_chrome(chrome);
            }
            None
        }
        ChromeCommand::SaveSettings { key, value } => {
            let should_close_overlay = key == "onboarding_done";
            let needs_layout = key == "sidebar_mode" || key == "show_bookmarks_bar";
            let ad_blocker_toggled = key == "ad_blocker_enabled";
            handle_save_settings(key, value, state, chrome);
            if should_close_overlay {
                state.chrome_overlay_open = false;
                state.push_state_to_chrome(chrome);
                return Some(TabAction::SyncViews);
            }
            // Push state first so JS classes (e.g. show-bookmarks-bar) update immediately,
            // then return SyncViews so layout/clip region also updates.
            state.push_state_to_chrome(chrome);
            if needs_layout {
                return Some(TabAction::SyncViews);
            }
            if ad_blocker_toggled {
                return None;
            }
            None
        }
        ChromeCommand::TabAudioState { tab_id, playing } => {
            if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.is_audio_playing = playing;
                if !playing {
                    tab.is_muted = false;
                }
            }
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::MuteTab { tab_id } => {
            let muted =
                if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
                    tab.is_muted = !tab.is_muted;
                    tab.is_muted
                } else {
                    return None;
                };
            state.push_state_to_chrome(chrome);
            let js = format!(
                "document.querySelectorAll('audio,video').forEach(function(m){{m.muted={}}});",
                if muted { "true" } else { "false" }
            );
            Some(TabAction::ContentScriptOnTab { tab_id, js })
        }
        ChromeCommand::AdBlockToggleSite => {
            let url = state
                .tab_manager
                .active_tab()
                .map(|t| t.url.clone())
                .unwrap_or_default();
            // Don't toggle for internal pages or when globally disabled.
            if url.starts_with("neura://")
                || url.starts_with("about:")
                || !state.settings.privacy.ad_blocker_enabled
            {
                return None;
            }
            let tab_id = state.tab_manager.active_tab_id.clone().unwrap_or_default();
            state.ad_block_engine.toggle_exception(&url);
            state.settings.privacy.ad_blocker_exceptions =
                state.ad_block_engine.exceptions().to_vec();
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            state.push_state_to_chrome(chrome);
            Some(TabAction::RebuildContent { tab_id, url })
        }
        ChromeCommand::AdBlockStats { killed } => {
            state.adblock_page_kills = killed;
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::OpenSettings => {
            state.chrome_overlay_open = true;
            Some(TabAction::SyncViews)
        }
        ChromeCommand::FocusAddressBar => {
            let _ = chrome.evaluate_script("focusUrl()");
            None
        }
        ChromeCommand::OpenTabSearch => {
            state.chrome_overlay_open = true;
            let _ = chrome.evaluate_script("openTabSearch(true)");
            Some(TabAction::SyncViews)
        }
        ChromeCommand::CloseSettings => {
            state.chrome_overlay_open = false;
            Some(TabAction::SyncViews)
        }
        ChromeCommand::SidebarToggle => {
            state.sidebar_collapsed = !state.sidebar_collapsed;
            Some(TabAction::SyncViews)
        }
        ChromeCommand::ReopenTab => {
            if let Some(url) = state.tab_manager.reopen_closed_tab() {
                let tab_id;
                {
                    let tab = state.tab_manager.new_tab(Some(&url));
                    tab_id = tab.id.clone();
                }
                state.push_state_to_chrome(chrome);
                return Some(TabAction::Create { tab_id, url });
            }
            None
        }
        ChromeCommand::ExportSettings => {
            if let Ok(json) = serde_json::to_string_pretty(&state.settings) {
                if let Some(download_dir) = directories::UserDirs::new()
                    .and_then(|d| d.download_dir().map(|p| p.to_path_buf()))
                {
                    let path = download_dir.join("neura-settings.json");
                    if std::fs::write(&path, &json).is_ok() {
                        let _ = chrome.evaluate_script(
                            "window.__neura && window.__neura.showSuccess('Settings exported to Downloads')"
                        );
                    }
                }
            }
            None
        }
        // OpenDevtools is handled directly in main.rs (calls wv.open_devtools() on
        // the active content WebView). Nothing to do here at the app-state level.
        ChromeCommand::OpenDevtools => None,
        ChromeCommand::GetHistory { q } => {
            // Always use frecency ranking for omnibox — most-visited sites surface first.
            let results =
                repositories::search_history_frecency(&state.conn, &q, 30).unwrap_or_default();
            let json = serde_json::to_string(&results).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setHistory({})",
                json
            ));
            None
        }
        ChromeCommand::DeleteHistoryEntry { id } => {
            let _ = repositories::delete_history_entry(&state.conn, id);
            let results = repositories::list_history(&state.conn, 100).unwrap_or_default();
            let json = serde_json::to_string(&results).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setHistory({})",
                json
            ));
            None
        }
        ChromeCommand::OpenFile { path } => {
            #[cfg(windows)]
            {
                // Strip the Zone.Identifier ADS so Windows doesn't show the "trusted source"
                // security prompt — same operation as the "Unblock" checkbox in file Properties.
                let _ = std::fs::remove_file(format!("{}:Zone.Identifier", path));
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "", &path])
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&path).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
            None
        }
        ChromeCommand::RevealFile { path } => {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let target = std::path::PathBuf::from(&path);
                let path_to_open = if target.exists() {
                    path.replace('/', "\\")
                } else if let Some(parent) = target.parent().filter(|p| p.exists()) {
                    parent.to_string_lossy().replace('/', "\\")
                } else {
                    path.replace('/', "\\")
                };
                // Use raw_arg so the /select, prefix is not quoted by Rust's arg escaper.
                // Explorer expects: explorer.exe /select,"C:\path\to\file" (path quoted, not the whole arg).
                let _ = std::process::Command::new("explorer.exe")
                    .raw_arg(format!("/select,\"{}\"", path_to_open))
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open")
                .args(["-R", &path])
                .spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("nautilus").arg(&path).spawn();
            None
        }
        ChromeCommand::SidebarPeek { visible, pinned } => {
            let was_pinned = state.sidebar_pinned;
            state.sidebar_auto_hide_open = visible;
            state.sidebar_pinned = visible && pinned;
            // Pin state change affects content width — need full layout sync
            if state.sidebar_pinned != was_pinned {
                Some(TabAction::SyncViews)
            } else {
                Some(TabAction::SyncClipOnly)
            }
        }
        ChromeCommand::SidebarAutoClose => {
            // Skip when pinned — pinned sidebar is solid and stays open
            if state.sidebar_auto_hide_open && !state.sidebar_pinned {
                let _ = chrome.evaluate_script(
                    "window.__neura&&window.__neura.closeSidebar&&window.__neura.closeSidebar()",
                );
            }
            None
        }
        ChromeCommand::SuggestionOverlay {
            visible,
            x,
            y,
            width,
            height,
        } => {
            state.suggestion_overlay_rect = if visible && width > 0.0 && height > 0.0 {
                Some(ChromeClipRect {
                    x,
                    y,
                    width,
                    height,
                })
            } else {
                None
            };
            Some(TabAction::SyncClipOnly)
        }
        ChromeCommand::CheckForUpdate => {
            let _ = chrome.evaluate_script(
                "window.__neura && window.__neura.setUpdateState({status:'checking'})",
            );
            None
        }
        ChromeCommand::LoadNeuraFeed => None,
        ChromeCommand::OpenInNewTab { url } => {
            let resolved = resolve_navigation_url(&url, state);
            let tab_id;
            {
                let tab = state.tab_manager.new_tab(Some(&resolved));
                tab_id = tab.id.clone();
            }
            state.push_state_to_chrome(chrome);
            Some(TabAction::Create {
                tab_id,
                url: resolved,
            })
        }
        ChromeCommand::ContextMenuSaveImage { url } => {
            let url_js = serde_json::to_string(&url).unwrap_or_default();
            Some(TabAction::ContentScript(format!(
                "(() => {{ const a = document.createElement('a'); a.href = {}; a.download = ''; \
                 document.body.appendChild(a); a.click(); setTimeout(() => {{ if (a.parentNode) a.parentNode.removeChild(a); }}, 100); }})()",
                url_js
            )))
        }
        ChromeCommand::ClearDownloads => {
            let _ = repositories::clear_downloads(&state.conn);
            state.downloads.downloads.clear();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::DeleteDownload { id } => {
            let _ = repositories::delete_download(&state.conn, &id);
            state.downloads.downloads.retain(|d| d.id != id);
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::InstallUpdate => {
            if let Some(url) = state.pending_update_url.clone() {
                let _ = chrome.evaluate_script(
                    "window.__neura && window.__neura.setUpdateState({status:'downloading',received:0,total:0})",
                );
                // Return a sentinel so main.rs can spawn the download with its proxy reference
                Some(TabAction::DownloadUpdate(url))
            } else {
                None
            }
        }
        ChromeCommand::ZoomSet { level } => {
            let clamped = level.clamp(0.25, 3.0);
            if let Some(id) = state.tab_manager.active_tab_id.clone() {
                set_tab_zoom(state, &id, clamped);
            }
            Some(TabAction::SetZoom(clamped))
        }
        ChromeCommand::ZoomDelta { delta } => {
            let Some(id) = state.tab_manager.active_tab_id.clone() else {
                return None;
            };
            let cur = tab_zoom(state, &id);
            let next = ((cur + delta).clamp(0.25, 3.0) * 10.0).round() / 10.0;
            set_tab_zoom(state, &id, next);
            let _ = chrome.evaluate_script(&format!("showZoomToast({})", (next * 100.0).round()));
            Some(TabAction::SetZoom(next))
        }
        ChromeCommand::ZoomGlobal { level } => {
            let clamped = (level.clamp(0.5, 1.5) * 10.0).round() / 10.0;
            state.settings.appearance.zoom_level = clamped;
            state.zoom_levels.clear();
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            Some(TabAction::SetZoomAll(clamped))
        }
        ChromeCommand::ToggleFullscreen => {
            let new_state = !state.content_fullscreen;
            state.content_fullscreen = new_state;
            Some(TabAction::SetFullscreen(new_state))
        }
        ChromeCommand::ContentFullscreenChange { active } => {
            if state.content_fullscreen == active {
                return None;
            }
            state.content_fullscreen = active;
            Some(TabAction::SetFullscreen(active))
        }
        ChromeCommand::OpenInNewWindow { url } => {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .arg("--new-window")
                    .arg("--url")
                    .arg(&url)
                    .spawn();
            }
            None
        }
        ChromeCommand::DismissUpdate { version } => {
            let _ = settings_store::set(&state.conn, "dismissed_update_version", &version);
            None
        }
        ChromeCommand::BeginSpotlight => {
            if state.spotlight_open {
                state.spotlight_open = false;
                let _ = chrome.evaluate_script("hideSpotlight()");
            } else {
                state.spotlight_open = true;
                let _ = chrome.evaluate_script("spotlightOpen=false;showSpotlight()");
            }
            Some(TabAction::SyncViews)
        }
        ChromeCommand::EndSpotlight => {
            state.spotlight_open = false;
            let _ = chrome.evaluate_script("hideSpotlight()");
            Some(TabAction::SyncViews)
        }
        ChromeCommand::OpenHistoryPanel => {
            let _ = chrome.evaluate_script("openSettings('history')");
            None
        }
        ChromeCommand::OpenDownloadsPanel => {
            let _ = chrome.evaluate_script("openSettings('downloads')");
            None
        }
        _ => None,
    }
}

fn clear_transient_chrome(state: &mut AppState, chrome: &WebView) {
    state.chrome_overlay_open = false;
    state.spotlight_open = false;
    state.suggestion_overlay_rect = None;
    let _ = chrome.evaluate_script(
        "window.__neura&&window.__neura.clearTransientUi&&window.__neura.clearTransientUi()",
    );
}

fn clean_workspace_color(color: Option<String>) -> Option<String> {
    let color = color?;
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return None;
    }
    if !color[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(color.to_ascii_lowercase())
}

fn finish_internal_nav(tab_id: &str, url: &str, state: &mut AppState, chrome: &WebView) -> bool {
    if !url.starts_with("neura://") {
        return false;
    }
    state.pending_nav_urls.remove(tab_id);
    state.load_recoveries.remove(&load_key(tab_id, url));
    state.tab_manager.set_tab_loading(tab_id, false);
    let _ = chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
    state.push_state_to_chrome(chrome);
    true
}

fn navigate_current_tab(url: String, state: &mut AppState, chrome: &WebView) -> Option<TabAction> {
    if let Some(tab_id) = state.tab_manager.active_tab_id.clone() {
        clear_transient_chrome(state, chrome);
        let resolved_url = resolve_navigation_url(&url, state);
        state.tab_manager.visit_tab(&tab_id, &resolved_url, "");
        if finish_internal_nav(&tab_id, &resolved_url, state, chrome) {
            return Some(TabAction::ContentNavigate(resolved_url));
        }
        state.tab_manager.set_tab_loading(&tab_id, true);
        state
            .pending_nav_urls
            .insert(tab_id.clone(), resolved_url.clone());
        let _ = chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
        state.push_state_to_chrome(chrome);
        Some(TabAction::ContentNavigate(resolved_url))
    } else {
        None
    }
}

fn reload_current_tab(state: &mut AppState, chrome: &WebView) -> Option<TabAction> {
    let tab_id = state.tab_manager.active_tab_id.clone()?;
    let url = state.tab_manager.get_tab(&tab_id)?.url.clone();
    if url.starts_with("neura://") || url.trim().is_empty() {
        return None;
    }
    clear_transient_chrome(state, chrome);
    state.pending_nav_urls.insert(tab_id.clone(), url.clone());
    state.tab_manager.set_tab_loading(&tab_id, true);
    state.load_recoveries.remove(&load_key(&tab_id, &url));
    let _ = chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
    state.push_state_to_chrome(chrome);
    Some(TabAction::ReloadContent { tab_id, url })
}

fn set_tab_zoom(state: &mut AppState, id: &str, level: f64) {
    let rounded = (level * 10.0).round() / 10.0;
    if (rounded - state.settings.appearance.zoom_level).abs() < f64::EPSILON {
        state.zoom_levels.remove(id);
    } else {
        state.zoom_levels.insert(id.to_string(), rounded);
    }
}

pub fn tab_zoom(state: &AppState, id: &str) -> f64 {
    state
        .zoom_levels
        .get(id)
        .copied()
        .unwrap_or(state.settings.appearance.zoom_level)
}

fn favicon_for_url(url: &str, favicon: Option<String>) -> Option<String> {
    if let Some(icon) = favicon
        .map(|v| v.trim().to_string())
        .filter(|v| v.starts_with("http://") || v.starts_with("https://"))
    {
        return Some(icon);
    }
    crate::utils::url::extract_favicon_url(url)
}

enum PendingNav {
    None,
    Match,
    Stale,
}

fn take_pending(state: &mut AppState, tab_id: &str, url: &str) -> PendingNav {
    let Some(expected) = state.pending_nav_urls.get(tab_id) else {
        return PendingNav::None;
    };
    if !same_nav(expected, url) {
        return PendingNav::Stale;
    }
    state.pending_nav_urls.remove(tab_id);
    PendingNav::Match
}

fn same_nav(expected: &str, actual: &str) -> bool {
    let expected = crate::utils::url::clean_tracking_url(expected);
    if expected == actual {
        return true;
    }
    let Ok(a) = url::Url::parse(&expected) else {
        return false;
    };
    let Ok(b) = url::Url::parse(actual) else {
        return false;
    };
    host_key(a.host_str()) == host_key(b.host_str())
        && a.path() == b.path()
        && a.query() == b.query()
}

fn host_key(host: Option<&str>) -> String {
    host.unwrap_or_default()
        .trim_start_matches("www.")
        .to_lowercase()
}

fn can_accept_redirect(state: &AppState, tab_id: &str, url: &str) -> bool {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }
    state
        .tab_manager
        .get_tab(tab_id)
        .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
        .unwrap_or(false)
}

pub fn handle_app_event_inner(
    event: AppEvent,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    match event {
        AppEvent::Chrome(cmd) => handle_chrome_command(cmd, state, chrome),
        AppEvent::ContentNav { tab_id, url, title } => {
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                state.adblock_page_kills = 0;
            }
            let was_loading = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
                .unwrap_or(false);
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            match take_pending(state, &tab_id, &clean_url) {
                PendingNav::Match => {
                    state
                        .tab_manager
                        .replace_tab_nav(&tab_id, &clean_url, &title);
                }
                PendingNav::None => {
                    state.tab_manager.visit_tab(&tab_id, &clean_url, &title);
                }
                PendingNav::Stale => {
                    if !can_accept_redirect(state, &tab_id, &clean_url) {
                        return None;
                    }
                    state.pending_nav_urls.remove(&tab_id);
                    state
                        .tab_manager
                        .replace_tab_nav(&tab_id, &clean_url, &title);
                }
            }
            if was_loading {
                state.tab_manager.set_tab_loading(&tab_id, true);
            }
            let is_active = state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str());
            if is_active {
                let is_bm = repositories::is_bookmarked(&state.conn, &clean_url).unwrap_or(false);
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setBookmarked({})",
                    is_bm
                ));
            }
            let in_incognito = state.tab_manager.tab_is_incognito(&tab_id);
            if !state.settings.privacy.disable_history
                && !clean_url.starts_with("neura://")
                && !in_incognito
            {
                let _ = repositories::add_history(&state.conn, &clean_url, &title, None);
                // Prepend to cached history and keep at most 30 entries.
                state.cached_history.insert(
                    0,
                    repositories::HistoryEntry {
                        id: 0,
                        url: clean_url.clone(),
                        title: title.clone(),
                        workspace_id: None,
                        visited_at: chrono::Utc::now().timestamp_millis(),
                    },
                );
                state.cached_history.truncate(30);
            }
            if is_active {
                let url_js = serde_json::to_string(&clean_url).unwrap_or_default();
                let title_js = serde_json::to_string(&title).unwrap_or_default();
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setUrl({}, {})",
                    url_js, title_js
                ));
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::ContentLoadStart { tab_id, url } => {
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            if !clean_url.trim().is_empty()
                && clean_url != "about:blank"
                && !clean_url.starts_with("neura://")
            {
                let cur = state
                    .tab_manager
                    .get_tab(&tab_id)
                    .map(|tab| tab.url.clone())
                    .unwrap_or_default();
                if cur.is_empty() || !same_nav(&cur, &clean_url) {
                    state
                        .pending_nav_urls
                        .insert(tab_id.clone(), clean_url.clone());
                    state.tab_manager.visit_tab(&tab_id, &clean_url, "");
                }
            }
            let was_loading = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
                .unwrap_or(false);
            state.tab_manager.set_tab_loading(&tab_id, true);
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let active = state
                    .tab_manager
                    .active_tab()
                    .map(|t| (t.can_go_back, t.can_go_forward));
                let (back, fwd) = active.unwrap_or((false, false));
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.updateNavState({},{},true)",
                    back, fwd
                ));
            }
            if !was_loading {
                let _ =
                    chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::ContentLoadEnd { tab_id, url } => {
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            if let Some(expected) = state.pending_nav_urls.get(&tab_id) {
                if !same_nav(expected, &clean_url) {
                    if !can_accept_redirect(state, &tab_id, &clean_url) {
                        return None;
                    }
                    state.pending_nav_urls.remove(&tab_id);
                    state
                        .tab_manager
                        .replace_tab_nav(&tab_id, &clean_url, &clean_url);
                }
            }
            if state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| !same_nav(&tab.url, &clean_url))
                .unwrap_or(true)
            {
                return None;
            }
            if let Some(tab) = state.tab_manager.get_tab(&tab_id) {
                state.load_recoveries.remove(&load_key(&tab_id, &tab.url));
            }
            state.tab_manager.set_tab_loading(&tab_id, false);
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let active = state
                    .tab_manager
                    .active_tab()
                    .map(|t| (t.can_go_back, t.can_go_forward));
                if let Some((back, fwd)) = active {
                    let _ = chrome.evaluate_script(&format!(
                        "window.__neura && window.__neura.updateNavState({},{},false)",
                        back, fwd
                    ));
                }
                let _ =
                    chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
            }
            state.push_state_to_chrome(chrome);
            Some(TabAction::SetZoomFor {
                tab_id: tab_id.clone(),
                level: tab_zoom(state, &tab_id),
            })
        }
        AppEvent::ContentLoadProgress {
            tab_id,
            url,
            progress,
        } => {
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setLoadProgress({:.3})",
                    progress.clamp(0.0, 1.0)
                ));
            }
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            let done = progress >= 0.92
                && !clean_url.trim().is_empty()
                && clean_url != "about:blank"
                && state
                    .tab_manager
                    .get_tab(&tab_id)
                    .map(|tab| {
                        tab.status == crate::browser::tab::TabStatus::Loading
                            && same_nav(&tab.url, &clean_url)
                    })
                    .unwrap_or(false);
            if done {
                state.pending_nav_urls.remove(&tab_id);
                state.load_recoveries.remove(&load_key(&tab_id, &clean_url));
                state.tab_manager.set_tab_loading(&tab_id, false);
                if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                    let active = state
                        .tab_manager
                        .active_tab()
                        .map(|t| (t.can_go_back, t.can_go_forward));
                    if let Some((back, fwd)) = active {
                        let _ = chrome.evaluate_script(&format!(
                            "window.__neura && window.__neura.updateNavState({},{},false)",
                            back, fwd
                        ));
                    }
                    let _ = chrome
                        .evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
                }
                state.push_state_to_chrome(chrome);
            }
            None
        }
        AppEvent::ContentLoadStalled { tab_id, url, .. } => {
            let Some(tab) = state.tab_manager.get_tab(&tab_id) else {
                return None;
            };
            if tab.status != crate::browser::tab::TabStatus::Loading {
                return None;
            }
            // Skip stalls on internal or empty URLs — they can never be stuck in a
            // meaningful way and any watch for them is a spurious leftover.
            if url.starts_with("neura://") || url.trim().is_empty() {
                return None;
            }
            // If the current tab URL differs from the stall URL it means a redirect
            // occurred while the 30-second timer was in flight: `ContentMetadata`
            // updated `tab.url` to the final destination before the timer fired.
            // Previously we returned None here, leaving the spinner running forever.
            // Instead, recover against the *current* URL so we reload the real page
            // rather than the pre-redirect URL (which would just loop back again).
            let recover_url = if !tab.url.is_empty() && tab.url != url {
                tab.url.clone()
            } else {
                url.clone()
            };
            let key = load_key(&tab_id, &recover_url);
            let tries = state.load_recoveries.entry(key).or_insert(0);
            if *tries < 2 {
                *tries += 1;
                return Some(TabAction::ReloadContent {
                    tab_id,
                    url: recover_url,
                });
            }
            // After 2 reload attempts, stop and let the user decide.
            // RebuildContent is intentionally not used here: it destroys the WebView2
            // instance, which loses server-side sessions and in-memory auth tokens even
            // though cookies persist — causing unexpected logouts.
            state
                .load_recoveries
                .remove(&load_key(&tab_id, &recover_url));
            state.tab_manager.set_tab_loading(&tab_id, false);
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let active = state
                    .tab_manager
                    .active_tab()
                    .map(|t| (t.can_go_back, t.can_go_forward));
                if let Some((back, fwd)) = active {
                    let _ = chrome.evaluate_script(&format!(
                        "window.__neura && window.__neura.updateNavState({},{},false)",
                        back, fwd
                    ));
                }
                let _ =
                    chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::ContentMetadata {
            tab_id,
            url,
            title,
            favicon,
            replace,
        } => {
            let was_loading = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
                .unwrap_or(false);
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            let safe_title = if title.trim().is_empty() {
                clean_url.clone()
            } else {
                title
            };
            let pending = match take_pending(state, &tab_id, &clean_url) {
                PendingNav::Match => true,
                PendingNav::None => false,
                PendingNav::Stale => {
                    if !can_accept_redirect(state, &tab_id, &clean_url) {
                        return None;
                    }
                    state.pending_nav_urls.remove(&tab_id);
                    true
                }
            };
            let record_replace = replace
                && state
                    .tab_manager
                    .get_tab(&tab_id)
                    .map(|tab| should_record_replace_as_visit(&tab.url, &clean_url))
                    .unwrap_or(false);
            if pending || replace && !record_replace {
                state
                    .tab_manager
                    .replace_tab_nav(&tab_id, &clean_url, &safe_title);
            } else {
                state
                    .tab_manager
                    .visit_tab(&tab_id, &clean_url, &safe_title);
            }
            let icon = favicon_for_url(&clean_url, favicon);
            if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.favicon = icon;
            }
            if was_loading {
                state.tab_manager.set_tab_loading(&tab_id, true);
            }
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let is_bm = repositories::is_bookmarked(&state.conn, &clean_url).unwrap_or(false);
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setBookmarked({})",
                    is_bm
                ));
                let url_js = serde_json::to_string(&clean_url).unwrap_or_default();
                let title_js = serde_json::to_string(&safe_title).unwrap_or_default();
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setUrl({}, {})",
                    url_js, title_js
                ));
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::AiChunk { text, done } => {
            let text_js = serde_json::to_string(&text).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.appendAiChunk({}, {})",
                text_js, done
            ));
            None
        }
        AppEvent::AiError { message } => {
            let msg_js = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.showError({})",
                msg_js
            ));
            // Mark AI as done so the UI stops showing the loading indicator
            let _ =
                chrome.evaluate_script("window.__neura && window.__neura.appendAiChunk('', true)");
            None
        }
        AppEvent::AiToolCallDisplay { label } => {
            let label_js = serde_json::to_string(&label).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.appendAiToolCall({})",
                label_js
            ));
            None
        }
        AppEvent::AiSaveMessages {
            user_text,
            assistant_text,
        } => {
            state
                .ai_messages
                .push(crate::ai::ChatMessage::user(user_text));
            state
                .ai_messages
                .push(crate::ai::ChatMessage::assistant(assistant_text));
            None
        }
        AppEvent::AiToolResult { call_id, result } => {
            if let Ok(mut map) = state.ai_pending_tools.lock() {
                if let Some(tx) = map.remove(&call_id) {
                    let _ = tx.send(result);
                }
            }
            None
        }
        // AiExecutePageJs is handled in main.rs (needs content_views access)
        AppEvent::AiExecutePageJs { .. } => None,
        AppEvent::SpotlightAiChunk { text, done } => {
            let text_js = serde_json::to_string(&text).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.spotlightAiChunk({}, {})",
                text_js, done
            ));
            None
        }
        AppEvent::SpotlightAiError { message } => {
            let msg_js = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.spotlightAiError({})",
                msg_js
            ));
            None
        }
        AppEvent::DownloadStarted {
            url,
            filename,
            path,
        } => {
            let mut dl = crate::browser::downloads::Download::new(&url, &filename);
            dl.local_path = Some(path);
            let dl = state.downloads.add(dl).clone();
            if let Err(e) = repositories::save_download(&state.conn, &dl) {
                tracing::warn!("save download start failed: {}", e);
            }
            let _ =
                chrome.evaluate_script("window.__neura && window.__neura.setDownloadActive(true)");
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::DownloadCompleted { url, path, success } => {
            if let Some(dl) = state.downloads.downloads.iter_mut().rev().find(|d| {
                d.url == url || (path.is_some() && d.local_path.as_deref() == path.as_deref())
            }) {
                if success {
                    dl.status = crate::browser::downloads::DownloadStatus::Complete;
                    if dl.local_path.is_none() && path.is_some() {
                        dl.local_path = path;
                    }
                    dl.completed_at = Some(chrono::Utc::now().timestamp_millis());
                } else {
                    dl.status = crate::browser::downloads::DownloadStatus::Failed;
                    dl.completed_at = Some(chrono::Utc::now().timestamp_millis());
                }
                if let Err(e) = repositories::save_download(&state.conn, dl) {
                    tracing::warn!("save download completion failed: {}", e);
                }
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::UpdateCheckResult {
            available,
            version,
            notes,
            download_url,
        } => {
            if available {
                state.pending_update_url = Some(download_url);
                state.pending_update_version = Some(version.clone());
                let v = serde_json::to_string(&version).unwrap_or_default();
                let n = serde_json::to_string(&notes).unwrap_or_default();
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setUpdateState({{status:'available',version:{},notes:{}}})",
                    v, n
                ));
            } else {
                let _ = chrome.evaluate_script(
                    "window.__neura && window.__neura.setUpdateState({status:'up_to_date'})",
                );
            }
            None
        }
        AppEvent::UpdateCheckFailed { message } => {
            let m = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setUpdateState({{status:'error',error:{}}})",
                m
            ));
            None
        }
        AppEvent::NeuraFeedLoaded { articles } => {
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setNeuraFeed({})",
                articles
            ));
            None
        }
        AppEvent::NeuraFeedFailed { message } => {
            let m = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setNeuraFeedError({})",
                m
            ));
            None
        }
        AppEvent::UpdateDownloadProgress { received, total } => {
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setUpdateState({{status:'downloading',received:{},total:{}}})",
                received, total
            ));
            None
        }
        AppEvent::UpdateDownloaded { path } => {
            let _ = chrome.evaluate_script(
                "window.__neura && window.__neura.setUpdateState({status:'installing'})",
            );
            // Record the version we're installing as "dismissed" so the notification
            // won't re-appear on next launch even if the new binary has a stale version string.
            if let Some(v) = &state.pending_update_version {
                let _ = settings_store::set(&state.conn, "dismissed_update_version", v);
            }
            if let Err(e) = crate::updater::apply_update(std::path::Path::new(&path)) {
                let m = serde_json::to_string(&e.to_string()).unwrap_or_default();
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setUpdateState({{status:'error',error:{}}})",
                    m
                ));
            }
            None
        }
        AppEvent::UpdateDownloadFailed { message } => {
            let m = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setUpdateState({{status:'error',error:{}}})",
                m
            ));
            None
        }
        AppEvent::ContentContextMenu {
            tab_id,
            x,
            y,
            link_url,
            image_src,
            selected_text,
            page_url,
            can_back,
        } => {
            if state.tab_manager.active_tab_id.as_deref() != Some(tab_id.as_str()) {
                return None;
            }
            let can_fwd = state
                .tab_manager
                .active_tab()
                .map(|t| t.can_go_forward)
                .unwrap_or(false);
            let link_js = serde_json::to_string(&link_url).unwrap_or_default();
            let img_js = serde_json::to_string(&image_src).unwrap_or_default();
            let text_js = serde_json::to_string(&selected_text).unwrap_or_default();
            let page_js = serde_json::to_string(&page_url).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.showContextMenu({{x:{:.1},y:{:.1},linkUrl:{},imageSrc:{},selectedText:{},pageUrl:{},canBack:{},canFwd:{}}})",
                x, y, link_js, img_js, text_js, page_js, can_back, can_fwd
            ));
            None
        }
        AppEvent::ContentNavState { tab_id, can_back } => {
            if let Some(tab) = state.tab_manager.get_tab_mut(&tab_id) {
                tab.sync_nav_flags();
                if can_back && tab.back_stack.is_empty() {
                    tab.can_go_back = true;
                }
            }
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let active = state
                    .tab_manager
                    .active_tab()
                    .map(|t| (t.can_go_back, t.can_go_forward));
                if let Some((back, fwd)) = active {
                    let _ = chrome.evaluate_script(&format!(
                        "window.__neura && window.__neura.updateNavState({},{},false)",
                        back, fwd
                    ));
                }
                state.push_state_to_chrome(chrome);
            }
            None
        }
        _ => None,
    }
}

fn should_record_replace_as_visit(old_url: &str, new_url: &str) -> bool {
    if old_url == new_url || old_url.starts_with("neura://") || new_url.starts_with("neura://") {
        return false;
    }
    let Ok(old) = url::Url::parse(old_url) else {
        return false;
    };
    let Ok(new) = url::Url::parse(new_url) else {
        return false;
    };
    if old.scheme() != new.scheme() || old.host_str() != new.host_str() {
        return false;
    }
    old.path() != new.path() || old.query() != new.query()
}

pub(crate) fn load_key(tab_id: &str, url: &str) -> String {
    format!("{}\n{}", tab_id, url)
}

#[cfg(test)]
mod tests {
    use super::{favicon_for_url, load_key, normalize_homepage, should_record_replace_as_visit};

    #[test]
    fn youtube_route_replace_counts_as_visit() {
        assert!(should_record_replace_as_visit(
            "https://www.youtube.com/",
            "https://www.youtube.com/watch?v=abc"
        ));
    }

    #[test]
    fn canonical_host_replace_stays_replace() {
        assert!(!should_record_replace_as_visit(
            "https://youtube.com/",
            "https://www.youtube.com/"
        ));
    }

    #[test]
    fn newtab_replace_stays_replace() {
        assert!(!should_record_replace_as_visit(
            "neura://newtab",
            "https://www.youtube.com/"
        ));
    }

    #[test]
    fn load_key_keeps_tab_and_url_together() {
        assert_eq!(
            load_key("tab", "https://youtube.com"),
            "tab\nhttps://youtube.com"
        );
    }

    #[test]
    fn favicon_uses_reported_icon() {
        assert_eq!(
            favicon_for_url(
                "https://youtube.com/",
                Some("https://youtube.com/s/desktop/favicon.ico".to_string())
            ),
            Some("https://youtube.com/s/desktop/favicon.ico".to_string())
        );
    }

    #[test]
    fn favicon_falls_back_to_origin() {
        assert_eq!(
            favicon_for_url("https://youtube.com/watch?v=abc", None),
            Some("https://youtube.com/favicon.ico".to_string())
        );
    }

    #[test]
    fn favicon_ignores_empty_and_internal_urls() {
        assert_eq!(
            favicon_for_url("neura://newtab", Some("".to_string())),
            None
        );
    }

    #[test]
    fn homepage_adds_scheme_to_bare_domain() {
        assert_eq!(normalize_homepage("google.com"), "https://google.com");
    }

    #[test]
    fn homepage_keeps_neura_url() {
        assert_eq!(normalize_homepage("neura://newtab"), "neura://newtab");
    }

    #[test]
    fn homepage_empty_uses_newtab() {
        assert_eq!(normalize_homepage(""), "neura://newtab");
    }
}

fn handle_save_settings(
    key: String,
    value: serde_json::Value,
    state: &mut AppState,
    chrome: &WebView,
) {
    match key.as_str() {
        "theme" => {
            if let Some(t) = value.as_str() {
                state.settings.appearance.theme = match t {
                    "light" => crate::config::Theme::Light,
                    "system" => crate::config::Theme::System,
                    _ => crate::config::Theme::Dark,
                };
                let _ = settings_store::set(&state.conn, "theme", &t);
            }
        }
        "onboarding_done" => {
            let _ = settings_store::set(&state.conn, "onboarding_done", &true);
        }
        "default_engine" => {
            if let Some(id) = value.as_str() {
                let _ = repositories::set_default_search_engine(&state.conn, id);
                state.settings.search.default_engine = id.to_string();
                state.cached_search_engines =
                    repositories::list_search_engines(&state.conn).unwrap_or_default();
            }
        }
        "ai_keys" => {
            if let Some(obj) = value.as_object() {
                for (provider, key_val) in obj {
                    if let Some(k) = key_val.as_str() {
                        if provider == "model" {
                            if !k.is_empty() {
                                state.settings.ai.default_model = k.to_string();
                            }
                        } else if !k.is_empty() {
                            let _ = keychain::set_api_key(provider, k);
                        }
                    }
                }
                let _ = chrome.evaluate_script(
                    "window.__neura && window.__neura.showSuccess('API keys saved')",
                );
            }
        }
        "save_history" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.disable_history = !v;
                let _ = settings_store::set(
                    &state.conn,
                    "disable_history",
                    &state.settings.privacy.disable_history,
                );
            }
        }
        "ad_blocker_enabled" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.ad_blocker_enabled = v;
                state.ad_block_engine.set_enabled(v);
                let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            }
        }
        "ad_blocker_exceptions" => {
            if let Some(arr) = value.as_array() {
                let exceptions: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                state.ad_block_engine.set_exceptions(exceptions.clone());
                state.settings.privacy.ad_blocker_exceptions = exceptions;
            }
        }
        "sidebar_mode" => {
            if let Some(m) = value.as_str() {
                state.settings.appearance.sidebar_mode = match m {
                    "compact" => crate::config::SidebarMode::Compact,
                    "auto_hide" => crate::config::SidebarMode::AutoHide,
                    _ => crate::config::SidebarMode::Expanded,
                };
                state.sidebar_auto_hide_open = false;
                state.sidebar_pinned = false;
                state.sidebar_collapsed = m == "compact";
                let _ = settings_store::set(&state.conn, "sidebar_mode", &m);
            }
        }
        "startup_behavior" => {
            if let Some(v) = value.as_str() {
                state.settings.startup_behavior = match v {
                    "last_session" => crate::config::StartupBehavior::LastSession,
                    "home_page" | "specific_pages" => crate::config::StartupBehavior::HomePage,
                    _ => crate::config::StartupBehavior::NewTab,
                };
                let _ = settings_store::set(&state.conn, "startup_behavior", &v);
            }
        }
        "homepage" => {
            if let Some(v) = value.as_str() {
                let homepage = normalize_homepage(v);
                state.settings.homepage = homepage.clone();
                let _ = settings_store::set(&state.conn, "homepage", &homepage);
            }
        }
        "download_path" => {
            if let Some(v) = value.as_str() {
                let path = v.trim().to_string();
                state.settings.downloads.default_folder = path.clone();
                let _ = settings_store::set(&state.conn, "download_path", &path);
            }
        }
        "ask_download" => {
            if let Some(v) = value.as_bool() {
                state.settings.downloads.ask_where_to_save = v;
                let _ = settings_store::set(&state.conn, "ask_where_to_save", &v);
            }
        }
        "show_bookmarks_bar" => {
            if let Some(v) = value.as_bool() {
                state.settings.appearance.show_bookmarks_bar = v;
            }
        }
        "search_suggestions" => {
            if let Some(v) = value.as_bool() {
                state.settings.search.suggestions_enabled = v;
            }
        }
        "trending" => {
            if let Some(v) = value.as_bool() {
                state.settings.search.trending_enabled = v;
            }
        }
        "new_tab_show_search" => {
            if let Some(v) = value.as_bool() {
                state.settings.new_tab.show_search = v;
            }
        }
        "new_tab_show_quick_links" => {
            if let Some(v) = value.as_bool() {
                state.settings.new_tab.show_quick_links = v;
            }
        }
        "new_tab_show_background" => {
            if let Some(v) = value.as_bool() {
                state.settings.new_tab.show_background = v;
            }
        }
        "new_tab_feed_layout" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.feed_layout = match v {
                    "headlines" | "compact" => v.to_string(),
                    _ => "cards".to_string(),
                };
            }
        }
        "new_tab_theme" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.theme = match v {
                    "minimal" | "focus" | "horizon" | "informative" => v.to_string(),
                    _ => "informative".to_string(),
                };
            }
        }
        "new_tab_clock_style" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.clock_style = match v {
                    "sf" | "rounded" | "mono" | "serif" => v.to_string(),
                    _ => "sf".to_string(),
                };
            }
        }
        "new_tab_wallpaper_source" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.wallpaper_source = match v {
                    "daily" | "nature" | "url" | "upload" | "color" | "none" => v.to_string(),
                    _ => "daily".to_string(),
                };
            }
        }
        "new_tab_wallpaper_url" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.wallpaper_url = v.to_string();
            }
        }
        "new_tab_wallpaper_color" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.wallpaper_color = v.to_string();
            }
        }
        "region" => {
            if let Some(v) = value.as_str() {
                state.settings.region = v.to_string();
            }
        }
        _ => {
            let _ = settings_store::set(&state.conn, &key, &value);
        }
    }
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
    state.push_state_to_chrome(chrome);
}

fn ai_model_for(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "claude-3-5-sonnet-20241022",
        "gemini" => "gemini-2.5-flash",
        "openrouter" => "openai/gpt-4o-mini",
        "ollama" => "llama3.1",
        _ => "gpt-4o-mini",
    }
}

fn should_swap_ai_model(model: &str) -> bool {
    matches!(
        model,
        "" | "gpt-4o-mini"
            | "claude-3-5-sonnet-20241022"
            | "gemini-2.5-flash"
            | "openai/gpt-4o-mini"
            | "llama3.1"
    )
}

fn get_search_url(state: &AppState) -> String {
    let engine_id = &state.settings.search.default_engine;
    if let Ok(Some(engine)) = repositories::get_default_search_engine(&state.conn) {
        return engine.url_template;
    }
    match engine_id.as_str() {
        "google" => "https://www.google.com/search?q={query}".to_string(),
        "bing" => "https://www.bing.com/search?q={query}".to_string(),
        "brave" => "https://search.brave.com/search?q={query}".to_string(),
        "perplexity" => "https://www.perplexity.ai/search?q={query}".to_string(),
        _ => "https://www.google.com/search?q={query}".to_string(),
    }
}

fn resolve_navigation_url(input: &str, state: &AppState) -> String {
    if state.settings.search.site_shortcuts_enabled {
        if let Ok(engines) = repositories::list_search_engines(&state.conn) {
            if let Some((_, url)) = crate::browser::search_engine::SearchEngine::resolve_shortcut(
                input.trim(),
                &engines,
            ) {
                return url;
            }
        }
    }
    let search_url = get_search_url(state);
    crate::browser::navigation::resolve_input(input, &search_url).url
}

pub fn normalize_homepage(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "neura://newtab".to_string();
    }
    let result = crate::browser::navigation::resolve_input(
        trimmed,
        "https://www.google.com/search?q={query}",
    );
    if result.is_search {
        return trimmed.to_string();
    }
    result.url
}

fn pick_download_folder(current: &str) -> Option<String> {
    let mut dlg = rfd::FileDialog::new().set_title("Choose download folder");
    let cur = current.trim();
    if !cur.is_empty() {
        let path = std::path::PathBuf::from(cur);
        if path.exists() {
            dlg = dlg.set_directory(path);
        }
    }
    dlg.pick_folder()
        .map(|path| path.to_string_lossy().to_string())
}
