use crate::browser::{
    downloads::{Download, DownloadStatus},
    search_engine::SearchEngine,
    tab::Tab,
    workspace::Workspace,
};
use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTab {
    pub id: String,
    pub workspace_id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub pinned: bool,
    pub is_essential: bool,
    pub created_at: i64,
    pub last_active_at: i64,
    pub back_stack: Vec<String>,
    pub forward_stack: Vec<String>,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub version: u32,
    pub saved_at: i64,
    pub active_workspace_id: String,
    pub active_tab_id: Option<String>,
    pub workspaces: Vec<Workspace>,
    pub tabs: Vec<SavedTab>,
}

pub fn save_session(
    conn: &Connection,
    workspaces: &[Workspace],
    active_workspace_id: &str,
    tabs: &[Tab],
    active_id: Option<&str>,
) -> Result<()> {
    let state = build_session(workspaces, active_workspace_id, tabs, active_id);
    let json = serde_json::to_string(&state)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM tabs", [])?;
    tx.execute("DELETE FROM workspaces", [])?;
    for ws in &state.workspaces {
        tx.execute(
            "INSERT INTO workspaces(id, name, icon, accent_color, position, created_at, is_incognito) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![ws.id, ws.name, ws.icon, ws.accent_color, ws.position, ws.created_at, ws.is_incognito as i32],
        )?;
    }
    for tab in &state.tabs {
        tx.execute(
            "INSERT INTO tabs(id, workspace_id, url, title, pinned, is_essential, position, created_at, last_active_at) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                tab.id,
                tab.workspace_id,
                tab.url,
                tab.title,
                tab.pinned as i32,
                tab.is_essential as i32,
                tab.position as i64,
                tab.created_at,
                tab.last_active_at,
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO settings(key, value) VALUES('session_state', ?1) ON CONFLICT(key) DO UPDATE SET value=?1",
        [json],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn load_session(conn: &Connection) -> Result<Option<SessionState>> {
    let saved = conn.query_row(
        "SELECT value FROM settings WHERE key = 'session_state'",
        [],
        |row| row.get::<_, String>(0),
    );
    if let Ok(json) = saved {
        if let Ok(state) = serde_json::from_str::<SessionState>(&json) {
            return Ok(normalize_session(state));
        }
    }

    let mut stmt =
        conn.prepare("SELECT id, workspace_id, url, title, pinned, is_essential, position, created_at, last_active_at FROM tabs ORDER BY position ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(SavedTab {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            url: row.get(2)?,
            title: row.get(3)?,
            favicon: None,
            pinned: row.get::<_, i32>(4)? != 0,
            is_essential: row.get::<_, i32>(5)? != 0,
            position: row.get(6)?,
            created_at: row.get(7)?,
            last_active_at: row.get(8)?,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
        })
    })?;
    let tabs = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if tabs.is_empty() {
        return Ok(None);
    }
    let workspaces = list_workspaces(conn).unwrap_or_default();
    let state = SessionState {
        version: 1,
        saved_at: chrono::Utc::now().timestamp_millis(),
        active_workspace_id: String::new(),
        active_tab_id: None,
        workspaces,
        tabs,
    };
    Ok(normalize_session(state))
}

