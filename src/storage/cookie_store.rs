//! Isolated, self-contained SQLite cookie store.
//!
//! Uses a *dedicated* database file (`cookie_store.db`) that is completely
//! separate from the main application database.  This means cookie persistence
//! is unaffected even if the main database has problems, migrations fail, or
//! the app crashes mid-write.
//!
//! Lifecycle
//! ---------
//! * `open(data_dir)` — open (or create) the store.
//! * `load_all(conn)` — called once at startup to get cookies to restore.
//! * `save(conn, cookies)` — upsert batch received from the WebView2 callback.
//! * `purge_expired(conn)` — housekeeping; call after each save.
//!
//! The store intentionally holds *plain-text* cookie values.  WebView2
//! handles DPAPI/AES encryption for the live browser profile; we only need
//! the unencrypted form so we can inject cookies back through the
//! `ICoreWebView2CookieManager` API on the next startup.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

/// One cookie as read from / written to WebView2's CookieManager.
#[derive(Debug, Clone)]
pub struct CookieRecord {
    /// Cookie name, e.g. `SID`
    pub name: String,
    /// Plain-text value
    pub value: String,
    /// Domain, e.g. `.google.com` or `mail.google.com`
    pub domain: String,
    /// Path, e.g. `/`
    pub path: String,
    /// Unix seconds (f64).  `-1.0` = session cookie (no hard expiry).
    pub expires: f64,
    pub is_secure: bool,
    pub is_http_only: bool,
    /// `"None"`, `"Lax"`, or `"Strict"`
    pub same_site: String,
}

/// Open (or create) the isolated cookie-store database and ensure the schema
/// exists.  Calling this function on a path that already has a valid database
/// is a no-op — it's safe to call on every startup.
pub fn open(data_dir: &Path) -> Result<Connection> {
    let path = data_dir.join("cookie_store.db");
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous   = NORMAL;
         PRAGMA foreign_keys  = OFF;
         PRAGMA busy_timeout  = 5000;

         CREATE TABLE IF NOT EXISTS cookies (
             domain       TEXT    NOT NULL,
             path         TEXT    NOT NULL,
             name         TEXT    NOT NULL,
             value        TEXT    NOT NULL,
             expires      REAL    NOT NULL DEFAULT -1,
             is_secure    INTEGER NOT NULL DEFAULT 0,
             is_http_only INTEGER NOT NULL DEFAULT 0,
             same_site    TEXT    NOT NULL DEFAULT 'Lax',
             saved_at     INTEGER NOT NULL,
             PRIMARY KEY (domain, path, name)
         );

         CREATE INDEX IF NOT EXISTS idx_cookies_domain
             ON cookies (domain);",
    )?;
    Ok(conn)
}

/// Upsert all cookies in `cookies` into the store.
/// Session cookies (`expires == -1`) are included so the browser can restore
/// a previous session even after an abnormal shutdown.
pub fn save(conn: &Connection, cookies: &[CookieRecord]) -> Result<()> {
    if cookies.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now().timestamp();
    let mut stmt = conn.prepare_cached(
        "INSERT INTO cookies
             (domain, path, name, value, expires,
              is_secure, is_http_only, same_site, saved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(domain, path, name) DO UPDATE SET
             value        = excluded.value,
             expires      = excluded.expires,
             is_secure    = excluded.is_secure,
             is_http_only = excluded.is_http_only,
             same_site    = excluded.same_site,
             saved_at     = excluded.saved_at",
    )?;

    for c in cookies {
        stmt.execute(params![
            &c.domain,
            &c.path,
            &c.name,
            &c.value,
            c.expires,
            c.is_secure as i32,
            c.is_http_only as i32,
            &c.same_site,
            now,
        ])?;
    }
    Ok(())
}

/// Load all non-expired cookies from the store.
/// Cookies with `expires == -1` (session cookies) are always returned.
/// Cookies whose hard expiry has already passed are excluded automatically.
pub fn load_all(conn: &Connection) -> Result<Vec<CookieRecord>> {
    let now_secs = chrono::Utc::now().timestamp() as f64;
    let mut stmt = conn.prepare(
        "SELECT domain, path, name, value, expires,
                is_secure, is_http_only, same_site
         FROM cookies
         WHERE expires < 0 OR expires > ?1
         ORDER BY domain, path, name",
    )?;
    let rows = stmt.query_map(params![now_secs], |row| {
        Ok(CookieRecord {
            domain: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            value: row.get(3)?,
            expires: row.get(4)?,
            is_secure: row.get::<_, i32>(5)? != 0,
            is_http_only: row.get::<_, i32>(6)? != 0,
            same_site: row.get(7)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Delete cookies that expired more than 24 hours ago to keep the DB small.
/// Returns the number of rows deleted.
pub fn purge_expired(conn: &Connection) -> Result<usize> {
    // Keep a 24 h grace window so we don't delete anything that might still
    // be valid due to clock skew or a short background task delay.
    let cutoff = (chrono::Utc::now().timestamp() as f64) - 86_400.0;
    let n = conn.execute(
        "DELETE FROM cookies WHERE expires > 0 AND expires < ?1",
        params![cutoff],
    )?;
    Ok(n)
}

/// Return the number of cookies currently in the store.
pub fn count(conn: &Connection) -> usize {
    conn.query_row("SELECT COUNT(*) FROM cookies", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as usize
}
