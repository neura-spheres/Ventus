use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTool {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub requires_confirmation: bool,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

pub fn all_tools() -> Vec<BrowserTool> {
    vec![
        BrowserTool {
            id: "get_current_page_url",
            name: "Get Current URL",
            description: "Get the URL of the currently active tab",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "get_current_page_title",
            name: "Get Page Title",
            description: "Get the title of the current page",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "get_current_page_text",
            name: "Get Page Text",
            description: "Extract readable text content from the current page",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "summarize_current_page",
            name: "Summarize Page",
            description: "Generate a summary of the current page",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "open_url",
            name: "Open URL",
            description: "Navigate the current tab to a URL",
            requires_confirmation: true,
            risk_level: RiskLevel::Medium,
        },
        BrowserTool {
            id: "create_new_tab",
            name: "Create New Tab",
            description: "Open a new browser tab",
            requires_confirmation: true,
            risk_level: RiskLevel::Medium,
        },
        BrowserTool {
            id: "close_current_tab",
            name: "Close Tab",
            description: "Close the current tab",
            requires_confirmation: true,
            risk_level: RiskLevel::High,
        },
        BrowserTool {
            id: "switch_tab",
            name: "Switch Tab",
            description: "Switch to a different tab",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "list_tabs",
            name: "List Tabs",
            description: "List all open tabs in the current workspace",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "list_workspaces",
            name: "List Workspaces",
            description: "List all workspaces",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "switch_workspace",
            name: "Switch Workspace",
            description: "Switch to a different workspace",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "search_history",
            name: "Search History",
            description: "Search browser history",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "list_bookmarks",
            name: "List Bookmarks",
            description: "List saved bookmarks",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "add_bookmark",
            name: "Add Bookmark",
            description: "Bookmark the current page",
            requires_confirmation: true,
            risk_level: RiskLevel::Low,
        },
        BrowserTool {
            id: "search_web",
            name: "Search Web",
            description: "Perform a web search",
            requires_confirmation: true,
            risk_level: RiskLevel::Medium,
        },
        BrowserTool {
            id: "get_browser_settings",
            name: "Get Settings",
            description: "Read current browser settings",
            requires_confirmation: false,
            risk_level: RiskLevel::Low,
        },
    ]
}
