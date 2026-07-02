pub fn inject_theme_script(theme: &str) -> String {
    format!(
        r#"document.documentElement.setAttribute('data-theme', '{}');"#,
        theme
    )
}