fn build_session(
    workspaces: &[Workspace],
    active_workspace_id: &str,
    tabs: &[Tab],
    active_id: Option<&str>,
) -> SessionState {
    let mut safe_workspaces: Vec<Workspace> = workspaces
        .iter()
        .filter(|ws| safe_id(&ws.id) && !ws.name.trim().is_empty() && !ws.is_incognito)
        .cloned()
        .collect();
    if safe_workspaces.is_empty() {
        safe_workspaces.push(Workspace::default());
    }
    let private_ids: std::collections::HashSet<String> = workspaces
        .iter()
        .filter(|ws| ws.is_incognito)
        .map(|ws| ws.id.clone())
        .collect();
    let workspace_ids: std::collections::HashSet<String> =
        safe_workspaces.iter().map(|ws| ws.id.clone()).collect();
    let fallback_ws = safe_workspaces[0].id.clone();
    let mut safe_tabs = Vec::new();
    for (pos, tab) in tabs.iter().enumerate() {
        if !safe_id(&tab.id) || !safe_url(&tab.url) {
            continue;
        }
        if private_ids.contains(&tab.workspace_id) {
            continue;
        }
        let workspace_id = if workspace_ids.contains(&tab.workspace_id) {
            tab.workspace_id.clone()
        } else {
            fallback_ws.clone()
        };
        safe_tabs.push(SavedTab {
            id: tab.id.clone(),
            workspace_id,
            url: tab.url.clone(),
            title: clean_text(&tab.title, 512),
            favicon: tab.favicon.as_ref().filter(|v| safe_url(v)).cloned(),
            pinned: tab.pinned,
            is_essential: tab.is_essential,
            created_at: tab.created_at,
            last_active_at: if active_id == Some(tab.id.as_str()) {
                i64::MAX
            } else {
                tab.last_active_at
            },
            back_stack: clean_stack(&tab.back_stack),
            forward_stack: clean_stack(&tab.forward_stack),
            position: pos as i32,
        });
    }
    if safe_tabs.is_empty() {
        let mut tab = Tab::new(fallback_ws.clone(), Tab::new_tab_url());
        if let Some(id) = active_id.filter(|id| safe_id(id)) {
            tab.id = id.to_string();
        }
        safe_tabs.push(SavedTab {
            id: tab.id,
            workspace_id: fallback_ws.clone(),
            url: Tab::new_tab_url().to_string(),
            title: "New Tab".to_string(),
            favicon: None,
            pinned: false,
            is_essential: false,
            created_at: tab.created_at,
            last_active_at: i64::MAX,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            position: 0,
        });
    }
    let active_tab_id = active_id
        .filter(|id| safe_tabs.iter().any(|tab| tab.id == *id))
        .map(|id| id.to_string())
        .or_else(|| {
            safe_tabs
                .iter()
                .max_by_key(|tab| tab.last_active_at)
                .map(|tab| tab.id.clone())
        });
    let active_workspace_id = if let Some(ws_id) = active_tab_id
        .as_ref()
        .and_then(|id| safe_tabs.iter().find(|tab| &tab.id == id))
        .map(|tab| tab.workspace_id.clone())
    {
        ws_id
    } else if safe_workspaces
        .iter()
        .any(|ws| ws.id == active_workspace_id)
    {
        active_workspace_id.to_string()
    } else {
        fallback_ws
    };
    SessionState {
        version: 1,
        saved_at: chrono::Utc::now().timestamp_millis(),
        active_workspace_id,
        active_tab_id,
        workspaces: safe_workspaces,
        tabs: safe_tabs,
    }
}

fn normalize_session(mut state: SessionState) -> Option<SessionState> {
    let private_ids: std::collections::HashSet<String> = state
        .workspaces
        .iter()
        .filter(|ws| ws.is_incognito)
        .map(|ws| ws.id.clone())
        .collect();
    state
        .workspaces
        .retain(|ws| safe_id(&ws.id) && !ws.name.trim().is_empty() && !ws.is_incognito);
    if state.workspaces.is_empty() {
        state.workspaces.push(Workspace::default());
    }
    let workspace_ids: std::collections::HashSet<String> =
        state.workspaces.iter().map(|ws| ws.id.clone()).collect();
    let fallback_ws = state.workspaces[0].id.clone();
    state.tabs.retain(|tab| {
        safe_id(&tab.id) && safe_url(&tab.url) && !private_ids.contains(&tab.workspace_id)
    });
    for (pos, tab) in state.tabs.iter_mut().enumerate() {
        if !workspace_ids.contains(&tab.workspace_id) {
            tab.workspace_id = fallback_ws.clone();
        }
        tab.title = clean_text(&tab.title, 512);
        tab.favicon = tab.favicon.as_ref().filter(|v| safe_url(v)).cloned();
        tab.back_stack = clean_stack(&tab.back_stack);
        tab.forward_stack = clean_stack(&tab.forward_stack);
        tab.position = pos as i32;
    }
    if state.tabs.is_empty() {
        return None;
    }
    if !state
        .workspaces
        .iter()
        .any(|ws| ws.id == state.active_workspace_id)
    {
        state.active_workspace_id = state
            .active_tab_id
            .as_ref()
            .and_then(|id| state.tabs.iter().find(|tab| &tab.id == id))
            .map(|tab| tab.workspace_id.clone())
            .unwrap_or(fallback_ws);
    }
    if !state
        .active_tab_id
        .as_ref()
        .map(|id| state.tabs.iter().any(|tab| &tab.id == id))
        .unwrap_or(false)
    {
        state.active_tab_id = state
            .tabs
            .iter()
            .max_by_key(|tab| tab.last_active_at)
            .map(|tab| tab.id.clone());
    }
    if let Some(ws_id) = state
        .active_tab_id
        .as_ref()
        .and_then(|id| state.tabs.iter().find(|tab| &tab.id == id))
        .map(|tab| tab.workspace_id.clone())
    {
        state.active_workspace_id = ws_id;
    }
    Some(state)
}

