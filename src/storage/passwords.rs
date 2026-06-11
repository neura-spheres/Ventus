use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::crypto;

#[derive(Serialize, Clone)]
pub struct Credential {
    pub id: String,
    pub origin: String,
    pub username: String,
    pub password: String,
    pub updated_at: i64,
    pub last_used: i64,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn save(
    conn: &Connection,
    key: &[u8; 32],
    origin: &str,
    username: &str,
    password: &str,
) -> Result<()> {
    let enc = crypto::encrypt_text(key, password)?;
    let ts = now();
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM credentials WHERE origin = ?1 AND username = ?2",
            params![origin, username],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        conn.execute(
            "UPDATE credentials SET password = ?1, updated_at = ?2 WHERE id = ?3",
            params![enc, ts, id],
        )?;
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO credentials (id, origin, username, password, created_at, updated_at, last_used) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)",
            params![id, origin, username, enc, ts],
        )?;
    }
    Ok(())
}

pub fn stored_password(
    conn: &Connection,
    key: &[u8; 32],
    origin: &str,
    username: &str,
) -> Result<Option<String>> {
    let enc: Option<String> = conn
        .query_row(
            "SELECT password FROM credentials WHERE origin = ?1 AND username = ?2",
            params![origin, username],
            |row| row.get(0),
        )
        .optional()?;
    match enc {
        Some(e) => Ok(Some(crypto::decrypt_text(key, &e)?)),
        None => Ok(None),
    }
}

pub fn for_origin(conn: &Connection, key: &[u8; 32], origin: &str) -> Result<Vec<Credential>> {
    let mut stmt = conn.prepare(
        "SELECT id, origin, username, password, updated_at, last_used FROM credentials WHERE origin = ?1 ORDER BY last_used DESC, updated_at DESC",
    )?;
    let rows = stmt.query_map(params![origin], row_to_enc)?;
    decrypt_rows(key, rows)
}

pub fn list(conn: &Connection, key: &[u8; 32]) -> Result<Vec<Credential>> {
    let mut stmt = conn.prepare(
        "SELECT id, origin, username, password, updated_at, last_used FROM credentials ORDER BY origin ASC, username ASC",
    )?;
    let rows = stmt.query_map([], row_to_enc)?;
    decrypt_rows(key, rows)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM credentials WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn touch(conn: &Connection, origin: &str, username: &str) -> Result<()> {
    conn.execute(
        "UPDATE credentials SET last_used = ?1 WHERE origin = ?2 AND username = ?3",
        params![now(), origin, username],
    )?;
    Ok(())
}

fn row_to_enc(row: &rusqlite::Row) -> rusqlite::Result<Credential> {
    Ok(Credential {
        id: row.get(0)?,
        origin: row.get(1)?,
        username: row.get(2)?,
        password: row.get(3)?,
        updated_at: row.get(4)?,
        last_used: row.get(5)?,
    })
}

fn decrypt_rows<I>(key: &[u8; 32], rows: I) -> Result<Vec<Credential>>
where
    I: Iterator<Item = rusqlite::Result<Credential>>,
{
    let mut out = Vec::new();
    for row in rows {
        let mut cred = row?;
        cred.password = crypto::decrypt_text(key, &cred.password).unwrap_or_default();
        out.push(cred);
    }
    Ok(out)
}
