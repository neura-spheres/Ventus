#[derive(Clone, Copy)]
struct LayoutConfig {
    sidebar_expanded_w: u32,
    sidebar_collapsed_w: u32,
    horizontal_tabs_h: u32,
    toolbar_h: u32,
    ai_sidebar_w: u32,
    min_content_w: u32,
    min_ai_sidebar_w: u32,
    app_sidebar_w: u32,
    app_header_h: u32,
}

#[derive(Clone, Copy)]
struct AppLayout {
    window_w: u32,
    window_h: u32,
    scale_factor: f64,
    clip_sidebar_w: u32,
    toolbar_h: u32,
    ai_w: u32,
    sidebar_css_w: f64,
    toolbar_css_h: f64,
    ai_css_w: f64,
    app_w: u32,
    app_css_w: f64,
    app_header_h: u32,
    app_body_chrome_owned: bool,
    app_panel_body: Option<Rect>,
    frame_side_w: u32,
    frame_bottom_h: u32,
    frame_side_css: f64,
    frame_bottom_css: f64,
    content: Rect,
}

impl AppLayout {
    fn calculate(
        size: tao::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        state: &AppState,
        config: &LayoutConfig,
    ) -> Self {
        let scale = scale_factor.max(1.0);
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;

        // Content fullscreen: hide all chrome, content fills the entire window.
        if state.content_fullscreen {
            return Self {
                window_w: size.width.max(1),
                window_h: size.height.max(1),
                scale_factor: scale,
                clip_sidebar_w: 0,
                toolbar_h: 0,
                ai_w: 0,
                sidebar_css_w: 0.0,
                toolbar_css_h: 0.0,
                ai_css_w: 0.0,
                app_w: 0,
                app_css_w: 0.0,
                app_header_h: 0,
                app_body_chrome_owned: false,
                app_panel_body: None,
                frame_side_w: 0,
                frame_bottom_h: 0,
                frame_side_css: 0.0,
                frame_bottom_css: 0.0,
                content: Rect {
                    x: 0,
                    y: 0,
                    width: size.width.max(1),
                    height: size.height.max(1),
                },
            };
        }

        let is_horizontal = matches!(
            state.settings.appearance.tab_layout,
            crate::config::TabLayout::Horizontal
        );
        let is_auto_hide = !is_horizontal
            && matches!(
                state.settings.appearance.sidebar_mode,
                crate::config::SidebarMode::AutoHide
            );
        let is_compact = !is_horizontal
            && matches!(
                state.settings.appearance.sidebar_mode,
                crate::config::SidebarMode::Compact
            );
        let min_content_w = config.min_content_w as f64;
        let sidebar_css_w = if is_horizontal {
            0.0
        } else if is_auto_hide {
            // Pinned auto-hide sidebar is solid and pushes content aside; unpinned it is
            // a zero-width overlay (the floating sidebar paints on top of full-width
            // content, so the content WebView stays full width and is visually clipped).
            if state.sidebar_pinned {
                (config.sidebar_expanded_w as f64).min((logical_w - min_content_w).max(0.0))
            } else {
                0.0
            }
        } else if is_compact || state.sidebar_collapsed {
            config.sidebar_collapsed_w as f64
        } else {
            (config.sidebar_expanded_w as f64).min((logical_w - min_content_w).max(0.0))
        };

        // AI sidebar pushes content from the right.
        let ai_css_w = if state.ai_sidebar_open {
            let max_for_ai = (logical_w - min_content_w).max(0.0);
            (config.ai_sidebar_w as f64)
                .min(max_for_ai)
                .max((config.min_ai_sidebar_w as f64).min(max_for_ai))
        } else {
            0.0
        };

        let app_css_w = if state.app_sidebar_open {
            let max_for_app = (logical_w - min_content_w).max(0.0);
            (config.app_sidebar_w as f64).min(max_for_app)
        } else {
            0.0
        };

        let bm_bar_extra = if state.settings.appearance.show_bookmarks_bar {
            30u32
        } else {
            0u32
        };
        let tab_bar_extra = if is_horizontal {
            config.horizontal_tabs_h
        } else {
            0
        };
        let toolbar_css_h =
            ((config.toolbar_h + tab_bar_extra + bm_bar_extra) as f64).min(logical_h.max(1.0));
        let sidebar_w = logical_to_physical(sidebar_css_w, scale);

        const FRAME_LOGICAL: f64 = 5.0;
        let frame_side = logical_to_physical(FRAME_LOGICAL, scale);
        let frame_bottom = logical_to_physical(FRAME_LOGICAL, scale);

        let clip_sidebar_w = if is_horizontal {
            frame_side
        } else if is_auto_hide && !state.sidebar_pinned {
            let exp_w = logical_to_physical(config.sidebar_expanded_w as f64, scale);
            if let Some(ov) = state.sidebar_clip_w_override {
                // JS streams the sidebar's live edge during the slide animation so the
                // chrome clip column (and the content cut derived from it) follow the
                // sidebar exactly, leaving no transparent gap that would expose the dark
                // window background as a black bar.
                logical_to_physical(ov, scale).min(exp_w)
            } else if state.sidebar_auto_hide_open {
                exp_w
            } else {
                frame_side
            }
        } else {
            // Solid sidebar (non-auto-hide, or pinned auto-hide): clip covers the full
            // sidebar column plus the left frame strip, aligning with content_offset.
            sidebar_w + frame_side
        };

        let toolbar_h = logical_to_physical(toolbar_css_h, scale);
        let ai_w = logical_to_physical(ai_css_w, scale).min(size.width);
        let app_w = logical_to_physical(app_css_w, scale).min(size.width);
        let right_w = (ai_w + app_w).min(size.width);
        let app_header_h = logical_to_physical(config.app_header_h as f64, scale);

        let content_offset = if is_horizontal || is_auto_hide && !state.sidebar_pinned {
            frame_side
        } else {
            sidebar_w + frame_side
        };

        let content_x = content_offset as i32;
        let content_w = size
            .width
            .saturating_sub(right_w)
            .saturating_sub(content_offset)
            .saturating_sub(frame_side) // right frame strip
            .max(1);
        let content_h = size
            .height
            .saturating_sub(toolbar_h)
            .saturating_sub(frame_bottom)
            .max(1);

        let app_body_chrome_owned = state.app_sidebar_open && state.app_panel_active.is_none();
        let app_panel_body = if state.app_sidebar_open && state.app_panel_active.is_some() {
            let body_h = size
                .height
                .saturating_sub(toolbar_h + app_header_h)
                .saturating_sub(frame_bottom)
                .max(1);
            Some(Rect {
                x: size.width.saturating_sub(app_w) as i32,
                y: (toolbar_h + app_header_h) as i32,
                width: app_w.max(1),
                height: body_h,
            })
        } else {
            None
        };

        Self {
            window_w: size.width.max(1),
            window_h: size.height.max(1),
            scale_factor: scale,
            clip_sidebar_w,
            toolbar_h,
            ai_w,
            sidebar_css_w,
            toolbar_css_h,
            ai_css_w,
            app_w,
            app_css_w,
            app_header_h,
            app_body_chrome_owned,
            app_panel_body,
            frame_side_w: frame_side,
            frame_bottom_h: frame_bottom,
            frame_side_css: FRAME_LOGICAL,
            frame_bottom_css: FRAME_LOGICAL,
            content: Rect {
                x: content_x,
                y: toolbar_h as i32,
                width: content_w,
                height: content_h,
            },
        }
    }
}

