use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use super::config;

fn doc_url(uid: &str, name: &str) -> String {
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/users/{}/sync/{}",
        config::FIREBASE_PROJECT_ID,
        uid,
        name
    )
}

pub async fn get_blob(id_token: &str, uid: &str, name: &str) -> Result<Option<String>> {
    let resp = Client::new()
        .get(doc_url(uid, name))
        .bearer_auth(id_token)
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Ok(None);
    }
    let data: Value = resp.json().await?;
    Ok(data["fields"]["data"]["stringValue"]
        .as_str()
        .map(|v| v.to_string()))
}

pub async fn put_blob(id_token: &str, uid: &str, name: &str, data: &str) -> Result<()> {
    let body = json!({
        "fields": {
            "data": { "stringValue": data },
            "updated_at": { "integerValue": chrono::Utc::now().timestamp_millis().to_string() },
        }
    });
    let resp = Client::new()
        .patch(doc_url(uid, name))
        .bearer_auth(id_token)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let data: Value = resp.json().await.unwrap_or_default();
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Could not sync your data");
        return Err(anyhow!(msg.to_string()));
    }
    Ok(())
}
