use crate::storage::cookie_store::CookieRecord;
use tokio::sync::mpsc::UnboundedSender;
use wry::WebView;

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use wv2core::{Interface, PCWSTR, PWSTR};

    use wv2win::Win32::Foundation::BOOL as WV2BOOL;

    use webview2_com::{
        GetCookiesCompletedHandler,
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2, ICoreWebView2CookieList, ICoreWebView2CookieManager, ICoreWebView2_2,
            COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX, COREWEBVIEW2_COOKIE_SAME_SITE_KIND_NONE,
            COREWEBVIEW2_COOKIE_SAME_SITE_KIND_STRICT,
        },
    };
    use wry::WebViewExtWindows;

    fn to_wide(s: &str) -> (PCWSTR, Vec<u16>) {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0u16);
        let ptr = PCWSTR(v.as_ptr());
        (ptr, v)
    }

    unsafe fn pwstr_to_string(ptr: PWSTR) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let s = ptr.to_string().unwrap_or_default();
        // CoTaskMemFree is from windows 0.57 but takes a raw *const c_void —
        // no type-version mismatch for this call.
        windows::Win32::System::Com::CoTaskMemFree(Some(ptr.0 as *const _));
        s
    }

    pub fn get_cookie_manager(wv: &WebView) -> Option<ICoreWebView2CookieManager> {
        let controller = wv.controller();
        let webview: ICoreWebView2 = unsafe { controller.CoreWebView2().ok()? };
        // ICoreWebView2_2 adds the CookieManager property.  Interface::cast
        // is from wv2core 0.54, which is type-compatible with webview2-com.
        let webview2: ICoreWebView2_2 = webview.cast().ok()?;
        unsafe { webview2.CookieManager().ok() }
    }

    pub fn restore_cookies(wv: &WebView, cookies: &[CookieRecord]) {
        if cookies.is_empty() {
            return;
        }
        let Some(cm) = get_cookie_manager(wv) else {
            tracing::warn!("cookie_manager: restore — could not obtain CookieManager");
            return;
        };

        let mut ok = 0usize;
        let mut fail = 0usize;

        for c in cookies {
            let (name_p, _nv) = to_wide(&c.name);
            let (value_p, _vv) = to_wide(&c.value);
            let (domain_p, _dv) = to_wide(&c.domain);
            let (path_p, _pv) = to_wide(&c.path);

            let cookie = unsafe {
                match cm.CreateCookie(name_p, value_p, domain_p, path_p) {
                    Ok(ck) => ck,
                    Err(e) => {
                        tracing::debug!(
                            "cookie_manager: CreateCookie failed [{}/{}]: {}",
                            c.domain,
                            c.name,
                            e
                        );
                        fail += 1;
                        continue;
                    }
                }
            };

            unsafe {
                // SetExpires(-1.0) makes it a session cookie in WebView2.
                let _ = cookie.SetExpires(c.expires);
                // bool implements IntoParam<BOOL> in windows-core 0.54.
                let _ = cookie.SetIsHttpOnly(c.is_http_only);
                let _ = cookie.SetIsSecure(c.is_secure);
                let same_site = match c.same_site.as_str() {
                    "None" => COREWEBVIEW2_COOKIE_SAME_SITE_KIND_NONE,
                    "Strict" => COREWEBVIEW2_COOKIE_SAME_SITE_KIND_STRICT,
                    _ => COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX,
                };
                let _ = cookie.SetSameSite(same_site);
            }

            match unsafe { cm.AddOrUpdateCookie(&cookie) } {
                Ok(()) => ok += 1,
                Err(e) => {
                    tracing::debug!(
                        "cookie_manager: AddOrUpdateCookie failed [{}/{}]: {}",
                        c.domain,
                        c.name,
                        e
                    );
                    fail += 1;
                }
            }
        }

        tracing::info!("cookie_manager: restored {} cookies ({} skipped)", ok, fail);
    }

    pub fn navigate_to_string(wv: &WebView, html: &str) {
        let controller = wv.controller();
        let webview: ICoreWebView2 = match unsafe { controller.CoreWebView2() } {
            Ok(w) => w,
            Err(_) => return,
        };
        let (ptr, _buf) = to_wide(html);
        unsafe {
            let _ = webview.NavigateToString(ptr);
        }
    }

    fn records_from_list(list: ICoreWebView2CookieList) -> Vec<CookieRecord> {
        let mut count = 0u32;
        unsafe {
            let _ = list.Count(&mut count);
        }
        if count == 0 {
            return Vec::new();
        }

        let mut records: Vec<CookieRecord> = Vec::with_capacity(count as usize);

        for i in 0..count {
            let cookie = match unsafe { list.GetValueAtIndex(i) } {
                Ok(c) => c,
                Err(_) => continue,
            };

            let name = unsafe {
                let mut p = PWSTR::null();
                let _ = cookie.Name(&mut p);
                pwstr_to_string(p)
            };
            if name.is_empty() {
                continue;
            }

            let value = unsafe {
                let mut p = PWSTR::null();
                let _ = cookie.Value(&mut p);
                pwstr_to_string(p)
            };
            let domain = unsafe {
                let mut p = PWSTR::null();
                let _ = cookie.Domain(&mut p);
                pwstr_to_string(p)
            };
            let path = unsafe {
                let mut p = PWSTR::null();
                let _ = cookie.Path(&mut p);
                pwstr_to_string(p)
            };

            let mut expires = -1.0f64;
            unsafe {
                let _ = cookie.Expires(&mut expires);
            }

            let is_http_only = unsafe {
                let mut b = WV2BOOL(0);
                let _ = cookie.IsHttpOnly(&mut b);
                b.as_bool()
            };
            let is_secure = unsafe {
                let mut b = WV2BOOL(0);
                let _ = cookie.IsSecure(&mut b);
                b.as_bool()
            };

            let mut same_site_kind = COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX;
            unsafe {
                let _ = cookie.SameSite(&mut same_site_kind);
            }
            let same_site = match same_site_kind {
                COREWEBVIEW2_COOKIE_SAME_SITE_KIND_NONE => "None",
                COREWEBVIEW2_COOKIE_SAME_SITE_KIND_STRICT => "Strict",
                _ => "Lax",
            }
            .to_string();

            records.push(CookieRecord {
                name,
                value,
                domain,
                path,
                expires,
                is_secure,
                is_http_only,
                same_site,
            });
        }

        records
    }

    pub fn trigger_save(wv: &WebView, tx: UnboundedSender<Vec<CookieRecord>>) {
        let Some(cm) = get_cookie_manager(wv) else {
            tracing::debug!("cookie_manager: trigger_save — no CookieManager");
            return;
        };

        // Empty-string URI → get ALL cookies for the profile (WebView2 docs).
        let (empty_uri, _ev) = to_wide("");

        // The closure is 'static + Send (required for COM callback).
        // UnboundedSender<T>: Clone + Send + 'static. ✓
        let handler = GetCookiesCompletedHandler::create(Box::new(
            move |_err, cookie_list: Option<ICoreWebView2CookieList>| {
                let Some(list) = cookie_list else {
                    return Ok(());
                };
                let records = records_from_list(list);

                if !records.is_empty() {
                    tracing::debug!("cookie_manager: snapshotted {} cookies", records.len());
                    let _ = tx.send(records);
                }
                Ok(())
            },
        ));

        // Fire the request.  WebView2 COM-AddRefs `handler` until the callback
        // returns, so our local drop here is safe.
        unsafe {
            if let Err(e) = cm.GetCookies(empty_uri, &handler) {
                tracing::warn!("cookie_manager: GetCookies failed: {}", e);
            }
        }
    }

    pub fn snapshot(wv: &WebView, wait: Duration) -> Vec<CookieRecord> {
        let Some(cm) = get_cookie_manager(wv) else {
            return Vec::new();
        };

        let (empty_uri, _ev) = to_wide("");
        let (tx, rx) = mpsc::channel::<Vec<CookieRecord>>();
        let handler = GetCookiesCompletedHandler::create(Box::new(
            move |_err, cookie_list: Option<ICoreWebView2CookieList>| {
                let records = cookie_list.map(records_from_list).unwrap_or_default();
                let _ = tx.send(records);
                Ok(())
            },
        ));

        unsafe {
            if cm.GetCookies(empty_uri, &handler).is_err() {
                return Vec::new();
            }
        }

        let deadline = Instant::now() + wait;
        loop {
            if let Ok(records) = rx.try_recv() {
                return records;
            }
            if Instant::now() >= deadline {
                return Vec::new();
            }
            pump_messages_once();
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    pub fn snapshot_settled(wv: &WebView, max_wait: Duration) -> Vec<CookieRecord> {
        let start = Instant::now();
        let mut best = snapshot(wv, Duration::from_millis(1000));
        let initial = best.len();
        let mut stable = 0u32;
        while start.elapsed() < max_wait {
            std::thread::sleep(Duration::from_millis(100));
            let cur = snapshot(wv, Duration::from_millis(600));
            if cur.len() > best.len() {
                best = cur;
                stable = 0;
            } else if !cur.is_empty() {
                stable += 1;
                if stable >= 2 {
                    break;
                }
            }
        }
        tracing::info!(
            target: "ventus::browser::cookie_manager",
            initial = initial,
            settled = best.len(),
            elapsed_ms = start.elapsed().as_millis() as u64,
            "cookie snapshot settled (waited for WebView2 profile load to finish)"
        );
        best
    }

    fn pump_messages_once() {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
        };

        unsafe {
            let mut msg = MSG::default();
            let mut budget = 256u32;
            while budget > 0 && PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
                budget -= 1;
            }
        }
    }
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn restore_cookies(wv: &WebView, cookies: &[CookieRecord]) {
    #[cfg(windows)]
    win::restore_cookies(wv, cookies);
}

/// Trigger an async cookie snapshot → forward to the save task via `tx`.
/// No-op on non-Windows platforms.
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn trigger_save(wv: &WebView, tx: UnboundedSender<Vec<CookieRecord>>) {
    #[cfg(windows)]
    win::trigger_save(wv, tx);
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn snapshot(wv: &WebView, wait: std::time::Duration) -> Vec<CookieRecord> {
    #[cfg(windows)]
    {
        return win::snapshot(wv, wait);
    }
    #[cfg(not(windows))]
    Vec::new()
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn snapshot_settled(wv: &WebView, max_wait: std::time::Duration) -> Vec<CookieRecord> {
    #[cfg(windows)]
    {
        return win::snapshot_settled(wv, max_wait);
    }
    #[cfg(not(windows))]
    Vec::new()
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn navigate_to_string(wv: &WebView, html: &str) {
    #[cfg(windows)]
    win::navigate_to_string(wv, html);
}
