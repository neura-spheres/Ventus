use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;

use super::config;

pub async fn upload_image(data_uri: &str) -> Result<String> {
    let url = format!(
        "https://api.cloudinary.com/v1_1/{}/image/upload",
        config::CLOUDINARY_CLOUD_NAME
    );
    let (mime, bytes) = decode_data_uri(data_uri)?;
    let ext = match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "png",
    };
    let part = Part::bytes(bytes)
        .file_name(format!("avatar.{ext}"))
        .mime_str(&mime)?;
    let form = Form::new()
        .text("upload_preset", config::CLOUDINARY_UPLOAD_PRESET)
        .part("file", part);
    let resp = Client::new().post(&url).multipart(form).send().await?;
    let status = resp.status();
    let data: Value = resp.json().await?;
    if !status.is_success() {
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Image upload failed");
        return Err(anyhow!(msg.to_string()));
    }
    let secure_url = data["secure_url"].as_str().unwrap_or_default().to_string();
    if secure_url.is_empty() {
        return Err(anyhow!("Image upload did not return a URL"));
    }
    Ok(secure_url)
}

fn decode_data_uri(data_uri: &str) -> Result<(String, Vec<u8>)> {
    let rest = data_uri
        .strip_prefix("data:")
        .ok_or_else(|| anyhow!("That file is not a valid image"))?;
    let (meta, b64) = rest
        .split_once(',')
        .ok_or_else(|| anyhow!("That image could not be read"))?;
    let mime = meta.split(';').next().unwrap_or("");
    let mime = if mime.is_empty() {
        "image/png".to_string()
    } else {
        mime.to_string()
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| anyhow!("That image could not be read"))?;
    Ok((mime, bytes))
}
