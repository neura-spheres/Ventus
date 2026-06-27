use std::path::Path;

use wry::WebView;

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::{Arc, Mutex};
    use webview2_com::{
        BrowserExtensionEnableCompletedHandler,
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2, ICoreWebView2BrowserExtension, ICoreWebView2BrowserExtensionList,
            ICoreWebView2Profile7, ICoreWebView2_13,
        },
        ProfileAddBrowserExtensionCompletedHandler, ProfileGetBrowserExtensionsCompletedHandler,
    };
    use wry::WebViewExtWindows;
    use wv2core::{Interface, PCWSTR, PWSTR};
    use wv2win::Win32::Foundation::BOOL as WV2BOOL;

    type Done = Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>;

    fn done_box(done: Box<dyn FnOnce() + Send + 'static>) -> Done {
        Arc::new(Mutex::new(Some(done)))
    }

    fn finish(done: &Done) {
        if let Ok(mut done) = done.lock() {
            if let Some(done) = done.take() {
                done();
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
        let mut current = WV2BOOL(0);
        if unsafe { ext.IsEnabled(&mut current) }.is_ok() && current.as_bool() == enabled {
            finish(&done);
            return;
        }
        let done_cb = done.clone();
        let handler = BrowserExtensionEnableCompletedHandler::create(Box::new(move |err| {
            if let Err(e) = err {
                tracing::warn!("ubol: enable failed: {}", e);
            }
            finish(&done_cb);
            Ok(())
        }));
        unsafe {
            if let Err(e) = ext.Enable(enabled, &handler) {
                tracing::warn!("ubol: enable request failed: {}", e);
                finish(&done);
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
                    finish(&done_cb);
                    return Ok(());
                }
                let Some(ext) = ext else {
                    tracing::warn!("ubol: install returned no extension");
                    finish(&done_cb);
                    return Ok(());
                };
                set_enabled(ext, enabled, done_cb.clone());
                Ok(())
            }));
        unsafe {
            if let Err(e) = profile.AddBrowserExtension(path, &handler) {
                tracing::warn!("ubol: install request failed: {}", e);
                finish(&done);
            }
        }
    }

    pub fn sync_ubol_with_done(
        wv: &WebView,
        dir: &Path,
        enabled: bool,
        done: Box<dyn FnOnce() + Send + 'static>,
    ) {
        let done = done_box(done);
        if !dir.join("manifest.json").exists() {
            tracing::warn!("ubol: manifest not found at {}", dir.display());
            finish(&done);
            return;
        }
        let Some(profile) = profile(wv) else {
            tracing::warn!("ubol: profile not available");
            finish(&done);
            return;
        };
        let add_profile = profile.clone();
        let dir = dir.to_string_lossy().to_string();
        let done_cb = done.clone();
        let handler =
            ProfileGetBrowserExtensionsCompletedHandler::create(Box::new(move |err, list| {
                if let Err(e) = err {
                    tracing::warn!("ubol: extension list failed: {}", e);
                    finish(&done_cb);
                    return Ok(());
                }
                if let Some(ext) = list.and_then(find_ubol) {
                    set_enabled(ext, enabled, done_cb.clone());
                    return Ok(());
                }
                add(add_profile.clone(), dir.clone(), enabled, done_cb.clone());
                Ok(())
            }));
        unsafe {
            if let Err(e) = profile.GetBrowserExtensions(&handler) {
                tracing::warn!("ubol: extension list request failed: {}", e);
                finish(&done);
            }
        }
    }
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn sync_ubol(wv: &WebView, dir: &Path, enabled: bool) {
    sync_ubol_with_done(wv, dir, enabled, || {});
}

#[cfg_attr(not(windows), allow(unused_variables))]
pub fn sync_ubol_with_done<F>(wv: &WebView, dir: &Path, enabled: bool, done: F)
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(windows)]
    win::sync_ubol_with_done(wv, dir, enabled, Box::new(done));
    #[cfg(not(windows))]
    done();
}
