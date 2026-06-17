use super::provider::ChatMessage;
use anyhow::Result;
use rusqlite::{params, Connection};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub page_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ChatSession {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Chat".to_string(),
            provider: provider.into(),
            model: model.into(),
            page_url: None,
            created_at: now,
            updated_at: now,
        }
    }
}

pub fn save_session(conn: &Connection, session: &ChatSession) -> Result<()> {
    conn.execute(
        "INSERT INTO ai_chat_sessions(id, title, provider, model, page_url, created_at, updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(id) DO UPDATE SET title=?2, updated_at=?7",
        params![session.id, session.title, session.provider, session.model, session.page_url, session.created_at, session.updated_at],
    )?;
    Ok(())
}

pub fn save_message(conn: &Connection, session_id: &str, message: &ChatMessage) -> Result<()> {
    let role = match message.role {
        super::provider::ChatRole::System => "system",
        super::provider::ChatRole::User => "user",
        super::provider::ChatRole::Assistant => "assistant",
        super::provider::ChatRole::Tool => "tool",
    };
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO ai_chat_messages(session_id, role, content, created_at) VALUES(?1,?2,?3,?4)",
        params![session_id, role, message.content, now],
    )?;
    Ok(())
}

pub fn list_sessions(conn: &Connection, limit: usize) -> Result<Vec<ChatSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, provider, model, page_url, created_at, updated_at FROM ai_chat_sessions ORDER BY updated_at DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(ChatSession {
            id: row.get(0)?,
            title: row.get(1)?,
            provider: row.get(2)?,
            model: row.get(3)?,
            page_url: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn load_messages(conn: &Connection, session_id: &str) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare(
        "SELECT role, content FROM ai_chat_messages WHERE session_id=?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        let role: String = row.get(0)?;
        let content: String = row.get(1)?;
        Ok((role, content))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (role, content) = r?;
        out.push(match role.as_str() {
            "assistant" => ChatMessage::assistant(content),
            "system" => ChatMessage::system(content),
            _ => ChatMessage::user(content),
        });
    }
    Ok(out)
}

pub fn delete_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM ai_chat_messages WHERE session_id=?1",
        params![id],
    )?;
    conn.execute("DELETE FROM ai_chat_sessions WHERE id=?1", params![id])?;
    Ok(())
}

pub fn clear_history(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM ai_chat_messages", [])?;
    conn.execute("DELETE FROM ai_chat_sessions", [])?;
    Ok(())
}
