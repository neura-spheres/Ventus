use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEngine {
    pub id: String,
    pub name: String,
    pub url_template: String,
    pub shortcut: Option<String>,
    pub is_default: bool,
    pub is_builtin: bool,
    pub icon: String,
}

impl SearchEngine {
    pub fn builtin_engines() -> Vec<SearchEngine> {
        vec![
            SearchEngine {
                id: "duckduckgo".to_string(),
                name: "DuckDuckGo".to_string(),
                url_template: "https://duckduckgo.com/?q={query}".to_string(),
                shortcut: Some("@ddg".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "🦆".to_string(),
            },
            SearchEngine {
                id: "google".to_string(),
                name: "Google".to_string(),
                url_template: "https://www.google.com/search?q={query}".to_string(),
                shortcut: Some("@g".to_string()),
                is_default: true,
                is_builtin: true,
                icon: "🔍".to_string(),
            },
            SearchEngine {
                id: "bing".to_string(),
                name: "Bing".to_string(),
                url_template: "https://www.bing.com/search?q={query}".to_string(),
                shortcut: Some("@b".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "🔎".to_string(),
            },
            SearchEngine {
                id: "brave".to_string(),
                name: "Brave Search".to_string(),
                url_template: "https://search.brave.com/search?q={query}".to_string(),
                shortcut: Some("@brave".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "🦁".to_string(),
            },
            SearchEngine {
                id: "perplexity".to_string(),
                name: "Perplexity".to_string(),
                url_template: "https://www.perplexity.ai/search?q={query}".to_string(),
                shortcut: Some("@px".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "✨".to_string(),
            },
            SearchEngine {
                id: "youtube".to_string(),
                name: "YouTube".to_string(),
                url_template: "https://www.youtube.com/results?search_query={query}".to_string(),
                shortcut: Some("@yt".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "▶️".to_string(),
            },
            SearchEngine {
                id: "github".to_string(),
                name: "GitHub".to_string(),
                url_template: "https://github.com/search?q={query}".to_string(),
                shortcut: Some("@gh".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "🐙".to_string(),
            },
            SearchEngine {
                id: "wikipedia".to_string(),
                name: "Wikipedia".to_string(),
                url_template: "https://en.wikipedia.org/wiki/Special:Search?search={query}"
                    .to_string(),
                shortcut: Some("@wiki".to_string()),
                is_default: false,
                is_builtin: true,
                icon: "📖".to_string(),
            },
        ]
    }

    pub fn resolve_shortcut(input: &str, engines: &[SearchEngine]) -> Option<(String, String)> {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return None;
        }
        let shortcut = parts[0];
        let query = parts[1];
        for engine in engines {
            if let Some(sc) = &engine.shortcut {
                if sc == shortcut {
                    let url = engine
                        .url_template
                        .replace("{query}", &urlencoding_encode(query));
                    return Some((engine.name.clone(), url));
                }
            }
        }
        None
    }
}

pub fn add_google_context(url: &str, region: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_string();
    };
    let host = parsed.host_str().unwrap_or_default();
    if !matches!(host, "google.com" | "www.google.com") || parsed.path() != "/search" {
        return url.to_string();
    }

    let params: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let Some(query) = params
        .iter()
        .find(|(key, _)| key == "q")
        .map(|(_, value)| value.clone())
    else {
        return url.to_string();
    };
    let has = |key: &str| params.iter().any(|(name, _)| name == key);
    let region = region.trim().to_uppercase();
    let valid_region = region.len() == 2 && region.chars().all(|c| c.is_ascii_uppercase());

    let mut pairs = parsed.query_pairs_mut();
    if !has("oq") {
        pairs.append_pair("oq", &query);
    }
    if !has("ie") {
        pairs.append_pair("ie", "UTF-8");
    }
    if !has("oe") {
        pairs.append_pair("oe", "UTF-8");
    }
    if valid_region && !has("gl") {
        pairs.append_pair("gl", &region);
    }
    drop(pairs);

    parsed.into()
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

#[cfg(test)]
mod tests {
    use super::add_google_context;

    #[test]
    fn google_search_gets_reusable_context() {
        let url = add_google_context("https://www.google.com/search?q=bioinformatika", "id");
        let parsed = url::Url::parse(&url).unwrap();
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(params.get("q").map(String::as_str), Some("bioinformatika"));
        assert_eq!(params.get("oq").map(String::as_str), Some("bioinformatika"));
        assert_eq!(params.get("ie").map(String::as_str), Some("UTF-8"));
        assert_eq!(params.get("oe").map(String::as_str), Some("UTF-8"));
        assert_eq!(params.get("gl").map(String::as_str), Some("ID"));
    }

    #[test]
    fn google_search_keeps_existing_context() {
        let url = add_google_context(
            "https://www.google.com/search?q=rust&oq=original&ie=latin1&gl=SG",
            "id",
        );
        let parsed = url::Url::parse(&url).unwrap();
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(params.get("oq").map(String::as_str), Some("original"));
        assert_eq!(params.get("ie").map(String::as_str), Some("latin1"));
        assert_eq!(params.get("gl").map(String::as_str), Some("SG"));
    }

    #[test]
    fn other_urls_are_unchanged() {
        let url = "https://www.bing.com/search?q=rust";
        assert_eq!(add_google_context(url, "ID"), url);
    }
}
