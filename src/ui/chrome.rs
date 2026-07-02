pub fn chrome_html() -> String {
    let logo = crate::ui::assets::logo_data_url();
    let logo_text_white = crate::ui::assets::logo_text_white_data_url();
    let logo_text_black = crate::ui::assets::logo_text_black_data_url();
    let version = crate::version::APP_VERSION;
    let html = include_str!("chrome.html");
    html.replace("__LOGO_URL__", &logo)
        .replace("__LOGO_TEXT_WHITE__", &logo_text_white)
        .replace("__LOGO_TEXT_BLACK__", &logo_text_black)
        .replace("__APP_VERSION__", version)
}

#[cfg(test)]
mod tests {
    use super::chrome_html;

    #[test]
    fn secure_dns_options_match_supported_provider_ids() {
        let html = chrome_html();
        for expected in [
            r#"<option value="cloudflare">Cloudflare 1.1.1.1</option>"#,
            r#"<option value="cloudflare_malware">Cloudflare Malware Protection</option>"#,
            r#"<option value="cloudflare_family">Cloudflare Family Protection</option>"#,
            r#"<option value="google">Google Public DNS</option>"#,
            r#"<option value="opendns">OpenDNS</option>"#,
            r#"<option value="opendns_family">OpenDNS FamilyShield</option>"#,
            r#"<option value="custom">Custom DoH Endpoint</option>"#,
        ] {
            assert!(
                html.contains(expected),
                "missing Secure DNS option: {expected}"
            );
        }
    }

    #[test]
    fn secure_dns_copy_no_longer_promises_restart() {
        let html = chrome_html();
        assert!(html.contains("Ventus stays open; active web pages refresh to use DNS changes."));
        assert!(!html.contains("Ventus restarts after DNS changes"));
    }

    #[test]
    fn ai_attachment_picker_is_wired() {
        let html = chrome_html();
        assert!(html.contains("onclick=\"pickAiAttachments()\""));
        assert!(html.contains("send('PickAiAttachments')"));
        assert!(html.contains("send('RemoveAiAttachment', {id})"));
        assert!(!html.contains("Media attachments are not available yet"));
    }

    #[test]
    fn spotlight_keeps_predicted_search_rows() {
        let html = chrome_html();
        assert!(!html.contains("if (isSearch && item.predicted) continue;"));
        assert!(html.contains("group: 'Search suggestions'"));
    }

    #[test]
    fn address_bar_disables_spellcheck() {
        let html = chrome_html();
        assert!(html.contains(
            r#"<input id="url-input" type="text" placeholder="Search or enter a URL" autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false""#
        ));
    }

    #[test]
    fn phrase_suggestions_stay_list_only() {
        let html = chrome_html();
        assert!(html.contains("group: 'Search suggestions'"));
        assert!(!html.contains("function inlineQueryCandidates"));
        assert!(!html.contains("function inlineQueryText"));
    }

    #[test]
    fn suggestions_stay_visible_while_refreshing() {
        let html = chrome_html();
        assert!(html.contains("function matchingSearchSuggestions(q)"));
        assert!(html.contains("if (q.length === 1) return;"));
        assert!(html.contains("matching.length >= SEARCH_SUGGESTION_REUSE_MIN"));
        assert!(html.contains("if (next.length) searchSuggestions = next;"));
        assert!(!html.contains(
            "searchSuggestionId += 1;\n  searchSuggestions = [];\n  const q = String(raw || '').trim();"
        ));
    }

    #[test]
    fn address_suggestions_attach_under_address_bar() {
        let html = chrome_html();
        assert!(!html.contains("suggestions-detached"));
        assert!(!html.contains("#url-suggestions.detached"));
        assert!(!html.contains("panel.classList.toggle('detached', bookmarksVisible)"));
        assert!(html.contains(
            "document.addEventListener('dragstart', function(e) {\n  hideSuggestions();"
        ));
    }

    #[test]
    fn bookmark_folder_icons_toggle_the_modal() {
        let html = chrome_html();
        assert!(html.contains("openFolderModal('${escAttr(f.id)}',this,false,true)"));
        assert!(html.contains("if (toggle && !rename && _activeFolderId === folderId"));
        assert!(html.contains("closeFolderModal();\n    return;"));
    }

    #[test]
    fn context_actions_clear_the_menu_before_running() {
        let html = chrome_html();
        assert!(html.contains("onclick=\"runCtxAction(${i})\""));
        assert!(html.contains(
            "const action = _ctxActions[idx];\n  _hideCtxMenu();\n  if (action) action();"
        ));
        assert!(html.contains("menu.replaceChildren();\n  _ctxActions = [];"));
    }

    #[test]
    fn folder_bookmark_new_tab_closes_the_folder() {
        let html = chrome_html();
        assert!(html.contains(
            "if (el.closest('#folder-modal')) closeFolderModal();\n      send('OpenInNewTab', {url});"
        ));
    }

    #[test]
    fn wallpaper_sources_use_curated_providers() {
        let html = chrome_html();
        assert!(html.contains("https://bing.biturl.top/?resolution=1920"));
        assert_eq!(
            html.matches("https://images.unsplash.com/photo-").count(),
            20
        );
        assert!(!html.contains("NT_FALLBACK_PHOTOS"));
        assert!(!html.contains("picsum.photos"));
        assert!(!html.contains("WallpaperDebug"));
    }

    #[test]
    fn horizontal_add_opens_newtab_without_changing_vertical_add() {
        let html = chrome_html();
        assert!(html.contains(
            "const H_TAB_ADD = `<button class=\"horizontal-new-tab\" onclick=\"send('NewTab')\""
        ));
        assert!(html.contains("class=\"sidebar-brand-add\" onclick=\"openNewTabSpotlight()\""));
    }
}
