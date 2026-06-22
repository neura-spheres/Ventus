{
        *control_flow = ControlFlow::Wait;
        let _ = &elwt;
        // Drop-guard times this whole turn and logs/escalates only if it ran long. Created
        // before the match so it covers every arm, including the early-return ones.
        let _evt_timer = MainEventTimer {
            start: Instant::now(),
            label: event_label(&event),
        };

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
                let beta = state.settings.beta_channel;
                rt.spawn(async move {
                    match updater::check_latest(beta).await {
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
                let attachments = std::mem::take(&mut state.pending_ai_attachments);
                if let Err(attachments) = handle_ai_message(
                    text,
                    attachments,
                    &state,
                    &chrome,
                    &proxy_main,
                    &rt,
                    &ai_generation,
                ) {
                    state.pending_ai_attachments = attachments;
                }
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
                    &mut app_panel_views,
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
                            webview_theme(&state.settings.appearance.theme),
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

            Event::UserEvent(AppEvent::CreateTabFromHandoff) => {
                #[cfg(windows)]
                {
                    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;
                    let pendings: Vec<PendingTabHandoff> =
                        PENDING_TAB_HANDOFFS.with(|q| q.borrow_mut().drain(..).collect());
                    for pending in pendings {
                        let tab_id = {
                            let tab = state.tab_manager.new_tab(Some(&pending.url));
                            tab.id.clone()
                        };
                        let is_incog = pending.incognito;
                        let ctx = if is_incog {
                            incognito_web_context.as_mut().unwrap()
                        } else {
                            content_web_context.as_mut().unwrap()
                        };
                        let ad_script = state.ad_block_engine.init_script().to_string();
                        let built = build_content_webview(
                            &window,
                            &tab_id,
                            &pending.url,
                            AppLayout::calculate(
                                layout_size(&window, &state),
                                window.scale_factor(),
                                &state,
                                &layout_config,
                            )
                            .content,
                            proxy_main.clone(),
                            ctx,
                            is_incog,
                            std::sync::Arc::clone(&shared_dl_dir),
                            tab_zoom(&state, &tab_id),
                            &browser_args,
                            webview_theme(&state.settings.appearance.theme),
                            ad_script,
                            state.settings.privacy.fingerprint_protection,
                            state.fingerprint_seed(is_incog).to_string(),
                            state.x_login_compat(&tab_id, &pending.url),
                            state.settings.privacy.strict_permissions,
                            state.settings.privacy.site_permissions.clone(),
                            state.settings.privacy.default_permissions.clone(),
                            state.settings.privacy.https_only,
                            false,
                        );
                        let wv = match built {
                            Ok(wv) => wv,
                            Err(e) => {
                                unsafe {
                                    let _ = pending.deferral.Complete();
                                }
                                state.tab_manager.close_tab(&tab_id);
                                tracing::error!("handoff tab build: {}", e);
                                continue;
                            }
                        };
                        let ok = unsafe {
                            match wv.controller().CoreWebView2() {
                                Ok(core) => {
                                    let core: ICoreWebView2 = core;
                                    pending.args.SetNewWindow(&core).is_ok()
                                }
                                Err(_) => false,
                            }
                        };
                        unsafe {
                            let _ = pending.deferral.Complete();
                        }
                        if !ok {
                            state.tab_manager.close_tab(&tab_id);
                            tracing::warn!("handoff tab SetNewWindow failed");
                            continue;
                        }
                        let hwnd = webview_hwnd(&wv);
                        content_views.insert(tab_id.clone(), wv);
                        track_content_hwnd(hwnd, &tab_id, &mut content_hwnds);
                        state.push_state_to_chrome(&chrome);
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

            Event::UserEvent(AppEvent::AppPanelSleep { id }) => {
                if id == app_panel_sleep_id && !state.app_sidebar_open {
                    app_panel_sleep_id = 0;
                    app_panel_sleep_task = None;
                    let count = app_panel_views.len();
                    app_panel_views.clear();
                    app_panel_suspended.clear();
                    tracing::debug!("app_panel_sleep: released {} view(s)", count);
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
                // Auto-report renderer crashes (cooldown-gated so a crash loop can't spam). A
                // fatal exit vs. an unresponsive renderer are reported as distinct kinds.
                if state.settings.privacy.auto_crash_report
                    && cloud::config::is_configured()
                    && Instant::now() >= renderer_report_at
                {
                    renderer_report_at = Instant::now() + RENDERER_REPORT_COOLDOWN;
                    let kind = if fatal {
                        "renderer_crash"
                    } else {
                        "renderer_unresponsive"
                    };
                    let msg = format!(
                        "content process failed (fatal={}) on {}",
                        fatal,
                        crate::utils::url::log_url(&url)
                    );
                    let report = app::build_report(&state, kind, msg, String::new());
                    rt.spawn(async move {
                        let _ = cloud::report::send(report).await;
                    });
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
                    webview_theme(&state.settings.appearance.theme),
                    ad_script,
                    state.settings.privacy.fingerprint_protection,
                    state.fingerprint_seed(is_incog).to_string(),
                    state.x_login_compat(&tab_id, &url),
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
                let theme_changed = matches!(
                    &app_event,
                    AppEvent::Chrome(ChromeCommand::SaveSettings { key, .. }) if key == "theme"
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
                #[cfg(windows)]
                let permission_settings_changed = matches!(
                    &app_event,
                    AppEvent::Chrome(
                        ChromeCommand::SetSitePermission { .. }
                            | ChromeCommand::SetDefaultPermission { .. }
                            | ChromeCommand::PermissionDecision { .. }
                    ) | AppEvent::SyncPulled { .. }
                ) || matches!(
                    &app_event,
                    AppEvent::Chrome(ChromeCommand::SaveSettings { key, .. })
                        if key == "strict_permissions"
                );
                let cover_before = state.content_cover_open;
                let app_was_open = state.app_sidebar_open;
                let action_opt = handle_app_event_inner(app_event, &mut state, &chrome);
                if app_was_open && !state.app_sidebar_open {
                    if let Some(task) = app_panel_sleep_task.take() {
                        task.abort();
                    }
                    app_panel_sleep_task = Some(arm_app_panel_sleep(
                        &rt,
                        &proxy_main,
                        &mut app_panel_sleep_id,
                    ));
                } else if !app_was_open && state.app_sidebar_open {
                    if let Some(task) = app_panel_sleep_task.take() {
                        task.abort();
                    }
                    app_panel_sleep_id = app_panel_sleep_id.wrapping_add(1).max(1);
                }
                #[cfg(windows)]
                if theme_changed {
                    let theme = webview_theme(&state.settings.appearance.theme);
                    for wv in content_views.values() {
                        let _ = wv.set_theme(theme);
                    }
                    for popup in popups.values() {
                        let _ = popup.content.set_theme(theme);
                    }
                }
                #[cfg(not(windows))]
                let _ = theme_changed;
                #[cfg(windows)]
                if permission_settings_changed {
                    set_permission_policy(&state.settings);
                }
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
                    if let TabAction::FetchSearchSuggestions {
                        q,
                        id,
                        engine,
                        region,
                    } = &action
                    {
                        if let Some(task) = search_suggestion_task.take() {
                            task.abort();
                        }
                        if let Some(client) = search_suggestion_client.clone() {
                            let q = q.clone();
                            let id = *id;
                            let engine = engine.clone();
                            let region = region.clone();
                            let proxy = proxy_main.clone();
                            search_suggestion_task = Some(rt.spawn(async move {
                                let items = browser::omnibox::fetch_queries(
                                    &client, &engine, &q, &region,
                                )
                                .await
                                .unwrap_or_default();
                                let _ = proxy.send_event(AppEvent::SearchSuggestionsLoaded {
                                    q,
                                    id,
                                    items,
                                });
                            }));
                        }
                        return;
                    }
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
                    sync_app_panel_views(
                        &app_panel_views,
                        &mut app_panel_suspended,
                        &state,
                        &window,
                        &layout_config,
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
                        TabAction::FetchSearchSuggestions { .. } => unreachable!(),
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
                                    webview_theme(&state.settings.appearance.theme),
                                    ad_script,
                                    state.settings.privacy.fingerprint_protection,
                                    state.fingerprint_seed(is_incog).to_string(),
                                    state.x_login_compat(&tab_id, &url),
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
                                                webview_theme(&state.settings.appearance.theme),
                                                ad_script,
                                                state.settings.privacy.fingerprint_protection,
                                                state.fingerprint_seed(is_incog).to_string(),
                                                state.x_login_compat(&active_id, &url),
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
                        TabAction::ShowErrorPage { tab_id } => {
                            content_views.remove(&tab_id);
                            content_hwnds.remove(&tab_id);
                            suspended_tabs.remove(&tab_id);
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
                        TabAction::WriteClipboardText(text) => {
                            let _ = write_clipboard_text(&text);
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
                        TabAction::SetDefaultZoom(level) => {
                            for (id, wv) in &content_views {
                                if !state.zoom_levels.contains_key(id) {
                                    let _ = wv.zoom(level);
                                }
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
                                        webview_theme(&state.settings.appearance.theme),
                                        ad_script,
                                        state.settings.privacy.fingerprint_protection,
                                        state.fingerprint_seed(is_incog).to_string(),
                                        state.x_login_compat(id, &url),
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
                                    webview_theme(&state.settings.appearance.theme),
                                    ad_script,
                                    state.settings.privacy.fingerprint_protection,
                                    state.fingerprint_seed(is_incog).to_string(),
                                    state.x_login_compat(&tab_id, &url),
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
                                // The tab has no WebView yet (restored/discarded). Building one is
                                // a ~400 ms synchronous, UI-thread-bound WebView2 operation, so
                                // doing it inline froze the UI on every such switch. Defer it:
                                // record the request and show the loading cover now (cheap), then
                                // let the coalescer in MainEventsCleared build only the tab that is
                                // still active once the switch burst settles — clicking through
                                // restored tabs no longer builds every tab you pass over.
                                state.tab_manager.wake_tab(&tab_id);
                                state.set_content_cover(&chrome, true);
                                state.push_state_to_chrome(&chrome);
                                pending_wake_build = Some((tab_id.clone(), url.clone()));
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
                                webview_theme(&state.settings.appearance.theme),
                                ad_script,
                                state.settings.privacy.fingerprint_protection,
                                state.fingerprint_seed(is_incog).to_string(),
                                state.x_login_compat(&tab_id, &url),
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
                                    &mut app_panel_views,
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
                                &mut app_panel_views,
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
                        TabAction::AppPanelSelect { url } => {
                            if !app_panel_views.contains_key(&url) {
                                let panel_id = app_panel_id(&url);
                                let rect = layout.app_panel_body.unwrap_or(layout.content);
                                let ctx = content_web_context.as_mut().unwrap();
                                let ad_script = state.ad_block_engine.init_script().to_string();
                                match build_content_webview(
                                    &window,
                                    &panel_id,
                                    &url,
                                    rect,
                                    proxy_main.clone(),
                                    ctx,
                                    false,
                                    std::sync::Arc::clone(&shared_dl_dir),
                                    state.settings.appearance.zoom_level,
                                    &browser_args,
                                    webview_theme(&state.settings.appearance.theme),
                                    ad_script,
                                    state.settings.privacy.fingerprint_protection,
                                    state.fingerprint_seed(false).to_string(),
                                    false,
                                    state.settings.privacy.strict_permissions,
                                    state.settings.privacy.site_permissions.clone(),
                                    state.settings.privacy.default_permissions.clone(),
                                    state.settings.privacy.https_only,
                                    true,
                                ) {
                                    Ok(wv) => {
                                        app_panel_views.insert(url.clone(), wv);
                                    }
                                    Err(e) => tracing::error!("create app panel view: {}", e),
                                }
                            }
                            sync_app_panel_views(
                                &app_panel_views,
                                &mut app_panel_suspended,
                                &state,
                                &window,
                                &layout_config,
                            );
                            if let Some(wv) = app_panel_views.get(&url) {
                                wake_content_webview(wv);
                            }
                        }
                        TabAction::AppPanelReload => {
                            if let Some(url) = state.app_panel_active.as_ref() {
                                if let Some(wv) = app_panel_views.get(url) {
                                    let _ = wv.evaluate_script("location.reload()");
                                }
                            }
                        }
                        TabAction::AppPanelCloseView => {
                            sync_app_panel_views(
                                &app_panel_views,
                                &mut app_panel_suspended,
                                &state,
                                &window,
                                &layout_config,
                            );
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
                                let is_neura = tab.is_neura_page();
                                let loading =
                                    tab.status == crate::browser::tab::TabStatus::Loading;
                                let missing = tab_needs_content(tab, content_views.contains_key(id));
                                if missing {
                                    state.tab_manager.wake_tab(id);
                                    state.set_content_cover(&chrome, true);
                                    state.push_state_to_chrome(&chrome);
                                    pending_wake_build = Some((id.clone(), url));
                                } else if loading && !url.trim().is_empty() && !is_neura {
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
                // Build at most one deferred wake-tab per drained event batch, and only if it is
                // still the active tab. Rapid tab switching enqueues several requests; they all
                // overwrite `pending_wake_build`, so here we pay the ~400 ms WebView2 build once,
                // for the tab the user actually landed on, instead of once per tab skipped over.
                if let Some((wake_id, wake_url)) = pending_wake_build.take() {
                    let still_active =
                        state.tab_manager.active_tab_id.as_deref() == Some(wake_id.as_str());
                    if still_active && !content_views.contains_key(&wake_id) {
                        build_woken_content_tab(
                            &wake_id,
                            &wake_url,
                            &window,
                            &chrome,
                            chrome_hwnd,
                            &layout_config,
                            &proxy_main,
                            &rt,
                            &mut state,
                            &mut content_views,
                            &mut content_hwnds,
                            &mut content_web_context,
                            &mut incognito_web_context,
                            &shared_dl_dir,
                            &browser_args,
                            &startup_cookies,
                            &mut cookies_restored,
                            &mut load_watches,
                            &mut load_watch_next,
                        );
                    }
                }
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
                    // Keep the media keep-alive timestamp fresh for any tab currently producing
                    // audio (native browser signal) or showing a large playing video (JS). The
                    // MEDIA_GRACE_MS window below then keeps it awake briefly after media truly
                    // stops, so a transient drop can't get a backgrounded livestream suspended.
                    for t in state.tab_manager.tabs.iter_mut() {
                        if t.native_audio || t.is_media_active {
                            t.last_media_active_at = now_ms;
                        }
                    }
                    let media_keep = |t: &crate::browser::tab::Tab| {
                        t.native_audio
                            || t.is_media_active
                            || (now_ms - t.last_media_active_at) < MEDIA_GRACE_MS
                    };
                    let to_suspend: Vec<String> = state
                        .tab_manager
                        .tabs
                        .iter()
                        .filter(|t| {
                            t.id != active
                                && !t.is_neura_page()
                                && t.status != crate::browser::tab::TabStatus::Loading
                                && !media_keep(t)
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
                                    && !media_keep(t)
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
                                    && !media_keep(t)
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
                        state.tab_manager.discard_tab(&id);
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
                {
                    if let Some(kind) = utils::log_buffer::take_auto_pending() {
                        error_report_at = Instant::now() + ERROR_REPORT_COOLDOWN;
                        let (kind, msg) = match kind {
                            utils::log_buffer::AutoLogKind::Error => {
                                ("error", "Automatic error report")
                            }
                            utils::log_buffer::AutoLogKind::Warning => {
                                ("warning", "Automatic warning report")
                            }
                        };
                        let report = app::build_report(&state, kind, msg.into(), String::new());
                        rt.spawn(async move {
                            match cloud::report::send(report).await {
                                Ok(()) => tracing::info!(target: "ventus::report", "automatic report uploaded"),
                                Err(e) => tracing::warn!(target: "ventus::report", error = %e, "automatic report upload failed"),
                            }
                        });
                    }
                }
                // A handler blocked the UI long enough to count as a real freeze: report it
                // (cooldown-gated) with the offending event + duration. The ride-along logs make
                // the cause identifiable after the fact.
                if state.settings.privacy.auto_crash_report
                    && cloud::config::is_configured()
                    && Instant::now() >= freeze_report_at
                {
                    let frozen = WORST_FREEZE.lock().ok().and_then(|mut g| g.take());
                    if let Some((ms, label)) = frozen {
                        freeze_report_at = Instant::now() + FREEZE_REPORT_COOLDOWN;
                        let msg = format!(
                            "Main-thread freeze: '{}' blocked the UI for {} ms",
                            label, ms
                        );
                        let report = app::build_report(&state, "freeze", msg, String::new());
                        rt.spawn(async move {
                            let _ = cloud::report::send(report).await;
                        });
                    }
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
                sync_app_panel_views(
                    &app_panel_views,
                    &mut app_panel_suspended,
                    &state,
                    &window,
                    &layout_config,
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
                sync_app_panel_views(
                    &app_panel_views,
                    &mut app_panel_suspended,
                    &state,
                    &window,
                    &layout_config,
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
                    &mut app_panel_views,
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
    }