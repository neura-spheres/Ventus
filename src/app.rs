use rusqlite::Connection;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;
use wry::WebView;

use crate::adblock::AdBlockEngine;
use crate::browser::downloads::DownloadManager;
use crate::browser::tab_manager::TabManager;
use crate::config::{
    clean_toolbar_buttons, valid_site_permission_key, valid_site_permission_value, AppSettings,
    SecureDnsMode, SecureDnsProvider, SitePermissions,
};
use crate::storage::{keychain, passwords, repositories, settings_store};
use crate::ui::events::{AppEvent, ChromeCommand};

const WEB_ERROR_UNKNOWN: i32 = 0;
const WEB_ERROR_CONNECTION_ABORTED: i32 = 9;
const WEB_ERROR_OPERATION_CANCELED: i32 = 14;

// How many times to rebuild the controller for a tab that is uncommitted AND has NO native
// navigation in flight — i.e. the controller produced no navigation at all and is genuinely
// wedged (the original black-tab bug). After this many rebuilds we stop the spinner. This
// path is NEVER taken while a navigation is still in flight; see IN_FLIGHT_PATIENT_TRIES.
const UNCOMMITTED_PATIENT_TRIES: u8 = 3;

// How many LOAD_STALL_AFTER windows (~6 s each) to keep extending the watchdog for an
// uncommitted navigation that is still genuinely in flight (WebView2 holds the tab in
// `native_nav_ids` from NavigationStarting until it fires Completed/Failed). A slow or cold
// main-document download — YouTube on an old machine, several tabs open — can take far longer
// than the wedged-controller budget to commit while its connection is perfectly alive.
// Rebuilding such a load aborts the live connection (status 9) and restarts from scratch,
// which on a slow machine never finishes and churns the tab black; so while in flight we only
// ever wait, never rebuild. This grants ~IN_FLIGHT_PATIENT_TRIES * 6 s (~72 s) before we give
// up the spinner — and even then we leave the connection untouched. A real network failure
// arrives much sooner as a NavigationCompleted/Failed (the `failed` retry ladder), so this
// ceiling only bites when the response is genuinely just very slow.
const IN_FLIGHT_PATIENT_TRIES: u8 = 12;

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
    pub pending_ai_attachments: Vec<crate::ai::AiAttachment>,
    pub current_ai_session_id: Option<String>,
    pub current_ai_provider: String,
    pub sidebar_collapsed: bool,
    pub ai_sidebar_open: bool,
    pub chrome_overlay_open: bool,
    pub sidebar_auto_hide_open: bool,
    pub sidebar_pinned: bool,
    /// Live clip-column width (CSS px) streamed by JS while the auto-hide sidebar
    /// slides. When `Some`, the layout uses it for the chrome clip + content cut so
    /// they follow the sidebar's animated edge. `None` = no animation in progress.
    pub sidebar_clip_w_override: Option<f64>,
    pub suggestion_overlay_rects: HashMap<String, ChromeClipRect>,
    pub content_cover_open: bool,
    pub pending_update_url: Option<String>,
    pub pending_update_version: Option<String>,
    pub pending_update_notes: Option<String>,
    pub content_fullscreen: bool,
    pub pending_nav_urls: HashMap<String, String>,
    pub https_upgrades: HashMap<String, String>,
    pub native_loads: HashMap<String, String>,
    pub native_nav_ids: HashMap<String, u64>,
    pub load_recoveries: HashMap<String, u8>,
    /// Highest load progress (0.0-1.0) seen for each tab's current load. Reset when a new
    /// load starts. Used to avoid reloading a page that has already rendered content but is
    /// just slow to fire its final load event (heavy page on a slow device).
    pub load_progress: HashMap<String, f64>,
    pub nav_started_at: HashMap<String, std::time::Instant>,
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
    pub cached_bookmark_folders: Vec<repositories::BookmarkFolder>,
    /// Explicit left-to-right order of the bookmark bar (folders + unfiled bookmarks
    /// intermixed). Persisted in the settings table under `bookmark_bar_order`.
    pub bookmark_bar_order: Vec<crate::ui::events::BarOrderRef>,
    pub cached_history: Vec<repositories::HistoryEntry>,
    pub auth: Option<crate::cloud::AuthSession>,
    pub user_profile: Option<crate::cloud::UserProfile>,
    /// Per-tab: (url, row_id) of the last history entry saved for the current page.
    /// Cleared on ContentLoadStart so the same URL can be re-saved after a reload/re-navigate.
    /// Used to deduplicate the multiple ContentMetadata events fired per page load and to
    /// update the title row when a better title arrives after the initial save.
    pub history_last_saved: HashMap<String, (String, i64)>,
    pub pwd_key: [u8; 32],
    pub pending_pwd_save: Option<(String, String, String)>,
    /// Per-origin set of permission keys the site has actually requested this session.
    /// Drives the site-info popover so it lists only what a page asked for.
    pub requested_permissions: HashMap<String, BTreeSet<String>>,
    /// Per-download speed sampling: (sample time ms, bytes at that sample, smoothed bytes/sec).
    /// Used to turn raw byte-count ticks into a smoothed bytes/sec figure for the UI.
    pub download_samples: HashMap<String, (i64, u64, u64)>,
    /// On-device learned ranker for address-bar suggestions. Updated each time the
    /// user picks a suggestion, persisted to the settings table.
    pub omnibox: crate::browser::omnibox::Model,
    pub trends: Vec<crate::browser::omnibox::Trend>,
    pub trends_region: String,
    pub trends_fetched_at: i64,
    pub trends_loading: bool,
    pub device_id: String,
    pub session_id: String,
}

impl AppState {
    pub fn new(
        conn: Connection,
        settings: AppSettings,
        data_dir: &std::path::Path,
        device_id: String,
        session_id: String,
    ) -> Self {
        let ai_provider = settings.ai.default_provider.clone();
        let ad_block_enabled = settings.privacy.ad_blocker_enabled;
        let ad_block_exceptions = settings.privacy.ad_blocker_exceptions.clone();
        let sidebar_collapsed = matches!(
            settings.appearance.sidebar_mode,
            crate::config::SidebarMode::Compact
        );
        let _ = repositories::fail_stale_downloads(&conn);
        let mut downloads = repositories::list_downloads(&conn, 100).unwrap_or_default();
        downloads.reverse();
        let cached_search_engines = repositories::list_search_engines(&conn).unwrap_or_default();
        let cached_bookmarks = repositories::list_bookmarks(&conn).unwrap_or_default();
        let cached_bookmark_folders =
            repositories::list_bookmark_folders(&conn).unwrap_or_default();
        let bookmark_bar_order = settings_store::get(&conn, "bookmark_bar_order")
            .ok()
            .flatten()
            .unwrap_or_default();
        let cached_history = repositories::list_history(&conn, 30).unwrap_or_default();
        let omnibox = crate::browser::omnibox::load(&conn);
        Self {
            tab_manager: TabManager::new(),
            downloads: DownloadManager::with_downloads(downloads),
            settings,
            conn,
            ai_messages: Vec::new(),
            pending_ai_attachments: Vec::new(),
            current_ai_session_id: None,
            current_ai_provider: ai_provider,
            sidebar_collapsed,
            ai_sidebar_open: false,
            chrome_overlay_open: false,
            sidebar_auto_hide_open: false,
            sidebar_pinned: false,
            sidebar_clip_w_override: None,
            suggestion_overlay_rects: HashMap::new(),
            content_cover_open: false,
            pending_update_url: None,
            pending_update_version: None,
            pending_update_notes: None,
            content_fullscreen: false,
            pending_nav_urls: HashMap::new(),
            https_upgrades: HashMap::new(),
            native_loads: HashMap::new(),
            native_nav_ids: HashMap::new(),
            load_recoveries: HashMap::new(),
            load_progress: HashMap::new(),
            nav_started_at: HashMap::new(),
            spotlight_open: false,
            zoom_levels: HashMap::new(),
            ai_pending_tools: Arc::new(Mutex::new(HashMap::new())),
            ad_block_engine: AdBlockEngine::new(ad_block_enabled, &ad_block_exceptions),
            adblock_page_kills: 0,
            cached_search_engines,
            cached_bookmarks,
            cached_bookmark_folders,
            bookmark_bar_order,
            cached_history,
            auth: None,
            user_profile: None,
            history_last_saved: HashMap::new(),
            pwd_key: crate::storage::crypto::store_key(data_dir).unwrap_or([0u8; 32]),
            pending_pwd_save: None,
            requested_permissions: HashMap::new(),
            download_samples: HashMap::new(),
            omnibox,
            trends: Vec::new(),
            trends_region: String::new(),
            trends_fetched_at: 0,
            trends_loading: false,
            device_id,
            session_id,
        }
    }

    fn download_speed(&mut self, id: &str, received: u64) -> u64 {
        let now = chrono::Utc::now().timestamp_millis();
        let entry = self
            .download_samples
            .entry(id.to_string())
            .or_insert((now, received, 0));
        let (t_last, b_last, ema) = *entry;
        let dt = now - t_last;
        if dt < 250 {
            return ema;
        }
        let instant = if received >= b_last {
            (received - b_last).saturating_mul(1000) / dt as u64
        } else {
            0
        };
        let new_ema = if ema == 0 {
            instant
        } else {
            (instant * 3 + ema * 7) / 10
        };
        *entry = (now, received, new_ema);
        new_ema
    }

    pub fn account_state_json(&self) -> String {
        let profile = self.user_profile.clone().unwrap_or_default();
        serde_json::json!({
            "configured": crate::cloud::config::is_configured(),
            "cloudinary": crate::cloud::config::cloudinary_configured(),
            "signed_in": self.auth.is_some(),
            "profile": {
                "email": profile.email,
                "username": profile.username,
                "full_name": profile.full_name,
                "birthdate": profile.birthdate,
                "bio": profile.bio,
                "photo_url": profile.photo_url,
                "country": profile.country,
            }
        })
        .to_string()
    }

    pub fn push_account(&self, chrome: &WebView) {
        let _ = chrome.evaluate_script(&format!(
            "window.__neura && window.__neura.setAccount({})",
            self.account_state_json()
        ));
    }

    pub fn persist_session(
        &self,
        session: &crate::cloud::AuthSession,
        profile: &crate::cloud::UserProfile,
    ) {
        let _ = keychain::set_api_key(crate::cloud::KEYCHAIN_REFRESH_KEY, &session.refresh_token);
        let _ = settings_store::set(&self.conn, crate::cloud::PROFILE_CACHE_KEY, profile);
    }

    pub fn clear_session(&self) {
        let _ = keychain::delete_api_key(crate::cloud::KEYCHAIN_REFRESH_KEY);
        let _ = settings_store::delete(&self.conn, crate::cloud::PROFILE_CACHE_KEY);
    }

    pub fn push_state_to_chrome(&self, chrome: &WebView) {
        let json = self.chrome_state_json();
        let _ = chrome.evaluate_script(&format!(
            "window.__neura && window.__neura.setState({})",
            json
        ));
    }

    pub fn set_content_cover(&mut self, chrome: &WebView, visible: bool) {
        if self.content_cover_open == visible {
            return;
        }
        self.content_cover_open = visible;
        let js = if visible {
            "window.__neura&&window.__neura.showContentLoading&&window.__neura.showContentLoading()"
        } else {
            "window.__neura&&window.__neura.hideContentLoading&&window.__neura.hideContentLoading()"
        };
        let _ = chrome.evaluate_script(js);
    }

    pub fn push_newtab_wallpaper_to_chrome(&self, chrome: &WebView) {
        let data = self.settings.new_tab.wallpaper_data.as_str();
        if data.is_empty() {
            return;
        }
        let json = serde_json::to_string(data).unwrap_or_default();
        let _ = chrome.evaluate_script(&format!(
            "window.__neura && window.__neura.setNewtabWallpaperData({})",
            json
        ));
    }

