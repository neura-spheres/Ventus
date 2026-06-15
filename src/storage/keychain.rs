use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::{collections::BTreeMap, fs, path::PathBuf};

const SERVICE: &str = "ventus";
const FALLBACK_FILE: &str = "api_keys.local.json";

pub fn set_api_key(provider: &str, key: &str) -> Result<()> {
    let provider = canonical_provider(provider);
    let keyring_result =
        keyring::Entry::new(SERVICE, provider).and_then(|entry| entry.set_password(key));
    let fallback_result = set_fallback_api_key(provider, key);
    if keyring_result.is_err() && fallback_result.is_err() {
        keyring_result?;
    }
    Ok(())
}

pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    let provider = canonical_provider(provider);
    let keyring_result =
        keyring::Entry::new(SERVICE, provider).and_then(|entry| entry.get_password());
    match keyring_result {
        Ok(key) => match non_empty_key(key) {
            Some(key) => Ok(Some(key)),
            None => get_fallback_api_key(provider),
        },
        Err(keyring::Error::NoEntry) => get_fallback_api_key(provider),
        Err(e) => match get_fallback_api_key(provider)? {
            Some(key) => Ok(Some(key)),
            None => Err(anyhow!("Keychain error: {}", e)),
        },
    }
}

pub fn delete_api_key(provider: &str) -> Result<()> {
    let provider = canonical_provider(provider);
    let _ = delete_fallback_api_key(provider);
    match keyring::Entry::new(SERVICE, provider).and_then(|entry| entry.delete_password()) {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow!("Keychain error: {}", e)),
    }
}

pub fn has_api_key(provider: &str) -> bool {
    get_api_key(provider).map(|k| k.is_some()).unwrap_or(false)
}

fn canonical_provider(provider: &str) -> &str {
    match provider {
        "openai_compatible" | "openai-compatible" | "openrouter" | "ollama" => "openai",
        "anthropic_compatible" | "anthropic-compatible" => "anthropic",
        "gemini_compatible" | "gemini-compatible" => "gemini",
        other => other,
    }
}

fn provider_aliases(provider: &str) -> &'static [&'static str] {
    match canonical_provider(provider) {
        "openai" => &[
            "openai",
            "openai_compatible",
            "openai-compatible",
            "openrouter",
            "ollama",
        ],
        "anthropic" => &["anthropic", "anthropic_compatible", "anthropic-compatible"],
        "gemini" => &["gemini", "gemini_compatible", "gemini-compatible"],
        _ => &[],
    }
}

fn non_empty_key(key: String) -> Option<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn fallback_path() -> PathBuf {
    crate::utils::platform::data_dir().join(FALLBACK_FILE)
}

fn load_fallback_keys() -> Result<BTreeMap<String, String>> {
    let path = fallback_path();
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let raw = fs::read_to_string(path)?;
    let map = serde_json::from_str(&raw).unwrap_or_default();
    Ok(map)
}

fn save_fallback_keys(keys: &BTreeMap<String, String>) -> Result<()> {
    let path = fallback_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(keys)?)?;
    Ok(())
}

fn set_fallback_api_key(provider: &str, key: &str) -> Result<()> {
    let mut keys = load_fallback_keys()?;
    keys.insert(provider.to_string(), general_purpose::STANDARD.encode(key));
    save_fallback_keys(&keys)
}

fn get_fallback_api_key(provider: &str) -> Result<Option<String>> {
    let keys = load_fallback_keys()?;
    let mut names = vec![provider];
    names.extend(provider_aliases(provider));
    for name in names {
        let Some(encoded) = keys.get(name) else {
            continue;
        };
        let decoded = general_purpose::STANDARD.decode(encoded)?;
        if let Some(key) = non_empty_key(String::from_utf8(decoded)?) {
            return Ok(Some(key));
        }
    }
    Ok(None)
}

fn delete_fallback_api_key(provider: &str) -> Result<()> {
    let mut keys = load_fallback_keys()?;
    keys.remove(provider);
    save_fallback_keys(&keys)
}
