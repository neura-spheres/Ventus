#[cfg(windows)]
mod imp {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use windows::core::{IInspectable, HSTRING, PCWSTR};
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::Foundation::TypedEventHandler;
    use windows::UI::Notifications::{
        ToastDismissedEventArgs, ToastNotification, ToastNotificationManager, ToastNotifier,
    };

    const AUMID: &str = "NeuraSpheres.Ventus";

    static TOASTS: OnceLock<Mutex<HashMap<String, ToastNotification>>> = OnceLock::new();
    static NOTIFIER: OnceLock<ToastNotifier> = OnceLock::new();

    fn toasts() -> &'static Mutex<HashMap<String, ToastNotification>> {
        TOASTS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn notifier() -> Option<&'static ToastNotifier> {
        if NOTIFIER.get().is_none() {
            if let Ok(n) =
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))
            {
                let _ = NOTIFIER.set(n);
            }
        }
        NOTIFIER.get()
    }

    pub fn register_aumid(data_dir: &std::path::Path) {
        use windows::Win32::System::Registry::{
            RegCloseKey, RegCreateKeyExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
            REG_OPTION_NON_VOLATILE,
        };
        let subkey = wide("Software\\Classes\\AppUserModelId\\NeuraSpheres.Ventus");
        let icon_path = ensure_toast_icon(data_dir);
        unsafe {
            let mut hkey = HKEY::default();
            let created = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            );
            if created.is_err() {
                return;
            }
            set_reg_sz(hkey, "DisplayName", "Ventus");
            if let Some(path) = icon_path.as_deref().and_then(|p| p.to_str()) {
                set_reg_sz(hkey, "IconUri", path);
            }
            let _ = RegCloseKey(hkey);
        }
    }

    unsafe fn set_reg_sz(
        hkey: windows::Win32::System::Registry::HKEY,
        name: &str,
        value: &str,
    ) {
        use windows::Win32::System::Registry::{RegSetValueExW, REG_SZ};
        let name_w = wide(name);
        let value_w = wide(value);
        let bytes =
            std::slice::from_raw_parts(value_w.as_ptr() as *const u8, value_w.len() * 2);
        let _ = RegSetValueExW(hkey, PCWSTR(name_w.as_ptr()), 0, REG_SZ, Some(bytes));
    }

    fn ensure_toast_icon(data_dir: &std::path::Path) -> Option<std::path::PathBuf> {
        const ICON_BYTES: &[u8] = include_bytes!("../public/ventus.png");
        let path = data_dir.join("ventus_toast_icon.png");
        let up_to_date = std::fs::metadata(&path)
            .map(|m| m.len() == ICON_BYTES.len() as u64)
            .unwrap_or(false);
        if !up_to_date && std::fs::write(&path, ICON_BYTES).is_err() {
            return None;
        }
        Some(path)
    }

    pub fn show(
        id: &str,
        title: &str,
        body: &str,
        site: &str,
        icon: &str,
        mut on_click: Box<dyn FnMut() + Send + 'static>,
        mut on_close: Box<dyn FnMut() + Send + 'static>,
    ) {
        let Some(notifier) = notifier() else { return };
        let image = image_xml(icon);
        let site = site_xml(site);
        let xml = format!(
            "<toast><visual><binding template=\"ToastGeneric\">{}<text>{}</text><text>{}</text>{}</binding></visual></toast>",
            image,
            esc(title),
            esc(body),
            site
        );
        let Ok(doc) = XmlDocument::new() else { return };
        if doc.LoadXml(&HSTRING::from(xml)).is_err() {
            return;
        }
        let Ok(toast) = ToastNotification::CreateToastNotification(&doc) else {
            return;
        };
        let _ = toast.Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(
            move |_, _| {
                on_click();
                Ok(())
            },
        ));
        let id_close = id.to_string();
        let _ = toast.Dismissed(&TypedEventHandler::<
            ToastNotification,
            ToastDismissedEventArgs,
        >::new(move |_, _| {
            on_close();
            if let Ok(mut m) = toasts().lock() {
                m.remove(&id_close);
            }
            Ok(())
        }));
        if notifier.Show(&toast).is_ok() {
            if let Ok(mut m) = toasts().lock() {
                m.insert(id.to_string(), toast);
            }
        }
    }

    pub fn hide(id: &str) {
        let toast = toasts().lock().ok().and_then(|mut m| m.remove(id));
        if let (Some(toast), Some(notifier)) = (toast, notifier()) {
            let _ = notifier.Hide(&toast);
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn image_xml(icon: &str) -> String {
        let icon = icon.trim();
        if icon.is_empty() {
            return String::new();
        }
        format!(
            "<image placement=\"appLogoOverride\" hint-crop=\"circle\" src=\"{}\"/>",
            esc(icon)
        )
    }

    fn site_xml(site: &str) -> String {
        let site = site.trim();
        if site.is_empty() {
            return String::new();
        }
        format!("<text placement=\"attribution\">{}</text>", esc(site))
    }
}

#[cfg(windows)]
pub use imp::{hide, register_aumid, show};

#[cfg(not(windows))]
pub fn register_aumid(_data_dir: &std::path::Path) {}

#[cfg(not(windows))]
pub fn show(
    _id: &str,
    _title: &str,
    _body: &str,
    _site: &str,
    _icon: &str,
    _on_click: Box<dyn FnMut() + Send + 'static>,
    _on_close: Box<dyn FnMut() + Send + 'static>,
) {
}

#[cfg(not(windows))]
pub fn hide(_id: &str) {}
