fn save_session(state: &AppState) {
    let active_id = state.tab_manager.active_tab_id.as_deref();
    let result = repositories::save_session(
        &state.conn,
        &state.tab_manager.workspaces,
        &state.tab_manager.active_workspace_id,
        &state.tab_manager.tabs,
        active_id,
    );
    match result {
        Ok(()) => tracing::debug!(
            target: "ventus::session",
            tab_count = state.tab_manager.tabs.len(),
            active_tab = ?active_id,
            "[SESSION] saved session snapshot"
        ),
        Err(e) => tracing::warn!(
            target: "ventus::session",
            error = %e,
            "[SESSION] save_session FAILED — last session may not restore correctly"
        ),
    }
}

fn queue_session_save(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    id: &mut u64,
) {
    *id = id.wrapping_add(1).max(1);
    let next = *id;
    let proxy = proxy.clone();
    rt.spawn(async move {
        tokio::time::sleep(SESSION_SAVE_DELAY).await;
        let _ = proxy.send_event(AppEvent::SaveSession { id: next });
    });
}

fn cloud_sync_kinds(event: &AppEvent) -> Option<(bool, bool, bool)> {
    match event {
        AppEvent::SyncPulled { .. } => Some((true, true, true)),
        AppEvent::ContentMetadata { .. } => Some((false, true, false)),
        AppEvent::Chrome(cmd) => match cmd {
            ChromeCommand::BookmarkAdd
            | ChromeCommand::BookmarkAddUrl { .. }
            | ChromeCommand::MoveBookmark { .. }
            | ChromeCommand::BookmarkRemove { .. }
            | ChromeCommand::BookmarkRemoveById { .. }
            | ChromeCommand::BookmarkCreateFolder { .. }
            | ChromeCommand::BookmarkMoveToFolder { .. }
            | ChromeCommand::BookmarkRemoveFromFolder { .. }
            | ChromeCommand::BookmarkFolderRename { .. }
            | ChromeCommand::BookmarkFolderDelete { .. } => Some((true, false, false)),
            ChromeCommand::HistoryClear | ChromeCommand::DeleteHistoryEntry { .. } => {
                Some((false, true, false))
            }
            ChromeCommand::SaveSettings { .. } => Some((false, false, true)),
            _ => None,
        },
        _ => None,
    }
}

fn queue_cloud_sync(
    rt: &tokio::runtime::Runtime,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    id: &mut u64,
) {
    *id = id.wrapping_add(1).max(1);
    let next = *id;
    let proxy = proxy.clone();
    rt.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let _ = proxy.send_event(AppEvent::SyncPush { id: next });
    });
}

fn cloud_bookmarks_blob(state: &AppState) -> String {
    let bookmarks = repositories::list_bookmarks(&state.conn).unwrap_or_default();
    let folders = repositories::list_bookmark_folders(&state.conn).unwrap_or_default();
    serde_json::json!({ "bookmarks": bookmarks, "folders": folders }).to_string()
}

fn cloud_history_blob(state: &AppState) -> String {
    let history = repositories::list_history(&state.conn, 150).unwrap_or_default();
    serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string())
}

fn should_save_session(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::ContentNav { .. }
            | AppEvent::ContentMetadata { .. }
            | AppEvent::Chrome(ChromeCommand::Navigate { .. })
            | AppEvent::Chrome(ChromeCommand::NavigateFromOverlay { .. })
            | AppEvent::Chrome(ChromeCommand::Back)
            | AppEvent::Chrome(ChromeCommand::Forward)
            | AppEvent::Chrome(ChromeCommand::NewTab)
            | AppEvent::Chrome(ChromeCommand::CloseTab { .. })
            | AppEvent::Chrome(ChromeCommand::SwitchTab { .. })
            | AppEvent::Chrome(ChromeCommand::PinTab { .. })
            | AppEvent::Chrome(ChromeCommand::UnpinTab { .. })
            | AppEvent::Chrome(ChromeCommand::NewWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::RenameWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::DeleteWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::SwitchWorkspace { .. })
            | AppEvent::Chrome(ChromeCommand::ReopenTab)
            | AppEvent::Chrome(ChromeCommand::OpenInNewTab { .. })
    )
}

fn notification_site(origin: &str, fallback: &str) -> String {
    let site = notification_site_label(origin);
    if !site.is_empty() {
        return site;
    }
    notification_site_label(fallback)
}

fn notification_site_label(raw: &str) -> String {
    let Ok(url) = url::Url::parse(raw) else {
        return String::new();
    };
    let Some(host) = url.host_str() else {
        return String::new();
    };
    site_case(&site_stem(host))
}

fn site_stem(host: &str) -> String {
    let labels: Vec<&str> = host.split('.').filter(|p| !p.is_empty()).collect();
    if labels.is_empty() {
        return String::new();
    }
    if labels.len() == 1 {
        return labels[0].to_string();
    }
    let mut idx = labels.len().saturating_sub(2);
    let slds = ["ac", "co", "com", "edu", "gov", "net", "org"];
    if labels.last().map(|p| p.len() == 2).unwrap_or(false)
        && slds.contains(&labels[idx])
        && idx > 0
    {
        idx -= 1;
    }
    labels[idx].to_string()
}

