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

/// Subscribe to WebView2's native `IsDocumentPlayingAudio` signal for a content tab.
///
/// This is computed by the browser process, so — unlike the JS `tab_audio_state`
/// heartbeat which runs on a `setInterval` that Chromium throttles in hidden tabs — it
/// stays accurate while a tab is backgrounded. It is the authoritative keep-alive signal
/// that stops a tab playing audio (e.g. a YouTube livestream left in another tab) from
/// being suspended by the idle-tab sleeper. Requires `ICoreWebView2_8` (Edge 87+); on
/// older runtimes the cast fails and we fall back to the JS signal + grace window.
#[cfg(windows)]
fn attach_audio_playing_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    tab_id: String,
) {
    use webview2_com::IsDocumentPlayingAudioChangedEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2, ICoreWebView2_8};
    use wv2core::Interface;

    let controller = wv.controller();
    let core: ICoreWebView2 = match unsafe { controller.CoreWebView2() } {
        Ok(c) => c,
        Err(_) => return,
    };
    let core8 = match core.cast::<ICoreWebView2_8>() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(target: "ventus::media", tab = %tab_id, error = %e, "audio keep-alive OFF: ICoreWebView2_8 cast failed (runtime too old)");
            return;
        }
    };

    // Emit the current state once so a tab already producing audio when the handler attaches
    // (e.g. after a deferred wake/rebuild) is registered as media-active right away.
    unsafe {
        let mut playing = Default::default();
        if core8.IsDocumentPlayingAudio(&mut playing).is_ok() {
            let _ = proxy.send_event(AppEvent::ContentAudioPlaying {
                tab_id: tab_id.clone(),
                playing: playing.as_bool(),
            });
        }
    }

    let handler_tab = tab_id.clone();
    let handler =
        IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |sender, _args| {
            if let Some(sender) = sender {
                if let Ok(core8) = sender.cast::<ICoreWebView2_8>() {
                    unsafe {
                        let mut playing = Default::default();
                        if core8.IsDocumentPlayingAudio(&mut playing).is_ok() {
                            tracing::debug!(target: "ventus::media", tab = %handler_tab, playing = playing.as_bool(), "native audio changed event fired");
                            let _ = proxy.send_event(AppEvent::ContentAudioPlaying {
                                tab_id: handler_tab.clone(),
                                playing: playing.as_bool(),
                            });
                        }
                    }
                }
            }
            Ok(())
        }));

    let mut token = Default::default();
    let added = unsafe { core8.add_IsDocumentPlayingAudioChanged(&handler, &mut token) };
    match added {
        Ok(_) => tracing::info!(target: "ventus::media", tab = %tab_id, "audio keep-alive handler attached"),
        Err(e) => tracing::warn!(target: "ventus::media", tab = %tab_id, error = %e, "audio keep-alive OFF: add_IsDocumentPlayingAudioChanged failed"),
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
    app_panel_views: &mut HashMap<String, WebView>,
    content_web_context: &mut Option<wry::WebContext>,
    incognito_web_context: &mut Option<wry::WebContext>,
) {
    #[cfg(windows)]
    let t0 = Instant::now();
    #[cfg(windows)]
    let view_count = content_views.len();
    #[cfg(windows)]
    drain_message_queue_ms(300);
    content_views.clear();
    content_hwnds.clear();
    app_panel_views.clear();
    drop(content_web_context.take());
    drop(incognito_web_context.take());
    #[cfg(windows)]
    let wait_start = Instant::now();
    #[cfg(windows)]
    let free = if crash_sentinel.is_some() {
        drain_message_queue_ms(300);
        wait_for_webview_profiles_released(profile_roots, WEBVIEW_PROFILE_RELEASE_TIMEOUT)
    } else {
        true
    };
    #[cfg(windows)]
    tracing::info!(
        target: "ventus::shutdown",
        views = view_count,
        profile_wait_ms = wait_start.elapsed().as_millis() as u64,
        total_ms = t0.elapsed().as_millis() as u64,
        profile_freed = free,
        "shutdown_webview2 complete"
    );
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

