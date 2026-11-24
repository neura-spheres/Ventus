#[cfg(windows)]
struct NativeDownload {
    op: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadOperation,
    owner: String,
}

/// A download WebView2 has announced but that is still waiting for a save path.
#[cfg(windows)]
struct PendingDownload {
    args: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadStartingEventArgs,
    deferral: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Deferral,
    owner: String,
    url: String,
    name: String,
    referer: String,
    total: Option<u64>,
}

#[cfg(windows)]
impl PendingDownload {
    fn cancel(&self) {
        unsafe {
            let _ = self
                .args
                .SetCancel(wv2win::Win32::Foundation::BOOL::from(true));
            let _ = self.deferral.Complete();
        }
    }
}

#[cfg(windows)]
fn control_download(
    id: &str,
    action: DownloadCtl,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    use std::sync::atomic::Ordering;
    if let Some(ctl) = ACCEL_DOWNLOADS.with(|m| m.borrow().get(id).cloned()) {
        match action {
            DownloadCtl::Pause => ctl.paused.store(true, Ordering::Relaxed),
            DownloadCtl::Resume => ctl.paused.store(false, Ordering::Relaxed),
            DownloadCtl::Cancel => ctl.cancel.store(true, Ordering::Relaxed),
        }
        return;
    }
    let op = DOWNLOAD_OPS.with(|m| m.borrow().get(id).map(|d| d.op.clone()));
    let result = match (&op, action) {
        (None, _) => None,
        (Some(op), DownloadCtl::Pause) => unsafe { op.Pause() }.ok(),
        (Some(op), DownloadCtl::Resume) => unsafe { op.Resume() }.ok(),
        (Some(op), DownloadCtl::Cancel) => unsafe { op.Cancel() }.ok(),
    };
    // Nothing is left to control, so settle the row instead of leaving it stuck.
    if result.is_none() && !matches!(action, DownloadCtl::Pause) {
        DOWNLOAD_OPS.with(|m| m.borrow_mut().remove(id));
        let _ = proxy.send_event(AppEvent::DownloadDone {
            id: id.to_string(),
            success: false,
            canceled: matches!(action, DownloadCtl::Cancel),
            path: None,
            review: None,
        });
    }
}

fn report_settled(
    id: String,
    settled: std::io::Result<browser::downloads::Settled>,
    overwrite: bool,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    use browser::downloads::Settled;
    if let Err(e) = &settled {
        tracing::warn!(target: "ventus::dl", id = %id, error = %e, "finished download could not be moved");
    }
    let success = settled.is_ok();
    let (path, review) = match settled {
        Ok(Settled::Saved(path)) => (Some(path.to_string_lossy().to_string()), None),
        Ok(Settled::Review) => (None, Some(overwrite)),
        Err(_) => (None, None),
    };
    let _ = proxy.send_event(AppEvent::DownloadDone {
        id,
        success,
        canceled: false,
        path,
        review,
    });
}

/// Gives a reviewed file its real name. A file that is only locked goes back to review.
fn keep_download(
    id: String,
    target: std::path::PathBuf,
    overwrite: bool,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    let proxy = proxy.clone();
    std::thread::spawn(move || {
        use browser::downloads::{finish_part, part_path, Settled};
        let part = part_path(&target, &id);
        let settled = match finish_part(&part, &target, overwrite) {
            Err(e) if part.exists() => {
                tracing::warn!(target: "ventus::dl", id = %id, error = %e, "kept file is locked");
                Ok(Settled::Review)
            }
            result => result.map(Settled::Saved),
        };
        report_settled(id, settled, overwrite, &proxy);
    });
}

#[cfg(windows)]
fn cancel_all_downloads() {
    use std::sync::atomic::Ordering;
    ACCEL_DOWNLOADS.with(|m| {
        for ctl in m.borrow().values() {
            ctl.cancel.store(true, Ordering::Relaxed);
        }
    });
    let pending: Vec<_> =
        PENDING_DOWNLOADS.with(|m| m.borrow_mut().drain().map(|(_, p)| p).collect());
    for item in pending {
        item.cancel();
    }
    let ops: Vec<_> =
        DOWNLOAD_OPS.with(|m| m.borrow().values().map(|d| d.op.clone()).collect());
    for op in ops {
        unsafe {
            let _ = op.Cancel();
        }
    }
}

