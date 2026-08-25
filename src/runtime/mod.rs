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
use wry::{Rect, Theme as WebViewTheme, WebView, WebViewBuilder};

use crate::{ai, app, browser, cloud, config, notify, storage, ui, updater, utils, version};

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
const SPA_STALL_AFTER: u64 = 10;
const COVER_MAX_MS: u64 = 1000;
const APP_PANEL_SLEEP_AFTER: Duration = Duration::from_secs(30);
const TAB_SLEEP_CHECK_EVERY: Duration = Duration::from_secs(5);
const TAB_SLEEP_CHECK_FAST: Duration = Duration::from_secs(6);
const TAB_SLEEP_CHECK_CRITICAL: Duration = Duration::from_secs(2);
const CRITICAL_COMMIT_MB: u64 = 768;
const EMERGENCY_DISCARD_IDLE_MS: i64 = 10_000;
// Grace period after media (audio or a large playing video) last stopped before a tab is
// eligible for sleep again. Gives hysteresis so a transient buffer/ad blip on a backgrounded
// livestream — where the keep-alive signal can briefly drop — does not get the tab suspended.
const MEDIA_GRACE_MS: i64 = 90_000;
const DISCARD_FREE_MB: u64 = 512;
const MAX_PRESERVED_WEBVIEWS: usize = 32;
const HEAL_CONTENT_EVERY: Duration = Duration::from_millis(750);
const HEAL_SETTLE_REPEATS: u8 = 6;
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
const FREEZE_REPORT_COOLDOWN: Duration = Duration::from_secs(2 * 60);
const RENDERER_REPORT_COOLDOWN: Duration = Duration::from_secs(60);
// Main-thread handler duration thresholds. Anything past WARN is janky and worth a log line;
// anything past FREEZE was a real "Not Responding" stall and is escalated to an auto report.
const MAIN_SLOW_WARN_MS: u128 = 350;
const MAIN_FREEZE_MS: u128 = 3000;

fn webview_theme(theme: &config::Theme) -> WebViewTheme {
    match theme {
        config::Theme::Dark => WebViewTheme::Dark,
        config::Theme::Light => WebViewTheme::Light,
        config::Theme::System => WebViewTheme::Auto,
    }
}

// Base background for content WebViews. Always the standard white canvas, exactly like real
// browsers: every page paints its OWN background, and Chromium already picks a dark canvas per-page
// for sites that opt in via `color-scheme: dark`. An older "white-flash" fix forced this dark
// whenever the app theme was dark, which removed a brief load flash on genuinely-dark sites but
// BROKE every light-only page — e.g. notebooklm.google renders its dark text and, having no
// explicit page background, inherited the forced-dark canvas → dark-on-dark and unreadable.
fn content_bg_for_theme(_theme: WebViewTheme) -> (u8, u8, u8, u8) {
    CONTENT_BG
}

fn window_backing_colorref(theme: WebViewTheme) -> u32 {
    let dark = match theme {
        WebViewTheme::Dark => true,
        WebViewTheme::Light => false,
        _ => crate::utils::sysinfo::os_prefers_dark(),
    };
    if dark {
        0x0014_1414
    } else {
        0x00f7_f3f2
    }
}

static WORST_FREEZE: std::sync::Mutex<Option<(u64, &'static str)>> = std::sync::Mutex::new(None);

struct MainEventTimer {
    start: Instant,
    label: &'static str,
}

impl Drop for MainEventTimer {
    fn drop(&mut self) {
        let ms = self.start.elapsed().as_millis();
        if ms < MAIN_SLOW_WARN_MS {
            return;
        }
        tracing::warn!(
            target: "ventus::perf",
            event = self.label,
            ms = ms as u64,
            "main-thread handler slow — UI was unresponsive this long"
        );
        if ms >= MAIN_FREEZE_MS {
            if let Ok(mut g) = WORST_FREEZE.lock() {
                let replace = g.map(|(prev, _)| ms as u64 > prev).unwrap_or(true);
                if replace {
                    *g = Some((ms as u64, self.label));
                }
            }
        }
    }
}

fn event_label(ev: &Event<AppEvent>) -> &'static str {
    use tao::event::WindowEvent as W;
    match ev {
        Event::NewEvents(_) => "new_events",
        Event::MainEventsCleared => "main_cleared",
        Event::RedrawRequested(_) => "redraw",
        Event::RedrawEventsCleared => "redraw_cleared",
        Event::LoopDestroyed => "loop_destroyed",
        Event::UserEvent(AppEvent::Chrome(cmd)) => cmd.report_name().unwrap_or("chrome_cmd"),
        Event::UserEvent(ev) => ev.label(),
        Event::WindowEvent { event, .. } => match event {
            W::CloseRequested => "win:close",
            W::Resized(_) => "win:resized",
            W::Moved(_) => "win:moved",
            W::Focused(_) => "win:focused",
            W::CursorMoved { .. } => "win:cursor",
            W::MouseInput { .. } => "win:mouse",
            W::MouseWheel { .. } => "win:wheel",
            W::KeyboardInput { .. } => "win:key",
            W::ScaleFactorChanged { .. } => "win:scale",
            _ => "win:other",
        },
        _ => "other",
    }
}

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
            // Reads the cache warmed at startup — no Win32 calls during unwinding.
            system: utils::sysinfo::summary_json(),
            logs: utils::log_buffer::snapshot(cloud::report::MAX_LOGS),
        };
        cloud::report::write_crash(&crash_path, &record);
        prev(info);
    }));
}