fn clean_stack(urls: &[String]) -> Vec<String> {
    urls.iter()
        .filter(|url| safe_url(url))
        .take(100)
        .cloned()
        .collect()
}

fn clean_text(text: &str, max: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\t' || *c == '\n')
        .take(max)
        .collect()
}

fn safe_id(id: &str) -> bool {
    !id.trim().is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn safe_url(url: &str) -> bool {
    if url.len() > 4096 {
        return false;
    }
    let trimmed = url.trim();
    if trimmed == Tab::new_tab_url() {
        return true;
    }
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}

pub fn list_workspaces(conn: &Connection) -> Result<Vec<Workspace>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, accent_color, position, created_at, is_incognito FROM workspaces ORDER BY position ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        let is_incognito_i: i32 = row.get(6).unwrap_or(0);
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            accent_color: row.get(3)?,
            created_at: row.get(5)?,
            position: row.get(4)?,
            is_incognito: is_incognito_i != 0,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn save_workspace(conn: &Connection, ws: &Workspace) -> Result<()> {
    conn.execute(
        "INSERT INTO workspaces(id, name, icon, accent_color, position, created_at, is_incognito) VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(id) DO UPDATE SET name=?2, icon=?3, accent_color=?4, position=?5, is_incognito=?7",
        params![ws.id, ws.name, ws.icon, ws.accent_color, ws.position, ws.created_at, ws.is_incognito as i32],
    )?;
    Ok(())
}

pub fn delete_workspace(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub favicon: Option<String>,
    pub folder_id: Option<String>,
    pub position: i32,
    pub created_at: i64,
    pub icon_only: bool,
}

pub fn add_bookmark(
    conn: &Connection,
    url: &str,
    title: &str,
    favicon: Option<&str>,
    folder_id: Option<&str>,
) -> Result<Bookmark> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let favicon = favicon.filter(|url| safe_url(url));
    // New bookmarks go to the top of the list (smallest position).
    let position: i32 = conn
        .query_row(
            "SELECT COALESCE(MIN(position), 0) - 1 FROM bookmarks",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO bookmarks(id, url, title, favicon, folder_id, position, created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, url, title, favicon, folder_id, position, now],
    )?;
    Ok(Bookmark {
        id,
        url: url.to_string(),
        title: title.to_string(),
        favicon: favicon.map(String::from),
        folder_id: folder_id.map(String::from),
        position,
        created_at: now,
        icon_only: false,
    })
}

pub fn set_bookmark_favicon_for_url(
    conn: &Connection,
    url: &str,
    favicon: Option<&str>,
) -> Result<()> {
    let Some(favicon) = favicon.filter(|url| safe_url(url)) else {
        return Ok(());
    };
    conn.execute(
        "UPDATE bookmarks SET favicon = ?1 WHERE url = ?2",
        params![favicon, url],
    )?;
    Ok(())
}

pub fn rename_bookmark(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE bookmarks SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    Ok(())
}

pub fn set_bookmark_icon_only(conn: &Connection, id: &str, icon_only: bool) -> Result<()> {
    conn.execute(
        "UPDATE bookmarks SET icon_only = ?1 WHERE id = ?2",
        params![icon_only, id],
    )?;
    Ok(())
}

pub fn remove_bookmark_by_url(conn: &Connection, url: &str) -> Result<()> {
    conn.execute("DELETE FROM bookmarks WHERE url = ?1", [url])?;
    Ok(())
}