fn logical_to_physical(value: f64, scale_factor: f64) -> u32 {
    (value * scale_factor).round().max(0.0) as u32
}

fn watch_load(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    watches: &mut HashMap<String, u64>,
    next: &mut u64,
    tab_id: String,
    url: String,
) {
    let proxy = proxy.clone();
    let url = crate::utils::url::clean_tracking_url(&url);
    *next = next.wrapping_add(1).max(1);
    let watch = *next;
    clear_load_watches(watches, &tab_id);
    watches.insert(app::load_key(&tab_id, &url), watch);
    let probe_proxy = proxy.clone();
    let probe_tab = tab_id.clone();
    let probe_url = url.clone();
    rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(BLACK_PROBE_AFTER)).await;
        let _ = probe_proxy.send_event(AppEvent::ContentBlackProbe {
            tab_id: probe_tab,
            url: probe_url,
            watch,
        });
    });
    rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(LOAD_STALL_AFTER)).await;
        let _ = proxy.send_event(AppEvent::ContentLoadStalled { tab_id, url, watch });
    });
}

fn arm_cover_watch(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    id: &mut u64,
) {
    *id = id.wrapping_add(1).max(1);
    let watch_id = *id;
    let proxy = proxy.clone();
    rt.spawn(async move {
        tokio::time::sleep(Duration::from_millis(COVER_MAX_MS)).await;
        let _ = proxy.send_event(AppEvent::CoverWatchdog { id: watch_id });
    });
}

fn arm_app_panel_sleep(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    id: &mut u64,
) -> tokio::task::JoinHandle<()> {
    *id = id.wrapping_add(1).max(1);
    let sleep_id = *id;
    let proxy = proxy.clone();
    rt.spawn(async move {
        tokio::time::sleep(APP_PANEL_SLEEP_AFTER).await;
        let _ = proxy.send_event(AppEvent::AppPanelSleep { id: sleep_id });
    })
}

fn refresh_nav_buttons(
    chrome: &WebView,
    content_views: &HashMap<String, WebView>,
    state: &mut AppState,
) {
    let Some(id) = state.tab_manager.active_tab_id.clone() else {
        return;
    };
    let Some(wv) = content_views.get(&id) else {
        return;
    };
    #[cfg(windows)]
    let (back, fwd) = (
        wv.can_go_back().unwrap_or(false),
        wv.can_go_forward().unwrap_or(false),
    );
    #[cfg(not(windows))]
    let (back, fwd) = {
        let _ = wv;
        (false, false)
    };
    let loading = state
        .tab_manager
        .get_tab(&id)
        .map(|t| t.status == crate::browser::tab::TabStatus::Loading)
        .unwrap_or(false);
    if let Some(tab) = state.tab_manager.get_tab_mut(&id) {
        tab.engine_can_back = Some(back);
        tab.engine_can_forward = Some(fwd);
    }
    let _ = chrome.evaluate_script(&format!(
        "window.__neura && window.__neura.updateNavState({},{},{})",
        back, fwd, loading
    ));
}

fn webview_cookie_db_exists(data_dir: &std::path::Path) -> bool {
    let base = data_dir.join("webview_data");
    [
        base.join("EBWebView")
            .join("Default")
            .join("Network")
            .join("Cookies"),
        base.join("Default").join("Network").join("Cookies"),
    ]
    .iter()
    .any(|path| path.is_file())
}

fn ubol_dir() -> Option<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(base) = exe.parent() {
            dirs.push(base.join("assets").join("extensions").join("ubol"));
            dirs.push(base.join("extensions").join("ubol"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets").join("extensions").join("ubol"));
    }
    dirs.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("extensions")
            .join("ubol"),
    );
    dirs.into_iter()
        .find(|dir| dir.join("manifest.json").exists())
}

fn active_ubol_enabled(state: &AppState) -> bool {
    if !state.settings.privacy.ad_blocker_enabled {
        return false;
    }
    let Some(id) = state.tab_manager.active_tab_id.as_ref() else {
        return false;
    };
    let Some(tab) = state.tab_manager.get_tab(id) else {
        return false;
    };
    !state.ad_block_engine.is_site_excepted(&tab.url) && !state.x_login_compat(id, &tab.url)
}

