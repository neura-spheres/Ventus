use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TabStatus {
    Loading,
    Complete,
    Error,
}

impl Default for TabStatus {
    fn default() -> Self {
        TabStatus::Complete
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub workspace_id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub status: TabStatus,
    pub pinned: bool,
    pub is_essential: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub nav_fwd_depth: i32,
    pub back_stack: Vec<String>,
    pub forward_stack: Vec<String>,
    pub created_at: i64,
    pub last_active_at: i64,
    pub is_audio_playing: bool,
    pub is_muted: bool,
}

impl Tab {
    pub fn new(workspace_id: impl Into<String>, url: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let url = url.into();
        let is_neura_page = url.starts_with("neura://");
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            url,
            title: "New Tab".to_string(),
            favicon: None,
            status: if is_neura_page {
                TabStatus::Complete
            } else {
                TabStatus::Loading
            },
            pinned: false,
            is_essential: false,
            can_go_back: false,
            can_go_forward: false,
            nav_fwd_depth: 0,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            created_at: now,
            last_active_at: now,
            is_audio_playing: false,
            is_muted: false,
        }
    }

    pub fn new_tab_url() -> &'static str {
        "neura://newtab"
    }

    pub fn display_title(&self) -> &str {
        if self.title.is_empty() || self.title == "New Tab" && self.url != "neura://newtab" {
            &self.url
        } else {
            &self.title
        }
    }

    pub fn display_url(&self) -> &str {
        match self.url.as_str() {
            "neura://newtab" => "New Tab",
            "neura://settings" => "Settings",
            "neura://apps" => "Apps",
            url => url,
        }
    }

    pub fn is_neura_page(&self) -> bool {
        self.url.starts_with("neura://")
    }

    pub fn sync_nav_flags(&mut self) {
        self.can_go_back = !self.back_stack.is_empty();
        self.can_go_forward = !self.forward_stack.is_empty();
        self.nav_fwd_depth = self.forward_stack.len() as i32;
    }
}
