use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub accent_color: String,
    pub created_at: i64,
    pub position: i32,
    #[serde(default)]
    pub is_incognito: bool,
}

impl Workspace {
    pub fn new(name: impl Into<String>, is_incognito: bool, icon: Option<String>) -> Self {
        let name = name.into();
        let icon = icon.unwrap_or_else(|| Self::default_icon_for_name(&name));
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            icon,
            accent_color: if is_incognito {
                "#6b7280".to_string()
            } else {
                "#8b5cf6".to_string()
            },
            created_at: chrono::Utc::now().timestamp_millis(),
            position: 0,
            is_incognito,
        }
    }

    pub fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Main".to_string(),
            icon: "🌐".to_string(),
            accent_color: "#8b5cf6".to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
            position: 0,
            is_incognito: false,
        }
    }

    fn default_icon_for_name(name: &str) -> String {
        match name.to_lowercase().as_str() {
            s if s.contains("work") => "💼",
            s if s.contains("personal") => "🏠",
            s if s.contains("research") => "🔬",
            s if s.contains("social") => "💬",
            s if s.contains("shop") => "🛍️",
            s if s.contains("news") => "📰",
            s if s.contains("dev") || s.contains("code") => "💻",
            s if s.contains("music") => "🎵",
            s if s.contains("video") => "🎬",
            _ => "📁",
        }
        .to_string()
    }

    pub fn accent_colors() -> &'static [(&'static str, &'static str)] {
        &[
            ("violet", "#8b5cf6"),
            ("blue", "#3b82f6"),
            ("green", "#22c55e"),
            ("orange", "#f97316"),
            ("red", "#ef4444"),
            ("pink", "#ec4899"),
            ("cyan", "#06b6d4"),
            ("yellow", "#eab308"),
        ]
    }
}
