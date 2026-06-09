use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use super::config;
use super::AuthSession;

const IDENTITY_BASE: &str = "https://identitytoolkit.googleapis.com/v1/accounts:";
const TOKEN_URL: &str = "https://securetoken.googleapis.com/v1/token";

fn expires_at(expires_in: &str) -> i64 {
    let secs = expires_in.parse::<i64>().unwrap_or(3600);
    chrono::Utc::now().timestamp_millis() + secs * 1000
}

fn friendly_error(code: &str) -> String {
    let base = code.split(" : ").next().unwrap_or(code).trim();
    let msg = match base {
        "EMAIL_EXISTS" => "That email is already registered.",
        "EMAIL_NOT_FOUND" => "No account found for that email.",
        "INVALID_PASSWORD" | "INVALID_LOGIN_CREDENTIALS" => "Incorrect email or password.",
        "INVALID_EMAIL" => "That email address looks invalid.",
        "USER_DISABLED" => "This account has been disabled.",
        "TOO_MANY_ATTEMPTS_TRY_LATER" => "Too many attempts. Please try again later.",
        "MISSING_PASSWORD" => "Please enter a password.",
        "MISSING_EMAIL" => "Please enter an email address.",
        "CREDENTIAL_TOO_OLD_LOGIN_AGAIN" => "Please sign in again to make this change.",
        other if other.starts_with("WEAK_PASSWORD") => "Password should be at least 6 characters.",
        other => other,
    };
    msg.to_string()
}

async fn parse_auth_response(resp: reqwest::Response) -> Result<AuthSession> {
    let status = resp.status();
    let data: Value = resp.json().await?;
    if !status.is_success() {
        let msg = data["error"]["message"].as_str().unwrap_or("Authentication failed");
        return Err(anyhow!(friendly_error(msg)));
    }
    let uid = data["localId"].as_str().unwrap_or_default().to_string();
    let id_token = data["idToken"].as_str().unwrap_or_default().to_string();
    let refresh_token = data["refreshToken"].as_str().unwrap_or_default().to_string();
    let email = data["email"].as_str().unwrap_or_default().to_string();
    let expires_at_ms = expires_at(data["expiresIn"].as_str().unwrap_or("3600"));
    if uid.is_empty() || id_token.is_empty() || refresh_token.is_empty() {
        return Err(anyhow!("Invalid authentication response"));
    }
    Ok(AuthSession {
        uid,
        id_token,
        refresh_token,
        email,
        expires_at_ms,
    })
}

async fn identity_call(method: &str, body: Value) -> Result<reqwest::Response> {
    let url = format!("{}{}?key={}", IDENTITY_BASE, method, config::FIREBASE_API_KEY);
    let resp = Client::new().post(&url).json(&body).send().await?;
    Ok(resp)
}

pub async fn sign_up(email: &str, password: &str) -> Result<AuthSession> {
    let resp = identity_call(
        "signUp",
        json!({ "email": email, "password": password, "returnSecureToken": true }),
    )
    .await?;
    parse_auth_response(resp).await
}

pub async fn sign_in(email: &str, password: &str) -> Result<AuthSession> {
    let resp = identity_call(
        "signInWithPassword",
        json!({ "email": email, "password": password, "returnSecureToken": true }),
    )
    .await?;
    parse_auth_response(resp).await
}

pub async fn refresh(refresh_token: &str) -> Result<AuthSession> {
    let url = format!("{}?key={}", TOKEN_URL, config::FIREBASE_API_KEY);
    let resp = Client::new()
        .post(&url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;
    let status = resp.status();
    let data: Value = resp.json().await?;
    if !status.is_success() {
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Your session expired");
        return Err(anyhow!(friendly_error(msg)));
    }
    let uid = data["user_id"].as_str().unwrap_or_default().to_string();
    let id_token = data["id_token"].as_str().unwrap_or_default().to_string();
    let new_refresh = data["refresh_token"]
        .as_str()
        .unwrap_or(refresh_token)
        .to_string();
    let expires_at_ms = expires_at(data["expires_in"].as_str().unwrap_or("3600"));
    if uid.is_empty() || id_token.is_empty() {
        return Err(anyhow!("Could not refresh your session"));
    }
    Ok(AuthSession {
        uid,
        id_token,
        refresh_token: new_refresh,
        email: String::new(),
        expires_at_ms,
    })
}

pub async fn update_password(id_token: &str, new_password: &str) -> Result<AuthSession> {
    let resp = identity_call(
        "update",
        json!({ "idToken": id_token, "password": new_password, "returnSecureToken": true }),
    )
    .await?;
    parse_auth_response(resp).await
}

pub async fn update_profile(
    id_token: &str,
    display_name: Option<&str>,
    photo_url: Option<&str>,
) -> Result<()> {
    let mut body = json!({ "idToken": id_token, "returnSecureToken": false });
    if let Some(name) = display_name {
        body["displayName"] = json!(name);
    }
    if let Some(photo) = photo_url {
        body["photoUrl"] = json!(photo);
    }
    let resp = identity_call("update", body).await?;
    let status = resp.status();
    if !status.is_success() {
        let data: Value = resp.json().await.unwrap_or_default();
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Could not update profile");
        return Err(anyhow!(friendly_error(msg)));
    }
    Ok(())
}
