use serde::{Deserialize, Serialize};

use crate::cloud::{AuthSession, UserProfile};

/// One prior turn of the spotlight Quick-AI conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightTurn {
    pub role: String,
    pub content: String,
}

/// One entry in the bookmark-bar order. `folder` distinguishes a folder id from a
/// bookmark id (both are UUIDs, so the flag is what tells them apart).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarOrderRef {
    pub id: String,
    #[serde(default)]
    pub folder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum ChromeCommand {
    Navigate {
        url: String,
    },
    NavigateFromOverlay {
        url: String,
    },
    ContinueHttp {
        url: String,
    },
    ProceedBlockedSite {
        url: String,
    },
    Back,
    Forward,
    Reload,
    Stop,
    OmniboxPaste,
    WriteClipboard {
        text: String,
    },
    NewTab,
    CloseTab {
        id: String,
    },
    SwitchTab {
        id: String,
    },
    SwitchTabOffset {
        delta: i32,
    },
    PinTab {
        id: String,
    },
    UnpinTab {
        id: String,
    },
    /// Reorder a tab in the sidebar list. `before` is the id of the tab to insert ahead of,
    /// or None to move it to the end of its group.
    MoveTab {
        id: String,
        #[serde(default)]
        before: Option<String>,
    },
    NewWorkspace {
        name: String,
        #[serde(default)]
        is_incognito: bool,
        #[serde(default)]
        icon: Option<String>,
        #[serde(default)]
        accent_color: Option<String>,
    },
    RenameWorkspace {
        id: String,
        name: String,
        #[serde(default)]
        icon: Option<String>,
        #[serde(default)]
        accent_color: Option<String>,
    },
    DeleteWorkspace {
        id: String,
    },
    SwitchWorkspace {
        id: String,
    },
    ToggleAiSidebar,
    ToggleAppSidebar,
    AppPanelSelect {
        url: String,
    },
    AppPanelReload,
    AppPanelOpenInTab {
        url: String,
    },
    AppPanelClose,
    PickAiAttachments,
    RemoveAiAttachment {
        id: String,
    },
    AiMessage {
        text: String,
    },
    AiProviderChange {
        provider: String,
    },
    AiModelChange {
        model: String,
        #[serde(default)]
        provider: Option<String>,
    },
    AiQuickAction {
        action: String,
    },
    AiStop,
    AiClearChat,
    GetAiSessions,
    LoadAiSession {
        id: String,
    },
    DeleteAiSession {
        id: String,
    },
    SaveSpotlightChat {
        session_id: String,
        title: String,
        query: String,
        answer: String,
    },
    BookmarkAdd,
    /// Save a bookmark from a dropped link (drag-to-bookmark). `before_id` positions the new
    /// bar entry ahead of an existing one (None = append). `folder_id` files it inside a folder
    /// instead of placing it on the bar.
    BookmarkAddUrl {
        url: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        before_id: Option<String>,
        #[serde(default)]
        folder_id: Option<String>,
    },
    /// Reorder a bookmark. `before` is the id of the bookmark to insert ahead of, or None for end.
    MoveBookmark {
        id: String,
        #[serde(default)]
        before: Option<String>,
    },
    MoveBookmarkFolder {
        id: String,
        #[serde(default)]
        before: Option<String>,
    },
    /// Persist the full left-to-right order of the bookmark bar (folders + unfiled
    /// bookmarks intermixed). Source of truth for the bar layout only; the per-table
    /// `position` columns used by the sidebar list and folder modal are untouched.
    SetBarOrder {
        order: Vec<BarOrderRef>,
    },
    BookmarkRemove {
        url: String,
    },
    BookmarkRemoveById {
        id: String,
    },
    BookmarkRename {
        id: String,
        title: String,
    },
    BookmarkSetIconOnly {
        id: String,
        icon_only: bool,
    },
    BookmarkCreateFolder {
        bookmark_id_a: String,
        bookmark_id_b: String,
    },
    BookmarkMoveToFolder {
        bookmark_id: String,
        folder_id: String,
    },
    BookmarkRemoveFromFolder {
        bookmark_id: String,
    },
    BookmarkFolderRename {
        folder_id: String,
        name: String,
    },
    BookmarkFolderDelete {
        folder_id: String,
    },
    BookmarkNewFolder,
    OpenSettings,
    CloseSettings,
    CaptureBlurBackdrop,
    BrowseDownloadFolder,
    SaveSettings {
        key: String,
        value: serde_json::Value,
    },
    SetSitePermission {
        origin: String,
        permission: String,
        value: String,
    },
    SetDefaultPermission {
        permission: String,
        value: String,
    },
    ReopenTab,
    WindowDragStart,
    WindowClose,
    WindowMinimize,
    WindowMaximize,
    ThemeToggle,
    SidebarToggle,
    SearchTabs {
        q: String,
    },
    FocusAddressBar,
    OpenTabSearch,
    OpenFindBar,
    FindInPage {
        query: String,
        forward: bool,
        #[serde(default)]
        tab_id: Option<String>,
    },
    GetPageText,
    TranslatePage {
        lang: String,
    },
    SplitView {
        mode: String,
    },
    CloseSplit,
    OpenDevtools,
    RestartForWebSecurity,
    DevRestart,
    DevOpenDataFolder,
    DevCopyDiagnostics,
    DevResetOnboarding,
    DevTestReport,
    DevTestCrash,
    HistoryClear,
    ExportSettings,
    ImportSettings {
        path: String,
    },
    GetHistory {
        q: String,
    },
    GetHistoryPage {
        q: String,
        offset: i64,
    },
    OmniboxSuggest {
        q: String,
    },
    FetchSearchSuggestions {
        q: String,
        id: u64,
    },
    OmniboxPick {
        q: String,
        url: String,
        #[serde(default)]
        shown: Vec<String>,
    },
    /// Pin / unpin / block a recommendation site. `pinned`/`blocked` are tri-state:
    /// None leaves that flag untouched. `q` is the current query so the refreshed
    /// suggestions match what the user is looking at.
    OmniboxSetPref {
        url: String,
        #[serde(default)]
        pinned: Option<bool>,
        #[serde(default)]
        blocked: Option<bool>,
        #[serde(default)]
        q: String,
    },
    RefreshTrends,
    DeleteHistoryEntry {
        id: i64,
    },
    DeleteHistoryDay {
        start: i64,
        end: i64,
    },
    PwdSaveConfirm,
    PwdSaveDismiss,
    PwdList,
    PwdReveal {
        id: String,
    },
    PwdDelete {
        id: String,
    },
    OpenFile {
        path: String,
    },
    RevealFile {
        path: String,
    },
    SidebarPeek {
        visible: bool,
        pinned: bool,
    },
    SidebarAutoClose,
    /// Cursor dwelled at the left window edge during a drag — open the auto-hide sidebar so
    /// the user can drop onto it.
    DragEdgePeek,
    /// During the auto-hide sidebar's slide animation, JS streams the sidebar's
    /// live right-edge position (CSS px) so Rust can size the chrome clip column
    /// and content cut to track it exactly — preventing the dark window background
    /// from showing through the transparent chrome behind the sliding sidebar.
    /// `w < 0` clears the override (animation finished, return to normal clip).
    SidebarClipWidth {
        w: f64,
    },
    SuggestionOverlay {
        owner: String,
        visible: bool,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    CheckForUpdate,
    LoadNeuraFeed,
    InstallUpdate,
    ZoomSet {
        level: f64,
    },
    ZoomReset,
    ZoomDelta {
        delta: f64,
    },
    ZoomGlobal {
        level: f64,
    },
    ToggleFullscreen,
    ContentFullscreenChange {
        active: bool,
    },
    PeekSidebar,
    ClearDownloads,
    DeleteDownload {
        id: String,
    },
    PauseDownload {
        id: String,
    },
    ResumeDownload {
        id: String,
    },
    CancelDownload {
        id: String,
    },
    OpenInNewTab {
        url: String,
    },
    ContextMenuSaveImage {
        url: String,
    },
    CopyImage {
        url: String,
    },
    OpenInNewWindow {
        url: String,
    },
    DismissUpdate {
        version: String,
    },
    BeginSpotlight,
    EndSpotlight,
    OpenHistoryPanel,
    OpenDownloadsPanel,
    SpotlightAiQuery {
        text: String,
        /// Prior turns of the spotlight conversation (oldest first) so follow-ups have
        /// context. Each entry is `{role: "user"|"assistant", content}`.
        #[serde(default)]
        history: Vec<SpotlightTurn>,
    },
    /// Content WebView reports audio/video playback state change for a tab.
    TabAudioState {
        tab_id: String,
        playing: bool,
        #[serde(default)]
        active: bool,
    },
    /// Chrome UI requests mute/unmute toggle for a specific tab.
    MuteTab {
        tab_id: String,
    },
    /// JS resize-handle mousedown → Rust initiates native Win32 resize.
    /// `edge` is one of: left, right, top, bottom, topleft, topright, bottomleft, bottomright.
    BeginResize {
        edge: String,
    },
    /// Toggle the ad blocker exception for the currently active tab's site.
    /// Rust adds/removes the host from exceptions and reloads the tab.
    AdBlockToggleSite,
    XLoginCompatibilityToggle,
    /// Reported by the content WebView init script: how many DOM elements were hidden/removed.
    AdBlockStats {
        killed: u32,
    },
    FetchCurrencyRates,
    OpenIncognito,
    /// A pointer/mouse press landed inside the page content (not the chrome overlay).
    /// Used to dismiss chrome popovers that are clipped to their own rect (e.g. the
    /// download panel) when the user clicks out into the live web page.
    ContentPointerDown,
    AuthSignUp {
        email: String,
        password: String,
    },
    AuthSignIn {
        email: String,
        password: String,
    },
    AuthSignInGoogle,
    AuthSignOut,
    AccountUpdateProfile {
        username: String,
        full_name: String,
        birthdate: String,
        bio: String,
    },
    AccountSetPhoto {
        data_uri: String,
    },
    AccountChangePassword {
        current: String,
        new_password: String,
    },
    GetAccountState,
    PermissionDecision {
        id: String,
        origin: String,
        permission: String,
        decision: String,
    },
    SendReport {
        message: String,
    },
}

impl ChromeCommand {
    pub fn report_name(&self) -> Option<&'static str> {
        match self {
            ChromeCommand::GetHistory { .. }
            | ChromeCommand::OmniboxSuggest { .. }
            | ChromeCommand::FetchSearchSuggestions { .. }
            | ChromeCommand::SearchTabs { .. }
            | ChromeCommand::FindInPage { .. }
            | ChromeCommand::SidebarPeek { .. }
            | ChromeCommand::SidebarAutoClose
            | ChromeCommand::SidebarClipWidth { .. }
            | ChromeCommand::SuggestionOverlay { .. }
            | ChromeCommand::ContentPointerDown => None,
            ChromeCommand::Navigate { .. } => Some("navigate"),
            ChromeCommand::NavigateFromOverlay { .. } => Some("navigate_overlay"),
            ChromeCommand::ContinueHttp { .. } => Some("continue_http"),
            ChromeCommand::ProceedBlockedSite { .. } => Some("proceed_blocked_site"),
            ChromeCommand::Back => Some("back"),
            ChromeCommand::Forward => Some("forward"),
            ChromeCommand::Reload => Some("reload"),
            ChromeCommand::Stop => Some("stop"),
            ChromeCommand::OmniboxPaste => Some("omnibox_paste"),
            ChromeCommand::WriteClipboard { .. } => Some("write_clipboard"),
            ChromeCommand::NewTab => Some("new_tab"),
            ChromeCommand::CloseTab { .. } => Some("close_tab"),
            ChromeCommand::SwitchTab { .. } => Some("switch_tab"),
            ChromeCommand::SwitchTabOffset { .. } => Some("switch_tab_offset"),
            ChromeCommand::PinTab { .. } => Some("pin_tab"),
            ChromeCommand::UnpinTab { .. } => Some("unpin_tab"),
            ChromeCommand::MoveTab { .. } => Some("move_tab"),
            ChromeCommand::NewWorkspace { .. } => Some("new_workspace"),
            ChromeCommand::RenameWorkspace { .. } => Some("rename_workspace"),
            ChromeCommand::DeleteWorkspace { .. } => Some("delete_workspace"),
            ChromeCommand::SwitchWorkspace { .. } => Some("switch_workspace"),
            ChromeCommand::ToggleAiSidebar => Some("toggle_ai_sidebar"),
            ChromeCommand::ToggleAppSidebar => Some("toggle_app_sidebar"),
            ChromeCommand::AppPanelSelect { .. } => Some("app_panel_select"),
            ChromeCommand::AppPanelReload => Some("app_panel_reload"),
            ChromeCommand::AppPanelOpenInTab { .. } => Some("app_panel_open_in_tab"),
            ChromeCommand::AppPanelClose => Some("app_panel_close"),
            ChromeCommand::PickAiAttachments => Some("pick_ai_attachments"),
            ChromeCommand::RemoveAiAttachment { .. } => Some("remove_ai_attachment"),
            ChromeCommand::AiMessage { .. } => Some("ai_message"),
            ChromeCommand::AiProviderChange { .. } => Some("ai_provider_change"),
            ChromeCommand::AiModelChange { .. } => Some("ai_model_change"),
            ChromeCommand::AiQuickAction { .. } => Some("ai_quick_action"),
            ChromeCommand::AiStop => Some("ai_stop"),
            ChromeCommand::AiClearChat => Some("ai_clear_chat"),
            ChromeCommand::GetAiSessions => Some("get_ai_sessions"),
            ChromeCommand::LoadAiSession { .. } => Some("load_ai_session"),
            ChromeCommand::DeleteAiSession { .. } => Some("delete_ai_session"),
            ChromeCommand::SaveSpotlightChat { .. } => Some("save_spotlight_chat"),
            ChromeCommand::BookmarkAdd => Some("bookmark_add"),
            ChromeCommand::BookmarkAddUrl { .. } => Some("bookmark_add_url"),
            ChromeCommand::MoveBookmark { .. } => Some("move_bookmark"),
            ChromeCommand::MoveBookmarkFolder { .. } => Some("move_bookmark_folder"),
            ChromeCommand::SetBarOrder { .. } => Some("set_bar_order"),
            ChromeCommand::BookmarkRemove { .. } => Some("bookmark_remove"),
            ChromeCommand::BookmarkRemoveById { .. } => Some("bookmark_remove_by_id"),
            ChromeCommand::BookmarkRename { .. } => Some("bookmark_rename"),
            ChromeCommand::BookmarkSetIconOnly { .. } => Some("bookmark_icon_only"),
            ChromeCommand::BookmarkCreateFolder { .. } => Some("bookmark_create_folder"),
            ChromeCommand::BookmarkMoveToFolder { .. } => Some("bookmark_move_to_folder"),
            ChromeCommand::BookmarkRemoveFromFolder { .. } => Some("bookmark_remove_from_folder"),
            ChromeCommand::BookmarkFolderRename { .. } => Some("bookmark_folder_rename"),
            ChromeCommand::BookmarkFolderDelete { .. } => Some("bookmark_folder_delete"),
            ChromeCommand::BookmarkNewFolder => Some("bookmark_new_folder"),
            ChromeCommand::OpenSettings => Some("open_settings"),
            ChromeCommand::CloseSettings => Some("close_settings"),
            ChromeCommand::CaptureBlurBackdrop => Some("capture_blur_backdrop"),
            ChromeCommand::BrowseDownloadFolder => Some("browse_download_folder"),
            ChromeCommand::SaveSettings { .. } => Some("save_settings"),
            ChromeCommand::SetSitePermission { .. } => Some("set_site_permission"),
            ChromeCommand::SetDefaultPermission { .. } => Some("set_default_permission"),
            ChromeCommand::ReopenTab => Some("reopen_tab"),
            ChromeCommand::WindowDragStart => Some("window_drag"),
            ChromeCommand::WindowClose => Some("window_close"),
            ChromeCommand::WindowMinimize => Some("window_minimize"),
            ChromeCommand::WindowMaximize => Some("window_maximize"),
            ChromeCommand::ThemeToggle => Some("theme_toggle"),
            ChromeCommand::SidebarToggle => Some("sidebar_toggle"),
            ChromeCommand::FocusAddressBar => Some("focus_address_bar"),
            ChromeCommand::OpenTabSearch => Some("open_tab_search"),
            ChromeCommand::OpenFindBar => Some("open_find_bar"),
            ChromeCommand::GetPageText => Some("get_page_text"),
            ChromeCommand::TranslatePage { .. } => Some("translate_page"),
            ChromeCommand::SplitView { .. } => Some("split_view"),
            ChromeCommand::CloseSplit => Some("close_split"),
            ChromeCommand::OpenDevtools => Some("open_devtools"),
            ChromeCommand::RestartForWebSecurity => Some("restart_for_web_security"),
            ChromeCommand::DevRestart => Some("dev_restart"),
            ChromeCommand::DevOpenDataFolder => Some("dev_open_data_folder"),
            ChromeCommand::DevCopyDiagnostics => Some("dev_copy_diagnostics"),
            ChromeCommand::DevResetOnboarding => Some("dev_reset_onboarding"),
            ChromeCommand::DevTestReport => Some("dev_test_report"),
            ChromeCommand::DevTestCrash => Some("dev_test_crash"),
            ChromeCommand::HistoryClear => Some("history_clear"),
            ChromeCommand::ExportSettings => Some("export_settings"),
            ChromeCommand::ImportSettings { .. } => Some("import_settings"),
            ChromeCommand::OmniboxPick { .. } => Some("omnibox_pick"),
            ChromeCommand::OmniboxSetPref { .. } => Some("omnibox_set_pref"),
            ChromeCommand::RefreshTrends => Some("refresh_trends"),
            ChromeCommand::DeleteHistoryEntry { .. } => Some("delete_history_entry"),
            ChromeCommand::GetHistoryPage { .. } => Some("get_history_page"),
            ChromeCommand::DeleteHistoryDay { .. } => Some("delete_history_day"),
            ChromeCommand::PwdSaveConfirm => Some("password_save_confirm"),
            ChromeCommand::PwdSaveDismiss => Some("password_save_dismiss"),
            ChromeCommand::PwdList => Some("password_list"),
            ChromeCommand::PwdReveal { .. } => Some("password_reveal"),
            ChromeCommand::PwdDelete { .. } => Some("password_delete"),
            ChromeCommand::OpenFile { .. } => Some("open_file"),
            ChromeCommand::RevealFile { .. } => Some("reveal_file"),
            ChromeCommand::DragEdgePeek => Some("drag_edge_peek"),
            ChromeCommand::CheckForUpdate => Some("check_update"),
            ChromeCommand::LoadNeuraFeed => Some("load_neura_feed"),
            ChromeCommand::InstallUpdate => Some("install_update"),
            ChromeCommand::ZoomSet { .. } => Some("zoom_set"),
            ChromeCommand::ZoomReset => Some("zoom_reset"),
            ChromeCommand::ZoomDelta { .. } => Some("zoom_delta"),
            ChromeCommand::ZoomGlobal { .. } => Some("zoom_global"),
            ChromeCommand::ToggleFullscreen => Some("toggle_fullscreen"),
            ChromeCommand::ContentFullscreenChange { .. } => Some("content_fullscreen_change"),
            ChromeCommand::PeekSidebar => Some("peek_sidebar"),
            ChromeCommand::ClearDownloads => Some("clear_downloads"),
            ChromeCommand::DeleteDownload { .. } => Some("delete_download"),
            ChromeCommand::PauseDownload { .. } => Some("pause_download"),
            ChromeCommand::ResumeDownload { .. } => Some("resume_download"),
            ChromeCommand::CancelDownload { .. } => Some("cancel_download"),
            ChromeCommand::OpenInNewTab { .. } => Some("open_in_new_tab"),
            ChromeCommand::ContextMenuSaveImage { .. } => Some("save_image"),
            ChromeCommand::CopyImage { .. } => Some("copy_image"),
            ChromeCommand::OpenInNewWindow { .. } => Some("open_in_new_window"),
            ChromeCommand::DismissUpdate { .. } => Some("dismiss_update"),
            ChromeCommand::BeginSpotlight => Some("begin_spotlight"),
            ChromeCommand::EndSpotlight => Some("end_spotlight"),
            ChromeCommand::OpenHistoryPanel => Some("open_history_panel"),
            ChromeCommand::OpenDownloadsPanel => Some("open_downloads_panel"),
            ChromeCommand::SpotlightAiQuery { .. } => Some("spotlight_ai_query"),
            ChromeCommand::TabAudioState { .. } => Some("tab_audio_state"),
            ChromeCommand::MuteTab { .. } => Some("mute_tab"),
            ChromeCommand::BeginResize { .. } => Some("begin_resize"),
            ChromeCommand::AdBlockToggleSite => Some("adblock_toggle_site"),
            ChromeCommand::XLoginCompatibilityToggle => Some("x_login_compatibility_toggle"),
            ChromeCommand::AdBlockStats { .. } => Some("adblock_stats"),
            ChromeCommand::FetchCurrencyRates => Some("fetch_currency_rates"),
            ChromeCommand::OpenIncognito => Some("open_incognito"),
            ChromeCommand::AuthSignUp { .. } => Some("auth_sign_up"),
            ChromeCommand::AuthSignIn { .. } => Some("auth_sign_in"),
            ChromeCommand::AuthSignInGoogle => Some("auth_google"),
            ChromeCommand::AuthSignOut => Some("auth_sign_out"),
            ChromeCommand::AccountUpdateProfile { .. } => Some("account_update_profile"),
            ChromeCommand::AccountSetPhoto { .. } => Some("account_set_photo"),
            ChromeCommand::AccountChangePassword { .. } => Some("account_change_password"),
            ChromeCommand::GetAccountState => Some("get_account_state"),
            ChromeCommand::PermissionDecision { .. } => Some("permission_decision"),
            ChromeCommand::SendReport { .. } => Some("send_report"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Chrome(ChromeCommand),
    ChromeReady,
    SaveSession {
        id: u64,
    },
    AppPanelSleep {
        id: u64,
    },
    /// Timer-driven safety net: force-dismiss the loading cover if it has obscured
    /// the content WebView for too long (see `COVER_MAX_MS` in main.rs). While the
    /// cover is up the chrome owns the whole window (full-window clip + top
    /// Z-order), so the content is dark and non-interactive; this guarantees it can
    /// never stay stuck even if the normal progress/end/recovery signals never
    /// arrive. `id` is matched against the latest armed watchdog so stale timers
    /// no-op.
    CoverWatchdog {
        id: u64,
    },
    Shortcut {
        code: u32,
    },
    FindResult {
        tab_id: String,
        result: String,
    },
    ContentNav {
        tab_id: String,
        url: String,
        title: String,
    },
    ContentLoadStart {
        tab_id: String,
        url: String,
        native: bool,
        nav_id: u64,
    },
    ContentFirstPaint {
        tab_id: String,
    },
    ContentLoadEnd {
        tab_id: String,
        url: String,
        start_url: String,
        nav_id: u64,
    },
    ContentLoadProgress {
        tab_id: String,
        url: String,
        progress: f64,
    },
    ContentLoadStalled {
        tab_id: String,
        url: String,
        watch: u64,
    },
    ContentSpaStalled {
        tab_id: String,
        url: String,
        watch: u64,
    },
    /// Early "is this tab black?" probe, fired sooner than the full stall timeout.
    /// Only rebuilds the controller if the active tab never committed a document
    /// (pure black). A committed-but-slow page is left for the gentle stall path.
    ContentBlackProbe {
        tab_id: String,
        url: String,
        watch: u64,
    },
    ContentNavigationFailed {
        tab_id: String,
        url: String,
        status: i32,
        nav_id: u64,
    },
    ContentStrictBlocked {
        tab_id: String,
        url: String,
    },
    UbolReady {
        tab_id: String,
        url: String,
    },
    UbolSyncComplete {
        generation: u64,
    },
    UbolSyncTimeout {
        generation: u64,
    },
    HttpsUpgradeFailed {
        tab_id: String,
        https_url: String,
        http_url: String,
    },
    ContentMetadata {
        tab_id: String,
        url: String,
        title: String,
        favicon: Option<String>,
        replace: bool,
    },
    FaviconCached {
        domain: String,
        favicon_url: String,
        data_uri: String,
    },
    ContentTitle {
        tab_id: String,
        title: String,
    },
    ContentPageText {
        tab_id: String,
        text: String,
    },
    TranslateUnavailable {
        host: String,
        silent: bool,
    },
    TranslateAvailable {
        host: String,
    },
    /// A content WebView's site requested a browser permission (camera, mic, etc.).
    /// Recorded per-origin so the site-info popover can show only what was actually asked for.
    PermissionRequested {
        origin: String,
        key: String,
    },
    /// A site requested a permission that resolves to "ask". The native handler took
    /// a deferral keyed by `id`; chrome shows a prompt and replies with PermissionDecision.
    PermissionPrompt {
        id: String,
        origin: String,
        key: String,
    },
    PwdFillRequest {
        tab_id: String,
        origin: String,
    },
    PwdCapture {
        tab_id: String,
        origin: String,
        username: String,
        password: String,
    },
    AiChunk {
        text: String,
        done: bool,
    },
    AiError {
        message: String,
    },
    ContentBlurReady(String),
    DownloadStarted {
        id: Option<String>,
        url: String,
        filename: String,
        path: String,
        total: Option<u64>,
    },
    /// Live byte-count update for a native WebView2 download, keyed by the id handed
    /// out in `DownloadStarted`. `total` may still be unknown early in the transfer.
    DownloadProgress {
        id: String,
        received: u64,
        total: Option<u64>,
    },
    /// A native download was paused by the user (WebView2 reports USER_PAUSED).
    DownloadPaused {
        id: String,
    },
    DownloadResume {
        id: String,
    },
    AccelProbe {
        id: String,
        url: String,
    },
    AccelDecision {
        id: String,
        accelerate: bool,
        url: String,
        final_url: String,
        total: u64,
        referer: String,
    },
    /// A native download reached a terminal state. `canceled` distinguishes a user
    /// cancel from a real failure.
    DownloadDone {
        id: String,
        success: bool,
        canceled: bool,
    },
    DownloadCompleted {
        url: String,
        path: Option<String>,
        success: bool,
    },
    /// Result of a "Copy image" operation (Win32 clipboard write).
    CopyImageResult {
        success: bool,
    },
    /// Result of a "Copy image" fetch performed inside the content WebView.
    /// `data` is a base64 data URL when `ok`; otherwise empty and `src` is the
    /// original image URL so the main loop can fall back to a server-side fetch.
    CopyImageData {
        ok: bool,
        data: String,
        src: String,
    },
    /// Result of a "Save image as" fetch performed inside the content WebView.
    /// `data` is a base64 data URL when `ok`; otherwise empty. Matched to the
    /// pending destination path by `id` in the main loop.
    SaveImageData {
        id: String,
        ok: bool,
        data: String,
    },
    UpdateCheckResult {
        available: bool,
        version: String,
        notes: String,
        download_url: String,
    },
    UpdateCheckFailed {
        message: String,
    },
    NeuraFeedLoaded {
        articles: serde_json::Value,
    },
    NeuraFeedFailed {
        message: String,
    },
    TrendsLoaded {
        region: String,
        trends: Vec<crate::browser::omnibox::Trend>,
        fetched_at: i64,
    },
    TrendsFailed {
        region: String,
    },
    SearchSuggestionsLoaded {
        q: String,
        id: u64,
        items: Vec<String>,
    },
    UpdateDownloadProgress {
        received: u64,
        total: u64,
    },
    UpdateDownloaded {
        path: String,
    },
    UpdateDownloadFailed {
        message: String,
    },
    ContentNavState {
        tab_id: String,
        can_back: bool,
        can_forward: bool,
    },
    /// Native WebView2 (browser-process) audio-playing signal for a tab. Unlike the JS
    /// `tab_audio_state` heartbeat it is computed by the browser process, so it is immune
    /// to background-tab timer throttling — this is what keeps a backgrounded media tab
    /// (e.g. a YouTube livestream in another tab) from being put to sleep.
    ContentAudioPlaying {
        tab_id: String,
        playing: bool,
    },
    ContentContextMenu {
        tab_id: String,
        x: f64,
        y: f64,
        link_url: String,
        image_src: String,
        selected_text: String,
        page_url: String,
        can_back: bool,
    },
    /// Agent loop → main thread: execute JS in the active content WebView, result posted back via IPC
    AiExecutePageJs {
        call_id: String,
        tab_id: String,
        js: String,
    },
    /// Content WebView IPC → main thread: result of an AI page-tool JS execution
    AiToolResult {
        call_id: String,
        result: String,
    },
    /// Agent loop → chrome UI: display a tool call in the AI sidebar
    AiToolCallDisplay {
        label: String,
    },
    /// Agent loop → main thread: persist the completed exchange to ai_messages
    AiSaveMessages {
        user_text: String,
        assistant_text: String,
        attachments: Vec<crate::ai::AiAttachment>,
    },
    /// Spotlight AI answer chunk (streamed back from the AI task)
    SpotlightAiChunk {
        text: String,
        done: bool,
    },
    /// Spotlight AI answer error
    SpotlightAiError {
        message: String,
    },
    CurrencyRatesLoaded {
        rates: serde_json::Value,
    },
    CurrencyRatesFailed,
    /// WebView2 renderer process for a specific tab crashed or became unresponsive.
    /// The main thread auto-reloads the tab so the user doesn't see a permanent blank page.
    ContentProcessFailed {
        tab_id: String,
        fatal: bool,
    },
    ContentSubprocessCrashed {
        tab_id: String,
    },
    /// A sized popup (OAuth, share, payment) was requested. The main loop drains the
    /// pending-popup queue and builds a Ventus-wrapped window for each.
    CreatePopupWindow,
    /// A new window for blob:/data: content was requested. These can't be reopened by url
    /// in a fresh tab, so the main loop drains the handoff queue and hands each one a real
    /// content WebView (via SetNewWindow) that lives as a normal tab.
    CreateTabFromHandoff,
    /// Close a wrapped popup window (its close button, or JS window.close()).
    PopupClose {
        id: u64,
    },
    /// Begin moving a wrapped popup window (top-bar drag).
    PopupDrag {
        id: u64,
    },
    /// The popup's content navigated; update its top-bar origin display.
    PopupUrlChanged {
        id: u64,
        url: String,
    },
    AuthApplied {
        session: AuthSession,
        profile: UserProfile,
        message: String,
    },
    AuthError {
        message: String,
    },
    SyncPulled {
        bookmarks: Option<String>,
        history: Option<String>,
        settings: Option<String>,
    },
    SyncPush {
        id: u64,
    },
    ReportSent {
        ok: bool,
    },
    /// A page created a web Notification (intercepted by the content JS shim). Shown as a
    /// native Windows toast instead of WebView2's default banner.
    WebNotification {
        tab_id: String,
        id: String,
        title: String,
        body: String,
        icon: String,
        origin: String,
    },
    WebNotificationReady {
        tab_id: String,
        id: String,
        title: String,
        body: String,
        site: String,
        icon: String,
    },
    /// The page called notification.close() — dismiss the matching toast.
    WebNotificationClose {
        tab_id: String,
        id: String,
    },
    /// The user clicked a native toast — focus the tab and fire the notification's onclick.
    NotificationClicked {
        tab_id: String,
        id: String,
    },
    /// A native toast was dismissed — fire the notification's onclose.
    NotificationClosed {
        tab_id: String,
        id: String,
    },
}

impl AppEvent {
    pub fn label(&self) -> &'static str {
        match self {
            AppEvent::Chrome(_) => "chrome_cmd",
            AppEvent::ChromeReady => "chrome_ready",
            AppEvent::SaveSession { .. } => "save_session",
            AppEvent::AppPanelSleep { .. } => "app_panel_sleep",
            AppEvent::CoverWatchdog { .. } => "cover_watchdog",
            AppEvent::Shortcut { .. } => "shortcut",
            AppEvent::FindResult { .. } => "find_result",
            AppEvent::ContentNav { .. } => "content_nav",
            AppEvent::ContentLoadStart { .. } => "content_load_start",
            AppEvent::ContentFirstPaint { .. } => "content_first_paint",
            AppEvent::ContentLoadEnd { .. } => "content_load_end",
            AppEvent::ContentLoadProgress { .. } => "content_load_progress",
            AppEvent::ContentLoadStalled { .. } => "content_load_stalled",
            AppEvent::ContentSpaStalled { .. } => "content_spa_stalled",
            AppEvent::ContentBlackProbe { .. } => "content_black_probe",
            AppEvent::ContentNavigationFailed { .. } => "content_navigation_failed",
            AppEvent::ContentStrictBlocked { .. } => "content_strict_blocked",
            AppEvent::UbolReady { .. } => "ubol_ready",
            AppEvent::UbolSyncComplete { .. } => "ubol_sync_complete",
            AppEvent::UbolSyncTimeout { .. } => "ubol_sync_timeout",
            AppEvent::HttpsUpgradeFailed { .. } => "https_upgrade_failed",
            AppEvent::ContentMetadata { .. } => "content_metadata",
            AppEvent::FaviconCached { .. } => "favicon_cached",
            AppEvent::ContentTitle { .. } => "content_title",
            AppEvent::ContentPageText { .. } => "content_page_text",
            AppEvent::TranslateUnavailable { .. } => "translate_unavailable",
            AppEvent::TranslateAvailable { .. } => "translate_available",
            AppEvent::PermissionRequested { .. } => "permission_requested",
            AppEvent::PermissionPrompt { .. } => "permission_prompt",
            AppEvent::PwdFillRequest { .. } => "pwd_fill_request",
            AppEvent::PwdCapture { .. } => "pwd_capture",
            AppEvent::AiChunk { .. } => "ai_chunk",
            AppEvent::AiError { .. } => "ai_error",
            AppEvent::ContentBlurReady(_) => "content_blur_ready",
            AppEvent::DownloadStarted { .. } => "download_started",
            AppEvent::DownloadProgress { .. } => "download_progress",
            AppEvent::DownloadPaused { .. } => "download_paused",
            AppEvent::DownloadResume { .. } => "download_resume",
            AppEvent::AccelProbe { .. } => "accel_probe",
            AppEvent::AccelDecision { .. } => "accel_decision",
            AppEvent::DownloadDone { .. } => "download_done",
            AppEvent::DownloadCompleted { .. } => "download_completed",
            AppEvent::CopyImageResult { .. } => "copy_image_result",
            AppEvent::CopyImageData { .. } => "copy_image_data",
            AppEvent::SaveImageData { .. } => "save_image_data",
            AppEvent::UpdateCheckResult { .. } => "update_check_result",
            AppEvent::UpdateCheckFailed { .. } => "update_check_failed",
            AppEvent::NeuraFeedLoaded { .. } => "neura_feed_loaded",
            AppEvent::NeuraFeedFailed { .. } => "neura_feed_failed",
            AppEvent::TrendsLoaded { .. } => "trends_loaded",
            AppEvent::TrendsFailed { .. } => "trends_failed",
            AppEvent::SearchSuggestionsLoaded { .. } => "search_suggestions_loaded",
            AppEvent::UpdateDownloadProgress { .. } => "update_download_progress",
            AppEvent::UpdateDownloaded { .. } => "update_downloaded",
            AppEvent::UpdateDownloadFailed { .. } => "update_download_failed",
            AppEvent::ContentNavState { .. } => "content_nav_state",
            AppEvent::ContentAudioPlaying { .. } => "content_audio_playing",
            AppEvent::ContentContextMenu { .. } => "content_context_menu",
            AppEvent::AiExecutePageJs { .. } => "ai_execute_page_js",
            AppEvent::AiToolResult { .. } => "ai_tool_result",
            AppEvent::AiToolCallDisplay { .. } => "ai_tool_call_display",
            AppEvent::AiSaveMessages { .. } => "ai_save_messages",
            AppEvent::SpotlightAiChunk { .. } => "spotlight_ai_chunk",
            AppEvent::SpotlightAiError { .. } => "spotlight_ai_error",
            AppEvent::CurrencyRatesLoaded { .. } => "currency_rates_loaded",
            AppEvent::CurrencyRatesFailed => "currency_rates_failed",
            AppEvent::ContentProcessFailed { .. } => "content_process_failed",
            AppEvent::ContentSubprocessCrashed { .. } => "content_subprocess_crashed",
            AppEvent::CreatePopupWindow => "create_popup_window",
            AppEvent::CreateTabFromHandoff => "create_tab_from_handoff",
            AppEvent::PopupClose { .. } => "popup_close",
            AppEvent::PopupDrag { .. } => "popup_drag",
            AppEvent::PopupUrlChanged { .. } => "popup_url_changed",
            AppEvent::AuthApplied { .. } => "auth_applied",
            AppEvent::AuthError { .. } => "auth_error",
            AppEvent::SyncPulled { .. } => "sync_pulled",
            AppEvent::SyncPush { .. } => "sync_push",
            AppEvent::ReportSent { .. } => "report_sent",
            AppEvent::WebNotification { .. } => "web_notification",
            AppEvent::WebNotificationReady { .. } => "web_notification_ready",
            AppEvent::WebNotificationClose { .. } => "web_notification_close",
            AppEvent::NotificationClicked { .. } => "notification_clicked",
            AppEvent::NotificationClosed { .. } => "notification_closed",
        }
    }
}
