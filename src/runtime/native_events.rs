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

#[cfg(windows)]
fn attach_csp_translate_check(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2, ICoreWebView2_2};
    use webview2_com::{NavigationStartingEventHandler, WebResourceResponseReceivedEventHandler};
    use wv2core::{Interface, PCWSTR, PWSTR};

    let controller = wv.controller();
    let webview: ICoreWebView2 = match unsafe { controller.CoreWebView2() } {
        Ok(w) => w,
        Err(_) => return,
    };
    let Ok(wv2) = webview.cast::<ICoreWebView2_2>() else {
        return;
    };

    let nav_url = Arc::new(Mutex::new(String::new()));
    let nav_url_start = Arc::clone(&nav_url);
    let start_handler = NavigationStartingEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            let mut ptr = PWSTR::null();
            args.Uri(&mut ptr)?;
            let url = take_pwstr(ptr);
            if let Some(key) = doc_url_key(&url) {
                if let Ok(mut current) = nav_url_start.lock() {
                    *current = key;
                }
            }
        }
        Ok(())
    }));
    let mut start_token = Default::default();
    unsafe {
        let _ = webview.add_NavigationStarting(&start_handler, &mut start_token);
    }

    let last_checked = Arc::new(Mutex::new(String::new()));
    let nav_url_response = Arc::clone(&nav_url);
    let handler = WebResourceResponseReceivedEventHandler::create(Box::new(move |_sender, args| {
        let Some(args) = args else {
            return Ok(());
        };
        unsafe {
            let Ok(response) = args.Response() else {
                return Ok(());
            };
            let Ok(headers) = response.Headers() else {
                return Ok(());
            };
            let name: Vec<u16> = "Content-Security-Policy"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let name_ptr = PCWSTR(name.as_ptr());
            let mut has = wv2win::Win32::Foundation::BOOL::default();
            if headers.Contains(name_ptr, &mut has).is_err() || !has.as_bool() {
                return Ok(());
            }
            let Ok(request) = args.Request() else {
                return Ok(());
            };
            let mut uptr = PWSTR::null();
            if request.Uri(&mut uptr).is_err() {
                return Ok(());
            }
            let uri = take_pwstr(uptr);
            if !(uri.starts_with("http://") || uri.starts_with("https://")) {
                return Ok(());
            }
            let Some(key) = doc_url_key(&uri) else {
                return Ok(());
            };
            let current = nav_url_response
                .lock()
                .ok()
                .map(|value| value.clone())
                .unwrap_or_default();
            if current != key {
                return Ok(());
            }
            {
                let Ok(mut last) = last_checked.lock() else {
                    return Ok(());
                };
                if *last == key {
                    return Ok(());
                }
                *last = key;
            }
            let mut vptr = PWSTR::null();
            if headers.GetHeader(name_ptr, &mut vptr).is_err() {
                return Ok(());
            }
            let csp = take_pwstr(vptr);
            if app::csp_blocks_translate(&csp) {
                if let Some(host) = url::Url::parse(&uri)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
                {
                    let _ = proxy.send_event(AppEvent::TranslateUnavailable { host, silent: true });
                }
            }
        }
        Ok(())
    }));
    let mut token = Default::default();
    unsafe {
        let _ = wv2.add_WebResourceResponseReceived(&handler, &mut token);
    }
}

#[cfg(windows)]
fn doc_url_key(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    parsed.set_fragment(None);
    Some(parsed.to_string())
}

