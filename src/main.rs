#![cfg_attr(windows, windows_subsystem = "windows")]
#![deny(warnings)]
#![allow(dead_code)]

mod adblock;
mod ai;
mod app;
mod browser;
mod cloud;
mod config;
mod notify;
mod storage;
mod ui;
mod updater;
mod utils;
mod version;

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    io::Cursor,
    sync::{
        atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    keyboard::KeyCode,
    window::{Fullscreen, WindowBuilder},
};
use wry::{Rect, WebView, WebViewBuilder};

#[cfg(windows)]
use tao::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
#[cfg(windows)]
use wry::{MemoryUsageLevel, WebViewBuilderExtWindows, WebViewExtWindows};

use app::{handle_app_event_inner, tab_zoom, AppState, DownloadCtl, TabAction};
use image::GenericImageView;
use storage::{cookie_store, database, migrations, repositories, settings_store};
use ui::chrome::chrome_html;
use ui::events::{AppEvent, ChromeCommand};

const MOD_CTRL: usize = 1;
const MOD_SHIFT: usize = 2;
const MOD_ALT: usize = 4;
const SC_NONE: usize = 0;
const SC_SPOTLIGHT: usize = 1;
const SC_REOPEN_TAB: usize = 2;
const SC_NEW_WINDOW: usize = 3;
const SC_CLOSE_TAB: usize = 4;
const SC_FOCUS_URL: usize = 5;
const SC_TAB_SEARCH: usize = 6;
const SC_HISTORY: usize = 7;
const SC_DOWNLOADS: usize = 8;
const SC_BOOKMARK: usize = 9;
const SC_AI: usize = 10;
const SC_SIDEBAR: usize = 11;
const SC_SETTINGS: usize = 12;
const SC_RELOAD: usize = 13;
const SC_ZOOM_IN: usize = 14;
const SC_ZOOM_OUT: usize = 15;
const SC_ZOOM_RESET: usize = 16;
const SC_BACK: usize = 17;
const SC_FORWARD: usize = 18;
const SC_FULLSCREEN: usize = 19;
const SC_DEVTOOLS: usize = 20;
const SC_INCOGNITO: usize = 21;
const SC_TAB_1: usize = 22;
const SC_TAB_9: usize = 30;
const SC_FIND: usize = 31;
const SC_NEXT_TAB: usize = 32;
const SC_PREV_TAB: usize = 33;
const LOAD_STALL_AFTER: u64 = 6;
const BLACK_PROBE_AFTER: u64 = 3;
const COVER_MAX_MS: u64 = 1000;
const TAB_SLEEP_CHECK_EVERY: Duration = Duration::from_secs(20);
const SUSPEND_IDLE_MS: i64 = 180_000;
const DISCARD_FREE_MB: u64 = 512;
const MAX_PRESERVED_WEBVIEWS: usize = 32;
const HEAL_CONTENT_EVERY: Duration = Duration::from_millis(750);
const MAX_DOWNLOAD_RESUMES: u32 = 8;
const DOWNLOAD_RESUME_DELAY: Duration = Duration::from_secs(2);
const SESSION_SAVE_DELAY: Duration = Duration::from_secs(3);
const WEBVIEW_PROFILE_RELEASE_TIMEOUT: Duration = Duration::from_secs(12);
const WEBVIEW_PROFILE_RELEASE_POLL: u64 = 50;
const CONTENT_BG: (u8, u8, u8, u8) = (255, 255, 255, 255);
#[cfg(windows)]
const APP_ID: &str = "NeuraSpheres.Ventus";
const COOKIE_SAVE_EVERY: Duration = Duration::from_secs(5 * 60);
const ERROR_REPORT_COOLDOWN: Duration = Duration::from_secs(5 * 60);

fn install_panic_hook(crash_path: std::path::PathBuf, session_id: String) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let panic = format!("{}\n\n{}", info, std::backtrace::Backtrace::force_capture());
        let record = cloud::report::CrashRecord {
            session_id: session_id.clone(),
            app_version: version::APP_VERSION.to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            panic,
            logs: utils::log_buffer::snapshot(cloud::report::MAX_LOGS),
        };
        cloud::report::write_crash(&crash_path, &record);
        prev(info);
    }));
}

