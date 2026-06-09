use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use super::config;
use super::UserProfile;

fn doc_url(uid: &str) -> String {
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/users/{}",
        config::FIREBASE_PROJECT_ID,
        uid
    )
}

fn str_field(v: &str) -> Value {
    json!({ "stringValue": v })
}

pub async fn get_profile(id_token: &str, uid: &str) -> Result<Option<UserProfile>> {
    let resp = Client::new()
        .get(doc_url(uid))
        .bearer_auth(id_token)
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let status = resp.status();
    let data: Value = resp.json().await?;
    if !status.is_success() {
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Could not load your profile");
        return Err(anyhow!(msg.to_string()));
    }
    let fields = &data["fields"];
    let get = |key: &str| {
        fields[key]["stringValue"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    Ok(Some(UserProfile {
        uid: uid.to_string(),
        email: get("email"),
        username: get("username"),
        full_name: get("full_name"),
        birthdate: get("birthdate"),
        bio: get("bio"),
        photo_url: get("photo_url"),
        country: get("country"),
    }))
}

pub async fn save_profile(id_token: &str, profile: &UserProfile) -> Result<()> {
    let body = json!({
        "fields": {
            "email": str_field(&profile.email),
            "username": str_field(&profile.username),
            "full_name": str_field(&profile.full_name),
            "birthdate": str_field(&profile.birthdate),
            "bio": str_field(&profile.bio),
            "photo_url": str_field(&profile.photo_url),
            "country": str_field(&profile.country),
            "updated_at": { "integerValue": chrono::Utc::now().timestamp_millis().to_string() },
        }
    });
    let resp = Client::new()
        .patch(doc_url(&profile.uid))
        .bearer_auth(id_token)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let data: Value = resp.json().await.unwrap_or_default();
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Could not save your profile");
        return Err(anyhow!(msg.to_string()));
    }
    Ok(())
}
