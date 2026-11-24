use std::path::Path;

use wry::WebView;

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::{Arc, Mutex};
    use webview2_com::{
        BrowserExtensionEnableCompletedHandler, BrowserExtensionRemoveCompletedHandler,
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2, ICoreWebView2BrowserExtension, ICoreWebView2BrowserExtensionList,
            ICoreWebView2Profile7, ICoreWebView2_13,
        },
        ProfileAddBrowserExtensionCompletedHandler, ProfileGetBrowserExtensionsCompletedHandler,
    };
    use wry::WebViewExtWindows;
    use wv2core::{Interface, PCWSTR, PWSTR};
    use wv2win::Win32::Foundation::BOOL as WV2BOOL;

    type Done = Arc<Mutex<Option<Box<dyn FnOnce(Option<String>) + Send + 'static>>>>;

    fn done_box(done: Box<dyn FnOnce(Option<String>) + Send + 'static>) -> Done {
        Arc::new(Mutex::new(Some(done)))
    }

    fn finish(done: &Done, id: Option<String>) {
        if let Ok(mut done) = done.lock() {
            if let Some(done) = done.take() {
                done(id);
            }
        }
    }

    fn to_wide(s: &str) -> (PCWSTR, Vec<u16>) {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        (PCWSTR(v.as_ptr()), v)
    }

    unsafe fn pwstr_to_string(ptr: PWSTR) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let s = ptr.to_string().unwrap_or_default();
        windows::Win32::System::Com::CoTaskMemFree(Some(ptr.0 as *const _));
        s
    }

    fn profile(wv: &WebView) -> Option<ICoreWebView2Profile7> {
        let controller = wv.controller();
        let webview: ICoreWebView2 = unsafe { controller.CoreWebView2().ok()? };
        let webview13: ICoreWebView2_13 = webview.cast().ok()?;
        let profile = unsafe { webview13.Profile().ok()? };
        profile.cast().ok()
    }

    fn ext_name(ext: &ICoreWebView2BrowserExtension) -> String {
        unsafe {
            let mut p = PWSTR::null();
            let _ = ext.Name(&mut p);
            pwstr_to_string(p)
        }
    }

    fn is_ubol(ext: &ICoreWebView2BrowserExtension) -> bool {
        let name = ext_name(ext).to_ascii_lowercase();
        name.contains("ubo lite") || name.contains("ublock origin lite")
    }

    fn find_ubol(list: ICoreWebView2BrowserExtensionList) -> Option<ICoreWebView2BrowserExtension> {
        let mut count = 0;
        unsafe {
            let _ = list.Count(&mut count);
        }
        for i in 0..count {
            let Ok(ext) = (unsafe { list.GetValueAtIndex(i) }) else {
                continue;
            };
            if is_ubol(&ext) {
                return Some(ext);
            }
        }
        None
    }

    fn set_enabled(ext: ICoreWebView2BrowserExtension, enabled: bool, done: Done) {
        let mut raw_id = PWSTR::null();
        let id = unsafe {
            ext.Id(&mut raw_id).ok();
            pwstr_to_string(raw_id)
        };
        let mut current = WV2BOOL(0);
        if unsafe { ext.IsEnabled(&mut current) }.is_ok() && current.as_bool() == enabled {
            tracing::info!(target: "ventus::ubol", enabled, "ubol: extension already in requested state");
            finish(&done, Some(id));
            return;
        }
        let done_cb = done.clone();
        let handler = BrowserExtensionEnableCompletedHandler::create(Box::new(move |err| {
            if let Err(e) = err {
                tracing::warn!("ubol: enable failed: {}", e);
                finish(&done_cb, None);
            } else {
                tracing::info!(target: "ventus::ubol", enabled, "ubol: enable completed");
                finish(&done_cb, Some(id.clone()));
            }
            Ok(())
        }));
        unsafe {
            if let Err(e) = ext.Enable(enabled, &handler) {
                tracing::warn!("ubol: enable request failed: {}", e);
                finish(&done, None);
            }
        }
    }

    fn add(profile: ICoreWebView2Profile7, dir: String, enabled: bool, done: Done) {
        let (path, _path_buf) = to_wide(&dir);
        let done_cb = done.clone();
        let handler =
            ProfileAddBrowserExtensionCompletedHandler::create(Box::new(move |err, ext| {
                if let Err(e) = err {
                    tracing::warn!("ubol: install failed: {}", e);
                    finish(&done_cb, None);
                    return Ok(());
                }
                let Some(ext) = ext else {
                    tracing::warn!("ubol: install returned no extension");
                    finish(&done_cb, None);
                    return Ok(());
                };
                tracing::info!(target: "ventus::ubol", "ubol: install completed");
                set_enabled(ext, enabled, done_cb.clone());
                Ok(())
            }));
        unsafe {
            if let Err(e) = profile.AddBrowserExtension(path, &handler) {
                tracing::warn!("ubol: install request failed: {}", e);
                finish(&done, None);
            }
        }
    }

    fn reinstall(
        ext: ICoreWebView2BrowserExtension,
        profile: ICoreWebView2Profile7,
        dir: String,
        done: Done,
    ) {
        let done_cb = done.clone();
        let handler = BrowserExtensionRemoveCompletedHandler::create(Box::new(move |err| {
            if let Err(e) = err {
                tracing::warn!("ubol: remove failed: {}", e);
            }
            add(profile.clone(), dir.clone(), true, done_cb.clone());
            Ok(())
        }));
        if let Err(e) = unsafe { ext.Remove(&handler) } {
            tracing::warn!("ubol: remove request failed: {}", e);
            finish(&done, None);
        }
    }

    pub fn sync_ubol(
        wv: &WebView,
        dir: &Path,
        enabled: bool,
        refresh: bool,
        done: Box<dyn FnOnce(Option<String>) + Send + 'static>,
    ) {
        let done = done_box(done);
        if !dir.join("manifest.json").exists() {
            tracing::warn!("ubol: manifest not found at {}", dir.display());
            finish(&done, None);
            return;
        }
        tracing::info!(target: "ventus::ubol", path = %dir.display(), enabled, "ubol: sync requested");
        let Some(profile) = profile(wv) else {
            tracing::warn!("ubol: profile not available");
            finish(&done, None);
            return;
        };
        let add_profile = profile.clone();
        let dir = dir.to_string_lossy().to_string();
        let done_cb = done.clone();
        let handler = ProfileGetBrowserExtensionsCompletedHandler::create(Box::new(
            move |err, list| {
                if let Err(e) = err {
                    tracing::warn!("ubol: extension list failed: {}", e);
                    finish(&done_cb, None);
                    return Ok(());
                }
                if let Some(ext) = list.and_then(find_ubol) {
                    tracing::info!(target: "ventus::ubol", "ubol: existing extension found");
                    let mut on = WV2BOOL(0);
                    let on = unsafe { ext.IsEnabled(&mut on) }.is_ok() && on.as_bool();
                    if enabled && !on {
                        // A re-enabled extension never finishes starting; a fresh install does.
                        reinstall(ext, add_profile.clone(), dir.clone(), done_cb.clone());
                    } else if refresh && enabled {
                        // Re-adding the same path reloads the unpacked extension and its updated scripts.
                        add(add_profile.clone(), dir.clone(), enabled, done_cb.clone());
                    } else {
                        set_enabled(ext, enabled, done_cb.clone());
                    }
                    return Ok(());
                }
                tracing::info!(target: "ventus::ubol", "ubol: existing extension not found; installing");
                add(add_profile.clone(), dir.clone(), enabled, done_cb.clone());
                Ok(())
            },
        ));
        unsafe {
            if let Err(e) = profile.GetBrowserExtensions(&handler) {
                tracing::warn!("ubol: extension list request failed: {}", e);
                finish(&done, None);
            }
        }
    }
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn sync_ubol<F>(wv: &WebView, dir: &Path, enabled: bool, refresh: bool, done: F)
where
    F: FnOnce(Option<String>) + Send + 'static,
{
    #[cfg(windows)]
    win::sync_ubol(wv, dir, enabled, refresh, Box::new(done));
    #[cfg(not(windows))]
    done(None);
}
