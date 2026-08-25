#[derive(Clone)]
struct CurrencyQuery {
    amount: f64,
    from: String,
    to: String,
}

fn detect_currency_query(q: &str) -> Option<CurrencyQuery> {
    let mut s = q.to_lowercase();
    s = s.replace(',', "");
    s = s.replace('$', " usd ");
    s = s.replace('€', " eur ");
    s = s.replace('£', " gbp ");
    s = s.replace('¥', " jpy ");
    for ch in ['?', '!', ':', ';', '(', ')', '[', ']', '{', '}'] {
        s = s.replace(ch, " ");
    }
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }

    let seps = ["to", "ke", "in", "into", "as", "for"];
    for (i, token) in tokens.iter().enumerate() {
        if !seps.contains(token) {
            continue;
        }
        let left = &tokens[..i];
        let right = &tokens[i + 1..];
        let (from, amount) = find_currency_left(left)?;
        let to = find_currency_right(right)?;
        if from != to {
            return Some(CurrencyQuery { amount, from, to });
        }
    }

    let found: Vec<(usize, String)> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, token)| currency_code(token).map(|code| (i, code.to_string())))
        .collect();
    if found.len() < 2 || found[0].1 == found[1].1 {
        return None;
    }
    let amount = tokens[..found[0].0]
        .iter()
        .rev()
        .find_map(|token| token.parse::<f64>().ok())
        .unwrap_or(1.0);
    Some(CurrencyQuery {
        amount,
        from: found[0].1.clone(),
        to: found[1].1.clone(),
    })
}

fn find_currency_left(tokens: &[&str]) -> Option<(String, f64)> {
    for token in tokens.iter().rev() {
        let Some(code) = currency_code(token) else {
            continue;
        };
        let amount = tokens
            .iter()
            .rev()
            .find_map(|token| token.parse::<f64>().ok())
            .unwrap_or(1.0);
        return Some((code.to_string(), amount));
    }
    None
}

fn find_currency_right(tokens: &[&str]) -> Option<String> {
    tokens
        .iter()
        .find_map(|token| currency_code(token).map(|code| code.to_string()))
}

fn currency_code(token: &str) -> Option<&'static str> {
    match token {
        "usd" | "dollar" | "dollars" | "buck" | "bucks" | "us$" => Some("USD"),
        "idr" | "rupiah" | "rp" => Some("IDR"),
        "jpy" | "yen" => Some("JPY"),
        "cny" | "yuan" | "rmb" | "renminbi" => Some("CNY"),
        "eur" | "euro" | "euros" => Some("EUR"),
        "gbp" | "pound" | "pounds" | "sterling" => Some("GBP"),
        "sgd" | "singapore" => Some("SGD"),
        "myr" | "ringgit" => Some("MYR"),
        "thb" | "baht" => Some("THB"),
        "aud" => Some("AUD"),
        "cad" => Some("CAD"),
        "chf" | "franc" => Some("CHF"),
        "hkd" => Some("HKD"),
        "krw" | "won" => Some("KRW"),
        "inr" | "rupee" | "rupees" => Some("INR"),
        "php" | "peso" | "pesos" => Some("PHP"),
        "vnd" | "dong" => Some("VND"),
        "twd" => Some("TWD"),
        "brl" | "real" | "reais" => Some("BRL"),
        "mxn" => Some("MXN"),
        "rub" | "ruble" | "rouble" => Some("RUB"),
        "zar" | "rand" => Some("ZAR"),
        "aed" | "dirham" => Some("AED"),
        "sar" | "riyal" => Some("SAR"),
        "try" | "lira" => Some("TRY"),
        "nok" => Some("NOK"),
        "sek" => Some("SEK"),
        "dkk" => Some("DKK"),
        "nzd" => Some("NZD"),
        _ => None,
    }
}

async fn fetch_currency_rate(query: &CurrencyQuery) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent(crate::version::USER_AGENT)
        .build()
        .ok()?;

    let url = format!("https://open.er-api.com/v6/latest/{}", query.from);
    let resp = client.get(&url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    if json["result"].as_str() != Some("success") {
        return None;
    }

    let rate = json["rates"][&query.to].as_f64()?;
    let converted = query.amount * rate;
    let updated = json["time_last_update_utc"].as_str().unwrap_or("recently");

    Some(format!(
        "**Live currency conversion:** {} {} = **{} {}**\nRate: 1 {} = {} {}.\nSource: open.er-api.com, updated: {}.",
        fmt_currency_value(query.amount),
        query.from,
        fmt_currency_value(converted),
        query.to,
        query.from,
        fmt_currency_value(rate),
        query.to,
        updated
    ))
}