fn main() {
    set_app_id();
    notify::register_aumid();
    utils::logging::init();
    tracing::info!("Ventus starting");

    let mut cli_url: Option<String> = None;
    let mut new_window = false;
    let mut wait_for_pid: Option<u32> = None;
    let mut restore_session = false;
    {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--new-window" => new_window = true,
                "--url" => {
                    if let Some(url) = args.next() {
                        cli_url = Some(normalize_launch_url(&url));
                    }
                }
                "--wait-for-pid" => wait_for_pid = args.next().and_then(|pid| pid.parse().ok()),
                "--restore-session" => restore_session = true,
                _ if cli_url.is_none() => {
                    cli_url = launch_url(&arg);
                }
                _ => {}
            }
        }
    }

    wait_for_relaunch_parent(wait_for_pid);
    let data_dir = utils::platform::data_dir();
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let _instance = claim_instance(new_window, cli_url.as_deref(), &data_dir);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();

    encrypt_app_storage(&data_dir);

    let conn = database::open(&data_dir.join("neura.db")).expect("open db");
    migrations::run(&conn).expect("migrations");
    repositories::seed_search_engines(&conn).expect("seed engines");

    let profile_cookie_db_found = webview_cookie_db_exists(&data_dir);
    let startup_cookies: Vec<cookie_store::CookieRecord> = match cookie_store::open(&data_dir) {
        Ok(cs_conn) => {
            let cookies = cookie_store::load_all(&cs_conn).unwrap_or_default();
            tracing::info!(
                "cookie_store: loaded {} backup cookies for startup heal (profile_db_found={})",
                cookies.len(),
                profile_cookie_db_found
            );
            cookies
        }
        Err(e) => {
            tracing::warn!("cookie_store: failed to open for startup read: {}", e);
            vec![]
        }
    };

    let (cookie_tx, cookie_rx) =
        tokio::sync::mpsc::unbounded_channel::<Vec<cookie_store::CookieRecord>>();
    {
        let cs_data_dir = data_dir.clone();
        rt.spawn(async move {
            run_cookie_save_task(cookie_rx, cs_data_dir).await;
        });
    }
    let mut cookies_restored = startup_cookies.is_empty();

    let settings = load_settings(&conn);

    let session_id = uuid::Uuid::new_v4().to_string();
    let device_id = cloud::report::get_or_create_device_id(&conn);
    let crash_path = data_dir.join("pending_crash.json");
    install_panic_hook(crash_path.clone(), session_id.clone());
    if settings.privacy.auto_crash_report {
        if let Some(record) = cloud::report::take_crash(&crash_path) {
            let did = device_id.clone();
            rt.spawn(cloud::report::send_crash(
                record,
                String::new(),
                String::new(),
                did,
            ));
        }
    } else {
        let _ = cloud::report::take_crash(&crash_path);
    }

    let shared_dl_dir = std::sync::Arc::new(std::sync::Mutex::new(download_prefs_from_settings(
        &settings,
    )));

    let onboarding_done: bool = settings_store::get::<bool>(&conn, "onboarding_done")
        .unwrap_or(None)
        .unwrap_or(false);

    let dismissed_update_version: Option<String> =
        settings_store::get::<String>(&conn, "dismissed_update_version").unwrap_or(None);

    let f11_msg = Arc::new(AtomicBool::new(false));
    let f11_msg_hook = f11_msg.clone();
    let shortcut_msg = Arc::new(AtomicUsize::new(SC_NONE));
    let shortcut_msg_hook = shortcut_msg.clone();
    let msg_mods = Arc::new(AtomicUsize::new(0));
    let msg_mods_hook = msg_mods.clone();
    let fullscreen_msg = Arc::new(AtomicBool::new(false));
    let fullscreen_msg_hook = fullscreen_msg.clone();
    let main_hwnd = Arc::new(AtomicIsize::new(0));
    let main_hwnd_hook = main_hwnd.clone();
    let mut event_loop_builder = EventLoopBuilder::<AppEvent>::with_user_event();
    #[cfg(windows)]
    event_loop_builder.with_msg_hook(move |msg| {
        use windows::Win32::{
            Foundation::HWND,
            Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
            },
            UI::WindowsAndMessaging::{
                IsChild, MINMAXINFO, MSG, WM_GETMINMAXINFO, WM_KEYDOWN, WM_KEYUP, WM_NCCALCSIZE,
                WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        };
        let msg = msg as *const MSG;
        if msg.is_null() {
            return false;
        }
        unsafe {
            let hwnd = main_hwnd_hook.load(Ordering::SeqCst);
            let msg_hwnd = (*msg).hwnd.0 as isize;
            let app_msg = hwnd != 0
                && msg_hwnd != 0
                && (msg_hwnd == hwnd || IsChild(HWND(hwnd), (*msg).hwnd).as_bool());
            if hwnd != 0 && msg_hwnd == hwnd && (*msg).message == WM_NCCALCSIZE {
                return true;
            }
            if hwnd != 0 && msg_hwnd == hwnd && (*msg).message == WM_GETMINMAXINFO {
                let info = (*msg).lParam;
                if info.0 != 0 {
                    let monitor = MonitorFromWindow(HWND(hwnd), MONITOR_DEFAULTTONEAREST);
                    let mut monitor_info = MONITORINFO {
                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
                        let monitor = monitor_info.rcMonitor;
                        let work = monitor_info.rcWork;
                        let fullscreen = fullscreen_msg_hook.load(Ordering::SeqCst);
                        let target = if fullscreen { monitor } else { work };
                        let mmi = &mut *(info.0 as *mut MINMAXINFO);
                        mmi.ptMaxPosition.x = target.left - monitor.left;
                        mmi.ptMaxPosition.y = target.top - monitor.top;
                        mmi.ptMaxSize.x = target.right - target.left;
                        mmi.ptMaxSize.y = target.bottom - target.top;
                        mmi.ptMaxTrackSize.x = target.right - target.left;
                        mmi.ptMaxTrackSize.y = target.bottom - target.top;
                        return true;
                    }
                }
            }
            if !app_msg {
                return false;
            }
            let is_down = (*msg).message == WM_KEYDOWN || (*msg).message == WM_SYSKEYDOWN;
            let is_up = (*msg).message == WM_KEYUP || (*msg).message == WM_SYSKEYUP;
            let vk = (*msg).wParam.0 as u32;
            if is_down || is_up {
                update_msg_mods(&msg_mods_hook, vk, is_down);
            }
            if is_down {
                let repeat = ((*msg).lParam.0 as u64 & (1 << 30)) != 0;
                let code = msg_shortcut(vk, msg_mods_hook.load(Ordering::SeqCst), repeat);
                if code != SC_NONE {
                    shortcut_msg_hook.store(code, Ordering::SeqCst);
                    return true;
                }
            }
            let is_f11 = vk == 0x7a;
            if is_down && is_f11 {
                f11_msg_hook.store(true, Ordering::SeqCst);
            }
        }
        false
    });
    let event_loop: EventLoop<AppEvent> = event_loop_builder.build();
    let proxy = event_loop.create_proxy();
    let _url_server = if new_window {
        None
    } else {
        start_url_server(&data_dir, proxy.clone())
    };

    if !new_window {
        let proxy_upd = proxy.clone();
        let dismissed_ver = dismissed_update_version.clone();
        rt.spawn(async move {
            if let Ok(Some(info)) = updater::check_latest().await {
                if dismissed_ver.as_deref() != Some(info.version.as_str()) {
                    let _ = proxy_upd.send_event(AppEvent::UpdateCheckResult {
                        available: true,
                        version: info.version,
                        notes: info.notes,
                        download_url: info.download_url,
                    });
                }
            }
        });
    }

    let win_w = settings.window_width;
    let win_h = settings.window_height;

    static LOGO_PNG: &[u8] = include_bytes!("../public/ventus.png");
    let window_icon = image::load_from_memory(LOGO_PNG).ok().and_then(|img| {
        let img = img.resize(32, 32, image::imageops::FilterType::Lanczos3);
        let (w, h) = img.dimensions();
        tao::window::Icon::from_rgba(img.to_rgba8().into_raw(), w, h).ok()
    });

    let window = WindowBuilder::new()
        .with_title("Ventus")
        .with_inner_size(LogicalSize::new(win_w, win_h))
        .with_min_inner_size(LogicalSize::new(800u32, 500u32))
        .with_window_icon(window_icon)
        .with_decorations(false)
        .with_visible(false)
        .build(&event_loop)
        .expect("build window");
    #[cfg(windows)]
    main_hwnd.store(window.hwnd() as isize, Ordering::SeqCst);
    keep_frameless(&window);
    set_square_corners(&window);
    set_window_background_dark(&window);
    clamp_window_to_work_area(&window);

    let layout_config = LayoutConfig {
        sidebar_expanded_w: settings.sidebar_width,
        sidebar_collapsed_w: 52,
        toolbar_h: 44,
        ai_sidebar_w: 340,
        min_content_w: 320,
        min_ai_sidebar_w: 280,
    };

    let mut state = AppState::new(conn, settings, &data_dir, device_id, session_id);
    state.chrome_overlay_open = !onboarding_done;
    let mut restored_tabs: HashSet<String> = HashSet::new();

    if let Some(ref url) = cli_url {
        let ws_id = state.tab_manager.active_workspace_id.clone();
        let tab = browser::tab::Tab::new(ws_id, url.clone());
        let tab_id = tab.id.clone();
        state.tab_manager.tabs.clear();
        state.tab_manager.tabs.push(tab);
        state.tab_manager.active_tab_id = Some(tab_id);
    }

    let restore_requested = (restore_session
        || matches!(
            state.settings.startup_behavior,
            config::StartupBehavior::LastSession
        ))
        && cli_url.is_none();
    tracing::info!(
        target: "ventus::session",
        startup_behavior = ?state.settings.startup_behavior,
        restore_session_flag = restore_session,
        restore_requested,
        "[SESSION] startup: deciding whether to restore last session"
    );
    if restore_requested {
        match repositories::load_session(&state.conn) {
            Ok(Some(saved)) => {
                state.tab_manager.workspaces = saved.workspaces;
                state.tab_manager.active_workspace_id = saved.active_workspace_id;
                state.tab_manager.tabs.clear();
                state.tab_manager.active_tab_id = saved.active_tab_id;
                let saved_tab_count = saved.tabs.len();
                for saved_tab in saved.tabs {
                    let mut tab = browser::tab::Tab::new(saved_tab.workspace_id, saved_tab.url);
                    tab.id = saved_tab.id;
                    tab.title = saved_tab.title;
                    tab.favicon = saved_tab.favicon;
                    tab.pinned = saved_tab.pinned;
                    tab.is_essential = saved_tab.is_essential;
                    tab.created_at = saved_tab.created_at;
                    tab.last_active_at = saved_tab.last_active_at;
                    tab.back_stack = saved_tab.back_stack;
                    tab.forward_stack = saved_tab.forward_stack;
                    tab.status = browser::tab::TabStatus::Complete;
                    tab.sync_nav_flags();
                    if !tab.is_neura_page() {
                        restored_tabs.insert(tab.id.clone());
                    }
                    tracing::debug!(
                        target: "ventus::session",
                        tab = %tab.id,
                        url = %tab.url,
                        neura_page = tab.is_neura_page(),
                        "[SESSION] restored tab"
                    );
                    state.tab_manager.tabs.push(tab);
                }
                if state.tab_manager.active_tab_id.is_none() {
                    state.tab_manager.active_tab_id =
                        state.tab_manager.tabs.first().map(|tab| tab.id.clone());
                }
                let active_id = state.tab_manager.active_tab_id.clone();
                let active_url = state
                    .tab_manager
                    .active_tab()
                    .map(|t| t.url.clone())
                    .unwrap_or_default();
                tracing::info!(
                    target: "ventus::session",
                    saved_tab_count,
                    live_content_tabs = restored_tabs.len(),
                    active_tab = ?active_id,
                    active_url = %active_url,
                    "[SESSION] restore complete: active tab will load eagerly at cold start, others stay sleeping"
                );
            }
            Ok(None) => {
                tracing::info!(
                    target: "ventus::session",
                    "[SESSION] restore requested but no saved session found"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "ventus::session",
                    error = %e,
                    "[SESSION] restore requested but load_session failed"
                );
            }
        }
    }

    if matches!(
        state.settings.startup_behavior,
        config::StartupBehavior::HomePage
    ) && cli_url.is_none()
    {
        let homepage = app::normalize_homepage(&state.settings.homepage);
        if !homepage.is_empty() {
            if let Some(tab) = state
                .tab_manager
                .tabs
                .iter_mut()
                .find(|t| state.tab_manager.active_tab_id.as_deref() == Some(t.id.as_str()))
            {
                tab.url = homepage;
            }
        }
    }

    let first_tab_id = state
        .tab_manager
        .active_tab_id
        .clone()
        .expect("TabManager always has an active tab");
    let first_url = state
        .tab_manager
        .active_tab()
        .map(|t| t.url.clone())
        .unwrap_or_else(|| browser::tab::Tab::new_tab_url().to_string());
    let win_size = window.inner_size();
    #[cfg(windows)]
    if !new_window {
        cleanup_secondary_webview_profiles(&data_dir);
    }
    let webview_data_dir = if new_window {
        data_dir.join(format!("webview_data_window_{}", std::process::id()))
    } else {
        data_dir.join("webview_data")
    };
    if new_window {
        std::fs::remove_dir_all(&webview_data_dir).ok();
    }
    let crash_sentinel: Option<std::path::PathBuf> = if !new_window {
        let p = data_dir.join("running.lock");
        if p.exists() {
            let roots = vec![webview_data_dir.as_path()];
            tracing::warn!(
                target: "ventus::startup",
                lock = %p.display(),
                "[STARTUP] previous session ended WITHOUT clean shutdown (running.lock present) — waiting for its WebView2 profile lock to release before building content WebViews"
            );
            let wait_started = Instant::now();
            wait_for_previous_instance(&p, &roots);
            tracing::info!(
                target: "ventus::startup",
                waited_ms = wait_started.elapsed().as_millis() as u64,
                "[STARTUP] finished waiting for previous instance / profile release"
            );
        } else {
            tracing::info!(
                target: "ventus::startup",
                "[STARTUP] clean previous shutdown (no running.lock) — profile should be free"
            );
        }
        let _ = std::fs::write(&p, std::process::id().to_string().as_bytes());
        Some(p)
    } else {
        None
    };
    #[cfg(windows)]
    {
        let content_free = webview_profile_lock_released(&webview_data_dir);
        tracing::info!(
            target: "ventus::startup",
            content_profile_free = content_free,
            orphan_msedgewebview2 = count_msedgewebview2_processes(),
            "[STARTUP] WebView2 profile lock state just before building WebViews"
        );
    }

    let browser_args = webview_args(&state.settings);

    std::fs::create_dir_all(&webview_data_dir).expect("create WebView2 profile");
    encrypt_app_storage(&webview_data_dir);
    #[cfg(windows)]
    sync_webview_secure_dns_prefs(&webview_data_dir, &state.settings);
    #[cfg(windows)]
    if !new_window
        && !wait_for_webview_profiles_released(
            &[webview_data_dir.as_path()],
            WEBVIEW_PROFILE_RELEASE_TIMEOUT,
        )
    {
        tracing::warn!(
            target: "ventus::startup",
            "[STARTUP] WebView2 profile still locked after waiting; chrome build will retry"
        );
    }
    let mut content_web_context = Some(wry::WebContext::new(Some(webview_data_dir.clone())));

    let chrome = {
        const MAX_CHROME_ATTEMPTS: u32 = 60;
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let proxy_chrome = proxy.clone();
            let proxy_chrome_load = proxy.clone();
            let builder = WebViewBuilder::new_as_child(&window)
                .with_bounds(Rect {
                    x: 0,
                    y: 0,
                    width: win_size.width,
                    height: win_size.height,
                })
                .with_transparent(true);
            #[cfg(windows)]
            let builder = builder
                .with_browser_accelerator_keys(false)
                .with_additional_browser_args(browser_args.clone());
            let built = builder
                .with_web_context(content_web_context.as_mut().unwrap())
                .with_html(chrome_html())
                .with_ipc_handler(move |req: wry::http::Request<String>| {
                    let body = req.body();
                    match serde_json::from_str::<ChromeCommand>(body) {
                        Ok(cmd) => {
                            let _ = proxy_chrome.send_event(AppEvent::Chrome(cmd));
                        }
                        Err(e) => tracing::warn!("IPC parse: {} | body: {}", e, body),
                    }
                })
                .with_on_page_load_handler(move |event, _url: String| {
                    if let wry::PageLoadEvent::Finished = event {
                        let _ = proxy_chrome_load.send_event(AppEvent::ChromeReady);
                    }
                })
                .build();
            match built {
                Ok(chrome) => break chrome,
                Err(e) if attempt < MAX_CHROME_ATTEMPTS && is_busy_message(&e.to_string()) => {
                    tracing::warn!(
                        target: "ventus::startup",
                        attempt,
                        error = %e,
                        "[STARTUP] chrome WebView profile busy (locked); retrying"
                    );
                    drop(content_web_context.take());
                    #[cfg(windows)]
                    drain_message_queue_ms(100);
                    content_web_context =
                        Some(wry::WebContext::new(Some(webview_data_dir.clone())));
                    std::thread::sleep(Duration::from_millis(500));
                }
                Err(e) => {
                    tracing::error!("build chrome webview: {}", e);
                    if let Some(ref sentinel) = crash_sentinel {
                        let _ = std::fs::remove_file(sentinel);
                    }
                    if is_busy_message(&e.to_string()) {
                        show_startup_error(
                            "Ventus couldn't open because its browser engine profile is still in use.\n\nThis is usually antivirus scanning the profile after the last session closed. Wait a few seconds and open Ventus again.\n\nTo stop it happening, exclude this folder from your antivirus:\n%APPDATA%\\neura\\NeuraBrowser\\data",
                        );
                    } else {
                        show_startup_error(&format!(
                            "Ventus could not start because WebView2 is missing or broken.\n\nRun the installer again, or install Microsoft Edge WebView2 Runtime from Microsoft.\n\n{}",
                            e
                        ));
                    }
                    return;
                }
            }
        }
    };
    #[cfg(windows)]
    let chrome_hwnd = webview_hwnd(&chrome);
    #[cfg(not(windows))]
    let chrome_hwnd = None;

    let initial_layout =
        AppLayout::calculate(win_size, window.scale_factor(), &state, &layout_config);

    // Incognito tabs get their own ephemeral data directory that is wiped on every startup,
    // so cookies, cache, and localStorage never persist across sessions.
    // New windows use a per-process incognito dir so concurrent windows don't wipe each other.
    let incognito_data_dir = if new_window {
        data_dir.join(format!("incognito_session_{}", std::process::id()))
    } else {
        data_dir.join("incognito_session")
    };
    std::fs::remove_dir_all(&incognito_data_dir).ok();
    std::fs::create_dir_all(&incognito_data_dir).ok();
    encrypt_app_storage(&incognito_data_dir);
    #[cfg(windows)]
    sync_webview_secure_dns_prefs(&incognito_data_dir, &state.settings);
    let mut incognito_web_context = Some(wry::WebContext::new(Some(incognito_data_dir.clone())));

    let mut content_views: HashMap<String, WebView> = HashMap::new();
    let mut suspended_tabs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut popups: HashMap<u64, PopupWindow> = HashMap::new();
    let mut next_popup_id: u64 = 1;
    #[cfg(windows)]
    let mut auth_window: Option<(tao::window::Window, WebView)> = None;
    let mut content_hwnds: HashMap<String, isize> = HashMap::new();
    let mut load_watches: HashMap<String, u64> = HashMap::new();
    let mut load_watch_next = 0u64;
    // In-flight "Save image as" requests: save_id → chosen destination. Filled when the
    // save dialog is confirmed; drained when the content WebView posts back the image bytes.
    let mut pending_image_saves: HashMap<String, PendingImageSave> = HashMap::new();
    let mut cover_was_open = false;
    let mut cover_watch_id = 0u64;
    let mut last_active_tab_id: Option<String> = state.tab_manager.active_tab_id.clone();
    let mut sleep_check_at = Instant::now() + TAB_SLEEP_CHECK_EVERY;
    let mut heal_content_at = Instant::now() + HEAL_CONTENT_EVERY;
    let mut error_report_at = Instant::now();
    let mut save_id = 0u64;
    let mut sync_id = 0u64;
    let mut sync_dirty = (false, false, false);
    let ubol_dir = ubol_dir();
    let mut ubol_done = false;
    let mut ubol_enabled: Option<bool> = None;
    let mut ubol_tab: Option<String> = None;
    let mut first_load_after_layout: Option<(String, String)> = None;

    if !first_url.starts_with("neura://") {
        tracing::info!(
            target: "ventus::startup",
            tab = %first_tab_id,
            url = %first_url,
            restored = restored_tabs.contains(&first_tab_id),
            "[STARTUP] building FIRST content WebView (eager cold-start load) — this is the moment most likely to hit a locked profile after a restore"
        );
        let first_is_incognito = state.tab_manager.tab_is_incognito(&first_tab_id);
        begin_native_load(&mut state, &chrome, &first_tab_id);
        let ctx = if first_is_incognito {
            incognito_web_context.as_mut().unwrap()
        } else {
            content_web_context.as_mut().unwrap()
        };
        let first_ad_script = state.ad_block_engine.init_script().to_string();
        match build_content_webview(
            &window,
            &first_tab_id,
            &first_url,
            initial_layout.content,
            proxy.clone(),
            ctx,
            first_is_incognito,
            std::sync::Arc::clone(&shared_dl_dir),
            tab_zoom(&state, &first_tab_id),
            &browser_args,
            first_ad_script,
            state.settings.privacy.fingerprint_protection,
            state.settings.privacy.strict_permissions,
            state.settings.privacy.site_permissions.clone(),
            state.settings.privacy.default_permissions.clone(),
            state.settings.privacy.https_only,
            false,
        ) {
            Ok(first_wv) => {
                tracing::info!(
                    target: "ventus::startup",
                    tab = %first_tab_id,
                    "[STARTUP] FIRST content WebView built OK"
                );
                #[cfg(windows)]
                let first_hwnd = webview_hwnd(&first_wv);
                content_views.insert(first_tab_id.clone(), first_wv);
                if let Some(wv) = content_views.get(&first_tab_id) {
                    restore_startup_cookies(
                        wv,
                        first_is_incognito,
                        &startup_cookies,
                        &mut cookies_restored,
                    );
                }
                first_load_after_layout = Some((first_tab_id.clone(), first_url.clone()));
                #[cfg(windows)]
                track_content_hwnd(first_hwnd, &first_tab_id, &mut content_hwnds);
                sync_active_ubol(
                    &content_views,
                    &state,
                    ubol_dir.as_deref(),
                    &mut ubol_done,
                    &mut ubol_enabled,
                    &mut ubol_tab,
                );
            }
            Err(e) => {
                tracing::error!(
                    target: "ventus::startup",
                    tab = %first_tab_id,
                    url = %first_url,
                    error = %e,
                    "[STARTUP] FIRST content WebView build FAILED — tab will be blank/black. If error contains 0x800700AA the content profile was still locked by an orphaned msedgewebview2.exe"
                );
                if let Some(tab) = state.tab_manager.get_tab_mut(&first_tab_id) {
                    tab.url = browser::tab::Tab::new_tab_url().to_string();
                    tab.title = "New Tab".to_string();
                    tab.status = browser::tab::TabStatus::Complete;
                }
            }
        };
    }
    apply_layout(
        &chrome,
        chrome_hwnd,
        &content_views,
        &state,
        &layout_config,
        &window,
    );
    if let Some((tab_id, url)) = first_load_after_layout.take() {
        if let Some(wv) = content_views.get(&tab_id) {
            let _ = wv.load_url(&url);
            watch_load(
                &rt,
                &proxy,
                &mut load_watches,
                &mut load_watch_next,
                tab_id,
                url,
            );
        }
    }
    // Apply screenshot protection immediately if the initial workspace is incognito.
    #[cfg(windows)]
    {
        let is_incog = state
            .tab_manager
            .active_workspace()
            .map(|w| w.is_incognito)
            .unwrap_or(false);
        set_screenshot_protection(window.hwnd() as isize, is_incog);
    }

    let proxy_main = proxy.clone();
    let ai_generation = Arc::new(AtomicUsize::new(0));
    let mut chrome_shown = true;
    keep_frameless(&window);
    window.set_visible(true);
    let mut last_fullscreen_toggle: Option<Instant> = None;
    let mut sync_fullscreen_layout = false;
    let mut fullscreen_restore_maximized: Option<bool> = None;
    let mut restore_maximized_after_fullscreen = false;
    let mut custom_maximized = false;
    if cloud::config::is_configured() {
        if let Ok(Some(refresh_token)) = storage::keychain::get_api_key(cloud::KEYCHAIN_REFRESH_KEY)
        {
            let proxy_restore = proxy.clone();
            let region = state.settings.region.clone();
            let cached =
                settings_store::get::<cloud::UserProfile>(&state.conn, cloud::PROFILE_CACHE_KEY)
                    .ok()
                    .flatten();
            rt.spawn(async move {
                if let Ok(session) = cloud::auth::refresh(&refresh_token).await {
                    let (session, profile) =
                        cloud::finalize_sign_in(session, None, None, region, cached).await;
                    let pull_session = session.clone();
                    let _ = proxy_restore.send_event(AppEvent::AuthApplied {
                        session,
                        profile,
                        message: String::new(),
                    });
                    let snap = cloud::pull_all(&pull_session).await;
                    let _ = proxy_restore.send_event(AppEvent::SyncPulled {
                        bookmarks: snap.bookmarks,
                        history: snap.history,
                        settings: snap.settings,
                    });
                }
            });
        }
    }
    // Periodic proactive cookie snapshot (backs up cookies even between navigations).
    let mut cookie_save_at = Instant::now() + COOKIE_SAVE_EVERY;
    event_loop.run(move |event, elwt, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _ = &elwt;

        match event {
            Event::UserEvent(AppEvent::ChromeReady) => {
                let js = state.chrome_state_json();
                let _ = chrome
                    .evaluate_script(&format!("window.__neura&&window.__neura.setState({})", js));
                sync_window_maximized(&chrome, custom_maximized || window.is_maximized());
                state.push_newtab_wallpaper_to_chrome(&chrome);
                if state.content_cover_open {
                    let _ = chrome.evaluate_script(
                        "window.__neura&&window.__neura.showContentLoading&&window.__neura.showContentLoading()",
                    );
                }
                if !onboarding_done {
                    let _ = chrome.evaluate_script(
                        "setTimeout(()=>window.__neura&&window.__neura.showOnboarding(),100)",
                    );
                }
                if !chrome_shown {
                    chrome_shown = true;
                    keep_frameless(&window);
                    window.set_visible(true);
                }
                // The startup update check may have finished before the UI existed (its
                // setUpdateState call no-op'd against a missing window.__neura). Flush any
                // pending result now so the bottom-right notification still appears.
                if let (Some(version), Some(notes)) = (
                    state.pending_update_version.clone(),
                    state.pending_update_notes.clone(),
                ) {
                    let v = serde_json::to_string(&version).unwrap_or_default();
                    let n = serde_json::to_string(&notes).unwrap_or_default();
                    let _ = chrome.evaluate_script(&format!(
                        "window.__neura && window.__neura.setUpdateState({{status:'available',version:{},notes:{}}})",
                        v, n
                    ));
                }
                refresh_trends(&mut state, &proxy_main, &rt);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::CheckForUpdate)) => {
                tracing::info!(target: "ventus::feature", action = "check_update", "feature action");
                let proxy_upd = proxy_main.clone();
                rt.spawn(async move {
                    match updater::check_latest().await {
                        Ok(Some(info)) => {
                            let _ = proxy_upd.send_event(AppEvent::UpdateCheckResult {
                                available: true,
                                version: info.version,
                                notes: info.notes,
                                download_url: info.download_url,
                            });
                        }
                        Ok(None) => {
                            let _ = proxy_upd.send_event(AppEvent::UpdateCheckResult {
                                available: false,
                                version: String::new(),
                                notes: String::new(),
                                download_url: String::new(),
                            });
                        }
                        Err(e) => {
                            let _ = proxy_upd.send_event(AppEvent::UpdateCheckFailed {
                                message: e.to_string(),
                            });
                        }
                    }
                });
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::LoadNeuraFeed)) => {
                tracing::info!(target: "ventus::feature", action = "load_neura_feed", "feature action");
                let proxy_feed = proxy_main.clone();
                rt.spawn(async move {
                    let client = reqwest::Client::builder()
                        .user_agent(crate::version::USER_AGENT)
                        .build();
                    let res = match client {
                        Ok(client) => {
                            client
                                .get("https://feed.neuraspheres.com/api/recent-news?limit=15")
                                .send()
                                .await
                        }
                        Err(e) => Err(e),
                    };
                    match res {
                        Ok(resp) => match resp.json::<serde_json::Value>().await {
                            Ok(json) => {
                                let articles = slim_feed_articles(&json);
                                let _ =
                                    proxy_feed.send_event(AppEvent::NeuraFeedLoaded { articles });
                            }
                            Err(e) => {
                                let _ = proxy_feed.send_event(AppEvent::NeuraFeedFailed {
                                    message: e.to_string(),
                                });
                            }
                        },
                        Err(e) => {
                            let _ = proxy_feed.send_event(AppEvent::NeuraFeedFailed {
                                message: e.to_string(),
                            });
                        }
                    }
                });
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::RefreshTrends)) => {
                tracing::info!(target: "ventus::feature", action = "refresh_trends", "feature action");
                refresh_trends(&mut state, &proxy_main, &rt);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::FetchCurrencyRates)) => {
                tracing::info!(target: "ventus::feature", action = "fetch_currency_rates", "feature action");
                let proxy_cur = proxy_main.clone();
                rt.spawn(async move {
                    let client = reqwest::Client::builder()
                        .user_agent(crate::version::USER_AGENT)
                        .timeout(std::time::Duration::from_secs(10))
                        .build();
                    let res = match client {
                        Ok(c) => c.get("https://open.er-api.com/v6/latest/USD").send().await,
                        Err(e) => Err(e),
                    };
                    match res {
                        Ok(resp) => match resp.json::<serde_json::Value>().await {
                            Ok(json) => {
                                if let Some(rates) = json.get("rates").cloned() {
                                    let _ = proxy_cur
                                        .send_event(AppEvent::CurrencyRatesLoaded { rates });
                                } else {
                                    let _ = proxy_cur.send_event(AppEvent::CurrencyRatesFailed);
                                }
                            }
                            Err(_) => {
                                let _ = proxy_cur.send_event(AppEvent::CurrencyRatesFailed);
                            }
                        },
                        Err(_) => {
                            let _ = proxy_cur.send_event(AppEvent::CurrencyRatesFailed);
                        }
                    }
                });
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AiMessage { text })) => {
                handle_ai_message(text, &state, &chrome, &proxy_main, &rt, &ai_generation);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::SpotlightAiQuery { text, history })) => {
                handle_spotlight_ai_query(text, history, &state, &proxy_main, &rt);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AiQuickAction { action })) => {
                handle_ai_quick_action(action, &state, &chrome, &proxy_main, &rt, &ai_generation);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AiStop)) => {
                tracing::info!(target: "ventus::ai", "ai stop requested");
                ai_generation.fetch_add(1, Ordering::SeqCst);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AuthSignUp { email, password })) => {
                tracing::info!(target: "ventus::auth", mode = "sign_up", email_domain = %email_domain(&email), "auth requested");
                auth_email_password(true, email, password, &state, &proxy_main, &rt);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AuthSignIn { email, password })) => {
                tracing::info!(target: "ventus::auth", mode = "sign_in", email_domain = %email_domain(&email), "auth requested");
                auth_email_password(false, email, password, &state, &proxy_main, &rt);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AuthSignInGoogle)) => {
                tracing::info!(target: "ventus::auth", mode = "google", "auth requested");
                if let Some(port) = auth_google(&state, &chrome, &proxy_main, &rt) {
                    #[cfg(windows)]
                    {
                        auth_window = None;
                        if let Some(ctx) = content_web_context.as_mut() {
                            auth_window = spawn_auth_window(
                                elwt,
                                &window,
                                port,
                                ctx,
                                proxy_main.clone(),
                                &browser_args,
                            );
                        }
                    }
                }
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AccountUpdateProfile {
                username,
                full_name,
                birthdate,
                bio,
            })) => {
                tracing::info!(target: "ventus::auth", action = "profile_update", "account action");
                account_update_profile(
                    username,
                    full_name,
                    birthdate,
                    bio,
                    &state,
                    &proxy_main,
                    &rt,
                );
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AccountSetPhoto { data_uri })) => {
                tracing::info!(target: "ventus::auth", action = "photo_update", bytes = data_uri.len(), "account action");
                account_set_photo(data_uri, &state, &chrome, &proxy_main, &rt);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::AccountChangePassword {
                current,
                new_password,
            })) => {
                tracing::info!(target: "ventus::auth", action = "password_change", "account action");
                account_change_password(current, new_password, &state, &proxy_main, &rt);
            }

            // JS resize-handle mousedown → initiate native Win32 resize via ReleaseCapture
            // + SendMessage(WM_NCLBUTTONDOWN, hit_code).  This is the standard way to
            // trigger a system resize loop from a frameless window without NC hit testing.
            Event::UserEvent(AppEvent::Chrome(ChromeCommand::BeginResize { edge })) => {
                #[cfg(windows)]
                begin_window_resize(window.hwnd() as isize, &edge);
            }

            // Execute JS in the active content WebView on behalf of the AI agent loop
            Event::UserEvent(AppEvent::AiExecutePageJs {
                ref call_id,
                ref tab_id,
                ref js,
            }) => {
                if let Some(wv) = content_views.get(tab_id) {
                    let _ = wv.evaluate_script(js);
                } else if let Ok(mut map) = state.ai_pending_tools.lock() {
                    if let Some(tx) = map.remove(call_id) {
                        let _ = tx.send(r#"{"error":"tab is not available"}"#.to_string());
                    }
                }
            }

            Event::UserEvent(AppEvent::CopyImageResult { success }) => {
                let js = if success {
                    "window.__neura && window.__neura.showSuccess('Image copied to clipboard')"
                } else {
                    "window.__neura && window.__neura.showError('Could not copy image')"
                };
                let _ = chrome.evaluate_script(js);
            }

            Event::UserEvent(AppEvent::PwdFillRequest {
                ref tab_id,
                ref origin,
            }) => {
                let creds = crate::storage::passwords::for_origin(
                    &state.conn,
                    &state.pwd_key,
                    origin,
                )
                .unwrap_or_default();
                if let (Some(c), Some(wv)) = (creds.first(), content_views.get(tab_id)) {
                    let js = format!(
                        "window.__ventusPwd && window.__ventusPwd.fill({},{})",
                        serde_json::to_string(&c.username).unwrap_or_default(),
                        serde_json::to_string(&c.password).unwrap_or_default()
                    );
                    let _ = wv.evaluate_script(&js);
                    let _ = crate::storage::passwords::touch(&state.conn, origin, &c.username);
                }
            }

            Event::UserEvent(AppEvent::PwdCapture {
                ref origin,
                ref username,
                ref password,
                ..
            }) => {
                if !username.is_empty() && !password.is_empty() {
                    let stored = crate::storage::passwords::stored_password(
                        &state.conn,
                        &state.pwd_key,
                        origin,
                        username,
                    )
                    .unwrap_or(None);
                    if stored.as_deref() != Some(password.as_str()) {
                        let is_update = stored.is_some();
                        state.pending_pwd_save =
                            Some((origin.clone(), username.clone(), password.clone()));
                        let _ = chrome.evaluate_script(&format!(
                            "window.__neura && window.__neura.showSavePassword({},{},{})",
                            serde_json::to_string(origin).unwrap_or_default(),
                            serde_json::to_string(username).unwrap_or_default(),
                            is_update
                        ));
                    }
                }
            }

            // Image bytes (or failure) returned by the content WebView for a "Save image as".
            Event::UserEvent(AppEvent::SaveImageData { id, ok, data }) => {
                if let Some(pending) = pending_image_saves.remove(&id) {
                    let PendingImageSave { dest, url, referer } = pending;
                    let dest_str = dest.to_string_lossy().to_string();
                    let proxy_sa = proxy_main.clone();
                    if ok && data.starts_with("data:") {
                        // The page successfully read the image — decode and write it.
                        rt.spawn(async move {
                            let success = decode_data_url(&data)
                                .and_then(|bytes| Ok(std::fs::write(&dest, bytes)?))
                                .is_ok();
                            let _ = proxy_sa.send_event(AppEvent::DownloadCompleted {
                                url,
                                path: Some(dest_str),
                                success,
                            });
                        });
                    } else if url.starts_with("http") {
                        // The page couldn't read it (cross-origin, no CORS) — fetch server-side,
                        // where CORS doesn't apply, using the page URL as Referer.
                        rt.spawn(async move {
                            let success = match fetch_image_bytes(&url, &referer).await {
                                Ok(bytes) => std::fs::write(&dest, bytes).is_ok(),
                                Err(_) => false,
                            };
                            let _ = proxy_sa.send_event(AppEvent::DownloadCompleted {
                                url,
                                path: Some(dest_str),
                                success,
                            });
                        });
                    } else {
                        // blob:/other scheme the page failed on — nothing more we can do.
                        let _ = proxy_sa.send_event(AppEvent::DownloadCompleted {
                            url,
                            path: Some(dest_str),
                            success: false,
                        });
                    }
                }
            }

            // F12 / OpenDevtools — open Chrome DevTools for the active content WebView.
            // `open_devtools()` may create a new child HWND inside the main window that
            // initially appears black while loading.  Re-applying the layout immediately
            // after restores the Chrome WebView Z-order so the browser UI stays visible.
            Event::UserEvent(AppEvent::Chrome(ChromeCommand::OpenDevtools)) => {
                tracing::info!(target: "ventus::feature", action = "open_devtools", "feature action");
                if let Some(ref id) = state.tab_manager.active_tab_id.clone() {
                    if let Some(wv) = content_views.get(id) {
                        wv.open_devtools();
                    }
                }
                apply_layout(
                    &chrome,
                    chrome_hwnd,
                    &content_views,
                    &state,
                    &layout_config,
                    &window,
                );
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::WindowDragStart)) => {
                tracing::info!(target: "ventus::window", action = "drag", "window action");
                let _ = window.drag_window();
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::WindowMinimize)) => {
                tracing::info!(target: "ventus::window", action = "minimize", "window action");
                window.set_minimized(true);
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::WindowMaximize)) => {
                if state.content_fullscreen {
                    return;
                }
                tracing::info!(
                    target: "ventus::window",
                    action = if window.is_maximized() { "restore" } else { "maximize" },
                    "window action"
                );
                custom_maximized = window.is_maximized();
                toggle_window_maximized(&window, &mut custom_maximized);
                sync_window_maximized(&chrome, custom_maximized);
                apply_layout(
                    &chrome,
                    chrome_hwnd,
                    &content_views,
                    &state,
                    &layout_config,
                    &window,
                );
            }

            Event::UserEvent(AppEvent::Chrome(ChromeCommand::WindowClose)) => {
                tracing::info!(target: "ventus::window", action = "close", "window action");
                if !new_window {
                    save_window_size(&window, &mut state, custom_maximized);
                    save_session(&state);
                }
                save_open_cookies(&content_views, &state, &data_dir);
                #[cfg(windows)]
                {
                    let _ = auth_window.take();
                }
                popups.clear();
                close_chrome_controller(&chrome);
                shutdown_webview2(
                    crash_sentinel.as_deref(),
                    &[webview_data_dir.as_path(), incognito_data_dir.as_path()],
                    &mut content_views,
                    &mut content_hwnds,
                    &mut content_web_context,
                    &mut incognito_web_context,
                );
                if new_window {
                    std::fs::remove_dir_all(&webview_data_dir).ok();
                }
                *control_flow = ControlFlow::Exit;
            }

            Event::UserEvent(AppEvent::CreatePopupWindow) => {
                #[cfg(windows)]
                {
                    let pendings: Vec<PendingPopup> =
                        PENDING_POPUPS.with(|q| q.borrow_mut().drain(..).collect());
                    for pending in pendings {
                        let id = next_popup_id;
                        next_popup_id = next_popup_id.wrapping_add(1).max(1);
                        let web_ctx = if pending.incognito {
                            incognito_web_context.as_mut().unwrap()
                        } else {
                            content_web_context.as_mut().unwrap()
                        };
                        let popup = spawn_popup_window(
                            elwt,
                            &window,
                            &pending,
                            id,
                            proxy_main.clone(),
                            web_ctx,
                            &browser_args,
                        );
                        unsafe {
                            let _ = pending.deferral.Complete();
                        }
                        if let Some(p) = popup {
                            p.window.set_visible(true);
                            popups.insert(id, p);
                        }
                    }
                }
            }

            Event::UserEvent(AppEvent::PopupClose { id }) => {
                popups.remove(&id);
            }

            Event::UserEvent(AppEvent::PopupDrag { id }) => {
                if let Some(p) = popups.get(&id) {
                    let _ = p.window.drag_window();
                }
            }

            Event::UserEvent(AppEvent::PopupUrlChanged { id, url }) => {
                if let Some(p) = popups.get(&id) {
                    let (host, secure) = popup_origin(&url);
                    let host_js = serde_json::to_string(&host).unwrap_or_default();
                    let _ = p.bar.evaluate_script(&format!(
                        "window.__popup&&window.__popup.setOrigin({},{})",
                        host_js, secure
                    ));
                }
            }

            Event::UserEvent(AppEvent::Shortcut { code }) => {
                run_shortcut(code as usize, &proxy_main, &state);
            }

            Event::UserEvent(AppEvent::SaveSession { id }) => {
                if save_id == id {
                    save_id = 0;
                    save_session(&state);
                }
            }

            Event::UserEvent(AppEvent::SyncPush { id }) => {
                if sync_id == id {
                    sync_id = 0;
                    let (db, dh, ds) = sync_dirty;
                    sync_dirty = (false, false, false);
                    if let Some(session) = state.auth.clone() {
                        let bookmarks = db.then(|| cloud_bookmarks_blob(&state));
                        let history = dh.then(|| cloud_history_blob(&state));
                        let settings = ds.then(|| {
                            serde_json::to_string(&state.settings).unwrap_or_default()
                        });
                        rt.spawn(cloud::push_blobs(session, bookmarks, history, settings));
                    }
                }
            }

            Event::UserEvent(AppEvent::ContentProcessFailed { tab_id, fatal }) => {
                tracing::warn!(
                    target: "ventus::content",
                    tab = %tab_id,
                    fatal,
                    "content process failed"
                );
                let url = state
                    .tab_manager
                    .tabs
                    .iter()
                    .find(|t| t.id == tab_id)
                    .map(|t| t.url.clone())
                    .unwrap_or_default();
                if url.is_empty() || url.starts_with("neura://") {
                    return;
                }
                if !fatal {
                    if let Some(wv) = content_views.get(&tab_id) {
                        let active = state.tab_manager.active_tab_id.as_deref()
                            == Some(tab_id.as_str());
                        wake_content_webview(wv);
                        if active {
                            let _ = wv.focus();
                            if state
                                .tab_manager
                                .get_tab(&tab_id)
                                .map(|tab| tab.status == crate::browser::tab::TabStatus::Loading)
                                .unwrap_or(false)
                            {
                                watch_load(
                                    &rt,
                                    &proxy_main,
                                    &mut load_watches,
                                    &mut load_watch_next,
                                    tab_id.clone(),
                                    url,
                                );
                            }
                        }
                        apply_layout(
                            &chrome,
                            chrome_hwnd,
                            &content_views,
                            &state,
                            &layout_config,
                            &window,
                        );
                        return;
                    }
                }
                if state.tab_manager.active_tab_id.as_deref() != Some(tab_id.as_str()) {
                    content_views.remove(&tab_id);
                    content_hwnds.remove(&tab_id);
                    suspended_tabs.remove(&tab_id);
                    clear_load_watches(&mut load_watches, &tab_id);
                    state.load_progress.remove(&tab_id);
                    return;
                }
                content_views.remove(&tab_id);
                content_hwnds.remove(&tab_id);
                suspended_tabs.remove(&tab_id);
                clear_load_watches(&mut load_watches, &tab_id);
                let layout = AppLayout::calculate(
                    layout_size(&window, &state),
                    window.scale_factor(),
                    &state,
                    &layout_config,
                );
                let is_incog = state.tab_manager.tab_is_incognito(&tab_id);
                let ctx = if is_incog {
                    incognito_web_context.as_mut().unwrap()
                } else {
                    content_web_context.as_mut().unwrap()
                };
                let ad_script = state.ad_block_engine.init_script().to_string();
                match build_content_webview(
                    &window,
                    &tab_id,
                    &url,
                    layout.content,
                    proxy_main.clone(),
                    ctx,
                    is_incog,
                    std::sync::Arc::clone(&shared_dl_dir),
                    tab_zoom(&state, &tab_id),
                    &browser_args,
                    ad_script,
                    state.settings.privacy.fingerprint_protection,
                    state.settings.privacy.strict_permissions,
                    state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                    state.settings.privacy.https_only,
                    false,
                ) {
                    Ok(wv) => {
                        #[cfg(windows)]
                        let hwnd = webview_hwnd(&wv);
                        restore_startup_cookies(
                            &wv,
                            is_incog,
                            &startup_cookies,
                            &mut cookies_restored,
                        );
                        content_views.insert(tab_id.clone(), wv);
                        begin_native_load(&mut state, &chrome, &tab_id);
                        #[cfg(windows)]
                        track_content_hwnd(hwnd, &tab_id, &mut content_hwnds);
                        apply_layout(
                            &chrome,
                            chrome_hwnd,
                            &content_views,
                            &state,
                            &layout_config,
                            &window,
                        );
                        if let Some(wv) = content_views.get(&tab_id) {
                            let _ = wv.load_url(&url);
                        }
                        watch_load(
                            &rt,
                            &proxy_main,
                            &mut load_watches,
                            &mut load_watch_next,
                            tab_id.clone(),
                            url,
                        );
                    }
                    Err(e) => tracing::error!("recover content process: {}", e),
                }
            }

            Event::UserEvent(AppEvent::CoverWatchdog { id }) => {
                if id == cover_watch_id && state.content_cover_open {
                    state.set_content_cover(&chrome, false);
                    apply_layout(
                        &chrome,
                        chrome_hwnd,
                        &content_views,
                        &state,
                        &layout_config,
                        &window,
                    );
                }
            }

            #[cfg(windows)]
            Event::UserEvent(AppEvent::AccelProbe { id, url }) => {
                let referer = state
                    .tab_manager
                    .active_tab()
                    .map(|t| t.url.clone())
                    .unwrap_or_default();
                let ua = browser_user_agent();
                let p = proxy_main.clone();
                rt.spawn(async move {
                    let res = browser::accel_download::probe(&url, &ua, &referer).await;
                    let (accelerate, final_url, total) = match res {
                        Some(pr) => (true, pr.final_url, pr.total),
                        None => (false, url.clone(), 0),
                    };
                    let _ = p.send_event(AppEvent::AccelDecision {
                        id,
                        accelerate,
                        url,
                        final_url,
                        total,
                        referer,
                    });
                });
            }

            #[cfg(windows)]
            Event::UserEvent(AppEvent::AccelDecision {
                id,
                accelerate,
                url,
                final_url,
                total,
                referer,
            }) => {
                use wv2win::Win32::Foundation::BOOL;
                let entry = DOWNLOAD_DEFERRALS.with(|m| m.borrow_mut().remove(&id));
                if let Some((deferral, args, default_name)) = entry {
                    match resolve_download_target(&default_name, &shared_dl_dir) {
                        None => unsafe {
                            let _ = args.SetCancel(BOOL::from(true));
                            let _ = deferral.Complete();
                        },
                        Some(target) if accelerate => {
                            unsafe {
                                let _ = args.SetCancel(BOOL::from(true));
                                let _ = deferral.Complete();
                            }
                            let filename = target
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&default_name)
                                .to_string();
                            let path_str = target.to_string_lossy().to_string();
                            let ctl = browser::accel_download::AccelControl::new();
                            ACCEL_DOWNLOADS.with(|m| m.borrow_mut().insert(id.clone(), ctl.clone()));
                            let _ = proxy_main.send_event(AppEvent::DownloadStarted {
                                id: Some(id.clone()),
                                url,
                                filename,
                                path: path_str,
                                total: Some(total),
                            });
                            tracing::info!(target: "ventus::dl", id = %id, total, "accelerated download started");
                            rt.spawn(browser::accel_download::run(
                                final_url,
                                total,
                                browser_user_agent(),
                                referer,
                                target,
                                id,
                                ctl,
                                proxy_main.clone(),
                            ));
                        }
                        Some(target) => {
                            begin_native_download(&args, &target, url, &proxy_main);
                            unsafe {
                                let _ = deferral.Complete();
                            }
                        }
                    }
                }
            }

            Event::UserEvent(AppEvent::WebNotification {
                tab_id,
                id,
                title,
                body,
                icon,
                origin,
            }) => {
                tracing::info!(
                    target: "ventus::notifications",
                    tab = %tab_id,
                    origin = %origin,
                    title_chars = title.chars().count(),
                    body_chars = body.chars().count(),
                    "web notification requested"
                );
                let tab = state.tab_manager.get_tab(&tab_id);
                let site = notification_site(&origin, tab.map(|t| t.url.as_str()).unwrap_or(""));
                let icons =
                    notification_icons(&icon, &origin, tab.and_then(|t| t.favicon.as_deref()));
                let proxy_ready = proxy_main.clone();
                rt.spawn(async move {
                    let icon = cache_notification_icon(icons).await;
                    let _ = proxy_ready.send_event(AppEvent::WebNotificationReady {
                        tab_id,
                        id,
                        title,
                        body,
                        site,
                        icon,
                    });
                });
            }
            Event::UserEvent(AppEvent::WebNotificationReady {
                tab_id,
                id,
                title,
                body,
                site,
                icon,
            }) => {
                tracing::info!(
                    target: "ventus::notifications",
                    tab = %tab_id,
                    site = %site,
                    has_icon = !icon.is_empty(),
                    "web notification ready"
                );
                let proxy_click = proxy_main.clone();
                let proxy_close = proxy_main.clone();
                let (tc, ic) = (tab_id.clone(), id.clone());
                let (td, idd) = (tab_id.clone(), id.clone());
                notify::show(
                    &id,
                    &title,
                    &body,
                    &site,
                    &icon,
                    Box::new(move || {
                        let _ = proxy_click.send_event(AppEvent::NotificationClicked {
                            tab_id: tc.clone(),
                            id: ic.clone(),
                        });
                    }),
                    Box::new(move || {
                        let _ = proxy_close.send_event(AppEvent::NotificationClosed {
                            tab_id: td.clone(),
                            id: idd.clone(),
                        });
                    }),
                );
            }
            Event::UserEvent(AppEvent::WebNotificationClose { id, .. }) => {
                tracing::info!(target: "ventus::notifications", id = %id, "web notification closed");
                notify::hide(&id);
            }
            Event::UserEvent(AppEvent::NotificationClicked { tab_id, id }) => {
                tracing::info!(
                    target: "ventus::notifications",
                    tab = %tab_id,
                    id = %id,
                    "web notification clicked"
                );
                window.set_minimized(false);
                window.set_focus();
                if state.tab_manager.active_tab_id.as_deref() != Some(tab_id.as_str())
                    && state.tab_manager.get_tab(&tab_id).is_some()
                {
                    let _ = proxy_main.send_event(AppEvent::Chrome(ChromeCommand::SwitchTab {
                        id: tab_id.clone(),
                    }));
                }
                if let Some(wv) = content_views.get(&tab_id) {
                    let _ = wv.evaluate_script(&format!(
                        "window.__neuraNotifClick&&window.__neuraNotifClick({})",
                        serde_json::to_string(&id).unwrap_or_default()
                    ));
                }
            }
            Event::UserEvent(AppEvent::NotificationClosed { tab_id, id }) => {
                tracing::info!(
                    target: "ventus::notifications",
                    tab = %tab_id,
                    id = %id,
                    "web notification dismissed"
                );
                if let Some(wv) = content_views.get(&tab_id) {
                    let _ = wv.evaluate_script(&format!(
                        "window.__neuraNotifClose&&window.__neuraNotifClose({})",
                        serde_json::to_string(&id).unwrap_or_default()
                    ));
                }
            }
            Event::UserEvent(app_event) => {
                if matches!(
                    &app_event,
                    AppEvent::Chrome(ChromeCommand::ToggleFullscreen)
                ) {
                    let now = Instant::now();
                    if last_fullscreen_toggle
                        .map(|last| now.duration_since(last) < Duration::from_millis(250))
                        .unwrap_or(false)
                    {
                        return;
                    }
                    last_fullscreen_toggle = Some(now);
                }

                if let AppEvent::ContentLoadStalled { tab_id, url, watch } = &app_event {
                    let key = app::load_key(tab_id, url);
                    if load_watches.get(&key).copied() != Some(*watch) {
                        return;
                    }
                    load_watches.remove(&key);
                }
                if let AppEvent::ContentBlackProbe { tab_id, url, watch } = &app_event {
                    // Validate the watch is still current, but DO NOT consume it: if the probe
                    // decides not to act (page committed, or not yet black), the full 6s stall
                    // timer must still be allowed to fire on this same load.
                    let key = app::load_key(tab_id, url);
                    if load_watches.get(&key).copied() != Some(*watch) {
                        return;
                    }
                }
                if let AppEvent::ContentLoadEnd { tab_id, url, .. } = &app_event {
                    clear_load_watches(&mut load_watches, tab_id);
                    // Snapshot cookies after each page load into our isolated store.
                    // Skip internal pages and incognito tabs (they never share cookies).
                    if !url.starts_with("neura://") && !state.tab_manager.tab_is_incognito(tab_id) {
                        if let Some(wv) = content_views.get(tab_id.as_str()) {
                            browser::cookie_manager::trigger_save(wv, cookie_tx.clone());
                        }
                    }
                    // Page navigation creates a new window context, resetting any JS
                    // globals we injected via evaluate_script (including __vLeftEdge).
                    // Re-inject it so the resize-cursor guard stays correct after load.
                    let nav_layout = AppLayout::calculate(
                        layout_size(&window, &state),
                        window.scale_factor(),
                        &state,
                        &layout_config,
                    );
                    if let Some(wv) = content_views.get(tab_id.as_str()) {
                        let _ = wv.evaluate_script(&format!(
                            "window.__vLeftEdge={}",
                            nav_layout.content.x.max(0)
                        ));
                    }
                }
                if let AppEvent::ContentNavigationFailed { tab_id, url, .. } = &app_event {
                    load_watches.remove(&app::load_key(
                        tab_id,
                        &crate::utils::url::clean_tracking_url(url),
                    ));
                }
                let quiet_nav_event = repeated_native_load_start(&state, &app_event)
                    || stale_canceled_nav_failure(&state, &app_event);
                if let AppEvent::ContentLoadStart {
                    tab_id,
                    url,
                    native,
                    ..
                } = &app_event
                {
                    if *native && !quiet_nav_event {
                        watch_load(
                            &rt,
                            &proxy_main,
                            &mut load_watches,
                            &mut load_watch_next,
                            tab_id.clone(),
                            crate::utils::url::clean_tracking_url(url),
                        );
                    }
                }
                let persist_session = should_save_session(&app_event);
                let is_save_settings = matches!(
                    &app_event,
                    AppEvent::Chrome(
                        ChromeCommand::SaveSettings { .. } | ChromeCommand::BrowseDownloadFolder
                    )
                );
                let progress_tab_id = match &app_event {
                    AppEvent::ContentLoadProgress { tab_id, .. } => Some(tab_id.clone()),
                    _ => None,
                };
                let load_end_tab_id = match &app_event {
                    AppEvent::ContentLoadEnd { tab_id, .. } => Some(tab_id.clone()),
                    _ => None,
                };
                let defer_session = matches!(&app_event, AppEvent::ContentMetadata { .. });
                let nav_state_event = matches!(
                    &app_event,
                    AppEvent::ContentLoadStart { .. }
                        | AppEvent::ContentLoadEnd { .. }
                        | AppEvent::ContentNav { .. }
                        | AppEvent::ContentNavState { .. }
                        | AppEvent::ContentNavigationFailed { .. }
                );
                let auth_terminal = matches!(
                    &app_event,
                    AppEvent::AuthApplied { .. } | AppEvent::AuthError { .. }
                );
                let sync_kinds = cloud_sync_kinds(&app_event);
                let cover_before = state.content_cover_open;
                let action_opt = handle_app_event_inner(app_event, &mut state, &chrome);
                #[cfg(windows)]
                if auth_terminal {
                    auth_window = None;
                }
                #[cfg(not(windows))]
                let _ = auth_terminal;
                if let Some((db, dh, ds)) = sync_kinds {
                    if state.auth.is_some() {
                        sync_dirty.0 |= db;
                        sync_dirty.1 |= dh;
                        sync_dirty.2 |= ds;
                        queue_cloud_sync(&rt, &proxy_main, &mut sync_id);
                    }
                }
                clear_stale_cover(&mut state, &chrome);
                let cover_cleared = cover_before && !state.content_cover_open;
                if let Some(tab_id) = load_end_tab_id {
                    let done = state
                        .tab_manager
                        .get_tab(&tab_id)
                        .map(|tab| tab.status == crate::browser::tab::TabStatus::Complete)
                        .unwrap_or(false)
                        && !state.native_loads.contains_key(&tab_id);
                    if done {
                        restored_tabs.remove(&tab_id);
                    }
                }
                if let Some(tab_id) = progress_tab_id {
                    if state
                        .tab_manager
                        .get_tab(&tab_id)
                        .map(|tab| tab.status != crate::browser::tab::TabStatus::Loading)
                        .unwrap_or(true)
                    {
                        clear_load_watches(&mut load_watches, &tab_id);
                    }
                }
                // Keep the shared download dir in sync whenever any setting is saved.
                if is_save_settings {
                    if let Ok(mut guard) = shared_dl_dir.lock() {
                        *guard = download_prefs_from_settings(&state.settings);
                    }
                }
                if nav_state_event && !quiet_nav_event {
                    refresh_nav_buttons(&chrome, &content_views, &mut state);
                }
                if let Some(action) = action_opt {
                    sync_active_ubol(
                        &content_views,
                        &state,
                        ubol_dir.as_deref(),
                        &mut ubol_done,
                        &mut ubol_enabled,
                        &mut ubol_tab,
                    );
                    if persist_session {
                        if defer_session {
                            queue_session_save(&rt, &proxy_main, &mut save_id);
                        } else {
                            save_id = 0;
                            save_session(&state);
                        }
                    }
                    if matches!(action, TabAction::SyncClipOnly | TabAction::SyncSidebarClip) {
                        let layout = AppLayout::calculate(
                            layout_size(&window, &state),
                            window.scale_factor(),
                            &state,
                            &layout_config,
                        );
                        sync_chrome_clip(chrome_hwnd, &state, layout);
                        #[cfg(windows)]
                        sync_content_clip(&content_views, &state, layout);
                        #[cfg(windows)]
                        {
                            let repaint = matches!(action, TabAction::SyncClipOnly);
                            sync_content_z_order(&content_views, chrome_hwnd, &state, repaint);
                        }
                        return;
                    }

                    let cover = action_content_cover(&action, &state, &content_views);
                    state.set_content_cover(&chrome, cover);
                    if cover {
                        arm_cover_watch(&rt, &proxy_main, &mut cover_watch_id);
                    }
                    apply_layout(
                        &chrome,
                        chrome_hwnd,
                        &content_views,
                        &state,
                        &layout_config,
                        &window,
                    );
                    let layout = AppLayout::calculate(
                        layout_size(&window, &state),
                        window.scale_factor(),
                        &state,
                        &layout_config,
                    );

                    let focus_spotlight = matches!(action, TabAction::FocusSpotlight);
                    match action {
                        TabAction::SyncClipOnly | TabAction::SyncSidebarClip => unreachable!(),
                        TabAction::ResolvePermission { origin, key, allow } => {
                            #[cfg(windows)]
                            resolve_permission(&origin, &key, allow);
                            #[cfg(not(windows))]
                            let _ = (origin, key, allow);
                        }
                        TabAction::SendReport(report) => {
                            let proxy_r = proxy_main.clone();
                            rt.spawn(async move {
                                match cloud::report::send(*report).await {
                                    Ok(()) => {
                                        tracing::info!(target: "ventus::report", "report uploaded");
                                        let _ = proxy_r.send_event(AppEvent::ReportSent { ok: true });
                                    }
                                    Err(e) => {
                                        tracing::warn!(target: "ventus::report", error = %e, "report upload failed");
                                        let _ = proxy_r.send_event(AppEvent::ReportSent { ok: false });
                                    }
                                }
                            });
                        }
                        TabAction::Create { tab_id, url } => {
                            if !url.starts_with("neura://") {
                                let is_incog = state.tab_manager.tab_is_incognito(&tab_id);
                                let ctx = if is_incog {
                                    incognito_web_context.as_mut().unwrap()
                                } else {
                                    content_web_context.as_mut().unwrap()
                                };
                                let ad_script = state.ad_block_engine.init_script().to_string();
                                match build_content_webview(
                                    &window,
                                    &tab_id,
                                    &url,
                                    layout.content,
                                    proxy_main.clone(),
                                    ctx,
                                    is_incog,
                                    std::sync::Arc::clone(&shared_dl_dir),
                                    tab_zoom(&state, &tab_id),
                                    &browser_args,
                                    ad_script,
                                    state.settings.privacy.fingerprint_protection,
                                    state.settings.privacy.strict_permissions,
                                    state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                                    state.settings.privacy.https_only,
                                    false,
                                ) {
                                    Ok(wv) => {
                                        #[cfg(windows)]
                                        let hwnd = webview_hwnd(&wv);
                                        restore_startup_cookies(
                                            &wv,
                                            is_incog,
                                            &startup_cookies,
                                            &mut cookies_restored,
                                        );
                                        content_views.insert(tab_id.clone(), wv);
                                        #[cfg(windows)]
                                        track_content_hwnd(hwnd, &tab_id, &mut content_hwnds);
                                        apply_layout(
                                            &chrome,
                                            chrome_hwnd,
                                            &content_views,
                                            &state,
                                            &layout_config,
                                            &window,
                                        );
                                        if let Some(wv) = content_views.get(&tab_id) {
                                            let _ = wv.load_url(&url);
                                        }
                                        watch_load(
                                            &rt,
                                            &proxy_main,
                                            &mut load_watches,
                                            &mut load_watch_next,
                                            tab_id.clone(),
                                            url.clone(),
                                        );
                                    }
                                    Err(e) => tracing::error!("create content view: {}", e),
                                }
                            }
                        }
                        TabAction::Remove(id) => {
                            content_views.remove(&id);
                            content_hwnds.remove(&id);
                            suspended_tabs.remove(&id);
                            clear_load_watches(&mut load_watches, &id);
                            apply_layout(
                                &chrome,
                                chrome_hwnd,
                                &content_views,
                                &state,
                                &layout_config,
                                &window,
                            );
                        }
                        TabAction::RemoveMany(ids) => {
                            for id in ids {
                                content_views.remove(&id);
                                content_hwnds.remove(&id);
                                suspended_tabs.remove(&id);
                                clear_load_watches(&mut load_watches, &id);
                            }
                            apply_layout(
                                &chrome,
                                chrome_hwnd,
                                &content_views,
                                &state,
                                &layout_config,
                                &window,
                            );
                        }
                        TabAction::DropContent { tab_id } => {
                            content_views.remove(&tab_id);
                            content_hwnds.remove(&tab_id);
                            clear_load_watches(&mut load_watches, &tab_id);
                            apply_layout(
                                &chrome,
                                chrome_hwnd,
                                &content_views,
                                &state,
                                &layout_config,
                                &window,
                            );
                        }
                        TabAction::SyncViews | TabAction::FocusSpotlight => {
                            if let Some(active_id) = state.tab_manager.active_tab_id.clone() {
                                if !content_views.contains_key(&active_id) {
                                    if let Some(tab) = state.tab_manager.get_tab(&active_id) {
                                        let url = tab.url.clone();
                                        if !url.starts_with("neura://") {
                                            let rect = AppLayout::calculate(
                                                layout_size(&window, &state),
                                                window.scale_factor(),
                                                &state,
                                                &layout_config,
                                            )
                                            .content;
                                            let is_incog =
                                                state.tab_manager.tab_is_incognito(&active_id);
                                            let ctx = if is_incog {
                                                incognito_web_context.as_mut().unwrap()
                                            } else {
                                                content_web_context.as_mut().unwrap()
                                            };
                                            let ad_script =
                                                state.ad_block_engine.init_script().to_string();
                                            if let Ok(wv) = build_content_webview(
                                                &window,
                                                &active_id,
                                                &url,
                                                rect,
                                                proxy_main.clone(),
                                                ctx,
                                                is_incog,
                                                std::sync::Arc::clone(&shared_dl_dir),
                                                tab_zoom(&state, &active_id),
                                                &browser_args,
                                                ad_script,
                                                state.settings.privacy.fingerprint_protection,
                                                state.settings.privacy.strict_permissions,
                                                state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                                                state.settings.privacy.https_only,
                                                false,
                                            ) {
                                                #[cfg(windows)]
                                                let hwnd = webview_hwnd(&wv);
                                                restore_startup_cookies(
                                                    &wv,
                                                    is_incog,
                                                    &startup_cookies,
                                                    &mut cookies_restored,
                                                );
                                                content_views.insert(active_id.clone(), wv);
                                                #[cfg(windows)]
                                                track_content_hwnd(
                                                    hwnd,
                                                    &active_id,
                                                    &mut content_hwnds,
                                                );
                                                apply_layout(
                                                    &chrome,
                                                    chrome_hwnd,
                                                    &content_views,
                                                    &state,
                                                    &layout_config,
                                                    &window,
                                                );
                                                if let Some(wv) = content_views.get(&active_id) {
                                                    let _ = wv.load_url(&url);
                                                }
                                                watch_load(
                                                    &rt,
                                                    &proxy_main,
                                                    &mut load_watches,
                                                    &mut load_watch_next,
                                                    active_id.clone(),
                                                    url.clone(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            // Sync screenshot protection whenever layout is re-evaluated,
                            // which covers all workspace switches, tab creates, and mode changes.
                            #[cfg(windows)]
                            {
                                let is_incog = state
                                    .tab_manager
                                    .active_workspace()
                                    .map(|w| w.is_incognito)
                                    .unwrap_or(false);
                                set_screenshot_protection(window.hwnd() as isize, is_incog);
                            }
                            if focus_spotlight {
                                let _ = chrome.focus();
                                let _ = chrome.evaluate_script(
                                    "window.__neura&&window.__neura.focusSpotlight&&window.__neura.focusSpotlight()",
                                );
                            }
                        }
                        TabAction::ContentScript(js) => {
                            if let Some(id) = &state.tab_manager.active_tab_id {
                                if let Some(wv) = content_views.get(id) {
                                    let _ = wv.evaluate_script(&js);
                                }
                            }
                        }
                        TabAction::ContentScriptOnTab { tab_id, js } => {
                            if let Some(wv) = content_views.get(&tab_id) {
                                let _ = wv.evaluate_script(&js);
                            }
                        }
                        TabAction::ContentGoBack => {
                            if let Some(id) = &state.tab_manager.active_tab_id {
                                if let Some(wv) = content_views.get(id) {
                                    let _ = wv.go_back();
                                }
                            }
                        }
                        TabAction::ContentGoForward => {
                            if let Some(id) = &state.tab_manager.active_tab_id {
                                if let Some(wv) = content_views.get(id) {
                                    let _ = wv.go_forward();
                                }
                            }
                        }
                        TabAction::ReadClipboardForOmnibox => {
                            let text = read_clipboard_text().unwrap_or_default();
                            let json =
                                serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".into());
                            let _ = chrome.evaluate_script(&format!(
                                "window.__neura && window.__neura.applyClipboardPaste({})",
                                json
                            ));
                        }
                        TabAction::FindInPage {
                            tab_id,
                            query,
                            forward,
                        } => {
                            let result = find_empty_result(&query);
                            let Some(wv) = content_views.get(&tab_id) else {
                                let _ = proxy_main.send_event(AppEvent::FindResult {
                                    tab_id,
                                    result,
                                });
                                return;
                            };
                            let js = find_page_script(&query, forward);
                            let proxy_find = proxy_main.clone();
                            let tab_id_cb = tab_id.clone();
                            let failed = wv
                                .evaluate_script_with_callback(&js, move |result| {
                                    let _ = proxy_find.send_event(AppEvent::FindResult {
                                        tab_id: tab_id_cb.clone(),
                                        result,
                                    });
                                })
                                .is_err();
                            if failed {
                                let _ = proxy_main.send_event(AppEvent::FindResult {
                                    tab_id,
                                    result,
                                });
                            }
                        }
                        TabAction::SetZoom(level) => {
                            if let Some(id) = &state.tab_manager.active_tab_id {
                                if let Some(wv) = content_views.get(id) {
                                    let _ = wv.zoom(level);
                                }
                            }
                        }
                        TabAction::SetZoomFor { tab_id, level } => {
                            if let Some(wv) = content_views.get(&tab_id) {
                                let _ = wv.zoom(level);
                            }
                        }
                        TabAction::SetZoomAll(level) => {
                            for wv in content_views.values() {
                                let _ = wv.zoom(level);
                            }
                        }
                        TabAction::ContentNavigate(url) => {
                            let watch_tab_id = state.tab_manager.active_tab_id.clone();
                            if let Some(id) = watch_tab_id.clone() {
                                if !url.starts_with("neura://") {
                                    watch_load(
                                        &rt,
                                        &proxy_main,
                                        &mut load_watches,
                                        &mut load_watch_next,
                                        id,
                                        url.clone(),
                                    );
                                }
                            }
                            apply_layout(
                                &chrome,
                                chrome_hwnd,
                                &content_views,
                                &state,
                                &layout_config,
                                &window,
                            );
                            if let Some(id) = &state.tab_manager.active_tab_id {
                                if url.starts_with("neura://") {
                                    content_views.remove(id);
                                    apply_layout(
                                        &chrome,
                                        chrome_hwnd,
                                        &content_views,
                                        &state,
                                        &layout_config,
                                        &window,
                                    );
                                } else if let Some(wv) = content_views.get(id) {
                                    let _ = wv.load_url(&url);
                                } else {
                                    let is_incog = state.tab_manager.tab_is_incognito(id);
                                    let ctx = if is_incog {
                                        incognito_web_context.as_mut().unwrap()
                                    } else {
                                        content_web_context.as_mut().unwrap()
                                    };
                                    let ad_script = state.ad_block_engine.init_script().to_string();
                                    match build_content_webview(
                                        &window,
                                        id,
                                        &url,
                                        layout.content,
                                        proxy_main.clone(),
                                        ctx,
                                        is_incog,
                                        std::sync::Arc::clone(&shared_dl_dir),
                                        tab_zoom(&state, id),
                                        &browser_args,
                                        ad_script,
                                        state.settings.privacy.fingerprint_protection,
                                        state.settings.privacy.strict_permissions,
                                        state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                                        state.settings.privacy.https_only,
                                        false,
                                    ) {
                                        Ok(wv) => {
                                            #[cfg(windows)]
                                            let hwnd = webview_hwnd(&wv);
                                            restore_startup_cookies(
                                                &wv,
                                                is_incog,
                                                &startup_cookies,
                                                &mut cookies_restored,
                                            );
                                            content_views.insert(id.clone(), wv);
                                            #[cfg(windows)]
                                            track_content_hwnd(hwnd, id, &mut content_hwnds);
                                            apply_layout(
                                                &chrome,
                                                chrome_hwnd,
                                                &content_views,
                                                &state,
                                                &layout_config,
                                                &window,
                                            );
                                            if let Some(wv) = content_views.get(id) {
                                                let _ = wv.load_url(&url);
                                            }
                                            watch_load(
                                                &rt,
                                                &proxy_main,
                                                &mut load_watches,
                                                &mut load_watch_next,
                                                id.clone(),
                                                url.clone(),
                                            );
                                        }
                                        Err(e) => tracing::error!(
                                            "create content view for navigation: {}",
                                            e
                                        ),
                                    }
                                }
                            }
                        }
                        TabAction::ExtendLoadWatch { tab_id, url } => {
                            // The navigation is still in-flight (slow main-document download).
                            // Do NOT touch the WebView or re-navigate — that would abort the
                            // live connection. Just re-arm the stall watchdog so we keep
                            // giving the connection room to commit.
                            if content_views.contains_key(&tab_id) {
                                tracing::info!(
                                    target: "ventus::nav",
                                    tab = %tab_id,
                                    url = %url,
                                    "ExtendLoadWatch: re-arming stall watchdog, keeping the live connection"
                                );
                                watch_load(
                                    &rt,
                                    &proxy_main,
                                    &mut load_watches,
                                    &mut load_watch_next,
                                    tab_id,
                                    url,
                                );
                            }
                        }
                        TabAction::NudgeContent { tab_id, url } => {
                            if let Some(wv) = content_views.get(&tab_id) {
                                let active = state.tab_manager.active_tab_id.as_deref()
                                    == Some(tab_id.as_str());
                                wake_content_webview(wv);
                                if active {
                                    let _ = wv.set_visible(false);
                                    let _ = wv.set_visible(true);
                                    set_content_bounds(wv, layout.content);
                                    let _ = wv.focus();
                                    #[cfg(windows)]
                                    if let Some(&hwnd) = content_hwnds.get(&tab_id) {
                                        repaint_content_webview(hwnd);
                                    }
                                }
                                watch_load(
                                    &rt,
                                    &proxy_main,
                                    &mut load_watches,
                                    &mut load_watch_next,
                                    tab_id,
                                    url,
                                );
                            }
                        }
                        TabAction::ReloadContent { tab_id, url } => {
                            if let Some(wv) = content_views.get(&tab_id) {
                                wake_content_webview(wv);
                                let _ = wv.load_url(&url);
                                watch_load(
                                    &rt,
                                    &proxy_main,
                                    &mut load_watches,
                                    &mut load_watch_next,
                                    tab_id.clone(),
                                    url.clone(),
                                );
                                apply_layout(
                                    &chrome,
                                    chrome_hwnd,
                                    &content_views,
                                    &state,
                                    &layout_config,
                                    &window,
                                );
                            } else {
                                let is_incog = state.tab_manager.tab_is_incognito(&tab_id);
                                let ctx = if is_incog {
                                    incognito_web_context.as_mut().unwrap()
                                } else {
                                    content_web_context.as_mut().unwrap()
                                };
                                let ad_script = state.ad_block_engine.init_script().to_string();
                                match build_content_webview(
                                    &window,
                                    &tab_id,
                                    &url,
                                    layout.content,
                                    proxy_main.clone(),
                                    ctx,
                                    is_incog,
                                    std::sync::Arc::clone(&shared_dl_dir),
                                    tab_zoom(&state, &tab_id),
                                    &browser_args,
                                    ad_script,
                                    state.settings.privacy.fingerprint_protection,
                                    state.settings.privacy.strict_permissions,
                                    state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                                    state.settings.privacy.https_only,
                                    false,
                                ) {
                                    Ok(wv) => {
                                        #[cfg(windows)]
                                        let hwnd = webview_hwnd(&wv);
                                        restore_startup_cookies(
                                            &wv,
                                            is_incog,
                                            &startup_cookies,
                                            &mut cookies_restored,
                                        );
                                        content_views.insert(tab_id.clone(), wv);
                                        #[cfg(windows)]
                                        track_content_hwnd(hwnd, &tab_id, &mut content_hwnds);
                                        apply_layout(
                                            &chrome,
                                            chrome_hwnd,
                                            &content_views,
                                            &state,
                                            &layout_config,
                                            &window,
                                        );
                                        if let Some(wv) = content_views.get(&tab_id) {
                                            let _ = wv.load_url(&url);
                                        }
                                        watch_load(
                                            &rt,
                                            &proxy_main,
                                            &mut load_watches,
                                            &mut load_watch_next,
                                            tab_id.clone(),
                                            url.clone(),
                                        );
                                    }
                                    Err(e) => tracing::error!("reload content view: {}", e),
                                }
                            }
                        }
                        TabAction::ActivateContent {
                            tab_id,
                            url,
                            loading,
                        } => {
                            apply_layout(
                                &chrome,
                                chrome_hwnd,
                                &content_views,
                                &state,
                                &layout_config,
                                &window,
                            );
                            if let Some(wv) = content_views.get(&tab_id) {
                                let restored = restored_tabs.contains(&tab_id);
                                let sleeping = state
                                    .tab_manager
                                    .get_tab(&tab_id)
                                    .map(|tab| tab.sleeping)
                                    .unwrap_or(false);
                                let is_loading = state
                                    .tab_manager
                                    .get_tab(&tab_id)
                                    .map(|tab| {
                                        tab.status == crate::browser::tab::TabStatus::Loading
                                    })
                                    .unwrap_or(loading);
                                let was_suspended = suspended_tabs.remove(&tab_id);
                                state.tab_manager.wake_tab(&tab_id);
                                let _ = wv.focus();
                                let _ = wv.zoom(tab_zoom(&state, &tab_id));
                                if was_suspended {
                                    wake_content_webview(wv);
                                    state.push_state_to_chrome(&chrome);
                                } else if sleeping && !restored {
                                    wake_content_webview(wv);
                                    state.push_state_to_chrome(&chrome);
                                } else if restored && !is_loading {
                                    state.load_recoveries.remove(&app::load_key(&tab_id, &url));
                                    begin_native_load(&mut state, &chrome, &tab_id);
                                    state.push_state_to_chrome(&chrome);
                                    wake_content_webview(wv);
                                    let _ = wv.load_url(&url);
                                    watch_load(
                                        &rt,
                                        &proxy_main,
                                        &mut load_watches,
                                        &mut load_watch_next,
                                        tab_id.clone(),
                                        url.clone(),
                                    );
                                } else if is_loading {
                                    state.load_recoveries.remove(&app::load_key(&tab_id, &url));
                                    watch_load(
                                        &rt,
                                        &proxy_main,
                                        &mut load_watches,
                                        &mut load_watch_next,
                                        tab_id.clone(),
                                        url.clone(),
                                    );
                                }
                            } else {
                                let is_incog = state.tab_manager.tab_is_incognito(&tab_id);
                                let ctx = if is_incog {
                                    incognito_web_context.as_mut().unwrap()
                                } else {
                                    content_web_context.as_mut().unwrap()
                                };
                                let ad_script = state.ad_block_engine.init_script().to_string();
                                match build_content_webview(
                                    &window,
                                    &tab_id,
                                    &url,
                                    layout.content,
                                    proxy_main.clone(),
                                    ctx,
                                    is_incog,
                                    std::sync::Arc::clone(&shared_dl_dir),
                                    tab_zoom(&state, &tab_id),
                                    &browser_args,
                                    ad_script,
                                    state.settings.privacy.fingerprint_protection,
                                    state.settings.privacy.strict_permissions,
                                    state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                                    state.settings.privacy.https_only,
                                    false,
                                ) {
                                    Ok(wv) => {
                                        #[cfg(windows)]
                                        let hwnd = webview_hwnd(&wv);
                                        restore_startup_cookies(
                                            &wv,
                                            is_incog,
                                            &startup_cookies,
                                            &mut cookies_restored,
                                        );
                                        content_views.insert(tab_id.clone(), wv);
                                        state.tab_manager.wake_tab(&tab_id);
                                        begin_native_load(&mut state, &chrome, &tab_id);
                                        state.push_state_to_chrome(&chrome);
                                        state.load_recoveries.remove(&app::load_key(&tab_id, &url));
                                        #[cfg(windows)]
                                        track_content_hwnd(hwnd, &tab_id, &mut content_hwnds);
                                        apply_layout(
                                            &chrome,
                                            chrome_hwnd,
                                            &content_views,
                                            &state,
                                            &layout_config,
                                            &window,
                                        );
                                        if let Some(wv) = content_views.get(&tab_id) {
                                            let _ = wv.focus();
                                        }
                                        if let Some(wv) = content_views.get(&tab_id) {
                                            let _ = wv.load_url(&url);
                                        }
                                        tracing::info!(
                                            target: "ventus::session",
                                            tab = %tab_id,
                                            url = %url,
                                            "[SESSION] woke sleeping/restored tab: content WebView built OK"
                                        );
                                        watch_load(
                                            &rt,
                                            &proxy_main,
                                            &mut load_watches,
                                            &mut load_watch_next,
                                            tab_id.clone(),
                                            url.clone(),
                                        );
                                    }
                                    Err(e) => tracing::error!(
                                        target: "ventus::session",
                                        tab = %tab_id,
                                        url = %url,
                                        error = %e,
                                        "[SESSION] woke sleeping/restored tab but content WebView build FAILED — tab stays black. 0x800700AA = profile still locked"
                                    ),
                                }
                            }
                        }
                        TabAction::RebuildContent { tab_id, url } => {
                            content_views.remove(&tab_id);
                            content_hwnds.remove(&tab_id);
                            suspended_tabs.remove(&tab_id);
                            clear_load_watches(&mut load_watches, &tab_id);
                            let is_incog = state.tab_manager.tab_is_incognito(&tab_id);
                            let ctx = if is_incog {
                                incognito_web_context.as_mut().unwrap()
                            } else {
                                content_web_context.as_mut().unwrap()
                            };
                            let ad_script = state.ad_block_engine.init_script().to_string();
                            match build_content_webview(
                                &window,
                                &tab_id,
                                &url,
                                layout.content,
                                proxy_main.clone(),
                                ctx,
                                is_incog,
                                std::sync::Arc::clone(&shared_dl_dir),
                                tab_zoom(&state, &tab_id),
                                &browser_args,
                                ad_script,
                                state.settings.privacy.fingerprint_protection,
                                state.settings.privacy.strict_permissions,
                                state.settings.privacy.site_permissions.clone(),
state.settings.privacy.default_permissions.clone(),
                                state.settings.privacy.https_only,
                                false,
                            ) {
                                Ok(wv) => {
                                    #[cfg(windows)]
                                    let hwnd = webview_hwnd(&wv);
                                    restore_startup_cookies(
                                        &wv,
                                        is_incog,
                                        &startup_cookies,
                                        &mut cookies_restored,
                                    );
                                    content_views.insert(tab_id.clone(), wv);
                                    #[cfg(windows)]
                                    track_content_hwnd(hwnd, &tab_id, &mut content_hwnds);
                                    apply_layout(
                                        &chrome,
                                        chrome_hwnd,
                                        &content_views,
                                        &state,
                                        &layout_config,
                                        &window,
                                    );
                                    if let Some(wv) = content_views.get(&tab_id) {
                                        let _ = wv.load_url(&url);
                                    }
                                    watch_load(
                                        &rt,
                                        &proxy_main,
                                        &mut load_watches,
                                        &mut load_watch_next,
                                        tab_id.clone(),
                                        url.clone(),
                                    );
                                }
                                Err(e) => tracing::error!("rebuild content view: {}", e),
                            }
                        }
                        TabAction::ApplyWebSecurity => {
                            // DoH / third-party-cookie blocking are baked into the WebView2
                            // environment's browser args, fixed when the environment is created.
                            // The chrome UI and every content tab now share ONE environment (a
                            // single browser process), so it can't be torn down and rebuilt in
                            // place. Relaunch Ventus: the fresh process recreates the shared
                            // environment with the new args and --restore-session reopens every
                            // tab. Session + cookies are snapshotted first, exactly like a normal
                            // exit, so nothing is lost.
                            if new_window {
                                // Secondary windows share the main instance's browser process;
                                // the env args only change when the whole app restarts.
                                let _ = chrome.evaluate_script(
                                    "window.__neura && window.__neura.showSuccess('Restart Ventus to apply privacy changes')",
                                );
                            } else {
                                save_window_size(&window, &mut state, custom_maximized);
                                save_session(&state);
                                save_open_cookies(&content_views, &state, &data_dir);
                                popups.clear();
                                relaunch_self(true);
                                // Close chrome's controller so the shared msedgewebview2.exe can
                                // flush its cookie DB and exit, releasing the profile lock before
                                // the relaunched instance claims it.
                                close_chrome_controller(&chrome);
                                shutdown_webview2(
                                    crash_sentinel.as_deref(),
                                    &[webview_data_dir.as_path(), incognito_data_dir.as_path()],
                                    &mut content_views,
                                    &mut content_hwnds,
                                    &mut content_web_context,
                                    &mut incognito_web_context,
                                );
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                        TabAction::DownloadControl { id, action } => {
                            #[cfg(windows)]
                            control_download(&id, action);
                            #[cfg(not(windows))]
                            {
                                let _ = (&id, action);
                            }
                        }
                        TabAction::DownloadCancelAll => {
                            #[cfg(windows)]
                            cancel_all_downloads();
                        }
                        TabAction::DownloadUpdate(download_url) => {
                            let proxy_dl = proxy_main.clone();
                            rt.spawn(async move {
                                let result =
                                    updater::download_update(&download_url, |received, total| {
                                        let _ =
                                            proxy_dl.send_event(AppEvent::UpdateDownloadProgress {
                                                received,
                                                total,
                                            });
                                    })
                                    .await;
                                match result {
                                    Ok(path) => {
                                        let _ = proxy_dl.send_event(AppEvent::UpdateDownloaded {
                                            path: path.to_string_lossy().to_string(),
                                        });
                                    }
                                    Err(e) => {
                                        let _ =
                                            proxy_dl.send_event(AppEvent::UpdateDownloadFailed {
                                                message: e.to_string(),
                                            });
                                    }
                                }
                            });
                        }
                        TabAction::ApplyUpdate(path) => {
                            if let Err(e) = updater::apply_update(std::path::Path::new(&path)) {
                                let m = serde_json::to_string(&e.to_string()).unwrap_or_default();
                                let _ = chrome.evaluate_script(&format!(
                                    "window.__neura && window.__neura.setUpdateState({{status:'error',error:{}}})",
                                    m
                                ));
                                return;
                            }
                            if !new_window {
                                save_window_size(&window, &mut state, custom_maximized);
                                save_session(&state);
                            }
                            save_open_cookies(&content_views, &state, &data_dir);
                            #[cfg(windows)]
                            {
                                let _ = auth_window.take();
                            }
                            popups.clear();
                            close_chrome_controller(&chrome);
                            shutdown_webview2(
                                crash_sentinel.as_deref(),
                                &[webview_data_dir.as_path(), incognito_data_dir.as_path()],
                                &mut content_views,
                                &mut content_hwnds,
                                &mut content_web_context,
                                &mut incognito_web_context,
                            );
                            if new_window {
                                std::fs::remove_dir_all(&webview_data_dir).ok();
                            }
                            *control_flow = ControlFlow::Exit;
                        }
                        TabAction::SaveImageAs { url } => {
                            let filename = image_filename_from_url(&url);
                            let mut dlg = rfd::FileDialog::new()
                                .set_title("Save image as")
                                .set_file_name(&filename)
                                .add_filter(
                                    "Images",
                                    &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"],
                                )
                                .add_filter("All files", &["*"]);
                            // Default to the configured download folder, else the user's Downloads.
                            let default_dir = {
                                let prefs = shared_dl_dir
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .clone();
                                if prefs.dir.exists() {
                                    Some(prefs.dir)
                                } else {
                                    directories::UserDirs::new()
                                        .and_then(|ud| ud.download_dir().map(|p| p.to_path_buf()))
                                }
                            };
                            if let Some(dir) = default_dir {
                                dlg = dlg.set_directory(dir);
                            }
                            if let Some(dest) = dlg.save_file() {
                                let dest_str = dest.to_string_lossy().to_string();
                                let fname = dest
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(&filename)
                                    .to_string();
                                // Show the download immediately (panel + feedback animation).
                                let _ = proxy_main.send_event(AppEvent::DownloadStarted {
                                    id: None,
                                    url: url.clone(),
                                    filename: fname,
                                    path: dest_str.clone(),
                                    total: None,
                                });
                                if url.starts_with("data:") {
                                    // Self-contained — decode and write without involving the page.
                                    let proxy_sa = proxy_main.clone();
                                    rt.spawn(async move {
                                        let ok = decode_data_url(&url)
                                            .and_then(|bytes| Ok(std::fs::write(&dest, bytes)?))
                                            .is_ok();
                                        let _ = proxy_sa.send_event(AppEvent::DownloadCompleted {
                                            url,
                                            path: Some(dest_str),
                                            success: ok,
                                        });
                                    });
                                } else {
                                    // Fetch inside the active page (cookies, session, blob: access),
                                    // then write the bytes it returns. SaveImageData resolves it.
                                    let active = state.tab_manager.active_tab_id.clone();
                                    let referer = state
                                        .tab_manager
                                        .active_tab()
                                        .map(|t| t.url.clone())
                                        .unwrap_or_default();
                                    let live = active
                                        .as_ref()
                                        .and_then(|id| content_views.get(id))
                                        .is_some();
                                    if live {
                                        let save_id = uuid::Uuid::new_v4().to_string();
                                        let script = save_image_fetch_script(&save_id, &url);
                                        if let Some(wv) =
                                            active.as_ref().and_then(|id| content_views.get(id))
                                        {
                                            let _ = wv.evaluate_script(&script);
                                            pending_image_saves.insert(
                                                save_id,
                                                PendingImageSave {
                                                    dest,
                                                    url,
                                                    referer,
                                                },
                                            );
                                        }
                                    } else {
                                        // No live page (e.g. internal tab) — best-effort server fetch.
                                        let proxy_sa = proxy_main.clone();
                                        rt.spawn(async move {
                                            let ok = match fetch_image_bytes(&url, &referer).await {
                                                Ok(bytes) => std::fs::write(&dest, bytes).is_ok(),
                                                Err(_) => false,
                                            };
                                            let _ = proxy_sa.send_event(AppEvent::DownloadCompleted {
                                                url,
                                                path: Some(dest_str),
                                                success: ok,
                                            });
                                        });
                                    }
                                }
                            }
                        }
                        TabAction::CopyImageToClipboard { url } => {
                            let referer = state
                                .tab_manager
                                .active_tab()
                                .map(|t| t.url.clone())
                                .unwrap_or_default();
                            let proxy_cp = proxy_main.clone();
                            rt.spawn(async move {
                                let result: anyhow::Result<()> = if url.starts_with("data:") {
                                    decode_data_url(&url).and_then(|b| write_image_to_clipboard(&b))
                                } else {
                                    fetch_image_bytes(&url, &referer)
                                        .await
                                        .and_then(|b| write_image_to_clipboard(&b))
                                };
                                let _ = proxy_cp.send_event(AppEvent::CopyImageResult {
                                    success: result.is_ok(),
                                });
                            });
                        }
                        TabAction::SetFullscreen(active) => {
                            let _ = chrome.evaluate_script(&format!(
                                "window.__neura && window.__neura.setContentFullscreen({})",
                                active
                            ));
                            if active {
                                let was_maximized = custom_maximized || window.is_maximized();
                                if fullscreen_restore_maximized.is_none() {
                                    fullscreen_restore_maximized = Some(was_maximized);
                                }
                                if was_maximized {
                                    window.set_maximized(false);
                                    restore_window(&window);
                                    custom_maximized = false;
                                    keep_frameless(&window);
                                }
                                fullscreen_msg.store(true, Ordering::SeqCst);
                                window.set_fullscreen(Some(Fullscreen::Borderless(
                                    window.current_monitor(),
                                )));
                                set_fullscreen_z(&window, true);
                            } else {
                                fullscreen_msg.store(false, Ordering::SeqCst);
                                let should_restore_maximized =
                                    fullscreen_restore_maximized.take().unwrap_or(false);
                                window.set_fullscreen(None);
                                set_fullscreen_z(&window, false);
                                keep_frameless(&window);
                                restore_maximized_after_fullscreen = should_restore_maximized;
                            }
                            apply_layout(
                                &chrome,
                                chrome_hwnd,
                                &content_views,
                                &state,
                                &layout_config,
                                &window,
                            );
                            sync_fullscreen_layout = true;
                            window.request_redraw();
                        }
                    }
                    let current_active = state.tab_manager.active_tab_id.clone();
                    if current_active != last_active_tab_id {
                        #[cfg(windows)]
                        sync_content_z_order(&content_views, chrome_hwnd, &state, true);
                        if let Some(ref id) = current_active {
                            if let Some(wv) = content_views.get(id) {
                                if !chrome_owns_content(&state) {
                                    wake_content_webview(wv);
                                    let _ = wv.focus();
                                }
                            }
                            if let Some(tab) = state.tab_manager.get_tab(id) {
                                let url = tab.url.clone();
                                if tab.status == crate::browser::tab::TabStatus::Loading
                                    && !url.trim().is_empty()
                                    && !url.starts_with("neura://")
                                {
                                    state.load_recoveries.remove(&app::load_key(id, &url));
                                    watch_load(
                                        &rt,
                                        &proxy_main,
                                        &mut load_watches,
                                        &mut load_watch_next,
                                        id.clone(),
                                        url,
                                    );
                                }
                            }
                            #[cfg(windows)]
                            if let Some(&hwnd) = content_hwnds.get(id) {
                                repaint_content_webview(hwnd);
                            }
                        }
                        last_active_tab_id = current_active;
                    }
                } else {
                    if persist_session {
                        if defer_session {
                            queue_session_save(&rt, &proxy_main, &mut save_id);
                        } else {
                            save_id = 0;
                            save_session(&state);
                        }
                    }
                    if cover_cleared {
                        apply_layout(
                            &chrome,
                            chrome_hwnd,
                            &content_views,
                            &state,
                            &layout_config,
                            &window,
                        );
                    }
                }
                sync_active_ubol(
                    &content_views,
                    &state,
                    ubol_dir.as_deref(),
                    &mut ubol_done,
                    &mut ubol_enabled,
                    &mut ubol_tab,
                );
            }

            Event::MainEventsCleared => {
                if clear_stale_cover(&mut state, &chrome) {
                    apply_layout(
                        &chrome,
                        chrome_hwnd,
                        &content_views,
                        &state,
                        &layout_config,
                        &window,
                    );
                }
                if state.content_cover_open && !cover_was_open {
                    arm_cover_watch(&rt, &proxy_main, &mut cover_watch_id);
                } else if !state.content_cover_open && cover_was_open {
                    #[cfg(windows)]
                    {
                        let layout = AppLayout::calculate(
                            layout_size(&window, &state),
                            window.scale_factor(),
                            &state,
                            &layout_config,
                        );
                        nudge_active_content(&content_views, &content_hwnds, &state, layout);
                    }
                }
                cover_was_open = state.content_cover_open;

                let code = shortcut_msg.swap(SC_NONE, Ordering::SeqCst);
                if code != SC_NONE {
                    run_shortcut(code, &proxy_main, &state);
                }
                if f11_msg.swap(false, Ordering::SeqCst) {
                    let _ =
                        proxy_main.send_event(AppEvent::Chrome(ChromeCommand::ToggleFullscreen));
                }
                if restore_maximized_after_fullscreen {
                    set_window_maximized(&window, true, &mut custom_maximized);
                    sync_window_maximized(&chrome, custom_maximized);
                    restore_maximized_after_fullscreen = false;
                }
                if sync_fullscreen_layout {
                    apply_layout(
                        &chrome,
                        chrome_hwnd,
                        &content_views,
                        &state,
                        &layout_config,
                        &window,
                    );
                    sync_fullscreen_layout = false;
                }
                if Instant::now() >= heal_content_at {
                    heal_content_at = Instant::now() + HEAL_CONTENT_EVERY;
                    #[cfg(windows)]
                    {
                        let layout = AppLayout::calculate(
                            layout_size(&window, &state),
                            window.scale_factor(),
                            &state,
                            &layout_config,
                        );
                        heal_active_content(&content_views, &content_hwnds, &state, layout);
                    }
                }
                if Instant::now() >= sleep_check_at {
                    sleep_check_at = Instant::now() + TAB_SLEEP_CHECK_EVERY;
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let free_mb = available_memory_mb();
                    let threshold_ms = sleep_threshold_ms(free_mb) as i64;
                    let active = state.tab_manager.active_tab_id.clone().unwrap_or_default();
                    let mut tabs_changed = false;
                    let to_suspend: Vec<String> = state
                        .tab_manager
                        .tabs
                        .iter()
                        .filter(|t| {
                            t.id != active
                                && !t.is_neura_page()
                                && t.status != crate::browser::tab::TabStatus::Loading
                                && !t.is_media_active
                                && !tab_notifications_allowed(&t.url, &state.settings)
                                && content_views.contains_key(&t.id)
                                && !t.sleeping
                                && !suspended_tabs.contains(&t.id)
                                && (now_ms - t.last_active_at) >= SUSPEND_IDLE_MS
                        })
                        .map(|t| t.id.clone())
                        .collect();
                    for id in to_suspend {
                        if let Some(wv) = content_views.get(&id) {
                            if sleep_content_webview(wv) {
                                suspended_tabs.insert(id.clone());
                                state.tab_manager.sleep_tab(&id);
                                tabs_changed = true;
                                tracing::debug!(
                                    "tab_sleep: suspended tab {} (free_mb={})",
                                    id,
                                    free_mb
                                );
                            }
                        }
                    }

                    let mut to_discard: Vec<String> = if free_mb <= DISCARD_FREE_MB {
                        state
                            .tab_manager
                            .tabs
                            .iter()
                            .filter(|t| {
                                t.id != active
                                    && !t.is_neura_page()
                                    && !t.pinned
                                    && !t.is_essential
                                    && t.status != crate::browser::tab::TabStatus::Loading
                                    && !t.is_media_active
                                    && !tab_notifications_allowed(&t.url, &state.settings)
                                    && content_views.contains_key(&t.id)
                                    && (now_ms - t.last_active_at) >= threshold_ms
                            })
                            .map(|t| t.id.clone())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    let max_live = if free_mb <= DISCARD_FREE_MB {
                        max_live_webviews(free_mb)
                    } else {
                        MAX_PRESERVED_WEBVIEWS
                    };
                    if content_views.len() > max_live {
                        let need = content_views.len() - max_live;
                        let mut extra: Vec<(String, bool, bool, i64)> = state
                            .tab_manager
                            .tabs
                            .iter()
                            .filter(|t| {
                                t.id != active
                                    && !t.is_neura_page()
                                    && t.status != crate::browser::tab::TabStatus::Loading
                                    && !t.is_media_active
                                    && !tab_notifications_allowed(&t.url, &state.settings)
                                    && content_views.contains_key(&t.id)
                                    && !to_discard.iter().any(|id| id == &t.id)
                            })
                            .map(|t| {
                                let protected = t.pinned || t.is_essential;
                                let awake = !suspended_tabs.contains(&t.id);
                                (t.id.clone(), protected, awake, t.last_active_at)
                            })
                            .collect();
                        extra.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
                        to_discard.extend(extra.into_iter().take(need).map(|(id, _, _, _)| id));
                    }
                    to_discard.sort();
                    to_discard.dedup();
                    for id in to_discard {
                        content_views.remove(&id);
                        content_hwnds.remove(&id);
                        suspended_tabs.remove(&id);
                        clear_load_watches(&mut load_watches, &id);
                        state.tab_manager.sleep_tab(&id);
                        tabs_changed = true;
                        tracing::debug!(
                            "tab_discard: unloaded tab {} (free_mb={}, max_live={}, threshold={}min)",
                            id,
                            free_mb,
                            max_live,
                            threshold_ms / 60_000
                        );
                    }
                    if tabs_changed {
                        state.push_state_to_chrome(&chrome);
                    }
                }
                // Periodic proactive cookie snapshot.  Complements per-navigation saves
                // so cookies written by background JS (e.g. Google token refresh) are
                // also captured even if no new page navigation occurs.
                if Instant::now() >= cookie_save_at {
                    cookie_save_at = Instant::now() + COOKIE_SAVE_EVERY;
                    let active_id = state.tab_manager.active_tab_id.clone();
                    if let Some(id) = active_id {
                        if !state.tab_manager.tab_is_incognito(&id) {
                            if let Some(wv) = content_views.get(&id) {
                                browser::cookie_manager::trigger_save(wv, cookie_tx.clone());
                            }
                        }
                    }
                }
                if state.settings.privacy.auto_crash_report
                    && cloud::config::is_configured()
                    && Instant::now() >= error_report_at
                    && utils::log_buffer::take_error_pending()
                {
                    error_report_at = Instant::now() + ERROR_REPORT_COOLDOWN;
                    let report =
                        app::build_report(&state, "error", "Automatic error report".into(), String::new());
                    rt.spawn(async move {
                        match cloud::report::send(report).await {
                            Ok(()) => tracing::info!(target: "ventus::report", "automatic report uploaded"),
                            Err(e) => tracing::warn!(target: "ventus::report", error = %e, "automatic report upload failed"),
                        }
                    });
                }
            }

            // All events for a wrapped popup window are handled here so they never reach the
            // main-window arms below (a popup close must not exit the app).
            Event::WindowEvent {
                window_id, event, ..
            } if popups.values().any(|p| p.window.id() == window_id) => {
                match event {
                    WindowEvent::CloseRequested => {
                        if let Some(id) = popups
                            .iter()
                            .find(|(_, p)| p.window.id() == window_id)
                            .map(|(id, _)| *id)
                        {
                            popups.remove(&id);
                        }
                    }
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        if let Some(p) = popups.values().find(|p| p.window.id() == window_id) {
                            layout_popup(p);
                        }
                    }
                    _ => {}
                }
            }

            #[cfg(windows)]
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } if auth_window.as_ref().map(|(w, _)| w.id()) == Some(window_id) => {
                auth_window = None;
                let _ = chrome.evaluate_script(
                    "window.__neura && window.__neura.authIdle && window.__neura.authIdle()",
                );
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event,
                        is_synthetic: false,
                        ..
                    },
                ..
            } if key_pressed(&event, KeyCode::F11) => {
                let _ = proxy_main.send_event(AppEvent::Chrome(ChromeCommand::ToggleFullscreen));
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event,
                        is_synthetic: false,
                        ..
                    },
                ..
            } if key_pressed(&event, KeyCode::F12) => {
                let _ = proxy_main.send_event(AppEvent::Chrome(ChromeCommand::OpenDevtools));
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event,
                        is_synthetic: false,
                        ..
                    },
                ..
            } if state.content_fullscreen && key_pressed(&event, KeyCode::Escape) => {
                let _ = proxy_main.send_event(AppEvent::Chrome(
                    ChromeCommand::ContentFullscreenChange { active: false },
                ));
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                custom_maximized = window.is_maximized();
                sync_window_maximized(&chrome, custom_maximized);
                if state.content_fullscreen {
                    set_fullscreen_z(&window, true);
                } else {
                    keep_frameless(&window);
                }
                apply_layout(
                    &chrome,
                    chrome_hwnd,
                    &content_views,
                    &state,
                    &layout_config,
                    &window,
                );
            }

            Event::WindowEvent {
                event: WindowEvent::Focused(true),
                ..
            } => {
                if chrome_owns_content(&state) {
                    return;
                }
                if let Some(id) = state.tab_manager.active_tab_id.clone() {
                    if let Some(wv) = content_views.get(&id) {
                        wake_content_webview(wv);
                        let _ = wv.focus();
                    }
                    #[cfg(windows)]
                    if let Some(&hwnd) = content_hwnds.get(&id) {
                        repaint_content_webview(hwnd);
                    }
                }
            }

            Event::WindowEvent {
                event: WindowEvent::ScaleFactorChanged { .. },
                ..
            } => {
                if state.content_fullscreen {
                    set_fullscreen_z(&window, true);
                } else {
                    keep_frameless(&window);
                }
                apply_layout(
                    &chrome,
                    chrome_hwnd,
                    &content_views,
                    &state,
                    &layout_config,
                    &window,
                );
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if !new_window {
                    save_window_size(&window, &mut state, custom_maximized);
                    save_session(&state);
                }
                save_open_cookies(&content_views, &state, &data_dir);
                #[cfg(windows)]
                {
                    let _ = auth_window.take();
                }
                popups.clear();
                close_chrome_controller(&chrome);
                shutdown_webview2(
                    crash_sentinel.as_deref(),
                    &[webview_data_dir.as_path(), incognito_data_dir.as_path()],
                    &mut content_views,
                    &mut content_hwnds,
                    &mut content_web_context,
                    &mut incognito_web_context,
                );
                if new_window {
                    std::fs::remove_dir_all(&webview_data_dir).ok();
                }
                *control_flow = ControlFlow::Exit;
            }

            _ => {}
        }
    });
}

struct UrlServer {
    path: std::path::PathBuf,
    _thread: std::thread::JoinHandle<()>,
}

impl Drop for UrlServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
fn set_app_id() {
    let id: Vec<u16> = APP_ID.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = unsafe {
        windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(windows::core::PCWSTR(
            id.as_ptr(),
        ))
    };
}

#[cfg(not(windows))]
fn set_app_id() {}

#[cfg(windows)]
fn show_startup_error(msg: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let title: Vec<u16> = "Ventus".encode_utf16().chain(std::iter::once(0)).collect();
    let msg: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            HWND(0),
            PCWSTR(msg.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_startup_error(msg: &str) {
    eprintln!("{msg}");
}

fn normalize_launch_url(url: &str) -> String {
    launch_url(url).unwrap_or_else(|| url.trim().to_string())
}

fn launch_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
        || url.starts_with("neura://")
    {
        return Some(url.to_string());
    }
    let path = std::path::Path::new(url);
    if path.is_absolute() && path.exists() {
        if let Ok(file_url) = url::Url::from_file_path(path) {
            return Some(file_url.to_string());
        }
    }
    None
}

fn url_server_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("url-server.txt")
}

#[cfg(windows)]
fn start_url_server(
    data_dir: &std::path::Path,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
) -> Option<UrlServer> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let addr = listener.local_addr().ok()?;
    let path = url_server_path(data_dir);
    std::fs::write(&path, addr.to_string()).ok()?;
    let thread = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let mut s = String::new();
            let mut stream = std::io::Read::take(stream, 8192);
            if std::io::Read::read_to_string(&mut stream, &mut s).is_err() {
                continue;
            }
            let Some(url) = launch_url(&s) else {
                continue;
            };
            let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::OpenInNewTab { url }));
        }
    });
    Some(UrlServer {
        path,
        _thread: thread,
    })
}

