use crate::storage::crypto;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CookieRecord {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub is_secure: bool,
    pub is_http_only: bool,
    pub same_site: String,
}

pub struct CookieStore {
    conn: Connection,
    key: [u8; 32],
}

pub fn open(data_dir: &Path) -> Result<CookieStore> {
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
    Ok(CookieStore {
        conn,
        key: crypto::store_key(data_dir)?,
    })
}

pub fn save(store: &CookieStore, cookies: &[CookieRecord]) -> Result<()> {
    if cookies.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now().timestamp();
    let mut stmt = store.conn.prepare_cached(
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
        let value = crypto::encrypt_text(&store.key, &c.value)?;
        stmt.execute(params![
            &c.domain,
            &c.path,
            &c.name,
            &value,
            c.expires,
            c.is_secure as i32,
            c.is_http_only as i32,
            &c.same_site,
            now,
        ])?;
    }
    Ok(())
}

pub fn load_all(store: &CookieStore) -> Result<Vec<CookieRecord>> {
    let now_secs = chrono::Utc::now().timestamp() as f64;
    let mut stmt = store.conn.prepare(
        "SELECT domain, path, name, value, expires,
                is_secure, is_http_only, same_site
         FROM cookies
         WHERE expires < 0 OR expires > ?1
         ORDER BY domain, path, name",
    )?;
    let rows = stmt.query_map(params![now_secs], |row| {
        let value: String = row.get(3)?;
        Ok(CookieRecord {
            domain: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            value: crypto::decrypt_text(&store.key, &value).unwrap_or_default(),
            expires: row.get(4)?,
            is_secure: row.get::<_, i32>(5)? != 0,
            is_http_only: row.get::<_, i32>(6)? != 0,
            same_site: row.get(7)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn purge_expired(store: &CookieStore) -> Result<usize> {
    let cutoff = (chrono::Utc::now().timestamp() as f64) - 86_400.0;
    let n = store.conn.execute(
        "DELETE FROM cookies WHERE expires > 0 AND expires < ?1",
        params![cutoff],
    )?;
    Ok(n)
}

pub fn count(store: &CookieStore) -> usize {
    store
        .conn
        .query_row("SELECT COUNT(*) FROM cookies", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as usize
}