fn fmt_currency_value(n: f64) -> String {
    let decimals = if n.abs() >= 100.0 { 2 } else { 4 };
    let s = format!("{n:.decimals$}");
    let parts: Vec<&str> = s.split('.').collect();
    let mut int = String::new();
    for (i, ch) in parts[0].chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            int.push(',');
        }
        int.push(ch);
    }
    let int = int.chars().rev().collect::<String>();
    if parts.len() == 1 {
        return int;
    }
    let frac = parts[1].trim_end_matches('0');
    if frac.is_empty() {
        int
    } else {
        format!("{int}.{frac}")
    }
}

#[cfg(test)]
mod webview_arg_tests {
    use super::*;

    fn secure_dns_cases() -> [(config::SecureDnsProvider, &'static str); 6] {
        [
            (
                config::SecureDnsProvider::Cloudflare,
                config::CLOUDFLARE_DOH,
            ),
            (
                config::SecureDnsProvider::CloudflareMalware,
                config::CLOUDFLARE_MALWARE_DOH,
            ),
            (
                config::SecureDnsProvider::CloudflareFamily,
                config::CLOUDFLARE_FAMILY_DOH,
            ),
            (config::SecureDnsProvider::Google, config::GOOGLE_DOH),
            (config::SecureDnsProvider::OpenDns, config::OPENDNS_DOH),
            (
                config::SecureDnsProvider::OpenDnsFamily,
                config::OPENDNS_FAMILY_DOH,
            ),
        ]
    }

    #[test]
    fn secure_dns_adds_cloudflare_args() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        let args = webview_args(&settings);
        assert!(args.contains("--enable-async-dns"));
        assert!(args.contains(&doh_feature_arg(
            config::CLOUDFLARE_DOH,
            &config::SecureDnsMode::Secure
        )));
        assert!(args.contains("--dns-over-https-mode=secure"));
        assert!(args.contains("--dns-over-https-templates=https://cloudflare-dns.com/dns-query"));
    }