pub fn run() {
    set_app_id();
    utils::logging::init();
    // Warm the host-info cache once, up front: it must already be populated before the panic
    // hook (which reads it) can fire, and doing it here keeps every later report/crash read
    // free of any Win32 work. Also gives every log file a clear hardware banner at the top.
    let sys = utils::sysinfo::get();
    tracing::info!(
        target: "ventus::startup",
        version = version::APP_VERSION,
        os = %sys.os,
        os_build = %sys.os_build,
        os_display = %sys.os_display,
        cpu = %sys.cpu,
        cpu_cores = sys.cpu_cores,
        ram_total_mb = sys.ram_total_mb,
        gpu = %sys.gpu,
        arch = %sys.arch,
        screen = %sys.screen,
        monitors = sys.monitors,
        "Ventus starting — host info"
    );

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
    notify::register_aumid(&data_dir);
    let _instance = claim_instance(new_window, cli_url.as_deref(), &data_dir);

    let worker_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .clamp(1, 4);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let _guard = rt.enter();

    encrypt_app_storage(&data_dir);

    let conn = database::open(&data_dir.join("neura.db")).expect("open db");
    migrations::run(&conn).expect("migrations");
    repositories::seed_search_engines(&conn).expect("seed engines");

    let profile_cookie_db_found = webview_cookie_db_exists(&data_dir);
    let startup_cookies: Vec<cookie_store::CookieRecord> = if profile_cookie_db_found {
        tracing::info!("cookie_store: profile cookie DB found, skipping backup cookie heal");
        vec![]
    } else {
        match cookie_store::open(&data_dir) {
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
        let beta = settings.beta_channel;
        rt.spawn(async move {
            if let Ok(Some(info)) = updater::check_latest(beta).await {
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

    static LOGO_PNG: &[u8] = include_bytes!("../../public/ventus.png");
    let window_icon = image::load_from_memory(LOGO_PNG).ok().and_then(|img| {
        let img = img.resize(32, 32, image::imageops::FilterType::Lanczos3);
        let (w, h) = img.dimensions();
        tao::window::Icon::from_rgba(img.to_rgba8().into_raw(), w, h).ok()
    });

    let window = WindowBuilder::new()
        .with_title("Ventus")
        .with_inner_size(LogicalSize::new(win_w, win_h))
        .with_min_inner_size(LogicalSize::new(460u32, 420u32))
        .with_window_icon(window_icon)
        .with_decorations(false)
        .with_visible(false)
        .build(&event_loop)
        .expect("build window");
    #[cfg(windows)]
    main_hwnd.store(window.hwnd() as isize, Ordering::SeqCst);
    keep_frameless(&window);
    set_square_corners(&window);
    set_window_background(
        &window,
        window_backing_colorref(webview_theme(&settings.appearance.theme)),
    );
    clamp_window_to_work_area(&window);

    let layout_config = LayoutConfig {
        sidebar_expanded_w: settings.sidebar_width,
        sidebar_collapsed_w: 52,
        horizontal_tabs_h: 40,
        toolbar_h: 44,
        ai_sidebar_w: 340,
        min_content_w: 320,
        min_ai_sidebar_w: 280,
        app_sidebar_w: 380,
        app_header_h: 46,
    };

    let mut state = AppState::new(conn, settings, &data_dir, device_id, session_id);
    #[cfg(windows)]
    set_permission_policy(&state.settings);
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
            #[cfg(windows)]
            {
                let reaped = reap_orphan_webview2(&p);
                if reaped > 0 {
                    tracing::warn!(
                        target: "ventus::startup",
                        reaped,
                        "[STARTUP] terminated orphaned msedgewebview2.exe left by the previous crash before building WebViews"
                    );
                }
            }
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
            "[STARTUP] WebView2 profile lock state just before building WebViews"
        );
        std::thread::spawn(|| {
            tracing::info!(
                target: "ventus::startup",
                orphan_msedgewebview2 = count_msedgewebview2_processes(),
                "[STARTUP] orphan msedgewebview2.exe count"
            );
        });
    }

    let browser_args = webview_args(&state.settings);

    std::fs::create_dir_all(&webview_data_dir).expect("create WebView2 profile");
    encrypt_app_storage(&webview_data_dir);
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
    #[cfg(windows)]
    sync_webview_secure_dns_prefs(&webview_data_dir, &state.settings);
    let mut content_web_context = Some(wry::WebContext::new(Some(webview_data_dir.clone())));

    let chrome = {
        const MAX_CHROME_ATTEMPTS: u32 = 60;
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let proxy_chrome = proxy.clone();
            let proxy_chrome_load = proxy.clone();
            let opaque_chrome = cfg!(debug_assertions) && state.settings.dev.opaque_chrome;
            let builder = WebViewBuilder::new_as_child(&window)
                .with_bounds(Rect {
                    x: 0,
                    y: 0,
                    width: win_size.width,
                    height: win_size.height,
                })
                .with_transparent(!opaque_chrome);
            let builder = if opaque_chrome {
                builder.with_background_color((20, 20, 20, 255))
            } else {
                builder
            };
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
    trash_incognito_dir(&incognito_data_dir);
    sweep_incognito_trash(&data_dir);
    std::fs::create_dir_all(&incognito_data_dir).ok();
    encrypt_app_storage(&incognito_data_dir);
    #[cfg(windows)]
    sync_webview_secure_dns_prefs(&incognito_data_dir, &state.settings);
    let mut incognito_web_context = Some(wry::WebContext::new(Some(incognito_data_dir.clone())));

    let mut content_views: HashMap<String, WebView> = HashMap::new();
    let mut app_panel_views: HashMap<String, WebView> = HashMap::new();
    let mut app_panel_suspended: HashSet<String> = HashSet::new();
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
    let mut app_panel_sleep_id = 0u64;
    let mut app_panel_sleep_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut last_active_tab_id: Option<String> = state.tab_manager.active_tab_id.clone();
    let mut sleep_check_at = Instant::now() + TAB_SLEEP_CHECK_EVERY;
    let mut heal_content_at = Instant::now() + HEAL_CONTENT_EVERY;
    #[cfg(windows)]
    let mut heal_last_sig: Option<(Option<String>, (i32, i32, u32, u32), bool)> = None;
    #[cfg(windows)]
    let mut heal_repeats_left: u8 = HEAL_SETTLE_REPEATS;
    let mut error_report_at = Instant::now();
    let mut freeze_report_at = Instant::now();
    let mut renderer_report_at = Instant::now();
    // Pending wake-build (tab_id, url): a switched-to tab with no WebView yet. The actual build
    // is deferred to MainEventsCleared so a burst of switches coalesces to just the final tab.
    let mut pending_wake_build: Option<(String, String)> = None;
    let mut save_id = 0u64;
    let mut sync_id = 0u64;
    let mut sync_dirty = (false, false, false);
    let ubol_dir = ubol_dir(&data_dir);
    let mut incognito_ubol = UbolGate::default();
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
            webview_theme(&state.settings.appearance.theme),
            first_ad_script,
            state.settings.privacy.fingerprint_protection,
            state.fingerprint_seed(first_is_incognito).to_string(),
            state.x_login_compat(&first_tab_id, &first_url),
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
                    &mut incognito_ubol,
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
        load_url_or_gate_incognito_ubol(
            &content_views,
            &tab_id,
            &url,
            &state,
            &mut incognito_ubol,
            ubol_dir.as_deref(),
            &window,
            &proxy,
            &rt,
            &mut incognito_web_context,
            &shared_dl_dir,
            &browser_args,
            &mut load_watches,
            &mut load_watch_next,
        );
    }
    // Apply screenshot protection immediately if the initial workspace is incognito.
    #[cfg(windows)]
    {
        let is_incog = state
            .tab_manager
            .active_workspace()
            .map(|w| w.is_incognito)
            .unwrap_or(false);
        let allow_ss = state.settings.dev.enabled && state.settings.dev.allow_incognito_screenshot;
        set_screenshot_protection(window.hwnd() as isize, is_incog && !allow_ss);
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
    let search_suggestion_client = reqwest::Client::builder()
        .user_agent(crate::version::USER_AGENT)
        .timeout(Duration::from_secs(4))
        .build()
        .ok();
    let mut search_suggestion_task: Option<tokio::task::JoinHandle<()>> = None;
    event_loop.run(move |event, elwt, control_flow| include!("event_loop.rs"));
}

include!("startup.rs");
include!("permissions.rs");
include!("downloads.rs");
include!("native_events.rs");
include!("webviews.rs");
include!("scripts.rs");
include!("feed.rs");
include!("layout.rs");
include!("services.rs");
include!("assistant.rs");
include!("search_answers.rs");
include!("system.rs");