fn site_case(stem: &str) -> String {
    let lower = stem.to_ascii_lowercase();
    match lower.as_str() {
        "whatsapp" => "WhatsApp".to_string(),
        "youtube" => "YouTube".to_string(),
        "chatgpt" => "ChatGPT".to_string(),
        "github" => "GitHub".to_string(),
        "gmail" => "Gmail".to_string(),
        "google" => "Google".to_string(),
        "linkedin" => "LinkedIn".to_string(),
        _ => lower
            .split(['-', '_'])
            .filter(|p| !p.is_empty())
            .map(title_word)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn title_word(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first.to_uppercase().chain(chars).collect()
}

fn notification_icons(icon: &str, origin: &str, favicon: Option<&str>) -> Vec<String> {
    let mut icons = Vec::new();
    push_icon(&mut icons, notification_icon_url(icon, origin));
    push_icon(
        &mut icons,
        favicon.and_then(|v| notification_icon_url(v, origin)),
    );
    push_icon(&mut icons, origin_favicon(origin));
    push_icon(&mut icons, google_favicon(origin));
    icons
}

fn push_icon(icons: &mut Vec<String>, icon: Option<String>) {
    let Some(icon) = icon else {
        return;
    };
    if icons.iter().any(|i| i == &icon) {
        return;
    }
    icons.push(icon);
}

fn notification_icon_url(raw: &str, base: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with("data:") || raw.starts_with("blob:") {
        return None;
    }
    let url = url::Url::parse(raw)
        .or_else(|_| url::Url::parse(base).and_then(|base| base.join(raw)))
        .ok()?;
    if !matches!(url.scheme(), "http" | "https" | "file") {
        return None;
    }
    Some(url.to_string())
}

fn origin_favicon(origin: &str) -> Option<String> {
    let mut url = url::Url::parse(origin).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_path("/favicon.ico");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

fn google_favicon(origin: &str) -> Option<String> {
    let url = url::Url::parse(origin).ok()?;
    let host = url.host_str()?;
    Some(format!(
        "https://www.google.com/s2/favicons?domain={}&sz=64",
        host
    ))
}

async fn cache_notification_icon(icons: Vec<String>) -> String {
    for icon in icons {
        if let Some(path) = cache_notification_icon_url(&icon).await {
            return path;
        }
    }
    String::new()
}

async fn cache_notification_icon_url(icon: &str) -> Option<String> {
    let url = url::Url::parse(icon).ok()?;
    if url.scheme() == "file" {
        return Some(icon.to_string());
    }
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let path = notification_icon_path(icon);
    if path.is_file() {
        return notification_icon_file_url(&path);
    }
    let bytes = download_notification_icon(icon).await?;
    write_notification_icon(&path, &bytes)
}

async fn download_notification_icon(icon: &str) -> Option<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent(browser_user_agent())
        .build()
        .ok()?;
    let resp = client.get(icon).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > 1_500_000 {
        return None;
    }
    Some(bytes.to_vec())
}

fn write_notification_icon(path: &std::path::Path, bytes: &[u8]) -> Option<String> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = img.resize(64, 64, image::imageops::FilterType::Lanczos3);
    let dir = path.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .ok()?;
    std::fs::write(path, buf).ok()?;
    notification_icon_file_url(path)
}

fn notification_icon_path(icon: &str) -> std::path::PathBuf {
    utils::platform::data_dir()
        .join("notification-icons")
        .join(format!("{}.png", notification_icon_hash(icon)))
}

fn notification_icon_file_url(path: &std::path::Path) -> Option<String> {
    url::Url::from_file_path(path)
        .ok()
        .map(|url| url.to_string())
}

fn notification_icon_hash(icon: &str) -> u64 {
    let mut h = DefaultHasher::new();
    icon.hash(&mut h);
    h.finish()
}

fn repeated_native_load_start(state: &AppState, event: &AppEvent) -> bool {
    let AppEvent::ContentLoadStart {
        tab_id,
        url,
        native,
        ..
    } = event
    else {
        return false;
    };
    if !*native {
        return false;
    }
    let clean_url = crate::utils::url::clean_tracking_url(url);
    if clean_url.trim().is_empty()
        || clean_url == "about:blank"
        || clean_url.starts_with("neura://")
    {
        return false;
    }
    let Some(tab) = state.tab_manager.get_tab(tab_id) else {
        return false;
    };
    if tab.status != crate::browser::tab::TabStatus::Loading {
        return false;
    }
    let current_match = app::same_nav(&tab.url, &clean_url);
    let pending_match = state
        .pending_nav_urls
        .get(tab_id)
        .map(|expected| app::same_nav(expected, &clean_url))
        .unwrap_or(false);
    let native_match = state
        .native_loads
        .get(tab_id)
        .map(|expected| app::same_nav(expected, &clean_url))
        .unwrap_or(false);
    current_match || pending_match || native_match
}

fn stale_canceled_nav_failure(state: &AppState, event: &AppEvent) -> bool {
    let AppEvent::ContentNavigationFailed {
        tab_id,
        status,
        nav_id,
        ..
    } = event
    else {
        return false;
    };
    if !canceled_web_error_status(*status) {
        return false;
    }
    state
        .native_nav_ids
        .get(tab_id)
        .map(|id| *nav_id == 0 || *id != *nav_id)
        .unwrap_or(false)
}

fn save_window_size(window: &tao::window::Window, state: &mut AppState, maxed: bool) {
    if state.content_fullscreen || maxed {
        return;
    }
    let size = window.inner_size();
    let scale = window.scale_factor().max(1.0);
    state.settings.window_width = (size.width as f64 / scale).round() as u32;
    state.settings.window_height = (size.height as f64 / scale).round() as u32;
    let _ = settings_store::set(&state.conn, "app_settings", &state.settings);
}