#[cfg(windows)]
fn view_has_download(owner: &str) -> bool {
    DOWNLOAD_OPS.with(|m| m.borrow().values().any(|d| d.owner == owner))
        || PENDING_DOWNLOADS.with(|m| m.borrow().values().any(|d| d.owner == owner))
}

#[cfg(not(windows))]
fn view_has_download(_owner: &str) -> bool {
    false
}

/// Removes a tab's WebView, keeping it hidden and silent until its downloads finish.
fn drop_content_view(views: &mut HashMap<String, WebView>, id: &str) {
    let Some(wv) = views.remove(id) else {
        return;
    };
    #[cfg(windows)]
    if view_has_download(id) {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
        use wv2core::Interface;
        unsafe {
            if let Ok(core) = wv
                .controller()
                .CoreWebView2()
                .and_then(|core| core.cast::<ICoreWebView2_8>())
            {
                let _ = core.SetIsMuted(wv2win::Win32::Foundation::BOOL::from(true));
            }
        }
        let _ = wv.set_visible(false);
        let _ = wv.load_url("about:blank");
        tracing::info!(target: "ventus::dl", tab = %id, "keeping closed tab alive for its download");
        PARKED_VIEWS.with(|m| m.borrow_mut().push((id.to_string(), Box::new(wv))));
    }
}

#[cfg(windows)]
fn release_parked_views() {
    let done: Vec<_> = PARKED_VIEWS.with(|m| {
        let mut all = m.borrow_mut();
        let (keep, done): (Vec<_>, Vec<_>) = all
            .drain(..)
            .partition(|(owner, _)| view_has_download(owner));
        *all = keep;
        done
    });
    drop(done);
}

#[cfg(windows)]
fn attach_download_handler(
    wv: &WebView,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    owner: String,
) {
    use webview2_com::DownloadStartingEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2, ICoreWebView2_4};
    use wv2core::{Interface, PWSTR};

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

    let handler = DownloadStartingEventHandler::create(Box::new(move |sender, args| {
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
            let name = std::path::Path::new(&take_pwstr(rp))
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
                .to_string();
            let source = sender.as_ref().map(|w| webview_source(w)).unwrap_or_default();
            let deferral = args.GetDeferral()?;
            let id = uuid::Uuid::new_v4().to_string();
            tracing::info!(target: "ventus::dl", id = %id, owner = %owner, name = %name, "download requested");
            PENDING_DOWNLOADS.with(|m| {
                m.borrow_mut().insert(
                    id.clone(),
                    PendingDownload {
                        deferral,
                        owner: owner.clone(),
                        url: url.clone(),
                        name,
                        referer: source.clone(),
                        total: read_download_total(&op),
                        args,
                    },
                )
            });
            let _ = proxy.send_event(AppEvent::DownloadNavigation {
                tab_id: owner.clone(),
                url,
                source,
            });
            let _ = proxy.send_event(AppEvent::DownloadRequested { id });
        }
        Ok(())
    }));
    let mut token = Default::default();
    unsafe {
        let _ = webview4.add_DownloadStarting(&handler, &mut token);
    }
}

/// Picks the save path (which may wait on a dialog) and checks whether the link can be split,
/// both off the UI thread.
#[cfg(windows)]
fn prepare_download(
    id: String,
    prefs: DownloadPrefs,
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    let Some((url, name, mut referer, total)) = PENDING_DOWNLOADS.with(|m| {
        m.borrow()
            .get(&id)
            .map(|p| (p.url.clone(), p.name.clone(), p.referer.clone(), p.total))
    }) else {
        return;
    };
    if !referer.starts_with("http") {
        referer.clear();
    }
    let proxy = proxy.clone();
    rt.spawn(async move {
        let overwrite = prefs.ask;
        let pick = tokio::task::spawn_blocking(move || resolve_download_target(&name, &prefs));
        let ua = browser_user_agent();
        let probe = async {
            match total {
                Some(total) if url.starts_with("http") => {
                    browser::accel_download::probe(&url, &ua, &referer, total).await
                }
                _ => None,
            }
        };
        let (target, accel) = tokio::join!(pick, probe);
        let _ = proxy.send_event(AppEvent::DownloadReady {
            id,
            target: target.ok().flatten(),
            overwrite,
            accel,
            referer,
        });
    });
}