    #[test]
    fn secure_dns_automatic_allows_fallback_in_feature_params() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        settings.privacy.secure_dns_mode = config::SecureDnsMode::Automatic;
        let args = webview_args(&settings);
        assert!(args.contains(&doh_feature_arg(
            config::CLOUDFLARE_DOH,
            &config::SecureDnsMode::Automatic
        )));
        assert!(args.contains("--dns-over-https-mode=automatic"));
    }

    #[test]
    fn secure_dns_args_cover_all_builtin_providers() {
        for (provider, endpoint) in secure_dns_cases() {
            let mut settings = config::AppSettings::default();
            settings.privacy.secure_dns_enabled = true;
            settings.privacy.secure_dns_provider = provider;

            let args = webview_args(&settings);

            assert!(args.contains("--enable-async-dns"));
            assert!(args.contains(&doh_feature_arg(endpoint, &config::SecureDnsMode::Secure)));
            assert!(args.contains("--dns-over-https-mode=secure"));
            assert!(args.contains(&format!("--dns-over-https-templates={endpoint}")));
        }
    }

    #[test]
    fn secure_dns_custom_args_require_valid_https_endpoint() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        settings.privacy.secure_dns_provider = config::SecureDnsProvider::Custom;
        settings.privacy.secure_dns_template = " https://dns.example/dns-query ".to_string();

        let args = webview_args(&settings);
        assert!(args.contains("--dns-over-https-templates=https://dns.example/dns-query"));

        for invalid in [
            "",
            "http://dns.example/dns-query",
            "https://dns.example/dns query",
        ] {
            settings.privacy.secure_dns_template = invalid.to_string();
            let args = webview_args(&settings);
            assert!(!args.contains("dns-over-https"));
        }
    }

    #[test]
    fn secure_dns_off_keeps_doh_out() {
        let settings = config::AppSettings::default();
        let args = webview_args(&settings);
        assert!(!args.contains("dns-over-https"));
    }

    #[test]
    fn content_background_matches_browser_default() {
        assert_eq!(CONTENT_BG, (255, 255, 255, 255));
    }

    #[test]
    fn content_identity_overrides_ua_data() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script = content_initialization_script(
            1.0,
            "",
            false,
            "test-seed",
            false,
            false,
            &sites,
            &defaults,
        );
        assert!(!script.contains("__ventusIdentity"));
        assert!(script.contains("userAgentData"));
        assert!(script.contains("{brand: 'Ventus'"));
        assert!(script.contains("{brand: 'Chromium'"));
        assert!(script.contains("{brand: 'Google Chrome'"));
        assert!(!script.contains("Microsoft Edge"));
    }

    #[test]
    fn browser_user_agent_keeps_chromium_and_ventus() {
        let ua = browser_user_agent();
        assert!(ua.contains("Win64; x64; Ventus"));
        assert!(ua.contains("Chrome/"));
        assert!(ua.contains("Safari/"));
        assert!(!ua.contains("Chrome/0.0.0.0"));
        assert!(!ua.contains("Edg/"));
        assert!(!ua.contains("Microsoft Edge"));
    }

    #[test]
    fn content_zoom_wheel_runs_before_page_handlers() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script = content_initialization_script(
            1.0,
            "",
            false,
            "test-seed",
            false,
            false,
            &sites,
            &defaults,
        );
        assert!(script.contains("window.addEventListener('wheel'"));
        assert!(script.contains("e.stopImmediatePropagation()"));
        assert!(!script.contains("document.addEventListener('wheel'"));
    }

    #[test]
    fn missing_web_tab_needs_content_view() {
        let web = crate::browser::tab::Tab::new("ws", "https://example.com");
        let internal = crate::browser::tab::Tab::new("ws", "neura://newtab");
        assert!(tab_needs_content(&web, false));
        assert!(!tab_needs_content(&web, true));
        assert!(!tab_needs_content(&internal, false));
    }

    #[test]
    fn fingerprint_noise_is_stable_for_profile_and_site() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script =
            privacy_initialization_script(true, "profile-seed", false, false, &sites, &defaults);
        assert!(script.contains("const fpProfileSeed = \"profile-seed\""));
        assert!(script.contains("fpHash(fpProfileSeed + '|' + location.origin)"));
        assert!(!script.contains("Math.random() * 0x7fffffff"));
    }

    #[test]
    fn fingerprint_compatibility_is_limited_to_x_auth_paths() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script =
            privacy_initialization_script(true, "profile-seed", true, false, &sites, &defaults);
        assert!(script.contains("const fpCompat = true"));
        assert!(script.contains("const fpAuthHost = fpHost === 'x.com'"));
        assert!(script.contains("fpPath.startsWith('/i/flow/')"));
        assert!(!script.contains("fpCompat && fpAuthHost && (location.protocol"));
    }

    #[test]
    fn strict_permissions_keep_clipboard_copy_available() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script =
            privacy_initialization_script(false, "test-seed", false, true, &sites, &defaults);
        assert!(script.contains("navigator.clipboard.read = blk('read');"));
        assert!(script.contains("navigator.clipboard.readText = blk('readText');"));
        assert!(!script.contains("navigator.clipboard.write = blocked;"));
        assert!(!script.contains("navigator.clipboard.writeText = blocked;"));
    }

    #[test]
    fn strict_permissions_keep_media_devices_available() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let script =
            privacy_initialization_script(false, "test-seed", false, true, &sites, &defaults);
        assert!(!script.contains("getUserMedia = function"));
        assert!(!script.contains("enumerateDevices = () => Promise.resolve([])"));
    }

    #[cfg(windows)]
    #[test]
    fn strict_permissions_ask_for_media_by_default() {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        assert_eq!(
            permission_action(
                true,
                &sites,
                &defaults,
                "https://meet.google.com",
                "microphone"
            ),
            "ask"
        );
        assert_eq!(
            permission_action(true, &sites, &defaults, "https://meet.google.com", "camera"),
            "ask"
        );
        assert_eq!(
            permission_action(
                true,
                &sites,
                &defaults,
                "https://meet.google.com",
                "geolocation"
            ),
            "block"
        );
    }

    #[cfg(windows)]
    #[test]
    fn explicit_media_permission_rules_win() {
        let mut sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        let mut perms = config::SitePermissions::default();
        assert!(perms.set("microphone", "block"));
        sites.insert("https://meet.google.com".to_string(), perms);
        assert_eq!(
            permission_action(
                true,
                &sites,
                &defaults,
                "https://meet.google.com",
                "microphone"
            ),
            "block"
        );

        let sites = config::SitePermissionMap::new();
        let mut defaults = config::SitePermissions::default();
        assert!(defaults.set("camera", "allow"));
        assert_eq!(
            permission_action(true, &sites, &defaults, "https://meet.google.com", "camera"),
            "allow"
        );
    }

    #[cfg(windows)]
    #[test]
    fn live_permission_policy_updates_without_rebuilding_tabs() {
        let mut settings = config::AppSettings::default();
        set_permission_policy(&settings);
        assert_eq!(
            current_permission_action("https://claude.ai", "microphone"),
            "ask"
        );

        let mut perms = config::SitePermissions::default();
        assert!(perms.set("microphone", "allow"));
        settings
            .privacy
            .site_permissions
            .insert("https://claude.ai".to_string(), perms);
        set_permission_policy(&settings);
        assert_eq!(
            current_permission_action("https://claude.ai", "microphone"),
            "allow"
        );
    }

    #[test]
    fn chromium_versions_parse_runtime_strings() {
        assert_eq!(
            chromium_versions_from_raw("149.0.3065.92").unwrap(),
            (
                "149.0.3065.92".to_string(),
                "149.0.0.0".to_string(),
                "149".to_string()
            )
        );
        assert_eq!(
            chromium_versions_from_raw("WebView2 Runtime 150.1").unwrap(),
            (
                "150.1.0.0".to_string(),
                "150.0.0.0".to_string(),
                "150".to_string()
            )
        );
        assert!(chromium_versions_from_raw("runtime unavailable").is_none());
    }

    #[test]
    fn chromium_identity_uses_modern_fallback_when_runtime_is_bad() {
        assert_eq!(
            chromium_identity_versions("runtime unavailable"),
            chrome_identity_fallback()
        );
        assert_eq!(
            chromium_identity_versions("109.0.1518.140"),
            chrome_identity_fallback()
        );
    }

    #[test]
    fn chromium_identity_keeps_recent_runtime() {
        assert_eq!(
            chromium_identity_versions("150.1"),
            (
                "150.1.0.0".to_string(),
                "150.0.0.0".to_string(),
                "150".to_string()
            )
        );
    }

    #[test]
    fn auth_popup_urls_stay_popups() {
        assert!(is_auth_popup_url(
            "https://accounts.google.com/gsi/confirm?client_id=abc"
        ));
        assert!(is_auth_popup_url(
            "https://x.com/i/flow/single_sign_on?provider=google"
        ));
        assert!(is_auth_popup_url(
            "https://login.live.com/oauth20_authorize.srf"
        ));
        assert!(!is_auth_popup_url(
            "https://example.com/article/oauth-history"
        ));
        assert!(!is_auth_popup_url("about:blank"));
    }

    #[test]
    fn secure_dns_local_state_sets_cloudflare_prefs() {
        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        let mut local_state = serde_json::json!({"existing": true});

        apply_secure_dns_local_state(&mut local_state, &settings);

        assert_eq!(local_state["existing"], true);
        assert_eq!(local_state["dns_over_https"]["mode"], "secure");
        assert_eq!(
            local_state["dns_over_https"]["templates"],
            "https://cloudflare-dns.com/dns-query"
        );
        assert_eq!(
            local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
            false
        );
        assert_eq!(local_state["async_dns"]["enabled"], true);
    }

    #[test]
    fn secure_dns_local_state_turns_off_stale_prefs() {
        let settings = config::AppSettings::default();
        let mut local_state = serde_json::json!({
            "dns_over_https": {
                "mode": "secure",
                "templates": "https://cloudflare-dns.com/dns-query",
                "automatic_mode_fallback_to_doh": true
            },
            "async_dns": {
                "enabled": true
            }
        });

        apply_secure_dns_local_state(&mut local_state, &settings);

        assert_eq!(local_state["dns_over_https"]["mode"], "off");
        assert_eq!(local_state["dns_over_https"]["templates"], "");
        assert_eq!(
            local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
            false
        );
        assert_eq!(local_state["async_dns"]["enabled"], false);
    }

    #[test]
    fn secure_dns_local_state_covers_all_providers_and_modes() {
        for (provider, endpoint) in secure_dns_cases() {
            for (mode, mode_arg, fallback) in [
                (config::SecureDnsMode::Secure, "secure", false),
                (config::SecureDnsMode::Automatic, "automatic", true),
            ] {
                let mut settings = config::AppSettings::default();
                settings.privacy.secure_dns_enabled = true;
                settings.privacy.secure_dns_provider = provider.clone();
                settings.privacy.secure_dns_mode = mode;
                let mut local_state = serde_json::json!({"keep_me": {"nested": true}});

                apply_secure_dns_local_state(&mut local_state, &settings);

                assert_eq!(local_state["keep_me"]["nested"], true);
                assert_eq!(local_state["dns_over_https"]["mode"], mode_arg);
                assert_eq!(local_state["dns_over_https"]["templates"], endpoint);
                assert_eq!(
                    local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
                    fallback
                );
                assert_eq!(local_state["async_dns"]["enabled"], true);
            }
        }
    }

    #[test]
    fn write_webview_secure_dns_prefs_preserves_existing_local_state() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!(
                "secure-dns-local-state-test-{}",
                std::process::id()
            ));
        let profile_dir = root.join("EBWebView");
        let local_state_path = profile_dir.join("Local State");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(
            &local_state_path,
            r#"{"unrelated":{"value":7},"dns_over_https":{"mode":"off"}}"#,
        )
        .unwrap();

        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;
        settings.privacy.secure_dns_provider = config::SecureDnsProvider::Google;
        settings.privacy.secure_dns_mode = config::SecureDnsMode::Automatic;

        write_webview_secure_dns_prefs(&root, &settings).unwrap();

        let local_state = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&local_state_path).unwrap(),
        )
        .unwrap();
        assert_eq!(local_state["unrelated"]["value"], 7);
        assert_eq!(local_state["dns_over_https"]["mode"], "automatic");
        assert_eq!(
            local_state["dns_over_https"]["templates"],
            config::GOOGLE_DOH
        );
        assert_eq!(
            local_state["dns_over_https"]["automatic_mode_fallback_to_doh"],
            true
        );
        assert_eq!(local_state["async_dns"]["enabled"], true);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn local_state_test_root(name: &str) -> std::path::PathBuf {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("EBWebView")).unwrap();
        root
    }

    #[test]
    fn corrupt_local_state_is_never_overwritten() {
        let root = local_state_test_root("local-state-corrupt");
        let path = root.join("EBWebView").join("Local State");
        let corrupt = r#"{"os_crypt":{"encrypted_key":"RFBBUEl"#;
        std::fs::write(&path, corrupt).unwrap();

        let mut settings = config::AppSettings::default();
        settings.privacy.secure_dns_enabled = true;

        assert!(write_webview_secure_dns_prefs(&root, &settings).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), corrupt);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_local_state_is_never_overwritten() {
        let root = local_state_test_root("local-state-empty");
        let path = root.join("EBWebView").join("Local State");
        std::fs::write(&path, b"").unwrap();

        let settings = config::AppSettings::default();
        assert!(write_webview_secure_dns_prefs(&root, &settings).is_err());
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn local_state_write_keeps_os_crypt_and_skips_when_unchanged() {
        let root = local_state_test_root("local-state-os-crypt");
        let path = root.join("EBWebView").join("Local State");
        std::fs::write(
            &path,
            r#"{"os_crypt":{"encrypted_key":"c2VjcmV0"},"variations_seed":"x"}"#,
        )
        .unwrap();

        let settings = config::AppSettings::default();
        write_webview_secure_dns_prefs(&root, &settings).unwrap();

        let state = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&path).unwrap(),
        )
        .unwrap();
        assert_eq!(state["os_crypt"]["encrypted_key"], "c2VjcmV0");
        assert_eq!(state["variations_seed"], "x");
        assert_eq!(state["dns_over_https"]["mode"], "off");

        let before = std::fs::metadata(&path).unwrap().modified().unwrap();
        write_webview_secure_dns_prefs(&root, &settings).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), before);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn undecryptable_cookie_backups_are_dropped_not_blanked() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("cookie-store-decrypt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let store = storage::cookie_store::open(&dir).unwrap();
        storage::cookie_store::save(
            &store,
            &[storage::cookie_store::CookieRecord {
                name: "SID".into(),
                value: "real-session".into(),
                domain: ".example.com".into(),
                path: "/".into(),
                expires: -1.0,
                is_secure: true,
                is_http_only: true,
                same_site: "Lax".into(),
            }],
        )
        .unwrap();
        assert_eq!(storage::cookie_store::load_all(&store).unwrap().len(), 1);
        drop(store);

        std::fs::remove_file(dir.join("Local State.json")).unwrap();
        let rekeyed = storage::cookie_store::open(&dir).unwrap();
        let loaded = storage::cookie_store::load_all(&rekeyed).unwrap();
        assert!(loaded.is_empty());
        drop(rekeyed);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn webview_profile_lock_paths_include_real_webview2_lockfile() {
        let root = std::path::PathBuf::from(r"C:\VentusProfile");
        let paths = webview_profile_lock_paths(&root);
        assert!(paths.contains(&root.join("EBWebView").join("lockfile")));
        assert!(paths.contains(&root.join("EBWebView").join("LOCK")));
        assert!(paths.contains(&root.join("EBWebView").join("Default").join("LOCK")));
    }

    #[cfg(windows)]
    #[test]
    fn webview_profile_lock_released_detects_exclusive_lock() {
        use std::os::windows::fs::OpenOptionsExt;

        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("webview-profile-lock-test-{}", std::process::id()));
        let lock_dir = root.join("EBWebView").join("Default");
        let lock_path = lock_dir.join("LOCK");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&lock_dir).unwrap();
        std::fs::write(&lock_path, b"").unwrap();

        assert!(webview_profile_lock_released(&root));

        let guard = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&lock_path)
            .unwrap();

        assert!(!webview_profile_lock_released(&root));

        drop(guard);
        assert!(webview_profile_lock_released(&root));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn wait_for_previous_instance_skips_pid_when_profile_is_free() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join(format!("startup-wait-test-{}", std::process::id()));
        let profile = root.join("webview_data");
        let sentinel = root.join("running.lock");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(&sentinel, std::process::id().to_string()).unwrap();

        let started = Instant::now();
        wait_for_previous_instance(&sentinel, &[profile.as_path()]);

        assert!(started.elapsed() < Duration::from_millis(500));
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn maps_tab_shortcuts() {
        assert_eq!(msg_shortcut(0x54, MOD_CTRL, false), SC_SPOTLIGHT);
        assert_eq!(
            msg_shortcut(0x54, MOD_CTRL | MOD_SHIFT, false),
            SC_REOPEN_TAB
        );
        assert_eq!(msg_shortcut(0x09, MOD_CTRL, false), SC_NEXT_TAB);
        assert_eq!(msg_shortcut(0x09, MOD_CTRL | MOD_SHIFT, false), SC_PREV_TAB);
        assert_eq!(msg_shortcut(0x57, MOD_CTRL, false), SC_CLOSE_TAB);
        assert_eq!(msg_shortcut(0x46, MOD_CTRL, false), SC_FIND);
    }

    #[test]
    fn maps_nav_and_window_shortcuts() {
        assert_eq!(msg_shortcut(0x25, MOD_ALT, false), SC_NONE);
        assert_eq!(msg_shortcut(0x27, MOD_ALT, false), SC_NONE);
        assert_eq!(msg_shortcut(0x74, 0, false), SC_RELOAD);
        assert_eq!(msg_shortcut(0x7a, 0, false), SC_FULLSCREEN);
    }

    #[test]
    fn ignores_repeats_and_plain_letters() {
        assert_eq!(msg_shortcut(0x54, MOD_CTRL, true), SC_NONE);
        assert_eq!(msg_shortcut(0x54, 0, false), SC_NONE);
    }
}

