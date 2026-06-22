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

