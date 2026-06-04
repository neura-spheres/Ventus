use serde::{Deserialize, Serialize};

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
    Back,
    Forward,
    Reload,
    Stop,
    NewTab,
    CloseTab {
        id: String,
    },
    SwitchTab {
        id: String,
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
    AiMessage {
        text: String,
    },
    AiProviderChange {
        provider: String,
    },
    AiModelChange {
        model: String,
    },
    AiQuickAction {
        action: String,
    },
    AiClearChat,
    BookmarkAdd,
    /// Save a bookmark from a dropped link (drag-to-bookmark).
    BookmarkAddUrl {
        url: String,
        #[serde(default)]
        title: String,
    },
    /// Reorder a bookmark. `before` is the id of the bookmark to insert ahead of, or None for end.
    MoveBookmark {
        id: String,
        #[serde(default)]
        before: Option<String>,
    },
    BookmarkRemove {
        url: String,
    },
    OpenSettings,
    CloseSettings,
    BrowseDownloadFolder,
    SaveSettings {
        key: String,
        value: serde_json::Value,
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
    GetPageText,
    SplitView {
        mode: String,
    },
    CloseSplit,
    OpenDevtools,
    HistoryClear,
    ExportSettings,
    ImportSettings {
        path: String,
    },
    GetHistory {
        q: String,
    },
    DeleteHistoryEntry {
        id: i64,
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
    OpenInNewTab {
        url: String,
    },
    ContextMenuSaveImage {
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
    },
    /// Content WebView reports audio/video playback state change for a tab.
    TabAudioState {
        tab_id: String,
        playing: bool,
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
    /// Reported by the content WebView init script: how many DOM elements were hidden/removed.
    AdBlockStats {
        killed: u32,
    },
    FetchCurrencyRates,
    OpenIncognito,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Chrome(ChromeCommand),
    ChromeReady,
    Shortcut {
        code: u32,
    },
    ContentNav {
        tab_id: String,
        url: String,
        title: String,
    },
    ContentLoadStart {
        tab_id: String,
        url: String,
    },
    ContentLoadEnd {
        tab_id: String,
        url: String,
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
    ContentNavigationFailed {
        tab_id: String,
        url: String,
        status: i32,
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
    ContentPageText {
        tab_id: String,
        text: String,
    },
    AiChunk {
        text: String,
        done: bool,
    },
    AiError {
        message: String,
    },
    DownloadStarted {
        url: String,
        filename: String,
        path: String,
    },
    DownloadCompleted {
        url: String,
        path: Option<String>,
        success: bool,
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
    /// A sized popup (OAuth, share, payment) was requested. The main loop drains the
    /// pending-popup queue and builds a Ventus-wrapped window for each.
    CreatePopupWindow,
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
}
