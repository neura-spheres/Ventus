use serde_json::Value;

use super::config;

pub struct GoogleResult {
    pub id_token: String,
    pub refresh_token: String,
    pub uid: String,
    pub email: String,
    pub display_name: String,
    pub photo_url: String,
}

pub fn parse_result(body: &str) -> Result<GoogleResult, String> {
    let v: Value =
        serde_json::from_str(body).map_err(|_| "Invalid sign-in response".to_string())?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        if !err.trim().is_empty() {
            return Err(err.to_string());
        }
    }
    let result = GoogleResult {
        id_token: v["id_token"].as_str().unwrap_or_default().to_string(),
        refresh_token: v["refresh_token"].as_str().unwrap_or_default().to_string(),
        uid: v["uid"].as_str().unwrap_or_default().to_string(),
        email: v["email"].as_str().unwrap_or_default().to_string(),
        display_name: v["display_name"].as_str().unwrap_or_default().to_string(),
        photo_url: v["photo_url"].as_str().unwrap_or_default().to_string(),
    };
    if result.id_token.is_empty() || result.uid.is_empty() || result.refresh_token.is_empty() {
        return Err("Google sign-in did not complete".to_string());
    }
    Ok(result)
}

pub fn auth_page_html() -> String {
    AUTH_PAGE
        .replace("__API_KEY__", config::FIREBASE_API_KEY)
        .replace("__AUTH_DOMAIN__", config::FIREBASE_AUTH_DOMAIN)
        .replace("__PROJECT_ID__", config::FIREBASE_PROJECT_ID)
        .replace("__APP_ID__", config::FIREBASE_APP_ID)
}

const AUTH_PAGE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sign in to Ventus</title>
<style>
  html,body{height:100%;margin:0}
  body{display:flex;align-items:center;justify-content:center;background:#0d0f13;color:#e8e8ea;font-family:system-ui,'Segoe UI',sans-serif}
  .card{text-align:center;max-width:340px;padding:28px}
  .spinner{width:34px;height:34px;border:3px solid rgba(255,255,255,.15);border-top-color:#8b5cf6;border-radius:50%;margin:0 auto 18px;animation:spin .8s linear infinite}
  @keyframes spin{to{transform:rotate(360deg)}}
  h1{font-size:17px;font-weight:600;margin:0 0 8px}
  p{font-size:13px;color:#9aa0a6;margin:0;line-height:1.5}
  .err{color:#ff6b6b}
</style>
</head>
<body>
<div class="card">
  <div class="spinner" id="spin"></div>
  <h1 id="status">Connecting to Google…</h1>
  <p id="hint">Pick your Google account in the window that opens.</p>
</div>
<script type="module">
  import { initializeApp } from 'https://www.gstatic.com/firebasejs/10.12.2/firebase-app.js';
  import {
    getAuth, GoogleAuthProvider, signInWithPopup,
    browserLocalPersistence, setPersistence
  } from 'https://www.gstatic.com/firebasejs/10.12.2/firebase-auth.js';

  const firebaseConfig = {
    apiKey: "__API_KEY__",
    authDomain: "__AUTH_DOMAIN__",
    projectId: "__PROJECT_ID__",
    appId: "__APP_ID__"
  };

  const statusEl = document.getElementById('status');
  const hintEl = document.getElementById('hint');
  const spinEl = document.getElementById('spin');
  const setStatus = (t, err) => { statusEl.textContent = t; statusEl.className = err ? 'err' : ''; };

  let posted = false;
  const post = async (payload) => {
    if (posted) return;
    posted = true;
    try {
      await fetch('/complete', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    } catch (_) {}
  };

  const complete = async (user) => {
    try {
      const idToken = await user.getIdToken();
      spinEl.style.display = 'none';
      setStatus('You\'re signed in');
      hintEl.textContent = 'You can close this window and return to Ventus.';
      await post({
        id_token: idToken,
        refresh_token: user.refreshToken || '',
        uid: user.uid,
        email: user.email || '',
        display_name: user.displayName || '',
        photo_url: user.photoURL || ''
      });
    } catch (e) {
      fail(e);
    }
  };

  const fail = (e) => {
    const msg = (e && e.message) ? e.message : String(e);
    spinEl.style.display = 'none';
    setStatus('Sign-in failed', true);
    hintEl.textContent = msg;
    post({ error: msg });
  };

  const run = async () => {
    try {
      const app = initializeApp(firebaseConfig);
      const auth = getAuth(app);
      auth.useDeviceLanguage();
      await setPersistence(auth, browserLocalPersistence);
      const provider = new GoogleAuthProvider();
      provider.setCustomParameters({ prompt: 'select_account' });

      const res = await signInWithPopup(auth, provider);
      if (res && res.user) { await complete(res.user); return; }
      fail(new Error('Sign-in did not finish'));
    } catch (e) {
      const code = e && e.code ? e.code : '';
      if (code === 'auth/popup-closed-by-user' || code === 'auth/cancelled-popup-request') {
        fail(new Error('Sign-in canceled'));
        return;
      }
      fail(e);
    }
  };

  run();
</script>
</body>
</html>"#;
