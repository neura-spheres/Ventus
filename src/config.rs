use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarMode {
    Expanded,
    Compact,
    AutoHide,
}

impl Default for SidebarMode {
    fn default() -> Self {
        SidebarMode::AutoHide
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupBehavior {
    NewTab,
    LastSession,
    /// Serializes as "home_page" to match the HTML <option value="home_page"> element.
    /// Accepts legacy "specific_pages" values from older saved settings.
    #[serde(alias = "specific_pages")]
    HomePage,
}

impl Default for StartupBehavior {
    fn default() -> Self {
        StartupBehavior::NewTab
    }
}

fn default_zoom_level() -> f64 {
    1.0
}

fn default_font_family() -> String {
    "system".to_string()
}

pub const TOOLBAR_BUTTON_LIMIT: usize = 4;
pub const TOOLBAR_BUTTON_IDS: [&str; 5] = ["ai", "downloads", "history", "bookmarks", "settings"];

pub fn default_toolbar_buttons() -> Vec<String> {
    vec!["ai".to_string()]
}

pub fn clean_toolbar_buttons(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let id = value.trim();
        if !TOOLBAR_BUTTON_IDS.contains(&id) {
            continue;
        }
        if out.iter().any(|item| item == id) {
            continue;
        }
        out.push(id.to_string());
        if out.len() >= TOOLBAR_BUTTON_LIMIT {
            break;
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub accent_color: String,
    pub sidebar_mode: SidebarMode,
    pub compact_toolbar: bool,
    pub show_home_button: bool,
    pub show_bookmarks_button: bool,
    pub show_bookmarks_bar: bool,
    pub corner_radius: String,
    pub font_size: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    pub new_tab_background: String,
    pub new_tab_bg_color: String,
    #[serde(default = "default_zoom_level")]
    pub zoom_level: f64,
    #[serde(default = "default_toolbar_buttons")]
    pub toolbar_buttons: Vec<String>,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            accent_color: "#8b5cf6".to_string(),
            sidebar_mode: SidebarMode::default(),
            compact_toolbar: false,
            show_home_button: true,
            show_bookmarks_button: true,
            show_bookmarks_bar: false,
            corner_radius: "soft".to_string(),
            font_size: "medium".to_string(),
            font_family: default_font_family(),
            new_tab_background: "default".to_string(),
            new_tab_bg_color: "#141414".to_string(),
            zoom_level: 1.0,
            toolbar_buttons: default_toolbar_buttons(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSettings {
    pub default_engine: String,
    pub site_shortcuts_enabled: bool,
    #[serde(default = "default_true")]
    pub suggestions_enabled: bool,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            default_engine: "google".to_string(),
            site_shortcuts_enabled: true,
            suggestions_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TabSettings {
    pub new_tab_position: String,
    pub close_tab_behavior: String,
    pub confirm_close_many: bool,
    pub enable_pinned_tabs: bool,
    pub restore_tabs_on_startup: bool,
}

impl Default for TabSettings {
    fn default() -> Self {
        Self {
            new_tab_position: "after_current".to_string(),
            close_tab_behavior: "focus_last_active".to_string(),
            confirm_close_many: true,
            enable_pinned_tabs: true,
            restore_tabs_on_startup: true,
        }
    }
}

fn default_feed_layout() -> String {
    "cards".to_string()
}

fn default_newtab_theme() -> String {
    "focus".to_string()
}

fn default_clock_style() -> String {
    "serif".to_string()
}

fn default_wallpaper_source() -> String {
    "nature".to_string()
}

fn default_wallpaper_color() -> String {
    "#141414".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NewTabSettings {
    #[serde(default = "default_true")]
    pub show_search: bool,
    #[serde(default = "default_true")]
    pub show_quick_links: bool,
    #[serde(default = "default_true")]
    pub show_background: bool,
    #[serde(default = "default_feed_layout")]
    pub feed_layout: String,
    /// Layout preset: "minimal" | "focus" | "horizon" | "informative"
    #[serde(default = "default_newtab_theme")]
    pub theme: String,
    #[serde(default = "default_clock_style")]
    pub clock_style: String,
    /// Wallpaper source: "daily" | "nature" | "url" | "upload" | "color" | "none"
    #[serde(default = "default_wallpaper_source")]
    pub wallpaper_source: String,
    /// Custom wallpaper URL (used when wallpaper_source == "url")
    #[serde(default)]
    pub wallpaper_url: String,
    /// Solid color hex for wallpaper_source == "color"
    #[serde(default = "default_wallpaper_color")]
    pub wallpaper_color: String,
    /// Base64 data URL for uploaded wallpaper (wallpaper_source == "upload")
    #[serde(default)]
    pub wallpaper_data: String,
    /// Custom font color hex override (empty = use theme default)
    #[serde(default)]
    pub font_color: String,
}

impl Default for NewTabSettings {
    fn default() -> Self {
        Self {
            show_search: true,
            show_quick_links: true,
            show_background: true,
            feed_layout: default_feed_layout(),
            theme: default_newtab_theme(),
            clock_style: default_clock_style(),
            wallpaper_source: default_wallpaper_source(),
            wallpaper_url: String::new(),
            wallpaper_color: default_wallpaper_color(),
            wallpaper_data: String::new(),
            font_color: String::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

pub const CLOUDFLARE_DOH: &str = "https://cloudflare-dns.com/dns-query";
pub const CLOUDFLARE_MALWARE_DOH: &str = "https://security.cloudflare-dns.com/dns-query";
pub const CLOUDFLARE_FAMILY_DOH: &str = "https://family.cloudflare-dns.com/dns-query";
pub const GOOGLE_DOH: &str = "https://dns.google/dns-query";
pub const OPENDNS_DOH: &str = "https://dns.opendns.com/dns-query";
pub const OPENDNS_FAMILY_DOH: &str = "https://familyshield.opendns.com/dns-query";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecureDnsProvider {
    Cloudflare,
    CloudflareMalware,
    CloudflareFamily,
    Google,
    #[serde(rename = "opendns", alias = "open_dns")]
    OpenDns,
    #[serde(rename = "opendns_family", alias = "open_dns_family")]
    OpenDnsFamily,
    Custom,
}

impl Default for SecureDnsProvider {
    fn default() -> Self {
        Self::Cloudflare
    }
}

impl SecureDnsProvider {
    pub fn from_id(id: &str) -> Self {
        match id {
            "cloudflare_malware" => Self::CloudflareMalware,
            "cloudflare_family" => Self::CloudflareFamily,
            "google" => Self::Google,
            "opendns" | "open_dns" => Self::OpenDns,
            "opendns_family" | "open_dns_family" => Self::OpenDnsFamily,
            "custom" => Self::Custom,
            _ => Self::Cloudflare,
        }
    }

    pub fn endpoint(&self, custom: &str) -> Option<String> {
        match self {
            Self::Cloudflare => Some(CLOUDFLARE_DOH.to_string()),
            Self::CloudflareMalware => Some(CLOUDFLARE_MALWARE_DOH.to_string()),
            Self::CloudflareFamily => Some(CLOUDFLARE_FAMILY_DOH.to_string()),
            Self::Google => Some(GOOGLE_DOH.to_string()),
            Self::OpenDns => Some(OPENDNS_DOH.to_string()),
            Self::OpenDnsFamily => Some(OPENDNS_FAMILY_DOH.to_string()),
            Self::Custom => clean_doh_url(custom),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecureDnsMode {
    Automatic,
    Secure,
}

impl Default for SecureDnsMode {
    fn default() -> Self {
        Self::Secure
    }
}

impl SecureDnsMode {
    pub fn from_id(id: &str) -> Self {
        match id {
            "automatic" => Self::Automatic,
            _ => Self::Secure,
        }
    }

    pub fn as_arg(&self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Secure => "secure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SitePermissions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub microphone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensors: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoplay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_fonts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_management: Option<String>,
}

impl Default for SitePermissions {
    fn default() -> Self {
        Self {
            camera: None,
            microphone: None,
            geolocation: None,
            notifications: None,
            clipboard: None,
            sensors: None,
            downloads: None,
            file_system: None,
            autoplay: None,
            local_fonts: None,
            midi: None,
            window_management: None,
        }
    }
}

impl SitePermissions {
    pub fn get_explicit(&self, key: &str) -> Option<&str> {
        match key {
            "camera" => self.camera.as_deref(),
            "microphone" => self.microphone.as_deref(),
            "geolocation" => self.geolocation.as_deref(),
            "notifications" => self.notifications.as_deref(),
            "clipboard" => self.clipboard.as_deref(),
            "sensors" => self.sensors.as_deref(),
            "downloads" => self.downloads.as_deref(),
            "file_system" => self.file_system.as_deref(),
            "autoplay" => self.autoplay.as_deref(),
            "local_fonts" => self.local_fonts.as_deref(),
            "midi" => self.midi.as_deref(),
            "window_management" => self.window_management.as_deref(),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> bool {
        if !valid_site_permission_value(value) {
            return false;
        }
        let value = Some(value.to_string());
        match key {
            "camera" => self.camera = value,
            "microphone" => self.microphone = value,
            "geolocation" => self.geolocation = value,
            "notifications" => self.notifications = value,
            "clipboard" => self.clipboard = value,
            "sensors" => self.sensors = value,
            "downloads" => self.downloads = value,
            "file_system" => self.file_system = value,
            "autoplay" => self.autoplay = value,
            "local_fonts" => self.local_fonts = value,
            "midi" => self.midi = value,
            "window_management" => self.window_management = value,
            _ => return false,
        }
        true
    }
}

pub type SitePermissionMap = BTreeMap<String, SitePermissions>;

pub const SITE_PERMISSION_KEYS: [&str; 12] = [
    "camera",
    "microphone",
    "geolocation",
    "notifications",
    "clipboard",
    "sensors",
    "downloads",
    "file_system",
    "autoplay",
    "local_fonts",
    "midi",
    "window_management",
];

pub fn valid_site_permission_key(key: &str) -> bool {
    SITE_PERMISSION_KEYS.contains(&key)
}

pub fn valid_site_permission_value(value: &str) -> bool {
    matches!(value, "ask" | "allow" | "block")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacySettings {
    pub disable_history: bool,
    pub do_not_track: bool,
    #[serde(default = "default_true")]
    pub https_only: bool,
    #[serde(default = "default_true")]
    pub block_third_party_cookies: bool,
    #[serde(default = "default_true")]
    pub storage_partitioning: bool,
    #[serde(default = "default_true")]
    pub fingerprint_protection: bool,
    #[serde(default = "default_true")]
    pub strict_permissions: bool,
    #[serde(default = "default_true")]
    pub ad_blocker_enabled: bool,
    #[serde(default)]
    pub ad_blocker_exceptions: Vec<String>,
    #[serde(default)]
    pub site_permissions: SitePermissionMap,
    #[serde(default)]
    pub default_permissions: SitePermissions,
    pub secure_dns_enabled: bool,
    pub secure_dns_provider: SecureDnsProvider,
    pub secure_dns_mode: SecureDnsMode,
    pub secure_dns_template: String,
    #[serde(default = "default_true")]
    pub auto_crash_report: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            disable_history: false,
            do_not_track: false,
            https_only: true,
            block_third_party_cookies: true,
            storage_partitioning: true,
            fingerprint_protection: true,
            strict_permissions: true,
            ad_blocker_enabled: true,
            ad_blocker_exceptions: Vec::new(),
            site_permissions: BTreeMap::new(),
            default_permissions: SitePermissions::default(),
            secure_dns_enabled: false,
            secure_dns_provider: SecureDnsProvider::default(),
            secure_dns_mode: SecureDnsMode::default(),
            secure_dns_template: CLOUDFLARE_DOH.to_string(),
            auto_crash_report: true,
        }
    }
}

impl PrivacySettings {
    pub fn secure_dns_endpoint(&self) -> Option<String> {
        if !self.secure_dns_enabled {
            return None;
        }
        self.secure_dns_provider.endpoint(&self.secure_dns_template)
    }
}

pub fn clean_doh_url(input: &str) -> Option<String> {
    let url = input.trim();
    if url.is_empty() || !url.starts_with("https://") {
        return None;
    }
    if url.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    Some(url.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadSettings {
    pub default_folder: String,
    pub ask_where_to_save: bool,
    pub auto_open: bool,
}

impl Default for DownloadSettings {
    fn default() -> Self {
        Self {
            default_folder: String::new(),
            ask_where_to_save: true,
            auto_open: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    pub enabled: bool,
    pub default_provider: String,
    pub default_model: String,
    pub openai_base_url: String,
    pub anthropic_base_url: String,
    pub gemini_base_url: String,
    pub openai_use_responses_api: bool,
    pub reasoning_effort: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream_responses: bool,
    pub save_chat_history: bool,
    pub summarization_style: String,
    pub system_prompt: String,
    pub include_page_context: bool,
    pub max_page_text_length: usize,
    pub warn_before_sending_page: bool,
    pub allow_browser_actions: bool,
    pub require_action_confirmation: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            default_provider: "openai".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            openai_base_url: "https://api.openai.com/v1".to_string(),
            anthropic_base_url: "https://api.anthropic.com/v1".to_string(),
            gemini_base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            openai_use_responses_api: false,
            reasoning_effort: "default".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            stream_responses: true,
            save_chat_history: true,
            summarization_style: "bullet_points".to_string(),
            system_prompt: "You are Neura, an intelligent browser assistant. You help users understand and navigate web content. Be concise and helpful.".to_string(),
            include_page_context: true,
            max_page_text_length: 8000,
            warn_before_sending_page: true,
            allow_browser_actions: false,
            require_action_confirmation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub enabled: bool,
    pub base_url: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub homepage: String,
    pub startup_behavior: StartupBehavior,
    pub appearance: AppearanceSettings,
    pub search: SearchSettings,
    pub tabs: TabSettings,
    #[serde(default)]
    pub new_tab: NewTabSettings,
    pub privacy: PrivacySettings,
    pub downloads: DownloadSettings,
    pub ai: AiSettings,
    pub window_width: u32,
    pub window_height: u32,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub sidebar_width: u32,
    #[serde(default)]
    pub region: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            homepage: "neura://newtab".to_string(),
            startup_behavior: StartupBehavior::default(),
            appearance: AppearanceSettings::default(),
            search: SearchSettings::default(),
            tabs: TabSettings::default(),
            new_tab: NewTabSettings::default(),
            privacy: PrivacySettings::default(),
            downloads: DownloadSettings::default(),
            ai: AiSettings::default(),
            window_width: 1400,
            window_height: 900,
            window_x: None,
            window_y: None,
            sidebar_width: 240,
            region: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn old_settings_keep_defaults() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"homepage":"https://google.com"}"#).expect("settings parse");
        assert_eq!(settings.homepage, "https://google.com");
        assert_eq!(settings.downloads.default_folder, "");
        assert!(settings.downloads.ask_where_to_save);
        assert!(settings.search.suggestions_enabled);
        assert!(settings.privacy.https_only);
        assert!(settings.privacy.block_third_party_cookies);
        assert!(settings.privacy.storage_partitioning);
        assert!(settings.privacy.fingerprint_protection);
        assert!(settings.privacy.strict_permissions);
        assert_eq!(settings.appearance.toolbar_buttons, vec!["ai".to_string()]);
        assert!(!settings.privacy.secure_dns_enabled);
        assert_eq!(
            settings.privacy.secure_dns_provider,
            super::SecureDnsProvider::Cloudflare
        );
        assert_eq!(
            settings.privacy.secure_dns_mode,
            super::SecureDnsMode::Secure
        );
    }

    #[test]
    fn built_in_providers_use_expected_doh_templates() {
        assert_eq!(
            super::SecureDnsProvider::Cloudflare.endpoint(""),
            Some(super::CLOUDFLARE_DOH.to_string())
        );
        assert_eq!(
            super::SecureDnsProvider::CloudflareMalware.endpoint(""),
            Some(super::CLOUDFLARE_MALWARE_DOH.to_string())
        );
        assert_eq!(
            super::SecureDnsProvider::CloudflareFamily.endpoint(""),
            Some(super::CLOUDFLARE_FAMILY_DOH.to_string())
        );
        assert_eq!(
            super::SecureDnsProvider::Google.endpoint(""),
            Some(super::GOOGLE_DOH.to_string())
        );
        assert_eq!(
            super::SecureDnsProvider::OpenDns.endpoint(""),
            Some(super::OPENDNS_DOH.to_string())
        );
        assert_eq!(
            super::SecureDnsProvider::OpenDnsFamily.endpoint(""),
            Some(super::OPENDNS_FAMILY_DOH.to_string())
        );
    }

    #[test]
    fn opendns_provider_ids_match_settings_ui() {
        assert_eq!(
            serde_json::to_string(&super::SecureDnsProvider::OpenDns).unwrap(),
            r#""opendns""#
        );
        assert_eq!(
            serde_json::to_string(&super::SecureDnsProvider::OpenDnsFamily).unwrap(),
            r#""opendns_family""#
        );
        assert_eq!(
            serde_json::from_str::<super::SecureDnsProvider>(r#""open_dns""#).unwrap(),
            super::SecureDnsProvider::OpenDns
        );
        assert_eq!(
            serde_json::from_str::<super::SecureDnsProvider>(r#""open_dns_family""#).unwrap(),
            super::SecureDnsProvider::OpenDnsFamily
        );
    }

    #[test]
    fn provider_ids_accept_all_settings_ui_values() {
        let cases = [
            ("cloudflare", super::SecureDnsProvider::Cloudflare),
            (
                "cloudflare_malware",
                super::SecureDnsProvider::CloudflareMalware,
            ),
            (
                "cloudflare_family",
                super::SecureDnsProvider::CloudflareFamily,
            ),
            ("google", super::SecureDnsProvider::Google),
            ("opendns", super::SecureDnsProvider::OpenDns),
            ("open_dns", super::SecureDnsProvider::OpenDns),
            ("opendns_family", super::SecureDnsProvider::OpenDnsFamily),
            ("open_dns_family", super::SecureDnsProvider::OpenDnsFamily),
            ("custom", super::SecureDnsProvider::Custom),
            ("unknown", super::SecureDnsProvider::Cloudflare),
        ];

        for (id, provider) in cases {
            assert_eq!(super::SecureDnsProvider::from_id(id), provider);
        }
    }

    #[test]
    fn secure_dns_endpoint_respects_toggle_and_custom_validation() {
        let mut privacy = super::PrivacySettings::default();
        assert_eq!(privacy.secure_dns_endpoint(), None);

        privacy.secure_dns_enabled = true;
        assert_eq!(
            privacy.secure_dns_endpoint(),
            Some(super::CLOUDFLARE_DOH.to_string())
        );

        privacy.secure_dns_provider = super::SecureDnsProvider::Custom;
        privacy.secure_dns_template = " https://dns.example/dns-query ".to_string();
        assert_eq!(
            privacy.secure_dns_endpoint(),
            Some("https://dns.example/dns-query".to_string())
        );

        for invalid in [
            "",
            "http://dns.example/dns-query",
            "https://dns.example/dns query",
            "https://dns.example/\tdns-query",
        ] {
            privacy.secure_dns_template = invalid.to_string();
            assert_eq!(privacy.secure_dns_endpoint(), None);
        }
    }

    #[test]
    fn toolbar_buttons_are_sanitized() {
        let values = vec![
            "ai".to_string(),
            "history".to_string(),
            "bogus".to_string(),
            "ai".to_string(),
            "downloads".to_string(),
            "bookmarks".to_string(),
            "settings".to_string(),
        ];
        assert_eq!(
            super::clean_toolbar_buttons(&values),
            vec![
                "ai".to_string(),
                "history".to_string(),
                "downloads".to_string(),
                "bookmarks".to_string()
            ]
        );
    }
}