#[cfg(test)]
mod currency_tests {
    use super::*;

    #[test]
    fn parses_code_amount() {
        let q = detect_currency_query("100 usd to idr").unwrap();
        assert_eq!(q.from, "USD");
        assert_eq!(q.to, "IDR");
        assert_eq!(q.amount, 100.0);
    }

    #[test]
    fn parses_names() {
        let q = detect_currency_query("jpy to yuan").unwrap();
        assert_eq!(q.from, "JPY");
        assert_eq!(q.to, "CNY");
        assert_eq!(q.amount, 1.0);
    }

    #[test]
    fn parses_symbol_amount() {
        let q = detect_currency_query("$10 to rupiah").unwrap();
        assert_eq!(q.from, "USD");
        assert_eq!(q.to, "IDR");
        assert_eq!(q.amount, 10.0);
    }

    #[test]
    fn parses_indonesian_separator() {
        let q = detect_currency_query("25000 yen ke rupiah").unwrap();
        assert_eq!(q.from, "JPY");
        assert_eq!(q.to, "IDR");
        assert_eq!(q.amount, 25000.0);
    }
}

fn detect_market_query(q: &str) -> Option<(String, String)> {
    let q = q.to_lowercase();
    if q.contains("ihsg") || q.contains("jkse") || q.contains("jakarta composite") {
        return Some(("%5EJKSE".to_string(), "IHSG".to_string()));
    }
    None
}

