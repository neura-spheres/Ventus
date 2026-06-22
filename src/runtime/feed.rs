fn slim_feed_articles(json: &serde_json::Value) -> serde_json::Value {
    let Some(items) = json.get("articles").and_then(|v| v.as_array()) else {
        return serde_json::json!([]);
    };
    serde_json::Value::Array(
        items
            .iter()
            .map(|a| {
                serde_json::json!({
                    "title": a.get("title").cloned().unwrap_or_default(),
                    "summary": a.get("summary").cloned().unwrap_or_default(),
                    "whyItMatters": a.get("whyItMatters").cloned().unwrap_or_default(),
                    "coverImage": a.get("coverImage").cloned().unwrap_or_default(),
                    "imageSource": a.get("imageSource").cloned().unwrap_or_default(),
                    "imageSourceUrl": a.get("imageSourceUrl").cloned().unwrap_or_default(),
                    "sources": a.get("sources").cloned().unwrap_or_default(),
                    "createdAt": a.get("createdAt").cloned().unwrap_or_default(),
                })
            })
            .collect(),
    )
}

fn refresh_trends(
    state: &mut AppState,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
    rt: &tokio::runtime::Runtime,
) {
    let region = browser::trends::clean_region(&state.settings.region);
    let now = chrono::Utc::now().timestamp_millis();
    if state.trends_loading {
        return;
    }
    if state.trends_region == region
        && now - state.trends_fetched_at < 600_000
        && !state.trends.is_empty()
    {
        return;
    }
    state.trends_loading = true;
    let proxy = proxy.clone();
    rt.spawn(async move {
        match browser::trends::fetch(&region).await {
            Ok(trends) => {
                let _ = proxy.send_event(AppEvent::TrendsLoaded {
                    region,
                    trends,
                    fetched_at: chrono::Utc::now().timestamp_millis(),
                });
            }
            Err(_) => {
                let _ = proxy.send_event(AppEvent::TrendsFailed { region });
            }
        }
    });
}
