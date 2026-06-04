use crate::browser::{tab::Tab, workspace::Workspace};
use std::collections::{HashMap, VecDeque};

const MAX_CLOSED_TABS: usize = 20;
const MAX_NAV_STACK: usize = 100;

pub struct TabManager {
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: String,
    pub tabs: Vec<Tab>,
    pub active_tab_id: Option<String>,
    pub closed_tab_urls: VecDeque<String>,
}

impl TabManager {
    pub fn new() -> Self {
        let default_ws = Workspace::default();
        let ws_id = default_ws.id.clone();
        let initial_tab = Tab::new(ws_id.clone(), Tab::new_tab_url());
        let tab_id = initial_tab.id.clone();

        Self {
            workspaces: vec![default_ws],
            active_workspace_id: ws_id,
            tabs: vec![initial_tab],
            active_tab_id: Some(tab_id),
            closed_tab_urls: VecDeque::new(),
        }
    }

    pub fn tab_is_incognito(&self, tab_id: &str) -> bool {
        let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) else {
            return false;
        };
        self.workspaces
            .iter()
            .find(|w| w.id == tab.workspace_id)
            .map(|w| w.is_incognito)
            .unwrap_or(false)
    }

    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|w| w.id == self.active_workspace_id)
    }

    pub fn add_workspace(
        &mut self,
        name: impl Into<String>,
        is_incognito: bool,
        icon: Option<String>,
    ) -> &mut Workspace {
        let mut ws = Workspace::new(name, is_incognito, icon);
        ws.position = self.workspaces.len() as i32;
        self.workspaces.push(ws);
        let idx = self.workspaces.len() - 1;
        let ws_id = self.workspaces[idx].id.clone();
        let tab = Tab::new(ws_id.clone(), Tab::new_tab_url());
        self.tabs.push(tab);
        &mut self.workspaces[idx]
    }

    pub fn switch_workspace(&mut self, id: &str) -> bool {
        if self.workspaces.iter().any(|w| w.id == id) {
            self.active_workspace_id = id.to_string();
            let last_active = self
                .workspace_tabs(id)
                .max_by_key(|t| t.last_active_at)
                .map(|t| t.id.clone());
            self.active_tab_id = last_active;
            true
        } else {
            false
        }
    }

    pub fn delete_workspace(&mut self, id: &str) -> bool {
        if self.workspaces.len() <= 1 {
            return false;
        }
        let pos = self.workspaces.iter().position(|w| w.id == id);
        if let Some(idx) = pos {
            self.workspaces.remove(idx);
            self.tabs.retain(|t| t.workspace_id != id);
            if self.active_workspace_id == id {
                let new_ws_id = self.workspaces[0].id.clone();
                let new_tab_id = self
                    .tabs
                    .iter()
                    .find(|t| t.workspace_id == new_ws_id)
                    .map(|t| t.id.clone());
                self.active_workspace_id = new_ws_id;
                self.active_tab_id = new_tab_id;
            }
            true
        } else {
            false
        }
    }

    pub fn workspace_tabs<'a>(&'a self, workspace_id: &'a str) -> impl Iterator<Item = &'a Tab> {
        self.tabs
            .iter()
            .filter(move |t| t.workspace_id == workspace_id)
    }

    pub fn active_workspace_tabs(&self) -> Vec<&Tab> {
        let ws_id = self.active_workspace_id.as_str();
        let mut pinned: Vec<&Tab> = self
            .tabs
            .iter()
            .filter(|t| t.workspace_id == ws_id && t.pinned)
            .collect();
        let regular: Vec<&Tab> = self
            .tabs
            .iter()
            .filter(|t| t.workspace_id == ws_id && !t.pinned)
            .collect();
        pinned.extend(regular);
        pinned
    }

    pub fn workspace_tab_counts(&self) -> HashMap<String, usize> {
        self.workspaces
            .iter()
            .map(|ws| {
                let count = self
                    .tabs
                    .iter()
                    .filter(|tab| tab.workspace_id == ws.id)
                    .count();
                (ws.id.clone(), count)
            })
            .collect()
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab_id
            .as_ref()
            .and_then(|id| self.tabs.iter().find(|t| &t.id == id))
    }

    pub fn get_tab(&self, id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn get_tab_mut(&mut self, id: &str) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn new_tab(&mut self, url: Option<&str>) -> &Tab {
        let ws_id = self.active_workspace_id.clone();
        let url = url.unwrap_or(Tab::new_tab_url());
        let tab = Tab::new(ws_id, url);
        let id = tab.id.clone();
        self.tabs.push(tab);
        self.active_tab_id = Some(id.clone());
        self.tabs.iter().find(|t| t.id == id).unwrap()
    }

    pub fn close_tab(&mut self, id: &str) -> Option<Tab> {
        let pos = self.tabs.iter().position(|t| t.id == id)?;
        let tab = self.tabs.remove(pos);

        if !tab.is_neura_page() {
            self.closed_tab_urls.push_front(tab.url.clone());
            if self.closed_tab_urls.len() > MAX_CLOSED_TABS {
                self.closed_tab_urls.pop_back();
            }
        }

        let ws_id = self.active_workspace_id.clone();
        let ws_tabs: Vec<_> = self.workspace_tabs(&ws_id).map(|t| t.id.clone()).collect();
        if ws_tabs.is_empty() {
            let new_tab = Tab::new(ws_id, Tab::new_tab_url());
            let new_id = new_tab.id.clone();
            self.tabs.push(new_tab);
            self.active_tab_id = Some(new_id);
        } else if self.active_tab_id.as_deref() == Some(id) {
            let new_idx = pos.saturating_sub(1).min(ws_tabs.len() - 1);
            self.active_tab_id = Some(ws_tabs[new_idx].clone());
        }

        Some(tab)
    }

    pub fn switch_tab(&mut self, id: &str) -> bool {
        if self.tabs.iter().any(|t| t.id == id) {
            let now = chrono::Utc::now().timestamp_millis();
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                tab.last_active_at = now;
            }
            self.active_tab_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn reopen_closed_tab(&mut self) -> Option<String> {
        self.closed_tab_urls.pop_front()
    }

    pub fn pin_tab(&mut self, id: &str, pinned: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.pinned = pinned;
        }
    }

    /// Move `id` to sit right before `before` in the tab list (or to the end of its workspace
    /// when `before` is None / not found). The display sorts pinned tabs first, so reordering
    /// stays within a tab's own pinned/regular group.
    pub fn move_tab(&mut self, id: &str, before: Option<&str>) {
        let Some(from) = self.tabs.iter().position(|t| t.id == id) else {
            return;
        };
        let tab = self.tabs.remove(from);
        let insert_at = before
            .and_then(|bid| self.tabs.iter().position(|t| t.id == bid))
            .unwrap_or(self.tabs.len());
        self.tabs.insert(insert_at, tab);
    }

    pub fn visit_tab(&mut self, id: &str, url: &str, title: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if tab.url != url {
                push_nav_url(&mut tab.back_stack, &tab.url);
                tab.forward_stack.clear();
            }
            tab.url = url.to_string();
            tab.title = if title.is_empty() {
                url.to_string()
            } else {
                title.to_string()
            };
            tab.status = crate::browser::tab::TabStatus::Complete;
            tab.last_active_at = chrono::Utc::now().timestamp_millis();
            tab.sync_nav_flags();
        }
    }

    pub fn replace_tab_nav(&mut self, id: &str, url: &str, title: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.url = url.to_string();
            if !title.is_empty() {
                tab.title = title.to_string();
            }
            tab.status = crate::browser::tab::TabStatus::Complete;
            tab.last_active_at = chrono::Utc::now().timestamp_millis();
            tab.sync_nav_flags();
        }
    }

    pub fn go_back(&mut self, id: &str) -> Option<String> {
        let tab = self.tabs.iter_mut().find(|t| t.id == id)?;
        let target = tab.back_stack.pop()?;
        push_nav_url(&mut tab.forward_stack, &tab.url);
        tab.url = target.clone();
        tab.title = target.clone();
        tab.status = crate::browser::tab::TabStatus::Loading;
        tab.last_active_at = chrono::Utc::now().timestamp_millis();
        tab.sync_nav_flags();
        Some(target)
    }

    pub fn go_forward(&mut self, id: &str) -> Option<String> {
        let tab = self.tabs.iter_mut().find(|t| t.id == id)?;
        let target = tab.forward_stack.pop()?;
        push_nav_url(&mut tab.back_stack, &tab.url);
        tab.url = target.clone();
        tab.title = target.clone();
        tab.status = crate::browser::tab::TabStatus::Loading;
        tab.last_active_at = chrono::Utc::now().timestamp_millis();
        tab.sync_nav_flags();
        Some(target)
    }

    pub fn set_tab_loading(&mut self, id: &str, loading: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.status = if loading {
                crate::browser::tab::TabStatus::Loading
            } else {
                crate::browser::tab::TabStatus::Complete
            };
        }
    }

    pub fn tabs_json(&self) -> String {
        let tabs = self.active_workspace_tabs();
        serde_json::to_string(&tabs).unwrap_or_default()
    }

    pub fn workspaces_json(&self) -> String {
        serde_json::to_string(&self.workspaces).unwrap_or_default()
    }

    pub fn state_json(&self) -> String {
        let active_tab = self.active_tab();
        serde_json::to_string(&serde_json::json!({
            "tabs": self.active_workspace_tabs(),
            "workspaces": self.workspaces,
            "active_tab_id": self.active_tab_id,
            "active_workspace_id": self.active_workspace_id,
            "active_url": active_tab.map(|t| t.url.as_str()).unwrap_or(""),
            "active_title": active_tab.map(|t| t.title.as_str()).unwrap_or(""),
            "can_go_back": active_tab.map(|t| t.can_go_back).unwrap_or(false),
            "can_go_fwd": active_tab.map(|t| t.can_go_forward).unwrap_or(false),
            "is_loading": active_tab.map(|t| t.status == crate::browser::tab::TabStatus::Loading).unwrap_or(false),
        })).unwrap_or_default()
    }
}

