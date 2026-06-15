use std::sync::Mutex;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() {
    // NOTE: the crate is `ventus`, so its log target is `ventus::*`. The old default only
    // enabled `neura_browser`, which silently suppressed every one of our own logs.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ventus=debug,neura_browser=debug,wry=warn,tao=warn"));

    // Mirror logs to a file in the data dir so they can be inspected even in release
    // builds (which have no console because of `windows_subsystem = "windows"`).
    // Path: %APPDATA%\com\neura\NeuraBrowser\data\ventus.log (truncated each run).
    let file_layer = open_log_file().map(|file| {
        fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(Mutex::new(file))
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).compact())
        .with(file_layer)
        .with(crate::utils::log_buffer::BufferLayer)
        .init();

    if let Some(path) = log_file_path() {
        tracing::info!("log file: {}", path.display());
    }
}

fn log_file_path() -> Option<std::path::PathBuf> {
    Some(crate::utils::platform::data_dir().join("ventus.log"))
}

fn open_log_file() -> Option<std::fs::File> {
    let path = log_file_path()?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .ok()
}