fn sync_active_ubol(
    views: &HashMap<String, WebView>,
    state: &AppState,
    dir: Option<&std::path::Path>,
    done: &mut bool,
    last: &mut Option<bool>,
    last_id: &mut Option<String>,
) {
    let Some(dir) = dir else {
        return;
    };
    let Some(id) = state.tab_manager.active_tab_id.as_ref() else {
        return;
    };
    let Some(wv) = views.get(id) else {
        return;
    };
    let enabled = active_ubol_enabled(state);
    if *done && *last == Some(enabled) && last_id.as_deref() == Some(id.as_str()) {
        return;
    }
    browser::extensions::sync_ubol(wv, dir, enabled);
    *done = true;
    *last = Some(enabled);
    *last_id = Some(id.clone());
}

fn restore_startup_cookies(
    wv: &WebView,
    incognito: bool,
    cookies: &[cookie_store::CookieRecord],
    restored: &mut bool,
) {
    if incognito || *restored || cookies.is_empty() {
        return;
    }
    *restored = true;
    let have = browser::cookie_manager::snapshot_settled(wv, Duration::from_millis(3000));
    let have_keys: HashSet<(String, String, String)> = have
        .iter()
        .map(|c| (c.domain.clone(), c.path.clone(), c.name.clone()))
        .collect();
    let missing: Vec<cookie_store::CookieRecord> = cookies
        .iter()
        .filter(|c| !have_keys.contains(&(c.domain.clone(), c.path.clone(), c.name.clone())))
        .cloned()
        .collect();
    tracing::info!(
        "cookie heal: webview loaded {} cookies, backup has {}, injecting {} missing",
        have.len(),
        cookies.len(),
        missing.len()
    );
    if !missing.is_empty() {
        browser::cookie_manager::restore_cookies(wv, &missing);
    }
}

fn save_open_cookies(
    content_views: &HashMap<String, WebView>,
    state: &AppState,
    data_dir: &std::path::Path,
) {
    let t0 = std::time::Instant::now();
    let Ok(conn) = cookie_store::open(data_dir) else {
        return;
    };
    // Every non-incognito tab shares ONE WebView2 profile (`content_web_context`), so its
    // cookie store is identical across all of them — `GetCookies("")` on any one normal tab
    // returns the whole shared set. Snapshotting per tab therefore re-reads the same cookies
    // N times, and each snapshot is a synchronous COM round trip that parks the UI thread for
    // up to ~900 ms. On shutdown with several tabs open that stacks into multiple seconds of a
    // frozen ("Not Responding") window. Take a SINGLE snapshot from one non-incognito tab
    // (prefer the active one) instead. Incognito tabs use a separate, intentionally ephemeral
    // context and are never persisted, so they are skipped entirely.
    let snapshot_wv = state
        .tab_manager
        .active_tab_id
        .as_deref()
        .filter(|id| !state.tab_manager.tab_is_incognito(id))
        .and_then(|id| content_views.get(id))
        .or_else(|| {
            content_views
                .iter()
                .find(|(tid, _)| !state.tab_manager.tab_is_incognito(tid))
                .map(|(_, wv)| wv)
        });
    let mut saved = 0usize;
    if let Some(wv) = snapshot_wv {
        let cookies = browser::cookie_manager::snapshot(wv, Duration::from_millis(900));
        saved = cookies.len();
        if !cookies.is_empty() {
            let _ = cookie_store::save(&conn, &cookies);
        }
    }
    let _ = cookie_store::purge_expired(&conn);
    tracing::info!(
        target: "ventus::shutdown",
        cookies = saved,
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "save_open_cookies: shared-profile snapshot done"
    );
}

fn clear_load_watches(watches: &mut HashMap<String, u64>, tab_id: &str) {
    let key = format!("{}\n", tab_id);
    watches.retain(|k, _| !k.starts_with(&key));
}

fn apply_layout(
    chrome: &WebView,
    chrome_hwnd: Option<isize>,
    content_views: &HashMap<String, WebView>,
    state: &AppState,
    config: &LayoutConfig,
    window: &tao::window::Window,
) {
    let layout = AppLayout::calculate(
        layout_size(window, state),
        window.scale_factor(),
        state,
        config,
    );
    sync_content_views(content_views, state, layout);
    #[cfg(windows)]
    sync_content_clip(content_views, state, layout);
    sync_chrome(chrome, chrome_hwnd, state, layout);
    #[cfg(windows)]
    sync_content_z_order(content_views, chrome_hwnd, state, true);
}

fn action_content_cover(
    action: &TabAction,
    state: &AppState,
    content_views: &HashMap<String, WebView>,
) -> bool {
    let active_missing = state
        .tab_manager
        .active_tab()
        .map(|tab| tab_needs_content(tab, content_views.contains_key(&tab.id)))
        .unwrap_or(false);
    if active_missing {
        return true;
    }
    match action {
        TabAction::Create { url, .. } => !url.starts_with("neura://"),
        TabAction::ContentNavigate(url) => {
            if url.starts_with("neura://") {
                return false;
            }
            let Some(id) = state.tab_manager.active_tab_id.as_deref() else {
                return false;
            };
            !content_views.contains_key(id)
        }
        TabAction::ActivateContent { tab_id, .. } => !content_views.contains_key(tab_id),
        TabAction::ReloadContent { tab_id, .. } => {
            state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str())
                && !content_views.contains_key(tab_id)
        }
        TabAction::RebuildContent { tab_id, .. } => {
            state.tab_manager.active_tab_id.as_deref() == Some(tab_id.as_str())
        }
        _ => state.content_cover_open && active_tab_loading(state),
    }
}

fn tab_needs_content(tab: &crate::browser::tab::Tab, has_view: bool) -> bool {
    !tab.is_neura_page() && !has_view
}

fn find_page_script(query: &str, forward: bool) -> String {
    let q = serde_json::to_string(query).unwrap_or_else(|_| "\"\"".into());
    let f = if forward { "true" } else { "false" };
    format!(
        "(()=>{{const q={};if(!window.__neuraFind)return {{query:q,total:0,index:0}};return window.__neuraFind.run(q,{})}})()",
        q, f
    )
}

fn find_empty_result(query: &str) -> String {
    serde_json::json!({"query":query,"total":0,"index":0}).to_string()
}

