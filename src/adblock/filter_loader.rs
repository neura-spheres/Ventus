use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

const EASYLIST_URL: &str = "https://easylist.to/easylist/easylist.txt";
const EASYPRIVACY_URL: &str = "https://easylist.to/easylist/easyprivacy.txt";
const UBLOCK_URL: &str =
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt";
const UBLOCK_PRIVACY_URL: &str =
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/privacy.txt";

const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600); // refresh every 7 days
const DL_TIMEOUT: Duration = Duration::from_secs(30);

pub struct LoadedLists {
    pub combined: String,
}

pub async fn load(data_dir: &Path) -> LoadedLists {
    let dir = data_dir.join("filter_lists");
    let _ = std::fs::create_dir_all(&dir);

    // Fetch all four lists in parallel.
    let (el, ep, ub, up) = tokio::join!(
        fetch_one(&dir, "easylist.txt", EASYLIST_URL),
        fetch_one(&dir, "easyprivacy.txt", EASYPRIVACY_URL),
        fetch_one(&dir, "ublock-filters.txt", UBLOCK_URL),
        fetch_one(&dir, "ublock-privacy.txt", UBLOCK_PRIVACY_URL),
    );

    let combined = [el, ep, ub, up].join("\n");
    LoadedLists { combined }
}

async fn fetch_one(dir: &Path, filename: &str, url: &str) -> String {
    let path = dir.join(filename);
    if is_fresh(&path) {
        if let Ok(s) = std::fs::read_to_string(&path) {
            if !s.is_empty() {
                return s;
            }
        }
    }
    tracing::debug!("Downloading filter list: {}", url);
    match download(url).await {
        Ok(s) => {
            let _ = std::fs::write(&path, &s);
            s
        }
        Err(e) => {
            tracing::warn!("Failed to download {}: {}", url, e);
            // Fall back to stale cache if available.
            std::fs::read_to_string(&path).unwrap_or_default()
        }
    }
}

fn is_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.elapsed()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        })
        .map(|age| age < MAX_AGE)
        .unwrap_or(false)
}

async fn download(url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(DL_TIMEOUT)
        .user_agent(crate::version::USER_AGENT)
        .build()?;
    Ok(client.get(url).send().await?.text().await?)
}

/// Parse combined filter list text and extract a deduplicated set of blocked root domains.
/// Only extracts `||domain^` style rules (pure domain-level blocks) — these are safe to use
/// in our JS Set lookup without needing full URL pattern matching.
pub fn extract_domains(filter_text: &str) -> Vec<String> {
    let mut raw: HashSet<String> = HashSet::new();

    for line in filter_text.lines() {
        let line = line.trim();
        // Skip blank lines, comments, exception rules, cosmetic rules, and procedural filters.
        if line.is_empty()
            || line.starts_with('!')
            || line.starts_with('#')
            || line.starts_with("@@")
            || line.contains("##")
            || line.contains("#@#")
            || line.contains("#?#")
        {
            continue;
        }

        // Must start with || (anchor at hostname start).
        if !line.starts_with("||") {
            continue;
        }
        let rest = &line[2..];

        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
            .unwrap_or(rest.len());
        let domain = &rest[..end];
        let boundary = rest[end..].chars().next();
        if matches!(boundary, Some('/') | Some('?') | Some('#') | Some('*')) {
            continue;
        }
        if boundary.is_some() && !matches!(boundary, Some('^') | Some('$')) {
            continue;
        }
        if boundary == Some('^') {
            let tail = &rest[end + 1..];
            if !tail.is_empty() && !tail.starts_with('$') {
                continue;
            }
        }

        if domain.is_empty()
            || domain.contains('*')
            || domain.contains(':')
            || domain.contains(' ')
            || domain.starts_with('.')
            || domain.ends_with('.')
        {
            continue;
        }
        if is_auth_domain(domain) {
            continue;
        }

        if !domain.contains('.')
            || !domain
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        {
            continue;
        }

        if let Some(opts) = opts_after_domain(rest, end) {
            if opts_has_resource_type(opts) {
                continue;
            }
        }

        raw.insert(domain.to_lowercase());
    }

    // Deduplicate subdomains: remove `sub.example.com` if `example.com` is already in the set.
    // This shrinks the list significantly (e.g. ad1.domain.com, ad2.domain.com → domain.com).
    let raw_ref: HashSet<&str> = raw.iter().map(|s| s.as_str()).collect();
    let mut result: Vec<String> = raw
        .iter()
        .filter(|d| {
            let parts: Vec<&str> = d.split('.').collect();
            // For each suffix of length >= 2 (excluding the domain itself), check if it's blocked.
            for i in 1..parts.len().saturating_sub(1) {
                let parent = parts[i..].join(".");
                if raw_ref.contains(parent.as_str()) {
                    return false; // parent already blocks this
                }
            }
            true
        })
        .cloned()
        .collect();

    result.sort();
    result
}

fn opts_after_domain(rest: &str, end: usize) -> Option<&str> {
    let tail = &rest[end..];
    if let Some(opts) = tail.strip_prefix('$') {
        return Some(opts);
    }
    tail.strip_prefix("^$")
}

fn opts_has_resource_type(opts: &str) -> bool {
    opts.split(',').any(|opt| {
        let opt = opt.trim_start_matches('~');
        let name = opt.split('=').next().unwrap_or(opt);
        matches!(
            name,
            "beacon"
                | "csp"
                | "document"
                | "font"
                | "image"
                | "media"
                | "object"
                | "other"
                | "ping"
                | "popup"
                | "script"
                | "stylesheet"
                | "subdocument"
                | "websocket"
                | "xmlhttprequest"
        )
    })
}

fn is_auth_domain(domain: &str) -> bool {
    const ROOTS: &[&str] = &[
        "apple.com",
        "github.com",
        "githubassets.com",
        "githubusercontent.com",
        "google.com",
        "googleapis.com",
        "googleusercontent.com",
        "gstatic.com",
        "live.com",
        "microsoft.com",
        "microsoftonline.com",
    ];
    ROOTS
        .iter()
        .any(|root| domain == *root || domain.ends_with(&format!(".{}", root)))
}

#[cfg(test)]
mod tests {
    use super::extract_domains;

    #[test]
    fn ignores_path_rules_for_auth_roots() {
        let domains =
            extract_domains("||google.com/pagead/\n||cse.google.com/cse_v2/ads$subdocument");
        assert!(!domains.iter().any(|d| d == "google.com"));
        assert!(!domains.iter().any(|d| d == "cse.google.com"));
    }

    #[test]
    fn keeps_plain_domain_rules() {
        let domains = extract_domains("||ads.example.com^\n||tracker.example.net^$third-party");
        assert!(domains.iter().any(|d| d == "ads.example.com"));
        assert!(domains.iter().any(|d| d == "tracker.example.net"));
    }

    #[test]
    fn skips_resource_limited_rules() {
        let domains = extract_domains("||cdn.example.com^$script,third-party");
        assert!(domains.is_empty());
    }
}
