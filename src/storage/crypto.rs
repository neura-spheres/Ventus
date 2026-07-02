use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde_json::Value;
use std::path::Path;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const PREFIX: &str = "v1:";

pub fn store_key(data_dir: &Path) -> Result<[u8; KEY_LEN]> {
    let path = data_dir.join("Local State.json");
    let mut state = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Value>(&text).unwrap_or_else(|_| serde_json::json!({})),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(err.into()),
    };
    if let Some(text) = state
        .get("os_crypt")
        .and_then(|v| v.get("encrypted_key"))
        .and_then(|v| v.as_str())
    {
        let protected = STANDARD.decode(text)?;
        let key = unprotect(&protected)?;
        if key.len() == KEY_LEN {
            let mut out = [0u8; KEY_LEN];
            out.copy_from_slice(&key);
            return Ok(out);
        }
    }
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    let protected = protect(&key)?;
    let root = object_child(&mut state, "os_crypt");
    root.insert(
        "encrypted_key".into(),
        Value::String(STANDARD.encode(protected)),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec(&state)?)?;
    Ok(key)
}

pub fn encrypt_text(key: &[u8; KEY_LEN], text: &str) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow!("bad key"))?;
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce), text.as_bytes())
        .map_err(|_| anyhow!("encrypt failed"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + encrypted.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&encrypted);
    Ok(format!("{PREFIX}{}", STANDARD.encode(out)))
}

pub fn decrypt_text(key: &[u8; KEY_LEN], text: &str) -> Result<String> {
    let Some(encoded) = text.strip_prefix(PREFIX) else {
        return Ok(text.to_string());
    };
    let bytes = STANDARD.decode(encoded)?;
    if bytes.len() <= NONCE_LEN {
        return Err(anyhow!("bad encrypted value"));
    }
    let (nonce, body) = bytes.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow!("bad key"))?;
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), body)
        .map_err(|_| anyhow!("decrypt failed"))?;
    Ok(String::from_utf8(plain)?)
}

fn object_child<'a>(value: &'a mut Value, key: &str) -> &'a mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = serde_json::json!({});
    }
    let root = value.as_object_mut().expect("JSON value is an object");
    let child = root
        .entry(key.to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !child.is_object() {
        *child = serde_json::json!({});
    }
    child.as_object_mut().expect("JSON child is an object")
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            windows::core::PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as _));
        Ok(bytes)
    }
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    use windows::Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as _));
        Ok(bytes)
    }
}

#[cfg(not(windows))]
fn protect(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}