#[cfg(windows)]
fn start_download(
    id: String,
    target: Option<std::path::PathBuf>,
    overwrite: bool,
    accel: Option<browser::accel_download::ProbeResult>,
    referer: String,
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    let Some(pending) = PENDING_DOWNLOADS.with(|m| m.borrow_mut().remove(&id)) else {
        return;
    };
    let Some(target) = target else {
        pending.cancel();
        return;
    };
    let Some(probe) = accel else {
        begin_native_download(id, &pending, &target, overwrite, proxy);
        unsafe {
            let _ = pending.deferral.Complete();
        }
        return;
    };
    pending.cancel();
    let ctl = browser::accel_download::AccelControl::new();
    ACCEL_DOWNLOADS.with(|m| m.borrow_mut().insert(id.clone(), ctl.clone()));
    tracing::info!(target: "ventus::dl", id = %id, total = probe.total, "accelerated download started");
    let _ = proxy.send_event(AppEvent::DownloadStarted {
        id: Some(id.clone()),
        url: pending.url.clone(),
        filename: target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| pending.name.clone()),
        path: target.to_string_lossy().to_string(),
        total: Some(probe.total),
    });
    let request = browser::accel_download::Request {
        url: pending.url.clone(),
        probe,
        ua: browser_user_agent(),
        referer,
        part: browser::downloads::part_path(&target, &id),
    };
    rt.spawn(browser::accel_download::run(
        request,
        target,
        overwrite,
        id,
        ctl,
        proxy.clone(),
    ));
}

fn resolve_download_target(
    default_name: &str,
    prefs: &DownloadPrefs,
) -> Option<std::path::PathBuf> {
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
    id: String,
    pending: &PendingDownload,
    target: &std::path::Path,
    overwrite: bool,
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
        let args = &pending.args;
        let part = browser::downloads::part_path(target, &id);
        let (path_pcwstr, _buf) = pcwstr(&part.to_string_lossy());
        let op = args
            .DownloadOperation()
            .and_then(|op| args.SetResultFilePath(path_pcwstr).map(|_| op))
            .and_then(|op| args.SetHandled(BOOL::from(true)).map(|_| op));
        let op = match op {
            Ok(op) => op,
            Err(e) => {
                tracing::warn!(target: "ventus::dl", id = %id, error = %e, "download could not be taken over");
                let _ = args.SetCancel(BOOL::from(true));
                return;
            }
        };
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
        let part_s = part.clone();
        let target_s = target.to_path_buf();
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
                let (proxy, id) = (proxy_s.clone(), id_s.clone());
                let (part, target) = (part_s.clone(), target_s.clone());
                // The move can wait on antivirus, so it stays off the UI thread.
                std::thread::spawn(move || {
                    let settled = browser::downloads::settle(&part, &target, overwrite);
                    report_settled(id, settled, overwrite, &proxy);
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
            let _ = std::fs::remove_file(&part_s);
            let _ = proxy_s.send_event(AppEvent::DownloadDone {
                id: id_s.clone(),
                success: false,
                canceled,
                path: None,
                review: None,
            });
            Ok(())
        }));
        let mut st_tok = Default::default();
        let _ = op.add_StateChanged(&state_handler, &mut st_tok);

        DOWNLOAD_OPS.with(|m| {
            m.borrow_mut().insert(
                id.clone(),
                NativeDownload {
                    op: op.clone(),
                    owner: pending.owner.clone(),
                },
            )
        });
        tracing::info!(target: "ventus::dl", id = %id, filename = %filename, total = total.unwrap_or(0), "download started");
        let _ = proxy.send_event(AppEvent::DownloadStarted {
            id: Some(id),
            url: pending.url.clone(),
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