fn push_nav_url(stack: &mut Vec<String>, url: &str) {
    if url.trim().is_empty() {
        return;
    }
    if stack.last().map(|last| last == url).unwrap_or(false) {
        return;
    }
    stack.push(url.to_string());
    if stack.len() > MAX_NAV_STACK {
        stack.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_id(tabs: &TabManager) -> String {
        tabs.active_tab_id.clone().unwrap()
    }

    #[test]
    fn visit_tab_tracks_back_and_forward() {
        let mut tabs = TabManager::new();
        let id = active_id(&tabs);

        tabs.visit_tab(&id, "https://example.com", "");
        tabs.visit_tab(&id, "https://neuraspheres.com", "");

        let tab = tabs.get_tab(&id).unwrap();
        assert!(tab.can_go_back);
        assert!(!tab.can_go_forward);

        let back = tabs.go_back(&id).unwrap();
        assert_eq!(back, "https://example.com");

        let tab = tabs.get_tab(&id).unwrap();
        assert!(tab.can_go_back);
        assert!(tab.can_go_forward);

        let forward = tabs.go_forward(&id).unwrap();
        assert_eq!(forward, "https://neuraspheres.com");

        let tab = tabs.get_tab(&id).unwrap();
        assert!(tab.can_go_back);
        assert!(!tab.can_go_forward);
    }

    #[test]
    fn first_page_can_go_back_to_newtab() {
        let mut tabs = TabManager::new();
        let id = active_id(&tabs);

        tabs.visit_tab(&id, "https://example.com", "");

        let tab = tabs.get_tab(&id).unwrap();
        assert_eq!(tab.back_stack, vec!["neura://newtab"]);
        assert!(tab.can_go_back);

        let back = tabs.go_back(&id).unwrap();
        assert_eq!(back, "neura://newtab");
    }

    #[test]
    fn repeated_url_does_not_duplicate_history() {
        let mut tabs = TabManager::new();
        let id = active_id(&tabs);

        tabs.visit_tab(&id, "https://example.com", "");
        tabs.visit_tab(&id, "https://example.com", "");

        let tab = tabs.get_tab(&id).unwrap();
        assert_eq!(tab.back_stack.len(), 1);
        assert!(!tab.can_go_forward);
    }

    #[test]
    fn replace_tab_nav_does_not_add_back_entry() {
        let mut tabs = TabManager::new();
        let id = active_id(&tabs);

        tabs.visit_tab(&id, "https://youtube.com", "");
        tabs.replace_tab_nav(&id, "https://www.youtube.com", "YouTube");

        let tab = tabs.get_tab(&id).unwrap();
        assert_eq!(tab.back_stack, vec!["neura://newtab"]);
        assert_eq!(tab.url, "https://www.youtube.com");
        assert!(tab.can_go_back);
        assert!(!tab.can_go_forward);
    }

    #[test]
    fn youtube_video_back_returns_to_youtube_home() {
        let mut tabs = TabManager::new();
        let id = active_id(&tabs);

        tabs.visit_tab(&id, "https://www.youtube.com", "YouTube");
        tabs.visit_tab(&id, "https://www.youtube.com/watch?v=abc", "Video");

        let back = tabs.go_back(&id).unwrap();
        assert_eq!(back, "https://www.youtube.com");

        let tab = tabs.get_tab(&id).unwrap();
        assert_eq!(
            tab.forward_stack,
            vec!["https://www.youtube.com/watch?v=abc"]
        );
    }

    #[test]
    fn workspace_tab_counts_include_inactive_workspaces() {
        let mut tabs = TabManager::new();
        let main_id = tabs.active_workspace_id.clone();
        let work_id = tabs.add_workspace("Work", false, None).id.clone();

        tabs.switch_workspace(&main_id);
        tabs.new_tab(Some("https://neuraspheres.com"));
        tabs.switch_workspace(&work_id);
        tabs.new_tab(Some("https://example.com"));

        let counts = tabs.workspace_tab_counts();
        assert_eq!(counts.get(&main_id), Some(&2));
        assert_eq!(counts.get(&work_id), Some(&2));
    }
}
