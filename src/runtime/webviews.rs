/// Create a content WebView, retrying briefly when WebView2 reports the user-data profile
/// is locked / in use (HRESULT 0x800700AA, ERROR_BUSY).

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
    theme: WebViewTheme,
    ad_block_script: String,
    fingerprint: bool,
    fingerprint_seed: String,
    x_login_compat: bool,
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
            theme,
            ad_block_script.clone(),
            fingerprint,
            fingerprint_seed.clone(),
            x_login_compat,
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
    theme: WebViewTheme,
    ad_block_script: String,
    fingerprint: bool,
    fingerprint_seed: String,
    x_login_compat: bool,
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
            &fingerprint_seed,
            x_login_compat,
            strict,
            &site_permissions,
            &default_permissions,
        ));
    #[cfg(windows)]
    let builder = builder
        .with_browser_accelerator_keys(false)
        .with_theme(theme)
        .with_additional_browser_args(browser_args.to_string());
    #[cfg(not(windows))]
    let _ = theme;
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
                if value.get("cmd").and_then(|v| v.as_str()) == Some("copy_image_data") {
                    let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    let data = value
                        .get("data")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let src = value
                        .get("src")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let _ = proxy_ipc.send_event(AppEvent::CopyImageData { ok, data, src });
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
                if value.get("cmd").and_then(|v| v.as_str()) == Some("translate_unavailable") {
                    let host = value
                        .get("host")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let silent = value
                        .get("silent")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if !host.is_empty() {
                        let _ = proxy_ipc
                            .send_event(AppEvent::TranslateUnavailable { host, silent });
                    }
                    return;
                }
                if value.get("cmd").and_then(|v| v.as_str()) == Some("translate_available") {
                    let host = value
                        .get("host")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    if !host.is_empty() {
                        let _ = proxy_ipc.send_event(AppEvent::TranslateAvailable { host });
                    }
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
        attach_client_hints_rewrite(&wv);
        if !incognito {
            attach_csp_translate_check(&wv, proxy.clone());
        }
        attach_title_handler(&wv, proxy.clone(), tab_id.to_string());
        attach_new_window_handler(&wv, proxy.clone(), incognito);
        attach_permission_handler(&wv, proxy.clone(), site_permissions.clone());
        attach_download_handler(&wv, proxy.clone(), std::sync::Arc::clone(&download_dir));
        attach_audio_playing_handler(&wv, proxy.clone(), tab_id.to_string());
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

fn app_panel_id(url: &str) -> String {
    format!("__app__:{}", url)
}

fn sync_app_panel_views(
    app_panel_views: &HashMap<String, WebView>,
    suspended: &mut HashSet<String>,
    state: &AppState,
    window: &tao::window::Window,
    config: &LayoutConfig,
) {
    if app_panel_views.is_empty() {
        return;
    }
    let layout = AppLayout::calculate(
        layout_size(window, state),
        window.scale_factor(),
        state,
        config,
    );
    let active = if state.app_sidebar_open {
        state.app_panel_active.as_deref()
    } else {
        None
    };
    for (url, wv) in app_panel_views {
        if Some(url.as_str()) == active {
            if let Some(rect) = layout.app_panel_body {
                set_content_bounds(wv, rect);
                let _ = wv.set_visible(true);
                if suspended.remove(url) {
                    resume_content_webview(wv);
                }
            }
        } else if !suspended.contains(url) && sleep_content_webview(wv) {
            suspended.insert(url.clone());
        }
    }
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
    // Mark the start of this app-initiated load (cold-start, omnibox, link open). The
    // ContentLoadStart handler takes an early-return for an already-loading tab, so without
    // this the load-duration timer would never start for these navigations.
    state
        .nav_started_at
        .insert(tab_id.to_string(), std::time::Instant::now());
    if state.tab_manager.active_tab_id.as_deref() == Some(tab_id) {
        state.set_content_cover(chrome, true);
        let _ = chrome.evaluate_script("window.__neura && window.__neura.startLoadProgress()");
    }
}

#[allow(clippy::too_many_arguments)]
fn build_woken_content_tab(
    tab_id: &str,
    url: &str,
    window: &tao::window::Window,
    chrome: &WebView,
    chrome_hwnd: Option<isize>,
    layout_config: &LayoutConfig,
    proxy_main: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
    state: &mut AppState,
    content_views: &mut HashMap<String, WebView>,
    content_hwnds: &mut HashMap<String, isize>,
    content_web_context: &mut Option<wry::WebContext>,
    incognito_web_context: &mut Option<wry::WebContext>,
    shared_dl_dir: &std::sync::Arc<std::sync::Mutex<DownloadPrefs>>,
    browser_args: &str,
    startup_cookies: &[cookie_store::CookieRecord],
    cookies_restored: &mut bool,
    load_watches: &mut HashMap<String, u64>,
    load_watch_next: &mut u64,
) {
    if content_views.contains_key(tab_id) {
        return;
    }
    let is_incog = state.tab_manager.tab_is_incognito(tab_id);
    let layout = AppLayout::calculate(
        layout_size(window, state),
        window.scale_factor(),
        state,
        layout_config,
    );
    let ad_script = state.ad_block_engine.init_script().to_string();
    let ctx = if is_incog {
        incognito_web_context.as_mut().unwrap()
    } else {
        content_web_context.as_mut().unwrap()
    };
    match build_content_webview(
        window,
        tab_id,
        url,
        layout.content,
        proxy_main.clone(),
        ctx,
        is_incog,
        std::sync::Arc::clone(shared_dl_dir),
        tab_zoom(state, tab_id),
        browser_args,
        webview_theme(&state.settings.appearance.theme),
        ad_script,
        state.settings.privacy.fingerprint_protection,
        state.fingerprint_seed(is_incog).to_string(),
        state.x_login_compat(tab_id, url),
        state.settings.privacy.strict_permissions,
        state.settings.privacy.site_permissions.clone(),
        state.settings.privacy.default_permissions.clone(),
        state.settings.privacy.https_only,
        false,
    ) {
        Ok(wv) => {
            #[cfg(windows)]
            let hwnd = webview_hwnd(&wv);
            restore_startup_cookies(&wv, is_incog, startup_cookies, cookies_restored);
            content_views.insert(tab_id.to_string(), wv);
            state.tab_manager.wake_tab(tab_id);
            begin_native_load(state, chrome, tab_id);
            state.push_state_to_chrome(chrome);
            state.load_recoveries.remove(&app::load_key(tab_id, url));
            #[cfg(windows)]
            track_content_hwnd(hwnd, tab_id, content_hwnds);
            #[cfg(not(windows))]
            let _ = &mut *content_hwnds;
            apply_layout(
                chrome,
                chrome_hwnd,
                content_views,
                state,
                layout_config,
                window,
            );
            if let Some(wv) = content_views.get(tab_id) {
                let _ = wv.focus();
                let _ = wv.load_url(url);
            }
            tracing::info!(
                target: "ventus::session",
                tab = %tab_id,
                url = %url,
                "[SESSION] woke sleeping/restored tab: content WebView built OK (deferred)"
            );
            watch_load(
                rt,
                proxy_main,
                load_watches,
                load_watch_next,
                tab_id.to_string(),
                url.to_string(),
            );
        }
        Err(e) => tracing::error!(
            target: "ventus::session",
            tab = %tab_id,
            url = %url,
            error = %e,
            "[SESSION] deferred wake build FAILED — tab stays black. 0x800700AA = profile still locked"
        ),
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
    let total_mb = crate::utils::sysinfo::total_memory_mb();
    let low_spec = total_mb > 0 && total_mb <= 4096;
    let very_low_spec = total_mb > 0 && total_mb <= 2048;
    let (disk_cache, media_cache) = if low_spec {
        (268435456u64, 134217728u64)
    } else {
        (1073741824u64, 536870912u64)
    };
    let mut args = vec![
        "--no-first-run".to_string(),
        format!("--disk-cache-size={}", disk_cache),
        format!("--media-cache-size={}", media_cache),
    ];
    if very_low_spec {
        args.push("--renderer-process-limit=4".to_string());
    }
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
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; Ventus) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{reduced} Safari/537.36"
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
