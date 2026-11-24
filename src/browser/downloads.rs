use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Where a download lives until it is complete. The neutral extension also keeps Chromium from
/// holding .exe and .jar files for its own hidden "Keep" prompt; Ventus asks instead.
pub fn part_path(target: &Path, id: &str) -> PathBuf {
    let short: String = id.chars().take(8).collect();
    target.with_file_name(format!("Unconfirmed {short}.part"))
}

/// File types Windows runs when they are opened, so the user has to keep them on purpose.
pub fn is_risky(target: &Path) -> bool {
    const RISKY: &[&str] = &[
        "appinstaller",
        "application",
        "appref-ms",
        "appx",
        "appxbundle",
        "bat",
        "chm",
        "cmd",
        "com",
        "cpl",
        "dll",
        "exe",
        "hlp",
        "hta",
        "inf",
        "jar",
        "jnlp",
        "js",
        "jse",
        "lnk",
        "msc",
        "msi",
        "msix",
        "msixbundle",
        "msp",
        "pif",
        "ps1",
        "psm1",
        "reg",
        "scf",
        "scr",
        "settingcontent-ms",
        "sys",
        "vbe",
        "vbs",
        "wsf",
        "wsh",
        "xll",
    ];
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    // Windows drops trailing dots and spaces, so "a.exe." still runs.
    name.trim_end_matches(['.', ' '])
        .rsplit_once('.')
        .is_some_and(|(_, ext)| RISKY.contains(&ext))
}

pub enum Settled {
    Saved(PathBuf),
    /// Still a part file until the user keeps or discards it.
    Review,
}

pub fn settle(part: &Path, target: &Path, overwrite: bool) -> std::io::Result<Settled> {
    if is_risky(target) {
        return std::fs::metadata(part).map(|_| Settled::Review);
    }
    finish_part(part, target, overwrite).map(Settled::Saved)
}

pub fn unique_path(candidate: PathBuf) -> PathBuf {
    if !candidate.exists() {
        return candidate;
    }
    let stem = candidate
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".into());
    let ext = candidate
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (1..1000)
        .map(|i| candidate.with_file_name(format!("{stem} ({i}){ext}")))
        .find(|path| !path.exists())
        .unwrap_or(candidate)
}

/// Moves a finished download to its real name. Antivirus can hold a new file for a moment, so a
/// locked file is retried. Blocking.
pub fn finish_part(part: &Path, target: &Path, overwrite: bool) -> std::io::Result<PathBuf> {
    let mut tries = 0;
    loop {
        let dest = if overwrite {
            target.to_path_buf()
        } else {
            unique_path(target.to_path_buf())
        };
        match std::fs::rename(part, &dest) {
            Ok(()) => return Ok(dest),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound || tries >= 40 => return Err(e),
            Err(_) => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Complete,
    Failed,
    Cancelled,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub local_path: Option<String>,
    pub mime_type: Option<String>,
    pub total_bytes: Option<u64>,
    pub received_bytes: u64,
    pub status: DownloadStatus,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

impl Download {
    pub fn new(url: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url: url.into(),
            filename: filename.into(),
            local_path: None,
            mime_type: None,
            total_bytes: None,
            received_bytes: 0,
            status: DownloadStatus::Pending,
            started_at: chrono::Utc::now().timestamp_millis(),
            completed_at: None,
        }
    }

    /// Deletes a finished file the user never kept.
    pub fn drop_unkept(&self) {
        if let (DownloadStatus::Review, Some(path)) = (&self.status, &self.local_path) {
            let _ = std::fs::remove_file(part_path(Path::new(path), &self.id));
        }
    }

    pub fn progress_percent(&self) -> Option<f32> {
        self.total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (self.received_bytes as f32 / total as f32) * 100.0
            }
        })
    }
}

pub struct DownloadManager {
    pub downloads: Vec<Download>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self { downloads: vec![] }
    }

    pub fn with_downloads(downloads: Vec<Download>) -> Self {
        Self { downloads }
    }

    pub fn add(&mut self, download: Download) -> &Download {
        self.downloads.push(download);
        self.downloads.last().unwrap()
    }

    pub fn update_progress(&mut self, id: &str, received: u64, total: Option<u64>) {
        if let Some(d) = self.downloads.iter_mut().find(|d| d.id == id) {
            d.received_bytes = received;
            if total.is_some() {
                d.total_bytes = total;
            }
            d.status = DownloadStatus::Downloading;
        }
    }

    pub fn complete(&mut self, id: &str, path: &str) {
        if let Some(d) = self.downloads.iter_mut().find(|d| d.id == id) {
            d.status = DownloadStatus::Complete;
            d.local_path = Some(path.to_string());
            d.completed_at = Some(chrono::Utc::now().timestamp_millis());
        }
    }

    pub fn fail(&mut self, id: &str) {
        if let Some(d) = self.downloads.iter_mut().find(|d| d.id == id) {
            d.status = DownloadStatus::Failed;
        }
    }

    pub fn pause(&mut self, id: &str) {
        if let Some(d) = self.downloads.iter_mut().find(|d| d.id == id) {
            d.status = DownloadStatus::Paused;
        }
    }

    pub fn cancel(&mut self, id: &str) {
        if let Some(d) = self.downloads.iter_mut().find(|d| d.id == id) {
            d.status = DownloadStatus::Cancelled;
            d.completed_at = Some(chrono::Utc::now().timestamp_millis());
        }
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Download> {
        self.downloads.iter_mut().find(|d| d.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risky_files_wait_for_the_user() {
        for name in ["a.exe", "A.JAR", "x.tar.Msi", "a.exe.", "a.bat  "] {
            assert!(is_risky(Path::new(name)), "{name}");
        }
        for name in ["a.zip", "movie.mkv", "exe", "a.exe.zip", "a.pdf"] {
            assert!(!is_risky(Path::new(name)), "{name}");
        }
        let dir = std::env::temp_dir().join(format!("ventus-risky-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("mod.jar");
        let mut dl = Download::new("https://a.test/mod.jar", "mod.jar");
        dl.local_path = Some(target.to_string_lossy().to_string());
        let part = part_path(&target, &dl.id);
        std::fs::write(&part, "x").unwrap();
        assert!(matches!(settle(&part, &target, false), Ok(Settled::Review)));
        assert!(part.exists() && !target.exists());
        dl.drop_unkept();
        assert!(part.exists());
        dl.status = DownloadStatus::Review;
        dl.drop_unkept();
        assert!(!part.exists());
        assert!(settle(&part, &target, false).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn finished_parts_never_replace_another_file() {
        let dir = std::env::temp_dir().join(format!("ventus-part-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("mod.jar");
        for (id, body) in [("aaaaaaaa-1", "one"), ("bbbbbbbb-2", "two")] {
            let part = part_path(&target, id);
            std::fs::write(&part, body).unwrap();
            finish_part(&part, &target, false).unwrap();
            assert!(!part.exists());
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"one");
        assert_eq!(std::fs::read(dir.join("mod (1).jar")).unwrap(), b"two");
        let part = part_path(&target, "cccccccc-3");
        std::fs::write(&part, "three").unwrap();
        assert_eq!(finish_part(&part, &target, true).unwrap(), target);
        assert_eq!(std::fs::read(&target).unwrap(), b"three");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