    pub fn chrome_state_json(&self) -> String {
        let active_tab = self.tab_manager.active_tab();
        let workspace_tab_counts = self.tab_manager.workspace_tab_counts();
        let active_url = active_tab.map(|t| t.url.as_str()).unwrap_or("");
        let ad_blocker_active = self.settings.privacy.ad_blocker_enabled;
        let ad_blocker_site_excepted =
            !active_url.is_empty() && self.ad_block_engine.is_site_excepted(active_url);
        let mut settings_value = serde_json::to_value(&self.settings).unwrap_or_default();
        if let Some(nt) = settings_value
            .get_mut("new_tab")
            .and_then(|v| v.as_object_mut())
        {
            nt.insert(
                "wallpaper_data".to_string(),
                serde_json::Value::String(String::new()),
            );
        }
        serde_json::to_string(&serde_json::json!({
            "tabs": self.tab_manager.active_workspace_tabs(),
            "workspaces": &self.tab_manager.workspaces,
            "workspace_tab_counts": workspace_tab_counts,
            "active_tab_id": self.tab_manager.active_tab_id,
            "active_workspace_id": self.tab_manager.active_workspace_id,
            "active_url": active_url,
            "active_title": active_tab.map(|t| t.title.as_str()).unwrap_or(""),
            "can_go_back": active_tab.map(|t| t.nav_back()).unwrap_or(false),
            "can_go_fwd": active_tab.map(|t| t.nav_forward()).unwrap_or(false),
            "is_loading": active_tab.map(|t| t.status == crate::browser::tab::TabStatus::Loading).unwrap_or(false),
            "settings": settings_value,
            "search_engines": &self.cached_search_engines,
            "sidebar_collapsed": self.sidebar_collapsed,
            "ai_open": self.ai_sidebar_open,
            "bookmarks": &self.cached_bookmarks,
            "bookmark_folders": &self.cached_bookmark_folders,
            "bar_order": &self.bookmark_bar_order,
            "history": &self.cached_history,
            "downloads": &self.downloads.downloads,
            "ai_key_status": {
                "anthropic": keychain::has_api_key("anthropic"),
                "anthropic_compatible": keychain::has_api_key("anthropic"),
                "openai": keychain::has_api_key("openai"),
                "openai_compatible": keychain::has_api_key("openai"),
                "gemini": keychain::has_api_key("gemini"),
                "gemini_compatible": keychain::has_api_key("gemini"),
                "openrouter": keychain::has_api_key("openrouter"),
            },
            "ad_blocker_active": ad_blocker_active,
            "ad_blocker_site_excepted": ad_blocker_site_excepted,
            "ad_blocker_kills": self.adblock_page_kills,
            "requested_permissions": &self.requested_permissions,
        }))
        .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DownloadCtl {
    Pause,
    Resume,
    Cancel,
}

pub enum TabAction {
    Create {
        tab_id: String,
        url: String,
    },
    Remove(String),
    RemoveMany(Vec<String>),
    SyncViews,
    FocusSpotlight,
    SyncClipOnly,
    SyncSidebarClip,
    ContentScript(String),
    SetZoom(f64),
    SetZoomFor {
        tab_id: String,
        level: f64,
    },
    SetZoomAll(f64),
    ContentNavigate(String),
    ContentGoBack,
    ContentGoForward,
    ReadClipboardForOmnibox,
    ReloadContent {
        tab_id: String,
        url: String,
    },
    NudgeContent {
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
    ExtendLoadWatch {
        tab_id: String,
        url: String,
    },
    DropContent {
        tab_id: String,
    },
    ShowErrorPage {
        tab_id: String,
    },
    ApplyWebSecurity,
    DownloadUpdate(String),
    ApplyUpdate(String),
    DownloadControl {
        id: String,
        action: DownloadCtl,
    },
    DownloadCancelAll,
    SaveImageAs {
        url: String,
    },
    CopyImageToClipboard {
        url: String,
    },
    SetFullscreen(bool),
    ContentScriptOnTab {
        tab_id: String,
        js: String,
    },
    FindInPage {
        tab_id: String,
        query: String,
        forward: bool,
    },
    ResolvePermission {
        origin: String,
        key: String,
        allow: bool,
    },
    SendReport(Box<crate::cloud::report::Report>),
}

fn switch_to_tab(state: &mut AppState, chrome: &WebView, id: String) -> Option<TabAction> {
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

pub fn handle_chrome_command(
    cmd: ChromeCommand,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    log_feature_command(&cmd, state);
    match cmd {
        ChromeCommand::Navigate { url } => navigate_current_tab(url, state, chrome),
        ChromeCommand::ContinueHttp { url } => {
            navigate_current_tab_with_policy(url, state, chrome, false)
        }
        ChromeCommand::NavigateFromOverlay { url } => {
            state.chrome_overlay_open = false;
            state.suggestion_overlay_rects.clear();
            navigate_current_tab(url, state, chrome).or(Some(TabAction::SyncViews))
        }
        ChromeCommand::Back => {
            let tab_id = state.tab_manager.active_tab_id.clone()?;
            begin_user_nav(state, chrome, &tab_id);
            if is_neura_tab(state, &tab_id) {
                let target = state.tab_manager.go_back(&tab_id)?;
                commit_stack_nav(&tab_id, &target, state, chrome)
            } else {
                Some(TabAction::ContentGoBack)
            }
        }
        ChromeCommand::Forward => {
            let tab_id = state.tab_manager.active_tab_id.clone()?;
            begin_user_nav(state, chrome, &tab_id);
            if is_neura_tab(state, &tab_id) {
                let target = state.tab_manager.go_forward(&tab_id)?;
                commit_stack_nav(&tab_id, &target, state, chrome)
            } else {
                Some(TabAction::ContentGoForward)
            }
        }
        ChromeCommand::Reload => reload_current_tab(state, chrome),
        ChromeCommand::Stop => Some(TabAction::ContentScript("window.stop()".into())),
        ChromeCommand::OmniboxPaste => Some(TabAction::ReadClipboardForOmnibox),
        ChromeCommand::NewTab => {
            let tab_id;
            let url;
            {
                let tab = state.tab_manager.new_tab(None);
                tab_id = tab.id.clone();
                url = tab.url.clone();
            }
            state.push_state_to_chrome(chrome);
            let _ = chrome.focus();
            let _ = chrome.evaluate_script("focusUrl()");
            Some(TabAction::Create { tab_id, url })
        }
        ChromeCommand::CloseTab { id } => {
            state.tab_manager.close_tab(&id);
            state.load_progress.remove(&id);
            state.native_loads.remove(&id);
            state.native_nav_ids.remove(&id);
            state.history_last_saved.remove(&id);
            state.push_state_to_chrome(chrome);
            let ws_id = state.tab_manager.active_workspace_id.clone();
            let landed_on_lone_newtab = state.tab_manager.workspace_tabs(&ws_id).count() == 1
                && state
                    .tab_manager
                    .active_tab()
                    .map(|t| t.url == "neura://newtab")
                    .unwrap_or(false);
            if landed_on_lone_newtab {
                let _ = chrome.focus();
                let _ = chrome.evaluate_script("focusUrl()");
            }
            Some(TabAction::Remove(id))
        }
        ChromeCommand::SwitchTab { id } => switch_to_tab(state, chrome, id),
        ChromeCommand::SwitchTabOffset { delta } => {
            let id = {
                let tabs = state.tab_manager.active_workspace_tabs();
                if tabs.len() < 2 {
                    return None;
                }
                let current = state.tab_manager.active_tab_id.as_deref();
                let index = current
                    .and_then(|id| tabs.iter().position(|tab| tab.id == id))
                    .unwrap_or(0);
                let next = (index as i32 + delta).rem_euclid(tabs.len() as i32) as usize;
                tabs[next].id.clone()
            };
            switch_to_tab(state, chrome, id)
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
        ChromeCommand::MoveTab { id, before } => {
            state.tab_manager.move_tab(&id, before.as_deref());
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
            for tab_id in &tab_ids {
                state.load_progress.remove(tab_id);
                state.native_loads.remove(tab_id);
                state.native_nav_ids.remove(tab_id);
                state.history_last_saved.remove(tab_id);
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
        ChromeCommand::PickAiAttachments => {
            let used_count = state
                .ai_messages
                .iter()
                .map(|message| message.attachments.len())
                .sum();
            let used_bytes = state
                .ai_messages
                .iter()
                .flat_map(|message| &message.attachments)
                .map(|item| item.size)
                .sum();
            match crate::ai::attachments::pick(
                &state.pending_ai_attachments,
                used_count,
                used_bytes,
            ) {
                Ok(Some(attachments)) => {
                    state.pending_ai_attachments = attachments;
                    push_ai_attachments(chrome, &state.pending_ai_attachments);
                }
                Ok(None) => {}
                Err(e) => {
                    let message = serde_json::to_string(&e.to_string()).unwrap_or_default();
                    let _ = chrome.evaluate_script(&format!(
                        "window.__neura&&window.__neura.showError({message})"
                    ));
                }
            }
            None
        }
        ChromeCommand::RemoveAiAttachment { id } => {
            crate::ai::attachments::remove(&mut state.pending_ai_attachments, &id);
            push_ai_attachments(chrome, &state.pending_ai_attachments);
            None
        }
        ChromeCommand::AiProviderChange { provider } => {
            let provider = normalize_ai_provider(&provider).to_string();
            state.current_ai_provider = provider.clone();
            state.settings.ai.default_provider = provider.clone();
            if !ai_model_matches_provider(&provider, &state.settings.ai.default_model) {
                state.settings.ai.default_model = ai_model_for(&provider).to_string();
            }
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::AiModelChange { model, provider } => {
            if let Some(provider) = provider
                .as_deref()
                .map(normalize_ai_provider)
                .or_else(|| ai_provider_for_model(&model))
            {
                state.current_ai_provider = provider.to_string();
                state.settings.ai.default_provider = provider.to_string();
            }
            state.settings.ai.default_model = model;
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::AiClearChat => {
            state.ai_messages.clear();
            state.pending_ai_attachments.clear();
            state.current_ai_session_id = None;
            push_ai_attachments(chrome, &state.pending_ai_attachments);
            None
        }
        ChromeCommand::GetAiSessions => {
            let json = ai_sessions_json(&state.conn);
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setAiSessions({})",
                json
            ));
            None
        }
        ChromeCommand::LoadAiSession { id } => {
            match crate::ai::chat::load_messages(&state.conn, &id) {
                Ok(messages) if !messages.is_empty() => {
                    let payload = ai_messages_json(&messages);
                    state.ai_messages = messages;
                    state.pending_ai_attachments.clear();
                    state.current_ai_session_id = Some(id);
                    state.ai_sidebar_open = true;
                    let _ = chrome.evaluate_script(&format!(
                        "window.__neura && window.__neura.showAiConversation({})",
                        payload
                    ));
                    push_ai_attachments(chrome, &state.pending_ai_attachments);
                    state.push_state_to_chrome(chrome);
                    Some(TabAction::SyncViews)
                }
                Ok(_) => None,
                Err(e) => {
                    tracing::warn!("LoadAiSession failed: {}", e);
                    None
                }
            }
        }
        ChromeCommand::DeleteAiSession { id } => {
            if let Err(e) = crate::ai::chat::delete_session(&state.conn, &id) {
                tracing::warn!("DeleteAiSession failed: {}", e);
            }
            if state.current_ai_session_id.as_deref() == Some(id.as_str()) {
                state.current_ai_session_id = None;
            }
            let json = ai_sessions_json(&state.conn);
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setAiSessions({})",
                json
            ));
            None
        }
        ChromeCommand::SaveSpotlightChat {
            session_id,
            title,
            query,
            answer,
        } => {
            persist_ai_exchange(state, &session_id, &title, &query, &answer, Vec::new());
            None
        }
        ChromeCommand::AiStop => None,
        ChromeCommand::BookmarkAdd => {
            if let Some(tab) = state.tab_manager.active_tab() {
                let url = tab.url.clone();
                let title = tab.title.clone();
                let favicon = favicon_for_url(&url, tab.favicon.clone());
                match repositories::add_bookmark(
                    &state.conn,
                    &url,
                    &title,
                    favicon.as_deref(),
                    None,
                ) {
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
        ChromeCommand::BookmarkAddUrl { url, title } => {
            let url = url.trim().to_string();
            if url.is_empty() || url.starts_with("neura://") {
                return None;
            }
            if repositories::is_bookmarked(&state.conn, &url).unwrap_or(false) {
                let _ = chrome.evaluate_script(
                    "window.__neura && window.__neura.showSuccess('Already bookmarked')",
                );
                return None;
            }
            let title = if title.trim().is_empty() {
                url.clone()
            } else {
                title
            };
            let favicon = state
                .tab_manager
                .active_tab()
                .filter(|tab| tab.url == url)
                .and_then(|tab| favicon_for_url(&url, tab.favicon.clone()))
                .or_else(|| favicon_for_url(&url, None));
            match repositories::add_bookmark(&state.conn, &url, &title, favicon.as_deref(), None) {
                Ok(_) => {
                    state.cached_bookmarks =
                        repositories::list_bookmarks(&state.conn).unwrap_or_default();
                    if state
                        .tab_manager
                        .active_tab()
                        .map(|t| t.url == url)
                        .unwrap_or(false)
                    {
                        let _ = chrome.evaluate_script(
                            "window.__neura && window.__neura.setBookmarked(true)",
                        );
                    }
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.showSuccess('Bookmark saved')",
                    );
                    state.push_state_to_chrome(chrome);
                }
                Err(e) => tracing::warn!("Bookmark add (drop) failed: {}", e),
            }
            None
        }
        ChromeCommand::MoveBookmark { id, before } => {
            let _ = repositories::move_bookmark(&state.conn, &id, before.as_deref());
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::MoveBookmarkFolder { id, before } => {
            let _ = repositories::move_bookmark_folder(&state.conn, &id, before.as_deref());
            state.cached_bookmark_folders =
                repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::SetBarOrder { order } => {
            let _ = settings_store::set(&state.conn, "bookmark_bar_order", &order);
            state.bookmark_bar_order = order;
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkRemove { url } => {
            let _ = repositories::remove_bookmark_by_url(&state.conn, &url);
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            let _ = chrome.evaluate_script("window.__neura && window.__neura.setBookmarked(false)");
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkRemoveById { id } => {
            let _ = state
                .conn
                .execute("DELETE FROM bookmarks WHERE id = ?1", [&id]);
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.cached_bookmark_folders =
                repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
            if state
                .tab_manager
                .active_tab()
                .map(|t| !state.cached_bookmarks.iter().any(|b| b.url == t.url))
                .unwrap_or(false)
            {
                let _ =
                    chrome.evaluate_script("window.__neura && window.__neura.setBookmarked(false)");
            }
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkRename { id, title } => {
            let title = title.trim();
            if !title.is_empty() {
                let _ = repositories::rename_bookmark(&state.conn, &id, title);
                state.cached_bookmarks =
                    repositories::list_bookmarks(&state.conn).unwrap_or_default();
                state.push_state_to_chrome(chrome);
                let _ = chrome.evaluate_script(
                    "window.__neura && window.__neura.showSuccess('Bookmark renamed')",
                );
            }
            None
        }
        ChromeCommand::BookmarkSetIconOnly { id, icon_only } => {
            let _ = repositories::set_bookmark_icon_only(&state.conn, &id, icon_only);
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkCreateFolder {
            bookmark_id_a,
            bookmark_id_b,
        } => {
            match repositories::add_bookmark_folder(&state.conn, "New Folder") {
                Ok(folder) => {
                    let _ = repositories::set_bookmark_folder(
                        &state.conn,
                        &bookmark_id_a,
                        Some(&folder.id),
                    );
                    let _ = repositories::set_bookmark_folder(
                        &state.conn,
                        &bookmark_id_b,
                        Some(&folder.id),
                    );
                    state.cached_bookmarks =
                        repositories::list_bookmarks(&state.conn).unwrap_or_default();
                    state.cached_bookmark_folders =
                        repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
                    state.push_state_to_chrome(chrome);
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.showSuccess('Folder created')",
                    );
                }
                Err(e) => tracing::warn!("BookmarkCreateFolder failed: {}", e),
            }
            None
        }
        ChromeCommand::BookmarkMoveToFolder {
            bookmark_id,
            folder_id,
        } => {
            let _ = repositories::set_bookmark_folder(&state.conn, &bookmark_id, Some(&folder_id));
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkRemoveFromFolder { bookmark_id } => {
            let _ = repositories::set_bookmark_folder(&state.conn, &bookmark_id, None);
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkFolderRename { folder_id, name } => {
            let _ = repositories::rename_bookmark_folder(&state.conn, &folder_id, &name);
            state.cached_bookmark_folders =
                repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkFolderDelete { folder_id } => {
            let _ = repositories::delete_bookmark_folder(&state.conn, &folder_id);
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.cached_bookmark_folders =
                repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            None
        }
        ChromeCommand::BookmarkNewFolder => {
            match repositories::add_bookmark_folder(&state.conn, "New folder") {
                Ok(_) => {
                    state.cached_bookmark_folders =
                        repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
                    state.push_state_to_chrome(chrome);
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.showSuccess('New folder added')",
                    );
                }
                Err(e) => tracing::warn!("BookmarkNewFolder failed: {}", e),
            }
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
            let needs_layout =
                key == "tab_layout" || key == "sidebar_mode" || key == "show_bookmarks_bar";
            let ad_blocker_toggled = key == "ad_blocker_enabled";
            let security_changed = key.starts_with("secure_dns_")
                || matches!(
                    key.as_str(),
                    "https_only"
                        | "block_third_party_cookies"
                        | "storage_partitioning"
                        | "fingerprint_protection"
                        | "strict_permissions"
                );
            let security_before = if security_changed {
                Some(web_security_signature(&state.settings))
            } else {
                None
            };
            let secure_dns_valid = key != "secure_dns_template"
                || value
                    .as_str()
                    .and_then(crate::config::clean_doh_url)
                    .is_some();
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
            if security_changed && secure_dns_valid {
                let security_after = web_security_signature(&state.settings);
                if security_before
                    .as_ref()
                    .map_or(true, |before| before != &security_after)
                {
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.restartNeeded && window.__neura.restartNeeded()",
                    );
                }
            }
            if ad_blocker_toggled {
                return None;
            }
            None
        }
        ChromeCommand::SetSitePermission {
            origin,
            permission,
            value,
        } => set_site_permission(origin, permission, value, state, chrome),
        ChromeCommand::SetDefaultPermission { permission, value } => {
            set_default_permission(permission, value, state, chrome)
        }
        ChromeCommand::PermissionDecision {
            id: _,
            origin,
            permission,
            decision,
        } => permission_decision(origin, permission, decision, state, chrome),
        ChromeCommand::SendReport { message } => {
            tracing::info!(
                target: "ventus::report",
                kind = "manual",
                message_len = message.chars().count(),
                "report requested"
            );
            if !crate::cloud::config::is_configured() {
                let _ = chrome.evaluate_script("window.__neura&&window.__neura.reportSent(false)");
                return None;
            }
            let report = build_report(state, "manual", message, String::new());
            Some(TabAction::SendReport(Box::new(report)))
        }
        ChromeCommand::TabAudioState {
            tab_id,
            playing,
            active,
        } => {
            if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.is_audio_playing = playing;
                tab.is_media_active = active;
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
        ChromeCommand::OpenFindBar => {
            let _ = chrome.focus();
            let _ = chrome.evaluate_script("openFindBar()");
            None
        }
        ChromeCommand::FindInPage {
            query,
            forward,
            tab_id,
        } => {
            let tab_id = tab_id.or_else(|| state.tab_manager.active_tab_id.clone());
            let Some(tab_id) = tab_id else {
                let _ = chrome.evaluate_script(
                    "window.__neura&&window.__neura.setFindResult({query:'',total:0,index:0})",
                );
                return None;
            };
            Some(TabAction::FindInPage {
                tab_id,
                query,
                forward,
            })
        }
        ChromeCommand::CloseSettings => {
            state.chrome_overlay_open = false;
            Some(TabAction::SyncViews)
        }
        ChromeCommand::SidebarToggle => {
            if matches!(
                state.settings.appearance.tab_layout,
                crate::config::TabLayout::Horizontal
            ) {
                return None;
            }
            // Auto-hide counts as "needs expanding" — toggle takes it to expanded,
            // not compact, which matches what the toolbar button and SC_SIDEBAR do.
            let is_compact_or_auto_hide = matches!(
                state.settings.appearance.sidebar_mode,
                crate::config::SidebarMode::Compact | crate::config::SidebarMode::AutoHide
            ) || state.sidebar_collapsed;
            let next = if is_compact_or_auto_hide {
                state.settings.appearance.sidebar_mode = crate::config::SidebarMode::Expanded;
                state.sidebar_collapsed = false;
                "expanded"
            } else {
                state.settings.appearance.sidebar_mode = crate::config::SidebarMode::Compact;
                state.sidebar_collapsed = true;
                "compact"
            };
            state.sidebar_auto_hide_open = false;
            state.sidebar_pinned = false;
            state.sidebar_clip_w_override = None;
            let _ = settings_store::set(&state.conn, "sidebar_mode", &next);
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            state.push_state_to_chrome(chrome);
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
        ChromeCommand::RestartForWebSecurity => Some(TabAction::ApplyWebSecurity),
        ChromeCommand::PwdSaveConfirm => {
            if let Some((origin, username, password)) = state.pending_pwd_save.take() {
                let _ = passwords::save(&state.conn, &state.pwd_key, &origin, &username, &password);
            }
            let _ = chrome.evaluate_script("window.__neura && window.__neura.hideSavePassword()");
            None
        }
        ChromeCommand::PwdSaveDismiss => {
            state.pending_pwd_save = None;
            let _ = chrome.evaluate_script("window.__neura && window.__neura.hideSavePassword()");
            None
        }
        ChromeCommand::PwdList => {
            push_passwords_list(state, chrome);
            None
        }
        ChromeCommand::PwdReveal { id } => {
            let pw = passwords::list(&state.conn, &state.pwd_key)
                .unwrap_or_default()
                .into_iter()
                .find(|c| c.id == id)
                .map(|c| c.password)
                .unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.revealPassword({}, {})",
                serde_json::to_string(&id).unwrap_or_default(),
                serde_json::to_string(&pw).unwrap_or_default()
            ));
            None
        }
        ChromeCommand::PwdDelete { id } => {
            let _ = passwords::delete(&state.conn, &id);
            push_passwords_list(state, chrome);
            None
        }
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
        ChromeCommand::GetHistoryPage { q, offset } => {
            const PAGE: i64 = 100;
            let mut entries =
                repositories::list_history_page(&state.conn, &q, offset, PAGE).unwrap_or_default();
            let has_more = entries.len() as i64 == PAGE;
            if let Ok(favmap) = repositories::all_favicons(&state.conn) {
                for e in entries.iter_mut() {
                    let domain = crate::utils::url::extract_domain(&e.url);
                    e.favicon = favmap.get(&domain).cloned();
                }
            }
            let json = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setHistoryPage({}, {}, {})",
                json, offset, has_more
            ));
            None
        }
        ChromeCommand::OmniboxSuggest { q } => {
            let items =
                crate::browser::omnibox::suggest(&state.conn, &state.omnibox, &q, &state.trends, 8);
            let payload = serde_json::json!({ "q": q, "items": items }).to_string();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setOmnibox({})",
                payload
            ));
            None
        }
        ChromeCommand::OmniboxPick { q, url, shown } => {
            crate::browser::omnibox::learn(&state.conn, &mut state.omnibox, &q, &url, &shown);
            None
        }
        ChromeCommand::OmniboxSetPref {
            url,
            pinned,
            blocked,
            q,
        } => {
            crate::browser::omnibox::set_pref(&state.conn, &url, pinned, blocked);
            let items =
                crate::browser::omnibox::suggest(&state.conn, &state.omnibox, &q, &state.trends, 8);
            let payload = serde_json::json!({ "q": q, "items": items }).to_string();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setOmnibox({})",
                payload
            ));
            None
        }
        ChromeCommand::RefreshTrends => None,
        ChromeCommand::DeleteHistoryEntry { id } => {
            let _ = repositories::delete_history_entry(&state.conn, id);
            None
        }
        ChromeCommand::DeleteHistoryDay { start, end } => {
            let _ = repositories::delete_history_range(&state.conn, start, end);
            None
        }
        ChromeCommand::OpenFile { path } => {
            if let Some(url) = pdf_file_url(&path) {
                let tab_id;
                {
                    let tab = state.tab_manager.new_tab(Some(&url));
                    tab_id = tab.id.clone();
                }
                state.push_state_to_chrome(chrome);
                return Some(TabAction::Create { tab_id, url });
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                // GUI apps have no console to inherit, so spawning cmd.exe makes Windows
                // allocate a fresh console window that flashes on screen. CREATE_NO_WINDOW
                // suppresses it — same flag the updater uses for its cmd invocation.
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                // Strip the Zone.Identifier ADS so Windows doesn't show the "trusted source"
                // security prompt — same operation as the "Unblock" checkbox in file Properties.
                let _ = std::fs::remove_file(format!("{}:Zone.Identifier", path));
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "", &path])
                    .creation_flags(CREATE_NO_WINDOW)
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
            if matches!(
                state.settings.appearance.tab_layout,
                crate::config::TabLayout::Horizontal
            ) {
                state.sidebar_auto_hide_open = false;
                state.sidebar_pinned = false;
                state.sidebar_clip_w_override = None;
                return None;
            }
            let was_pinned = state.sidebar_pinned;
            state.sidebar_auto_hide_open = visible;
            state.sidebar_pinned = visible && pinned;
            // Pinning/unpinning changes the solid sidebar width, so the content WebViews
            // must be repositioned and resized (full layout pass). A pure hover-peek (no
            // change in pin state) only needs the chrome clip column updated.
            if state.sidebar_pinned != was_pinned {
                Some(TabAction::SyncViews)
            } else {
                Some(TabAction::SyncSidebarClip)
            }
        }
        ChromeCommand::SidebarClipWidth { w } => {
            if matches!(
                state.settings.appearance.tab_layout,
                crate::config::TabLayout::Horizontal
            ) {
                state.sidebar_clip_w_override = None;
                return None;
            }
            // JS streams the sliding sidebar's live edge so the clip column + content
            // cut track it (no dark gap). `w < 0` ends the animation → normal clip.
            state.sidebar_clip_w_override = if w < 0.0 { None } else { Some(w.max(0.0)) };
            Some(TabAction::SyncSidebarClip)
        }
        ChromeCommand::SidebarAutoClose => {
            if matches!(
                state.settings.appearance.tab_layout,
                crate::config::TabLayout::Horizontal
            ) {
                return None;
            }
            // Skip when pinned — pinned sidebar is solid and stays open
            if state.sidebar_auto_hide_open && !state.sidebar_pinned {
                let _ = chrome.evaluate_script(
                    "window.__neura&&window.__neura.closeSidebar&&window.__neura.closeSidebar()",
                );
            }
            None
        }
        ChromeCommand::DragEdgePeek => {
            let is_auto_hide = matches!(
                state.settings.appearance.sidebar_mode,
                crate::config::SidebarMode::AutoHide
            ) && matches!(
                state.settings.appearance.tab_layout,
                crate::config::TabLayout::Vertical
            );
            if is_auto_hide && !state.sidebar_auto_hide_open {
                let _ = chrome.evaluate_script(
                    "window.__neura&&window.__neura.openSidebar&&window.__neura.openSidebar()",
                );
            }
            None
        }
        ChromeCommand::SuggestionOverlay {
            owner,
            visible,
            x,
            y,
            width,
            height,
        } => {
            if visible && width > 0.0 && height > 0.0 {
                state.suggestion_overlay_rects.insert(
                    owner,
                    ChromeClipRect {
                        x,
                        y,
                        width,
                        height,
                    },
                );
            } else {
                state.suggestion_overlay_rects.remove(&owner);
            }
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
        ChromeCommand::ContextMenuSaveImage { url } => Some(TabAction::SaveImageAs { url }),
        ChromeCommand::CopyImage { url } => Some(TabAction::CopyImageToClipboard { url }),
        ChromeCommand::ClearDownloads => {
            let _ = repositories::clear_downloads(&state.conn);
            state.downloads.downloads.clear();
            state.download_samples.clear();
            state.push_state_to_chrome(chrome);
            Some(TabAction::DownloadCancelAll)
        }
        ChromeCommand::DeleteDownload { id } => {
            let _ = repositories::delete_download(&state.conn, &id);
            state.downloads.downloads.retain(|d| d.id != id);
            state.download_samples.remove(&id);
            state.push_state_to_chrome(chrome);
            Some(TabAction::DownloadControl {
                id,
                action: DownloadCtl::Cancel,
            })
        }
        ChromeCommand::PauseDownload { id } => Some(TabAction::DownloadControl {
            id,
            action: DownloadCtl::Pause,
        }),
        ChromeCommand::ResumeDownload { id } => Some(TabAction::DownloadControl {
            id,
            action: DownloadCtl::Resume,
        }),
        ChromeCommand::CancelDownload { id } => Some(TabAction::DownloadControl {
            id,
            action: DownloadCtl::Cancel,
        }),
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
            let clamped = (level.clamp(0.25, 3.0) * 10.0).round() / 10.0;
            state.settings.appearance.zoom_level = clamped;
            state.zoom_levels.clear();
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            state.push_state_to_chrome(chrome);
            Some(TabAction::SetZoomAll(clamped))
        }
        ChromeCommand::ZoomDelta { delta } => {
            let cur = state.settings.appearance.zoom_level;
            let next = ((cur + delta).clamp(0.25, 3.0) * 10.0).round() / 10.0;
            state.settings.appearance.zoom_level = next;
            state.zoom_levels.clear();
            let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setActiveZoom({})",
                next
            ));
            Some(TabAction::SetZoomAll(next))
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
        ChromeCommand::FetchCurrencyRates => None,
        ChromeCommand::BeginSpotlight => {
            if state.spotlight_open {
                state.spotlight_open = false;
                let _ = chrome.evaluate_script("hideSpotlight()");
                Some(TabAction::SyncViews)
            } else {
                state.spotlight_open = true;
                let _ = chrome.evaluate_script("spotlightOpen=false;showSpotlight()");
                Some(TabAction::FocusSpotlight)
            }
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
        ChromeCommand::OpenIncognito => {
            let existing = state
                .tab_manager
                .workspaces
                .iter()
                .find(|w| w.is_incognito)
                .map(|w| w.id.clone());
            let (tab_id, url) = if let Some(ws_id) = existing {
                state.tab_manager.switch_workspace(&ws_id);
                let tab = state.tab_manager.new_tab(None);
                (tab.id.clone(), tab.url.clone())
            } else {
                let ws_id = {
                    let ws =
                        state
                            .tab_manager
                            .add_workspace("Incognito", true, Some("🔐".to_string()));
                    ws.accent_color = "#6b7280".to_string();
                    ws.id.clone()
                };
                state.tab_manager.switch_workspace(&ws_id);
                match state.tab_manager.active_tab() {
                    Some(tab) => (tab.id.clone(), tab.url.clone()),
                    None => return Some(TabAction::SyncViews),
                }
            };
            state.push_state_to_chrome(chrome);
            Some(TabAction::Create { tab_id, url })
        }
        ChromeCommand::ContentPointerDown => {
            // A press landed in the live web page — dismiss chrome popovers that are
            // clipped to their own rect (currently the download panel) so they get
            // click-outside-to-close without the chrome covering the page.
            let _ = chrome.evaluate_script(
                "window.__neura&&window.__neura.onContentPointerDown&&window.__neura.onContentPointerDown()",
            );
            None
        }
        ChromeCommand::GetAccountState => {
            state.push_account(chrome);
            None
        }
        ChromeCommand::AuthSignOut => {
            state.clear_session();
            state.auth = None;
            state.user_profile = None;
            state.push_account(chrome);
            let _ = chrome
                .evaluate_script("window.__neura && window.__neura.authSuccess('Signed out')");
            None
        }
        _ => None,
    }
}

fn clear_transient_chrome(state: &mut AppState, chrome: &WebView) {
    state.chrome_overlay_open = false;
    state.spotlight_open = false;
    state.suggestion_overlay_rects.clear();
    state.sidebar_auto_hide_open = false;
    state.sidebar_pinned = false;
    state.sidebar_clip_w_override = None;
    let _ = chrome.evaluate_script(
        "window.__neura&&window.__neura.clearTransientUi&&window.__neura.clearTransientUi()",
    );
}

fn find_result_json(raw: &str) -> String {
    let fallback = serde_json::json!({"query":"","total":0,"index":0});
    let value = serde_json::from_str::<serde_json::Value>(raw).unwrap_or(fallback);
    let value = if value.is_object() {
        value
    } else {
        serde_json::json!({"query":"","total":0,"index":0})
    };
    serde_json::to_string(&value)
        .unwrap_or_else(|_| "{\"query\":\"\",\"total\":0,\"index\":0}".into())
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
    clear_https_upgrades(state, tab_id);
    state.native_loads.remove(tab_id);
    state.native_nav_ids.remove(tab_id);
    state.load_recoveries.remove(&load_key(tab_id, url));
    state.load_progress.remove(tab_id);
    state.tab_manager.set_tab_loading(tab_id, false);
    if let Some(tab) = state.tab_manager.get_tab_mut(tab_id) {
        tab.engine_can_back = None;
        tab.engine_can_forward = None;
    }
    let _ = chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
    state.push_state_to_chrome(chrome);
    true
}

fn clear_native_nav(state: &mut AppState, tab_id: &str) {
    state.native_loads.remove(tab_id);
    state.native_nav_ids.remove(tab_id);
}

fn clear_tab_recoveries(state: &mut AppState, tab_id: &str) {
    let prefix = format!("{}\n", tab_id);
    state
        .load_recoveries
        .retain(|key, _| !key.starts_with(&prefix));
}

fn begin_user_nav(state: &mut AppState, chrome: &WebView, tab_id: &str) {
    clear_transient_chrome(state, chrome);
    clear_native_nav(state, tab_id);
    clear_tab_recoveries(state, tab_id);
    state.load_progress.remove(tab_id);
    if state.tab_manager.active_tab_id.as_deref() == Some(tab_id) {
        state.set_content_cover(chrome, false);
    }
}

fn clear_loading_favicon(state: &mut AppState, tab_id: &str) {
    if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
        tab.favicon = None;
    }
}

fn navigate_current_tab(url: String, state: &mut AppState, chrome: &WebView) -> Option<TabAction> {
    navigate_current_tab_with_policy(url, state, chrome, state.settings.privacy.https_only)
}

fn navigate_current_tab_with_policy(
    url: String,
    state: &mut AppState,
    chrome: &WebView,
    https_only: bool,
) -> Option<TabAction> {
    if let Some(tab_id) = state.tab_manager.active_tab_id.clone() {
        begin_user_nav(state, chrome, &tab_id);
        let raw_url = resolve_navigation_url_with_policy(&url, state, false);
        let resolved_url = secure_nav_url_with_policy(&raw_url, https_only);
        track_https_upgrade(state, &tab_id, &raw_url, &resolved_url);
        state.tab_manager.visit_tab(&tab_id, &resolved_url, "");
        if finish_internal_nav(&tab_id, &resolved_url, state, chrome) {
            return Some(TabAction::ContentNavigate(resolved_url));
        }
        clear_loading_favicon(state, &tab_id);
        state.tab_manager.set_tab_loading(&tab_id, true);
        state.load_progress.insert(tab_id.clone(), 0.0);
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

fn is_neura_tab(state: &AppState, tab_id: &str) -> bool {
    state
        .tab_manager
        .get_tab(tab_id)
        .map(|t| t.is_neura_page())
        .unwrap_or(false)
}

fn commit_stack_nav(
    tab_id: &str,
    url: &str,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    if finish_internal_nav(tab_id, url, state, chrome) {
        return Some(TabAction::ContentNavigate(url.to_string()));
    }
    clear_loading_favicon(state, tab_id);
    state.tab_manager.set_tab_loading(tab_id, true);
    state.load_progress.insert(tab_id.to_string(), 0.0);
    state
        .pending_nav_urls
        .insert(tab_id.to_string(), url.to_string());
    let _ = chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
    state.push_state_to_chrome(chrome);
    Some(TabAction::ContentNavigate(url.to_string()))
}

fn reload_current_tab(state: &mut AppState, chrome: &WebView) -> Option<TabAction> {
    let tab_id = state.tab_manager.active_tab_id.clone()?;
    let current_url = state.tab_manager.get_tab(&tab_id)?.url.clone();
    let raw_url = error_page_target(&current_url)
        .map(|(url, _)| url)
        .unwrap_or(current_url);
    let url = secure_nav_url(&raw_url, state);
    track_https_upgrade(state, &tab_id, &raw_url, &url);
    if url.starts_with("neura://") || url.trim().is_empty() {
        return None;
    }
    begin_user_nav(state, chrome, &tab_id);
    state.tab_manager.replace_tab_nav(&tab_id, &url, &url);
    state.pending_nav_urls.insert(tab_id.clone(), url.clone());
    clear_loading_favicon(state, &tab_id);
    state.tab_manager.set_tab_loading(&tab_id, true);
    state.load_recoveries.remove(&load_key(&tab_id, &url));
    state.load_progress.insert(tab_id.clone(), 0.0);
    let _ = chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
    state.push_state_to_chrome(chrome);
    Some(TabAction::ReloadContent { tab_id, url })
}

fn show_error_page(
    tab_id: String,
    url: String,
    code: i32,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    tracing::warn!(
        target: "ventus::nav",
        url = %crate::utils::url::log_url(&url),
        code,
        "error page shown"
    );
    let page = error_page_url(&url, code);
    state.pending_nav_urls.remove(&tab_id);
    clear_https_upgrades(state, &tab_id);
    clear_native_nav(state, &tab_id);
    clear_tab_recoveries(state, &tab_id);
    state.load_progress.remove(&tab_id);
    state.nav_started_at.remove(&tab_id);
    state.set_content_cover(chrome, false);
    state
        .tab_manager
        .replace_tab_nav(&tab_id, &page, "Can't reach this page");
    if let Some(tab) = state.tab_manager.get_tab_mut(&tab_id) {
        tab.favicon = None;
        tab.status = crate::browser::tab::TabStatus::Error;
        tab.engine_can_back = None;
        tab.engine_can_forward = None;
    }
    let _ = chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
    state.push_state_to_chrome(chrome);
    Some(TabAction::ShowErrorPage { tab_id })
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

pub(crate) fn same_nav(expected: &str, actual: &str) -> bool {
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
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")) {
        return false;
    }
    state
        .tab_manager
        .get_tab(tab_id)
        .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
        .unwrap_or(false)
}

fn recover_loading_tab(
    tab_id: String,
    url: String,
    failed: bool,
    error_code: i32,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    let Some(tab) = state.tab_manager.get_tab(&tab_id) else {
        return None;
    };
    if tab.status != crate::browser::tab::TabStatus::Loading {
        return None;
    }
    if url.starts_with("neura://") || url.trim().is_empty() {
        return None;
    }
    let active = state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str());
    let tab_url = tab.url.clone();
    let pending = state
        .pending_nav_urls
        .get(&tab_id)
        .map(|expected| same_nav(expected, &url))
        .unwrap_or(false);
    let native_match = state
        .native_loads
        .get(&tab_id)
        .map(|expected| same_nav(expected, &url))
        .unwrap_or(false);
    if !tab_url.is_empty() && !same_nav(&tab_url, &url) && !pending && !native_match {
        return None;
    }
    let recover_url = if native_match {
        url
    } else if !tab_url.is_empty() && same_nav(&tab_url, &url) {
        tab_url
    } else {
        url
    };
    if native_match {
        state
            .pending_nav_urls
            .insert(tab_id.clone(), recover_url.clone());
    }
    let key = load_key(&tab_id, &recover_url);
    let pct = state.load_progress.get(&tab_id).copied().unwrap_or(0.0);
    let tries = state.load_recoveries.get(&key).copied().unwrap_or(0);
    let elapsed_ms = state
        .nav_started_at
        .get(&tab_id)
        .map(|t| t.elapsed().as_millis() as u64)
        .unwrap_or(0);
    let free_mb = crate::utils::sysinfo::available_memory_mb();
    tracing::info!(
        target: "ventus::nav",
        tab = %tab_id,
        url = %recover_url,
        failed,
        active,
        pct,
        tries,
        elapsed_ms,
        free_mb = if free_mb == u64::MAX { 0 } else { free_mb },
        in_flight = state.native_nav_ids.contains_key(&tab_id),
        "recover_loading_tab: stall/failure detected"
    );

    // A load that has reported real progress is alive and already painting. Heavy sites
    // (YouTube, GitHub, Gmail) routinely render well before their navigation's final
    // "completed" signal arrives — especially under memory pressure with several tabs
    // open, where that signal can lag many seconds. Never tear such a page down: just
    // stop the spinner and keep what loaded. This MUST include native loads — on Windows
    // every real web navigation is native, so excluding them (the previous behaviour)
    // meant every slow-but-fine page got reloaded/rebuilt and ultimately blanked.
    if !failed && pct >= 0.5 {
        return stop_failed_load(state, chrome, &tab_id, &key);
    }

    // Background tab still spinning: don't throw the in-flight load away. Dropping it
    // forces a full reload from scratch when the user returns (under even more pressure).
    // Stop tracking it but keep the WebView — it finishes loading on its own.
    if !active {
        return stop_failed_load(state, chrome, &tab_id, &key);
    }

    // A genuine navigation error (network / WebError status). Retry gently before doing
    // anything destructive: a soft reload, then a full rebuild, then give up WITHOUT
    // blanking the tab — WebView2 shows its own error page, never a black void.
    if failed {
        return show_error_page(tab_id, recover_url, error_code, state, chrome);
    }

    // Did the navigation ever commit a document? The injected script fires its first
    // progress ping (0.12) at document-start of whatever document loads, so ANY non-zero
    // progress means a renderer is alive and parsing — this holds even for redirect-heavy
    // sites (Gmail/accounts), because the very first committed document bumps progress
    // before the redirects resolve. pct still exactly 0 means nothing ever rendered: a
    // pure black screen.
    let committed = pct > 0.0;

    if !committed {
        // pct == 0 means no document has committed yet — but that alone does NOT mean the
        // controller is wedged. A heavy main document over a slow or cold connection
        // (YouTube with many tabs open, on an old machine) can take far longer than usual to
        // commit while its network request is very much alive. The decisive signal is whether
        // a NATIVE NAVIGATION is still in flight: `native_nav_ids` holds the tab between
        // NavigationStarting and NavigationCompleted/Failed.
        let in_flight = state.native_nav_ids.contains_key(&tab_id);

        // In flight => the connection is ALIVE. Rebuilding here would abort it (WebView2
        // reports WebErrorStatus::ConnectionAborted, status 9) and restart from scratch — on
        // a 6 s timer that is exactly what churned YouTube into a permanent "loading… failed,
        // try again" loop on slow machines (each rebuild aborts the slow load before it can
        // commit, then the next one aborts again). So while a nav is in flight we NEVER
        // rebuild: we only keep the connection and re-arm the watchdog. WebView2 will itself
        // fire NavigationCompleted (success) or NavigationFailed (a real error, which routes
        // through the `failed` retry ladder) — both clear `native_nav_ids` — so this waiting
        // resolves on its own. Only if it blows past a generous ceiling do we stop the
        // spinner, and even then we leave the live connection untouched (it commits when the
        // response finally arrives, or WebView2 shows its own timeout page — never a void we
        // created by aborting it ourselves).
        if in_flight {
            if tries < IN_FLIGHT_PATIENT_TRIES {
                state.load_recoveries.insert(key, tries + 1);
                tracing::info!(
                    target: "ventus::nav",
                    tab = %tab_id,
                    url = %recover_url,
                    tries,
                    "recover: uncommitted but navigation still in-flight — staying patient (NOT rebuilding, which would abort the live connection)"
                );
                return Some(TabAction::ExtendLoadWatch {
                    tab_id,
                    url: recover_url,
                });
            }
            tracing::info!(
                target: "ventus::nav",
                tab = %tab_id,
                url = %recover_url,
                tries,
                "recover: uncommitted but still in-flight past patience ceiling — stopping spinner, keeping the live connection (no rebuild)"
            );
            return stop_failed_load(state, chrome, &tab_id, &key);
        }

        // NOT in flight => the controller produced no navigation at all: genuinely wedged
        // (the original black-tab bug). Recreate it promptly, a few times, then stop the
        // spinner (the tab is never left a permanent black void).
        if tries < UNCOMMITTED_PATIENT_TRIES {
            state.load_recoveries.insert(key, tries + 1);
            state.load_progress.insert(tab_id.clone(), 0.0);
            tracing::info!(
                target: "ventus::nav",
                tab = %tab_id,
                url = %recover_url,
                tries,
                "recover: uncommitted with no navigation in-flight — controller wedged, rebuilding"
            );
            return Some(TabAction::RebuildContent {
                tab_id,
                url: recover_url,
            });
        }
        return stop_failed_load(state, chrome, &tab_id, &key);
    }

    // Committed but slow to reach "interactive" — the page IS alive, so be gentle and
    // non-destructive: a cheap visibility/repaint kick, then a soft in-place reload, then
    // (only if still wedged) one controller rebuild, then stop. Never blank a live page.
    if tries == 0 {
        state.load_recoveries.insert(key, 1);
        return Some(TabAction::NudgeContent {
            tab_id,
            url: recover_url,
        });
    }
    if tries == 1 {
        state.load_recoveries.insert(key, 2);
        state.load_progress.insert(tab_id.clone(), 0.0);
        return Some(TabAction::ReloadContent {
            tab_id,
            url: recover_url,
        });
    }
    if tries == 2 {
        state.load_recoveries.insert(key, 3);
        state.load_progress.insert(tab_id.clone(), 0.0);
        return Some(TabAction::RebuildContent {
            tab_id,
            url: recover_url,
        });
    }
    stop_failed_load(state, chrome, &tab_id, &key)
}

fn rebuild_black_tab(
    tab_id: String,
    url: String,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    let _ = chrome;
    let Some(tab) = state.tab_manager.get_tab(&tab_id) else {
        return None;
    };
    if tab.status != crate::browser::tab::TabStatus::Loading {
        return None;
    }
    if url.starts_with("neura://") || url.trim().is_empty() {
        return None;
    }
    // Only ever touch the active tab.
    if state.tab_manager.active_tab_id.as_deref() != Some(tab_id.as_str()) {
        return None;
    }
    let tab_url = tab.url.clone();
    let pending = state
        .pending_nav_urls
        .get(&tab_id)
        .map(|expected| same_nav(expected, &url))
        .unwrap_or(false);
    let native_match = state
        .native_loads
        .get(&tab_id)
        .map(|expected| same_nav(expected, &url))
        .unwrap_or(false);
    if !tab_url.is_empty() && !same_nav(&tab_url, &url) && !pending && !native_match {
        return None;
    }
    let recover_url = if native_match {
        url
    } else if !tab_url.is_empty() && same_nav(&tab_url, &url) {
        tab_url
    } else {
        url
    };
    // Committed (renderer alive and parsing) → NOT black. Leave it for the gentle stall path.
    let pct = state.load_progress.get(&tab_id).copied().unwrap_or(0.0);
    if pct > 0.0 {
        return None;
    }
    // If a native navigation is still in-flight (NavigationStarting fired, no completion
    // yet), the connection is alive and the main document is simply slow to commit.
    // Rebuilding here would abort that live connection (ConnectionAborted / status 9) — the
    // exact churn that left YouTube stuck black. The early probe must only ever recover a
    // controller that produced NO navigation at all; in-flight loads are left to the patient
    // stall path.
    if state.native_nav_ids.contains_key(&tab_id) {
        return None;
    }
    let key = load_key(&tab_id, &recover_url);
    let tries = state.load_recoveries.get(&key).copied().unwrap_or(0);
    if tries >= 1 {
        return None;
    }
    if native_match {
        state
            .pending_nav_urls
            .insert(tab_id.clone(), recover_url.clone());
    }
    state.load_recoveries.insert(key, tries + 1);
    state.load_progress.insert(tab_id.clone(), 0.0);
    tracing::info!(
        target: "ventus::nav",
        tab = %tab_id,
        url = %recover_url,
        "black-probe: active tab uncommitted (black) at early window — rebuilding controller now instead of waiting for full stall timeout"
    );
    Some(TabAction::RebuildContent {
        tab_id,
        url: recover_url,
    })
}

fn stop_failed_load(
    state: &mut AppState,
    chrome: &WebView,
    tab_id: &str,
    key: &str,
) -> Option<TabAction> {
    state.load_recoveries.remove(key);
    state.pending_nav_urls.remove(tab_id);
    state.native_loads.remove(tab_id);
    state.native_nav_ids.remove(tab_id);
    state.load_progress.remove(tab_id);
    state.nav_started_at.remove(tab_id);
    clear_https_upgrades(state, tab_id);
    state.tab_manager.set_tab_loading(tab_id, false);
    if state.tab_manager.active_tab_id.as_deref() != Some(tab_id) {
        state.push_state_to_chrome(chrome);
        return None;
    }
    clear_transient_chrome(state, chrome);
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
    let _ = chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
    state.set_content_cover(chrome, false);
    state.push_state_to_chrome(chrome);
    Some(TabAction::SyncViews)
}

fn drop_failed_load(
    state: &mut AppState,
    chrome: &WebView,
    tab_id: String,
    key: &str,
) -> Option<TabAction> {
    state.load_recoveries.remove(key);
    state.pending_nav_urls.remove(&tab_id);
    state.native_loads.remove(&tab_id);
    state.native_nav_ids.remove(&tab_id);
    state.load_progress.remove(&tab_id);
    state.nav_started_at.remove(&tab_id);
    clear_https_upgrades(state, &tab_id);
    state.tab_manager.set_tab_loading(&tab_id, false);
    if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
        clear_transient_chrome(state, chrome);
        let active = state
            .tab_manager
            .active_tab()
            .map(|t| (t.nav_back(), t.nav_forward()));
        if let Some((back, fwd)) = active {
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.updateNavState({},{},false)",
                back, fwd
            ));
        }
        let _ = chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
        state.set_content_cover(chrome, false);
    }
    state.push_state_to_chrome(chrome);
    Some(TabAction::DropContent { tab_id })
}

fn canceled_nav_status(status: i32) -> bool {
    matches!(
        status,
        WEB_ERROR_UNKNOWN | WEB_ERROR_CONNECTION_ABORTED | WEB_ERROR_OPERATION_CANCELED
    )
}

fn web_security_signature(
    settings: &AppSettings,
) -> (Option<String>, SecureDnsMode, bool, bool, bool, bool, bool) {
    (
        settings.privacy.secure_dns_endpoint(),
        settings.privacy.secure_dns_mode.clone(),
        settings.privacy.https_only,
        settings.privacy.block_third_party_cookies,
        settings.privacy.storage_partitioning,
        settings.privacy.fingerprint_protection,
        settings.privacy.strict_permissions,
    )
}

fn normalize_site_origin(origin: &str) -> Option<String> {
    let url = url::Url::parse(origin).ok()?;
    if url.scheme() != "https" && url.scheme() != "http" {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    Some(format!("{}://{}{}", url.scheme(), host, port))
}

fn tab_site_origin(url: &str) -> Option<String> {
    normalize_site_origin(url)
}

fn set_site_permission(
    origin: String,
    permission: String,
    value: String,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    let Some(origin) = normalize_site_origin(&origin) else {
        let _ = chrome.evaluate_script(
            "window.__neura && window.__neura.showError('This page has no site permissions')",
        );
        return None;
    };
    if !valid_site_permission_key(&permission) || !valid_site_permission_value(&value) {
        let _ = chrome.evaluate_script(
            "window.__neura && window.__neura.showError('Permission was not saved')",
        );
        return None;
    }
    let mut perms = state
        .settings
        .privacy
        .site_permissions
        .get(&origin)
        .cloned()
        .unwrap_or_else(SitePermissions::default);
    if !perms.set(&permission, &value) {
        let _ = chrome.evaluate_script(
            "window.__neura && window.__neura.showError('Permission was not saved')",
        );
        return None;
    }
    state
        .settings
        .privacy
        .site_permissions
        .insert(origin.clone(), perms);
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
    state.push_state_to_chrome(chrome);
    let _ = chrome
        .evaluate_script("window.__neura && window.__neura.showSuccess('Site permission saved')");
    let Some(tab_id) = state.tab_manager.active_tab_id.clone() else {
        return None;
    };
    let Some(tab) = state.tab_manager.get_tab(&tab_id) else {
        return None;
    };
    let url = tab.url.clone();
    if tab_site_origin(&url).as_deref() != Some(origin.as_str()) {
        return None;
    }
    Some(TabAction::RebuildContent { tab_id, url })
}

fn set_default_permission(
    permission: String,
    value: String,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    if !valid_site_permission_key(&permission) || !valid_site_permission_value(&value) {
        let _ = chrome.evaluate_script(
            "window.__neura && window.__neura.showError('Permission was not saved')",
        );
        return None;
    }
    if !state
        .settings
        .privacy
        .default_permissions
        .set(&permission, &value)
    {
        let _ = chrome.evaluate_script(
            "window.__neura && window.__neura.showError('Permission was not saved')",
        );
        return None;
    }
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
    state.push_state_to_chrome(chrome);
    let _ = chrome.evaluate_script(
        "window.__neura && window.__neura.showSuccess('Default permission saved')",
    );
    let tab_id = state.tab_manager.active_tab_id.clone()?;
    let tab = state.tab_manager.get_tab(&tab_id)?;
    let url = tab.url.clone();
    Some(TabAction::RebuildContent { tab_id, url })
}

pub fn build_report(
    state: &AppState,
    kind: &str,
    message: String,
    panic: String,
) -> crate::cloud::report::Report {
    let (uid, email) = state
        .auth
        .as_ref()
        .map(|a| (a.uid.clone(), a.email.clone()))
        .unwrap_or_default();
    crate::cloud::report::Report {
        kind: kind.to_string(),
        message,
        uid,
        email,
        app_version: crate::version::APP_VERSION.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        device_id: state.device_id.clone(),
        session_id: state.session_id.clone(),
        panic,
        context: report_context(state),
        system: crate::utils::sysinfo::summary_json(),
        logs: crate::utils::log_buffer::snapshot(crate::cloud::report::MAX_LOGS),
    }
}

fn log_feature_command(cmd: &ChromeCommand, state: &AppState) {
    let Some(action) = cmd.report_name() else {
        return;
    };
    let active = state
        .tab_manager
        .active_tab()
        .map(|tab| crate::utils::url::log_url(&tab.url))
        .unwrap_or_default();
    tracing::info!(
        target: "ventus::feature",
        action = action,
        active = %active,
        "feature action"
    );
}

fn report_context(state: &AppState) -> String {
    let tab = state.tab_manager.active_tab();
    let ws = state.tab_manager.active_workspace();
    let active_tabs = state.tab_manager.active_workspace_tabs().len();
    let pending_tools = state
        .ai_pending_tools
        .lock()
        .map(|m| m.len())
        .unwrap_or_default();
    let downloads_active = state
        .downloads
        .downloads
        .iter()
        .filter(|d| {
            matches!(
                d.status,
                crate::browser::downloads::DownloadStatus::Pending
                    | crate::browser::downloads::DownloadStatus::Downloading
                    | crate::browser::downloads::DownloadStatus::Paused
            )
        })
        .count();
    let loading_tabs = state
        .tab_manager
        .tabs
        .iter()
        .filter(|t| t.status == crate::browser::tab::TabStatus::Loading)
        .count();
    let sleeping_tabs = state.tab_manager.tabs.iter().filter(|t| t.sleeping).count();
    serde_json::json!({
        "active": {
            "tab_id": state.tab_manager.active_tab_id.clone(),
            "url": tab.map(|t| crate::utils::url::log_url(&t.url)).unwrap_or_default(),
            "title": tab.map(|t| trim_report_text(&t.title, 180)).unwrap_or_default(),
            "loading": tab.map(|t| t.status == crate::browser::tab::TabStatus::Loading).unwrap_or(false),
            "sleeping": tab.map(|t| t.sleeping).unwrap_or(false),
            "workspace_id": state.tab_manager.active_workspace_id.clone(),
            "workspace_name": ws.map(|w| trim_report_text(&w.name, 80)).unwrap_or_default(),
            "incognito": ws.map(|w| w.is_incognito).unwrap_or(false),
        },
        "counts": {
            "tabs": state.tab_manager.tabs.len(),
            "active_workspace_tabs": active_tabs,
            "workspaces": state.tab_manager.workspaces.len(),
            "bookmarks": state.cached_bookmarks.len(),
            "bookmark_folders": state.cached_bookmark_folders.len(),
            "history_cached": state.cached_history.len(),
            "downloads": state.downloads.downloads.len(),
            "downloads_active": downloads_active,
            "pending_navs": state.pending_nav_urls.len(),
            "native_loads": state.native_loads.len(),
            "ai_pending_tools": pending_tools,
        },
        "ui": {
            "sidebar_collapsed": state.sidebar_collapsed,
            "sidebar_pinned": state.sidebar_pinned,
            "sidebar_auto_hide_open": state.sidebar_auto_hide_open,
            "ai_sidebar_open": state.ai_sidebar_open,
            "spotlight_open": state.spotlight_open,
            "content_cover_open": state.content_cover_open,
            "content_fullscreen": state.content_fullscreen,
        },
        "ai": {
            "provider": state.settings.ai.default_provider.clone(),
            "model": state.settings.ai.default_model.clone(),
            "temperature": state.settings.ai.temperature,
            "max_tokens": state.settings.ai.max_tokens,
            "reasoning_effort": state.settings.ai.reasoning_effort.clone(),
            "responses_api": state.settings.ai.openai_use_responses_api,
            "messages": state.ai_messages.len(),
        },
        "privacy": {
            "ad_blocker": state.settings.privacy.ad_blocker_enabled,
            "ad_blocker_kills": state.adblock_page_kills,
            "https_only": state.settings.privacy.https_only,
            "strict_permissions": state.settings.privacy.strict_permissions,
            "fingerprint_protection": state.settings.privacy.fingerprint_protection,
            "auto_crash_report": state.settings.privacy.auto_crash_report,
            "site_permission_origins": state.settings.privacy.site_permissions.len(),
            "requested_permission_origins": state.requested_permissions.len(),
        },
        "update": {
            "pending_version": state.pending_update_version.clone(),
            "has_pending_url": state.pending_update_url.is_some(),
        },
        "runtime": {
            "loading_tabs": loading_tabs,
            "sleeping_tabs": sleeping_tabs,
            "navs_in_flight": state.native_nav_ids.len(),
            "loads_recovering": state.load_recoveries.len(),
            "https_upgrades_pending": state.https_upgrades.len(),
            "downloads_sampling": state.download_samples.len(),
        },
        "memory": {
            "avail_mb": mb_or_null(crate::utils::sysinfo::available_memory_mb()),
            "total_mb": crate::utils::sysinfo::total_memory_mb(),
        },
        "session": {
            "id": state.session_id.clone(),
            "signed_in": state.auth.is_some(),
        },
    })
    .to_string()
}

fn mb_or_null(mb: u64) -> serde_json::Value {
    if mb == u64::MAX {
        serde_json::Value::Null
    } else {
        serde_json::Value::from(mb)
    }
}

fn ai_session_title(text: &str) -> String {
    let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        "New Chat".to_string()
    } else if t.chars().count() > 60 {
        let s: String = t.chars().take(57).collect();
        format!("{}…", s)
    } else {
        t
    }
}

fn ai_sessions_json(conn: &rusqlite::Connection) -> String {
    let sessions = crate::ai::chat::list_sessions(conn, 100).unwrap_or_default();
    let arr: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "title": s.title,
                "provider": s.provider,
                "model": s.model,
                "updated_at": s.updated_at,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn ai_messages_json(messages: &[crate::ai::ChatMessage]) -> String {
    use crate::ai::provider::ChatRole;
    let arr: Vec<serde_json::Value> = messages
        .iter()
        .filter(|m| matches!(m.role, ChatRole::User | ChatRole::Assistant))
        .map(|m| {
            let role = if matches!(m.role, ChatRole::Assistant) {
                "assistant"
            } else {
                "user"
            };
            serde_json::json!({
                "role": role,
                "content": m.content,
                "attachments": crate::ai::attachments::metadata_json(&m.attachments),
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn persist_ai_exchange(
    state: &AppState,
    session_id: &str,
    title: &str,
    user_text: &str,
    assistant_text: &str,
    attachments: Vec<crate::ai::AiAttachment>,
) {
    let conn = &state.conn;
    let now = chrono::Utc::now().timestamp_millis();
    let existing_title: Option<String> = conn
        .query_row(
            "SELECT title FROM ai_chat_sessions WHERE id=?1",
            rusqlite::params![session_id],
            |r| r.get(0),
        )
        .ok();
    let session = crate::ai::chat::ChatSession {
        id: session_id.to_string(),
        title: existing_title.unwrap_or_else(|| title.to_string()),
        provider: state.settings.ai.default_provider.clone(),
        model: state.settings.ai.default_model.clone(),
        page_url: None,
        created_at: now,
        updated_at: now,
    };
    if let Err(e) = crate::ai::chat::save_session(conn, &session) {
        tracing::warn!("persist_ai_exchange: save_session failed: {}", e);
        return;
    }
    let _ = crate::ai::chat::save_message(
        conn,
        session_id,
        &crate::ai::ChatMessage::user_with_attachments(user_text.to_string(), attachments),
    );
    let _ = crate::ai::chat::save_message(
        conn,
        session_id,
        &crate::ai::ChatMessage::assistant(assistant_text.to_string()),
    );
}

fn push_ai_attachments(chrome: &WebView, attachments: &[crate::ai::AiAttachment]) {
    let payload = crate::ai::attachments::metadata_json(attachments);
    let _ = chrome.evaluate_script(&format!(
        "window.__neura&&window.__neura.setAiAttachments({payload})"
    ));
}

fn trim_report_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push_str("...");
    out
}

fn permission_decision(
    origin: String,
    permission: String,
    decision: String,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    let allow = decision == "allow";
    let remember = decision == "allow" || decision == "block";
    if remember {
        if let Some(norm) = normalize_site_origin(&origin) {
            if valid_site_permission_key(&permission) {
                let mut perms = state
                    .settings
                    .privacy
                    .site_permissions
                    .get(&norm)
                    .cloned()
                    .unwrap_or_default();
                if perms.set(&permission, if allow { "allow" } else { "block" }) {
                    state.settings.privacy.site_permissions.insert(norm, perms);
                    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
                    state.push_state_to_chrome(chrome);
                }
            }
        }
    }
    Some(TabAction::ResolvePermission {
        origin,
        key: permission,
        allow,
    })
}

fn push_passwords_list(state: &AppState, chrome: &WebView) {
    let creds = passwords::list(&state.conn, &state.pwd_key).unwrap_or_default();
    let items: Vec<serde_json::Value> = creds
        .iter()
        .map(|c| serde_json::json!({"id": c.id, "origin": c.origin, "username": c.username}))
        .collect();
    let json = serde_json::to_string(&items).unwrap_or_default();
    let _ = chrome.evaluate_script(&format!(
        "window.__neura && window.__neura.setPasswords({})",
        json
    ));
}

pub fn handle_app_event_inner(
    event: AppEvent,
    state: &mut AppState,
    chrome: &WebView,
) -> Option<TabAction> {
    match event {
        AppEvent::Chrome(cmd) => handle_chrome_command(cmd, state, chrome),
        AppEvent::FindResult { tab_id, result } => {
            if state.tab_manager.active_tab_id.as_deref() != Some(tab_id.as_str()) {
                return None;
            }
            let payload = find_result_json(&result);
            let _ = chrome.evaluate_script(&format!(
                "window.__neura&&window.__neura.setFindResult({})",
                payload
            ));
            None
        }
        AppEvent::SaveSession { .. } => None,
        AppEvent::PermissionRequested { origin, key } => {
            if origin.is_empty() || !valid_site_permission_key(&key) {
                return None;
            }
            let added = state
                .requested_permissions
                .entry(origin)
                .or_default()
                .insert(key);
            if added {
                state.push_state_to_chrome(chrome);
            }
            None
        }
        AppEvent::ReportSent { ok } => {
            tracing::info!(target: "ventus::report", ok, "report finished");
            let js = if ok {
                "window.__neura&&window.__neura.reportSent(true)"
            } else {
                "window.__neura&&window.__neura.reportSent(false)"
            };
            let _ = chrome.evaluate_script(js);
            None
        }
        AppEvent::PermissionPrompt { id, origin, key } => {
            if origin.is_empty() || !valid_site_permission_key(&key) {
                return None;
            }
            tracing::info!(
                target: "ventus::permissions",
                origin = %origin,
                key = %key,
                "permission prompt"
            );
            let _ = chrome.evaluate_script(&format!(
                "window.__neura&&window.__neura.showPermissionPrompt({},{},{})",
                serde_json::to_string(&id).unwrap_or_default(),
                serde_json::to_string(&origin).unwrap_or_default(),
                serde_json::to_string(&key).unwrap_or_default(),
            ));
            None
        }
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
        AppEvent::ContentLoadStart {
            tab_id,
            url,
            native,
            nav_id,
        } => {
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            let track_url = !clean_url.trim().is_empty()
                && clean_url != "about:blank"
                && !clean_url.starts_with("neura://");
            let cur = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| tab.url.clone())
                .unwrap_or_default();
            let was_loading = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
                .unwrap_or(false);
            let same_id =
                native && nav_id != 0 && state.native_nav_ids.get(&tab_id).copied() == Some(nav_id);
            let redirect_start = state
                .pending_nav_urls
                .get(&tab_id)
                .map(|expected| !same_nav(expected, &clean_url) && same_id)
                .unwrap_or(false);
            let pending_match = state
                .pending_nav_urls
                .get(&tab_id)
                .map(|expected| same_nav(expected, &clean_url))
                .unwrap_or(false);
            let native_match = state
                .native_loads
                .get(&tab_id)
                .map(|expected| same_nav(expected, &clean_url))
                .unwrap_or(false);
            let current_match = track_url && !cur.is_empty() && same_nav(&cur, &clean_url);
            if let Some(expected) = state.pending_nav_urls.get(&tab_id) {
                if !same_nav(expected, &clean_url) && !redirect_start {
                    return None;
                }
            }
            let same_doc = track_url
                && !cur.is_empty()
                && cur != clean_url
                && current_match
                && !state.pending_nav_urls.contains_key(&tab_id);
            if same_doc {
                return None;
            }
            if native
                && track_url
                && was_loading
                && (current_match || pending_match || native_match)
            {
                state.native_loads.insert(tab_id.clone(), clean_url);
                if nav_id != 0 {
                    state.native_nav_ids.insert(tab_id, nav_id);
                }
                return None;
            }
            let new_url = track_url && !redirect_start && !current_match;
            if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.is_audio_playing = false;
                tab.is_media_active = false;
                tab.is_muted = false;
            }
            if new_url {
                state
                    .pending_nav_urls
                    .insert(tab_id.clone(), clean_url.clone());
                state.tab_manager.visit_tab(&tab_id, &clean_url, "");
            }
            if native && track_url {
                state.native_loads.insert(tab_id.clone(), clean_url.clone());
                if nav_id != 0 {
                    state.native_nav_ids.insert(tab_id.clone(), nav_id);
                }
            }
            // Clear per-tab history tracker so a reload / re-navigate to the same URL
            // produces a new entry. Non-native (SPA) load-starts also clear it so that
            // a pushState to a new URL is treated as a fresh navigation.
            if track_url {
                state.history_last_saved.remove(&tab_id);
            }
            clear_loading_favicon(state, &tab_id);
            state.tab_manager.set_tab_loading(&tab_id, true);
            if new_url || !was_loading {
                state.load_progress.insert(tab_id.clone(), 0.0);
                state
                    .nav_started_at
                    .insert(tab_id.clone(), std::time::Instant::now());
            } else {
                state.load_progress.entry(tab_id.clone()).or_insert(0.0);
                state
                    .nav_started_at
                    .entry(tab_id.clone())
                    .or_insert_with(std::time::Instant::now);
            }
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let active = state
                    .tab_manager
                    .active_tab()
                    .map(|t| (t.nav_back(), t.nav_forward()));
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
        AppEvent::ContentLoadEnd {
            tab_id,
            url,
            start_url,
            nav_id,
        } => {
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            let clean_start = crate::utils::url::clean_tracking_url(&start_url);
            let same_id = match state.native_nav_ids.get(&tab_id).copied() {
                Some(id) if nav_id == 0 || id != nav_id => return None,
                Some(_) => true,
                None => false,
            };
            if let Some(expected) = state.native_loads.get(&tab_id) {
                if !clean_start.trim().is_empty()
                    && !same_id
                    && !same_nav(expected, &clean_start)
                    && !same_nav(expected, &clean_url)
                {
                    return None;
                }
            }
            if let Some(expected) = state.pending_nav_urls.get(&tab_id) {
                let start_match =
                    !clean_start.trim().is_empty() && same_nav(expected, &clean_start);
                let end_match = same_nav(expected, &clean_url);
                if !start_match && !end_match && !same_id {
                    return None;
                }
                if !end_match {
                    if !can_accept_redirect(state, &tab_id, &clean_url) {
                        return None;
                    }
                    state.pending_nav_urls.remove(&tab_id);
                    state
                        .tab_manager
                        .replace_tab_nav(&tab_id, &clean_url, &clean_url);
                } else {
                    state.pending_nav_urls.remove(&tab_id);
                }
            }
            let url_mismatch = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| !same_nav(&tab.url, &clean_url))
                .unwrap_or(true);
            if url_mismatch {
                if !can_accept_redirect(state, &tab_id, &clean_url) {
                    return None;
                }
                state.pending_nav_urls.remove(&tab_id);
                state
                    .tab_manager
                    .replace_tab_nav(&tab_id, &clean_url, &clean_url);
            }
            if let Some(tab) = state.tab_manager.get_tab(&tab_id) {
                state.load_recoveries.remove(&load_key(&tab_id, &tab.url));
            }
            state.native_loads.remove(&tab_id);
            state.native_nav_ids.remove(&tab_id);
            state.load_progress.remove(&tab_id);
            if let Some(started) = state.nav_started_at.remove(&tab_id) {
                let dur_ms = started.elapsed().as_millis() as u64;
                let free_mb = crate::utils::sysinfo::available_memory_mb();
                tracing::info!(
                    target: "ventus::nav",
                    tab = %tab_id,
                    url = %crate::utils::url::log_url(&clean_url),
                    duration_ms = dur_ms,
                    free_mb = if free_mb == u64::MAX { 0 } else { free_mb },
                    tabs = state.tab_manager.tabs.len(),
                    slow = dur_ms >= 8000,
                    "load complete"
                );
            }
            clear_https_upgrades(state, &tab_id);
            state.tab_manager.set_tab_loading(&tab_id, false);
            if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                let active = state
                    .tab_manager
                    .active_tab()
                    .map(|t| (t.nav_back(), t.nav_forward()));
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
        AppEvent::ContentLoadProgress {
            tab_id,
            url,
            progress,
        } => {
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            let native = state.native_loads.get(&tab_id).cloned();
            if let Some(expected) = state.pending_nav_urls.get(&tab_id) {
                let native_match = native
                    .as_deref()
                    .map(|url| same_nav(url, &clean_url))
                    .unwrap_or(false);
                if clean_url.trim().is_empty() || !same_nav(expected, &clean_url) && !native_match {
                    return None;
                }
            }
            if let Some(expected) = native.as_deref() {
                if clean_url.trim().is_empty() || !same_nav(expected, &clean_url) {
                    return None;
                }
            }
            let tracked = state.load_progress.entry(tab_id.clone()).or_insert(0.0);
            *tracked = tracked.max(progress);
            let is_loading = state
                .tab_manager
                .get_tab(&tab_id)
                .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
                .unwrap_or(false);
            let active = state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str());
            if is_loading && active {
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.setLoadProgress({:.3})",
                    progress.clamp(0.0, 1.0)
                ));
                if progress >= 0.65 {
                    state.set_content_cover(chrome, false);
                }
            }
            let done = progress >= 0.92
                && !clean_url.trim().is_empty()
                && clean_url != "about:blank"
                && is_loading
                && native.is_none();
            if done {
                let url_mismatch = state
                    .tab_manager
                    .get_tab(&tab_id)
                    .map(|tab| !same_nav(&tab.url, &clean_url))
                    .unwrap_or(true);
                if url_mismatch {
                    if !can_accept_redirect(state, &tab_id, &clean_url) {
                        return None;
                    }
                    state
                        .tab_manager
                        .replace_tab_nav(&tab_id, &clean_url, &clean_url);
                }
                state.pending_nav_urls.remove(&tab_id);
                state.native_loads.remove(&tab_id);
                state.native_nav_ids.remove(&tab_id);
                state.load_recoveries.remove(&load_key(&tab_id, &clean_url));
                state.load_progress.remove(&tab_id);
                if let Some(started) = state.nav_started_at.remove(&tab_id) {
                    let dur_ms = started.elapsed().as_millis() as u64;
                    let free_mb = crate::utils::sysinfo::available_memory_mb();
                    tracing::info!(
                        target: "ventus::nav",
                        tab = %tab_id,
                        url = %crate::utils::url::log_url(&clean_url),
                        duration_ms = dur_ms,
                        free_mb = if free_mb == u64::MAX { 0 } else { free_mb },
                        tabs = state.tab_manager.tabs.len(),
                        slow = dur_ms >= 8000,
                        "load complete"
                    );
                }
                clear_https_upgrades(state, &tab_id);
                state.tab_manager.set_tab_loading(&tab_id, false);
                if state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str()) {
                    let active = state
                        .tab_manager
                        .active_tab()
                        .map(|t| (t.nav_back(), t.nav_forward()));
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
                return None;
            }
            None
        }
        AppEvent::ContentLoadStalled { tab_id, url, .. } => {
            recover_loading_tab(tab_id, url, false, 0, state, chrome)
        }
        AppEvent::ContentBlackProbe { tab_id, url, .. } => {
            rebuild_black_tab(tab_id, url, state, chrome)
        }
        AppEvent::ContentNavigationFailed {
            tab_id,
            url,
            status,
            nav_id,
        } => {
            tracing::debug!("navigation failed for {} with status {}", url, status);
            let clean_url = crate::utils::url::clean_tracking_url(&url);
            let same_id = match state.native_nav_ids.get(&tab_id).copied() {
                Some(id) if nav_id == 0 || id != nav_id => return None,
                Some(_) => true,
                None => false,
            };
            if let Some(expected) = state.pending_nav_urls.get(&tab_id) {
                if !same_nav(expected, &clean_url) && !same_id {
                    return None;
                }
            }
            let key = load_key(&tab_id, &clean_url);
            if state.settings.privacy.https_only && state.https_upgrades.contains_key(&key) {
                return None;
            }
            if canceled_nav_status(status) {
                return stop_failed_load(state, chrome, &tab_id, &key);
            }
            let action =
                recover_loading_tab(tab_id.clone(), clean_url, true, status, state, chrome);
            state.native_loads.remove(&tab_id);
            state.native_nav_ids.remove(&tab_id);
            action
        }
        AppEvent::HttpsUpgradeFailed {
            tab_id,
            https_url,
            http_url,
        } => {
            if !state.settings.privacy.https_only {
                return None;
            }
            if !https_url.starts_with("https://") || !http_url.starts_with("http://") {
                return None;
            }
            let key = load_key(&tab_id, &https_url);
            let Some(upgraded_url) = state.https_upgrades.remove(&key) else {
                return None;
            };
            if !same_nav(&upgraded_url, &http_url) {
                return None;
            }
            tracing::warn!(
                target: "ventus::autolog",
                https = %crate::utils::url::log_url(&https_url),
                http = %crate::utils::url::log_url(&upgraded_url),
                "dangerous http warning shown"
            );
            let url = http_warning_url(&https_url, &upgraded_url);
            state.tab_manager.visit_tab(&tab_id, &url, "HTTPS warning");
            state.pending_nav_urls.remove(&tab_id);
            state.native_loads.remove(&tab_id);
            state.native_nav_ids.remove(&tab_id);
            state.load_recoveries.remove(&load_key(&tab_id, &https_url));
            state.load_progress.remove(&tab_id);
            state.tab_manager.set_tab_loading(&tab_id, false);
            let _ = chrome.evaluate_script("window.__neura && window.__neura.finishLoadProgress()");
            state.push_state_to_chrome(chrome);
            Some(TabAction::ContentNavigate(url))
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
            let native = state.native_loads.get(&tab_id).cloned();
            let native_match = native
                .as_deref()
                .map(|url| same_nav(url, &clean_url))
                .unwrap_or(false);
            if let Some(expected) = native.as_deref() {
                if !same_nav(expected, &clean_url) {
                    return None;
                }
            }
            let pending = match state.pending_nav_urls.get(&tab_id).cloned() {
                Some(expected) if same_nav(&expected, &clean_url) => {
                    if native.is_none() {
                        state.pending_nav_urls.remove(&tab_id);
                    }
                    true
                }
                Some(_) if native_match => true,
                Some(_) => {
                    return None;
                }
                None => false,
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
            let icon_for_store = icon.clone();
            if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.favicon = icon.clone();
            }
            if icon.is_some()
                && repositories::is_bookmarked(&state.conn, &clean_url).unwrap_or(false)
            {
                let _ = repositories::set_bookmark_favicon_for_url(
                    &state.conn,
                    &clean_url,
                    icon.as_deref(),
                );
                state.cached_bookmarks =
                    repositories::list_bookmarks(&state.conn).unwrap_or_default();
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
            // ── History recording ────────────────────────────────────────────────
            // ContentMetadata is the authoritative "page settled" signal on Windows
            // (ContentNav is #[cfg(not(windows))] and never fires here).  We save a
            // history row on the first metadata event for each navigation and update
            // the title if a better one arrives on a subsequent event for the same URL.
            let in_incognito = state.tab_manager.tab_is_incognito(&tab_id);
            if !state.settings.privacy.disable_history
                && !clean_url.starts_with("neura://")
                && !clean_url.starts_with("about:")
                && !in_incognito
            {
                if let Some(ic) = icon_for_store.as_deref() {
                    let domain = crate::utils::url::extract_domain(&clean_url);
                    let _ = repositories::set_favicon(&state.conn, &domain, ic);
                }
                let last = state.history_last_saved.get(&tab_id).cloned();
                let is_url_fallback = |t: &str, u: &str| {
                    t == u || t.starts_with("http://") || t.starts_with("https://")
                };
                if !replace {
                    match last {
                        Some((ref saved_url, saved_id)) if saved_url == &clean_url => {
                            // Same page, second metadata fire — upgrade URL-fallback title
                            if !is_url_fallback(&safe_title, &clean_url) {
                                let old_is_fallback = state
                                    .cached_history
                                    .iter()
                                    .find(|e| e.id == saved_id)
                                    .map(|e| is_url_fallback(&e.title, &e.url))
                                    .unwrap_or(false);
                                if old_is_fallback {
                                    let _ = repositories::update_history_title(
                                        &state.conn,
                                        saved_id,
                                        &safe_title,
                                    );
                                    if let Some(e) =
                                        state.cached_history.iter_mut().find(|e| e.id == saved_id)
                                    {
                                        e.title = safe_title.clone();
                                    }
                                }
                            }
                        }
                        _ => {
                            // First metadata for this navigation, or SPA pushState to new URL
                            if let Ok(row_id) = repositories::add_history(
                                &state.conn,
                                &clean_url,
                                &safe_title,
                                None,
                            ) {
                                state
                                    .history_last_saved
                                    .insert(tab_id.clone(), (clean_url.clone(), row_id));
                                state.cached_history.insert(
                                    0,
                                    repositories::HistoryEntry {
                                        id: row_id,
                                        url: clean_url.clone(),
                                        title: safe_title.clone(),
                                        workspace_id: None,
                                        visited_at: chrono::Utc::now().timestamp_millis(),
                                        favicon: None,
                                    },
                                );
                                state.cached_history.truncate(30);
                            }
                        }
                    }
                } else if let Some((ref saved_url, saved_id)) = last {
                    // replace=true (SPA replaceState / popstate back-fwd): update title only
                    if saved_url == &clean_url && !is_url_fallback(&safe_title, &clean_url) {
                        let old_is_fallback = state
                            .cached_history
                            .iter()
                            .find(|e| e.id == saved_id)
                            .map(|e| is_url_fallback(&e.title, &e.url))
                            .unwrap_or(false);
                        if old_is_fallback {
                            let _ = repositories::update_history_title(
                                &state.conn,
                                saved_id,
                                &safe_title,
                            );
                            if let Some(e) =
                                state.cached_history.iter_mut().find(|e| e.id == saved_id)
                            {
                                e.title = safe_title.clone();
                            }
                        }
                    }
                }
            }
            // ── End history recording ────────────────────────────────────────────
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
            tracing::warn!(
                target: "ventus::ai",
                message = %trim_report_text(&message, 220),
                "ai error"
            );
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
            tracing::info!(
                target: "ventus::ai",
                tool = %trim_report_text(&label, 120),
                "ai tool call"
            );
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
            attachments,
        } => {
            tracing::info!(
                target: "ventus::ai",
                user_chars = user_text.chars().count(),
                assistant_chars = assistant_text.chars().count(),
                "ai exchange saved"
            );
            let session_id = state
                .current_ai_session_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            persist_ai_exchange(
                state,
                &session_id,
                &ai_session_title(&user_text),
                &user_text,
                &assistant_text,
                attachments.clone(),
            );
            state.current_ai_session_id = Some(session_id);
            state
                .ai_messages
                .push(crate::ai::ChatMessage::user_with_attachments(
                    user_text,
                    attachments,
                ));
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
            tracing::warn!(
                target: "ventus::ai",
                message = %trim_report_text(&message, 220),
                "spotlight ai error"
            );
            let msg_js = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.spotlightAiError({})",
                msg_js
            ));
            None
        }
        AppEvent::DownloadStarted {
            id,
            url,
            filename,
            path,
            total,
        } => {
            tracing::info!(
                target: "ventus::dl",
                url = %crate::utils::url::log_url(&url),
                filename = %trim_report_text(&filename, 120),
                total = total.unwrap_or(0),
                "download added"
            );
            let mut dl = crate::browser::downloads::Download::new(&url, &filename);
            if let Some(id) = id {
                dl.id = id;
            }
            dl.local_path = Some(path);
            dl.total_bytes = total;
            dl.status = crate::browser::downloads::DownloadStatus::Downloading;
            let dl = state.downloads.add(dl).clone();
            state
                .download_samples
                .insert(dl.id.clone(), (dl.started_at, 0, 0));
            if let Err(e) = repositories::save_download(&state.conn, &dl) {
                tracing::warn!("save download start failed: {}", e);
            }
            let nav_stops: Vec<(String, String)> = state
                .native_loads
                .iter()
                .filter(|(_, u)| same_nav(u.as_str(), url.as_str()))
                .map(|(tid, u)| (tid.clone(), u.clone()))
                .collect();
            let had_nav_stop = !nav_stops.is_empty();
            for (tid, nav_url) in &nav_stops {
                let key = load_key(tid, nav_url);
                let _ = stop_failed_load(state, chrome, tid, &key);
            }
            let _ =
                chrome.evaluate_script("window.__neura && window.__neura.setDownloadActive(true)");
            state.push_state_to_chrome(chrome);
            let _ = chrome.evaluate_script(
                "window.__neura && window.__neura.flashDownloadStart && window.__neura.flashDownloadStart()",
            );
            if had_nav_stop {
                Some(TabAction::SyncViews)
            } else {
                None
            }
        }
        AppEvent::DownloadCompleted { url, path, success } => {
            tracing::info!(
                target: "ventus::dl",
                url = %crate::utils::url::log_url(&url),
                success,
                has_path = path.is_some(),
                "download completed event"
            );
            let mut done_id = None;
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
                done_id = Some(dl.id.clone());
                if let Err(e) = repositories::save_download(&state.conn, dl) {
                    tracing::warn!("save download completion failed: {}", e);
                }
            }
            if let Some(id) = done_id {
                state.download_samples.remove(&id);
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::DownloadProgress {
            id,
            received,
            total,
        } => {
            let speed = state.download_speed(&id, received);
            state.downloads.update_progress(&id, received, total);
            let total_js = match total {
                Some(t) => t.to_string(),
                None => "null".to_string(),
            };
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.updateDownload({},{},{},'downloading',{})",
                serde_json::to_string(&id).unwrap_or_default(),
                received,
                total_js,
                speed
            ));
            None
        }
        AppEvent::DownloadPaused { id } => {
            tracing::info!(target: "ventus::dl", id = %id, "download paused");
            state.downloads.pause(&id);
            state.download_samples.remove(&id);
            if let Some(dl) = state.downloads.find_mut(&id) {
                let _ = repositories::save_download(&state.conn, dl);
            }
            state.push_state_to_chrome(chrome);
            None
        }
        AppEvent::DownloadResume { id } => {
            tracing::info!(target: "ventus::dl", id = %id, "download resume");
            if let Some(dl) = state.downloads.find_mut(&id) {
                dl.status = crate::browser::downloads::DownloadStatus::Downloading;
            }
            Some(TabAction::DownloadControl {
                id,
                action: DownloadCtl::Resume,
            })
        }
        AppEvent::DownloadDone {
            id,
            success,
            canceled,
        } => {
            tracing::info!(
                target: "ventus::dl",
                id = %id,
                success,
                canceled,
                "download done"
            );
            state.download_samples.remove(&id);
            if let Some(dl) = state.downloads.find_mut(&id) {
                dl.status = if success {
                    crate::browser::downloads::DownloadStatus::Complete
                } else if canceled {
                    crate::browser::downloads::DownloadStatus::Cancelled
                } else {
                    crate::browser::downloads::DownloadStatus::Failed
                };
                dl.completed_at = Some(chrono::Utc::now().timestamp_millis());
                let _ = repositories::save_download(&state.conn, dl);
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
            tracing::info!(
                target: "ventus::update",
                available,
                version = %version,
                "update check result"
            );
            if available {
                state.pending_update_url = Some(download_url);
                state.pending_update_version = Some(version.clone());
                state.pending_update_notes = Some(notes.clone());
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
            tracing::warn!(
                target: "ventus::update",
                message = %trim_report_text(&message, 220),
                "update check failed"
            );
            let m = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setUpdateState({{status:'error',error:{}}})",
                m
            ));
            None
        }
        AppEvent::CurrencyRatesLoaded { rates } => {
            tracing::info!(target: "ventus::feature", action = "currency_rates_loaded", "feature result");
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setCurrencyRates({})",
                rates
            ));
            None
        }
        AppEvent::CurrencyRatesFailed => {
            tracing::warn!(target: "ventus::feature", action = "currency_rates_failed", "feature result");
            None
        }
        AppEvent::NeuraFeedLoaded { articles } => {
            let count = articles.as_array().map(|a| a.len()).unwrap_or_default();
            tracing::info!(target: "ventus::feed", articles = count, "neura feed loaded");
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setNeuraFeed({})",
                articles
            ));
            None
        }
        AppEvent::NeuraFeedFailed { message } => {
            tracing::warn!(
                target: "ventus::feed",
                message = %trim_report_text(&message, 220),
                "neura feed failed"
            );
            let m = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.setNeuraFeedError({})",
                m
            ));
            None
        }
        AppEvent::TrendsLoaded {
            region,
            trends,
            fetched_at,
        } => {
            let count = trends.len();
            tracing::info!(target: "ventus::omnibox", region = %region, trends = count, "trends loaded");
            state.trends = trends;
            state.trends_region = region;
            state.trends_fetched_at = fetched_at;
            state.trends_loading = false;
            let _ = chrome.evaluate_script(
                "window.__neura && window.__neura.refreshOmnibox && window.__neura.refreshOmnibox()",
            );
            None
        }
        AppEvent::TrendsFailed { region } => {
            tracing::warn!(target: "ventus::omnibox", region = %region, "trends failed");
            if state.trends_region == region || state.trends_region.is_empty() {
                state.trends_loading = false;
            }
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
            Some(TabAction::ApplyUpdate(path))
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
        AppEvent::ContentNavState { .. } => None,
        AppEvent::AuthApplied {
            session,
            profile,
            message,
        } => {
            tracing::info!(
                target: "ventus::auth",
                has_message = !message.is_empty(),
                "auth applied"
            );
            state.persist_session(&session, &profile);
            state.auth = Some(session);
            state.user_profile = Some(profile);
            state.push_account(chrome);
            if !message.is_empty() {
                let m = serde_json::to_string(&message).unwrap_or_default();
                let _ = chrome.evaluate_script(&format!(
                    "window.__neura && window.__neura.authSuccess({})",
                    m
                ));
            }
            None
        }
        AppEvent::AuthError { message } => {
            tracing::warn!(
                target: "ventus::auth",
                message = %trim_report_text(&message, 220),
                "auth error"
            );
            let m = serde_json::to_string(&message).unwrap_or_default();
            let _ = chrome.evaluate_script(&format!(
                "window.__neura && window.__neura.authError({})",
                m
            ));
            state.push_account(chrome);
            None
        }
        AppEvent::SyncPulled {
            bookmarks,
            history,
            settings,
        } => {
            tracing::info!(
                target: "ventus::sync",
                bookmarks = bookmarks.as_ref().map(|b| b.len()).unwrap_or_default(),
                history = history.as_ref().map(|h| h.len()).unwrap_or_default(),
                has_settings = settings.is_some(),
                "cloud sync pulled"
            );
            if let Some(b) = bookmarks {
                merge_cloud_bookmarks(state, &b);
            }
            if let Some(h) = history {
                merge_cloud_history(state, &h);
            }
            if let Some(s) = settings {
                apply_cloud_settings(state, &s);
            }
            state.cached_bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
            state.cached_bookmark_folders =
                repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
            state.cached_history = repositories::list_history(&state.conn, 30).unwrap_or_default();
            state.push_state_to_chrome(chrome);
            Some(TabAction::SyncViews)
        }
        _ => None,
    }
}

