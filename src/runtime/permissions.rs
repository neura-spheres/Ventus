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
struct PermissionPolicy {
    strict: bool,
    sites: config::SitePermissionMap,
    defaults: config::SitePermissions,
}

#[cfg(windows)]
impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            strict: true,
            sites: config::SitePermissionMap::new(),
            defaults: config::SitePermissions::default(),
        }
    }
}

#[cfg(windows)]
fn set_permission_policy(settings: &config::AppSettings) {
    PERMISSION_POLICY.with(|policy| {
        *policy.borrow_mut() = PermissionPolicy {
            strict: settings.privacy.strict_permissions,
            sites: settings.privacy.site_permissions.clone(),
            defaults: settings.privacy.default_permissions.clone(),
        };
    });
}

#[cfg(windows)]
fn current_permission_action(origin: &str, key: &str) -> String {
    PERMISSION_POLICY.with(|policy| {
        let policy = policy.borrow();
        permission_action(policy.strict, &policy.sites, &policy.defaults, origin, key).to_string()
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
    site_permissions: config::SitePermissionMap,
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
            let action = current_permission_action(&origin, key);
            if action == "allow" {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
            } else if action == "block" {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
            } else if origin.is_empty() {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
            } else if let Ok(deferral) = args.GetDeferral() {
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
                if !dup
                    && proxy
                        .send_event(AppEvent::PermissionPrompt {
                            id,
                            origin: origin.clone(),
                            key: key.to_string(),
                        })
                        .is_err()
                {
                    resolve_permission(&origin, key, false);
                }
            } else {
                args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
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

    static PERMISSION_POLICY: std::cell::RefCell<PermissionPolicy> =
        std::cell::RefCell::new(PermissionPolicy::default());
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