fn active_tab_loading(state: &AppState) -> bool {
    state
        .tab_manager
        .active_tab()
        .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
        .unwrap_or(false)
}

fn clear_stale_cover(state: &mut AppState, chrome: &WebView) -> bool {
    if !state.content_cover_open || active_tab_loading(state) {
        return false;
    }
    state.set_content_cover(chrome, false);
    true
}

fn layout_size(window: &tao::window::Window, state: &AppState) -> PhysicalSize<u32> {
    #[cfg(windows)]
    if state.content_fullscreen {
        use windows::Win32::{
            Foundation::HWND,
            Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
            },
        };
        unsafe {
            let hwnd = HWND(window.hwnd());
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                let r = info.rcMonitor;
                return PhysicalSize::new((r.right - r.left) as u32, (r.bottom - r.top) as u32);
            }
        }
    }
    #[cfg(windows)]
    let _ = state;
    #[cfg(not(windows))]
    if state.content_fullscreen || window.fullscreen().is_some() {
        if let Some(monitor) = window.current_monitor() {
            return monitor.size();
        }
    }
    window.inner_size()
}

fn key_pressed(event: &KeyEvent, key: KeyCode) -> bool {
    event.state == ElementState::Pressed && !event.repeat && event.physical_key == key
}

fn update_msg_mods(mods: &AtomicUsize, vk: u32, down: bool) {
    let bit = match vk {
        0x10 | 0xa0 | 0xa1 => MOD_SHIFT,
        0x11 | 0xa2 | 0xa3 => MOD_CTRL,
        0x12 | 0xa4 | 0xa5 => MOD_ALT,
        _ => return,
    };
    if down {
        mods.fetch_or(bit, Ordering::SeqCst);
    } else {
        mods.fetch_and(!bit, Ordering::SeqCst);
    }
}

fn msg_shortcut(vk: u32, mods: usize, repeat: bool) -> usize {
    if repeat {
        return SC_NONE;
    }
    let ctrl = mods & MOD_CTRL != 0;
    let shift = mods & MOD_SHIFT != 0;
    let alt = mods & MOD_ALT != 0;
    if alt && !ctrl {
        return SC_NONE;
    }
    if ctrl {
        return match vk {
            0x09 if shift => SC_PREV_TAB,
            0x09 => SC_NEXT_TAB,
            0x54 if shift => SC_REOPEN_TAB,
            0x54 => SC_SPOTLIGHT,
            0x4e if shift => SC_INCOGNITO,
            0x4e => SC_NEW_WINDOW,
            0x57 => SC_CLOSE_TAB,
            0x46 => SC_FIND,
            0x4c => SC_FOCUS_URL,
            0x4b => SC_TAB_SEARCH,
            0x48 if !shift => SC_HISTORY,
            0x4a if !shift => SC_DOWNLOADS,
            0x44 => SC_BOOKMARK,
            0x41 if shift => SC_AI,
            0x42 => SC_SIDEBAR,
            0xbc => SC_SETTINGS,
            0x52 => SC_RELOAD,
            0xbb | 0x6b => SC_ZOOM_IN,
            0xbd | 0x6d => SC_ZOOM_OUT,
            0x30 | 0x60 => SC_ZOOM_RESET,
            n @ 0x31..=0x39 if !shift => SC_TAB_1 + (n as usize - 0x31),
            n @ 0x61..=0x69 if !shift => SC_TAB_1 + (n as usize - 0x61),
            _ => SC_NONE,
        };
    }
    match vk {
        0x74 => SC_RELOAD,
        0x7a => SC_FULLSCREEN,
        0x7b => SC_DEVTOOLS,
        _ => SC_NONE,
    }
}

fn run_shortcut(code: usize, proxy: &tao::event_loop::EventLoopProxy<AppEvent>, state: &AppState) {
    let cmd = match code {
        SC_SPOTLIGHT => Some(ChromeCommand::BeginSpotlight),
        SC_REOPEN_TAB => Some(ChromeCommand::ReopenTab),
        SC_NEW_WINDOW => Some(ChromeCommand::OpenInNewWindow {
            url: "neura://newtab".into(),
        }),
        SC_CLOSE_TAB => state
            .tab_manager
            .active_tab_id
            .clone()
            .map(|id| ChromeCommand::CloseTab { id }),
        SC_FIND => Some(ChromeCommand::OpenFindBar),
        SC_FOCUS_URL => Some(ChromeCommand::FocusAddressBar),
        SC_TAB_SEARCH => Some(ChromeCommand::OpenTabSearch),
        SC_HISTORY => Some(ChromeCommand::OpenHistoryPanel),
        SC_DOWNLOADS => Some(ChromeCommand::OpenDownloadsPanel),
        SC_BOOKMARK => bookmark_shortcut(state),
        SC_AI => Some(ChromeCommand::ToggleAiSidebar),
        SC_SIDEBAR => Some(ChromeCommand::SidebarToggle),
        SC_SETTINGS => Some(ChromeCommand::OpenSettings),
        SC_RELOAD => Some(ChromeCommand::Reload),
        SC_ZOOM_IN => Some(ChromeCommand::ZoomDelta { delta: 0.1 }),
        SC_ZOOM_OUT => Some(ChromeCommand::ZoomDelta { delta: -0.1 }),
        SC_ZOOM_RESET => Some(ChromeCommand::ZoomReset),
        SC_BACK => Some(ChromeCommand::Back),
        SC_FORWARD => Some(ChromeCommand::Forward),
        SC_FULLSCREEN => Some(ChromeCommand::ToggleFullscreen),
        SC_DEVTOOLS => Some(ChromeCommand::OpenDevtools),
        SC_INCOGNITO => Some(ChromeCommand::OpenIncognito),
        SC_NEXT_TAB => Some(ChromeCommand::SwitchTabOffset { delta: 1 }),
        SC_PREV_TAB => Some(ChromeCommand::SwitchTabOffset { delta: -1 }),
        code if (SC_TAB_1..=SC_TAB_9).contains(&code) => state
            .tab_manager
            .active_workspace_tabs()
            .get(code - SC_TAB_1)
            .map(|tab| ChromeCommand::SwitchTab { id: tab.id.clone() }),
        _ => None,
    };
    if let Some(cmd) = cmd {
        let _ = proxy.send_event(AppEvent::Chrome(cmd));
    }
}