fn merge_cloud_bookmarks(state: &AppState, blob: &str) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(blob) else {
        return;
    };
    let have_folders: std::collections::HashSet<String> =
        repositories::list_bookmark_folders(&state.conn)
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.id)
            .collect();
    let have_urls: std::collections::HashSet<String> = repositories::list_bookmarks(&state.conn)
        .unwrap_or_default()
        .into_iter()
        .map(|b| b.url)
        .collect();
    // One transaction for the whole merge — same per-row-autocommit cost as history, just at a
    // smaller scale.
    let Ok(tx) = state.conn.unchecked_transaction() else {
        return;
    };
    if let Some(folders) = v["folders"].as_array() {
        for f in folders {
            let id = f["id"].as_str().unwrap_or_default();
            if id.is_empty() || have_folders.contains(id) {
                continue;
            }
            let _ = tx.execute(
                "INSERT OR IGNORE INTO bookmark_folders(id, name, parent_id, position, created_at) VALUES(?1,?2,?3,?4,?5)",
                rusqlite::params![
                    id,
                    f["name"].as_str().unwrap_or("Folder"),
                    f["parent_id"].as_str(),
                    f["position"].as_i64().unwrap_or(0),
                    f["created_at"].as_i64().unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
                ],
            );
        }
    }
    if let Some(items) = v["bookmarks"].as_array() {
        for b in items {
            let url = b["url"].as_str().unwrap_or_default();
            if url.is_empty() || have_urls.contains(url) {
                continue;
            }
            let id = b["id"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let favicon = b["favicon"]
                .as_str()
                .filter(|url| url.starts_with("http://") || url.starts_with("https://"));
            let _ = tx.execute(
                "INSERT OR IGNORE INTO bookmarks(id, url, title, favicon, folder_id, position, created_at, icon_only) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![
                    id,
                    url,
                    b["title"].as_str().unwrap_or(url),
                    favicon,
                    b["folder_id"].as_str(),
                    b["position"].as_i64().unwrap_or(0),
                    b["created_at"].as_i64().unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
                    b["icon_only"].as_bool().unwrap_or(false),
                ],
            );
        }
    }
    let _ = tx.commit();
}

