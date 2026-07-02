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
    let history_user_text = match action.as_str() {
        "summarize" => "Summarize page",
        "explain" => "Explain this topic",
        "key_points" => "Extract key points",
        "ask_anything" => "What can you help with on this page?",
        other => other,
    }
    .to_string();

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
                            let assistant_text = resp.content;
                            let chars = assistant_text.chars().count();
                            let _ = proxy_task.send_event(AppEvent::AiChunk {
                                text: assistant_text.clone(),
                                done: false,
                            });
                            let _ = proxy_task.send_event(AppEvent::AiChunk {
                                text: String::new(),
                                done: true,
                            });
                            let _ = proxy_task.send_event(AppEvent::AiSaveMessages {
                                user_text: history_user_text.clone(),
                                assistant_text,
                                attachments: Vec::new(),
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
                let chars = accumulated.chars().count();
                let _ = proxy_task.send_event(AppEvent::AiSaveMessages {
                    user_text: history_user_text.clone(),
                    assistant_text: accumulated,
                    attachments: Vec::new(),
                });
                tracing::info!(
                    target: "ventus::ai",
                    kind = "quick_action",
                    chars,
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
                        let assistant_text = resp.content;
                        let chars = assistant_text.chars().count();
                        let _ = proxy_task.send_event(AppEvent::AiChunk {
                            text: assistant_text.clone(),
                            done: false,
                        });
                        let _ = proxy_task.send_event(AppEvent::AiChunk {
                            text: String::new(),
                            done: true,
                        });
                        let _ = proxy_task.send_event(AppEvent::AiSaveMessages {
                            user_text: history_user_text.clone(),
                            assistant_text,
                            attachments: Vec::new(),
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
    set_window_background(&window, window_backing_colorref(WebViewTheme::Auto));

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
    attachments: Vec<ai::AiAttachment>,
    state: &AppState,
    chrome: &WebView,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
    ai_generation: &Arc<AtomicUsize>,
) -> std::result::Result<(), Vec<ai::AiAttachment>> {
    let Some(prov) = ai::build_provider(&state.settings) else {
        let _ = chrome.evaluate_script(
            "window.__neura&&window.__neura.showError('No AI provider configured. Add an API key in Settings \u{2192} AI Providers.')"
        );
        return Err(attachments);
    };

    if !prov.supports_native_pdf()
        && attachments.iter().any(|item| {
            item.kind == ai::AiAttachmentKind::Pdf
                && item
                    .text
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
        })
    {
        let _ = chrome.evaluate_script(
            "window.__neura&&window.__neura.showError('This PDF has no readable text. Use Gemini, Anthropic, OpenRouter, or OpenAI Responses to read scanned PDFs.')",
        );
        return Err(attachments);
    }

    let snapshot = build_agent_snapshot(state);
    let pending = state.ai_pending_tools.clone();
    let proxy_agent = proxy.clone();
    let user_text = if text.trim().is_empty() {
        "Analyze the attached files".to_string()
    } else {
        text.clone()
    };
    let saved_attachments = attachments.clone();

    // Build initial message list including conversation history
    let active = state.tab_manager.active_tab();
    let page_ctx = active
        .map(|t| format!("Current page: {} ({})", t.title, t.url))
        .unwrap_or_default();
    let system = format!("{}\n\nYou have access to browser control tools. Use them to help the user interact with the browser and web pages. Always read the page first before clicking elements.\n\n{}", ai::prompts::SYSTEM_PROMPT, page_ctx);

    let mut msgs: Vec<ai::ChatMessage> = vec![ai::ChatMessage::system(system)];
    msgs.extend(state.ai_messages.clone());
    msgs.push(ai::ChatMessage::user_with_attachments(
        user_text.clone(),
        attachments,
    ));

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
    let _ = chrome.evaluate_script(
        "window.__neura&&window.__neura.attachmentsSent&&window.__neura.attachmentsSent()",
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
                        user_text: user_text.clone(),
                        assistant_text: accumulated,
                        attachments: saved_attachments.clone(),
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
                    user_text: user_text.clone(),
                    assistant_text: final_text,
                    attachments: saved_attachments.clone(),
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
            message: "Agent reached maximum steps. Please try a simpler request.".into(),
        });
    });
    Ok(())
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