#[cfg(not(windows))]
fn start_url_server(
    _data_dir: &std::path::Path,
    _proxy: tao::event_loop::EventLoopProxy<AppEvent>,
) -> Option<UrlServer> {
    None
}

#[cfg(windows)]
fn claim_instance(
    new_window: bool,
    url: Option<&str>,
    data_dir: &std::path::Path,
) -> windows::Win32::Foundation::HANDLE {
    use windows::Win32::Foundation::HANDLE;
    // Secondary windows skip the single-instance guard — they share the same profile.
    if new_window {
        return HANDLE(0);
    }
    use windows::core::w;
    use windows::Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    };

    let handle =
        unsafe { CreateMutexW(None, true, w!("Local\\VentusProfileLock")) }.unwrap_or(HANDLE(0));
    if !handle.is_invalid() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        tracing::warn!("Ventus is already running");
        if let Some(url) = url.and_then(launch_url) {
            if !send_launch_url(data_dir, &url) {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe)
                        .arg("--new-window")
                        .arg("--url")
                        .arg(url)
                        .spawn();
                }
            }
        }
        std::process::exit(0);
    }
    handle
}

#[cfg(not(windows))]
fn claim_instance(_new_window: bool, _url: Option<&str>, _data_dir: &std::path::Path) {}

#[cfg(windows)]
fn send_launch_url(data_dir: &std::path::Path, url: &str) -> bool {
    for _ in 0..20 {
        if let Ok(addr) = std::fs::read_to_string(url_server_path(data_dir)) {
            if let Ok(addr) = addr.trim().parse::<std::net::SocketAddr>() {
                if let Ok(mut stream) =
                    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200))
                {
                    if std::io::Write::write_all(&mut stream, url.as_bytes()).is_ok() {
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                        return true;
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Relaunch Ventus in a fresh process, then let the caller exit this one. Used when a
/// setting changes a WebView2 environment browser-arg (DoH, third-party-cookie blocking)
/// that can only take effect in a newly created environment — and, because the chrome UI
/// shares the single environment, that means a whole-process restart. The new instance
/// waits for this PID to fully exit (releasing the profile lock) via `--wait-for-pid`
/// before claiming the profile, and `--restore-session` reopens the saved tabs.
fn relaunch_self(restore_session: bool) {
    let Ok(exe) = std::env::current_exe() else {
        tracing::error!("relaunch_self: current_exe() failed; cannot restart");
        return;
    };
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--wait-for-pid")
        .arg(std::process::id().to_string());
    if restore_session {
        cmd.arg("--restore-session");
    }
    if let Err(e) = cmd.spawn() {
        tracing::error!("relaunch_self: failed to spawn new instance: {}", e);
    }
}

#[cfg(windows)]
fn wait_for_relaunch_parent(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };
    if pid == std::process::id() {
        return;
    }

    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) };
    match handle {
        Ok(h) if !h.is_invalid() => {
            unsafe {
                let _ = WaitForSingleObject(h, 8_000);
                let _ = CloseHandle(h);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        _ => std::thread::sleep(Duration::from_millis(1_000)),
    }
}

#[cfg(not(windows))]
fn wait_for_relaunch_parent(pid: Option<u32>) {
    if pid.is_some() {
        std::thread::sleep(Duration::from_millis(1_000));
    }
}

#[cfg(windows)]
fn attach_fullscreen_handler(wv: &WebView, proxy: tao::event_loop::EventLoopProxy<AppEvent>) {
    use webview2_com::{
        ContainsFullScreenElementChangedEventHandler,
        Microsoft::Web::WebView2::Win32::ICoreWebView2,
    };

    let controller = wv.controller();
    let webview: ICoreWebView2 = unsafe {
        match controller.CoreWebView2() {
            Ok(wv) => wv,
            Err(_) => return,
        }
    };

    let handler =
        ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _args| {
            let Some(sender) = sender else {
                return Ok(());
            };
            unsafe {
                let mut active = Default::default();
                let _ = sender.ContainsFullScreenElement(&mut active);
                let active: bool = active.as_bool();
                let _ =
                    proxy.send_event(AppEvent::Chrome(ChromeCommand::ContentFullscreenChange {
                        active,
                    }));
            }
            Ok(())
        }));

    let mut token = Default::default();
    unsafe {
        let _ = webview.add_ContainsFullScreenElementChanged(&handler, &mut token);
    }
}

/// Isolated cookie-save Tokio task.
///
/// Owns the long-lived write connection to `cookie_store.db`.  Receives
/// batches of `CookieRecord`s from the main event loop through `rx` and
/// writes them to SQLite.  Running in a dedicated async task means the save
/// logic is completely independent of the main browser event loop — even if
/// the event loop is blocked or panics, cookies already queued in the channel
/// will still be flushed when the Tokio runtime unwinds.
async fn run_cookie_save_task(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<cookie_store::CookieRecord>>,
    data_dir: std::path::PathBuf,
) {
    let conn = match cookie_store::open(&data_dir) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("cookie_store save task: failed to open DB: {}", e);
            return;
        }
    };
    tracing::debug!("cookie_store save task: running");
    while let Some(cookies) = rx.recv().await {
        if let Err(e) = cookie_store::save(&conn, &cookies) {
            tracing::warn!("cookie_store: save failed: {}", e);
        } else {
            tracing::debug!("cookie_store: saved {} cookies", cookies.len());
        }
        // Purge stale entries as part of each save cycle.
        if let Ok(n) = cookie_store::purge_expired(&conn) {
            if n > 0 {
                tracing::debug!("cookie_store: purged {} expired cookies", n);
            }
        }
    }
    tracing::debug!("cookie_store save task: channel closed, exiting");
}