#[cfg(windows)]
fn attach_title_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    tab_id: String,
) {
    use webview2_com::DocumentTitleChangedEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;

    let controller = wv.controller();
    let webview: ICoreWebView2 = match unsafe { controller.CoreWebView2() } {
        Ok(w) => w,
        Err(_) => return,
    };

    unsafe {
        let title = webview_title(&webview);
        if !title.trim().is_empty() {
            let _ = proxy.send_event(AppEvent::ContentTitle {
                tab_id: tab_id.clone(),
                title,
            });
        }
    }

    let handler = DocumentTitleChangedEventHandler::create(Box::new(move |sender, _args| {
        let Some(sender) = sender else {
            return Ok(());
        };
        let title = unsafe { webview_title(&sender) };
        if !title.trim().is_empty() {
            let _ = proxy.send_event(AppEvent::ContentTitle {
                tab_id: tab_id.clone(),
                title,
            });
        }
        Ok(())
    }));

    let mut token = Default::default();
    unsafe {
        let _ = webview.add_DocumentTitleChanged(&handler, &mut token);
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

            // blob:/data:/filesystem: content is bound to the opener's renderer, so a fresh
            // tab can't reopen it by url. Hand the new WebView over (SetNewWindow) into a real
            // tab instead of letting WebView2 spawn its own bare popup window.
            if needs_window_handoff(&url) {
                if let Ok(deferral) = args.GetDeferral() {
                    PENDING_TAB_HANDOFFS.with(|q| {
                        q.borrow_mut().push(PendingTabHandoff {
                            args: args.clone(),
                            deferral,
                            url: url.clone(),
                            incognito,
                        });
                    });
                    let _ = proxy.send_event(AppEvent::CreateTabFromHandoff);
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

fn needs_window_handoff(url: &str) -> bool {
    url.starts_with("blob:") || url.starts_with("data:") || url.starts_with("filesystem:")
}

#[cfg(windows)]
struct PendingTabHandoff {
    args: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2NewWindowRequestedEventArgs,
    deferral: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Deferral,
    url: String,
    incognito: bool,
}

#[cfg(windows)]
thread_local! {
    static PENDING_TAB_HANDOFFS: std::cell::RefCell<Vec<PendingTabHandoff>> =
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
    theme: WebViewTheme,
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
        theme,
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
    theme: WebViewTheme,
) -> Option<WebView> {
    // No URL and no init script: WebView2 navigates this WebView to the popup target itself
    // once it is handed over via SetNewWindow. Same WebContext + browser args as the opener so
    // the new-window handoff is accepted.
    let builder = WebViewBuilder::new_as_child(window)
        .with_bounds(rect)
        .with_background_color(CONTENT_BG)
        .with_incognito(incognito)
        .with_browser_accelerator_keys(false)
        .with_theme(theme)
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

#[cfg(windows)]
unsafe fn webview_title(
    webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
) -> String {
    let mut ptr = wv2core::PWSTR::null();
    if webview.DocumentTitle(&mut ptr).is_err() {
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

fn capture_content_blur(wv: &WebView, proxy: tao::event_loop::EventLoopProxy<AppEvent>) {
    use base64::Engine;
    use webview2_com::CapturePreviewCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG,
    };
    use wv2win::Win32::Foundation::HGLOBAL;
    use wv2win::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
    use wv2win::Win32::System::Com::{IStream, STREAM_SEEK_END, STREAM_SEEK_SET};

    let webview: ICoreWebView2 = match unsafe { wv.controller().CoreWebView2() } {
        Ok(w) => w,
        Err(_) => return,
    };

    let stream: IStream =
        match unsafe { CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true) } {
            Ok(s) => s,
            Err(_) => return,
        };
    let read_stream = stream.clone();

    let handler = CapturePreviewCompletedHandler::create(Box::new(move |err| {
        if err.is_err() {
            return Ok(());
        }
        let bytes = unsafe {
            let mut len: u64 = 0;
            if read_stream
                .Seek(0, STREAM_SEEK_END, Some(&mut len))
                .is_err()
            {
                return Ok(());
            }
            if read_stream.Seek(0, STREAM_SEEK_SET, None).is_err() {
                return Ok(());
            }
            let mut buf = vec![0u8; len as usize];
            let mut total = 0usize;
            while total < buf.len() {
                let mut read = 0u32;
                let hr = read_stream.Read(
                    buf.as_mut_ptr().add(total) as *mut _,
                    (buf.len() - total) as u32,
                    Some(&mut read),
                );
                if hr.is_err() || read == 0 {
                    break;
                }
                total += read as usize;
            }
            buf.truncate(total);
            buf
        };
        if bytes.is_empty() {
            return Ok(());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let data_url = format!("data:image/jpeg;base64,{}", encoded);
        let _ = proxy.send_event(AppEvent::ContentBlurReady(data_url));
        Ok(())
    }));

    unsafe {
        let _ = webview.CapturePreview(
            COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG,
            &stream,
            &handler,
        );
    }
}
