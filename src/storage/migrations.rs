use anyhow::Result;
use rusqlite::Connection;

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)?;
    // Add is_incognito column if it doesn't exist (existing databases).
    let _ = conn.execute_batch(
        "ALTER TABLE workspaces ADD COLUMN is_incognito INTEGER NOT NULL DEFAULT 0;",
    );
    let _ = conn
        .execute_batch("ALTER TABLE bookmarks ADD COLUMN icon_only INTEGER NOT NULL DEFAULT 0;");
    Ok(())
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT '📁',
    accent_color TEXT NOT NULL DEFAULT '#8b5cf6',
    position INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    is_incognito INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tabs (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'New Tab',
    pinned INTEGER NOT NULL DEFAULT 0,
    is_essential INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bookmarks (
    id TEXT PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    folder_id TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    icon_only INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (folder_id) REFERENCES bookmark_folders(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS bookmark_folders (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    parent_id TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    workspace_id TEXT,
    visited_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
CREATE INDEX IF NOT EXISTS idx_history_visited ON history(visited_at DESC);

CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,
    filename TEXT NOT NULL,
    local_path TEXT,
    mime_type TEXT,
    total_bytes INTEGER,
    received_bytes INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at INTEGER NOT NULL,
    completed_at INTEGER
);

CREATE TABLE IF NOT EXISTS search_engines (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    url_template TEXT NOT NULL,
    shortcut TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    icon TEXT NOT NULL DEFAULT '🔍',
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ai_chat_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL DEFAULT 'New Chat',
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    page_url TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES ai_chat_sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ai_providers (
    id TEXT PRIMARY KEY NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    base_url TEXT,
    model TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER NOT NULL DEFAULT 2048
);

CREATE TABLE IF NOT EXISTS keyboard_shortcuts (
    action TEXT PRIMARY KEY NOT NULL,
    shortcut TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS credentials (
    id TEXT PRIMARY KEY NOT NULL,
    origin TEXT NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_used INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_credentials_origin ON credentials(origin);
CREATE UNIQUE INDEX IF NOT EXISTS idx_credentials_origin_user ON credentials(origin, username);
"#;