/// Best-effort count of running msedgewebview2.exe processes. Used only for startup
/// diagnostics: a high count after a restart means previous WebView2 browser processes
/// did not exit and are holding the profile lock (the root cause of stuck black tabs).
#[cfg(windows)]
fn count_msedgewebview2_processes() -> usize {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = std::process::Command::new("tasklist")
        .args([
            "/FI",
            "IMAGENAME eq msedgewebview2.exe",
            "/NH",
            "/FO",
            "CSV",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .filter(|l| l.to_lowercase().contains("msedgewebview2.exe"))
                .count()
        }
        Err(_) => 0,
    }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn count_msedgewebview2_processes() -> usize {
    0
}

fn wait_for_previous_instance(sentinel: &std::path::Path, profiles: &[&std::path::Path]) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
        use windows::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        };

        let old_pid: Option<u32> = std::fs::read_to_string(sentinel)
            .ok()
            .and_then(|s| s.trim().parse().ok());

        if profiles
            .iter()
            .all(|profile_root| webview_profile_lock_released(profile_root))
        {
            return;
        }

        if let Some(pid) = old_pid {
            let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) };
            match handle {
                Ok(h) if !h.is_invalid() => {
                    tracing::debug!("waiting for previous Ventus PID {} to exit", pid);
                    let result = unsafe { WaitForSingleObject(h, 8_000) };
                    unsafe {
                        let _ = CloseHandle(h);
                    }
                    if result != WAIT_OBJECT_0 {
                        tracing::warn!(
                            "previous instance (PID {}) did not exit within 8 s; waiting for profile release anyway",
                            pid
                        );
                    }
                    let _ = wait_for_webview_profiles_released(
                        profiles,
                        WEBVIEW_PROFILE_RELEASE_TIMEOUT,
                    );
                    return;
                }
                _ => {
                    tracing::debug!("previous Ventus instance already gone");
                    let _ = wait_for_webview_profiles_released(
                        profiles,
                        WEBVIEW_PROFILE_RELEASE_TIMEOUT,
                    );
                    return;
                }
            }
        }
        let _ = wait_for_webview_profiles_released(profiles, WEBVIEW_PROFILE_RELEASE_TIMEOUT);
    }
    #[cfg(not(windows))]
    {
        let _ = sentinel;
        let _ = profiles;
        std::thread::sleep(Duration::from_millis(800));
    }
}

/// Pump the Win32 message queue for up to `ms` milliseconds.
///
/// WebView2 COM callbacks (e.g. the `GetCookies` completion handler fired by
/// `trigger_save`) are posted as window messages to the UI thread.  If we drop
/// the WebView objects before those messages are dispatched the callbacks never
/// run and the final cookie snapshot is lost.  Draining the queue here gives
/// any in-flight callbacks a chance to execute while COM objects are still alive.
#[cfg(windows)]
fn drain_message_queue_ms(ms: u64) {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };
    let deadline = std::time::Instant::now() + Duration::from_millis(ms);
    while std::time::Instant::now() < deadline {
        let mut msg = MSG::default();
        // SAFETY: standard Win32 call; msg is fully initialised by PeekMessageW.
        let got = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) };
        if got.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        } else {
            // Queue is empty — short sleep so we don't busy-spin.
            std::thread::sleep(Duration::from_millis(8));
        }
    }
}

#[cfg(windows)]
fn process_running(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) };
    let Ok(handle) = handle else {
        return false;
    };
    if handle.is_invalid() {
        return false;
    }
    let result = unsafe { WaitForSingleObject(handle, 0) };
    unsafe {
        let _ = CloseHandle(handle);
    }
    result != WAIT_OBJECT_0
}

#[cfg(windows)]
fn cleanup_secondary_webview_profiles(data_dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(pid) = name.strip_prefix("webview_data_window_") else {
            continue;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            continue;
        };
        if process_running(pid) {
            continue;
        }
        let _ = std::fs::remove_dir_all(entry.path());
    }
}

#[cfg(windows)]
fn webview_profile_lock_paths(profile_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    vec![
        profile_root.join("EBWebView").join("lockfile"),
        profile_root.join("EBWebView").join("LOCK"),
        profile_root.join("EBWebView").join("Default").join("LOCK"),
        profile_root
            .join("EBWebView")
            .join("Default")
            .join("Local Storage")
            .join("leveldb")
            .join("LOCK"),
        profile_root
            .join("EBWebView")
            .join("Default")
            .join("Session Storage")
            .join("LOCK"),
    ]
}

#[cfg(windows)]
fn webview_profile_lock_released(profile_root: &std::path::Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;

    webview_profile_lock_paths(profile_root)
        .into_iter()
        .filter(|lock_path| lock_path.exists())
        .all(|lock_path| {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(lock_path)
                .is_ok()
        })
}

#[cfg(windows)]
fn wait_for_webview_profiles_released(
    profile_roots: &[&std::path::Path],
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if profile_roots
            .iter()
            .all(|profile_root| webview_profile_lock_released(profile_root))
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        drain_message_queue_ms(WEBVIEW_PROFILE_RELEASE_POLL);
    }
}

#[cfg(windows)]
fn close_chrome_controller(chrome: &WebView) {
    unsafe {
        let _ = chrome.controller().Close();
    }
}
#[cfg(not(windows))]
fn close_chrome_controller(_chrome: &WebView) {}

fn shutdown_webview2(
    crash_sentinel: Option<&std::path::Path>,
    profile_roots: &[&std::path::Path],
    content_views: &mut HashMap<String, WebView>,
    content_hwnds: &mut HashMap<String, isize>,
    content_web_context: &mut Option<wry::WebContext>,
    incognito_web_context: &mut Option<wry::WebContext>,
) {
    #[cfg(windows)]
    drain_message_queue_ms(300);
    content_views.clear();
    content_hwnds.clear();
    drop(content_web_context.take());
    drop(incognito_web_context.take());
    #[cfg(windows)]
    let free = if crash_sentinel.is_some() {
        drain_message_queue_ms(300);
        wait_for_webview_profiles_released(profile_roots, WEBVIEW_PROFILE_RELEASE_TIMEOUT)
    } else {
        true
    };
    #[cfg(not(windows))]
    let free = {
        let _ = profile_roots;
        std::thread::sleep(Duration::from_millis(800));
        true
    };
    if let Some(sentinel) = crash_sentinel {
        if free {
            let _ = std::fs::remove_file(sentinel);
        } else {
            tracing::warn!(
                target: "ventus::shutdown",
                lock = %sentinel.display(),
                "WebView2 profile still busy after shutdown wait; keeping running lock for next launch"
            );
        }
    }
}

#[cfg(windows)]
fn attach_process_failed_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    tab_id: String,
) {
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2, COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
            COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE,
        },
        ProcessFailedEventHandler,
    };

    let controller = wv.controller();
    let webview: ICoreWebView2 = unsafe {
        match controller.CoreWebView2() {
            Ok(wv) => wv,
            Err(_) => return,
        }
    };

    let handler = ProcessFailedEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED;
        unsafe {
            let _ = args.ProcessFailedKind(&mut kind);
        }
        // Only auto-reload on renderer crash/unresponsive. Browser-process failure
        // is catastrophic (requires environment recreation) and rare; log it instead.
        if kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
            || kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE
        {
            let _ = proxy.send_event(AppEvent::ContentProcessFailed {
                tab_id: tab_id.clone(),
                fatal: kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
            });
        } else {
            tracing::error!(
                "WebView2 browser process failed (kind={}) — restart required",
                kind.0
            );
        }
        Ok(())
    }));

    let mut token = Default::default();
    unsafe {
        let _ = webview.add_ProcessFailed(&handler, &mut token);
    }
}

#[cfg(windows)]
type Wv2PermKind = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PERMISSION_KIND;

#[cfg(windows)]
type Wv2PermState = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PERMISSION_STATE;

#[cfg(windows)]
const SITE_PERMISSION_KEYS: [&str; 12] = [
    "microphone",
    "camera",
    "geolocation",
    "notifications",
    "sensors",
    "clipboard",
    "downloads",
    "file_system",
    "autoplay",
    "local_fonts",
    "midi",
    "window_management",
];

#[cfg(windows)]
fn site_permission_kind(key: &str) -> Option<Wv2PermKind> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY, COREWEBVIEW2_PERMISSION_KIND_CAMERA,
        COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ, COREWEBVIEW2_PERMISSION_KIND_FILE_READ_WRITE,
        COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION, COREWEBVIEW2_PERMISSION_KIND_LOCAL_FONTS,
        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
        COREWEBVIEW2_PERMISSION_KIND_MIDI_SYSTEM_EXCLUSIVE_MESSAGES,
        COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS,
        COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS, COREWEBVIEW2_PERMISSION_KIND_OTHER_SENSORS,
        COREWEBVIEW2_PERMISSION_KIND_WINDOW_MANAGEMENT,
    };
    Some(match key {
        "microphone" => COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
        "camera" => COREWEBVIEW2_PERMISSION_KIND_CAMERA,
        "geolocation" => COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION,
        "notifications" => COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS,
        "sensors" => COREWEBVIEW2_PERMISSION_KIND_OTHER_SENSORS,
        "clipboard" => COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ,
        "downloads" => COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS,
        "file_system" => COREWEBVIEW2_PERMISSION_KIND_FILE_READ_WRITE,
        "autoplay" => COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY,
        "local_fonts" => COREWEBVIEW2_PERMISSION_KIND_LOCAL_FONTS,
        "midi" => COREWEBVIEW2_PERMISSION_KIND_MIDI_SYSTEM_EXCLUSIVE_MESSAGES,
        "window_management" => COREWEBVIEW2_PERMISSION_KIND_WINDOW_MANAGEMENT,
        _ => return None,
    })
}

#[cfg(windows)]
fn site_permission_key(kind: Wv2PermKind) -> Option<&'static str> {
    SITE_PERMISSION_KEYS
        .iter()
        .copied()
        .find(|key| site_permission_kind(key) == Some(kind))
}

#[cfg(windows)]
fn site_permission_state(value: &str) -> Wv2PermState {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PERMISSION_STATE_ALLOW, COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
        COREWEBVIEW2_PERMISSION_STATE_DENY,
    };
    match value {
        "allow" => COREWEBVIEW2_PERMISSION_STATE_ALLOW,
        "block" => COREWEBVIEW2_PERMISSION_STATE_DENY,
        _ => COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
    }
}

#[cfg(windows)]
fn permission_asks_by_default(key: &str) -> bool {
    matches!(key, "microphone" | "camera" | "notifications")
}

#[cfg(windows)]
fn permission_action<'a>(
    strict: bool,
    site_permissions: &'a config::SitePermissionMap,
    default_permissions: &'a config::SitePermissions,
    origin: &str,
    key: &str,
) -> &'a str {
    site_permissions
        .get(origin)
        .and_then(|p| p.get_explicit(key))
        .filter(|s| *s == "allow" || *s == "block")
        .or_else(|| {
            default_permissions
                .get_explicit(key)
                .filter(|s| *s == "allow" || *s == "block")
        })
        .unwrap_or(if strict && !permission_asks_by_default(key) {
            "block"
        } else {
            "ask"
        })
}

#[cfg(windows)]
fn normalize_webview_origin(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    if url.scheme() != "https" && url.scheme() != "http" {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    Some(format!("{}://{}{}", url.scheme(), host, port))
}

#[cfg(windows)]
fn site_permission_profile(
    webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
) -> Option<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Profile4> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_13;
    use wv2core::Interface;
    let webview13: ICoreWebView2_13 = webview.cast().ok()?;
    let profile = unsafe { webview13.Profile().ok()? };
    profile.cast().ok()
}

#[cfg(windows)]
fn pcwstr(s: &str) -> (wv2core::PCWSTR, Vec<u16>) {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    (wv2core::PCWSTR(v.as_ptr()), v)
}

#[cfg(windows)]
fn apply_profile_site_permissions(
    webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
    site_permissions: &config::SitePermissionMap,
) {
    use webview2_com::SetPermissionStateCompletedHandler;
    let Some(profile) = site_permission_profile(webview) else {
        return;
    };
    for (origin, perms) in site_permissions {
        for key in SITE_PERMISSION_KEYS {
            let Some(kind) = site_permission_kind(key) else {
                continue;
            };
            let Some(value) = perms.get_explicit(key) else {
                continue;
            };
            let state = site_permission_state(value);
            let (origin_ptr, _origin_buf) = pcwstr(origin);
            let origin_log = origin.clone();
            let key_log = key.to_string();
            let handler = SetPermissionStateCompletedHandler::create(Box::new(move |err| {
                if let Err(e) = err {
                    tracing::warn!(
                        "permission state failed for {} {}: {}",
                        origin_log,
                        key_log,
                        e
                    );
                }
                Ok(())
            }));
            unsafe {
                let _ = profile.SetPermissionState(kind, origin_ptr, state, &handler);
            }
        }
    }
}

#[cfg(windows)]
fn attach_permission_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    strict: bool,
    site_permissions: config::SitePermissionMap,
    default_permissions: config::SitePermissions,
) {
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2, ICoreWebView2PermissionRequestedEventArgs3,
            COREWEBVIEW2_PERMISSION_STATE_ALLOW, COREWEBVIEW2_PERMISSION_STATE_DENY,
        },
        PermissionRequestedEventHandler,
    };
    use wv2core::{Interface, PWSTR};

    let controller = wv.controller();
    let webview: ICoreWebView2 = unsafe {
        match controller.CoreWebView2() {
            Ok(wv) => wv,
            Err(_) => return,
        }
    };
    apply_profile_site_permissions(&webview, &site_permissions);

    let handler = PermissionRequestedEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            if let Ok(args3) = args.cast::<ICoreWebView2PermissionRequestedEventArgs3>() {
                let _ = args3.SetSavesInProfile(false);
            }
            let mut kind = Default::default();
            args.PermissionKind(&mut kind)?;
            let Some(key) = site_permission_key(kind) else {
                return Ok(());
            };
            let mut ptr = PWSTR::null();
            args.Uri(&mut ptr)?;
            let origin = normalize_webview_origin(&take_pwstr(ptr)).unwrap_or_default();
            if !origin.is_empty() {
                let _ = proxy.send_event(AppEvent::PermissionRequested {
                    origin: origin.clone(),
                    key: key.to_string(),
                });
            }
            let action = permission_action(
                strict,
                &site_permissions,
                &default_permissions,
                &origin,
                key,
            );
            if action == "allow" {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
            } else if action == "block" {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
            } else if !origin.is_empty() {
                if let Ok(deferral) = args.GetDeferral() {
                    let id = uuid::Uuid::new_v4().to_string();
                    let dup = PERMISSION_DEFERRALS.with(|m| {
                        m.borrow()
                            .values()
                            .any(|(_, _, o, k)| o == &origin && k == key)
                    });
                    PERMISSION_DEFERRALS.with(|m| {
                        m.borrow_mut().insert(
                            id.clone(),
                            (deferral, args.clone(), origin.clone(), key.to_string()),
                        )
                    });
                    if !dup {
                        let _ = proxy.send_event(AppEvent::PermissionPrompt {
                            id,
                            origin,
                            key: key.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }));

    let mut token = Default::default();
    unsafe {
        let _ = webview.add_PermissionRequested(&handler, &mut token);
    }
}

#[cfg(windows)]
thread_local! {
    static DOWNLOAD_OPS: std::cell::RefCell<
        HashMap<String, webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadOperation>,
    > = std::cell::RefCell::new(HashMap::new());

    static DOWNLOAD_DEFERRALS: std::cell::RefCell<
        HashMap<
            String,
            (
                webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Deferral,
                webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadStartingEventArgs,
                String,
            ),
        >,
    > = std::cell::RefCell::new(HashMap::new());

    static ACCEL_DOWNLOADS: std::cell::RefCell<
        HashMap<String, crate::browser::accel_download::AccelControl>,
    > = std::cell::RefCell::new(HashMap::new());

    static PERMISSION_DEFERRALS: std::cell::RefCell<
        HashMap<
            String,
            (
                webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Deferral,
                webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PermissionRequestedEventArgs,
                String,
                String,
            ),
        >,
    > = std::cell::RefCell::new(HashMap::new());
}

#[cfg(windows)]
fn resolve_permission(origin: &str, key: &str, allow: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PERMISSION_STATE_ALLOW, COREWEBVIEW2_PERMISSION_STATE_DENY,
    };
    let entries: Vec<_> = PERMISSION_DEFERRALS.with(|m| {
        let mut b = m.borrow_mut();
        let ids: Vec<String> = b
            .iter()
            .filter(|(_, (_, _, o, k))| o == origin && k == key)
            .map(|(id, _)| id.clone())
            .collect();
        ids.iter().filter_map(|id| b.remove(id)).collect()
    });
    let state = if allow {
        COREWEBVIEW2_PERMISSION_STATE_ALLOW
    } else {
        COREWEBVIEW2_PERMISSION_STATE_DENY
    };
    for (deferral, args, _, _) in entries {
        unsafe {
            let _ = args.SetState(state);
            let _ = deferral.Complete();
        }
    }
}

#[cfg(windows)]
fn control_download(id: &str, action: DownloadCtl) {
    use std::sync::atomic::Ordering;
    if let Some(ctl) = ACCEL_DOWNLOADS.with(|m| m.borrow().get(id).cloned()) {
        match action {
            DownloadCtl::Pause => ctl.paused.store(true, Ordering::Relaxed),
            DownloadCtl::Resume => ctl.paused.store(false, Ordering::Relaxed),
            DownloadCtl::Cancel => {
                ctl.cancel.store(true, Ordering::Relaxed);
                ACCEL_DOWNLOADS.with(|m| m.borrow_mut().remove(id));
            }
        }
        return;
    }
    let op = DOWNLOAD_OPS.with(|m| m.borrow().get(id).cloned());
    let Some(op) = op else { return };
    unsafe {
        let _ = match action {
            DownloadCtl::Pause => op.Pause(),
            DownloadCtl::Resume => op.Resume(),
            DownloadCtl::Cancel => op.Cancel(),
        };
    }
}

#[cfg(windows)]
fn cancel_all_downloads() {
    use std::sync::atomic::Ordering;
    ACCEL_DOWNLOADS.with(|m| {
        for ctl in m.borrow().values() {
            ctl.cancel.store(true, Ordering::Relaxed);
        }
        m.borrow_mut().clear();
    });
    let ops: Vec<_> = DOWNLOAD_OPS.with(|m| m.borrow().values().cloned().collect());
    for op in ops {
        unsafe {
            let _ = op.Cancel();
        }
    }
    DOWNLOAD_OPS.with(|m| m.borrow_mut().clear());
}

#[cfg(windows)]
fn attach_download_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    dl_dir: std::sync::Arc<std::sync::Mutex<DownloadPrefs>>,
) {
    use webview2_com::DownloadStartingEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2, ICoreWebView2_4};
    use wv2core::{Interface, PWSTR};
    use wv2win::Win32::Foundation::BOOL;

    let controller = wv.controller();
    let webview: ICoreWebView2 = unsafe {
        match controller.CoreWebView2() {
            Ok(w) => w,
            Err(_) => return,
        }
    };
    let webview4: ICoreWebView2_4 = match webview.cast() {
        Ok(w) => w,
        Err(_) => return,
    };

    let handler = DownloadStartingEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            let op = args.DownloadOperation()?;

            let mut uri = PWSTR::null();
            op.Uri(&mut uri)?;
            let url = take_pwstr(uri);

            let mut rp = PWSTR::null();
            args.ResultFilePath(&mut rp)?;
            let default_name = std::path::Path::new(&take_pwstr(rp))
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
                .to_string();

            if is_accel_candidate(&url) {
                let deferral = args.GetDeferral()?;
                let id = uuid::Uuid::new_v4().to_string();
                DOWNLOAD_DEFERRALS.with(|m| {
                    m.borrow_mut()
                        .insert(id.clone(), (deferral, args.clone(), default_name));
                });
                let _ = proxy.send_event(AppEvent::AccelProbe { id, url });
                return Ok(());
            }

            let Some(target) = resolve_download_target(&default_name, &dl_dir) else {
                args.SetCancel(BOOL::from(true))?;
                return Ok(());
            };
            begin_native_download(&args, &target, url, &proxy);
        }
        Ok(())
    }));
    let mut token = Default::default();
    unsafe {
        let _ = webview4.add_DownloadStarting(&handler, &mut token);
    }
}

fn is_accel_candidate(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn resolve_download_target(
    default_name: &str,
    dl_dir: &std::sync::Arc<std::sync::Mutex<DownloadPrefs>>,
) -> Option<std::path::PathBuf> {
    let prefs = dl_dir.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let target = if prefs.ask {
        let mut dlg = rfd::FileDialog::new()
            .set_title("Save file")
            .set_file_name(default_name);
        if prefs.dir.exists() {
            dlg = dlg.set_directory(&prefs.dir);
        }
        dlg.save_file()?
    } else {
        unique_download_path(&prefs.dir, default_name)
    };
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Some(target)
}

#[cfg(windows)]
fn begin_native_download(
    args: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadStartingEventArgs,
    target: &std::path::Path,
    url: String,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_TRANSIENT_ERROR,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_DISCONNECTED,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_FAILED,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_SERVER_DOWN,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_TIMEOUT,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_FAILED,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED,
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED, COREWEBVIEW2_DOWNLOAD_STATE,
        COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED, COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS,
    };
    use webview2_com::{BytesReceivedChangedEventHandler, StateChangedEventHandler};
    use wv2win::Win32::Foundation::BOOL;

    unsafe {
        let Ok(op) = args.DownloadOperation() else {
            return;
        };
        let (path_pcwstr, _buf) = pcwstr(&target.to_string_lossy());
        if args.SetResultFilePath(path_pcwstr).is_err() {
            return;
        }
        if args.SetHandled(BOOL::from(true)).is_err() {
            return;
        }

        let id = uuid::Uuid::new_v4().to_string();
        let filename = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();
        let path_str = target.to_string_lossy().to_string();
        let total = read_download_total(&op);

        let proxy_p = proxy.clone();
        let id_p = id.clone();
        let last_emit =
            std::cell::Cell::new(std::time::Instant::now() - std::time::Duration::from_secs(1));
        let bytes_handler = BytesReceivedChangedEventHandler::create(Box::new(move |op, _| {
            let Some(op) = op else {
                return Ok(());
            };
            let now = std::time::Instant::now();
            if now.duration_since(last_emit.get()) < std::time::Duration::from_millis(150) {
                return Ok(());
            }
            last_emit.set(now);
            let mut recv = 0i64;
            op.BytesReceived(&mut recv)?;
            let _ = proxy_p.send_event(AppEvent::DownloadProgress {
                id: id_p.clone(),
                received: recv.max(0) as u64,
                total: read_download_total(&op),
            });
            Ok(())
        }));
        let mut bt = Default::default();
        let _ = op.add_BytesReceivedChanged(&bytes_handler, &mut bt);

        let proxy_s = proxy.clone();
        let id_s = id.clone();
        let resume_tries = std::cell::Cell::new(0u32);
        let resume_anchor = std::cell::Cell::new(0u64);
        let state_handler = StateChangedEventHandler::create(Box::new(move |op, _| {
            let Some(op) = op else {
                return Ok(());
            };
            let mut st = COREWEBVIEW2_DOWNLOAD_STATE::default();
            op.State(&mut st)?;
            if st == COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS {
                let mut recv = 0i64;
                op.BytesReceived(&mut recv)?;
                let recv = recv.max(0) as u64;
                if recv > resume_anchor.get().saturating_add(1_048_576) {
                    resume_anchor.set(recv);
                    resume_tries.set(0);
                }
                let _ = proxy_s.send_event(AppEvent::DownloadProgress {
                    id: id_s.clone(),
                    received: recv,
                    total: read_download_total(&op),
                });
                return Ok(());
            }
            if st == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED {
                DOWNLOAD_OPS.with(|m| m.borrow_mut().remove(&id_s));
                tracing::info!(target: "ventus::dl", id = %id_s, "download completed");
                let _ = proxy_s.send_event(AppEvent::DownloadDone {
                    id: id_s.clone(),
                    success: true,
                    canceled: false,
                });
                return Ok(());
            }
            let mut reason = COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON::default();
            let _ = op.InterruptReason(&mut reason);
            if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED {
                let _ = proxy_s.send_event(AppEvent::DownloadPaused { id: id_s.clone() });
                return Ok(());
            }
            let canceled = reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED;
            let recoverable = reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_FAILED
                || reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_TIMEOUT
                || reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_DISCONNECTED
                || reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_SERVER_DOWN
                || reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_FAILED
                || reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_TRANSIENT_ERROR;
            let mut can = BOOL::default();
            let _ = op.CanResume(&mut can);
            if !canceled
                && recoverable
                && can.as_bool()
                && resume_tries.get() < MAX_DOWNLOAD_RESUMES
            {
                resume_tries.set(resume_tries.get() + 1);
                tracing::warn!(target: "ventus::dl", id = %id_s, reason = reason.0, try_n = resume_tries.get(), "download interrupted; resuming");
                let proxy_r = proxy_s.clone();
                let id_r = id_s.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(DOWNLOAD_RESUME_DELAY);
                    let _ = proxy_r.send_event(AppEvent::DownloadResume { id: id_r });
                });
                return Ok(());
            }
            DOWNLOAD_OPS.with(|m| m.borrow_mut().remove(&id_s));
            if !canceled {
                tracing::warn!(target: "ventus::dl", id = %id_s, reason = reason.0, "download failed");
            }
            let _ = proxy_s.send_event(AppEvent::DownloadDone {
                id: id_s.clone(),
                success: false,
                canceled,
            });
            Ok(())
        }));
        let mut st_tok = Default::default();
        let _ = op.add_StateChanged(&state_handler, &mut st_tok);

        DOWNLOAD_OPS.with(|m| m.borrow_mut().insert(id.clone(), op.clone()));
        tracing::info!(target: "ventus::dl", id = %id, filename = %filename, total = total.unwrap_or(0), "download started");
        let _ = proxy.send_event(AppEvent::DownloadStarted {
            id: Some(id),
            url,
            filename,
            path: path_str,
            total,
        });
    }
}

#[cfg(windows)]
fn read_download_total(
    op: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadOperation,
) -> Option<u64> {
    let mut total = 0i64;
    unsafe {
        let _ = op.TotalBytesToReceive(&mut total);
    }
    if total > 0 {
        Some(total as u64)
    } else {
        None
    }
}

fn attach_navigation_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    tab_id: String,
) {
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::ICoreWebView2, NavigationCompletedEventHandler,
        NavigationStartingEventHandler,
    };
    use wv2core::PWSTR;

    let controller = wv.controller();
    let webview: ICoreWebView2 = unsafe {
        match controller.CoreWebView2() {
            Ok(wv) => wv,
            Err(_) => return,
        }
    };

    let navs = Arc::new(Mutex::new(HashMap::<u64, String>::new()));
    let navs_start = Arc::clone(&navs);
    let proxy_start = proxy.clone();
    let tab_start = tab_id.clone();
    let start_handler = NavigationStartingEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            let mut id = 0u64;
            let _ = args.NavigationId(&mut id);
            let mut ptr = PWSTR::null();
            args.Uri(&mut ptr)?;
            let url = take_pwstr(ptr);
            if id == 0 || !is_recoverable_nav_url(&url) {
                return Ok(());
            }
            if let Ok(mut map) = navs_start.lock() {
                map.insert(id, url.clone());
            }
            tracing::info!(target: "ventus::nav", tab = %tab_start, nav_id = id, url = %url, "NavigationStarting");
            let _ = proxy_start.send_event(AppEvent::ContentLoadStart {
                tab_id: tab_start.clone(),
                url,
                native: true,
                nav_id: id,
            });
        }
        Ok(())
    }));
    let mut start_token = Default::default();
    unsafe {
        let _ = webview.add_NavigationStarting(&start_handler, &mut start_token);
    }

    let navs_done = Arc::clone(&navs);
    let handler = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
        let Some(sender) = sender else {
            return Ok(());
        };
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            let mut ok = Default::default();
            args.IsSuccess(&mut ok)?;
            let mut id = 0u64;
            let _ = args.NavigationId(&mut id);
            let start_url = navs_done
                .lock()
                .ok()
                .and_then(|mut map| map.remove(&id))
                .filter(|u| !u.trim().is_empty())
                .unwrap_or_default();
            let source = webview_source(&sender);
            let url = if is_recoverable_nav_url(&source) {
                source
            } else {
                start_url.clone()
            };
            if ok.as_bool() {
                if is_recoverable_nav_url(&url) {
                    tracing::info!(target: "ventus::nav", tab = %tab_id, nav_id = id, url = %url, "NavigationCompleted OK");
                    let _ = proxy.send_event(AppEvent::ContentLoadEnd {
                        tab_id: tab_id.clone(),
                        start_url,
                        url,
                        nav_id: id,
                    });
                } else {
                    tracing::warn!(target: "ventus::nav", tab = %tab_id, nav_id = id, source = %url, "NavigationCompleted OK but non-recoverable URL (dropped)");
                }
                return Ok(());
            }
            if !is_recoverable_nav_url(&url) {
                return Ok(());
            }
            let mut status = Default::default();
            let _ = args.WebErrorStatus(&mut status);
            if canceled_web_error_status(status.0) {
                tracing::debug!(target: "ventus::nav", tab = %tab_id, nav_id = id, status = status.0, url = %url, "NavigationCompleted canceled");
            } else {
                tracing::warn!(target: "ventus::nav", tab = %tab_id, nav_id = id, status = status.0, url = %url, "NavigationCompleted FAILED");
            }
            let _ = proxy.send_event(AppEvent::ContentNavigationFailed {
                tab_id: tab_id.clone(),
                url: url.clone(),
                status: status.0,
                nav_id: id,
            });
            if let Some(http_url) = https_to_http(&url) {
                let _ = proxy.send_event(AppEvent::HttpsUpgradeFailed {
                    tab_id: tab_id.clone(),
                    https_url: url,
                    http_url,
                });
            }
        }
        Ok(())
    }));

    let mut token = Default::default();
    unsafe {
        let _ = webview.add_NavigationCompleted(&handler, &mut token);
    }
}

// A new-window request (target=_blank link, window.open, ctrl/middle click) should become a
// normal tab — NOT a bare popup OS window like WRY's default handler does. The exception is a
// real popup: OAuth sign-in, share sheets, and payment dialogs call window.open with an
// explicit width/height, which WebView2 reports via WindowFeatures.HasSize. Those we wrap in a
// Ventus-styled window (preserving window.opener via SetNewWindow); everything else is a tab.
#[cfg(windows)]
fn attach_new_window_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    incognito: bool,
) {
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::ICoreWebView2, NewWindowRequestedEventHandler,
    };
    use wv2core::PWSTR;

    let controller = wv.controller();
    let webview: ICoreWebView2 = unsafe {
        match controller.CoreWebView2() {
            Ok(wv) => wv,
            Err(_) => return,
        }
    };

    let handler = NewWindowRequestedEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            let mut ptr = PWSTR::null();
            args.Uri(&mut ptr)?;
            let url = take_pwstr(ptr);

            let mut sized: wv2win::Win32::Foundation::BOOL = Default::default();
            let mut win_w = 0u32;
            let mut win_h = 0u32;
            if let Ok(features) = args.WindowFeatures() {
                let _ = features.HasSize(&mut sized);
                let _ = features.Width(&mut win_w);
                let _ = features.Height(&mut win_h);
            }

            // Sized popup → wrap it. We take a deferral so the request stays pending while the
            // main loop (which can create windows) builds the wrapper and hands our WebView
            // back via SetNewWindow. If anything fails, the deferral completes without a new
            // window and WebView2 falls back to its own popup — the opener never breaks.
            if sized.as_bool() || is_auth_popup_url(&url) {
                if let Ok(deferral) = args.GetDeferral() {
                    PENDING_POPUPS.with(|q| {
                        q.borrow_mut().push(PendingPopup {
                            args: args.clone(),
                            deferral,
                            width: win_w,
                            height: win_h,
                            incognito,
                        });
                    });
                    let _ = proxy.send_event(AppEvent::CreatePopupWindow);
                }
                return Ok(());
            }

            // Blank/about:blank with no url → "open blank then set location" pattern: leave it
            // as a native popup so the opener keeps a real window reference.
            let real_url = url.starts_with("http://") || url.starts_with("https://");
            if !real_url {
                return Ok(());
            }

            args.SetHandled(true)?;
            let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::OpenInNewTab { url }));
        }
        Ok(())
    }));

    let mut token = Default::default();
    unsafe {
        let _ = webview.add_NewWindowRequested(&handler, &mut token);
    }
}

fn is_auth_popup_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    let path = parsed.path().to_ascii_lowercase();
    if host == "accounts.google.com" {
        return true;
    }
    if host == "appleid.apple.com" {
        return true;
    }
    if host == "login.live.com" || host == "login.microsoftonline.com" {
        return true;
    }
    if host.ends_with(".facebook.com") && path.contains("/dialog/oauth") {
        return true;
    }
    path.starts_with("/oauth")
        || path.contains("/oauth/")
        || path.contains("oauth20_authorize")
        || path.starts_with("/authorize")
        || path.contains("/authorize/")
        || path.starts_with("/signin")
        || path.contains("/signin/")
        || path.starts_with("/sign-in")
        || path.contains("/sign-in/")
        || path.starts_with("/sign_in")
        || path.contains("/sign_in/")
        || path.contains("/single_sign_on")
}

// A wrapped popup window: a frameless Ventus-styled window with a slim top bar (origin +
// close) above the popup content. Fields drop in declaration order, so the WebViews (children
// of the window) are torn down before the window itself.
struct PopupWindow {
    content: WebView,
    bar: WebView,
    window: tao::window::Window,
    bar_h: u32,
}

#[cfg(windows)]
struct PendingPopup {
    args: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2NewWindowRequestedEventArgs,
    deferral: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Deferral,
    width: u32,
    height: u32,
    incognito: bool,
}

// Queue of popups awaiting a wrapper window. The NewWindowRequested handler (a COM callback on
// the UI thread) pushes here and wakes the event loop, which drains it on the same thread — so
// the !Send COM objects never cross threads.
#[cfg(windows)]
thread_local! {
    static PENDING_POPUPS: std::cell::RefCell<Vec<PendingPopup>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(windows)]
fn spawn_popup_window(
    elwt: &tao::event_loop::EventLoopWindowTarget<AppEvent>,
    main_window: &tao::window::Window,
    pending: &PendingPopup,
    id: u64,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    web_context: &mut wry::WebContext,
    browser_args: &str,
) -> Option<PopupWindow> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;

    let bar_h = 40u32;
    let w = if pending.width >= 200 {
        pending.width.min(1600)
    } else {
        480
    };
    let body_h = if pending.height >= 200 {
        pending.height.min(1200)
    } else {
        600
    };
    let total_h = body_h + bar_h;

    let window = WindowBuilder::new()
        .with_title("Ventus")
        .with_inner_size(LogicalSize::new(w, total_h))
        .with_decorations(false)
        .with_resizable(false)
        .with_visible(false)
        .build(elwt)
        .ok()?;
    center_popup(&window, main_window, w, total_h);
    set_square_corners(&window);
    set_window_background_dark(&window);

    let size = window.inner_size();
    let scale = window.scale_factor();
    let bar_px = logical_to_physical(bar_h as f64, scale)
        .min(size.height.saturating_sub(1).max(1))
        .max(1);
    let bar_rect = Rect {
        x: 0,
        y: 0,
        width: size.width.max(1),
        height: bar_px,
    };
    let content_rect = Rect {
        x: 0,
        y: bar_px as i32,
        width: size.width.max(1),
        height: size.height.saturating_sub(bar_px).max(1),
    };

    let content = build_popup_content_webview(
        &window,
        content_rect,
        web_context,
        pending.incognito,
        browser_args,
    )?;
    let bar = build_popup_bar_webview(&window, bar_rect, web_context, proxy.clone(), id)?;

    // SetNewWindow is the last fallible step: if it succeeds we keep everything, and if it
    // fails both WebViews drop and the caller completes the deferral → default popup.
    let ok = unsafe {
        match content.controller().CoreWebView2() {
            Ok(core) => {
                let core: ICoreWebView2 = core;
                pending.args.SetNewWindow(&core).is_ok()
            }
            Err(_) => false,
        }
    };
    if !ok {
        return None;
    }

    attach_popup_content_handlers(&content, id, proxy, pending.incognito);
    Some(PopupWindow {
        content,
        bar,
        window,
        bar_h,
    })
}

#[cfg(windows)]
fn build_popup_content_webview(
    window: &tao::window::Window,
    rect: Rect,
    web_context: &mut wry::WebContext,
    incognito: bool,
    browser_args: &str,
) -> Option<WebView> {
    // No URL and no init script: WebView2 navigates this WebView to the popup target itself
    // once it is handed over via SetNewWindow. Same WebContext + browser args as the opener so
    // the new-window handoff is accepted.
    let builder = WebViewBuilder::new_as_child(window)
        .with_bounds(rect)
        .with_background_color(CONTENT_BG)
        .with_incognito(incognito)
        .with_user_agent(&browser_user_agent())
        .with_browser_accelerator_keys(false)
        .with_additional_browser_args(browser_args.to_string());
    let wv = builder.with_web_context(web_context).build().ok()?;
    let _ = wv.set_background_color(CONTENT_BG);
    Some(wv)
}