fn merge_cloud_history(state: &AppState, blob: &str) {
    let Ok(mut items) = serde_json::from_str::<Vec<repositories::HistoryEntry>>(blob) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    // Local history is capped at HISTORY_LIMIT rows, so only the newest HISTORY_LIMIT cloud
    // entries can ever survive the trim below. The cloud blob, however, can grow into the tens
    // of thousands — feeding the whole thing in meant inserting ~24k rows and then immediately
    // deleting them again on EVERY sync, as thousands of separate auto-committed INSERTs on the
    // UI thread. That is the multi-second "everything is unresponsive after restore" stall.
    // Keep only the newest HISTORY_LIMIT, dedup against the full local table, and apply the
    // genuinely-missing rows in ONE transaction (≈100x fewer fsyncs; usually near-zero work
    // once local and cloud agree).
    items.sort_by(|a, b| b.visited_at.cmp(&a.visited_at));
    items.truncate(repositories::HISTORY_LIMIT as usize);
    let mut have: std::collections::HashSet<(String, i64)> =
        repositories::list_history(&state.conn, repositories::HISTORY_LIMIT as usize)
            .unwrap_or_default()
            .into_iter()
            .map(|e| (e.url, e.visited_at))
            .collect();
    let Ok(tx) = state.conn.unchecked_transaction() else {
        return;
    };
    let mut inserted = 0usize;
    for e in items {
        if e.url.is_empty() || !have.insert((e.url.clone(), e.visited_at)) {
            continue;
        }
        let _ = tx.execute(
            "INSERT INTO history(url, title, workspace_id, visited_at) VALUES(?1,?2,?3,?4)",
            rusqlite::params![e.url, e.title, e.workspace_id, e.visited_at],
        );
        inserted += 1;
    }
    if inserted > 0 {
        let _ = tx.execute(
            "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY visited_at DESC LIMIT ?1)",
            rusqlite::params![repositories::HISTORY_LIMIT],
        );
    }
    let _ = tx.commit();
    tracing::info!(target: "ventus::sync", inserted, "merge_cloud_history applied");
}

