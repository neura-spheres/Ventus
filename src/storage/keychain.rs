use anyhow::{anyhow, Result};

const SERVICE: &str = "ventus";

pub fn set_api_key(provider: &str, key: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, provider)?;
    entry.set_password(key)?;
    Ok(())
}

pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    let entry = keyring::Entry::new(SERVICE, provider)?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow!("Keychain error: {}", e)),
    }
}

pub fn delete_api_key(provider: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, provider)?;
    match entry.delete_password() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow!("Keychain error: {}", e)),
    }
}

pub fn has_api_key(provider: &str) -> bool {
    get_api_key(provider).map(|k| k.is_some()).unwrap_or(false)
}