#[cfg(windows)]
fn build_popup_bar_webview(
    window: &tao::window::Window,
    rect: Rect,
    web_context: &mut wry::WebContext,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    id: u64,
) -> Option<WebView> {
    let builder = WebViewBuilder::new_as_child(window)
        .with_bounds(rect)
        .with_background_color((12, 10, 9, 255))
        .with_html(ui::popup::popup_chrome_html())
        .with_browser_accelerator_keys(false)
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(req.body()) else {
                return;
            };
            match v.get("cmd").and_then(|c| c.as_str()) {
                Some("popup_close") => {
                    let _ = proxy.send_event(AppEvent::PopupClose { id });
                }
                Some("popup_drag") => {
                    let _ = proxy.send_event(AppEvent::PopupDrag { id });
                }
                _ => {}
            }
        });
    builder.with_web_context(web_context).build().ok()
}

#[cfg(windows)]
fn attach_popup_content_handlers(
    content: &WebView,
    id: u64,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    incognito: bool,
) {
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::ICoreWebView2, SourceChangedEventHandler,
        WindowCloseRequestedEventHandler,
    };

    let webview: ICoreWebView2 = unsafe {
        match content.controller().CoreWebView2() {
            Ok(w) => w,
            Err(_) => return,
        }
    };

    let proxy_src = proxy.clone();
    let src = SourceChangedEventHandler::create(Box::new(move |sender, _args| {
        if let Some(sender) = sender {
            let url = unsafe { webview_source(&sender) };
            let _ = proxy_src.send_event(AppEvent::PopupUrlChanged { id, url });
        }
        Ok(())
    }));
    let mut t = Default::default();
    unsafe {
        let _ = webview.add_SourceChanged(&src, &mut t);
    }

    let proxy_close = proxy.clone();
    let close = WindowCloseRequestedEventHandler::create(Box::new(move |_s, _a| {
        let _ = proxy_close.send_event(AppEvent::PopupClose { id });
        Ok(())
    }));
    let mut t2 = Default::default();
    unsafe {
        let _ = webview.add_WindowCloseRequested(&close, &mut t2);
    }

    // Nested new windows from inside the popup follow the same tab/popup rules.
    attach_new_window_handler(content, proxy, incognito);
}

#[cfg(windows)]
fn center_popup(popup: &tao::window::Window, main: &tao::window::Window, w: u32, h: u32) {
    let Ok(main_pos) = main.outer_position() else {
        return;
    };
    let main_size = main.outer_size();
    let scale = main.scale_factor();
    let pw = logical_to_physical(w as f64, scale) as i32;
    let ph = logical_to_physical(h as f64, scale) as i32;
    let x = main_pos.x + (main_size.width as i32 - pw) / 2;
    let y = main_pos.y + (main_size.height as i32 - ph) / 2;
    popup.set_outer_position(tao::dpi::PhysicalPosition::new(x.max(0), y.max(0)));
}

fn layout_popup(p: &PopupWindow) {
    let size = p.window.inner_size();
    let scale = p.window.scale_factor();
    let bar_px = logical_to_physical(p.bar_h as f64, scale)
        .min(size.height.saturating_sub(1).max(1))
        .max(1);
    set_content_bounds(
        &p.bar,
        Rect {
            x: 0,
            y: 0,
            width: size.width.max(1),
            height: bar_px,
        },
    );
    set_content_bounds(
        &p.content,
        Rect {
            x: 0,
            y: bar_px as i32,
            width: size.width.max(1),
            height: size.height.saturating_sub(bar_px).max(1),
        },
    );
}

fn popup_origin(url: &str) -> (String, bool) {
    match url::Url::parse(url) {
        Ok(u) => (
            u.host_str().unwrap_or_default().to_string(),
            u.scheme() == "https",
        ),
        Err(_) => (String::new(), false),
    }
}

#[cfg(windows)]
unsafe fn webview_source(
    webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
) -> String {
    let mut ptr = wv2core::PWSTR::null();
    if webview.Source(&mut ptr).is_err() {
        return String::new();
    }
    take_pwstr(ptr)
}

fn is_recoverable_nav_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")
}

fn canceled_web_error_status(status: i32) -> bool {
    matches!(status, 0 | 9 | 14)
}

#[cfg(windows)]
unsafe fn take_pwstr(ptr: wv2core::PWSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let s = ptr.to_string().unwrap_or_default();
    windows::Win32::System::Com::CoTaskMemFree(Some(ptr.0 as *const _));
    s
}

fn https_to_http(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    parsed.set_scheme("http").ok()?;
    Some(parsed.to_string())
}

fn https_nav_url(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "http" || is_local_http_host(parsed.host_str()) {
        return None;
    }
    parsed.set_scheme("https").ok()?;
    Some(parsed.to_string())
}

fn is_local_http_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.trim_matches(['[', ']']).to_ascii_lowercase();
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "0.0.0.0" | "::1")
}

#[cfg(windows)]
fn attach_accelerators(wv: &WebView, proxy: tao::event_loop::EventLoopProxy<AppEvent>) {
    use webview2_com::{
        AcceleratorKeyPressedEventHandler,
        Microsoft::Web::WebView2::Win32::{
            COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
            COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN,
        },
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};

    let controller = wv.controller();
    let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };

        unsafe {
            let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
            args.KeyEventKind(&mut kind)?;
            if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN
            {
                return Ok(());
            }

            let mut vk = 0;
            args.VirtualKey(&mut vk)?;
            let mut lparam = 0;
            args.KeyEventLParam(&mut lparam)?;
            let repeat = (lparam as u32 & (1 << 30)) != 0;
            let mut mods = 0;
            if (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 {
                mods |= MOD_CTRL;
            }
            if (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 {
                mods |= MOD_SHIFT;
            }
            if (GetKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 {
                mods |= MOD_ALT;
            }
            let code = msg_shortcut(vk, mods, repeat);
            if code != SC_NONE {
                args.SetHandled(true)?;
                let _ = proxy.send_event(AppEvent::Shortcut { code: code as u32 });
            }
        }

        Ok(())
    }));
    let mut token = Default::default();
    unsafe {
        let _ = controller.add_AcceleratorKeyPressed(&handler, &mut token);
    }
}

/// Create a content WebView, retrying briefly when WebView2 reports the user-data profile
/// is locked / in use (HRESULT 0x800700AA, ERROR_BUSY).
///
/// That error means a previous Ventus — or an orphaned `msedgewebview2.exe` child left by
/// an unclean shutdown (crash, force-kill, or the old one-environment-per-tab behaviour) —
/// has not yet released the `webview_data` profile lock. Without a retry the tab would have
/// no WebView at all and sit permanently black with a spinning loader (the exact "loads
/// forever" failure). The lock is transient: a stale WebView2 browser process exits once
/// its host is gone, so a few short attempts recover cleanly. Only the failure path
/// sleeps — a normal create returns on the first attempt with no delay. In practice this
/// only ever matters for the FIRST content WebView (which creates the shared environment);
/// later tabs reuse that environment and never touch the profile lock.
fn build_content_webview(
    window: &tao::window::Window,
    tab_id: &str,
    url: &str,
    rect: Rect,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    web_context: &mut wry::WebContext,
    incognito: bool,
    download_dir: std::sync::Arc<std::sync::Mutex<DownloadPrefs>>,
    global_zoom: f64,
    browser_args: &str,
    ad_block_script: String,
    fingerprint: bool,
    strict: bool,
    site_permissions: config::SitePermissionMap,
    default_permissions: config::SitePermissions,
    https_only: bool,
    load_now: bool,
) -> anyhow::Result<WebView> {
    const MAX_ATTEMPTS: u32 = 6;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let result = build_content_webview_once(
            window,
            tab_id,
            url,
            rect,
            proxy.clone(),
            web_context,
            incognito,
            std::sync::Arc::clone(&download_dir),
            global_zoom,
            browser_args,
            ad_block_script.clone(),
            fingerprint,
            strict,
            site_permissions.clone(),
            default_permissions.clone(),
            https_only,
            load_now,
        );
        match result {
            Ok(wv) => {
                if attempt > 1 {
                    tracing::info!(target: "ventus::nav", tab = %tab_id, attempt, "content WebView created after retrying a busy profile lock");
                }
                return Ok(wv);
            }
            Err(e) if attempt < MAX_ATTEMPTS && is_profile_busy_error(&e) => {
                tracing::warn!(target: "ventus::nav", tab = %tab_id, attempt, error = %e, "content WebView profile busy (locked); retrying");
                std::thread::sleep(Duration::from_millis(600));
            }
            Err(e) => return Err(e),
        }
    }
}

/// True when a WebView2 build error is the transient "user-data profile is locked / in use"
/// failure (ERROR_BUSY, 0x800700AA) that a short retry can clear.
fn is_profile_busy_error(err: &anyhow::Error) -> bool {
    is_busy_message(&err.to_string())
}

fn is_busy_message(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("0x800700aa") || m.contains("requested resource is in use")
}

fn build_content_webview_once(
    window: &tao::window::Window,
    tab_id: &str,
    url: &str,
    rect: Rect,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    web_context: &mut wry::WebContext,
    incognito: bool,
    download_dir: std::sync::Arc<std::sync::Mutex<DownloadPrefs>>,
    global_zoom: f64,
    browser_args: &str,
    ad_block_script: String,
    fingerprint: bool,
    strict: bool,
    site_permissions: config::SitePermissionMap,
    default_permissions: config::SitePermissions,
    https_only: bool,
    load_now: bool,
) -> anyhow::Result<WebView> {
    let is_neura = url.starts_with("neura://");
    let proxy_ipc = proxy.clone();
    #[cfg(not(windows))]
    let proxy_load = proxy.clone();
    let tab_id_ipc = tab_id.to_string();
    #[cfg(not(windows))]
    let tab_id_str = tab_id.to_string();
    #[cfg(not(windows))]
    let _ = &download_dir;

    let builder = WebViewBuilder::new_as_child(window)
        .with_bounds(rect)
        .with_background_color(CONTENT_BG)
        .with_incognito(incognito)
        .with_user_agent(&browser_user_agent())
        .with_initialization_script(&content_initialization_script(
            global_zoom,
            &ad_block_script,
            fingerprint,
            strict,
            &site_permissions,
            &default_permissions,
        ));
    #[cfg(windows)]
    let builder = builder
        .with_browser_accelerator_keys(false)
        .with_additional_browser_args(browser_args.to_string());
    let builder = builder
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body();
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
                if value.get("cmd").and_then(|v| v.as_str()) == Some("open_in_new_tab") {
                    let url = value
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !url.trim().is_empty() {
                        let _ = proxy_ipc
                            .send_event(AppEvent::Chrome(ChromeCommand::OpenInNewTab { url }));
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("context_menu") {
                    let x = value.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let y = value.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let link_url = value
                        .get("link_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let image_src = value
                        .get("image_src")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let selected_text = value
                        .get("selected_text")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let page_url = value
                        .get("page_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let can_back = value
                        .get("can_back")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let _ = proxy_ipc.send_event(AppEvent::ContentContextMenu {
                        tab_id: tab_id_ipc.clone(),
                        x,
                        y,
                        link_url,
                        image_src,
                        selected_text,
                        page_url,
                        can_back,
                    });
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("content_nav_state") {
                    let can_back = value
                        .get("can_back")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let can_forward = value
                        .get("can_forward")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let _ = proxy_ipc.send_event(AppEvent::ContentNavState {
                        tab_id: tab_id_ipc.clone(),
                        can_back,
                        can_forward,
                    });
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("content_load_start") {
                    let url = value
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !url.trim().is_empty() && url != "about:blank" {
                        let _ = proxy_ipc.send_event(AppEvent::ContentLoadStart {
                            tab_id: tab_id_ipc.clone(),
                            url,
                            native: false,
                            nav_id: 0,
                        });
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("content_progress") {
                    let progress = value
                        .get("progress")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0);
                    let url = value
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let _ = proxy_ipc.send_event(AppEvent::ContentLoadProgress {
                        tab_id: tab_id_ipc.clone(),
                        url,
                        progress,
                    });
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("content_metadata") {
                    let url = value
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let title = value
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let favicon = value
                        .get("favicon")
                        .and_then(|v| v.as_str())
                        .filter(|v| !v.is_empty())
                        .map(|v| v.to_string());
                    let replace = value
                        .get("replace")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let _ = proxy_ipc.send_event(AppEvent::ContentMetadata {
                        tab_id: tab_id_ipc.clone(),
                        url,
                        title,
                        favicon,
                        replace,
                    });
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("ai_tool_result") {
                    let call_id = value
                        .get("call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let result = value
                        .get("result")
                        .and_then(|v| v.as_str())
                        .unwrap_or("{}")
                        .to_string();
                    let _ = proxy_ipc.send_event(AppEvent::AiToolResult { call_id, result });
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("save_image_data") {
                    let id = value
                        .get("save_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    let data = value
                        .get("data")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let _ = proxy_ipc.send_event(AppEvent::SaveImageData { id, ok, data });
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("pwd_fill_request") {
                    let origin = value
                        .get("origin")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !origin.is_empty() {
                        let _ = proxy_ipc.send_event(AppEvent::PwdFillRequest {
                            tab_id: tab_id_ipc.clone(),
                            origin,
                        });
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("pwd_capture") {
                    let origin = value
                        .get("origin")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let username = value
                        .get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let password = value
                        .get("password")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !origin.is_empty() {
                        let _ = proxy_ipc.send_event(AppEvent::PwdCapture {
                            tab_id: tab_id_ipc.clone(),
                            origin,
                            username,
                            password,
                        });
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("content_pointer_down") {
                    let _ =
                        proxy_ipc.send_event(AppEvent::Chrome(ChromeCommand::ContentPointerDown));
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("web_notification") {
                    let get = |k: &str| {
                        value
                            .get(k)
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string()
                    };
                    let id = get("id");
                    if !id.is_empty() {
                        let _ = proxy_ipc.send_event(AppEvent::WebNotification {
                            tab_id: tab_id_ipc.clone(),
                            id,
                            title: get("title"),
                            body: get("body"),
                            icon: get("icon"),
                            origin: get("origin"),
                        });
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("web_notification_close") {
                    let id = value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !id.is_empty() {
                        let _ = proxy_ipc.send_event(AppEvent::WebNotificationClose {
                            tab_id: tab_id_ipc.clone(),
                            id,
                        });
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("begin_resize") {
                    let edge = value
                        .get("edge")
                        .and_then(|v| v.as_str())
                        .unwrap_or("right")
                        .to_string();
                    let _ =
                        proxy_ipc.send_event(AppEvent::Chrome(ChromeCommand::BeginResize { edge }));
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("tab_audio_state") {
                    let playing = value
                        .get("playing")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let active = value
                        .get("active")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(playing);
                    let _ = proxy_ipc.send_event(AppEvent::Chrome(ChromeCommand::TabAudioState {
                        tab_id: tab_id_ipc.clone(),
                        playing,
                        active,
                    }));
                    return;
                }
            }
            // SECURITY: content WebViews host untrusted web pages, and every page has
            // window.ipc.postMessage. This fallthrough must NOT forward commands blindly —
            // doing so would let any site disable HTTPS-only (save_settings), execute a
            // local file (open_file), trigger an update install (install_update), close the
            // window (window_close), hijack navigation, etc.
            //
            // Only forward the small set of NON-privileged UI/state commands that the
            // injected content_initialization_script legitimately emits and that are not
            // already handled explicitly above. Everything else is dropped.
            //
            // When adding a new content→Rust command to the content init script, it MUST be
            // added to this whitelist too, otherwise it will be silently ignored here.
            if let Ok(cmd) = serde_json::from_str::<ChromeCommand>(body) {
                let allowed = matches!(
                    cmd,
                    ChromeCommand::SidebarAutoClose                  // mousemove auto-hide signal
                        | ChromeCommand::ToggleFullscreen            // F11 inside page content
                        | ChromeCommand::ContentFullscreenChange { .. } // video-player fullscreen
                        | ChromeCommand::BeginSpotlight              // Ctrl+T
                        | ChromeCommand::ReopenTab                   // Ctrl+Shift+T
                        | ChromeCommand::SwitchTabOffset { .. }
                        | ChromeCommand::OpenHistoryPanel            // Ctrl+H
                        | ChromeCommand::OpenDownloadsPanel          // Ctrl+J
                        | ChromeCommand::ZoomDelta { .. }            // Ctrl+wheel zoom
                        | ChromeCommand::DragEdgePeek // left-edge dwell while dragging
                );
                if allowed {
                    let _ = proxy_ipc.send_event(AppEvent::Chrome(cmd));
                } else {
                    tracing::debug!("blocked untrusted content IPC command");
                }
            }
        })
        .with_navigation_handler({
            let proxy_nav = proxy.clone();
            #[cfg(not(windows))]
            let tab_id_nav = tab_id.to_string();
            move |url| {
                if https_only {
                    if https_nav_url(&url).is_some() {
                        let _ =
                            proxy_nav.send_event(AppEvent::Chrome(ChromeCommand::Navigate { url }));
                        return false;
                    }
                }
                if !url.trim().is_empty() && url != "about:blank" {
                    #[cfg(not(windows))]
                    {
                        let _ = proxy_nav.send_event(AppEvent::ContentLoadStart {
                            tab_id: tab_id_nav.clone(),
                            url: url.clone(),
                            native: true,
                            nav_id: 0,
                        });
                    }
                }
                true
            }
        })
        .with_on_page_load_handler(move |event, loaded_url: String| match event {
            wry::PageLoadEvent::Started => {
                let nav_url = if is_neura && !loaded_url.starts_with("http") {
                    "neura://newtab".to_string()
                } else {
                    loaded_url
                };
                #[cfg(windows)]
                let _ = nav_url;
                #[cfg(not(windows))]
                {
                    let _ = proxy_load.send_event(AppEvent::ContentLoadStart {
                        tab_id: tab_id_str.clone(),
                        url: nav_url,
                        native: true,
                        nav_id: 0,
                    });
                }
            }
            wry::PageLoadEvent::Finished => {
                let nav_url = if is_neura && !loaded_url.starts_with("http") {
                    "neura://newtab".to_string()
                } else {
                    loaded_url
                };
                #[cfg(windows)]
                let _ = nav_url;
                #[cfg(not(windows))]
                {
                    let _ = proxy_load.send_event(AppEvent::ContentNav {
                        tab_id: tab_id_str.clone(),
                        url: nav_url.clone(),
                        title: String::new(),
                    });
                    let _ = proxy_load.send_event(AppEvent::ContentLoadEnd {
                        tab_id: tab_id_str.clone(),
                        start_url: nav_url.clone(),
                        url: nav_url,
                        nav_id: 0,
                    });
                }
            }
        });

    let wv = if is_neura {
        builder
            .with_web_context(web_context)
            .with_html(ui::new_tab::new_tab_html())
            .build()?
    } else if load_now {
        builder
            .with_web_context(web_context)
            .with_url(url)
            .build()?
    } else {
        builder.with_web_context(web_context).build()?
    };
    let _ = wv.set_background_color(CONTENT_BG);
    let _ = wv.zoom(global_zoom);
    #[cfg(windows)]
    attach_accelerators(&wv, proxy.clone());
    #[cfg(windows)]
    attach_fullscreen_handler(&wv, proxy.clone());
    #[cfg(windows)]
    {
        attach_process_failed_handler(&wv, proxy.clone(), tab_id.to_string());
        attach_navigation_handler(&wv, proxy.clone(), tab_id.to_string());
        attach_new_window_handler(&wv, proxy.clone(), incognito);
        attach_permission_handler(
            &wv,
            proxy.clone(),
            strict,
            site_permissions.clone(),
            default_permissions.clone(),
        );
        attach_download_handler(&wv, proxy.clone(), std::sync::Arc::clone(&download_dir));
    }

    tracing::info!(target: "ventus::nav", tab = %tab_id, url = %url, load_now, incognito, "content WebView built");
    Ok(wv)
}

fn wake_content_webview(wv: &WebView) {
    resume_content_webview(wv);
    let _ = wv.evaluate_script(
        "try{window.focus();window.dispatchEvent(new Event('focus'));window.dispatchEvent(new Event('resize'));document.dispatchEvent(new Event('visibilitychange'));}catch(_){ }",
    );
}

#[cfg(windows)]
fn set_content_memory_low(wv: &WebView) {
    let _ = wv.set_memory_usage_level(MemoryUsageLevel::Low);
}

#[cfg(not(windows))]
fn set_content_memory_low(_wv: &WebView) {}

#[cfg(windows)]
fn set_content_memory_normal(wv: &WebView) {
    let _ = wv.set_memory_usage_level(MemoryUsageLevel::Normal);
}

#[cfg(not(windows))]
fn set_content_memory_normal(_wv: &WebView) {}

#[cfg(windows)]
fn resume_content_webview(wv: &WebView) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2, ICoreWebView2_3};
    use wv2core::Interface;
    set_content_memory_normal(wv);
    let controller = wv.controller();
    let core: ICoreWebView2 = match unsafe { controller.CoreWebView2() } {
        Ok(c) => c,
        Err(_) => return,
    };
    let Ok(core3) = core.cast::<ICoreWebView2_3>() else {
        return;
    };
    unsafe {
        let _ = core3.Resume();
    }
}

#[cfg(not(windows))]
fn resume_content_webview(_wv: &WebView) {}

#[cfg(windows)]
fn sleep_content_webview(wv: &WebView) -> bool {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2, ICoreWebView2_3};
    use webview2_com::TrySuspendCompletedHandler;
    use wv2core::Interface;
    set_content_memory_low(wv);
    let _ = wv.set_visible(false);
    let controller = wv.controller();
    let core: ICoreWebView2 = match unsafe { controller.CoreWebView2() } {
        Ok(c) => c,
        Err(_) => return false,
    };
    let Ok(core3) = core.cast::<ICoreWebView2_3>() else {
        return false;
    };
    let handler = TrySuspendCompletedHandler::create(Box::new(move |_err, _ok| Ok(())));
    unsafe { core3.TrySuspend(&handler).is_ok() }
}

#[cfg(not(windows))]
fn sleep_content_webview(_wv: &WebView) -> bool {
    false
}

fn clear_tab_favicon(state: &mut AppState, tab_id: &str) {
    if let Some(tab) = state.tab_manager.tabs.iter_mut().find(|t| t.id == tab_id) {
        tab.favicon = None;
    }
}

fn begin_native_load(state: &mut AppState, chrome: &WebView, tab_id: &str) {
    clear_tab_favicon(state, tab_id);
    state.native_loads.remove(tab_id);
    state.native_nav_ids.remove(tab_id);
    state.tab_manager.set_tab_loading(tab_id, true);
    state.load_progress.insert(tab_id.to_string(), 0.0);
    if state.tab_manager.active_tab_id.as_deref() == Some(tab_id) {
        state.set_content_cover(chrome, true);
        let _ = chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
    }
}

#[derive(Clone)]
struct DownloadPrefs {
    dir: std::path::PathBuf,
    ask: bool,
}

fn download_prefs_from_settings(settings: &config::AppSettings) -> DownloadPrefs {
    DownloadPrefs {
        dir: download_dir_from_settings(settings),
        ask: settings.downloads.ask_where_to_save,
    }
}

fn webview_args(settings: &config::AppSettings) -> String {
    // NOTE: do NOT disable "msPdfOOUI" — on current WebView2 runtimes that makes the
    // built-in PDF viewer render blank/stuck, breaking file:// PDF tabs (the "Read PDF"
    // feature). Keeping it enabled also restores the PDF toolbar (zoom/print/download).
    // "msWebOOUI" only governs web overlay UI (web capture/select) and is safe to disable.
    let disable_features = vec![
        "msWebOOUI".to_string(),
        "msSmartScreenProtection".to_string(),
        "SleepingTabs".to_string(),
        "AutoDiscardTabs".to_string(),
        "CalculateNativeWinOcclusion".to_string(),
    ];
    let mut enable_features = vec!["ParallelDownloading".to_string()];
    let mut args = vec![
        "--no-first-run".to_string(),
        "--disk-cache-size=1073741824".to_string(),
        "--media-cache-size=536870912".to_string(),
    ];
    if settings.privacy.block_third_party_cookies {
        args.push("--webview-force-disable-3pcs".to_string());
    }
    if let Some(url) = settings.privacy.secure_dns_endpoint() {
        args.push("--enable-async-dns".to_string());
        enable_features.push(doh_feature_arg(&url, &settings.privacy.secure_dns_mode));
        args.push(format!(
            "--dns-over-https-mode={}",
            settings.privacy.secure_dns_mode.as_arg()
        ));
        args.push(format!("--dns-over-https-templates={}", url));
    }
    if !disable_features.is_empty() {
        args.push(format!("--disable-features={}", disable_features.join(",")));
    }
    if !enable_features.is_empty() {
        args.push(format!("--enable-features={}", enable_features.join(",")));
    }
    args.join(" ")
}

fn browser_user_agent() -> String {
    let (_, reduced, _) = chromium_versions();
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{reduced} Safari/537.36 Ventus/{reduced}"
    )
}

fn chromium_versions() -> (String, String, String) {
    let raw = wry::webview_version().ok().unwrap_or_default();
    chromium_versions_from_raw(&raw).unwrap_or_else(|| {
        let full = "0.0.0.0".to_string();
        (full.clone(), full, "0".to_string())
    })
}

fn chromium_versions_from_raw(raw: &str) -> Option<(String, String, String)> {
    let tokens = raw
        .trim()
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map(|p| p.trim_matches('.'))
        .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .collect::<Vec<_>>();
    let version = tokens
        .iter()
        .find(|p| p.contains('.'))
        .or_else(|| tokens.first())?;
    let parts = version
        .split('.')
        .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        .take(4)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let Some(major) = parts.first().cloned() else {
        return None;
    };
    let mut full = parts;
    while full.len() < 4 {
        full.push("0".to_string());
    }
    Some((full.join("."), format!("{major}.0.0.0"), major))
}

fn doh_feature_arg(url: &str, mode: &config::SecureDnsMode) -> String {
    let fallback = matches!(mode, config::SecureDnsMode::Automatic);
    let encoded_url: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
    format!("DnsOverHttps:Fallback/{fallback}/Templates/{encoded_url}")
}

fn sync_webview_secure_dns_prefs(profile_root: &std::path::Path, settings: &config::AppSettings) {
    if let Err(err) = write_webview_secure_dns_prefs(profile_root, settings) {
        tracing::warn!(
            "secure_dns: failed to update WebView2 prefs at {}: {}",
            profile_root.display(),
            err
        );
    }
}

fn write_webview_secure_dns_prefs(
    profile_root: &std::path::Path,
    settings: &config::AppSettings,
) -> anyhow::Result<()> {
    let profile_dir = profile_root.join("EBWebView");
    std::fs::create_dir_all(&profile_dir)?;
    let local_state_path = profile_dir.join("Local State");
    let mut local_state = match std::fs::read_to_string(&local_state_path) {
        Ok(json) => serde_json::from_str::<serde_json::Value>(&json)
            .unwrap_or_else(|_| serde_json::json!({})),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(err.into()),
    };
    apply_secure_dns_local_state(&mut local_state, settings);
    apply_webview_privacy_local_state(&mut local_state, settings);
    std::fs::write(local_state_path, serde_json::to_vec(&local_state)?)?;
    Ok(())
}

fn apply_secure_dns_local_state(
    local_state: &mut serde_json::Value,
    settings: &config::AppSettings,
) {
    let endpoint = settings.privacy.secure_dns_endpoint();
    let enabled = endpoint.is_some();
    let mode = if enabled {
        settings.privacy.secure_dns_mode.as_arg()
    } else {
        "off"
    };
    let templates = endpoint.unwrap_or_default();
    let fallback = enabled
        && matches!(
            settings.privacy.secure_dns_mode,
            config::SecureDnsMode::Automatic
        );

    let dns_prefs = json_object_child(local_state, "dns_over_https");
    dns_prefs.insert("mode".into(), serde_json::Value::String(mode.to_string()));
    dns_prefs.insert("templates".into(), serde_json::Value::String(templates));
    dns_prefs.insert(
        "automatic_mode_fallback_to_doh".into(),
        serde_json::Value::Bool(fallback),
    );

    let async_dns = json_object_child(local_state, "async_dns");
    async_dns.insert("enabled".into(), serde_json::Value::Bool(enabled));
}

fn apply_webview_privacy_local_state(
    local_state: &mut serde_json::Value,
    settings: &config::AppSettings,
) {
    let profile = json_object_child(local_state, "profile");
    profile.insert(
        "block_third_party_cookies".into(),
        serde_json::Value::Bool(settings.privacy.block_third_party_cookies),
    );
    profile.insert(
        "cookie_controls_mode".into(),
        serde_json::Value::Number(serde_json::Number::from(
            if settings.privacy.block_third_party_cookies {
                1
            } else {
                0
            },
        )),
    );
}

fn json_object_child<'a>(
    value: &'a mut serde_json::Value,
    key: &str,
) -> &'a mut serde_json::Map<String, serde_json::Value> {
    if !value.is_object() {
        *value = serde_json::json!({});
    }
    let root = value.as_object_mut().expect("JSON value is an object");
    let child = root
        .entry(key.to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !child.is_object() {
        *child = serde_json::json!({});
    }
    child.as_object_mut().expect("JSON child is an object")
}

fn download_dir_from_settings(settings: &config::AppSettings) -> std::path::PathBuf {
    let configured = settings.downloads.default_folder.trim();
    if !configured.is_empty() {
        return std::path::PathBuf::from(configured);
    }
    directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(std::path::PathBuf::from)
                .map(|home| home.join("Downloads"))
                .unwrap_or_else(|| std::path::PathBuf::from("Downloads"))
        })
}

fn load_settings(conn: &rusqlite::Connection) -> config::AppSettings {
    let mut settings = settings_store::get::<config::AppSettings>(conn, "app_settings")
        .unwrap_or(None)
        .unwrap_or_default();
    if let Some(v) = settings_store::get::<String>(conn, "homepage").unwrap_or(None) {
        settings.homepage = app::normalize_homepage(&v);
    }
    if let Some(v) = settings_store::get::<String>(conn, "download_path").unwrap_or(None) {
        settings.downloads.default_folder = v;
    }
    if let Some(v) = settings_store::get::<bool>(conn, "ask_where_to_save").unwrap_or(None) {
        settings.downloads.ask_where_to_save = v;
    }
    if let Some(v) = settings_store::get::<String>(conn, "startup_behavior").unwrap_or(None) {
        settings.startup_behavior = match v.as_str() {
            "last_session" => config::StartupBehavior::LastSession,
            "home_page" | "specific_pages" => config::StartupBehavior::HomePage,
            _ => config::StartupBehavior::NewTab,
        };
    }
    let migrated_new_tab_background =
        settings_store::get::<bool>(conn, "new_tab_background_default_migrated")
            .unwrap_or(None)
            .unwrap_or(false);
    if !migrated_new_tab_background {
        if settings.new_tab.wallpaper_source == "nature"
            && settings.new_tab.wallpaper_url.trim().is_empty()
            && settings.new_tab.wallpaper_data.trim().is_empty()
        {
            settings.new_tab.wallpaper_source = "none".to_string();
            settings.new_tab.show_background = false;
        }
        let _ = settings_store::set(conn, "new_tab_background_default_migrated", &true);
    }
    settings.homepage = app::normalize_homepage(&settings.homepage);
    let _ = settings_store::set(conn, "app_settings", &settings);
    settings
}

#[cfg(windows)]
fn encrypt_app_storage(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{core::PCWSTR, Win32::Storage::FileSystem::EncryptFileW};

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    if let Err(err) = unsafe { EncryptFileW(PCWSTR(wide.as_ptr())) } {
        tracing::debug!(
            "storage encryption unavailable for {}: {}",
            path.display(),
            err
        );
    }
}

#[cfg(not(windows))]
fn encrypt_app_storage(_path: &std::path::Path) {}

fn unique_download_path(download_dir: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let clean_name = if filename.trim().is_empty() {
        "download"
    } else {
        filename
    };
    let candidate = download_dir.join(clean_name);
    if !candidate.exists() {
        return candidate;
    }

    let path = std::path::Path::new(clean_name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("download");
    let ext = path.extension().and_then(|e| e.to_str());
    for i in 1..1000 {
        let filename = match ext {
            Some(ext) if !ext.is_empty() => format!("{} ({}).{}", stem, i, ext),
            _ => format!("{} ({})", stem, i),
        };
        let candidate = download_dir.join(filename);
        if !candidate.exists() {
            return candidate;
        }
    }
    download_dir.join(clean_name)
}

fn content_initialization_script(
    _global_zoom: f64,
    ad_block_script: &str,
    fingerprint: bool,
    strict: bool,
    site_permissions: &config::SitePermissionMap,
    default_permissions: &config::SitePermissions,
) -> String {
    let ad_prefix = if ad_block_script.is_empty() {
        String::new()
    } else {
        format!("{}\n", ad_block_script)
    };
    let identity_prefix = browser_identity_script();
    let privacy_prefix =
        privacy_initialization_script(fingerprint, strict, site_permissions, default_permissions);
    let script = r#"
(() => {
  let isTop = false;
  try { isTop = window.top === window; } catch (_) {}
  if (window.__neuraContentBridgeInstalled) return;
  window.__neuraContentBridgeInstalled = true;
  const post = (payload) => {
    try {
      if (window.ipc && typeof window.ipc.postMessage === 'function') {
        window.ipc.postMessage(JSON.stringify(payload));
      }
    } catch (_) {}
  };
  if (!isTop) return;
  (() => {
    const Native = window.Notification;
    if (!Native) return;
    let seq = 0;
    const live = {};
    function VN(title, opts) {
      opts = opts || {};
      const id = 'wn' + (++seq) + '_' + Date.now();
      this.title = String(title == null ? '' : title);
      this.body = opts.body || '';
      this.icon = opts.icon || '';
      this.tag = opts.tag || '';
      this.data = opts.data;
      this.dir = opts.dir || 'auto';
      this.lang = opts.lang || '';
      this.onclick = null; this.onclose = null; this.onerror = null; this.onshow = null;
      const L = {click: [], close: [], show: [], error: []};
      this.addEventListener = (t, f) => { if (L[t] && typeof f === 'function') L[t].push(f); };
      this.removeEventListener = (t, f) => { const a = L[t]; if (a) { const i = a.indexOf(f); if (i >= 0) a.splice(i, 1); } };
      this.dispatchEvent = () => true;
      this._fire = (t) => {
        const e = {type: t, target: this};
        try { const h = this['on' + t]; if (typeof h === 'function') h.call(this, e); } catch (_) {}
        (L[t] || []).forEach(f => { try { f.call(this, e); } catch (_) {} });
      };
      this.close = () => { post({cmd: 'web_notification_close', id}); delete live[id]; };
      live[id] = this;
      let icon = '';
      try { if (this.icon) icon = new URL(this.icon, location.href).href; } catch (_) {}
      post({cmd: 'web_notification', id, title: this.title, body: this.body, icon, origin: location.origin});
      setTimeout(() => this._fire('show'), 0);
    }
    Object.defineProperty(VN, 'permission', {get: () => Native.permission, configurable: true});
    VN.requestPermission = function() { try { return Native.requestPermission.apply(Native, arguments); } catch (_) { return Promise.resolve(Native.permission); } };
    try { VN.maxActions = Native.maxActions; } catch (_) {}
    window.__neuraNotifClick = (id) => { const n = live[id]; if (n) { n._fire('click'); try { window.focus(); } catch (_) {} } };
    window.__neuraNotifClose = (id) => { const n = live[id]; if (n) { n._fire('close'); delete live[id]; } };
    try { window.Notification = VN; } catch (_) {}
  })();
  const findApi = (() => {
    let q = '';
    let ranges = [];
    let spans = [];
    let idx = -1;
    const sid = '__ventus_find_style';
    const hn = 'ventus-find-match';
    const an = 'ventus-find-active';
    const addStyle = () => {
      if (document.getElementById(sid)) return;
      const el = document.createElement('style');
      el.id = sid;
      el.textContent = '::highlight(ventus-find-match){background:rgba(255,218,68,.72);color:inherit}::highlight(ventus-find-active){background:#ff9f1c;color:#111}.__ventus-find-match{background:rgba(255,218,68,.72);color:inherit;border-radius:2px}.__ventus-find-active{background:#ff9f1c!important;color:#111!important}';
      (document.head || document.documentElement).appendChild(el);
    };
    const useRanges = () => !!(window.CSS && CSS.highlights && typeof Highlight !== 'undefined' && typeof Range !== 'undefined');
    const clearRanges = () => {
      try {
        if (window.CSS && CSS.highlights) {
          CSS.highlights.delete(hn);
          CSS.highlights.delete(an);
        }
      } catch (_) {}
      ranges = [];
    };
    const clearSpans = () => {
      for (const span of spans) {
        const p = span.parentNode;
        if (!p) continue;
        p.replaceChild(document.createTextNode(span.textContent || ''), span);
        try { p.normalize(); } catch (_) {}
      }
      spans = [];
    };
    const clear = () => {
      clearRanges();
      clearSpans();
      idx = -1;
      try {
        const sel = window.getSelection && window.getSelection();
        if (sel) sel.removeAllRanges();
      } catch (_) {}
    };
    const skip = (n) => {
      if (!n || !n.nodeValue || !n.nodeValue.trim()) return true;
      const p = n.parentElement;
      if (!p) return true;
      return !!p.closest('script,style,noscript,textarea,input,select,option,.__ventus-find-match');
    };
    const nodes = () => {
      const root = document.body;
      if (!root || !window.NodeFilter) return [];
      const out = [];
      const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
        acceptNode(n) {
          return skip(n) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT;
        }
      });
      let n = w.nextNode();
      while (n) {
        out.push(n);
        n = w.nextNode();
      }
      return out;
    };
    const matchRanges = (needle) => {
      const out = [];
      const low = needle.toLowerCase();
      for (const n of nodes()) {
        const text = n.nodeValue || '';
        const hay = text.toLowerCase();
        let at = hay.indexOf(low);
        while (at >= 0) {
          const r = document.createRange();
          r.setStart(n, at);
          r.setEnd(n, at + needle.length);
          out.push(r);
          at = hay.indexOf(low, at + needle.length);
        }
      }
      return out;
    };
    const total = () => ranges.length || spans.length;
    const next = (same, forward) => {
      const n = total();
      if (!n) return -1;
      if (!same || idx < 0) return forward ? 0 : n - 1;
      return (idx + (forward ? 1 : -1) + n) % n;
    };
    const res = (needle) => ({query: needle, total: total(), index: idx >= 0 ? idx + 1 : 0});
    const rangeRect = (r) => {
      const rect = r.getBoundingClientRect();
      if (rect && (rect.width || rect.height)) return rect;
      const rs = r.getClientRects();
      return rs && rs.length ? rs[0] : null;
    };
    const showRange = () => {
      if (idx < 0 || !ranges[idx]) return;
      try {
        const h = new Highlight();
        h.add(ranges[idx]);
        CSS.highlights.set(an, h);
      } catch (_) {}
      try {
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(ranges[idx]);
      } catch (_) {}
      const rect = rangeRect(ranges[idx]);
      if (rect) window.scrollBy({top: rect.top - window.innerHeight * 0.35, left: rect.left - window.innerWidth * 0.35, behavior: 'smooth'});
    };
    const runRanges = (needle, forward) => {
      ranges = matchRanges(needle);
      try {
        const h = new Highlight();
        ranges.forEach(r => h.add(r));
        CSS.highlights.set(hn, h);
      } catch (_) {}
      idx = next(false, forward);
      showRange();
      return res(needle);
    };
    const makeSpans = (needle) => {
      const low = needle.toLowerCase();
      for (const n of nodes()) {
        const text = n.nodeValue || '';
        const hay = text.toLowerCase();
        let at = hay.indexOf(low);
        if (at < 0) continue;
        const frag = document.createDocumentFragment();
        let last = 0;
        while (at >= 0) {
          if (at > last) frag.appendChild(document.createTextNode(text.slice(last, at)));
          const span = document.createElement('span');
          span.className = '__ventus-find-match';
          span.textContent = text.slice(at, at + needle.length);
          spans.push(span);
          frag.appendChild(span);
          last = at + needle.length;
          at = hay.indexOf(low, last);
        }
        if (last < text.length) frag.appendChild(document.createTextNode(text.slice(last)));
        n.parentNode.replaceChild(frag, n);
      }
    };
    const showSpan = () => {
      spans.forEach(s => s.classList.remove('__ventus-find-active'));
      if (idx < 0 || !spans[idx]) return;
      const s = spans[idx];
      s.classList.add('__ventus-find-active');
      try { s.scrollIntoView({block: 'center', inline: 'nearest', behavior: 'smooth'}); } catch (_) {}
      try {
        const r = document.createRange();
        r.selectNodeContents(s);
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(r);
      } catch (_) {}
    };
    const runSpans = (needle, forward) => {
      makeSpans(needle);
      idx = next(false, forward);
      showSpan();
      return res(needle);
    };
    const run = (value, forward) => {
      const needle = String(value || '');
      if (!needle) {
        clear();
        q = '';
        return res('');
      }
      const same = needle === q && total() > 0;
      if (same) {
        idx = next(true, forward !== false);
        if (ranges.length) showRange(); else showSpan();
        return res(needle);
      }
      clear();
      q = needle;
      addStyle();
      return useRanges() ? runRanges(needle, forward !== false) : runSpans(needle, forward !== false);
    };
    return {run, clear};
  })();
  window.__neuraFind = findApi;
  const sendProgress = (progress) => {
    if (isTop) post({cmd:'content_progress', progress, url: location.href});
  };
  const faviconHref = () => {
    const root = document.head || document.documentElement;
    if (!root) return '';
    const icons = root.querySelectorAll('link[rel]');
    for (const icon of icons) {
      const rel = (icon.getAttribute('rel') || '').toLowerCase();
      if (icon.href && (rel.includes('icon') || rel.includes('apple-touch-icon') || rel.includes('mask-icon'))) return icon.href;
    }
    try { return new URL('/favicon.ico', location.href).href; } catch (_) { return ''; }
  };
  let lastHref = location.href;
  let lastMeta = '';
  let metaTimer = 0;
  const sendMetadata = (replace = false) => {
    if (!isTop) return;
    const favicon = faviconHref();
    const title = document.title || location.href;
    const key = location.href + '\n' + title + '\n' + favicon + '\n' + replace;
    if (key === lastMeta) return;
    lastMeta = key;
    post({
      cmd:'content_metadata',
      url: location.href,
      title,
      favicon,
      replace
    });
  };
  const queueMetadata = (replace = false, wait = 50) => {
    clearTimeout(metaTimer);
    metaTimer = setTimeout(() => sendMetadata(replace), wait);
  };
  const sendLocationChange = (replace = false) => {
    if (!isTop) return;
    const href = location.href;
    if (href === lastHref) {
      setTimeout(() => {
        queueMetadata(replace);
        sendNavState();
      }, 50);
      return;
    }
    lastHref = href;
    post({cmd:'content_load_start', url: href});
    sendProgress(0.22);
    let done = false;
    let obs = null;
    let settleTimer = 0;
    const finish = () => {
      if (done) return;
      done = true;
      clearTimeout(settleTimer);
      if (obs) obs.disconnect();
      queueMetadata(replace);
      sendNavState();
      sendProgress(0.96);
    };
    const settle = () => {
      clearTimeout(settleTimer);
      settleTimer = setTimeout(finish, 600);
    };
    try {
      if (document.body) {
        obs = new MutationObserver(settle);
        obs.observe(document.body, {childList:true, subtree:true});
      }
    } catch (_) {}
    setTimeout(() => {
      queueMetadata(replace);
      sendNavState();
      sendProgress(0.72);
    }, 350);
    setTimeout(finish, 8000);
  };
  const sendNavState = () => {
    if (!isTop) return;
    let canBack = false;
    let canFwd = false;
    try {
      const nav = window.navigation;
      if (nav && typeof nav.canGoBack === 'boolean') {
        canBack = nav.canGoBack;
        canFwd = nav.canGoForward;
      } else if (nav && nav.currentEntry) {
        const idx = nav.currentEntry.index;
        const len = nav.entries ? nav.entries().length : 0;
        canBack = idx > 0;
        canFwd = len > 0 && idx < len - 1;
      } else {
        canBack = history.length > 1;
      }
    } catch(_) {}
    try { post({cmd:'content_nav_state', can_back: canBack, can_forward: canFwd}); } catch(_) {}
  };

  sendProgress(0.12);
  document.addEventListener('readystatechange', () => {
    if (document.readyState === 'interactive') {
      sendProgress(0.65);
      queueMetadata();
    } else if (document.readyState === 'complete') {
      queueMetadata();
      sendNavState();
      sendProgress(0.92);
    }
  });
  window.addEventListener('DOMContentLoaded', () => {
    sendProgress(0.75);
    queueMetadata();
    sendNavState();
  });
  window.addEventListener('load', () => {
    queueMetadata();
    sendNavState();
    sendProgress(0.96);
  });
  try {
    const watchFavicons = () => {
      const head = document.head;
      if (!head) {
        setTimeout(watchFavicons, 120);
        return;
      }
      const favObs = new MutationObserver(records => {
        for (const r of records) {
          if (r.target && r.target.tagName === 'LINK') {
            queueMetadata(true, 80);
            return;
          }
          for (const n of r.addedNodes || []) {
            if (n.tagName === 'LINK') {
              queueMetadata(true, 80);
              return;
            }
          }
        }
      });
      favObs.observe(head, {subtree:true, childList:true, attributes:true, attributeFilter:['href','rel']});
    };
    watchFavicons();
  } catch (_) {}
  setInterval(() => {
    if (location.href !== lastHref) sendLocationChange(false);
  }, 1000);
  const pushState = history.pushState;
  history.pushState = function() {
    const result = pushState.apply(this, arguments);
    sendLocationChange(false);
    return result;
  };
  const replaceState = history.replaceState;
  history.replaceState = function() {
    const result = replaceState.apply(this, arguments);
    sendLocationChange(true);
    return result;
  };
  window.addEventListener('popstate', () => { sendLocationChange(true); });
  setTimeout(sendMetadata, 1200);


  // Drop a link dragged from outside (another browser, the desktop) → open a new tab.
  // Bubble phase + defaultPrevented check so a page's own drop zone wins; editable targets
  // keep native text-insert behavior; file drags are left to the page so uploads still work.
  const dragHasLink = (dt) => {
    if (!dt) return false;
    const t = dt.types || [];
    const has = (x) => Array.prototype.indexOf.call(t, x) !== -1;
    if (has('Files')) return false;
    return has('text/uri-list') || has('text/plain');
  };
  const dropTargetEditable = (el) => !!(el && el.closest && el.closest(
    'input,textarea,select,[contenteditable=""],[contenteditable="true"]'
  ));
  const extractDropUrl = (dt) => {
    if (!dt) return '';
    try {
      let u = (dt.getData('text/uri-list') || '').split('\n').find((l) => l && l[0] !== '#');
      if (!u) u = dt.getData('text/plain') || '';
      u = (u || '').trim();
      if (/^https?:\/\//i.test(u)) return u;
      if (/^[a-z0-9.-]+\.[a-z]{2,}([\/?#]|$)/i.test(u)) return u;
    } catch (_) {}
    return '';
  };
  let __edgeTimer = 0;
  let __neuraInternalDrag = false;
  const __clearEdgeTimer = () => { if (__edgeTimer) { clearTimeout(__edgeTimer); __edgeTimer = 0; } };
  window.addEventListener('dragstart', function() { __neuraInternalDrag = true; }, true);
  window.addEventListener('dragover', function(e) {
    if (e.defaultPrevented || dropTargetEditable(e.target)) { __clearEdgeTimer(); return; }
    if (__neuraInternalDrag) { __clearEdgeTimer(); return; }
    if (!dragHasLink(e.dataTransfer)) { __clearEdgeTimer(); return; }
    e.preventDefault();
    try { e.dataTransfer.dropEffect = 'copy'; } catch (_) {}
    // Dwell at the left window edge for 2s during a drag → open the auto-hide sidebar so the
    // user can drop the link onto it. Rust ignores this unless the sidebar is auto-hide + closed.
    if (e.clientX <= 24) {
      if (!__edgeTimer) {
        __edgeTimer = setTimeout(function() { __edgeTimer = 0; post({cmd:'drag_edge_peek'}); }, 2000);
      }
    } else {
      __clearEdgeTimer();
    }
  }, false);
  window.addEventListener('drop', function(e) {
    __clearEdgeTimer();
    if (e.defaultPrevented || dropTargetEditable(e.target)) return;
    if (__neuraInternalDrag) return;
    const url = extractDropUrl(e.dataTransfer);
    if (!url) return;
    e.preventDefault();
    post({cmd:'open_in_new_tab', url});
  }, false);
  window.addEventListener('dragend', function() {
    __neuraInternalDrag = false;
    __clearEdgeTimer();
    post({cmd:'sidebar_auto_close'});
  }, false);

  const fsChange = function() {
    post({cmd: 'content_fullscreen_change', active: !!document.fullscreenElement || !!document.webkitFullscreenElement});
  };
  const fsNames = ['requestFullscreen', 'webkitRequestFullscreen', 'webkitRequestFullScreen', 'msRequestFullscreen'];
  const fsName = fsNames.find(name => typeof Element.prototype[name] === 'function');
  const reqFs = fsName ? Element.prototype[fsName] : null;
  if (reqFs && !Element.prototype.__neuraFs) {
    Element.prototype.__neuraFs = true;
    const reqWrap = function() {
      post({cmd: 'content_fullscreen_change', active: true});
      const p = reqFs.apply(this, arguments);
      return p;
    };
    fsNames.forEach(name => {
      if (typeof Element.prototype[name] === 'function') Element.prototype[name] = reqWrap;
    });
  }
  document.addEventListener('keydown', function(e) {
    if (e.key === 'F11') {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'toggle_fullscreen'});
      return;
    }
    if (e.key === 'Escape' && window.__neuraContentFullscreen) {
      e.preventDefault();
      e.stopPropagation();
      if (document.fullscreenElement && document.exitFullscreen) {
        try { document.exitFullscreen().catch(function() {}); } catch(_) {}
      }
      post({cmd:'content_fullscreen_change', active:false});
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl) return;
    if (e.key === 'Tab') {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'switch_tab_offset', delta:e.shiftKey ? -1 : 1});
      return;
    }
    const key = e.key.toLowerCase();
    if (key === 't' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'begin_spotlight'});
    } else if (key === 't' && e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'reopen_tab'});
    } else if (key === 'h' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'open_history_panel'});
    } else if (key === 'j' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'open_downloads_panel'});
    }
  }, true);
  document.addEventListener('wheel', function(e) {
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    e.stopPropagation();
    const delta = e.deltaY < 0 ? 0.1 : -0.1;
    post({cmd:'zoom_delta', delta});
  }, {capture:true, passive:false});

  // Signal Rust when cursor enters the content area so the auto-hide sidebar can close.
  // Chrome's SetWindowRgn clip means WM_MOUSELEAVE never fires when cursor moves from
  // sidebar (inside clip) to content area (outside clip but inside window rectangle).
  // This IPC is the only reliable signal source for that transition.
  // Also tracks cursor proximity to the window edge so resize handles work.
  const RESIZE_ZONE = 6; // px from edge to show resize cursor
  let __resizeEdge = null;
  let __sbThrottle = false;
  document.addEventListener('mousemove', function(e) {
    // Sidebar auto-close throttle
    if (!__sbThrottle) {
      __sbThrottle = true;
      try { window.ipc.postMessage('{"cmd":"sidebar_auto_close"}'); } catch(_) {}
      setTimeout(function() { __sbThrottle = false; }, 300);
    }
    // Window-edge resize cursor detection
    var W = document.documentElement.clientWidth;
    var H = document.documentElement.clientHeight;
    var x = e.clientX, y = e.clientY;
    var onR = x >= W - RESIZE_ZONE;
    var onB = y >= H - RESIZE_ZONE;
    // __vLeftEdge is the content WebView's left offset in window coords, injected by Rust
    // via evaluate_script on every layout change + page load. When a sidebar is visible
    // content_x > 0, so the content's left edge is NOT the window's left edge — adding
    // the offset makes onL false for all practical cursor positions, suppressing the
    // spurious ew-resize cursor that otherwise appears at the sidebar's right border.
    var __wle = (typeof window.__vLeftEdge === 'number') ? window.__vLeftEdge : 0;
    var onL = (x + __wle) <= RESIZE_ZONE;
    var edge = null;
    var cur  = '';
    if (onR && onB) { edge = 'bottomright'; cur = 'nwse-resize'; }
    else if (onL && onB) { edge = 'bottomleft'; cur = 'nesw-resize'; }
    else if (onR)  { edge = 'right';  cur = 'ew-resize'; }
    else if (onB)  { edge = 'bottom'; cur = 's-resize'; }
    else if (onL)  { edge = 'left';   cur = 'ew-resize'; }
    if (edge !== __resizeEdge) {
      __resizeEdge = edge;
      document.documentElement.style.cursor = cur;
    }
  }, {passive: true, capture: true});

  document.addEventListener('mousedown', function(e) {
    if (__resizeEdge && e.button === 0) {
      try { post({cmd: 'begin_resize', edge: __resizeEdge}); } catch(_) {}
      // Don't preventDefault — let the page also handle it normally
    }
    // A press inside the live page dismisses chrome popovers that are clipped to their
    // own rect (e.g. the download panel), giving them click-outside-to-close behaviour
    // without the chrome having to cover — and block — the whole page.
    try { post({cmd: 'content_pointer_down'}); } catch(_) {}
  }, {capture: true});

  // Context menu: intercept right-clicks and relay target info to Rust so the
  // chrome overlay can render a custom browser context menu.
  document.addEventListener('contextmenu', function(e) {
    e.preventDefault();
    e.stopPropagation();
    const target = e.target;
    const linkEl = target.closest ? target.closest('a[href]') : null;
    const linkUrl = linkEl ? (linkEl.href || '') : '';
    let imageSrc = '';
    if (target.tagName === 'IMG') {
      imageSrc = target.src || target.currentSrc || '';
    } else if (target.tagName === 'VIDEO' || target.tagName === 'AUDIO') {
      imageSrc = target.src || target.currentSrc || '';
    }
    const sel = window.getSelection ? window.getSelection().toString().trim() : '';
    post({
      cmd: 'context_menu',
      x: e.clientX,
      y: e.clientY,
      link_url: linkUrl,
      image_src: imageSrc,
      selected_text: sel.length > 300 ? sel.slice(0, 300) : sel,
      page_url: location.href,
      can_back: history.length > 1
    });
  }, true);

  // Relay native fullscreen changes (e.g. YouTube player fullscreen button)
  // so Rust can resize the content WebView to fill the entire window.
  document.addEventListener('fullscreenchange', fsChange);
  document.addEventListener('webkitfullscreenchange', fsChange);

  // Audio/video playback detection — reports tab_audio_state to Rust via IPC so the
  // sidebar can show an animated speaker indicator and allow mute from the tab list.
  if (isTop) {
    let __audioPlaying = false;
    let __mediaActive = false;
    const __checkAudio = function() {
      const all = Array.from(document.querySelectorAll('audio,video'));
      const audible = all.some(function(m) { return !m.paused && !m.muted && !m.ended && m.readyState > 2; });
      const watching = all.some(function(m) {
        if (m.tagName !== 'VIDEO' || m.paused || m.ended || m.readyState < 3) return false;
        const r = m.getBoundingClientRect();
        return (r.width * r.height) >= 30000;
      });
      const active = audible || watching;
      if (audible !== __audioPlaying || active !== __mediaActive) {
        __audioPlaying = audible;
        __mediaActive = active;
        post({cmd:'tab_audio_state', playing: audible, active: active});
      }
    };
    document.addEventListener('play',         __checkAudio, true);
    document.addEventListener('pause',        __checkAudio, true);
    document.addEventListener('ended',        __checkAudio, true);
    document.addEventListener('volumechange', __checkAudio, true);
    setInterval(__checkAudio, 3000);
    window.__muteTab = function(muted) {
      document.querySelectorAll('audio,video').forEach(function(m) { m.muted = muted; });
    };
  }
})();
(() => {
  try { if (window.top !== window) return; } catch (_) { return; }
  if (window.__ventusPwd) return;
  const post = (o) => { try { window.ipc && window.ipc.postMessage(JSON.stringify(o)); } catch (_) {} };
  const vis = (el) => {
    if (!el) return false;
    const s = getComputedStyle(el);
    if (s.display === 'none' || s.visibility === 'hidden') return false;
    const r = el.getBoundingClientRect();
    return r.width > 4 && r.height > 4;
  };
  const pwFields = () => Array.prototype.slice.call(document.querySelectorAll('input[type=password]')).filter(vis);
  const userFor = (pw) => {
    const scope = pw.form || document;
    const inputs = Array.prototype.slice.call(scope.querySelectorAll('input'));
    const pi = inputs.indexOf(pw);
    for (let i = pi - 1; i >= 0; i--) {
      const el = inputs[i];
      const t = (el.type || 'text').toLowerCase();
      if ((t === 'text' || t === 'email' || t === 'tel') && vis(el)) return el;
    }
    const g = scope.querySelector('input[autocomplete="username"], input[type="email"], input[name*="user" i], input[name*="email" i], input[id*="user" i], input[id*="email" i]');
    return (g && vis(g)) ? g : null;
  };
  const setVal = (el, val) => {
    if (!el) return;
    try {
      const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, val);
    } catch (_) { el.value = val; }
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  };
  window.__ventusPwd = {
    fill: (username, password) => {
      const pws = pwFields();
      if (!pws.length) return;
      const pw = pws[0];
      const u = userFor(pw);
      if (u && username) setVal(u, username);
      if (password) setVal(pw, password);
    }
  };
  let asked = '';
  const ask = () => {
    if (!pwFields().length) return;
    if (asked === location.origin) return;
    asked = location.origin;
    post({ cmd: 'pwd_fill_request', origin: location.origin });
  };
  const capture = () => {
    const pws = pwFields();
    if (!pws.length) return;
    const pw = pws.filter((p) => p.value)[0] || pws[0];
    if (!pw.value) return;
    const u = userFor(pw);
    post({ cmd: 'pwd_capture', origin: location.origin, username: u ? u.value : '', password: pw.value });
  };
  document.addEventListener('submit', capture, true);
  document.addEventListener('click', (e) => {
    const t = e.target;
    const b = t && t.closest && t.closest('button, input[type=submit], input[type=button], [role=button]');
    if (b) setTimeout(capture, 60);
  }, true);
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.target && e.target.tagName === 'INPUT') setTimeout(capture, 60);
  }, true);
  const start = () => ask();
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
  let n = 0;
  const iv = setInterval(() => { n++; ask(); if (n > 25) clearInterval(iv); }, 600);
})();
"#;
    format!("{identity_prefix}{ad_prefix}{privacy_prefix}{script}")
}