fn bookmark_shortcut(state: &AppState) -> Option<ChromeCommand> {
    let tab = state.tab_manager.active_tab()?;
    let url = tab.url.clone();
    if repositories::is_bookmarked(&state.conn, &url).unwrap_or(false) {
        Some(ChromeCommand::BookmarkRemove { url })
    } else {
        Some(ChromeCommand::BookmarkAdd)
    }
}

fn toggle_window_maximized(window: &tao::window::Window, maxed: &mut bool) {
    set_window_maximized(window, !*maxed, maxed);
}

fn sync_window_maximized(chrome: &WebView, maxed: bool) {
    let _ = chrome.evaluate_script(&format!(
        "window.__neura&&window.__neura.setWindowMaximized({})",
        maxed
    ));
}

#[cfg(windows)]
fn set_window_maximized(window: &tao::window::Window, next: bool, maxed: &mut bool) {
    keep_frameless(window);
    window.set_maximized(next);
    *maxed = next;
    keep_frameless(window);
}

#[cfg(not(windows))]
fn set_window_maximized(window: &tao::window::Window, next: bool, maxed: &mut bool) {
    window.set_maximized(next);
    *maxed = next;
}

#[cfg(windows)]
fn set_fullscreen_z(window: &tao::window::Window, active: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE,
    };
    if active {
        return;
    }
    unsafe {
        let hwnd = HWND(window.hwnd());
        let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    }
}

#[cfg(not(windows))]
fn set_fullscreen_z(_window: &tao::window::Window, _active: bool) {}

#[cfg(windows)]
fn restore_window(window: &tao::window::Window) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_RESTORE};
    unsafe {
        let _ = ShowWindow(HWND(window.hwnd()), SW_RESTORE);
    }
}

#[cfg(not(windows))]
fn restore_window(_window: &tao::window::Window) {}

/// Initiate a native Windows resize drag from a JS-side resize handle mousedown.
/// ReleaseCapture() frees any mouse capture (e.g. from the WebView child), then
/// SendMessage(WM_NCLBUTTONDOWN) hands control to the system's resize loop, which
/// tracks the mouse until button-up and resizes the window normally.
#[cfg(windows)]
fn begin_window_resize(hwnd: isize, edge: &str) {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageW, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT,
        HTTOPRIGHT, WM_NCLBUTTONDOWN,
    };
    let ht: usize = match edge {
        "left" => HTLEFT as usize,
        "right" => HTRIGHT as usize,
        "top" => HTTOP as usize,
        "bottom" => HTBOTTOM as usize,
        "topleft" => HTTOPLEFT as usize,
        "topright" => HTTOPRIGHT as usize,
        "bottomleft" => HTBOTTOMLEFT as usize,
        "bottomright" => HTBOTTOMRIGHT as usize,
        _ => return,
    };
    unsafe {
        let _ = ReleaseCapture();
        let _ = SendMessageW(HWND(hwnd), WM_NCLBUTTONDOWN, WPARAM(ht), LPARAM(0));
    }
}

fn keep_frameless(window: &tao::window::Window) {
    #[cfg(windows)]
    {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{
                GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
                SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX,
                WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
            },
        };

        unsafe {
            let hwnd = HWND(window.hwnd());
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let remove = WS_POPUP.0 as isize;
            let add = (WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0
                as isize;
            let next = (style & !remove) | add;
            if next != style {
                let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, next);
                let _ = SetWindowPos(
                    hwnd,
                    HWND(0),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
        }
    }
    #[cfg(not(windows))]
    window.set_decorations(false);
}

#[cfg(windows)]
fn set_square_corners(window: &tao::window::Window) {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND},
    };
    unsafe {
        let hwnd = HWND(window.hwnd());
        let pref = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const _,
            std::mem::size_of_val(&pref) as u32,
        );
    }
}

#[cfg(not(windows))]
fn set_square_corners(_window: &tao::window::Window) {}

#[cfg(windows)]
fn set_window_background_dark(window: &tao::window::Window) {
    use windows::Win32::{
        Foundation::{COLORREF, HWND},
        Graphics::Gdi::CreateSolidBrush,
        UI::WindowsAndMessaging::{SetClassLongPtrW, GET_CLASS_LONG_INDEX},
    };
    unsafe {
        let hwnd = HWND(window.hwnd());
        let brush = CreateSolidBrush(COLORREF(0x00090706));
        SetClassLongPtrW(hwnd, GET_CLASS_LONG_INDEX(-10i32), brush.0 as isize);
    }
}

#[cfg(not(windows))]
fn set_window_background_dark(_window: &tao::window::Window) {}

/// Prevent the window from appearing in screenshots/screen-recordings when in incognito mode.
/// Uses WDA_EXCLUDEFROMCAPTURE (Windows 10 2004+). Silently no-ops on older builds.
#[cfg(windows)]
fn set_screenshot_protection(hwnd_val: isize, protect: bool) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE},
    };
    unsafe {
        let hwnd = HWND(hwnd_val);
        let affinity = if protect {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        let _ = SetWindowDisplayAffinity(hwnd, affinity);
    }
}

