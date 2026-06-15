pub mod auth;
pub mod cloudinary;
pub mod config;
pub mod firestore;
pub mod google;
pub mod local_server;
pub mod report;
pub mod sync;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const KEYCHAIN_REFRESH_KEY: &str = "firebase_refresh";
pub const PROFILE_CACHE_KEY: &str = "user_profile_cache";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub uid: String,
    pub id_token: String,
    pub refresh_token: String,
    pub email: String,
    pub expires_at_ms: i64,
}

impl AuthSession {
    pub fn needs_refresh(&self) -> bool {
        chrono::Utc::now().timestamp_millis() >= self.expires_at_ms - 5 * 60 * 1000
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProfile {
    pub uid: String,
    pub email: String,
    pub username: String,
    pub full_name: String,
    pub birthdate: String,
    pub bio: String,
    pub photo_url: String,
    pub country: String,
}

#[derive(Debug, Clone, Default)]
pub struct SyncSnapshot {
    pub bookmarks: Option<String>,
    pub history: Option<String>,
    pub settings: Option<String>,
}

pub async fn pull_all(session: &AuthSession) -> SyncSnapshot {
    SyncSnapshot {
        bookmarks: sync::get_blob(&session.id_token, &session.uid, "bookmarks")
            .await
            .ok()
            .flatten(),
        history: sync::get_blob(&session.id_token, &session.uid, "history")
            .await
            .ok()
            .flatten(),
        settings: sync::get_blob(&session.id_token, &session.uid, "settings")
            .await
            .ok()
            .flatten(),
    }
}

pub async fn push_blobs(
    session: AuthSession,
    bookmarks: Option<String>,
    history: Option<String>,
    settings: Option<String>,
) {
    let session = match ensure_fresh(session).await {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Some(b) = bookmarks {
        let _ = sync::put_blob(&session.id_token, &session.uid, "bookmarks", &b).await;
    }
    if let Some(h) = history {
        let _ = sync::put_blob(&session.id_token, &session.uid, "history", &h).await;
    }
    if let Some(s) = settings {
        let _ = sync::put_blob(&session.id_token, &session.uid, "settings", &s).await;
    }
}

pub async fn ensure_fresh(session: AuthSession) -> Result<AuthSession> {
    if session.needs_refresh() {
        let mut refreshed = auth::refresh(&session.refresh_token).await?;
        if refreshed.email.is_empty() {
            refreshed.email = session.email;
        }
        Ok(refreshed)
    } else {
        Ok(session)
    }
}

pub fn gen_username(email: &str, full_name: &str) -> String {
    let base = if !full_name.trim().is_empty() {
        full_name.to_string()
    } else {
        email.split('@').next().unwrap_or("user").to_string()
    };
    let mut s: String = base
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if s.is_empty() {
        s.push_str("user");
    }
    if s.len() > 18 {
        s.truncate(18);
    }
    let suffix = chrono::Utc::now().timestamp_millis().rem_euclid(10000);
    format!("{}{:04}", s, suffix)
}

pub async fn finalize_sign_in(
    session: AuthSession,
    google_name: Option<String>,
    google_photo: Option<String>,
    country: String,
    cached: Option<UserProfile>,
) -> (AuthSession, UserProfile) {
    let display_name = google_name.clone().unwrap_or_default();
    let fresh = UserProfile {
        uid: session.uid.clone(),
        email: session.email.clone(),
        username: gen_username(&session.email, &display_name),
        full_name: display_name,
        birthdate: String::new(),
        bio: String::new(),
        photo_url: google_photo.clone().unwrap_or_default(),
        country: country.clone(),
    };
    let profile = match firestore::get_profile(&session.id_token, &session.uid).await {
        Ok(Some(mut p)) => {
            p.uid = session.uid.clone();
            let mut changed = false;
            if p.email.is_empty() && !session.email.is_empty() {
                p.email = session.email.clone();
                changed = true;
            }
            if p.username.is_empty() {
                p.username = gen_username(&p.email, &p.full_name);
                changed = true;
            }
            if p.full_name.is_empty() {
                if let Some(name) = google_name.filter(|n| !n.is_empty()) {
                    p.full_name = name;
                    changed = true;
                }
            }
            if p.photo_url.is_empty() {
                if let Some(photo) = google_photo.filter(|n| !n.is_empty()) {
                    p.photo_url = photo;
                    changed = true;
                }
            }
            if p.country.is_empty() && !country.is_empty() {
                p.country = country;
                changed = true;
            }
            if changed {
                let _ = firestore::save_profile(&session.id_token, &p).await;
            }
            p
        }
        Ok(None) => {
            let _ = firestore::save_profile(&session.id_token, &fresh).await;
            fresh
        }
        Err(_) => cached.unwrap_or(fresh),
    };
    (session, profile)
}

pub async fn apply_profile_edits(
    session: &AuthSession,
    mut profile: UserProfile,
    username: String,
    full_name: String,
    birthdate: String,
    bio: String,
) -> Result<UserProfile> {
    profile.uid = session.uid.clone();
    profile.username = username;
    profile.full_name = full_name;
    profile.birthdate = birthdate;
    profile.bio = bio;
    if profile.email.is_empty() {
        profile.email = session.email.clone();
    }
    let display = if !profile.full_name.is_empty() {
        profile.full_name.clone()
    } else {
        profile.username.clone()
    };
    if !display.is_empty() {
        let _ = auth::update_profile(&session.id_token, Some(&display), None).await;
    }
    firestore::save_profile(&session.id_token, &profile).await?;
    Ok(profile)
}

pub async fn apply_photo(
    session: &AuthSession,
    mut profile: UserProfile,
    data_uri: String,
) -> Result<UserProfile> {
    let url = cloudinary::upload_image(&data_uri).await?;
    profile.uid = session.uid.clone();
    profile.photo_url = url;
    let _ = auth::update_profile(&session.id_token, None, Some(&profile.photo_url)).await;
    firestore::save_profile(&session.id_token, &profile).await?;
    Ok(profile)
}
