#[cfg(windows)]
fn available_memory_mb() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut ms).is_ok() {
            return ms.ullAvailPhys / 1024 / 1024;
        }
    }
    u64::MAX
}

#[cfg(not(windows))]
fn available_memory_mb() -> u64 {
    u64::MAX
}

#[cfg(windows)]
fn available_commit_mb() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut ms).is_ok() {
            return ms.ullAvailPageFile / 1024 / 1024;
        }
    }
    u64::MAX
}

#[cfg(not(windows))]
fn available_commit_mb() -> u64 {
    u64::MAX
}

#[cfg(windows)]
fn trim_working_set() {
    use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
    unsafe {
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

#[cfg(not(windows))]
fn trim_working_set() {}

fn sleep_threshold_ms(free_mb: u64) -> u64 {
    match free_mb {
        m if m > 8192 => 15 * 60 * 1000,
        m if m > 4096 => 10 * 60 * 1000,
        m if m > 2048 => 6 * 60 * 1000,
        m if m > 1024 => 3 * 60 * 1000,
        m if m > 512 => 90 * 1000,
        _ => 30 * 1000,
    }
}

fn max_live_webviews(free_mb: u64) -> usize {
    match free_mb {
        m if m > 8192 => 10,
        m if m > 4096 => 8,
        m if m > 2048 => 6,
        m if m > 1024 => 4,
        _ => 3,
    }
}

fn suspend_idle_ms(free_mb: u64) -> i64 {
    match free_mb {
        m if m > 8192 => 180_000,
        m if m > 4096 => 120_000,
        m if m > 2048 => 75_000,
        m if m > 1024 => 45_000,
        m if m > 512 => 25_000,
        _ => 12_000,
    }
}

fn is_comm_app(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    if host.is_empty() {
        return false;
    }
    const COMM_HOSTS: &[&str] = &[
        "web.whatsapp.com",
        "discord.com",
        "discordapp.com",
        "web.telegram.org",
        "messenger.com",
        "teams.microsoft.com",
        "teams.live.com",
        "web.skype.com",
        "chat.google.com",
        "slack.com",
    ];
    COMM_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{}", h)))
}

fn tab_notifications_allowed(url: &str, settings: &config::AppSettings) -> bool {
    #[cfg(windows)]
    {
        let Some(origin) = normalize_webview_origin(url) else {
            return false;
        };
        permission_action(
            settings.privacy.strict_permissions,
            &settings.privacy.site_permissions,
            &settings.privacy.default_permissions,
            &origin,
            "notifications",
        ) == "allow"
    }
    #[cfg(not(windows))]
    {
        let _ = (url, settings);
        false
    }
}

fn image_filename_from_url(url: &str) -> String {
    if url.starts_with("data:") {
        let mime = url
            .get(5..)
            .and_then(|s| s.split(';').next())
            .unwrap_or("image/png");
        let ext = match mime {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "image/bmp" => "bmp",
            "image/ico" | "image/x-icon" => "ico",
            _ => "png",
        };
        return format!("image.{}", ext);
    }
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(last) = parsed.path_segments().and_then(|s| s.last()) {
            let name = last.split('?').next().unwrap_or(last);
            let lower = name.to_lowercase();
            let known_ext = [
                "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif",
            ]
            .iter()
            .any(|e| lower.ends_with(&format!(".{}", e)));
            if known_ext && !name.is_empty() {
                return name.to_string();
            }
        }
    }
    "image.png".to_string()
}

/// A confirmed "Save image as" destination awaiting the image bytes from the content WebView.
struct PendingImageSave {
    dest: std::path::PathBuf,
    url: String,
    /// Page URL used as the Referer for the reqwest fallback (hotlink-protected hosts).
    referer: String,
}

/// Decode the bytes encoded in a `data:` URL. Handles both base64 (`;base64,`) and
/// percent-encoded payloads (e.g. inline SVG). The `data:` part may be absent — anything
/// after the first comma is treated as the payload.
fn decode_data_url(data: &str) -> anyhow::Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let comma = data
        .find(',')
        .ok_or_else(|| anyhow::anyhow!("malformed data URL"))?;
    let header = &data[..comma];
    let payload = &data[comma + 1..];
    if header.contains(";base64") {
        // Whitespace can appear in long base64 data URLs — strip it before decoding.
        let cleaned: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
        Ok(STANDARD.decode(cleaned)?)
    } else {
        // Percent-decoded text payload.
        Ok(percent_decode(payload))
    }
}

