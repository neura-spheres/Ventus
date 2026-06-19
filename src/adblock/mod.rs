pub struct AdBlockEngine {
    pub exceptions: Vec<String>,
    pub enabled: bool,
}

impl AdBlockEngine {
    pub fn new(enabled: bool, exceptions: &[String]) -> Self {
        Self {
            exceptions: exceptions.iter().map(|s| normalize_host(s)).collect(),
            enabled,
        }
    }

    pub fn init_script(&self) -> &str {
        ""
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn toggle_exception(&mut self, host: &str) -> bool {
        let host = normalize_host(host);
        if let Some(pos) = self.exceptions.iter().position(|e| e == &host) {
            self.exceptions.remove(pos);
            return false;
        }
        self.exceptions.push(host);
        true
    }

    pub fn ensure_exception(&mut self, host: &str) -> bool {
        let host = normalize_host(host);
        if host.is_empty() || self.exceptions.iter().any(|e| e == &host) {
            return false;
        }
        self.exceptions.push(host);
        true
    }

    pub fn set_exceptions(&mut self, exceptions: Vec<String>) {
        self.exceptions = exceptions.iter().map(|s| normalize_host(s)).collect();
    }

    pub fn exceptions(&self) -> &[String] {
        &self.exceptions
    }

    pub fn is_site_excepted(&self, url: &str) -> bool {
        let Ok(u) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = u.host_str() else {
            return false;
        };
        host_matches_list(host, &self.exceptions)
    }

    pub fn is_host_excepted(&self, host: &str) -> bool {
        host_matches_list(host, &self.exceptions)
    }
}

fn normalize_host(host: &str) -> String {
    let host = host.trim().trim_end_matches('.');
    if let Ok(url) = url::Url::parse(host) {
        if let Some(host) = url.host_str() {
            return host.to_lowercase().trim_start_matches("www.").to_string();
        }
    }
    host.to_lowercase()
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn host_matches_list<S: AsRef<str>>(host: &str, list: &[S]) -> bool {
    let host = normalize_host(host);
    let parts: Vec<&str> = host.split('.').collect();
    for i in 0..parts.len().saturating_sub(1) {
        let candidate = parts[i..].join(".");
        if list.iter().any(|item| item.as_ref() == candidate) {
            return true;
        }
    }
    false
}

pub fn is_x_compat_host(host: &str) -> bool {
    let host = normalize_host(host);
    host == "x.com"
        || host.ends_with(".x.com")
        || host == "twitter.com"
        || host.ends_with(".twitter.com")
}

pub fn is_x_compat_url(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    url.host_str().map(is_x_compat_host).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_compat_url_matches_only_x_and_twitter() {
        assert!(is_x_compat_url("https://x.com/i/flow/login"));
        assert!(is_x_compat_url("https://mobile.x.com/login"));
        assert!(is_x_compat_url("https://twitter.com/i/flow/login"));
        assert!(is_x_compat_url(
            "https://api.twitter.com/oauth/authenticate"
        ));

        assert!(!is_x_compat_url("https://example.com/x.com"));
        assert!(!is_x_compat_url("https://notx.com/login"));
        assert!(!is_x_compat_url("https://x.com.example/login"));
        assert!(!is_x_compat_url("file:///x.com/login"));
    }

    #[test]
    fn ensure_exception_is_idempotent() {
        let mut engine = AdBlockEngine::new(true, &[]);
        assert!(engine.ensure_exception("https://www.x.com/login"));
        assert!(!engine.ensure_exception("x.com"));
        assert!(engine.is_site_excepted("https://mobile.x.com/i/flow/login"));
        assert_eq!(engine.exceptions(), &["x.com".to_string()]);
    }
}
