use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[derive(Default)]
struct StrictBlockRules {
    domains: Vec<String>,
    regex_rules: Vec<StrictRegexRule>,
}

struct StrictRegexRule {
    domains: Vec<String>,
    excluded_domains: Vec<String>,
    regex: Regex,
}

#[derive(Deserialize)]
struct Manifest {
    declarative_net_request: Option<DeclarativeNetRequest>,
}

#[derive(Deserialize)]
struct DeclarativeNetRequest {
    rule_resources: Vec<RuleResource>,
}

#[derive(Deserialize)]
struct RuleResource {
    id: String,
    #[serde(default)]
    enabled: bool,
}

#[derive(Deserialize)]
struct DnrRule {
    condition: Option<DnrCondition>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DnrCondition {
    regex_filter: Option<String>,
    request_domains: Option<Vec<String>>,
    excluded_request_domains: Option<Vec<String>>,
    resource_types: Option<Vec<String>>,
    excluded_resource_types: Option<Vec<String>>,
}

pub fn matches(url: &str) -> bool {
    rules().matches(url)
}

fn rules() -> &'static StrictBlockRules {
    static RULES: OnceLock<StrictBlockRules> = OnceLock::new();
    RULES.get_or_init(load_rules)
}

fn load_rules() -> StrictBlockRules {
    let Some(dir) = ubol_dir() else {
        tracing::warn!("strictblock: ubol directory not found");
        return StrictBlockRules::default();
    };
    let enabled = enabled_rulesets(&dir);
    if enabled.is_empty() {
        tracing::warn!("strictblock: no enabled ubol rulesets found");
        return StrictBlockRules::default();
    }
    let mut rules = StrictBlockRules::default();
    let strict_dir = dir.join("rulesets").join("strictblock");
    for id in enabled {
        load_file(&strict_dir.join(format!("{id}.json")), &mut rules);
    }
    rules
}

fn ubol_dir() -> Option<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(base) = exe.parent() {
            dirs.push(base.join("assets").join("extensions").join("ubol"));
            dirs.push(base.join("extensions").join("ubol"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets").join("extensions").join("ubol"));
    }
    dirs.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("extensions")
            .join("ubol"),
    );
    dirs.into_iter()
        .find(|dir| dir.join("manifest.json").exists())
}

fn enabled_rulesets(dir: &Path) -> HashSet<String> {
    let Ok(text) = fs::read_to_string(dir.join("manifest.json")) else {
        return HashSet::new();
    };
    let Ok(manifest) = serde_json::from_str::<Manifest>(&text) else {
        return HashSet::new();
    };
    manifest
        .declarative_net_request
        .map(|dnr| {
            dnr.rule_resources
                .into_iter()
                .filter(|resource| resource.enabled)
                .map(|resource| resource.id)
                .collect()
        })
        .unwrap_or_default()
}

fn load_file(path: &Path, rules: &mut StrictBlockRules) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(items) = serde_json::from_str::<Vec<DnrRule>>(&text) else {
        tracing::warn!("strictblock: failed to parse {}", path.display());
        return;
    };
    let mut seen_domains: HashSet<String> = rules.domains.iter().cloned().collect();
    for item in items {
        let Some(condition) = item.condition else {
            continue;
        };
        if !applies_to_main_frame(&condition) {
            continue;
        }
        add_condition(condition, rules, &mut seen_domains);
    }
}

fn add_condition(
    condition: DnrCondition,
    rules: &mut StrictBlockRules,
    seen_domains: &mut HashSet<String>,
) {
    let domains = clean_domains(condition.request_domains.unwrap_or_default());
    let excluded_domains = clean_domains(condition.excluded_request_domains.unwrap_or_default());
    let regex = condition.regex_filter.unwrap_or_default();
    if domains.is_empty() && regex.trim().is_empty() {
        return;
    }
    if broad_regex(&regex) && !domains.is_empty() && excluded_domains.is_empty() {
        for domain in domains {
            if domain.contains('.') && seen_domains.insert(domain.clone()) {
                rules.domains.push(domain);
            }
        }
        return;
    }
    let Ok(regex) = Regex::new(&regex) else {
        return;
    };
    rules.regex_rules.push(StrictRegexRule {
        domains,
        excluded_domains,
        regex,
    });
}

fn applies_to_main_frame(condition: &DnrCondition) -> bool {
    if condition
        .excluded_resource_types
        .as_ref()
        .map(|types| has_resource_type(types, "main_frame"))
        .unwrap_or(false)
    {
        return false;
    }
    condition
        .resource_types
        .as_ref()
        .map(|types| has_resource_type(types, "main_frame"))
        .unwrap_or(true)
}

fn has_resource_type(types: &[String], needle: &str) -> bool {
    types.iter().any(|kind| kind == needle)
}

fn clean_domains(domains: Vec<String>) -> Vec<String> {
    domains
        .into_iter()
        .map(|domain| clean_domain(&domain))
        .filter(|domain| !domain.is_empty())
        .collect()
}

fn clean_domain(domain: &str) -> String {
    domain
        .trim()
        .trim_start_matches("*.")
        .trim_start_matches('.')
        .trim_start_matches("www.")
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn broad_regex(regex: &str) -> bool {
    matches!(regex.trim(), "" | "^https?://.*")
}

impl StrictBlockRules {
    fn matches(&self, url: &str) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            return false;
        }
        let Some(host) = parsed.host_str().map(clean_domain) else {
            return false;
        };
        if domain_matches(&host, &self.domains, false) {
            return true;
        }
        self.regex_rules
            .iter()
            .any(|rule| rule.matches(parsed.as_str(), &host))
    }
}

impl StrictRegexRule {
    fn matches(&self, url: &str, host: &str) -> bool {
        if !self.domains.is_empty() && !domain_matches(host, &self.domains, true) {
            return false;
        }
        if domain_matches(host, &self.excluded_domains, true) {
            return false;
        }
        self.regex.is_match(url)
    }
}

fn domain_matches(host: &str, domains: &[String], allow_tld: bool) -> bool {
    domains
        .iter()
        .any(|domain| domain_match(host, domain, allow_tld))
}

fn domain_match(host: &str, domain: &str, allow_tld: bool) -> bool {
    if host == domain {
        return true;
    }
    if !allow_tld && !domain.contains('.') {
        return false;
    }
    let Some(offset) = host.len().checked_sub(domain.len() + 1) else {
        return false;
    };
    host.ends_with(domain) && host.as_bytes().get(offset) == Some(&b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_match_youku_not_gmail() {
        assert!(matches("https://youku.tv/"));
        assert!(matches("https://www.youku.tv/"));
        assert!(!matches("https://gmail.com/"));
        assert!(!matches("https://mail.google.com/mail/u/0/#inbox"));
    }

    #[test]
    fn regex_rules_need_url_match() {
        let rule = StrictRegexRule {
            domains: vec!["com".to_string()],
            excluded_domains: Vec::new(),
            regex: Regex::new("^https://a\\.com/path$").unwrap(),
        };
        assert!(rule.matches("https://a.com/path", "a.com"));
        assert!(!rule.matches("https://gmail.com/", "gmail.com"));
    }
}