fn apply_cloud_settings(state: &mut AppState, blob: &str) {
    let Ok(settings) = serde_json::from_str::<AppSettings>(blob) else {
        return;
    };
    if state.settings.settings_rev > 0 && settings.settings_rev <= state.settings.settings_rev {
        return;
    }
    state.settings = settings;
    state.sidebar_collapsed = matches!(
        state.settings.appearance.sidebar_mode,
        crate::config::SidebarMode::Compact
    );
    state.sidebar_auto_hide_open = false;
    state.sidebar_pinned = false;
    state.sidebar_clip_w_override = None;
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
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

fn clear_https_upgrades(state: &mut AppState, tab_id: &str) {
    let prefix = format!("{}\n", tab_id);
    state
        .https_upgrades
        .retain(|key, _| !key.starts_with(&prefix));
}

fn track_https_upgrade(state: &mut AppState, tab_id: &str, before: &str, after: &str) {
    clear_https_upgrades(state, tab_id);
    if before == after {
        return;
    }
    let Ok(a) = url::Url::parse(before) else {
        return;
    };
    let Ok(b) = url::Url::parse(after) else {
        return;
    };
    if a.scheme() != "http" || b.scheme() != "https" {
        return;
    }
    state
        .https_upgrades
        .insert(load_key(tab_id, after), before.to_string());
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
    let sync_wallpaper_data = matches!(
        key.as_str(),
        "new_tab_wallpaper_source" | "new_tab_wallpaper_data"
    );
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
                let mut save_failed = false;
                for (provider, key_val) in obj {
                    if let Some(k) = key_val.as_str() {
                        if provider == "model" {
                            if !k.is_empty() {
                                state.settings.ai.default_model = k.to_string();
                            }
                        } else if !k.is_empty() {
                            if keychain::set_api_key(provider, k).is_err() {
                                save_failed = true;
                            }
                        }
                    }
                }
                if save_failed {
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.showError('Some API keys could not be saved')",
                    );
                } else {
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.showSuccess('API keys saved')",
                    );
                }
            }
        }
        "openai_base_url" => {
            if let Some(v) = value.as_str() {
                state.settings.ai.openai_base_url =
                    clean_ai_base_url(v, "https://api.openai.com/v1");
            }
        }
        "anthropic_base_url" => {
            if let Some(v) = value.as_str() {
                state.settings.ai.anthropic_base_url =
                    clean_ai_base_url(v, "https://api.anthropic.com/v1");
            }
        }
        "gemini_base_url" => {
            if let Some(v) = value.as_str() {
                state.settings.ai.gemini_base_url =
                    clean_ai_base_url(v, "https://generativelanguage.googleapis.com/v1beta");
            }
        }
        "openai_use_responses_api" => {
            if let Some(v) = value.as_bool() {
                state.settings.ai.openai_use_responses_api = v;
            }
        }
        "reasoning_effort" => {
            if let Some(v) = value.as_str() {
                state.settings.ai.reasoning_effort = clean_reasoning_effort(v).to_string();
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
        "https_only" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.https_only = v;
            }
        }
        "block_third_party_cookies" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.block_third_party_cookies = v;
            }
        }
        "storage_partitioning" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.storage_partitioning = v;
            }
        }
        "fingerprint_protection" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.fingerprint_protection = v;
            }
        }
        "strict_permissions" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.strict_permissions = v;
            }
        }
        "auto_crash_report" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.auto_crash_report = v;
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
        "secure_dns_enabled" => {
            if let Some(v) = value.as_bool() {
                state.settings.privacy.secure_dns_enabled = v;
            }
        }
        "secure_dns_provider" => {
            if let Some(v) = value.as_str() {
                state.settings.privacy.secure_dns_provider = SecureDnsProvider::from_id(v);
            }
        }
        "secure_dns_mode" => {
            if let Some(v) = value.as_str() {
                state.settings.privacy.secure_dns_mode = SecureDnsMode::from_id(v);
            }
        }
        "secure_dns_template" => {
            if let Some(v) = value.as_str() {
                if let Some(url) = crate::config::clean_doh_url(v) {
                    state.settings.privacy.secure_dns_template = url;
                } else {
                    let _ = chrome.evaluate_script(
                        "window.__neura && window.__neura.showError('Use a valid HTTPS DNS endpoint')",
                    );
                }
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
        "tab_layout" => {
            if let Some(v) = value.as_str() {
                state.settings.appearance.tab_layout = match v {
                    "horizontal" => crate::config::TabLayout::Horizontal,
                    _ => crate::config::TabLayout::Vertical,
                };
                state.sidebar_auto_hide_open = false;
                state.sidebar_pinned = false;
                state.sidebar_clip_w_override = None;
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
        "font_family" => {
            if let Some(v) = value.as_str() {
                state.settings.appearance.font_family = match v {
                    "segoe" | "aptos" | "rounded" | "serif" | "mono" => v.to_string(),
                    _ => "system".to_string(),
                };
            }
        }
        "toolbar_buttons" => {
            if let Some(arr) = value.as_array() {
                let buttons: Vec<String> = arr
                    .iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect();
                state.settings.appearance.toolbar_buttons = clean_toolbar_buttons(&buttons);
            }
        }
        "search_suggestions" => {
            if let Some(v) = value.as_bool() {
                state.settings.search.suggestions_enabled = v;
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
                let src = match v {
                    "daily" | "nature" | "url" | "upload" | "color" | "none" => v.to_string(),
                    _ => "daily".to_string(),
                };
                state.settings.new_tab.show_background = src != "none";
                state.settings.new_tab.wallpaper_source = src;
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
        "new_tab_wallpaper_data" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.wallpaper_data = v.to_string();
            }
        }
        "new_tab_font_color" => {
            if let Some(v) = value.as_str() {
                state.settings.new_tab.font_color = v.to_string();
            }
        }
        "region" => {
            if let Some(v) = value.as_str() {
                state.settings.region = v.to_string();
                state.trends.clear();
                state.trends_region.clear();
                state.trends_fetched_at = 0;
            }
        }
        _ => {
            let _ = settings_store::set(&state.conn, &key, &value);
        }
    }
    state.settings.settings_rev = chrono::Utc::now().timestamp_millis();
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
    state.push_state_to_chrome(chrome);
    if sync_wallpaper_data {
        state.push_newtab_wallpaper_to_chrome(chrome);
    }
}

fn ai_model_for(provider: &str) -> &'static str {
    match normalize_ai_provider(provider) {
        "anthropic" => "claude-3-5-sonnet-20241022",
        "gemini" => "gemini-3.5-flash",
        _ => "gpt-5.4-mini",
    }
}

fn normalize_ai_provider(provider: &str) -> &str {
    match provider {
        "openai_compatible" | "openai-compatible" | "openrouter" | "ollama" => "openai",
        "anthropic_compatible" | "anthropic-compatible" => "anthropic",
        "gemini_compatible" | "gemini-compatible" => "gemini",
        other => other,
    }
}

fn clean_ai_base_url(value: &str, default_url: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        default_url.to_string()
    } else {
        trimmed.to_string()
    }
}