fn browser_identity_script() -> String {
    let (full, _, major) = chromium_versions();
    let full = serde_json::to_string(&full).unwrap_or_else(|_| "\"0.0.0.0\"".to_string());
    let major = serde_json::to_string(&major).unwrap_or_else(|_| "\"0\"".to_string());
    format!(
        r#"
(() => {{
  if (window.__ventusIdentity) return;
  window.__ventusIdentity = true;
  const major = {major};
  const fullVersion = {full};
  const low = () => [
    {{brand:'Ventus', version:major}},
    {{brand:'Chromium', version:major}},
    {{brand:'Not:A-Brand', version:'24'}}
  ];
  const high = () => [
    {{brand:'Ventus', version:fullVersion}},
    {{brand:'Chromium', version:fullVersion}},
    {{brand:'Not:A-Brand', version:'24.0.0.0'}}
  ];
  const data = {{}};
  try {{
    Object.defineProperties(data, {{
      brands: {{get: low}},
      mobile: {{get: () => false}},
      platform: {{get: () => 'Windows'}},
      getHighEntropyValues: {{value: async hints => {{
        const out = {{brands: low(), mobile: false, platform: 'Windows'}};
        for (const hint of hints || []) {{
          if (hint === 'architecture') out.architecture = 'x86';
          if (hint === 'bitness') out.bitness = '64';
          if (hint === 'fullVersionList') out.fullVersionList = high();
          if (hint === 'model') out.model = '';
          if (hint === 'platformVersion') out.platformVersion = '10.0.0';
          if (hint === 'uaFullVersion') out.uaFullVersion = fullVersion;
          if (hint === 'wow64') out.wow64 = false;
        }}
        return out;
      }}}},
      toJSON: {{value: () => ({{brands: low(), mobile: false, platform: 'Windows'}})}}
    }});
    Object.defineProperty(Navigator.prototype, 'userAgentData', {{get: () => data, configurable: true}});
  }} catch (_) {{}}
}})();
"#
    )
}

fn privacy_initialization_script(
    fingerprint: bool,
    strict: bool,
    site_permissions: &config::SitePermissionMap,
    default_permissions: &config::SitePermissions,
) -> String {
    let fingerprint_script = if fingerprint {
        r#"
(() => {
  if (window.__neuraPrivacyFp) return;
  window.__neuraPrivacyFp = true;
  const noise = data => {
    if (!data || !data.length) return;
    const step = Math.max(4, Math.floor(data.length / 64));
    for (let i = 0; i < data.length; i += step) {
      const n = ((Math.random() * 3) | 0) - 1;
      data[i] = (data[i] + n + 256) & 255;
      if (i + 1 < data.length) data[i + 1] = (data[i + 1] - n + 256) & 255;
    }
  };
  const noiseImage = img => {
    try { if (img && img.data) noise(img.data); } catch (_) {}
    return img;
  };
  const cloneCanvas = canvas => {
    const c = document.createElement('canvas');
    c.width = canvas.width;
    c.height = canvas.height;
    const ctx = c.getContext('2d', {willReadFrequently:true});
    if (!ctx) return null;
    ctx.drawImage(canvas, 0, 0);
    try {
      const img = ctx.getImageData(0, 0, c.width, c.height);
      noiseImage(img);
      ctx.putImageData(img, 0, 0);
    } catch (_) {
      return null;
    }
    return c;
  };
  const patch = (obj, name, fn) => {
    try {
      const orig = obj && obj[name];
      if (typeof orig !== 'function' || orig.__neuraPatched) return;
      const wrapped = fn(orig);
      wrapped.__neuraPatched = true;
      Object.defineProperty(obj, name, {value: wrapped, configurable:true, writable:true});
    } catch (_) {}
  };
  patch(window.CanvasRenderingContext2D && window.CanvasRenderingContext2D.prototype, 'getImageData', orig => function() {
    return noiseImage(orig.apply(this, arguments));
  });
  patch(window.HTMLCanvasElement && window.HTMLCanvasElement.prototype, 'toDataURL', orig => function() {
    const c = cloneCanvas(this);
    return orig.apply(c || this, arguments);
  });
  patch(window.HTMLCanvasElement && window.HTMLCanvasElement.prototype, 'toBlob', orig => function() {
    const c = cloneCanvas(this);
    return orig.apply(c || this, arguments);
  });
  const patchGl = proto => {
    if (!proto) return;
    patch(proto, 'readPixels', orig => function() {
      const result = orig.apply(this, arguments);
      const pixels = arguments[6];
      if (pixels && typeof pixels.length === 'number') noise(pixels);
      return result;
    });
    patch(proto, 'getParameter', orig => function(p) {
      if (p === 37445) return 'Ventus GPU';
      if (p === 37446) return 'Ventus Renderer';
      return orig.apply(this, arguments);
    });
  };
  patchGl(window.WebGLRenderingContext && WebGLRenderingContext.prototype);
  patchGl(window.WebGL2RenderingContext && WebGL2RenderingContext.prototype);
})();
"#
    } else {
        ""
    };
    let site_permissions_json =
        serde_json::to_string(site_permissions).unwrap_or_else(|_| "{}".to_string());
    let default_permissions_json =
        serde_json::to_string(default_permissions).unwrap_or_else(|_| "{}".to_string());
    let has_default = default_permissions_json != "{}";
    let strict_script = if strict || !site_permissions.is_empty() || has_default {
        r#"
(() => {
  if (window.__neuraPrivacyPerms) return;
  window.__neuraPrivacyPerms = true;
  const sitePermissions = __SITE_PERMISSIONS__;
  const defaultPermissions = __DEFAULT_PERMISSIONS__;
  const strictDefault = __STRICT__;
  const rules = (() => {
    try { return sitePermissions[location.origin] || {}; } catch (_) { return {}; }
  })();
  const decisive = v => (v === 'allow' || v === 'block') ? v : null;
  const askByDefault = key => key === 'microphone' || key === 'camera' || key === 'notifications';
  const action = key => decisive(rules[key]) || decisive(defaultPermissions[key]) || ((strictDefault && !askByDefault(key)) ? 'block' : 'ask');
  const isBlocked = key => action(key) === 'block';
  const nativeMask = (fn, name) => {
    try { if (name) Object.defineProperty(fn, 'name', {value: name, configurable: true}); } catch (_) {}
    try {
      const s = 'function ' + (name || fn.name || '') + '() { [native code] }';
      Object.defineProperty(fn, 'toString', {value: () => s, configurable: true, writable: true});
    } catch (_) {}
    return fn;
  };
  const blk = name => nativeMask(function () { return Promise.reject(new DOMException('Blocked by Ventus strict permissions', 'NotAllowedError')); }, name);
  try {
    if (navigator.geolocation && isBlocked('geolocation')) {
      navigator.geolocation.getCurrentPosition = nativeMask(function(_, err) {
        if (typeof err === 'function') setTimeout(() => err({code:1, message:'Blocked by Ventus strict permissions'}), 0);
      }, 'getCurrentPosition');
      navigator.geolocation.watchPosition = nativeMask(function(_, err) {
        if (typeof err === 'function') setTimeout(() => err({code:1, message:'Blocked by Ventus strict permissions'}), 0);
        return 0;
      }, 'watchPosition');
      navigator.geolocation.clearWatch = nativeMask(function() {}, 'clearWatch');
    }
  } catch (_) {}
  try {
    if (navigator.clipboard && isBlocked('clipboard')) {
      navigator.clipboard.read = blk('read');
      navigator.clipboard.readText = blk('readText');
    }
  } catch (_) {}
  try {
    if (window.queryLocalFonts && isBlocked('local_fonts')) window.queryLocalFonts = blk('queryLocalFonts');
  } catch (_) {}
  try {
    if (navigator.requestMIDIAccess && isBlocked('midi')) navigator.requestMIDIAccess = blk('requestMIDIAccess');
  } catch (_) {}
  try {
    if (window.getScreenDetails && isBlocked('window_management')) window.getScreenDetails = blk('getScreenDetails');
  } catch (_) {}
  try {
    if (window.showOpenFilePicker && isBlocked('file_system')) window.showOpenFilePicker = blk('showOpenFilePicker');
    if (window.showSaveFilePicker && isBlocked('file_system')) window.showSaveFilePicker = blk('showSaveFilePicker');
    if (window.showDirectoryPicker && isBlocked('file_system')) window.showDirectoryPicker = blk('showDirectoryPicker');
  } catch (_) {}
  try {
    if (window.Notification && Notification.requestPermission && isBlocked('notifications')) {
      Notification.requestPermission = nativeMask(function(cb) {
        if (typeof cb === 'function') setTimeout(() => cb('denied'), 0);
        return Promise.resolve('denied');
      }, 'requestPermission');
    }
  } catch (_) {}
})();
"#
        .replace("__SITE_PERMISSIONS__", &site_permissions_json)
        .replace("__DEFAULT_PERMISSIONS__", &default_permissions_json)
        .replace("__STRICT__", if strict { "true" } else { "false" })
    } else {
        String::new()
    };
    format!("{fingerprint_script}{strict_script}")
}

fn slim_feed_articles(json: &serde_json::Value) -> serde_json::Value {
    let Some(items) = json.get("articles").and_then(|v| v.as_array()) else {
        return serde_json::json!([]);
    };
    serde_json::Value::Array(
        items
            .iter()
            .map(|a| {
                serde_json::json!({
                    "title": a.get("title").cloned().unwrap_or_default(),
                    "summary": a.get("summary").cloned().unwrap_or_default(),
                    "whyItMatters": a.get("whyItMatters").cloned().unwrap_or_default(),
                    "coverImage": a.get("coverImage").cloned().unwrap_or_default(),
                    "imageSource": a.get("imageSource").cloned().unwrap_or_default(),
                    "imageSourceUrl": a.get("imageSourceUrl").cloned().unwrap_or_default(),
                    "sources": a.get("sources").cloned().unwrap_or_default(),
                    "createdAt": a.get("createdAt").cloned().unwrap_or_default(),
                })
            })
            .collect(),
    )
}

fn refresh_trends(
    state: &mut AppState,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    let region = browser::trends::clean_region(&state.settings.region);
    let now = chrono::Utc::now().timestamp_millis();
    if state.trends_loading {
        return;
    }
    if state.trends_region == region
        && now - state.trends_fetched_at < 600_000
        && !state.trends.is_empty()
    {
        return;
    }
    state.trends_loading = true;
    let proxy = proxy.clone();
    rt.spawn(async move {
        match browser::trends::fetch(&region).await {
            Ok(trends) => {
                let _ = proxy.send_event(AppEvent::TrendsLoaded {
                    region,
                    trends,
                    fetched_at: chrono::Utc::now().timestamp_millis(),
                });
            }
            Err(_) => {
                let _ = proxy.send_event(AppEvent::TrendsFailed { region });
            }
        }
    });
}

#[derive(Clone, Copy)]
struct LayoutConfig {
    sidebar_expanded_w: u32,
    sidebar_collapsed_w: u32,
    toolbar_h: u32,
    ai_sidebar_w: u32,
    min_content_w: u32,
    min_ai_sidebar_w: u32,
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

        let is_auto_hide = matches!(
            state.settings.appearance.sidebar_mode,
            crate::config::SidebarMode::AutoHide
        );
        let is_compact = matches!(
            state.settings.appearance.sidebar_mode,
            crate::config::SidebarMode::Compact
        );
        let min_content_w = config.min_content_w as f64;
        let sidebar_css_w = if is_auto_hide {
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

        let bm_bar_extra = if state.settings.appearance.show_bookmarks_bar {
            30u32
        } else {
            0u32
        };
        let toolbar_css_h = ((config.toolbar_h + bm_bar_extra) as f64).min(logical_h.max(1.0));
        let sidebar_w = logical_to_physical(sidebar_css_w, scale);

        const FRAME_LOGICAL: f64 = 5.0;
        let frame_side = logical_to_physical(FRAME_LOGICAL, scale);
        let frame_bottom = logical_to_physical(FRAME_LOGICAL, scale);

        let clip_sidebar_w = if is_auto_hide && !state.sidebar_pinned {
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

        let content_offset = if is_auto_hide && !state.sidebar_pinned {
            frame_side
        } else {
            sidebar_w + frame_side
        };

        let content_x = content_offset as i32;
        let content_w = size
            .width
            .saturating_sub(ai_w)
            .saturating_sub(content_offset)
            .saturating_sub(frame_side) // right frame strip
            .max(1);
        let content_h = size
            .height
            .saturating_sub(toolbar_h)
            .saturating_sub(frame_bottom)
            .max(1);

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
    !state.ad_block_engine.is_site_excepted(&tab.url)
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
    let have = browser::cookie_manager::snapshot(wv, Duration::from_millis(1200));
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
    let Ok(conn) = cookie_store::open(data_dir) else {
        return;
    };
    for (tid, wv) in content_views {
        if state.tab_manager.tab_is_incognito(tid) {
            continue;
        }
        let cookies = browser::cookie_manager::snapshot(wv, Duration::from_millis(900));
        if cookies.is_empty() {
            continue;
        }
        let _ = cookie_store::save(&conn, &cookies);
    }
    let _ = cookie_store::purge_expired(&conn);
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
        SC_ZOOM_RESET => Some(ChromeCommand::ZoomSet { level: 1.0 }),
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
        "window.__neura&&window.__neura.setLayout({:.3},{:.3},{:.3},{:.3},{:.3})",
        layout.sidebar_css_w,
        layout.toolbar_css_h,
        layout.ai_css_w,
        layout.frame_side_css,
        layout.frame_bottom_css
    ));

    #[cfg(windows)]
    if let Some(hwnd) = chrome_hwnd {
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
            state
                .suggestion_overlay_rect
                .map(|rect| rect.to_physical(layout.scale_factor)),
            layout.frame_side_w,
            layout.frame_bottom_h,
        );
    }
}

fn sync_chrome_clip(chrome_hwnd: Option<isize>, state: &AppState, layout: AppLayout) {
    #[cfg(windows)]
    if let Some(hwnd) = chrome_hwnd {
        set_chrome_clip_region(
            hwnd,
            layout.window_w,
            layout.window_h,
            layout.clip_sidebar_w,
            layout.toolbar_h,
            layout.ai_w,
            state.ai_sidebar_open,
            chrome_owns_content(state),
            state
                .suggestion_overlay_rect
                .map(|rect| rect.to_physical(layout.scale_factor)),
            layout.frame_side_w,
            layout.frame_bottom_h,
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
    chrome_owns_content(state) || state.suggestion_overlay_rect.is_some() || state.ai_sidebar_open
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
    floating_rect: Option<PhysicalClipRect>,
    frame_side_w: u32,
    frame_bottom_h: u32,
) {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_OR},
    };

    #[derive(Clone, Copy)]
    struct ClipSpec {
        window_w: u32,
        window_h: u32,
        sidebar_w: u32,
        toolbar_h: u32,
        ai_sidebar_w: u32,
        ai_open: bool,
        overlay_open: bool,
        floating_rect: Option<PhysicalClipRect>,
        frame_side_w: u32,
        frame_bottom_h: u32,
    }

    unsafe fn create_region(spec: ClipSpec) -> windows::Win32::Graphics::Gdi::HRGN {
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

        if let Some(rect) = spec.floating_rect {
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
            let right_r = spec.window_w.saturating_sub(spec.ai_sidebar_w) as i32;
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
            floating_rect,
            frame_side_w,
            frame_bottom_h,
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

fn save_session(state: &AppState) {
    let active_id = state.tab_manager.active_tab_id.as_deref();
    let result = repositories::save_session(
        &state.conn,
        &state.tab_manager.workspaces,
        &state.tab_manager.active_workspace_id,
        &state.tab_manager.tabs,
        active_id,
    );
    match result {
        Ok(()) => tracing::debug!(
            target: "ventus::session",
            tab_count = state.tab_manager.tabs.len(),
            active_tab = ?active_id,
            "[SESSION] saved session snapshot"
        ),
        Err(e) => tracing::warn!(
            target: "ventus::session",
            error = %e,
            "[SESSION] save_session FAILED — last session may not restore correctly"
        ),
    }
}

fn queue_session_save(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    id: &mut u64,
) {
    *id = id.wrapping_add(1).max(1);
    let next = *id;
    let proxy = proxy.clone();
    rt.spawn(async move {
        tokio::time::sleep(SESSION_SAVE_DELAY).await;
        let _ = proxy.send_event(AppEvent::SaveSession { id: next });
    });
}

fn cloud_sync_kinds(event: &AppEvent) -> Option<(bool, bool, bool)> {
    match event {
        AppEvent::SyncPulled { .. } => Some((true, true, true)),
        AppEvent::ContentMetadata { .. } => Some((false, true, false)),
        AppEvent::Chrome(cmd) => match cmd {
            ChromeCommand::BookmarkAdd
            | ChromeCommand::BookmarkAddUrl { .. }
            | ChromeCommand::MoveBookmark { .. }
            | ChromeCommand::BookmarkRemove { .. }
            | ChromeCommand::BookmarkRemoveById { .. }
            | ChromeCommand::BookmarkCreateFolder { .. }
            | ChromeCommand::BookmarkMoveToFolder { .. }
            | ChromeCommand::BookmarkRemoveFromFolder { .. }
            | ChromeCommand::BookmarkFolderRename { .. }
            | ChromeCommand::BookmarkFolderDelete { .. } => Some((true, false, false)),
            ChromeCommand::HistoryClear | ChromeCommand::DeleteHistoryEntry { .. } => {
                Some((false, true, false))
            }
            ChromeCommand::SaveSettings { .. } => Some((false, false, true)),
            _ => None,
        },
        _ => None,
    }
}

fn queue_cloud_sync(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    id: &mut u64,
) {
    *id = id.wrapping_add(1).max(1);
    let next = *id;
    let proxy = proxy.clone();
    rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let _ = proxy.send_event(AppEvent::SyncPush { id: next });
    });
}

fn cloud_bookmarks_blob(state: &AppState) -> String {
    let bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
    let folders = repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
    serde_json::json!({ "bookmarks": bookmarks, "folders": folders }).to_string()
}

fn cloud_history_blob(state: &AppState) -> String {
    let history = repositories::list_history(&state.conn, 150).unwrap_or_default();
    serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string())
}

