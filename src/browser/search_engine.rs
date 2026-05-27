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
