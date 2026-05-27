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
}

fn normalize_host(host: &str) -> String {
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
