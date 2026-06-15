use anyhow::{anyhow, Result};
use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

use super::config;
use crate::storage::settings_store;
use crate::utils::log_buffer::LogEntry;

const DEVICE_ID_KEY: &str = "report_device_id";
pub const MAX_LOGS: usize = 400;
const MAX_MESSAGE: usize = 5000;
const MAX_CONTEXT: usize = 16000;

pub struct Report {
    pub kind: String,
    pub message: String,
    pub uid: String,
    pub email: String,
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub device_id: String,
    pub session_id: String,
    pub panic: String,
    pub context: String,
    pub logs: Vec<LogEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct CrashRecord {
    pub session_id: String,
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub ts: i64,
    pub panic: String,
    pub logs: Vec<LogEntry>,
}

pub fn get_or_create_device_id(conn: &Connection) -> String {
    if let Ok(Some(id)) = settings_store::get::<String>(conn, DEVICE_ID_KEY) {
        if !id.is_empty() {
            return id;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _ = settings_store::set(conn, DEVICE_ID_KEY, &id);
    id
}

pub fn write_crash(path: &Path, record: &CrashRecord) {
    if let Ok(json) = serde_json::to_string(record) {
        let _ = std::fs::write(path, json);
    }
}

pub fn take_crash(path: &Path) -> Option<CrashRecord> {
    let text = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    serde_json::from_str(&text).ok()
}

fn s(v: &str) -> Value {
    json!({ "stringValue": v })
}

fn logs_value(logs: &[LogEntry]) -> Value {
    let start = logs.len().saturating_sub(MAX_LOGS);
    let items: Vec<Value> = logs[start..]
        .iter()
        .map(|e| {
            json!({ "mapValue": { "fields": {
                "ts": { "integerValue": e.ts.to_string() },
                "level": s(&e.level),
                "target": s(&e.target),
                "message": s(&e.message),
            }}})
        })
        .collect();
    json!({ "arrayValue": { "values": items } })
}

pub async fn send(report: Report) -> Result<()> {
    if !config::is_configured() {
        return Err(anyhow!("cloud not configured"));
    }
    let mut message = report.message;
    trim(&mut message, MAX_MESSAGE);
    let mut context = report.context;
    trim(&mut context, MAX_CONTEXT);
    let url = format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/reports?key={}",
        config::FIREBASE_PROJECT_ID,
        config::FIREBASE_API_KEY,
    );
    let body = json!({ "fields": {
        "kind": s(&report.kind),
        "message": s(&message),
        "uid": s(&report.uid),
        "email": s(&report.email),
        "anonymous": { "booleanValue": report.uid.is_empty() },
        "appVersion": s(&report.app_version),
        "os": s(&report.os),
        "arch": s(&report.arch),
        "deviceId": s(&report.device_id),
        "sessionId": s(&report.session_id),
        "panic": s(&report.panic),
        "context": s(&context),
        "createdAt": { "timestampValue": chrono::Utc::now().to_rfc3339() },
        "logs": logs_value(&report.logs),
    }});
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let resp = client.post(url).json(&body).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let data: Value = resp.json().await.unwrap_or_default();
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("report upload failed");
        return Err(anyhow!(msg.to_string()));
    }
    Ok(())
}

pub async fn send_crash(record: CrashRecord, uid: String, email: String, device_id: String) {
    let report = Report {
        kind: "crash".to_string(),
        message: "Automatic crash report".to_string(),
        uid,
        email,
        app_version: record.app_version,
        os: record.os,
        arch: record.arch,
        device_id,
        session_id: record.session_id,
        panic: record.panic,
        context: serde_json::json!({
            "crash_ts": record.ts,
        })
        .to_string(),
        logs: record.logs,
    };
    let _ = send(report).await;
}

fn trim(text: &mut String, max: usize) {
    if text.len() <= max {
        return;
    }
    let mut n = max.min(text.len());
    while n > 0 && !text.is_char_boundary(n) {
        n -= 1;
    }
    text.truncate(n);
}
