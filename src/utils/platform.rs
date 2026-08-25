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

pub fn read_json(path: &std::path::Path) -> anyhow::Result<Option<serde_json::Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let value: serde_json::Value = serde_json::from_str(&text)?;
    if !value.is_object() {
        anyhow::bail!("{} is not a JSON object", path.display());
    }
    Ok(Some(value))
}

pub fn write_json_atomic(path: &std::path::Path, value: &serde_json::Value) -> anyhow::Result<()> {
    use std::io::Write;

    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        anyhow::bail!("{} has no file name", path.display());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_file_name(format!("{name}.ventus-tmp"));
    let mut file = std::fs::File::create(&tmp)?;
    let write = file
        .write_all(&serde_json::to_vec(value)?)
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(err) = write {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.into());
    }
    if let Err(err) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.into());
    }
    Ok(())
}