fn clean_reasoning_effort(value: &str) -> &'static str {
    match value {
        "default" | "" => "default",
        "none" => "none",
        "minimal" => "minimal",
        "low" => "low",
        "high" => "high",
        "xhigh" | "x-high" | "x_high" => "xhigh",
        "medium" => "medium",
        _ => "default",
    }
}

fn ai_model_matches_provider(provider: &str, model: &str) -> bool {
    match ai_provider_for_model(model) {
        Some(model_provider) => model_provider == normalize_ai_provider(provider),
        None => !model.trim().is_empty(),
    }
}

fn ai_provider_for_model(model: &str) -> Option<&'static str> {
    let model = model.trim();
    if model.is_empty() {
        return None;
    }
    if model.starts_with("claude-") {
        Some("anthropic")
    } else if model.starts_with("gemini-") || model.starts_with("models/gemini-") {
        Some("gemini")
    } else if model.starts_with("gpt-")
        || model.starts_with("o")
        || model.starts_with("chatgpt-")
        || model.starts_with("openai/")
    {
        Some("openai")
    } else {
        None
    }
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
    resolve_navigation_url_with_policy(input, state, state.settings.privacy.https_only)
}

fn resolve_navigation_url_with_policy(input: &str, state: &AppState, https_only: bool) -> String {
    if state.settings.search.site_shortcuts_enabled {
        if let Ok(engines) = repositories::list_search_engines(&state.conn) {
            if let Some((_, url)) = crate::browser::search_engine::SearchEngine::resolve_shortcut(
                input.trim(),
                &engines,
            ) {
                return secure_nav_url_with_policy(&url, https_only);
            }
        }
    }
    let search_url = get_search_url(state);
    let url = crate::browser::navigation::resolve_input(input, &search_url).url;
    secure_nav_url_with_policy(&url, https_only)
}