#[cfg(windows)]
fn clamp_window_to_work_area(window: &tao::window::Window) {
    use windows::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::{GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER},
    };

    unsafe {
        let hwnd = HWND(window.hwnd());
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return;
        }

        let work = info.rcWork;
        let max_w = work.right - work.left;
        let max_h = work.bottom - work.top;
        let old_w = rect.right - rect.left;
        let old_h = rect.bottom - rect.top;
        let bad_size = old_w >= max_w || old_h >= max_h;
        let w = if bad_size {
            1280.min(max_w).max(800)
        } else {
            old_w.min(max_w).max(800)
        };
        let h = if bad_size {
            800.min(max_h).max(500)
        } else {
            old_h.min(max_h).max(500)
        };
        let mut x = if bad_size {
            work.left + (max_w - w) / 2
        } else {
            rect.left
        };
        let mut y = if bad_size {
            work.top + (max_h - h) / 2
        } else {
            rect.top
        };

        if x + w > work.right {
            x = work.right - w;
        }
        if y + h > work.bottom {
            y = work.bottom - h;
        }
        if x < work.left {
            x = work.left;
        }
        if y < work.top {
            y = work.top;
        }

        if x == rect.left && y == rect.top && w == old_w && h == old_h {
            return;
        }

        let _ = SetWindowPos(hwnd, HWND(0), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

#[cfg(not(windows))]
fn clamp_window_to_work_area(_window: &tao::window::Window) {}

fn sync_content_views(
    content_views: &HashMap<String, WebView>,
    state: &AppState,
    layout: AppLayout,
) {
    let active_id = state.tab_manager.active_tab_id.as_deref().unwrap_or("");
    let active_is_neura_page = state
        .tab_manager
        .active_tab()
        .map(|tab| tab.is_neura_page())
        .unwrap_or(false);

    // content_x is the WebView's left offset in window coordinates. Push it so the
    // init-script resize detection can determine whether the WebView's left edge is
    // actually the window's left edge (content_x == 0, auto-hide mode) or the sidebar's
    // right border (content_x > 0) — in the latter case onL must stay false.
    let content_left = layout.content.x.max(0);
    let left_edge_js = format!("window.__vLeftEdge={}", content_left);

    if !active_is_neura_page {
        if let Some(wv) = content_views.get(active_id) {
            set_content_bounds(wv, layout.content);
            set_content_memory_normal(wv);
            let _ = wv.set_visible(true);
            let _ = wv.evaluate_script(&format!(
                "window.__neuraContentFullscreen={};{}",
                state.content_fullscreen, left_edge_js
            ));
        }
    }

    for (id, wv) in content_views {
        if id == active_id {
            continue;
        }
        set_content_bounds(wv, layout.content);
        set_content_memory_low(wv);
        let _ = wv.set_visible(false);
    }

    if active_is_neura_page {
        if let Some(wv) = content_views.get(active_id) {
            set_content_bounds(wv, layout.content);
            set_content_memory_low(wv);
            let _ = wv.set_visible(false);
        }
    }
}

fn set_content_bounds(wv: &WebView, rect: Rect) {
    #[cfg(windows)]
    if set_content_bounds_win(wv, rect).is_ok() {
        return;
    }

    let _ = wv.set_bounds(rect);
}

#[cfg(windows)]
fn set_content_bounds_win(wv: &WebView, rect: Rect) -> anyhow::Result<()> {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER},
    };
    use wv2win::Win32::Foundation::RECT;

    let width = rect.width.max(1) as i32;
    let height = rect.height.max(1) as i32;
    unsafe {
        wv.controller().SetBounds(RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        })?;
        let Some(hwnd) = webview_hwnd(wv) else {
            anyhow::bail!("missing webview hwnd");
        };
        SetWindowPos(
            HWND(hwnd),
            HWND(0),
            rect.x,
            rect.y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
        let _ = wv.controller().NotifyParentWindowPositionChanged();
    }
    Ok(())
}

fn sync_chrome(chrome: &WebView, chrome_hwnd: Option<isize>, state: &AppState, layout: AppLayout) {
    let _ = chrome.set_bounds(Rect {
        x: 0,
        y: 0,
        width: layout.window_w,
        height: layout.window_h,
    });
    let _ = chrome.evaluate_script(&format!(
        "window.__neura&&window.__neura.setLayout({:.3},{:.3},{:.3},{:.3},{:.3},{:.3})",
        layout.sidebar_css_w,
        layout.toolbar_css_h,
        layout.ai_css_w,
        layout.frame_side_css,
        layout.frame_bottom_css,
        layout.app_css_w
    ));

    #[cfg(windows)]
    if let Some(hwnd) = chrome_hwnd {
        let floating_rects = state
            .suggestion_overlay_rects
            .values()
            .map(|rect| rect.to_physical(layout.scale_factor))
            .collect::<Vec<_>>();
        move_child_window(hwnd, 0, 0, layout.window_w, layout.window_h);
        set_chrome_clip_region(
            hwnd,
            layout.window_w,
            layout.window_h,
            layout.clip_sidebar_w,
            layout.toolbar_h,
            layout.ai_w,
            state.ai_sidebar_open,
            chrome_owns_content(state),
            &floating_rects,
            layout.frame_side_w,
            layout.frame_bottom_h,
            layout.app_w,
            state.app_sidebar_open,
            layout.app_header_h,
            layout.app_body_chrome_owned,
        );
    }
}

fn sync_chrome_clip(chrome_hwnd: Option<isize>, state: &AppState, layout: AppLayout) {
    #[cfg(windows)]
    if let Some(hwnd) = chrome_hwnd {
        let floating_rects = state
            .suggestion_overlay_rects
            .values()
            .map(|rect| rect.to_physical(layout.scale_factor))
            .collect::<Vec<_>>();
        set_chrome_clip_region(
            hwnd,
            layout.window_w,
            layout.window_h,
            layout.clip_sidebar_w,
            layout.toolbar_h,
            layout.ai_w,
            state.ai_sidebar_open,
            chrome_owns_content(state),
            &floating_rects,
            layout.frame_side_w,
            layout.frame_bottom_h,
            layout.app_w,
            state.app_sidebar_open,
            layout.app_header_h,
            layout.app_body_chrome_owned,
        );
    }

    #[cfg(not(windows))]
    let _ = (chrome_hwnd, state, layout);
}

