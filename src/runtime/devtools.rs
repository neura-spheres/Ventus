#[cfg(windows)]
mod native_inspector {
    use super::*;
    use std::cell::RefCell;
    use webview2_com::{
        CallDevToolsProtocolMethodCompletedHandler, DevToolsProtocolEventReceivedEventHandler,
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2, ICoreWebView2DevToolsProtocolEventReceivedEventArgs2,
            ICoreWebView2DevToolsProtocolEventReceiver, ICoreWebView2_11,
        },
    };
    use wv2core::{Interface, PCWSTR, PWSTR};

    const FRONTEND: &str = "https://msedgedevtools.microsoft.com/serve_file/@af2ba23ae0ecf7084d900446fc59d5119dd5fd5e/";
    type Proxy = tao::event_loop::EventLoopProxy<AppEvent>;
    type Token = wv2win::Win32::System::WinRT::EventRegistrationToken;

    struct Inspector {
        view: WebView,
        core: ICoreWebView2,
        receivers: Vec<(ICoreWebView2DevToolsProtocolEventReceiver, Token)>,
        generation: String,
        hwnd: Option<isize>,
        inspected_hwnd: Option<isize>,
        ready: bool,
        inspect: bool,
        picker: bool,
    }

    impl Drop for Inspector {
        fn drop(&mut self) {
            for (receiver, token) in &self.receivers {
                let _ = unsafe { receiver.remove_DevToolsProtocolEventReceived(*token) };
            }
            let handler =
                CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_, _| Ok(())));
            for method in [
                "Debugger.disable",
                "Overlay.disable",
                "CSS.disable",
                "DOM.disable",
                "Network.disable",
                "Runtime.disable",
            ] {
                let method = wide(method);
                let params = wide("{}");
                let _ = unsafe {
                    self.core.CallDevToolsProtocolMethod(
                        PCWSTR(method.as_ptr()),
                        PCWSTR(params.as_ptr()),
                        &handler,
                    )
                };
            }
        }
    }

    thread_local! {
        static PANES: RefCell<HashMap<String, Inspector>> = RefCell::new(HashMap::new());
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    pub(super) fn open(
        inspected: &WebView,
        window: &tao::window::Window,
        tab_id: &str,
        context: &mut wry::WebContext,
        proxy: &Proxy,
        inspect: bool,
        picker: bool,
    ) -> anyhow::Result<()> {
        if PANES.with(|panes| panes.borrow().contains_key(tab_id)) {
            PANES.with(|panes| {
                if let Some(pane) = panes.borrow_mut().get_mut(tab_id) {
                    pane.inspect |= inspect;
                    pane.picker |= picker;
                }
            });
            activate(tab_id, proxy);
            return Ok(());
        }
        let core = unsafe { inspected.controller().CoreWebView2()? };
        let generation = uuid::Uuid::new_v4().to_string();
        let tab = tab_id.to_string();
        let gen = generation.clone();
        let ipc_proxy = proxy.clone();
        let view = WebViewBuilder::new_as_child(window)
            .with_web_context(context)
            .with_bounds(Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            })
            .with_visible(false)
            .with_devtools(cfg!(debug_assertions))
            .with_initialization_script(include_str!("../ui/devtools-bridge.js"))
            .with_navigation_handler(|url| url.starts_with(FRONTEND))
            .with_new_window_req_handler(|_| false)
            .with_ipc_handler(move |request| {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(request.body()) else {
                    return;
                };
                let event = match value.get("type").and_then(|v| v.as_str()) {
                    Some("ready") => AppEvent::DevtoolsReady {
                        tab_id: tab.clone(),
                        generation: gen.clone(),
                    },
                    Some("protocol") => {
                        let Some(message) = value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .filter(|s| s.len() <= 16 * 1024 * 1024)
                        else {
                            return;
                        };
                        AppEvent::DevtoolsCommand {
                            tab_id: tab.clone(),
                            generation: gen.clone(),
                            message: message.to_string(),
                        }
                    }
                    _ => return,
                };
                let _ = ipc_proxy.send_event(event);
            })
            .with_url(format!(
                "{FRONTEND}inspector.html?ws=ventus-devtools/page&panel=elements"
            ))
            .build()?;
        let mut receivers = Vec::new();
        let events: Vec<String> =
            serde_json::from_str(include_str!("../../assets/devtools-events.json"))?;
        for method in events {
            let method_wide = wide(&method);
            let Ok(receiver) =
                (unsafe { core.GetDevToolsProtocolEventReceiver(PCWSTR(method_wide.as_ptr())) })
            else {
                continue;
            };
            let tab = tab_id.to_string();
            let gen = generation.clone();
            let event_proxy = proxy.clone();
            let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    unsafe {
                        let mut params = PWSTR::null();
                        args.ParameterObjectAsJson(&mut params)?;
                        let text = params.to_string().unwrap_or_else(|_| "{}".into());
                        wv2win::Win32::System::Com::CoTaskMemFree(Some(params.0.cast()));
                        let mut message = serde_json::json!({"method": method, "params": serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default()});
                        if let Ok(args2) =
                            args.cast::<ICoreWebView2DevToolsProtocolEventReceivedEventArgs2>()
                        {
                            let mut session = PWSTR::null();
                            if args2.SessionId(&mut session).is_ok() {
                                let id = session.to_string().unwrap_or_default();
                                wv2win::Win32::System::Com::CoTaskMemFree(Some(session.0.cast()));
                                if !id.is_empty() {
                                    message["sessionId"] = id.into();
                                }
                            }
                        }
                        let _ = event_proxy.send_event(AppEvent::DevtoolsEvent {
                            tab_id: tab.clone(),
                            generation: gen.clone(),
                            message: message.to_string(),
                        });
                    }
                    Ok(())
                },
            ));
            let mut token = Token::default();
            unsafe {
                receiver.add_DevToolsProtocolEventReceived(&handler, &mut token)?;
            }
            receivers.push((receiver, token));
        }
        let hwnd = webview_hwnd(&view);
        PANES.with(|panes| {
            panes.borrow_mut().insert(
                tab_id.to_string(),
                Inspector {
                    view,
                    core,
                    receivers,
                    generation,
                    hwnd,
                    inspected_hwnd: webview_hwnd(inspected),
                    ready: false,
                    inspect,
                    picker,
                },
            );
        });
        Ok(())
    }

    pub(super) fn command(tab: &str, generation: &str, message: &str, proxy: &Proxy) {
        let Some(core) = PANES.with(|panes| {
            panes
                .borrow()
                .get(tab)
                .filter(|p| p.generation == generation)
                .map(|p| p.core.clone())
        }) else {
            return;
        };
        let Ok(request) = serde_json::from_str::<serde_json::Value>(message) else {
            return;
        };
        let Some(method) = request.get("method").and_then(|v| v.as_str()) else {
            return;
        };
        let Some(id) = request.get("id").cloned() else {
            return;
        };
        let session = request
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let callback_tab = tab.to_string();
        let callback_gen = generation.to_string();
        let callback_proxy = proxy.clone();
        let response_id = id.clone();
        let response_session = session.to_string();
        let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
            move |result, payload| {
                let mut message = match result {
                    Ok(()) => {
                        serde_json::json!({"id": response_id, "result": serde_json::from_str::<serde_json::Value>(&payload).unwrap_or_else(|_| serde_json::json!({}))})
                    }
                    Err(error) => {
                        serde_json::json!({"id": response_id, "error": {"code": -32000, "message": error.to_string()}})
                    }
                };
                if !response_session.is_empty() {
                    message["sessionId"] = response_session.clone().into();
                }
                let _ = callback_proxy.send_event(AppEvent::DevtoolsEvent {
                    tab_id: callback_tab.clone(),
                    generation: callback_gen.clone(),
                    message: message.to_string(),
                });
                Ok(())
            },
        ));
        let method = wide(method);
        let params = wide(
            &request
                .get("params")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}))
                .to_string(),
        );
        let result = unsafe {
            if session.is_empty() {
                core.CallDevToolsProtocolMethod(
                    PCWSTR(method.as_ptr()),
                    PCWSTR(params.as_ptr()),
                    &handler,
                )
            } else {
                let session = wide(session);
                core.cast::<ICoreWebView2_11>().and_then(|core| {
                    core.CallDevToolsProtocolMethodForSession(
                        PCWSTR(session.as_ptr()),
                        PCWSTR(method.as_ptr()),
                        PCWSTR(params.as_ptr()),
                        &handler,
                    )
                })
            }
        };
        if let Err(error) = result {
            let _ = proxy.send_event(AppEvent::DevtoolsEvent { tab_id: tab.into(), generation: generation.into(), message: serde_json::json!({"id": id, "error": {"code": -32000, "message": error.to_string()}}).to_string() });
        }
    }

    pub(super) fn receive(tab: &str, generation: &str, message: &str) {
        if serde_json::from_str::<serde_json::Value>(message)
            .ok()
            .and_then(|value| value.get("id").and_then(|id| id.as_u64()))
            .is_some_and(|id| id >= 2_000_000_000)
        {
            return;
        }
        PANES.with(|panes| {
            if let Some(pane) = panes
                .borrow()
                .get(tab)
                .filter(|p| p.generation == generation)
            {
                let _ = pane.view.evaluate_script(&format!(
                    "window.__ventusDevtoolsReceive&&window.__ventusDevtoolsReceive({})",
                    message
                ));
            }
        });
    }

    pub(super) fn ready(tab: &str, generation: &str, proxy: &Proxy) {
        PANES.with(|panes| {
            if let Some(pane) = panes
                .borrow_mut()
                .get_mut(tab)
                .filter(|p| p.generation == generation)
            {
                pane.ready = true;
            }
        });
        activate(tab, proxy);
    }

    fn activate(tab: &str, proxy: &Proxy) {
        let pending = PANES.with(|panes| {
            let mut panes = panes.borrow_mut();
            let pane = panes.get_mut(tab)?;
            if !pane.ready {
                return None;
            }
            let result = (pane.generation.clone(), pane.inspect, pane.picker);
            pane.inspect = false;
            pane.picker = false;
            Some(result)
        });
        let Some((generation, inspect, picker)) = pending else {
            return;
        };
        if inspect {
            command(tab, &generation, &serde_json::json!({"id": 2_000_000_001u32, "method": "Runtime.evaluate", "params": {"expression": "inspect(window.__ventusInspectTarget || document.documentElement)", "includeCommandLineAPI": true}}).to_string(), proxy);
        }
        if picker {
            command(tab, &generation, &serde_json::json!({"id": 2_000_000_002u32, "method": "Overlay.setInspectMode", "params": {"mode": "searchForNode", "highlightConfig": {"showInfo": true, "contentColor": {"r":111,"g":168,"b":220,"a":0.66}}}}).to_string(), proxy);
        }
    }

    pub(super) fn sync_hosted(state: &AppState, layout: AppLayout, chrome_hwnd: Option<isize>) {
        PANES.with(|panes| {
            for (tab, pane) in panes.borrow().iter() {
                let rect =
                    if state.tab_manager.active_tab_id.as_ref() == Some(tab) && !state.minimized {
                        layout.devtools
                    } else {
                        None
                    };
                let _ = pane.view.set_visible(rect.is_some());
                if let Some(rect) = rect {
                    set_content_bounds(&pane.view, rect);
                    if let Some(hwnd) = pane.hwnd {
                        bring_hwnd_to_top(hwnd);
                    }
                }
            }
        });
        if state.chrome_overlay_open
            || state.spotlight_open
            || state.content_cover_open
            || !state.suggestion_overlay_rects.is_empty()
        {
            if let Some(hwnd) = chrome_hwnd {
                bring_hwnd_to_top(hwnd);
            }
        }
    }

    pub(super) fn owns_message_window(hwnd: isize) -> bool {
        use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsChild};
        PANES.with(|panes| {
            panes.try_borrow().is_ok_and(|panes| {
                panes.values().any(|pane| {
                    pane.hwnd.is_some_and(|root| {
                        root == hwnd || unsafe { IsChild(HWND(root), HWND(hwnd)).as_bool() }
                    })
                })
            })
        })
    }

    pub(super) fn stale_tabs(
        state: &AppState,
        content_hwnds: &HashMap<String, isize>,
        chrome_hwnd: Option<isize>,
    ) -> Vec<String> {
        PANES.with(|panes| {
            panes
                .borrow()
                .iter()
                .filter(|(id, pane)| {
                    let Some(tab) = state.tab_manager.get_tab(id) else {
                        return true;
                    };
                    let current = if tab.is_neura_page() {
                        chrome_hwnd
                    } else {
                        content_hwnds.get(*id).copied()
                    };
                    // Sleeping/recreated tabs must not retain the previous debugging target.
                    current != pane.inspected_hwnd
                })
                .map(|(id, _)| id.clone())
                .collect()
        })
    }
    pub(super) fn close(tab: &str) {
        PANES.with(|panes| {
            panes.borrow_mut().remove(tab);
        });
    }
    pub(super) fn close_all() {
        PANES.with(|panes| panes.borrow_mut().clear());
    }
}