fn percent_decode(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

/// Download an image over HTTP(S) using a browser-like User-Agent and a Referer header.
/// The Referer lets hotlink-protected hosts serve the asset, and a real UA avoids the
/// blanket 403s some CDNs return for unknown clients. Errors on any non-success status so
/// the caller can mark the download Failed instead of writing an HTML error page to disk.
async fn fetch_image_bytes(url: &str, referer: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent(crate::version::USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let mut req = client.get(url);
    if !referer.trim().is_empty() && referer.starts_with("http") {
        req = req.header(reqwest::header::REFERER, referer);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("http {}", resp.status().as_u16()));
    }
    Ok(resp.bytes().await?.to_vec())
}

/// Decode image bytes (any supported format) into a CF_DIB bitmap and write it to
/// the Windows clipboard so the user can paste it into any application.
/// Uses a 24-bit BGR bottom-up DIB which is universally accepted by Win32 apps.
#[cfg(windows)]
fn write_image_to_clipboard(bytes: &[u8]) -> anyhow::Result<()> {
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, RegisterClipboardFormatW,
    };

    let img = image::load_from_memory(bytes)?.to_rgba8();
    let width = img.width() as usize;
    let height = img.height() as usize;
    let pixels = img.as_raw();

    let mut png: Vec<u8> = Vec::new();
    {
        use image::ImageEncoder;
        image::codecs::png::PngEncoder::new(&mut png).write_image(
            pixels,
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )?;
    }

    let stride = (width * 3 + 3) & !3;
    const HDR: usize = 40;
    let total = HDR + stride * height;
    let mut dib = vec![0u8; total];

    dib[0..4].copy_from_slice(&40u32.to_le_bytes());
    dib[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    dib[8..12].copy_from_slice(&(height as i32).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&24u16.to_le_bytes());

    for dib_row in 0..height {
        let img_row = height - 1 - dib_row;
        let dst = HDR + dib_row * stride;
        let src = img_row * width * 4;
        for col in 0..width {
            dib[dst + col * 3] = pixels[src + col * 4 + 2];
            dib[dst + col * 3 + 1] = pixels[src + col * 4 + 1];
            dib[dst + col * 3 + 2] = pixels[src + col * 4];
        }
    }

    open_clipboard_retry()?;
    let result = (|| -> anyhow::Result<()> {
        unsafe { EmptyClipboard()? };
        set_clipboard_mem(8u32, &dib)?;
        let png_fmt = unsafe { RegisterClipboardFormatW(windows::core::w!("PNG")) };
        if png_fmt != 0 {
            let _ = set_clipboard_mem(png_fmt, &png);
        }
        Ok(())
    })();
    unsafe {
        let _ = CloseClipboard();
    }
    result
}

#[cfg(windows)]
fn open_clipboard_retry() -> anyhow::Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::OpenClipboard;
    for _ in 0..12 {
        if unsafe { OpenClipboard(HWND(0)) }.is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    Err(anyhow::anyhow!("OpenClipboard busy"))
}

#[cfg(windows)]
fn set_clipboard_mem(fmt: u32, data: &[u8]) -> anyhow::Result<()> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::SetClipboardData;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    unsafe {
        let hmem = GlobalAlloc(GMEM_MOVEABLE, data.len())?;
        let ptr = GlobalLock(hmem) as *mut u8;
        if ptr.is_null() {
            return Err(anyhow::anyhow!("GlobalLock failed"));
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        let _ = GlobalUnlock(hmem);
        if let Err(e) = SetClipboardData(fmt, HANDLE(hmem.0 as isize)) {
            return Err(anyhow::anyhow!("SetClipboardData: {e}"));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
fn write_image_to_clipboard(_bytes: &[u8]) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "clipboard write not supported on this platform"
    ))
}

#[cfg(windows)]
fn read_clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::{HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    const CF_UNICODETEXT: u32 = 13;
    unsafe {
        if OpenClipboard(HWND(0)).is_err() {
            return None;
        }
        let text = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
            if handle.0 == 0 {
                return None;
            }
            let hglobal = HGLOBAL(handle.0 as *mut core::ffi::c_void);
            let ptr = GlobalLock(hglobal) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            let _ = GlobalUnlock(hglobal);
            Some(s)
        })();
        let _ = CloseClipboard();
        text
    }
}

