pub fn os_name() -> &'static str {
    std::env::consts::OS
}

pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

pub fn data_dir() -> std::path::PathBuf {
    use directories::ProjectDirs;
    ProjectDirs::from("com", "neura", "NeuraBrowser")
        .map(|p| p.data_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from(".neura-data"))
}

pub fn ensure_data_dir() -> anyhow::Result<std::path::PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
