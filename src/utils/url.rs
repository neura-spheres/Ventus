use url::Url;

/// Well-known tracking query parameters that carry no page content.
/// Stripped from URLs before display and before saving to history.
static TRACKING_PARAMS: &[&str] = &[
    // UTM standard
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_content",
    "utm_term",
    "utm_id",
    // Google Ads
    "gclid",
    "gclsrc",
    "gad_source",
    "gad_campaignid",
    "gbraid",
    "wbraid",
    // Generic Google campaign fields (chatgpt.com ads style)
    "c_id",
    "c_agid",
    "c_crid",
    "c_kwid",
    "c_ims",
    "c_pms",
    "c_nw",
    "c_dvc",
    // Meta / Facebook
    "fbclid",
    "igshid",
    // Microsoft / Bing
    "msclkid",
    // Mailchimp
    "mc_cid",
    "mc_eid",
    // Twitter / X
    "twclid",
    // LinkedIn
    "li_fat_id",
    // Pinterest
    "epik",
    // Yandex
    "yclid",
    // TikTok
    "ttclid",
    // Google Analytics client ID appended by some sites
    "_ga",
    "_gid",
    "_gl",
];

/// Strip tracking query parameters and resolve Google redirect URLs.
/// Returns the cleaned URL; returns the original string unchanged on parse failure.
pub fn clean_tracking_url(raw: &str) -> String {
    let Ok(mut parsed) = Url::parse(raw) else {
        return raw.to_string();
    };

    // Resolve Google redirect: google.com/url?q=TARGET
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if host.ends_with("google.com") && parsed.path() == "/url" {
        if let Some(target) = parsed
            .query_pairs()
            .find(|(k, _)| k == "q")
            .map(|(_, v)| v.into_owned())
        {
            return clean_tracking_url(&target);
        }
    }

    // Strip known tracking params
    let kept: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !TRACKING_PARAMS.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let total = parsed.query_pairs().count();
    if kept.len() == total {
        return raw.to_string(); // nothing to strip
    }

    parsed.query_pairs_mut().clear();
    {
        let mut pairs = parsed.query_pairs_mut();
        for (k, v) in kept {
            pairs.append_pair(&k, &v);
        }
    }
    parsed.to_string()
}

pub fn is_valid_url(s: &str) -> bool {
    Url::parse(s)
        .map(|u| u.scheme() == "http" || u.scheme() == "https")
        .unwrap_or(false)
}

pub fn extract_domain(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| url.to_string())
}

pub fn extract_favicon_url(page_url: &str) -> Option<String> {
    let u = Url::parse(page_url).ok()?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return None;
    }
    let host = u.host_str()?;
    Some(format!("{}://{}/favicon.ico", u.scheme(), host))
}

pub fn sanitize_url_for_display(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .to_string()
}

pub fn log_url(raw: &str) -> String {
    let raw = raw.trim().replace(['\r', '\n'], " ");
    if raw.is_empty() {
        return String::new();
    }
    if let Ok(mut url) = Url::parse(&raw) {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_fragment(None);
        return cut(url.to_string(), 500);
    }
    cut(raw, 300)
}

fn cut(mut text: String, max: usize) -> String {
    if text.len() <= max {
        return text;
    }
    let mut n = max.min(text.len());
    while n > 0 && !text.is_char_boundary(n) {
        n -= 1;
    }
    text.truncate(n);
    text.push_str("...");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_tracking_preserves_encoded_values() {
        let url = "https://example.com/search?q=a%26b&utm_source=x";
        assert_eq!(
            clean_tracking_url(url),
            "https://example.com/search?q=a%26b"
        );
    }

    #[test]
    fn clean_tracking_keeps_clean_url_same() {
        let url = "https://example.com/search?q=a%26b";
        assert_eq!(clean_tracking_url(url), url);
    }
}