async fn fetch_market_quote(symbol: &str, name: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent(crate::version::USER_AGENT)
        .build()
        .ok()?;
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1d&interval=1m",
        symbol
    );
    let json: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let result = json["chart"]["result"].as_array()?.first()?;
    let meta = &result["meta"];
    let price = meta["regularMarketPrice"]
        .as_f64()
        .or_else(|| meta["previousClose"].as_f64())?;
    let previous = meta["previousClose"]
        .as_f64()
        .or_else(|| meta["chartPreviousClose"].as_f64());
    let currency = meta["currency"].as_str().unwrap_or("");
    let exchange = meta["exchangeName"].as_str().unwrap_or("Yahoo Finance");
    let updated = meta["regularMarketTime"]
        .as_i64()
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "latest available".to_string());
    let change = previous.map(|prev| {
        let diff = price - prev;
        let pct = if prev != 0.0 {
            diff / prev * 100.0
        } else {
            0.0
        };
        format!(" ({diff:+.2}, {pct:+.2}%)")
    });

    Some(format!(
        "**Live market data:** {name} is **{price:.2} {currency}**{}.\nSource: Yahoo Finance chart API, exchange: {exchange}, updated: {updated}.",
        change.unwrap_or_default()
    ))
}

/// Search Wikipedia and return up to 4 plain-text snippet strings.
/// Uses the MediaWiki Action API — completely free, no key, works from any network.
/// Snippets are HTML-cleaned via `decode_html_text` before being returned.
async fn fetch_wikipedia_search(query: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent(crate::version::USER_AGENT)
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let resp = match client
        .get("https://en.wikipedia.org/w/api.php")
        .query(&[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", query),
            ("format", "json"),
            ("utf8", "1"),
            ("srlimit", "4"),
            ("srprop", "snippet|titlesnippet"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return Vec::new(),
    };

    let arr = match json["query"]["search"].as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .filter_map(|item| {
            let title = item["title"].as_str()?;
            let snippet = item["snippet"].as_str()?;
            let clean = decode_html_text(snippet);
            let clean = clean.trim();
            if clean.is_empty() {
                return None;
            }
            Some(format!("**{}**: {}", title, clean))
        })
        .collect()
}

/// Decode common HTML entities and strip inline tags from a snippet string.
/// Handles &amp; &lt; &gt; &quot; &#39; and removes <b>/<em>/<span> tags.
fn decode_html_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Skip until closing '>'
            for c in chars.by_ref() {
                if c == '>' {
                    break;
                }
            }
        } else if ch == '&' {
            // Collect entity up to ';'
            let mut entity = String::new();
            for c in chars.by_ref() {
                if c == ';' {
                    break;
                }
                entity.push(c);
            }
            match entity.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "#39" | "apos" => out.push('\''),
                "#160" | "nbsp" => out.push(' '),
                _ => {
                    // Unknown entity — emit as-is
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
}

/// Fetch live data from the DuckDuckGo Instant Answer API (no key required).
/// Returns a formatted context string when a direct answer or abstract is available,
/// covering currency conversions, calculations, unit conversions, and factual queries.
async fn fetch_duckduckgo_instant(query: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent(crate::version::USER_AGENT)
        .build()
        .ok()?;

    let resp = client
        .get("https://api.duckduckgo.com/")
        .query(&[
            ("q", query),
            ("format", "json"),
            ("no_html", "1"),
            ("skip_disambig", "1"),
        ])
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;

    let mut parts: Vec<String> = Vec::new();

    // Direct answer — covers currency conversions, calculations, unit conversions, etc.
    if let Some(answer) = json["Answer"].as_str() {
        let a = answer.trim();
        if !a.is_empty() {
            parts.push(format!("**Live answer:** {}", a));
        }
    }

    // Wikipedia / knowledge-base abstract for factual queries
    if let Some(text) = json["AbstractText"].as_str() {
        let t = text.trim();
        if !t.is_empty() {
            let snippet = if t.len() > 500 { &t[..500] } else { t };
            let source = json["AbstractSource"].as_str().unwrap_or("reference");
            parts.push(format!("**From {}:** {}", source, snippet));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}