/// Reorder a bookmark to sit before `before_id` (or at the end when None). Rewrites every
/// bookmark's position to a normalized 0..N sequence reflecting the new order.
pub fn move_bookmark(conn: &Connection, id: &str, before_id: Option<&str>) -> Result<()> {
    let mut ids: Vec<String> = list_bookmarks(conn)?.into_iter().map(|b| b.id).collect();
    let Some(from) = ids.iter().position(|b| b == id) else {
        return Ok(());
    };
    let moved = ids.remove(from);
    let insert_at = before_id
        .and_then(|bid| ids.iter().position(|b| b == bid))
        .unwrap_or(ids.len());
    ids.insert(insert_at, moved);
    for (pos, bid) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE bookmarks SET position = ?1 WHERE id = ?2",
            params![pos as i32, bid],
        )?;
    }
    Ok(())
}

pub fn is_bookmarked(conn: &Connection, url: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bookmarks WHERE url = ?1",
        [url],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn list_bookmarks(conn: &Connection) -> Result<Vec<Bookmark>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, favicon, folder_id, position, created_at, icon_only FROM bookmarks ORDER BY position ASC, created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Bookmark {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            favicon: row.get(3)?,
            folder_id: row.get(4)?,
            position: row.get(5)?,
            created_at: row.get(6)?,
            icon_only: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn search_bookmarks(conn: &Connection, q: &str) -> Result<Vec<Bookmark>> {
    let pattern = format!("%{}%", q);
    let mut stmt = conn.prepare(
        "SELECT id, url, title, favicon, folder_id, position, created_at, icon_only FROM bookmarks WHERE title LIKE ?1 OR url LIKE ?1 ORDER BY position ASC, created_at DESC LIMIT 50"
    )?;
    let rows = stmt.query_map([&pattern], |row| {
        Ok(Bookmark {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            favicon: row.get(3)?,
            folder_id: row.get(4)?,
            position: row.get(5)?,
            created_at: row.get(6)?,
            icon_only: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BookmarkFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub position: i32,
    pub created_at: i64,
}

pub fn add_bookmark_folder(conn: &Connection, name: &str) -> Result<BookmarkFolder> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let position: i32 = conn
        .query_row(
            "SELECT COALESCE(MIN(position), 0) - 1 FROM bookmark_folders",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO bookmark_folders(id, name, parent_id, position, created_at) VALUES(?1,?2,NULL,?3,?4)",
        params![id, name, position, now],
    )?;
    Ok(BookmarkFolder {
        id,
        name: name.to_string(),
        parent_id: None,
        position,
        created_at: now,
    })
}

pub fn list_bookmark_folders(conn: &Connection) -> Result<Vec<BookmarkFolder>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, parent_id, position, created_at FROM bookmark_folders ORDER BY position ASC, created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BookmarkFolder {
            id: row.get(0)?,
            name: row.get(1)?,
            parent_id: row.get(2)?,
            position: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn set_bookmark_folder(
    conn: &Connection,
    bookmark_id: &str,
    folder_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE bookmarks SET folder_id = ?1 WHERE id = ?2",
        params![folder_id, bookmark_id],
    )?;
    Ok(())
}

pub fn delete_bookmark_folder(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE bookmarks SET folder_id = NULL WHERE folder_id = ?1",
        [id],
    )?;
    conn.execute("DELETE FROM bookmark_folders WHERE id = ?1", [id])?;
    Ok(())
}

pub fn rename_bookmark_folder(conn: &Connection, id: &str, name: &str) -> Result<()> {
    conn.execute(
        "UPDATE bookmark_folders SET name = ?1 WHERE id = ?2",
        params![name, id],
    )?;
    Ok(())
}

pub fn move_bookmark_folder(conn: &Connection, id: &str, before_id: Option<&str>) -> Result<()> {
    let mut ids: Vec<String> = list_bookmark_folders(conn)?
        .into_iter()
        .map(|f| f.id)
        .collect();
    let Some(from) = ids.iter().position(|f| f == id) else {
        return Ok(());
    };
    let moved = ids.remove(from);
    let insert_at = before_id
        .and_then(|bid| ids.iter().position(|f| f == bid))
        .unwrap_or(ids.len());
    ids.insert(insert_at, moved);
    for (pos, fid) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE bookmark_folders SET position = ?1 WHERE id = ?2",
            params![pos as i32, fid],
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub workspace_id: Option<String>,
    pub visited_at: i64,
}

pub const HISTORY_LIMIT: i64 = 1000;

pub fn add_history(
    conn: &Connection,
    url: &str,
    title: &str,
    workspace_id: Option<&str>,
) -> Result<i64> {
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO history(url, title, workspace_id, visited_at) VALUES(?1,?2,?3,?4)",
        params![url, title, workspace_id, now],
    )?;
    let row_id = conn.last_insert_rowid();
    conn.execute(
        "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY visited_at DESC LIMIT ?1)",
        params![HISTORY_LIMIT],
    )?;
    Ok(row_id)
}

pub fn update_history_title(conn: &Connection, id: i64, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE history SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    Ok(())
}

pub fn list_history(conn: &Connection, limit: usize) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, workspace_id, visited_at FROM history ORDER BY visited_at DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            workspace_id: row.get(3)?,
            visited_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn search_history(conn: &Connection, q: &str) -> Result<Vec<HistoryEntry>> {
    let pattern = format!("%{}%", q);
    let mut stmt = conn.prepare(
        "SELECT id, url, title, workspace_id, visited_at FROM history WHERE url LIKE ?1 OR title LIKE ?1 ORDER BY visited_at DESC LIMIT 50"
    )?;
    let rows = stmt.query_map([&pattern], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            workspace_id: row.get(3)?,
            visited_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Returns history entries ranked by frecency (visit count × recency decay).
/// When query is empty, returns the most-visited sites overall.
pub fn search_history_frecency(
    conn: &Connection,
    q: &str,
    limit: usize,
) -> Result<Vec<HistoryEntry>> {
    let pattern = if q.is_empty() {
        "%".to_string()
    } else {
        format!("%{}%", q)
    };
    let mut stmt = conn.prepare(
        "SELECT MAX(id), url,
                COALESCE(MAX(CASE WHEN title != '' THEN title ELSE NULL END), url),
                NULL, MAX(visited_at)
         FROM history
         WHERE (url LIKE ?1 OR title LIKE ?1)
           AND url NOT LIKE 'neura://%'
           AND url NOT LIKE 'about:%'
         GROUP BY url
         ORDER BY COUNT(*) * 1.0 / (1.0 + (strftime('%s','now') - MAX(visited_at)/1000) / 86400.0) DESC
         LIMIT ?2"
    )?;
    let rows = stmt.query_map(rusqlite::params![pattern, limit as i64], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            workspace_id: row.get(3)?,
            visited_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone)]
pub struct HistoryCandidate {
    pub url: String,
    pub title: String,
    pub visits: i64,
    pub last: i64,
}

pub fn history_candidates(
    conn: &Connection,
    q: &str,
    limit: usize,
) -> Result<Vec<HistoryCandidate>> {
    let pattern = if q.is_empty() {
        "%".to_string()
    } else {
        format!("%{}%", q)
    };
    let mut stmt = conn.prepare(
        "SELECT url,
                COALESCE(MAX(CASE WHEN title != '' THEN title ELSE NULL END), url),
                COUNT(*), MAX(visited_at)
         FROM history
         WHERE (url LIKE ?1 OR title LIKE ?1)
           AND url NOT LIKE 'neura://%'
           AND url NOT LIKE 'about:%'
         GROUP BY url
         ORDER BY COUNT(*) * 1.0 / (1.0 + (strftime('%s','now') - MAX(visited_at)/1000) / 86400.0) DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![pattern, limit as i64], |row| {
        Ok(HistoryCandidate {
            url: row.get(0)?,
            title: row.get(1)?,
            visits: row.get(2)?,
            last: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn history_stats(conn: &Connection, url: &str) -> Result<Option<HistoryCandidate>> {
    let mut stmt = conn.prepare(
        "SELECT url,
                COALESCE(MAX(CASE WHEN title != '' THEN title ELSE NULL END), url),
                COUNT(*), MAX(visited_at)
         FROM history WHERE url = ?1 GROUP BY url",
    )?;
    let mut rows = stmt.query([url])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(HistoryCandidate {
        url: row.get(0)?,
        title: row.get(1)?,
        visits: row.get(2)?,
        last: row.get(3)?,
    }))
}

pub fn clear_history(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM history", [])?;
    Ok(())
}

pub fn delete_history_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM history WHERE id = ?1", [id])?;
    Ok(())
}

fn download_status_to_str(status: &DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Pending => "pending",
        DownloadStatus::Downloading => "downloading",
        DownloadStatus::Paused => "paused",
        DownloadStatus::Complete => "complete",
        DownloadStatus::Failed => "failed",
        DownloadStatus::Cancelled => "cancelled",
    }
}

fn download_status_from_str(status: &str) -> DownloadStatus {
    match status {
        "downloading" => DownloadStatus::Downloading,
        "paused" => DownloadStatus::Paused,
        "complete" => DownloadStatus::Complete,
        "failed" => DownloadStatus::Failed,
        "cancelled" => DownloadStatus::Cancelled,
        _ => DownloadStatus::Pending,
    }
}

pub fn save_download(conn: &Connection, download: &Download) -> Result<()> {
    conn.execute(
        "INSERT INTO downloads(id, url, filename, local_path, mime_type, total_bytes, received_bytes, status, started_at, completed_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
         ON CONFLICT(id) DO UPDATE SET
           url=excluded.url,
           filename=excluded.filename,
           local_path=excluded.local_path,
           mime_type=excluded.mime_type,
           total_bytes=excluded.total_bytes,
           received_bytes=excluded.received_bytes,
           status=excluded.status,
           started_at=excluded.started_at,
           completed_at=excluded.completed_at",
        params![
            download.id,
            download.url,
            download.filename,
            download.local_path,
            download.mime_type,
            download.total_bytes.map(|v| v as i64),
            download.received_bytes as i64,
            download_status_to_str(&download.status),
            download.started_at,
            download.completed_at,
        ],
    )?;
    Ok(())
}

pub fn list_downloads(conn: &Connection, limit: usize) -> Result<Vec<Download>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, filename, local_path, mime_type, total_bytes, received_bytes, status, started_at, completed_at
         FROM downloads ORDER BY started_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        let total_bytes: Option<i64> = row.get(5)?;
        let received_bytes: i64 = row.get(6)?;
        let status: String = row.get(7)?;
        Ok(Download {
            id: row.get(0)?,
            url: row.get(1)?,
            filename: row.get(2)?,
            local_path: row.get(3)?,
            mime_type: row.get(4)?,
            total_bytes: total_bytes.map(|v| v.max(0) as u64),
            received_bytes: received_bytes.max(0) as u64,
            status: download_status_from_str(&status),
            started_at: row.get(8)?,
            completed_at: row.get(9)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn delete_download(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM downloads WHERE id = ?1", [id])?;
    Ok(())
}

pub fn clear_downloads(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM downloads", [])?;
    Ok(())
}

pub fn fail_stale_downloads(conn: &Connection) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE downloads SET status = 'failed', completed_at = ?1 WHERE status IN ('downloading','pending','paused')",
        params![now],
    )?;
    Ok(())
}

pub fn seed_search_engines(conn: &Connection) -> Result<()> {
    for (pos, engine) in SearchEngine::builtin_engines().iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO search_engines(id, name, url_template, shortcut, is_default, is_builtin, icon, position) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![engine.id, engine.name, engine.url_template, engine.shortcut, engine.is_default, engine.is_builtin, engine.icon, pos as i64],
        )?;
        conn.execute(
            "UPDATE search_engines SET name = ?2, url_template = ?3, shortcut = ?4, is_builtin = ?5, icon = ?6, position = ?7 WHERE id = ?1",
            params![engine.id, engine.name, engine.url_template, engine.shortcut, engine.is_builtin, engine.icon, pos as i64],
        )?;
    }
    let default_id: Option<String> = conn
        .query_row(
            "SELECT id FROM search_engines WHERE is_default = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    if default_id.as_deref().is_none() || default_id.as_deref() == Some("duckduckgo") {
        set_default_search_engine(conn, "google")?;
    }
    Ok(())
}

pub fn list_search_engines(conn: &Connection) -> Result<Vec<SearchEngine>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url_template, shortcut, is_default, is_builtin, icon FROM search_engines ORDER BY position ASC"
    )?;
    let rows = stmt.query_map([], |row| {
        let is_default: i64 = row.get(4)?;
        let is_builtin: i64 = row.get(5)?;
        Ok(SearchEngine {
            id: row.get(0)?,
            name: row.get(1)?,
            url_template: row.get(2)?,
            shortcut: row.get(3)?,
            is_default: is_default != 0,
            is_builtin: is_builtin != 0,
            icon: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get_default_search_engine(conn: &Connection) -> Result<Option<SearchEngine>> {
    let engines = list_search_engines(conn)?;
    Ok(engines.into_iter().find(|e| e.is_default))
}

pub fn set_default_search_engine(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("UPDATE search_engines SET is_default = 0", [])?;
    conn.execute(
        "UPDATE search_engines SET is_default = 1 WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{database, migrations};

    #[test]
    fn session_round_trips_browser_state() {
        let conn = database::in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let ws = Workspace::default();
        let mut tab = Tab::new(ws.id.clone(), "https://example.com");
        tab.title = "Example".to_string();
        tab.pinned = true;
        tab.back_stack = vec![Tab::new_tab_url().to_string()];
        tab.forward_stack = vec!["https://neuraspheres.com".to_string()];
        tab.sync_nav_flags();

        save_session(&conn, &[ws.clone()], &ws.id, &[tab.clone()], Some(&tab.id)).unwrap();
        let saved = load_session(&conn).unwrap().unwrap();

        assert_eq!(saved.active_workspace_id, ws.id);
        assert_eq!(saved.active_tab_id, Some(tab.id.clone()));
        assert_eq!(saved.workspaces.len(), 1);
        assert_eq!(saved.tabs.len(), 1);
        assert_eq!(saved.tabs[0].url, "https://example.com");
        assert_eq!(saved.tabs[0].title, "Example");
        assert!(saved.tabs[0].pinned);
        assert_eq!(saved.tabs[0].back_stack, vec![Tab::new_tab_url()]);
        assert_eq!(
            saved.tabs[0].forward_stack,
            vec!["https://neuraspheres.com"]
        );
    }

    #[test]
    fn session_filters_unsafe_restore_urls() {
        let conn = database::in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let ws = Workspace::default();
        let bad = Tab::new(ws.id.clone(), "javascript:alert(1)");
        let good = Tab::new(ws.id.clone(), "https://neuraspheres.com");

        save_session(
            &conn,
            &[ws.clone()],
            &ws.id,
            &[bad.clone(), good.clone()],
            Some(&bad.id),
        )
        .unwrap();
        let saved = load_session(&conn).unwrap().unwrap();

        assert_eq!(saved.tabs.len(), 1);
        assert_eq!(saved.tabs[0].id, good.id);
        assert_eq!(saved.active_tab_id, Some(good.id));
    }

    #[test]
    fn session_filters_incognito_state() {
        let conn = database::in_memory().unwrap();
        migrations::run(&conn).unwrap();
        let normal_ws = Workspace::default();
        let private_ws = Workspace::new("Private", true, None);
        let normal_tab = Tab::new(normal_ws.id.clone(), "https://neuraspheres.com");
        let private_tab = Tab::new(private_ws.id.clone(), "https://google.com");

        save_session(
            &conn,
            &[normal_ws.clone(), private_ws.clone()],
            &private_ws.id,
            &[normal_tab.clone(), private_tab.clone()],
            Some(&private_tab.id),
        )
        .unwrap();
        let saved = load_session(&conn).unwrap().unwrap();

        assert_eq!(saved.workspaces.len(), 1);
        assert_eq!(saved.workspaces[0].id, normal_ws.id);
        assert_eq!(saved.tabs.len(), 1);
        assert_eq!(saved.tabs[0].id, normal_tab.id);
        assert_eq!(saved.active_workspace_id, normal_ws.id);
        assert_eq!(saved.active_tab_id, Some(normal_tab.id));
    }

    #[test]
    fn bookmark_stores_and_updates_favicon() {
        let conn = database::in_memory().unwrap();
        migrations::run(&conn).unwrap();

        add_bookmark(
            &conn,
            "https://mail.google.com/mail/u/0/",
            "Gmail",
            Some("https://ssl.gstatic.com/ui/v1/icons/mail/rfr/gmail.ico"),
            None,
        )
        .unwrap();
        let saved = list_bookmarks(&conn).unwrap();
        assert_eq!(
            saved[0].favicon.as_deref(),
            Some("https://ssl.gstatic.com/ui/v1/icons/mail/rfr/gmail.ico")
        );

        set_bookmark_favicon_for_url(
            &conn,
            "https://mail.google.com/mail/u/0/",
            Some("https://www.gstatic.com/images/branding/product/1x/gmail_2020q4_32dp.png"),
        )
        .unwrap();
        let updated = list_bookmarks(&conn).unwrap();
        assert_eq!(
            updated[0].favicon.as_deref(),
            Some("https://www.gstatic.com/images/branding/product/1x/gmail_2020q4_32dp.png")
        );
    }
}