fn should_save_session(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::ContentNav { .. }
            | AppEvent::ContentMetadata { .. }
            | AppEvent::Chrome(ChromeCommand::Navigate { .. })
            | AppEvent::Chrome(ChromeCommand::NavigateFromOverlay { .. })
            | AppEvent::Chrome(ChromeCommand::Back)
            | AppEvent::Chrome(ChromeCommand::Forward)
            | AppEvent::Chrome(ChromeCommand::NewTab)
            | AppEvent::Chrome(ChromeCommand::CloseTab { .. })
            | AppEvent::Chrome(ChromeCommand::SwitchTab { .. })
            | AppEvent::Chrome(ChromeCommand::PinTab { .. })
            | AppEvent::Chrome(ChromeCommand::UnpinTab { .. })
            | AppEvent::Chrome(ChromeCommand::NewWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::RenameWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::DeleteWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::SwitchWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::ReopenTab)
            | AppEvent::Chrome(ChromeCommand::OpenInNewTab { .. })
    )
}

fn notification_site(origin: &str, fallback: &str) -> String {
    let site = notification_site_label(origin);
    if !site.is_empty() {
        return site;
    }
    notification_site_label(fallback)
}

fn notification_site_label(raw: &str) -> String {
    let Ok(url) = url::Url::parse(raw) else {
        return String::new();
    };
    let Some(host) = url.host_str() else {
        return String::new();
    };
    site_case(&site_stem(host))
}

fn site_stem(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').filter(|p| !p.is_empty()).collect();
    if labels.is_empty() {
        return String::new();
    }
    if labels.len() == 1 {
        return labels[0].to_string();
    }
    let mut idx = labels.len().saturating_sub(2);
    let slds = ["ac", "co", "com", "edu", "gov", "net", "org"];
    if labels.last().map(|p| p.len() == 2).unwrap_or(false)
        && slds.contains(&labels[idx])
        && idx > 0
    {
        idx -= 1;
    }
    labels[idx].to_string()
}

fn site_case(stem: &str) -> String {
    let lower = stem.to_ascii_lowercase();
    match lower.as_str() {
        "whatsapp" => "WhatsApp".to_string(),
        "youtube" => "YouTube".to_string(),
        "chatgpt" => "ChatGPT".to_string(),
        "github" => "GitHub".to_string(),
        "gmail" => "Gmail".to_string(),
        "google" => "Google".to_string(),
        "linkedin" => "LinkedIn".to_string(),
        _ => lower
            .split(['-', '_'])
            .filter(|p| !p.is_empty())
            .map(title_word)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn title_word(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}

fn notification_icons(icon: &str, origin: &str, favicon: Option<&str>) -> Vec<String> {
    let mut icons = Vec::new();
    push_icon(&mut icons, notification_icon_url(icon, origin));
    push_icon(
        &mut icons,
        favicon.and_then(|v| notification_icon_url(v, origin)),
    );
    push_icon(&mut icons, origin_favicon(origin));
    push_icon(&mut icons, google_favicon(origin));
    icons
}

fn push_icon(icons: &mut Vec<String>, icon: Option<String>) {
    let Some(icon) = icon else {
        return;
    };
    if icons.iter().any(|i| i == &icon) {
        return;
    }
    icons.push(icon);
}

fn notification_icon_url(raw: &str, base: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with("data:") || raw.starts_with("blob:") {
        return None;
    }
    let url = url::Url::parse(raw)
        .or_else(|_| url::Url::parse(base).and_then(|base| base.join(raw)))
        .ok()?;
    if !matches!(url.scheme(), "http" | "https" | "file") {
        return None;
    }
    Some(url.to_string())
}

fn origin_favicon(origin: &str) -> Option<String> {
    let mut url = url::Url::parse(origin).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_path("/favicon.ico");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

fn google_favicon(origin: &str) -> Option<String> {
    let url = url::Url::parse(origin).ok()?;
    let host = url.host_str()?;
    Some(format!(
        "https://www.google.com/s2/favicons?domain={}&sz=64",
        host
    ))
}

async fn cache_notification_icon(icons: Vec<String>) -> String {
    for icon in icons {
        if let Some(path) = cache_notification_icon_url(&icon).await {
            return path;
        }
    }
    String::new()
}

async fn cache_notification_icon_url(icon: &str) -> Option<String> {
    let url = url::Url::parse(icon).ok()?;
    if url.scheme() == "file" {
        return Some(icon.to_string());
    }
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let path = notification_icon_path(icon);
    if path.is_file() {
        return notification_icon_file_url(&path);
    }
    let bytes = download_notification_icon(icon).await?;
    write_notification_icon(&path, &bytes)
}

async fn download_notification_icon(icon: &str) -> Option<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent(browser_user_agent())
        .build()
        .ok()?;
    let resp = client.get(icon).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > 1_500_000 {
        return None;
    }
    Some(bytes.to_vec())
}

fn write_notification_icon(path: &std::path::Path, bytes: &[u8]) -> Option<String> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = img.resize(64, 64, image::imageops::FilterType::Lanczos3);
    let dir = path.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .ok()?;
    std::fs::write(path, buf).ok()?;
    notification_icon_file_url(path)
}

fn notification_icon_path(icon: &str) -> std::path::PathBuf {
    utils::platform::data_dir()
        .join("notification-icons")
        .join(format!("{}.png", notification_icon_hash(icon)))
}

fn notification_icon_file_url(path: &std::path::Path) -> Option<String> {
    url::Url::from_file_path(path)
        .ok()
        .map(|url| url.to_string())
}

fn notification_icon_hash(icon: &str) -> u64 {
    let mut h = DefaultHasher::new();
    icon.hash(&mut h);
    h.finish()
}

fn repeated_native_load_start(state: &AppState, event: &AppEvent) -> bool {
    let AppEvent::ContentLoadStart {
        tab_id,
        url,
        native,
        ..
    } = event
    else {
        return false;
    };
    if !*native {
        return false;
    }
    let clean_url = crate::utils::url::clean_tracking_url(url);
    if clean_url.trim().is_empty()
        || clean_url == "about:blank"
        || clean_url.starts_with("neura://")
    {
        return false;
    }
    let Some(tab) = state.tab_manager.get_tab(tab_id) else {
        return false;
    };
    if tab.status != crate::browser::tab::TabStatus::Loading {
        return false;
    }
    let current_match = app::same_nav(&tab.url, &clean_url);
    let pending_match = state
        .pending_nav_urls
        .get(tab_id)
        .map(|expected| app::same_nav(expected, &clean_url))
        .unwrap_or(false);
    let native_match = state
        .native_loads
        .get(tab_id)
        .map(|expected| app::same_nav(expected, &clean_url))
        .unwrap_or(false);
    current_match || pending_match || native_match
}

fn stale_canceled_nav_failure(state: &AppState, event: &AppEvent) -> bool {
    let AppEvent::ContentNavigationFailed {
        tab_id,
        status,
        nav_id,
        ..
    } = event
    else {
        return false;
    };
    if !canceled_web_error_status(*status) {
        return false;
    }
    state
        .native_nav_ids
        .get(tab_id)
        .map(|id| *nav_id == 0 || *id != *nav_id)
        .unwrap_or(false)
}

fn save_window_size(window: &tao::window::Window, state: &mut AppState, maxed: bool) {
    if state.content_fullscreen || maxed {
        return;
    }
    let size = window.inner_size();
    let scale = window.scale_factor().max(1.0);
    state.settings.window_width = (size.width as f64 / scale).round() as u32;
    state.settings.window_height = (size.height as f64 / scale).round() as u32;
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
}

// ── Agent context snapshot ─────────────────────────────────────────────────────

/// Snapshot of browser state passed to the async agent loop.
struct AgentSnapshot {
    tab_id: String,
    page_url: String,
    page_title: String,
    tabs: Vec<serde_json::Value>,
    history_items: Vec<serde_json::Value>,
}

fn build_agent_snapshot(state: &AppState) -> AgentSnapshot {
    let active = state.tab_manager.active_tab();
    let tab_id = state.tab_manager.active_tab_id.clone().unwrap_or_default();
    let page_url = active.map(|t| t.url.clone()).unwrap_or_default();
    let page_title = active.map(|t| t.title.clone()).unwrap_or_default();

    let tabs: Vec<serde_json::Value> = state
        .tab_manager
        .active_workspace_tabs()
        .iter()
        .map(|t| {
            let is_active = state.tab_manager.active_tab_id.as_deref() == Some(&t.id);
            serde_json::json!({
                "tab_id": t.id,
                "title": t.title,
                "url": t.url,
                "active": is_active,
            })
        })
        .collect();

    let history_items: Vec<serde_json::Value> =
        storage::repositories::list_history(&state.conn, 60)
            .unwrap_or_default()
            .into_iter()
            .map(|h| {
                serde_json::json!({
                    "url": h.url,
                    "title": h.title,
                })
            })
            .collect();

    AgentSnapshot {
        tab_id,
        page_url,
        page_title,
        tabs,
        history_items,
    }
}

// ── Tool execution ─────────────────────────────────────────────────────────────

/// Execute a single tool call from the AI, returning the result string.
async fn execute_tool(
    tc: &ai::provider::ToolCall,
    snapshot: &AgentSnapshot,
    pending: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
    >,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) -> String {
    use ai::browser_tools::{
        js_click_element, js_get_interactive_elements, js_get_page_text, js_scroll_page,
        js_type_text,
    };

    let args: serde_json::Value =
        serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));

    match tc.function.name.as_str() {
        // ── State tools (answered instantly from snapshot) ────────────────────
        "get_current_url" => serde_json::json!({
            "url": snapshot.page_url,
            "title": snapshot.page_title,
        })
        .to_string(),
        "list_tabs" => serde_json::to_string(&snapshot.tabs).unwrap_or_else(|_| "[]".into()),
        "search_history" => {
            let q = args["query"].as_str().unwrap_or("").to_lowercase();
            let results: Vec<_> = snapshot
                .history_items
                .iter()
                .filter(|h| {
                    let url = h["url"].as_str().unwrap_or("").to_lowercase();
                    let title = h["title"].as_str().unwrap_or("").to_lowercase();
                    url.contains(&q) || title.contains(&q)
                })
                .take(20)
                .collect();
            serde_json::to_string(&results).unwrap_or_else(|_| "[]".into())
        }

        // ── Browser action tools (fire proxy event, no result needed) ─────────
        "navigate" => {
            let url = args["url"].as_str().unwrap_or("").to_string();
            if !url.is_empty() {
                let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::Navigate {
                    url: url.clone(),
                }));
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                serde_json::json!({"success": true, "navigating_to": url}).to_string()
            } else {
                r#"{"error":"url is required"}"#.into()
            }
        }
        "go_back" => {
            let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::Back));
            r#"{"success":true}"#.into()
        }
        "go_forward" => {
            let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::Forward));
            r#"{"success":true}"#.into()
        }
        "new_tab" => {
            let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::NewTab));
            if let Some(url) = args["url"].as_str() {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::Navigate {
                    url: url.to_string(),
                }));
            }
            r#"{"success":true}"#.into()
        }
        "switch_tab" => {
            let id = args["tab_id"].as_str().unwrap_or("").to_string();
            let _ = proxy.send_event(AppEvent::Chrome(ChromeCommand::SwitchTab { id }));
            r#"{"success":true}"#.into()
        }

        // ── Page tools (JS round-trip through content WebView) ────────────────
        "get_page_text" => {
            execute_page_js(
                js_get_page_text(&tc.id),
                &tc.id,
                &snapshot.tab_id,
                pending,
                proxy,
            )
            .await
        }
        "get_page_interactive_elements" => {
            execute_page_js(
                js_get_interactive_elements(&tc.id),
                &tc.id,
                &snapshot.tab_id,
                pending,
                proxy,
            )
            .await
        }
        "click_element" => {
            let eid = args["element_id"].as_str().unwrap_or("").to_string();
            execute_page_js(
                js_click_element(&tc.id, &eid),
                &tc.id,
                &snapshot.tab_id,
                pending,
                proxy,
            )
            .await
        }
        "type_text" => {
            let eid = args["element_id"].as_str().unwrap_or("");
            let text = args["text"].as_str().unwrap_or("");
            execute_page_js(
                js_type_text(&tc.id, eid, text),
                &tc.id,
                &snapshot.tab_id,
                pending,
                proxy,
            )
            .await
        }
        "scroll_page" => {
            let dir = args["direction"].as_str().unwrap_or("down");
            let amount = args["amount"].as_i64().unwrap_or(400);
            execute_page_js(
                js_scroll_page(&tc.id, dir, amount),
                &tc.id,
                &snapshot.tab_id,
                pending,
                proxy,
            )
            .await
        }

        other => format!(r#"{{"error":"unknown tool: {other}"}}"#),
    }
}

/// Send JS to the active content WebView and wait (with timeout) for the result.
async fn execute_page_js(
    js: String,
    call_id: &str,
    tab_id: &str,
    pending: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<String>>>,
    >,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) -> String {
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    if let Ok(mut map) = pending.lock() {
        map.insert(call_id.to_string(), tx);
    }
    let _ = proxy.send_event(AppEvent::AiExecutePageJs {
        call_id: call_id.to_string(),
        tab_id: tab_id.to_string(),
        js,
    });
    match tokio::time::timeout(std::time::Duration::from_secs(12), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => r#"{"error":"channel closed"}"#.into(),
        Err(_) => {
            // Timeout — clean up the pending entry
            if let Ok(mut map) = pending.lock() {
                map.remove(call_id);
            }
            r#"{"error":"timeout waiting for page result"}"#.into()
        }
    }
}

// ── Main entry point ───────────────────────────────────────────────────────────

/// Handle a quick-action button click (Summarize, Explain, Key Points, Ask Anything).
///
/// Unlike handle_ai_message (which uses the agent loop and relies on the AI choosing to
/// call get_page_text), this function reads the page text DIRECTLY before calling the AI,
/// then sends a single prompt with the page content baked in.  No tool calls, no indirection,
/// no risk of the AI skipping the tool and returning an empty response.
fn handle_ai_quick_action(
    action: String,
    state: &AppState,
    chrome: &WebView,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
    ai_generation: &Arc<AtomicUsize>,
) {
    let Some(prov) = ai::build_provider(&state.settings) else {
        let _ = chrome.evaluate_script(
            "window.__neura&&window.__neura.showError('No AI provider configured. Add an API key in Settings \u{2192} AI Providers.')"
        );
        return;
    };

    let task_label: &str = match action.as_str() {
        "summarize" => "write a clear, concise summary of the page",
        "explain" => "explain in plain language what this page is about",
        "key_points" => "list the key points from this page as a bullet list",
        "ask_anything" => "describe what you can help the user with based on this page",
        other => other,
    };
    let task_label = task_label.to_string();

    let pending = state.ai_pending_tools.clone();
    let proxy_task = proxy.clone();
    let model = state.settings.ai.default_model.clone();
    let temperature = state.settings.ai.temperature;
    let max_tokens = state.settings.ai.max_tokens;
    let reasoning_effort = Some(state.settings.ai.reasoning_effort.clone());
    let tab_id = state.tab_manager.active_tab_id.clone().unwrap_or_default();
    let request_generation = ai_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let ai_generation_guard = Arc::clone(ai_generation);
    let provider = prov.provider_id();
    let supports_streaming = prov.supports_streaming();
    tracing::info!(
        target: "ventus::ai",
        kind = "quick_action",
        action = %action,
        provider,
        model = %model,
        tab = %tab_id,
        supports_streaming,
        "ai request started"
    );

    // Build a unique call_id for the page-text round-trip
    let call_id = format!("qa-{}", uuid_v4_simple());

    // Generate the JS that reads the page and posts the result back via IPC
    let page_js = ai::browser_tools::js_get_page_text(&call_id);

    rt.spawn(async move {
        let started = Instant::now();
        // Step 1 — read the page text directly (guaranteed, no AI indirection)
        let page_text_raw =
            execute_page_js(page_js, &call_id, &tab_id, &pending, &proxy_task).await;
        if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
            return;
        }

        // Unquote the JSON-encoded string the JS sends back
        let page_text: String =
            serde_json::from_str(&page_text_raw).unwrap_or_else(|_| page_text_raw.clone());
        tracing::info!(
            target: "ventus::ai",
            kind = "quick_action",
            action = %task_label,
            page_chars = page_text.chars().count(),
            "ai page context read"
        );

        let page_context = if page_text.trim().is_empty() || page_text.starts_with("Error") {
            "The page text could not be read (the page may not have loaded yet or may be empty)."
                .to_string()
        } else {
            format!(
                "=== PAGE TEXT ===\n{}\n=== END OF PAGE TEXT ===",
                page_text.trim()
            )
        };

        // Step 2 — single AI call with page text embedded in the system prompt
        let system = format!(
            "You are a helpful AI assistant embedded in a web browser. \
             The user has pressed a quick-action button and you must {} \
             based ONLY on the page text provided below. \
             Do NOT use your training knowledge about the topic — \
             answer solely from the page content. \
             Use markdown formatting where helpful (bold, bullet lists). \
             Keep your response focused and under 400 words.\n\n{}",
            task_label, page_context
        );

        let msgs = vec![
            ai::ChatMessage::system(system),
            ai::ChatMessage::user(format!("Please {}.", task_label)),
        ];

        let req = ai::ChatRequest {
            messages: msgs,
            model,
            temperature,
            max_tokens,
            stream: true,
            reasoning_effort,
            tools: None, // No tool calls — page text is already in the prompt
        };

        use futures_util::StreamExt;
        match prov.stream_chat(req.clone()).await {
            Ok(mut stream) => {
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                let mut accumulated = String::new();
                while let Some(chunk) = stream.next().await {
                    if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                        return;
                    }
                    match chunk {
                        Ok(text) if !text.is_empty() => {
                            accumulated.push_str(&text);
                            let _ = proxy_task.send_event(AppEvent::AiChunk { text, done: false });
                        }
                        Ok(_) => {}
                        Err(e) => {
                            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                                return;
                            }
                            tracing::warn!(
                                target: "ventus::ai",
                                kind = "quick_action",
                                error = %e,
                                elapsed_ms = started.elapsed().as_millis() as u64,
                                "ai stream failed"
                            );
                            let _ = proxy_task.send_event(AppEvent::AiError {
                                message: format!("AI stream error: {}", e),
                            });
                            return;
                        }
                    }
                }
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }

                if accumulated.trim().is_empty() {
                    tracing::warn!(
                        target: "ventus::ai",
                        kind = "quick_action",
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "ai stream empty; retrying without streaming"
                    );
                    let mut fallback_req = req;
                    fallback_req.stream = false;
                    match prov.chat(fallback_req).await {
                        Ok(resp) => {
                            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                                return;
                            }
                            if resp.content.trim().is_empty() {
                                tracing::warn!(
                                    target: "ventus::ai",
                                    kind = "quick_action",
                                    elapsed_ms = started.elapsed().as_millis() as u64,
                                    "ai fallback empty response"
                                );
                                let _ = proxy_task.send_event(AppEvent::AiError {
                                    message: "AI returned an empty response. Please try again."
                                        .into(),
                                });
                                return;
                            }
                            let chars = resp.content.chars().count();
                            let _ = proxy_task.send_event(AppEvent::AiChunk {
                                text: resp.content,
                                done: false,
                            });
                            let _ = proxy_task.send_event(AppEvent::AiChunk {
                                text: String::new(),
                                done: true,
                            });
                            tracing::info!(
                                target: "ventus::ai",
                                kind = "quick_action",
                                fallback = true,
                                chars,
                                elapsed_ms = started.elapsed().as_millis() as u64,
                                "ai request finished"
                            );
                            return;
                        }
                        Err(e) => {
                            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                                return;
                            }
                            tracing::warn!(
                                target: "ventus::ai",
                                kind = "quick_action",
                                error = %e,
                                elapsed_ms = started.elapsed().as_millis() as u64,
                                "ai empty-stream fallback failed"
                            );
                            let _ = proxy_task.send_event(AppEvent::AiError {
                                message: format!("AI error: {}", e),
                            });
                            return;
                        }
                    }
                }

                let _ = proxy_task.send_event(AppEvent::AiChunk {
                    text: String::new(),
                    done: true,
                });
                tracing::info!(
                    target: "ventus::ai",
                    kind = "quick_action",
                    chars = accumulated.chars().count(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "ai request finished"
                );
            }
            Err(stream_error) => {
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                let mut fallback_req = req;
                fallback_req.stream = false;
                match prov.chat(fallback_req).await {
                    Ok(resp) => {
                        if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                            return;
                        }
                        if resp.content.trim().is_empty() {
                            tracing::warn!(
                                target: "ventus::ai",
                                kind = "quick_action",
                                elapsed_ms = started.elapsed().as_millis() as u64,
                                "ai fallback empty response"
                            );
                            let _ = proxy_task.send_event(AppEvent::AiError {
                                message: "AI returned an empty response. Please try again.".into(),
                            });
                            return;
                        }
                        let chars = resp.content.chars().count();
                        let _ = proxy_task.send_event(AppEvent::AiChunk {
                            text: resp.content,
                            done: false,
                        });
                        let _ = proxy_task.send_event(AppEvent::AiChunk {
                            text: String::new(),
                            done: true,
                        });
                        tracing::info!(
                            target: "ventus::ai",
                            kind = "quick_action",
                            fallback = true,
                            chars,
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "ai request finished"
                        );
                    }
                    Err(e) => {
                        if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                            return;
                        }
                        tracing::warn!(
                            target: "ventus::ai",
                            kind = "quick_action",
                            error = %e,
                            stream_error = %stream_error,
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "ai fallback failed"
                        );
                        let _ = proxy_task.send_event(AppEvent::AiError {
                            message: format!("AI error: {} (stream failed: {})", e, stream_error),
                        });
                    }
                }
            }
        }
    });
}

/// Generate a simple random hex ID without pulling in the uuid crate.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", t)
}

fn open_in_system_browser(url: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

fn email_domain(email: &str) -> String {
    email
        .split('@')
        .nth(1)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn auth_email_password(
    is_sign_up: bool,
    email: String,
    password: String,
    state: &AppState,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    if !cloud::config::is_configured() {
        let _ = proxy.send_event(AppEvent::AuthError {
            message: "Cloud sign-in is not configured.".into(),
        });
        return;
    }
    let proxy = proxy.clone();
    let region = state.settings.region.clone();
    let cached = settings_store::get::<cloud::UserProfile>(&state.conn, cloud::PROFILE_CACHE_KEY)
        .ok()
        .flatten();
    rt.spawn(async move {
        let result = if is_sign_up {
            cloud::auth::sign_up(&email, &password).await
        } else {
            cloud::auth::sign_in(&email, &password).await
        };
        match result {
            Ok(session) => {
                let (session, profile) =
                    cloud::finalize_sign_in(session, None, None, region, cached).await;
                let message = if is_sign_up {
                    "Account created".to_string()
                } else {
                    "Signed in".to_string()
                };
                let pull_session = session.clone();
                let _ = proxy.send_event(AppEvent::AuthApplied {
                    session,
                    profile,
                    message,
                });
                let snap = cloud::pull_all(&pull_session).await;
                let _ = proxy.send_event(AppEvent::SyncPulled {
                    bookmarks: snap.bookmarks,
                    history: snap.history,
                    settings: snap.settings,
                });
            }
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
            }
        }
    });
}

fn auth_google(
    state: &AppState,
    chrome: &WebView,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) -> Option<u16> {
    if !cloud::config::is_configured() {
        let _ = proxy.send_event(AppEvent::AuthError {
            message: "Cloud sign-in is not configured.".into(),
        });
        return None;
    }
    let (listener, port) = match cloud::local_server::bind() {
        Ok(v) => v,
        Err(e) => {
            let _ = proxy.send_event(AppEvent::AuthError {
                message: format!("Could not start sign-in: {}", e),
            });
            return None;
        }
    };
    let html = cloud::google::auth_page_html();
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    rt.spawn(cloud::local_server::serve(listener, html, tx));
    let proxy_done = proxy.clone();
    let region = state.settings.region.clone();
    let cached = settings_store::get::<cloud::UserProfile>(&state.conn, cloud::PROFILE_CACHE_KEY)
        .ok()
        .flatten();
    rt.spawn(async move {
        let body = match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(b)) => b,
            _ => {
                let _ = proxy_done.send_event(AppEvent::AuthError {
                    message: "Google sign-in timed out.".into(),
                });
                return;
            }
        };
        match cloud::google::parse_result(&body) {
            Ok(g) => {
                let session = cloud::AuthSession {
                    uid: g.uid,
                    id_token: g.id_token,
                    refresh_token: g.refresh_token,
                    email: g.email,
                    expires_at_ms: chrono::Utc::now().timestamp_millis() + 3_600_000,
                };
                let name = (!g.display_name.is_empty()).then_some(g.display_name);
                let photo = (!g.photo_url.is_empty()).then_some(g.photo_url);
                let (session, profile) =
                    cloud::finalize_sign_in(session, name, photo, region, cached).await;
                let pull_session = session.clone();
                let _ = proxy_done.send_event(AppEvent::AuthApplied {
                    session,
                    profile,
                    message: "Signed in with Google".into(),
                });
                let snap = cloud::pull_all(&pull_session).await;
                let _ = proxy_done.send_event(AppEvent::SyncPulled {
                    bookmarks: snap.bookmarks,
                    history: snap.history,
                    settings: snap.settings,
                });
            }
            Err(e) => {
                let _ = proxy_done.send_event(AppEvent::AuthError { message: e });
            }
        }
    });
    let _ = chrome.evaluate_script(
        "window.__neura && window.__neura.authPending && window.__neura.authPending()",
    );
    Some(port)
}

#[cfg(windows)]
fn spawn_auth_window(
    elwt: &tao::event_loop::EventLoopWindowTarget<AppEvent>,
    main_window: &tao::window::Window,
    port: u16,
    web_context: &mut wry::WebContext,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    browser_args: &str,
) -> Option<(tao::window::Window, WebView)> {
    let w = 440u32;
    let h = 560u32;
    let window = WindowBuilder::new()
        .with_title("Sign in with Google")
        .with_inner_size(LogicalSize::new(w, h))
        .with_resizable(false)
        .with_visible(false)
        .build(elwt)
        .ok()?;
    center_popup(&window, main_window, w, h);
    set_window_background_dark(&window);

    let size = window.inner_size();
    let rect = Rect {
        x: 0,
        y: 0,
        width: size.width.max(1),
        height: size.height.max(1),
    };
    let wv = WebViewBuilder::new_as_child(&window)
        .with_bounds(rect)
        .with_background_color((13, 15, 19, 255))
        .with_url(&format!("http://localhost:{}/", port))
        .with_user_agent(&browser_user_agent())
        .with_browser_accelerator_keys(false)
        .with_additional_browser_args(browser_args.to_string())
        .with_web_context(web_context)
        .build()
        .ok()?;
    attach_new_window_handler(&wv, proxy, false);
    window.set_visible(true);
    Some((window, wv))
}

fn account_update_profile(
    username: String,
    full_name: String,
    birthdate: String,
    bio: String,
    state: &AppState,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    let Some(session) = state.auth.clone() else {
        return;
    };
    let mut profile = state.user_profile.clone().unwrap_or_default();
    profile.country = state.settings.region.clone();
    let proxy = proxy.clone();
    rt.spawn(async move {
        let session = match cloud::ensure_fresh(session).await {
            Ok(s) => s,
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
                return;
            }
        };
        match cloud::apply_profile_edits(&session, profile, username, full_name, birthdate, bio)
            .await
        {
            Ok(profile) => {
                let _ = proxy.send_event(AppEvent::AuthApplied {
                    session,
                    profile,
                    message: "Profile saved".into(),
                });
            }
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
            }
        }
    });
}

fn account_set_photo(
    data_uri: String,
    state: &AppState,
    chrome: &WebView,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    let Some(session) = state.auth.clone() else {
        return;
    };
    if !cloud::config::cloudinary_configured() {
        let _ = proxy.send_event(AppEvent::AuthError {
            message: "Photo uploads are not configured.".into(),
        });
        return;
    }
    let profile = state.user_profile.clone().unwrap_or_default();
    let proxy = proxy.clone();
    let _ = chrome.evaluate_script(
        "window.__neura && window.__neura.authPending && window.__neura.authPending()",
    );
    rt.spawn(async move {
        let session = match cloud::ensure_fresh(session).await {
            Ok(s) => s,
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
                return;
            }
        };
        match cloud::apply_photo(&session, profile, data_uri).await {
            Ok(profile) => {
                let _ = proxy.send_event(AppEvent::AuthApplied {
                    session,
                    profile,
                    message: "Photo updated".into(),
                });
            }
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
            }
        }
    });
}

fn account_change_password(
    current: String,
    new_password: String,
    state: &AppState,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    let Some(session) = state.auth.clone() else {
        return;
    };
    let profile = state.user_profile.clone().unwrap_or_default();
    let email = session.email.clone();
    let proxy = proxy.clone();
    rt.spawn(async move {
        if !current.is_empty() && !email.is_empty() {
            if let Err(e) = cloud::auth::sign_in(&email, &current).await {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
                return;
            }
        }
        let session = match cloud::ensure_fresh(session).await {
            Ok(s) => s,
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
                return;
            }
        };
        match cloud::auth::update_password(&session.id_token, &new_password).await {
            Ok(mut new_session) => {
                if new_session.email.is_empty() {
                    new_session.email = session.email.clone();
                }
                let _ = proxy.send_event(AppEvent::AuthApplied {
                    session: new_session,
                    profile,
                    message: "Password changed".into(),
                });
            }
            Err(e) => {
                let _ = proxy.send_event(AppEvent::AuthError {
                    message: e.to_string(),
                });
            }
        }
    });
}

fn handle_ai_message(
    text: String,
    state: &AppState,
    chrome: &WebView,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
    ai_generation: &Arc<AtomicUsize>,
) {
    let Some(prov) = ai::build_provider(&state.settings) else {
        let _ = chrome.evaluate_script(
            "window.__neura&&window.__neura.showError('No AI provider configured. Add an API key in Settings \u{2192} AI Providers.')"
        );
        return;
    };

    let snapshot = build_agent_snapshot(state);
    let pending = state.ai_pending_tools.clone();
    let proxy_agent = proxy.clone();
    let user_text = text.clone();

    // Build initial message list including conversation history
    let active = state.tab_manager.active_tab();
    let page_ctx = active
        .map(|t| format!("Current page: {} ({})", t.title, t.url))
        .unwrap_or_default();
    let system = format!("{}\n\nYou have access to browser control tools. Use them to help the user interact with the browser and web pages. Always read the page first before clicking elements.\n\n{}", ai::prompts::SYSTEM_PROMPT, page_ctx);

    let mut msgs: Vec<ai::ChatMessage> = vec![ai::ChatMessage::system(system)];
    msgs.extend(state.ai_messages.clone());
    msgs.push(ai::ChatMessage::user(text));

    let model = state.settings.ai.default_model.clone();
    let temperature = state.settings.ai.temperature;
    let max_tokens = state.settings.ai.max_tokens;
    let reasoning_effort = Some(state.settings.ai.reasoning_effort.clone());
    let request_generation = ai_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let ai_generation_guard = Arc::clone(ai_generation);
    let provider = prov.provider_id();
    let supports_streaming = prov.supports_streaming();
    let active_url = active
        .map(|t| crate::utils::url::log_url(&t.url))
        .unwrap_or_default();
    tracing::info!(
        target: "ventus::ai",
        kind = "chat",
        provider,
        model = %model,
        prompt_chars = user_text.chars().count(),
        history_messages = state.ai_messages.len(),
        active = %active_url,
        supports_streaming,
        "ai request started"
    );

    rt.spawn(async move {
        use futures_util::StreamExt;
        let started = Instant::now();

        // Agent loop — up to 12 iterations (tool call rounds)
        const MAX_STEPS: usize = 12;

        for step in 0..MAX_STEPS {
            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                return;
            }
            // ── 1. Try real streaming first ──────────────────────────────────
            let req_stream = ai::ChatRequest {
                messages: msgs.clone(),
                model: model.clone(),
                temperature,
                max_tokens,
                stream: true,
                reasoning_effort: reasoning_effort.clone(),
                tools: Some(ai::browser_tools::browser_tool_definitions()),
            };

            if let Ok(mut stream) = prov.stream_chat(req_stream).await {
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                let mut accumulated = String::new();
                let mut had_error = false;
                while let Some(chunk) = stream.next().await {
                    if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                        return;
                    }
                    match chunk {
                        Ok(ref t) if !t.is_empty() => {
                            accumulated.push_str(t);
                            let _ = proxy_agent.send_event(AppEvent::AiChunk {
                                text: t.clone(),
                                done: false,
                            });
                        }
                        Err(e) => {
                            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                                return;
                            }
                            tracing::warn!(
                                target: "ventus::ai",
                                kind = "chat",
                                step,
                                error = %e,
                                elapsed_ms = started.elapsed().as_millis() as u64,
                                "ai stream failed"
                            );
                            let _ = proxy_agent.send_event(AppEvent::AiError {
                                message: format!("AI stream error: {}", e),
                            });
                            had_error = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if had_error {
                    return;
                }
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }

                // Check if the stream produced actual final-answer text
                // (strip thinking tags to see if there's real content)
                let clean = {
                    let mut s = accumulated.clone();
                    // Remove all <thinking>...</thinking> and <think>...</think> blocks
                    loop {
                        let lower = s.to_lowercase();
                        if let Some(start) = lower.find("<think") {
                            // Find the end of this opening tag
                            if let Some(tag_end) = s[start..].find('>') {
                                let after_tag = start + tag_end + 1;
                                // Find the closing tag
                                if let Some(close_start) = lower[after_tag..].find("</think") {
                                    if let Some(close_end) = s[after_tag + close_start..].find('>')
                                    {
                                        let end = after_tag + close_start + close_end + 1;
                                        s = format!("{}{}", &s[..start], &s[end..]);
                                        continue;
                                    }
                                }
                                // Unclosed thinking tag — stream is still in thinking
                                s = s[..start].to_string();
                            }
                        }
                        break;
                    }
                    s.trim().to_string()
                };

                if !clean.is_empty() {
                    if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                        return;
                    }
                    // Real text → final answer already streamed to UI
                    let _ = proxy_agent.send_event(AppEvent::AiChunk {
                        text: String::new(),
                        done: true,
                    });
                    let _ = proxy_agent.send_event(AppEvent::AiSaveMessages {
                        user_text,
                        assistant_text: accumulated,
                    });
                    tracing::info!(
                        target: "ventus::ai",
                        kind = "chat",
                        step,
                        chars = clean.chars().count(),
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "ai request finished"
                    );
                    return;
                }
                // Stream was empty/thinking-only → model likely wants tool calls
            }

            // ── 2. Fallback to non-streaming chat() for tool calls ────────────
            let req_fb = ai::ChatRequest {
                messages: msgs.clone(),
                model: model.clone(),
                temperature,
                max_tokens,
                stream: false,
                reasoning_effort: reasoning_effort.clone(),
                tools: Some(ai::browser_tools::browser_tool_definitions()),
            };

            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                return;
            }
            let resp = match prov.chat(req_fb).await {
                Ok(r) => r,
                Err(e) => {
                    if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                        return;
                    }
                    tracing::warn!(
                        target: "ventus::ai",
                        kind = "chat",
                        step,
                        error = %e,
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "ai fallback failed"
                    );
                    let _ = proxy_agent.send_event(AppEvent::AiError {
                        message: format!("AI error: {}", e),
                    });
                    return;
                }
            };
            if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                return;
            }

            let has_tool_calls = resp
                .tool_calls
                .as_ref()
                .map(|v| !v.is_empty())
                .unwrap_or(false);

            if !has_tool_calls {
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                // Final text response from fallback (no tool calls)
                let final_text = resp.content.clone();
                if !final_text.is_empty() {
                    let _ = proxy_agent.send_event(AppEvent::AiChunk {
                        text: final_text.clone(),
                        done: false,
                    });
                }
                let _ = proxy_agent.send_event(AppEvent::AiChunk {
                    text: String::new(),
                    done: true,
                });
                let _ = proxy_agent.send_event(AppEvent::AiSaveMessages {
                    user_text,
                    assistant_text: final_text,
                });
                tracing::info!(
                    target: "ventus::ai",
                    kind = "chat",
                    step,
                    fallback = true,
                    chars = resp.content.chars().count(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "ai request finished"
                );
                return;
            }

            // Tool calls requested
            let tool_calls = resp.tool_calls.unwrap();
            tracing::info!(
                target: "ventus::ai",
                kind = "chat",
                step,
                tool_calls = tool_calls.len(),
                "ai tool calls requested"
            );

            // Record assistant's tool-call message in the conversation
            msgs.push(ai::ChatMessage::assistant_with_tool_calls(
                tool_calls.clone(),
            ));

            // Show tool call labels in the AI sidebar
            for tc in &tool_calls {
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                let label =
                    ai::browser_tools::tool_call_label(&tc.function.name, &tc.function.arguments);
                let _ = proxy_agent.send_event(AppEvent::AiToolCallDisplay { label });
            }

            // Execute all tool calls and collect results
            for tc in &tool_calls {
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                let result = execute_tool(tc, &snapshot, &pending, &proxy_agent).await;
                if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
                    return;
                }
                tracing::debug!(
                    "tool {} → {}",
                    tc.function.name,
                    &result[..result.len().min(200)]
                );
                tracing::info!(
                    target: "ventus::ai",
                    kind = "chat",
                    step,
                    tool = %tc.function.name,
                    result_chars = result.chars().count(),
                    "ai tool result"
                );
                msgs.push(ai::ChatMessage::tool_result(&tc.id, result));
            }

            // Guard against runaway loops
            if step > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        // Exceeded max steps
        if ai_generation_guard.load(Ordering::SeqCst) != request_generation {
            return;
        }
        tracing::warn!(
            target: "ventus::ai",
            kind = "chat",
            elapsed_ms = started.elapsed().as_millis() as u64,
            "ai max steps"
        );
        let _ = proxy_agent.send_event(AppEvent::AiError {
            message: "Agent reached maximum steps — please try a simpler request.".into(),
        });
    });
}