fn secure_nav_url(url: &str, state: &AppState) -> String {
    secure_nav_url_with_policy(url, state.settings.privacy.https_only)
}

fn secure_nav_url_with_policy(url: &str, https_only: bool) -> String {
    if !https_only {
        return url.to_string();
    }
    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_string();
    };
    if parsed.scheme() != "http" {
        return url.to_string();
    }
    if is_local_http_host(parsed.host_str()) {
        return url.to_string();
    }
    if parsed.set_scheme("https").is_err() {
        return url.to_string();
    }
    parsed.to_string()
}

fn pdf_file_url(path: &str) -> Option<String> {
    let path = path.trim().trim_matches('"');
    if path.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(path);
    let is_pdf = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    if !is_pdf || !path.exists() {
        return None;
    }
    let path = path.canonicalize().unwrap_or(path);
    url::Url::from_file_path(path)
        .ok()
        .map(|url| url.to_string())
}

fn is_local_http_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "0.0.0.0" | "::1")
}

fn http_warning_url(https_url: &str, http_url: &str) -> String {
    format!(
        "neura://http-warning?https={}&target={}",
        query_encode(https_url),
        query_encode(http_url)
    )
}

fn error_page_url(url: &str, code: i32) -> String {
    format!("neura://error?url={}&code={}", query_encode(url), code)
}

fn error_page_target(url: &str) -> Option<(String, i32)> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "neura" || parsed.host_str() != Some("error") {
        return None;
    }
    let mut target = String::new();
    let mut code = 0;
    for (key, value) in parsed.query_pairs() {
        if key == "url" {
            target = value.into_owned();
        } else if key == "code" {
            code = value.parse().unwrap_or(0);
        }
    }
    if target.trim().is_empty() {
        return None;
    }
    Some((target, code))
}

fn query_encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
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
