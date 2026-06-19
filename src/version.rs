pub const APP_VERSION: &str = env!("NEURA_APP_VERSION");
pub const USER_AGENT: &str = concat!("Ventus/", env!("NEURA_APP_VERSION"));

pub fn browser_like_user_agent() -> String {
    let reduced = wry::webview_version()
        .ok()
        .and_then(|raw| chromium_reduced_version(&raw))
        .unwrap_or_else(|| "0.0.0.0".to_string());
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{reduced} Safari/537.36"
    )
}

fn chromium_reduced_version(raw: &str) -> Option<String> {
    raw.trim()
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .find_map(|token| {
            let parts: Vec<&str> = token.split('.').filter(|p| !p.is_empty()).collect();
            if parts.len() >= 4 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
                Some(format!("{}.{}.{}.0", parts[0], parts[1], parts[2]))
            } else {
                None
            }
        })
}
