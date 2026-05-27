use url::Url;

pub struct NavigationResult {
    pub url: String,
    pub is_search: bool,
}

pub fn resolve_input(input: &str, search_engine_url: &str) -> NavigationResult {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return NavigationResult {
            url: "neura://newtab".to_string(),
            is_search: false,
        };
    }

    if trimmed.starts_with("neura://") {
        return NavigationResult {
            url: trimmed.to_string(),
            is_search: false,
        };
    }

    if let Ok(url) = Url::parse(trimmed) {
        if url.scheme() == "http" || url.scheme() == "https" || url.scheme() == "file" {
            return NavigationResult {
                url: trimmed.to_string(),
                is_search: false,
            };
        }
    }

    if !trimmed.contains(' ') && trimmed.contains('.') {
        let with_scheme = format!("https://{}", trimmed);
        if Url::parse(&with_scheme).is_ok() {
            return NavigationResult {
                url: with_scheme,
                is_search: false,
            };
        }
    }

    if trimmed.starts_with("localhost")
        || trimmed.starts_with("127.0.0.1")
        || trimmed.starts_with("0.0.0.0")
    {
        return NavigationResult {
            url: format!("http://{}", trimmed),
            is_search: false,
        };
    }

    let query = urlencoding_encode(trimmed);
    let search_url = search_engine_url.replace("{query}", &query);
    NavigationResult {
        url: search_url,
        is_search: true,
    }
}

fn urlencoding_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            b => result.push_str(&format!("%{:02X}", b)),
        }
    }
    result
}

pub fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("neura://") || url.starts_with("about:") || url.starts_with("data:") {
        return url.to_string();
    }
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://") {
        return url.to_string();
    }
    format!("https://{}", url)
}

pub fn extract_domain(url: &str) -> String {
    if let Ok(parsed) = Url::parse(url) {
        return parsed.host_str().unwrap_or(url).to_string();
    }
    url.to_string()
}

pub fn is_neura_url(url: &str) -> bool {
    url.starts_with("neura://")
}
