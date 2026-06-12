use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Complete,
    Failed,
    Cancelled,
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