#[cfg(not(windows))]
fn read_clipboard_text() -> Option<String> {
    None
}

#[cfg(windows)]
fn write_clipboard_text(text: &str) -> anyhow::Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::Foundation::HANDLE;

    const CF_UNICODETEXT: u32 = 13;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * 2;
    unsafe {
        OpenClipboard(HWND(0))?;
        EmptyClipboard()?;
        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len)?;
        let ptr = GlobalLock(hmem) as *mut u16;
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err(anyhow::anyhow!("GlobalLock failed"));
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        let _ = GlobalUnlock(hmem);
        if let Err(e) = SetClipboardData(CF_UNICODETEXT, HANDLE(hmem.0 as isize)) {
            let _ = CloseClipboard();
            return Err(anyhow::anyhow!("SetClipboardData: {e}"));
        }
        let _ = CloseClipboard();
        Ok(())
    }
}

#[cfg(not(windows))]
fn write_clipboard_text(_text: &str) -> anyhow::Result<()> {
    Ok(())
}

/// JS injected into the active content WebView to fetch an image the way the page itself
/// would — with its cookies/session and full access to `blob:` URLs that don't exist
/// outside the page. The bytes come back as a base64 data URL via IPC; Rust writes them to
/// the user-chosen path. Cross-origin images the page can't read (no CORS) reject here and
/// fall back to a server-side reqwest fetch in Rust.
fn save_image_fetch_script(save_id: &str, url: &str) -> String {
    let id_js = serde_json::to_string(save_id).unwrap_or_else(|_| "\"\"".into());
    let url_js = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(function(){{
  var __id={id}, __u={url};
  function __post(ok,data){{
    try{{ window.ipc.postMessage(JSON.stringify({{cmd:'save_image_data',save_id:__id,ok:ok,data:data||''}})); }}catch(e){{}}
  }}
  try{{
    fetch(__u,{{credentials:'include'}})
      .then(function(r){{ if(!r.ok) throw new Error('status'); return r.blob(); }})
      .then(function(b){{
        var fr=new FileReader();
        fr.onload=function(){{ __post(true,fr.result); }};
        fr.onerror=function(){{ __post(false,''); }};
        fr.readAsDataURL(b);
      }})
      .catch(function(){{ __post(false,''); }});
  }}catch(e){{ __post(false,''); }}
}})()"#,
        id = id_js,
        url = url_js
    )
}

fn copy_image_fetch_script(url: &str) -> String {
    let url_js = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(function(){{
  var __u={url};
  function __post(ok,data){{
    try{{ window.ipc.postMessage(JSON.stringify({{cmd:'copy_image_data',ok:ok,data:data||'',src:__u}})); }}catch(e){{}}
  }}
  try{{
    fetch(__u,{{credentials:'include'}})
      .then(function(r){{ if(!r.ok) throw new Error('status'); return r.blob(); }})
      .then(function(b){{
        if(!/^image\//.test(b.type||'')) throw new Error('type');
        var fr=new FileReader();
        fr.onload=function(){{ __post(true,fr.result); }};
        fr.onerror=function(){{ __post(false,''); }};
        fr.readAsDataURL(b);
      }})
      .catch(function(){{ __post(false,''); }});
  }}catch(e){{ __post(false,''); }}
}})()"#,
        url = url_js
    )
}