fn handle_spotlight_ai_query(
    text: String,
    history: Vec<crate::ui::events::SpotlightTurn>,
    state: &AppState,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    let Some(prov) = ai::build_provider(&state.settings) else {
        let _ = proxy.send_event(AppEvent::SpotlightAiError {
            message: "No AI provider configured. Add an API key in Settings → AI Providers."
                .to_string(),
        });
        return;
    };

    let proxy_task = proxy.clone();
    let model = state.settings.ai.default_model.clone();
    let provider_id = state.settings.ai.default_provider.clone();
    let temperature = state.settings.ai.temperature;
    let max_tokens = state.settings.ai.max_tokens;
    let reasoning_effort = Some(state.settings.ai.reasoning_effort.clone());
    let query_text = text.clone();
    tracing::info!(
        target: "ventus::ai",
        kind = "spotlight",
        provider = prov.provider_id(),
        model = %model,
        query_chars = text.chars().count(),
        "ai request started"
    );

    rt.spawn(async move {
        let started = Instant::now();
        let today = chrono::Local::now().format("%B %d, %Y").to_string();

        // ── 2. Try native provider web-search (Gemini Google Search / Anthropic web_search) ──
        let native_system = format!(
            "You are a helpful AI assistant embedded in a web browser. \
             Today's date is {today}. \
             Search the web for the latest information to answer the user's question. \
             Be concise and accurate. Use markdown formatting (bold key values, bullet lists). \
             Keep the answer under 300 words unless more detail is truly needed."
        );
        let mut native_msgs = vec![ai::ChatMessage::system(native_system)];
        for t in &history {
            native_msgs.push(if t.role == "assistant" {
                ai::ChatMessage::assistant(t.content.clone())
            } else {
                ai::ChatMessage::user(t.content.clone())
            });
        }
        native_msgs.push(ai::ChatMessage::user(query_text.clone()));
        let native_req = ai::ChatRequest {
            messages: native_msgs,
            model: model.clone(),
            temperature,
            max_tokens,
            stream: false,
            reasoning_effort: reasoning_effort.clone(),
            tools: None,
        };

        // Only call spotlight_chat for providers that support native search.
        // Others return Ok(None) from the default trait impl so we fall through.
        let native_result = if matches!(provider_id.as_str(), "gemini" | "anthropic") {
            prov.spotlight_chat(native_req).await
        } else {
            Ok(None)
        };

        match native_result {
            Ok(Some(answer)) if !answer.is_empty() => {
                // Native search returned a grounded answer — stream it.
                tracing::info!(
                    target: "ventus::ai",
                    kind = "spotlight",
                    source = "native",
                    chars = answer.chars().count(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "ai request finished"
                );
                stream_spotlight_text(answer, &proxy_task).await;
                return;
            }
            Err(e) => {
                // Native search failed — log and fall through to Wikipedia path.
                tracing::warn!(
                    target: "ventus::ai",
                    kind = "spotlight",
                    source = "native",
                    error = %e,
                    "ai native search failed"
                );
            }
            _ => {}
        }

        // ── 3. Fallback: parallel Wikipedia + currency + DDG Instant ──────────
        let currency_query = detect_currency_query(&query_text);
        let market_symbol = detect_market_query(&query_text);
        let (currency_ctx, market_ctx, instant_ctx, wiki_snippets) = tokio::join!(
            async {
                if let Some(query) = currency_query {
                    fetch_currency_rate(&query).await
                } else {
                    None
                }
            },
            async {
                if let Some((symbol, name)) = market_symbol {
                    fetch_market_quote(&symbol, &name).await
                } else {
                    None
                }
            },
            fetch_duckduckgo_instant(&query_text),
            fetch_wikipedia_search(&query_text)
        );

        let mut ctx_parts: Vec<String> = Vec::new();
        if let Some(ref rate) = currency_ctx {
            ctx_parts.push(rate.clone());
        }
        if let Some(ref market) = market_ctx {
            ctx_parts.push(market.clone());
        }
        if let Some(ref instant) = instant_ctx {
            ctx_parts.push(instant.clone());
        }
        if !wiki_snippets.is_empty() {
            ctx_parts.push(format!(
                "**Wikipedia search results:**\n{}",
                wiki_snippets
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("{}. {}", i + 1, s))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        let system = if ctx_parts.is_empty() {
            format!(
                "You are a helpful AI assistant embedded in a web browser. \
                 Today's date is {today}. \
                 No live search results were retrieved for this query. \
                 Answer honestly — if the answer depends on real-time or recent data \
                 that you may not have, say so explicitly rather than guessing. \
                 Use markdown formatting where helpful. Keep answers under 300 words."
            )
        } else {
            let search_block = ctx_parts.join("\n\n");
            format!(
                "You are a helpful AI assistant embedded in a web browser. \
                 Today's date is {today}.\n\n\
                 The following live web search results were retrieved for the user's query:\n\n\
                 {search_block}\n\n\
                 IMPORTANT RULES:\n\
                 - Answer using ONLY the search results shown above.\n\
                 - Do NOT use your training knowledge or make up facts.\n\
                 - If the search results do not contain enough information, say so.\n\
                 - Be concise and accurate. Use markdown formatting where helpful \
                 (bold for key values, bullet lists for multiple items).\n\
                 - Keep the answer under 300 words unless more detail is truly needed."
            )
        };

        let mut msgs = vec![ai::ChatMessage::system(system)];
        for t in &history {
            msgs.push(if t.role == "assistant" {
                ai::ChatMessage::assistant(t.content.clone())
            } else {
                ai::ChatMessage::user(t.content.clone())
            });
        }
        msgs.push(ai::ChatMessage::user(text));
        let req = ai::ChatRequest {
            messages: msgs,
            model,
            temperature,
            max_tokens,
            stream: false,
            reasoning_effort: reasoning_effort.clone(),
            tools: None,
        };

        match prov.chat(req).await {
            Ok(resp) => {
                if resp.content.is_empty() {
                    tracing::warn!(
                        target: "ventus::ai",
                        kind = "spotlight",
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "ai empty response"
                    );
                    let _ = proxy_task.send_event(AppEvent::SpotlightAiError {
                        message: "No response from AI.".to_string(),
                    });
                    return;
                }
                tracing::info!(
                    target: "ventus::ai",
                    kind = "spotlight",
                    source = "fallback",
                    context_parts = ctx_parts.len(),
                    chars = resp.content.chars().count(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "ai request finished"
                );
                stream_spotlight_text(resp.content, &proxy_task).await;
            }
            Err(e) => {
                tracing::warn!(
                    target: "ventus::ai",
                    kind = "spotlight",
                    error = %e,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "ai request failed"
                );
                let _ = proxy_task.send_event(AppEvent::SpotlightAiError {
                    message: format!("AI error: {}", e),
                });
            }
        }
    });
}

/// Stream an already-complete text answer back to the Spotlight UI word-by-word
/// so it feels like a live response.
async fn stream_spotlight_text(text: String, proxy: &tao::event_loop::EventLoopProxy<AppEvent>) {
    let words: Vec<&str> = text.split(' ').collect();
    let chunk_size = 3.max(words.len() / 25);
    let mut i = 0;
    while i < words.len() {
        let end = (i + chunk_size).min(words.len());
        let chunk = words[i..end].join(" ");
        let sep = if end < words.len() { " " } else { "" };
        let _ = proxy.send_event(AppEvent::SpotlightAiChunk {
            text: format!("{}{}", chunk, sep),
            done: false,
        });
        i = end;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let _ = proxy.send_event(AppEvent::SpotlightAiChunk {
        text: String::new(),
        done: true,
    });
}

#[derive(Clone)]
struct CurrencyQuery {
    amount: f64,
    from: String,
    to: String,
}

fn detect_currency_query(q: &str) -> Option<CurrencyQuery> {
    let mut s = q.to_lowercase();
    s = s.replace(',', "");
    s = s.replace('$', " usd ");
    s = s.replace('€', " eur ");
    s = s.replace('£', " gbp ");
    s = s.replace('¥', " jpy ");
    for ch in ['?', '!', ':', ';', '(', ')', '[', ']', '{', '}'] {
        s = s.replace(ch, " ");
    }
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }

    let seps = ["to", "ke", "in", "into", "as", "for"];
    for (i, token) in tokens.iter().enumerate() {
        if !seps.contains(token) {
            continue;
        }
        let left = &tokens[..i];
        let right = &tokens[i + 1..];
        let (from, amount) = find_currency_left(left)?;
        let to = find_currency_right(right)?;
        if from != to {
            return Some(CurrencyQuery { amount, from, to });
        }
    }

    let found: Vec<(usize, String)> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, token)| currency_code(token).map(|code| (i, code.to_string())))
        .collect();
    if found.len() < 2 || found[0].1 == found[1].1 {
        return None;
    }
    let amount = tokens[..found[0].0]
        .iter()
        .rev()
        .find_map(|token| token.parse::<f64>().ok())
        .unwrap_or(1.0);
    Some(CurrencyQuery {
        amount,
        from: found[0].1.clone(),
        to: found[1].1.clone(),
    })
}

fn find_currency_left(tokens: &[&str]) -> Option<(String, f64)> {
    for token in tokens.iter().rev() {
        let Some(code) = currency_code(token) else {
            continue;
        };
        let amount = tokens
            .iter()
            .rev()
            .find_map(|token| token.parse::<f64>().ok())
            .unwrap_or(1.0);
        return Some((code.to_string(), amount));
    }
    None
}

fn find_currency_right(tokens: &[&str]) -> Option<String> {
    tokens
        .iter()
        .find_map(|token| currency_code(token).map(|code| code.to_string()))
}

fn currency_code(token: &str) -> Option<&'static str> {
    match token {
        "usd" | "dollar" | "dollars" | "buck" | "bucks" | "us$" => Some("USD"),
        "idr" | "rupiah" | "rp" => Some("IDR"),
        "jpy" | "yen" => Some("JPY"),
        "cny" | "yuan" | "rmb" | "renminbi" => Some("CNY"),
        "eur" | "euro" | "euros" => Some("EUR"),
        "gbp" | "pound" | "pounds" | "sterling" => Some("GBP"),
        "sgd" | "singapore" => Some("SGD"),
        "myr" | "ringgit" => Some("MYR"),
        "thb" | "baht" => Some("THB"),
        "aud" => Some("AUD"),
        "cad" => Some("CAD"),
        "chf" | "franc" => Some("CHF"),
        "hkd" => Some("HKD"),
        "krw" | "won" => Some("KRW"),
        "inr" | "rupee" | "rupees" => Some("INR"),
        "php" | "peso" | "pesos" => Some("PHP"),
        "vnd" | "dong" => Some("VND"),
        "twd" => Some("TWD"),
        "brl" | "real" | "reais" => Some("BRL"),
        "mxn" => Some("MXN"),
        "rub" | "ruble" | "rouble" => Some("RUB"),
        "zar" | "rand" => Some("ZAR"),
        "aed" | "dirham" => Some("AED"),
        "sar" | "riyal" => Some("SAR"),
        "try" | "lira" => Some("TRY"),
        "nok" => Some("NOK"),
        "sek" => Some("SEK"),
        "dkk" => Some("DKK"),
        "nzd" => Some("NZD"),
        _ => None,
    }
}

async fn fetch_currency_rate(query: &CurrencyQuery) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent(crate::version::USER_AGENT)
        .build()
        .ok()?;

    let url = format!("https://open.er-api.com/v6/latest/{}", query.from);
    let resp = client.get(&url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    if json["result"].as_str() != Some("success") {
        return None;
    }

    let rate = json["rates"][&query.to].as_f64()?;
    let converted = query.amount * rate;
    let updated = json["time_last_update_utc"].as_str().unwrap_or("recently");

    Some(format!(
        "**Live currency conversion:** {} {} = **{} {}**\nRate: 1 {} = {} {}.\nSource: open.er-api.com, updated: {}.",
        fmt_currency_value(query.amount),
        query.from,
        fmt_currency_value(converted),
        query.to,
        query.from,
        fmt_currency_value(rate),
        query.to,
        updated
    ))
}

fn fmt_currency_value(n: f64) -> String {
    let decimals = if n.abs() >= 100.0 { 2 } else { 4 };
    let s = format!("{n:.decimals$}");
    let parts: Vec<&str> = s.split('.').collect();
    let mut int = String::new();
    for (i, ch) in parts[0].chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            int.push(',');
        }
        int.push(ch);
    }
    let int = int.chars().rev().collect::<String>();
    if parts.len() == 1 {
        return int;
    }
    let frac = parts[1].trim_end_matches('0');
    if frac.is_empty() {
        int
    } else {
        format!("{int}.{frac}")
    }
}

#[cfg(test)]
mod webview_arg_tests {
    use super::*;

    fn secure_dns_cases() -> [(config::SecureDnsProvider, &'static str); 6] {
        [
            (
                config::SecureDnsProvider::Cloudflare,
                config::CLOUDFLARE_DOH,
            ),
            (
                config::SecureDnsProvider::CloudflareMalware,
                config::CLOUDFLARE_MALWARE_DOH,
            ),
            (
                config::SecureDnsProvider::CloudflareFamily,
                config::CLOUDFLARE_FAMILY_DOH,
            ),
            (config::SecureDnsProvider::Google, config::GOOGLE_DOH),
            (config::SecureDnsProvider::OpenDns, config::OPENDNS_DOH),
            (
                config::SecureDnsProvider::OpenDnsFamily,
                config::OPENDNS_FAMILY_DOH,
            ),
        ]
    }

    #[test]
    fn secure_dns_adds_cloudflare_args() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        let args = webview_args(&settings);
        assert!(args.contains("--enable-async-dns"));
        assert!(args.contains(&doh_feature_arg(
            config::CLOUDFLARE_DOH,
            &config::SecureDnsMode::Secure
        )));
        assert!(args.contains("--dns-over-https-mode=secure"));
        assert!(args.contains("--dns-over-https-templates=https://cloudflare-dns.com/dns-query"));
    }

    #[test]
    fn secure_dns_automatic_allows_fallback_in_feature_params() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        settings.privacy.secure_dns_mode = config::SecureDnsMode::Automatic;
        let args = webview_args(&settings);
        assert!(args.contains(&doh_feature_arg(
            config::CLOUDFLARE_DOH,
            &config::SecureDnsMode::Automatic
        )));
        assert!(args.contains("--dns-over-https-mode=automatic"));
    }

    #[test]
    fn secure_dns_args_cover_all_builtin_providers() {
        for (provider, endpoint) in secure_dns_cases() {
            let mut settings = config::AppSettings::default();
            settings.privacy.secure_dns_enabled = true;
            settings.privacy.secure_dns_provider = provider;

            let args = webview_args(&settings);

            assert!(args.contains("--enable-async-dns"));
            assert!(args.contains(&doh_feature_arg(endpoint, &config::SecureDnsMode::Secure)));
            assert!(args.contains("--dns-over-https-mode=secure"));
            assert!(args.contains(&format!("--dns-over-https-templates={endpoint}")));
        }
    }

    #[test]
    fn secure_dns_custom_args_require_valid_https_endpoint() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        settings.privacy.secure_dns_provider = config::SecureDnsProvider::Custom;
        settings.privacy.secure_dns_template = " https://dns.example/dns-query ".to_string();

        let args = webview_args(&settings);
        assert!(args.contains("--dns-over-https-templates=https://dns.example/dns-query"));

        for invalid in [
            "",
            "http://dns.example/dns-query",
            "https://dns.example/dns query",
        ] {
            settings.privacy.secure_dns_template = invalid.to_string();
            let args = webview_args(&settings);
            assert!(!args.contains("dns-over-https"));
        }
    }

    #[test]
    fn secure_dns_off_keeps_doh_out() {
        let settings = config::AppSettings::default();
        let args = webview_args(&settings);
        assert!(!args.contains("dns-over-https"));
    }

    #[test]
    fn content_background_matches_browser_default() {
        assert_eq!(CONTENT_BG, (255, 255, 255, 255));
    }

    #[test]
    fn strict_permissions_keep_clipboard_copy_available() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script = privacy_initialization_script(false, true, &sites, &defaults);
        assert!(script.contains("navigator.clipboard.read = blocked;"));
        assert!(script.contains("navigator.clipboard.readText = blocked;"));
        assert!(!script.contains("navigator.clipboard.write = blocked;"));
        assert!(!script.contains("navigator.clipboard.writeText = blocked;"));
    }

    #[test]
    fn strict_permissions_keep_media_devices_available() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script = privacy_initialization_script(false, true, &sites, &defaults);
        assert!(!script.contains("getUserMedia = function"));
        assert!(!script.contains("enumerateDevices = () => Promise.resolve([])"));
    }

    #[cfg(windows)]
    #[test]
    fn strict_permissions_ask_for_media_by_default() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        assert_eq!(
            permission_action(
                true,
                &sites,
                &defaults,
                "https://meet.google.com",
                "microphone"
            ),
            "ask"
        );
        assert_eq!(
            permission_action(true, &sites, &defaults, "https://meet.google.com", "camera"),
            "ask"
        );
        assert_eq!(
            permission_action(
                true,
                &sites,
                &defaults,
                "https://meet.google.com",
                "geolocation"
            ),
            "block"
        );
    }

    #[cfg(windows)]
    #[test]
    fn explicit_media_permission_rules_win() {
        let mut sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let mut perms = config::SitePermissions::default();
        assert!(perms.set("microphone", "block"));
        sites.insert("https://meet.google.com".to_string(), perms);
        assert_eq!(
            permission_action(
                true,
                &sites,
                &defaults,
                "https://meet.google.com",
                "microphone"
            ),
            "block"
        );

        let sites = config::SitePermissionMap::new();
        let mut defaults = config::SitePermissions::default();
        assert!(defaults.set("camera", "allow"));
        assert_eq!(
            permission_action(true, &sites, &defaults, "https://meet.google.com", "camera"),
            "allow"
        );
    }

    #[test]
    fn chromium_versions_parse_runtime_strings() {
        assert_eq!(
            chromium_versions_from_raw("149.0.3065.92").unwrap(),
            (
                "149.0.3065.92".to_string(),
                "149.0.0.0".to_string(),
                "149".to_string()
            )
        );
        assert_eq!(
            chromium_versions_from_raw("WebView2 Runtime 150.1").unwrap(),
            (
                "150.1.0.0".to_string(),
                "150.0.0.0".to_string(),
                "150".to_string()
            )
        );
        assert!(chromium_versions_from_raw("runtime unavailable").is_none());
    }

    #[test]
    fn auth_popup_urls_stay_popups() {
        assert!(is_auth_popup_url(
            "https://accounts.google.com/gsi/confirm?client_id=abc"
        ));
        assert!(is_auth_popup_url(
            "https://x.com/i/flow/single_sign_on?provider=google"
        ));
        assert!(is_auth_popup_url(
            "https://login.live.com/oauth20_authorize.srf"
        ));
        assert!(!is_auth_popup_url(
            "https://example.com/article/oauth-history"
        ));
        assert!(!is_auth_popup_url("about:blank"));
    }

    #[test]
    fn secure_dns_local_state_sets_cloudflare_prefs() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        let mut local_state = serde_json::json!({"existing": true});

        apply_secure_dns_local_state(&mut local_state, &settings);

        assert_eq!(local_state["existing"], true);
        assert_eq!(local_state["dns_over_https"]["mode"], "secure");
        assert_eq!(
            local_state["dns_over_https"]["templates"],
            "https://cloudflare-dns.com/dns-query"
        );
        assert_eq!(
            local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
            false
        );
        assert_eq!(local_state["async_dns"]["enabled"], true);
    }

    #[test]
    fn secure_dns_local_state_turns_off_stale_prefs() {
        let settings = config::AppSettings::default();
        let mut local_state = serde_json::json!({
            "dns_over_https": {
                "mode": "secure",
                "templates": "https://cloudflare-dns.com/dns-query",
                "automatic_mode_fallback_to_doh": true
            },
            "async_dns": {
                "enabled": true
            }
        });

        apply_secure_dns_local_state(&mut local_state, &settings);

        assert_eq!(local_state["dns_over_https"]["mode"], "off");
        assert_eq!(local_state["dns_over_https"]["templates"], "");
        assert_eq!(
            local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
            false
        );
        assert_eq!(local_state["async_dns"]["enabled"], false);
    }

    #[test]
    fn secure_dns_local_state_covers_all_providers_and_modes() {
        for (provider, endpoint) in secure_dns_cases() {
            for (mode, mode_arg, fallback) in [
                (config::SecureDnsMode::Secure, "secure", false),
                (config::SecureDnsMode::Automatic, "automatic", true),
            ] {
                let mut settings = config::AppSettings::default();
                settings.privacy.secure_dns_enabled = true;
                settings.privacy.secure_dns_provider = provider.clone();
                settings.privacy.secure_dns_mode = mode;
                let mut local_state = serde_json::json!({"keep_me": {"nested": true}});

                apply_secure_dns_local_state(&mut local_state, &settings);

                assert_eq!(local_state["keep_me"]["nested"], true);
                assert_eq!(local_state["dns_over_https"]["mode"], mode_arg);
                assert_eq!(local_state["dns_over_https"]["templates"], endpoint);
                assert_eq!(
                    local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
                    fallback
                );
                assert_eq!(local_state["async_dns"]["enabled"], true);
            }
        }
    }

    #[test]
    fn write_webview_secure_dns_prefs_preserves_existing_local_state() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!(
                "secure-dns-local-state-test-{}",
                std::process::id()
            ));
        let profile_dir = root.join("EBWebView");
        let local_state_path = profile_dir.join("Local State");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(
            &local_state_path,
            r#"{"unrelated":{"value":7},"dns_over_https":{"mode":"off"}}"#,
        )
        .unwrap();

        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        settings.privacy.secure_dns_provider = config::SecureDnsProvider::Google;
        settings.privacy.secure_dns_mode = config::SecureDnsMode::Automatic;

        write_webview_secure_dns_prefs(&root, &settings).unwrap();

        let local_state = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&local_state_path).unwrap(),
        )
        .unwrap();
        assert_eq!(local_state["unrelated"]["value"], 7);
        assert_eq!(local_state["dns_over_https"]["mode"], "automatic");
        assert_eq!(
            local_state["dns_over_https"]["templates"],
            config::GOOGLE_DOH
        );
        assert_eq!(
            local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
            true
        );
        assert_eq!(local_state["async_dns"]["enabled"], true);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn webview_profile_lock_paths_include_real_webview2_lockfile() {
        let root = std::path::PathBuf::from(r"C:\VentusProfile");
        let paths = webview_profile_lock_paths(&root);
        assert!(paths.contains(&root.join("EBWebView").join("lockfile")));
        assert!(paths.contains(&root.join("EBWebView").join("LOCK")));
        assert!(paths.contains(&root.join("EBWebView").join("Default").join("LOCK")));
    }

    #[cfg(windows)]
    #[test]
    fn webview_profile_lock_released_detects_exclusive_lock() {
        use std::os::windows::fs::OpenOptionsExt;

        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("webview-profile-lock-test-{}", std::process::id()));
        let lock_dir = root.join("EBWebView").join("Default");
        let lock_path = lock_dir.join("LOCK");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&lock_dir).unwrap();
        std::fs::write(&lock_path, b"").unwrap();

        assert!(webview_profile_lock_released(&root));

        let guard = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&lock_path)
            .unwrap();

        assert!(!webview_profile_lock_released(&root));

        drop(guard);
        assert!(webview_profile_lock_released(&root));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn wait_for_previous_instance_skips_pid_when_profile_is_free() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("startup-wait-test-{}", std::process::id()));
        let profile = root.join("webview_data");
        let sentinel = root.join("running.lock");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(&sentinel, std::process::id().to_string()).unwrap();

        let started = Instant::now();
        wait_for_previous_instance(&sentinel, &[profile.as_path()]);

        assert!(started.elapsed() < Duration::from_millis(500));
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn maps_tab_shortcuts() {
        assert_eq!(msg_shortcut(0x54, MOD_CTRL, false), SC_SPOTLIGHT);
        assert_eq!(
            msg_shortcut(0x54, MOD_CTRL | MOD_SHIFT, false),
            SC_REOPEN_TAB
        );
        assert_eq!(msg_shortcut(0x09, MOD_CTRL, false), SC_NEXT_TAB);
        assert_eq!(msg_shortcut(0x09, MOD_CTRL | MOD_SHIFT, false), SC_PREV_TAB);
        assert_eq!(msg_shortcut(0x57, MOD_CTRL, false), SC_CLOSE_TAB);
        assert_eq!(msg_shortcut(0x46, MOD_CTRL, false), SC_FIND);
    }

    #[test]
    fn maps_nav_and_window_shortcuts() {
        assert_eq!(msg_shortcut(0x25, MOD_ALT, false), SC_NONE);
        assert_eq!(msg_shortcut(0x27, MOD_ALT, false), SC_NONE);
        assert_eq!(msg_shortcut(0x74, 0, false), SC_RELOAD);
        assert_eq!(msg_shortcut(0x7a, 0, false), SC_FULLSCREEN);
    }

    #[test]
    fn ignores_repeats_and_plain_letters() {
        assert_eq!(msg_shortcut(0x54, MOD_CTRL, true), SC_NONE);
        assert_eq!(msg_shortcut(0x54, 0, false), SC_NONE);
    }
}

#[cfg(test)]
mod currency_tests {
    use super::*;

    #[test]
    fn parses_code_amount() {
        let q = detect_currency_query("100 usd to idr").unwrap();
        assert_eq!(q.from, "USD");
        assert_eq!(q.to, "IDR");
        assert_eq!(q.amount, 100.0);
    }

    #[test]
    fn parses_names() {
        let q = detect_currency_query("jpy to yuan").unwrap();
        assert_eq!(q.from, "JPY");
        assert_eq!(q.to, "CNY");
        assert_eq!(q.amount, 1.0);
    }

    #[test]
    fn parses_symbol_amount() {
        let q = detect_currency_query("$10 to rupiah").unwrap();
        assert_eq!(q.from, "USD");
        assert_eq!(q.to, "IDR");
        assert_eq!(q.amount, 10.0);
    }

    #[test]
    fn parses_indonesian_separator() {
        let q = detect_currency_query("25000 yen ke rupiah").unwrap();
        assert_eq!(q.from, "JPY");
        assert_eq!(q.to, "IDR");
        assert_eq!(q.amount, 25000.0);
    }
}

fn detect_market_query(q: &str) -> Option<(String, String)> {
    let q = q.to_lowercase();
    if q.contains("ihsg") || q.contains("jkse") || q.contains("jakarta composite") {
        return Some(("%5EJKSE".to_string(), "IHSG".to_string()));
    }
    None
}

async fn fetch_market_quote(symbol: &str, name: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent(crate::version::USER_AGENT)
        .build()
        .ok()?;
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1d&interval=1m",
        symbol
    );
    let json: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let result = json["chart"]["result"].as_array()?.first()?;
    let meta = &result["meta"];
    let price = meta["regularMarketPrice"]
        .as_f64()
        .or_else(|| meta["previousClose"].as_f64())?;
    let previous = meta["previousClose"]
        .as_f64()
        .or_else(|| meta["chartPreviousClose"].as_f64());
    let currency = meta["currency"].as_str().unwrap_or("");
    let exchange = meta["exchangeName"].as_str().unwrap_or("Yahoo Finance");
    let updated = meta["regularMarketTime"]
        .as_i64()
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "latest available".to_string());
    let change = previous.map(|prev| {
        let diff = price - prev;
        let pct = if prev != 0.0 {
            diff / prev * 100.0
        } else {
            0.0
        };
        format!(" ({diff:+.2}, {pct:+.2}%)")
    });

    Some(format!(
        "**Live market data:** {name} is **{price:.2} {currency}**{}.\nSource: Yahoo Finance chart API, exchange: {exchange}, updated: {updated}.",
        change.unwrap_or_default()
    ))
}

/// Search Wikipedia and return up to 4 plain-text snippet strings.
/// Uses the MediaWiki Action API — completely free, no key, works from any network.
/// Snippets are HTML-cleaned via `decode_html_text` before being returned.
async fn fetch_wikipedia_search(query: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent(crate::version::USER_AGENT)
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let resp = match client
        .get("https://en.wikipedia.org/w/api.php")
        .query(&[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", query),
            ("format", "json"),
            ("utf8", "1"),
            ("srlimit", "4"),
            ("srprop", "snippet|titlesnippet"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return Vec::new(),
    };

    let arr = match json["query"]["search"].as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .filter_map(|item| {
            let title = item["title"].as_str()?;
            let snippet = item["snippet"].as_str()?;
            let clean = decode_html_text(snippet);
            let clean = clean.trim();
            if clean.is_empty() {
                return None;
            }
            Some(format!("**{}** — {}", title, clean))
        })
        .collect()
}

/// Decode common HTML entities and strip inline tags from a snippet string.
/// Handles &amp; &lt; &gt; &quot; &#39; and removes <b>/<em>/<span> tags.
fn decode_html_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Skip until closing '>'
            for c in chars.by_ref() {
                if c == '>' {
                    break;
                }
            }
        } else if ch == '&' {
            // Collect entity up to ';'
            let mut entity = String::new();
            for c in chars.by_ref() {
                if c == ';' {
                    break;
                }
                entity.push(c);
            }
            match entity.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "#39" | "apos" => out.push('\''),
                "#160" | "nbsp" => out.push(' '),
                _ => {
                    // Unknown entity — emit as-is
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
}

/// Fetch live data from the DuckDuckGo Instant Answer API (no key required).
/// Returns a formatted context string when a direct answer or abstract is available,
/// covering currency conversions, calculations, unit conversions, and factual queries.
async fn fetch_duckduckgo_instant(query: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent(crate::version::USER_AGENT)
        .build()
        .ok()?;

    let resp = client
        .get("https://api.duckduckgo.com/")
        .query(&[
            ("q", query),
            ("format", "json"),
            ("no_html", "1"),
            ("skip_disambig", "1"),
        ])
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;

    let mut parts: Vec<String> = Vec::new();

    // Direct answer — covers currency conversions, calculations, unit conversions, etc.
    if let Some(answer) = json["Answer"].as_str() {
        let a = answer.trim();
        if !a.is_empty() {
            parts.push(format!("**Live answer:** {}", a));
        }
    }

    // Wikipedia / knowledge-base abstract for factual queries
    if let Some(text) = json["AbstractText"].as_str() {
        let t = text.trim();
        if !t.is_empty() {
            let snippet = if t.len() > 500 { &t[..500] } else { t };
            let source = json["AbstractSource"].as_str().unwrap_or("reference");
            parts.push(format!("**From {}:** {}", source, snippet));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Returns available physical memory in MB. Falls back to u64::MAX so the
/// sleep threshold stays at its maximum when the value cannot be read.
#[cfg(windows)]
fn available_memory_mb() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut ms).is_ok() {
            return ms.ullAvailPhys / 1024 / 1024;
        }
    }
    u64::MAX
}

#[cfg(not(windows))]
fn available_memory_mb() -> u64 {
    u64::MAX
}

fn sleep_threshold_ms(free_mb: u64) -> u64 {
    match free_mb {
        m if m > 8192 => 4 * 60 * 60 * 1000,
        m if m > 4096 => 2 * 60 * 60 * 1000,
        m if m > 2048 => 30 * 60 * 1000,
        m if m > 1024 => 10 * 60 * 1000,
        m if m > 512 => 4 * 60 * 1000,
        _ => 90 * 1000,
    }
}

fn max_live_webviews(free_mb: u64) -> usize {
    match free_mb {
        m if m > 8192 => 24,
        m if m > 4096 => 16,
        m if m > 2048 => 10,
        m if m > 1024 => 6,
        _ => 4,
    }
}

fn tab_notifications_allowed(url: &str, settings: &config::AppSettings) -> bool {
    #[cfg(windows)]
    {
        let Some(origin) = normalize_webview_origin(url) else {
            return false;
        };
        permission_action(
            settings.privacy.strict_permissions,
            &settings.privacy.site_permissions,
            &settings.privacy.default_permissions,
            &origin,
            "notifications",
        ) == "allow"
    }
    #[cfg(not(windows))]
    {
        let _ = (url, settings);
        false
    }
}

fn image_filename_from_url(url: &str) -> String {
    if url.starts_with("data:") {
        let mime = url
            .get(5..)
            .and_then(|s| s.split(';').next())
            .unwrap_or("image/png");
        let ext = match mime {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "image/bmp" => "bmp",
            "image/ico" | "image/x-icon" => "ico",
            _ => "png",
        };
        return format!("image.{}", ext);
    }
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(last) = parsed.path_segments().and_then(|s| s.last()) {
            let name = last.split('?').next().unwrap_or(last);
            let lower = name.to_lowercase();
            let known_ext = [
                "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif",
            ]
            .iter()
            .any(|e| lower.ends_with(&format!(".{}", e)));
            if known_ext && !name.is_empty() {
                return name.to_string();
            }
        }
    }
    "image.png".to_string()
}

/// A confirmed "Save image as" destination awaiting the image bytes from the content WebView.
struct PendingImageSave {
    dest: std::path::PathBuf,
    url: String,
    /// Page URL used as the Referer for the reqwest fallback (hotlink-protected hosts).
    referer: String,
}

/// Decode the bytes encoded in a `data:` URL. Handles both base64 (`;base64,`) and
/// percent-encoded payloads (e.g. inline SVG). The `data:` part may be absent — anything
/// after the first comma is treated as the payload.
fn decode_data_url(data: &str) -> anyhow::Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let comma = data
        .find(',')
        .ok_or_else(|| anyhow::anyhow!("malformed data URL"))?;
    let header = &data[..comma];
    let payload = &data[comma + 1..];
    if header.contains(";base64") {
        // Whitespace can appear in long base64 data URLs — strip it before decoding.
        let cleaned: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
        Ok(STANDARD.decode(cleaned)?)
    } else {
        // Percent-decoded text payload.
        Ok(percent_decode(payload))
    }
}

fn percent_decode(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

/// Download an image over HTTP(S) using a browser-like User-Agent and a Referer header.
/// The Referer lets hotlink-protected hosts serve the asset, and a real UA avoids the
/// blanket 403s some CDNs return for unknown clients. Errors on any non-success status so
/// the caller can mark the download Failed instead of writing an HTML error page to disk.
async fn fetch_image_bytes(url: &str, referer: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent(crate::version::USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let mut req = client.get(url);
    if !referer.trim().is_empty() && referer.starts_with("http") {
        req = req.header(reqwest::header::REFERER, referer);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("http {}", resp.status().as_u16()));
    }
    Ok(resp.bytes().await?.to_vec())
}

/// Decode image bytes (any supported format) into a CF_DIB bitmap and write it to
/// the Windows clipboard so the user can paste it into any application.
/// Uses a 24-bit BGR bottom-up DIB which is universally accepted by Win32 apps.
#[cfg(windows)]
fn write_image_to_clipboard(bytes: &[u8]) -> anyhow::Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};

    let img = image::load_from_memory(bytes)?.to_rgba8();
    let width = img.width() as usize;
    let height = img.height() as usize;
    let pixels = img.as_raw();

    // CF_DIB: BITMAPINFOHEADER (40 bytes) + 24bpp BGR rows, bottom-up, 4-byte-aligned stride
    let stride = (width * 3 + 3) & !3;
    const HDR: usize = 40;
    let total = HDR + stride * height;
    let mut dib = vec![0u8; total];

    dib[0..4].copy_from_slice(&40u32.to_le_bytes()); // biSize
    dib[4..8].copy_from_slice(&(width as i32).to_le_bytes()); // biWidth
    dib[8..12].copy_from_slice(&(height as i32).to_le_bytes()); // biHeight (positive = bottom-up)
    dib[12..14].copy_from_slice(&1u16.to_le_bytes()); // biPlanes
    dib[14..16].copy_from_slice(&24u16.to_le_bytes()); // biBitCount
                                                       // biCompression, biSizeImage, biX/YPelsPerMeter, biClrUsed, biClrImportant = 0

    for dib_row in 0..height {
        let img_row = height - 1 - dib_row; // flip: DIB row 0 = bottom of image
        let dst = HDR + dib_row * stride;
        let src = img_row * width * 4;
        for col in 0..width {
            dib[dst + col * 3] = pixels[src + col * 4 + 2]; // B
            dib[dst + col * 3 + 1] = pixels[src + col * 4 + 1]; // G
            dib[dst + col * 3 + 2] = pixels[src + col * 4]; // R
        }
    }

    unsafe { OpenClipboard(HWND(0))? };
    let result = write_dib_to_open_clipboard(&dib);
    unsafe {
        let _ = CloseClipboard();
    }
    result
}

#[cfg(windows)]
fn write_dib_to_open_clipboard(dib: &[u8]) -> anyhow::Result<()> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{EmptyClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    unsafe {
        EmptyClipboard()?;
        let hmem = GlobalAlloc(GMEM_MOVEABLE, dib.len())?;
        let ptr = GlobalLock(hmem) as *mut u8;
        if ptr.is_null() {
            return Err(anyhow::anyhow!("GlobalLock failed"));
        }
        std::ptr::copy_nonoverlapping(dib.as_ptr(), ptr, dib.len());
        let _ = GlobalUnlock(hmem);
        if let Err(e) = SetClipboardData(8u32, HANDLE(hmem.0 as isize)) {
            return Err(anyhow::anyhow!("SetClipboardData: {e}"));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
fn write_image_to_clipboard(_bytes: &[u8]) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "clipboard write not supported on this platform"
    ))
}

#[cfg(windows)]
fn read_clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::{HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        if OpenClipboard(HWND(0)).is_err() {
            return None;
        }
        let text = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
            if handle.0 == 0 {
                return None;
            }
            let hglobal = HGLOBAL(handle.0 as *mut core::ffi::c_void);
            let ptr = GlobalLock(hglobal) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            let _ = GlobalUnlock(hglobal);
            Some(s)
        })();
        let _ = CloseClipboard();
        text
    }
}

#[cfg(not(windows))]
fn read_clipboard_text() -> Option<String> {
    None
}

/// JS injected into the active content WebView to fetch an image the way the page itself
/// would — with its cookies/session and full access to `blob:` URLs that don't exist
/// outside the page. The bytes come back as a base64 data URL via IPC; Rust writes them to
/// the user-chosen path. Cross-origin images the page can't read (no CORS) reject here and
/// fall back to a server-side reqwest fetch in Rust.
fn save_image_fetch_script(save_id: &str, url: &str) -> String {
    let id_js = serde_json::to_string(save_id).unwrap_or_else(|_| "\"\"".into());
    let url_js = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(function(){{
  var __id={id}, __u={url};
  function __post(ok,data){{
    try{{ window.ipc.postMessage(JSON.stringify({{cmd:'save_image_data',save_id:__id,ok:ok,data:data||''}})); }}catch(e){{}}
  }}
  try{{
    fetch(__u,{{credentials:'include'}})
      .then(function(r){{ if(!r.ok) throw new Error('status'); return r.blob(); }})
      .then(function(b){{
        var fr=new FileReader();
        fr.onload=function(){{ __post(true,fr.result); }};
        fr.onerror=function(){{ __post(false,''); }};
        fr.readAsDataURL(b);
      }})
      .catch(function(){{ __post(false,''); }});
  }}catch(e){{ __post(false,''); }}
}})()"#,
        id = id_js,
        url = url_js
    )
}