#[cfg(windows)]
fn sync_content_z_order(
    content_views: &HashMap<String, WebView>,
    chrome_hwnd: Option<isize>,
    state: &AppState,
    repaint: bool,
) {
    if chrome_needs_top(state) {
        if let Some(chrome) = chrome_hwnd {
            bring_hwnd_to_top(chrome);
        }
        return;
    }

    let Some(id) = state.tab_manager.active_tab_id.as_deref() else {
        return;
    };
    let Some(wv) = content_views.get(id) else {
        return;
    };
    let Some(hwnd) = webview_hwnd(wv) else {
        return;
    };
    bring_hwnd_to_top(hwnd);
    if repaint {
        repaint_content_webview(hwnd);
    }
}

fn chrome_owns_content(state: &AppState) -> bool {
    state.chrome_overlay_open
        || state.spotlight_open
        || state.content_cover_open
        || state
            .tab_manager
            .active_tab()
            .map(|tab| tab.is_neura_page())
            .unwrap_or(false)
}

fn chrome_needs_top(state: &AppState) -> bool {
    chrome_owns_content(state)
        || !state.suggestion_overlay_rects.is_empty()
        || state.ai_sidebar_open
        || state.app_sidebar_open
}

#[cfg(windows)]
fn sync_content_clip(
    content_views: &HashMap<String, WebView>,
    state: &AppState,
    layout: AppLayout,
) {
    let Some(id) = state.tab_manager.active_tab_id.as_deref() else {
        return;
    };
    let Some(wv) = content_views.get(id) else {
        return;
    };
    let Some(hwnd) = webview_hwnd(wv) else {
        return;
    };
    let cut = content_cut_w(state, layout);
    set_content_clip_region(hwnd, layout.content.width, layout.content.height, cut);
}

#[cfg(windows)]
fn content_cut_w(state: &AppState, layout: AppLayout) -> u32 {
    if state.content_fullscreen || !state.sidebar_auto_hide_open {
        return 0;
    }
    if matches!(
        state.settings.appearance.tab_layout,
        crate::config::TabLayout::Horizontal
    ) {
        return 0;
    }
    if !matches!(
        state.settings.appearance.sidebar_mode,
        crate::config::SidebarMode::AutoHide
    ) {
        return 0;
    }
    let left = layout.content.x.max(0) as u32;
    layout
        .clip_sidebar_w
        .saturating_sub(left)
        .min(layout.content.width)
}

#[cfg(windows)]
fn bring_hwnd_to_top(hwnd: isize) {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOREDRAW, SWP_NOSIZE,
        },
    };
    unsafe {
        // SWP_NOREDRAW: suppress GDI WM_ERASEBKGND/WM_PAINT during Z-order change.
        // WebView2 uses DirectComposition for rendering and updates independently, so
        // GDI repaints are not needed and only cause a black flash before DComp composites.
        let _ = SetWindowPos(
            HWND(hwnd),
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOREDRAW,
        );
    }
}

#[cfg(windows)]
fn move_child_window(hwnd: isize, x: i32, y: i32, width: u32, height: u32) {
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::MoveWindow};

    unsafe {
        // bRepaint=false: WebView2 uses DirectComposition and repaints itself via DComp,
        // not GDI WM_PAINT. Forcing GDI repaint here fires before SetWindowRgn updates
        // the clip region, causing chrome's opaque background to briefly cover content.
        let _ = MoveWindow(
            HWND(hwnd),
            x,
            y,
            width.max(1) as i32,
            height.max(1) as i32,
            false,
        );
    }
}

#[cfg(windows)]
fn webview_hwnd(wv: &WebView) -> Option<isize> {
    unsafe {
        let mut hwnd = std::mem::MaybeUninit::uninit();
        wv.controller().ParentWindow(hwnd.as_mut_ptr()).ok()?;
        let hwnd = hwnd.assume_init();
        if hwnd.0 == 0 {
            None
        } else {
            Some(hwnd.0 as isize)
        }
    }
}

#[derive(Clone, Copy)]
struct PhysicalClipRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl app::ChromeClipRect {
    fn to_physical(self, scale_factor: f64) -> PhysicalClipRect {
        PhysicalClipRect {
            x: (self.x * scale_factor).floor().max(0.0) as i32,
            y: (self.y * scale_factor).floor().max(0.0) as i32,
            width: (self.width * scale_factor).ceil().max(1.0) as i32,
            height: (self.height * scale_factor).ceil().max(1.0) as i32,
        }
    }
}

#[cfg(windows)]
fn set_chrome_clip_region(
    hwnd: isize,
    window_w: u32,
    window_h: u32,
    sidebar_w: u32,
    toolbar_h: u32,
    ai_sidebar_w: u32,
    ai_open: bool,
    overlay_open: bool,
    floating_rects: &[PhysicalClipRect],
    frame_side_w: u32,
    frame_bottom_h: u32,
    app_w: u32,
    app_open: bool,
    app_header_h: u32,
    app_body_chrome_owned: bool,
) {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_OR},
    };

    #[derive(Clone, Copy)]
    struct ClipSpec<'a> {
        window_w: u32,
        window_h: u32,
        sidebar_w: u32,
        toolbar_h: u32,
        ai_sidebar_w: u32,
        ai_open: bool,
        overlay_open: bool,
        floating_rects: &'a [PhysicalClipRect],
        frame_side_w: u32,
        frame_bottom_h: u32,
        app_w: u32,
        app_open: bool,
        app_header_h: u32,
        app_body_chrome_owned: bool,
    }

    unsafe fn create_region(spec: ClipSpec<'_>) -> windows::Win32::Graphics::Gdi::HRGN {
        if spec.overlay_open {
            return CreateRectRgn(0, 0, spec.window_w as i32, spec.window_h as i32);
        }

        let toolbar = CreateRectRgn(0, 0, spec.window_w as i32, spec.toolbar_h as i32);
        let sidebar = CreateRectRgn(
            0,
            spec.toolbar_h as i32,
            spec.sidebar_w as i32,
            spec.window_h as i32,
        );
        let _ = CombineRgn(toolbar, toolbar, sidebar, RGN_OR);
        let _ = DeleteObject(sidebar);

        if spec.ai_open {
            let left = spec.window_w.saturating_sub(spec.ai_sidebar_w) as i32;
            let ai = CreateRectRgn(
                left,
                spec.toolbar_h as i32,
                spec.window_w as i32,
                spec.window_h as i32,
            );
            let _ = CombineRgn(toolbar, toolbar, ai, RGN_OR);
            let _ = DeleteObject(ai);
        }

        if spec.app_open {
            let left = spec.window_w.saturating_sub(spec.app_w) as i32;
            let header_bottom = (spec.toolbar_h + spec.app_header_h).min(spec.window_h) as i32;
            let header = CreateRectRgn(
                left,
                spec.toolbar_h as i32,
                spec.window_w as i32,
                header_bottom,
            );
            let _ = CombineRgn(toolbar, toolbar, header, RGN_OR);
            let _ = DeleteObject(header);
            if spec.app_body_chrome_owned {
                let body = CreateRectRgn(
                    left,
                    header_bottom,
                    spec.window_w as i32,
                    spec.window_h as i32,
                );
                let _ = CombineRgn(toolbar, toolbar, body, RGN_OR);
                let _ = DeleteObject(body);
            }
        }

        for rect in spec.floating_rects {
            let left = rect.x.clamp(0, spec.window_w as i32);
            let top = rect.y.clamp(0, spec.window_h as i32);
            let right = (rect.x + rect.width).clamp(left, spec.window_w as i32);
            let bottom = (rect.y + rect.height).clamp(top, spec.window_h as i32);
            if right > left && bottom > top {
                let floating = CreateRectRgn(left, top, right, bottom);
                let _ = CombineRgn(toolbar, toolbar, floating, RGN_OR);
                let _ = DeleteObject(floating);
            }
        }

        // Right frame strip: between content and window/AI-panel edge
        if spec.frame_side_w > 0 {
            let right_r = spec.window_w.saturating_sub(spec.ai_sidebar_w + spec.app_w) as i32;
            let right_l = right_r - spec.frame_side_w as i32;
            if right_l >= 0 && right_r > right_l {
                let rf = CreateRectRgn(
                    right_l,
                    spec.toolbar_h as i32,
                    right_r,
                    spec.window_h as i32,
                );
                let _ = CombineRgn(toolbar, toolbar, rf, RGN_OR);
                let _ = DeleteObject(rf);
            }
        }

        // Bottom frame strip
        if spec.frame_bottom_h > 0 {
            let bot_top = spec.window_h.saturating_sub(spec.frame_bottom_h) as i32;
            let bf = CreateRectRgn(0, bot_top, spec.window_w as i32, spec.window_h as i32);
            let _ = CombineRgn(toolbar, toolbar, bf, RGN_OR);
            let _ = DeleteObject(bf);
        }

        toolbar
    }

    unsafe {
        let spec = ClipSpec {
            window_w,
            window_h,
            sidebar_w,
            toolbar_h,
            ai_sidebar_w,
            ai_open,
            overlay_open,
            floating_rects,
            frame_side_w,
            frame_bottom_h,
            app_w,
            app_open,
            app_header_h,
            app_body_chrome_owned,
        };
        let region = create_region(spec);
        let _ = SetWindowRgn(HWND(hwnd), region, false);
    }
}

#[cfg(windows)]
fn set_content_clip_region(hwnd: isize, width: u32, height: u32, cut_w: u32) {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, HRGN, RGN_DIFF},
    };

    unsafe {
        if cut_w == 0 {
            let _ = SetWindowRgn(HWND(hwnd), HRGN(0), false);
            return;
        }
        let full = CreateRectRgn(0, 0, width.max(1) as i32, height.max(1) as i32);
        let cut = CreateRectRgn(0, 0, cut_w.min(width).max(1) as i32, height.max(1) as i32);
        let _ = CombineRgn(full, full, cut, RGN_DIFF);
        let _ = DeleteObject(cut);
        let _ = SetWindowRgn(HWND(hwnd), full, false);
    }
}

#[cfg(windows)]
fn track_content_hwnd(
    hwnd: Option<isize>,
    tab_id: &str,
    content_hwnds: &mut HashMap<String, isize>,
) {
    let Some(hwnd) = hwnd else {
        return;
    };
    content_hwnds.insert(tab_id.to_string(), hwnd);
}

#[cfg(windows)]
fn repaint_content_webview(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
    unsafe {
        // bErase=false: do NOT erase the GDI background. WebView2 renders via DirectComposition,
        // not GDI. Erasing to black (bErase=true) leaves a black hole visible for the brief
        // window between the GDI erase and the DComp surface being composited on top.
        let _ = InvalidateRect(HWND(hwnd), None, false);
        let _ = UpdateWindow(HWND(hwnd));
    }
}

#[cfg(windows)]
fn nudge_active_content(
    content_views: &HashMap<String, WebView>,
    content_hwnds: &HashMap<String, isize>,
    state: &AppState,
    layout: AppLayout,
) {
    let Some(id) = state.tab_manager.active_tab_id.as_deref() else {
        return;
    };
    let Some(wv) = content_views.get(id) else {
        return;
    };
    wake_content_webview(wv);
    let _ = wv.set_visible(false);
    let _ = wv.set_visible(true);
    let _ = wv.focus();
    set_content_bounds(wv, layout.content);
    if let Some(&hwnd) = content_hwnds.get(id) {
        repaint_content_webview(hwnd);
    }
}

#[cfg(windows)]
fn heal_active_content(
    content_views: &HashMap<String, WebView>,
    content_hwnds: &HashMap<String, isize>,
    state: &AppState,
    layout: AppLayout,
) {
    if chrome_needs_top(state) {
        return;
    }
    let Some(id) = state.tab_manager.active_tab_id.as_deref() else {
        return;
    };
    let Some(wv) = content_views.get(id) else {
        return;
    };
    set_content_bounds(wv, layout.content);
    let _ = wv.set_visible(true);
    let hwnd = content_hwnds.get(id).copied().or_else(|| webview_hwnd(wv));
    let Some(hwnd) = hwnd else {
        return;
    };
    bring_hwnd_to_top(hwnd);
    repaint_content_webview(hwnd);
}
