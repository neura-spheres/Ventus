use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

const CAP: usize = 900;
const MAX_MSG: usize = 800;

#[derive(Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts: i64,
    pub level: String,
    pub target: String,
    pub message: String,
}

static BUFFER: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();
static AUTO_PENDING: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy)]
pub enum AutoLogKind {
    Error,
    Warning,
}

fn buffer() -> &'static Mutex<VecDeque<LogEntry>> {
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAP)))
}

pub fn snapshot(max: usize) -> Vec<LogEntry> {
    let buf = buffer().lock().unwrap_or_else(|e| e.into_inner());
    let start = buf.len().saturating_sub(max);
    buf.iter().skip(start).cloned().collect()
}

pub fn take_auto_pending() -> Option<AutoLogKind> {
    match AUTO_PENDING.swap(0, Ordering::Relaxed) {
        1 => Some(AutoLogKind::Error),
        2 => Some(AutoLogKind::Warning),
        _ => None,
    }
}

struct MsgVisitor {
    message: String,
    extra: String,
}

impl Visit for MsgVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            return;
        }
        if !self.extra.is_empty() {
            self.extra.push(' ');
        }
        self.extra
            .push_str(&format!("{}={:?}", field.name(), value));
    }
}

pub struct BufferLayer;

impl<S: Subscriber> Layer<S> for BufferLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut v = MsgVisitor {
            message: String::new(),
            extra: String::new(),
        };
        event.record(&mut v);
        let mut message = v.message;
        if !v.extra.is_empty() {
            if !message.is_empty() {
                message.push(' ');
            }
            message.push_str(&v.extra);
        }
        trim(&mut message, MAX_MSG);
        let level = *meta.level();
        if level == Level::ERROR {
            AUTO_PENDING.store(1, Ordering::Relaxed);
        } else if level == Level::WARN && dangerous_warning(meta.target(), &message) {
            let _ = AUTO_PENDING.compare_exchange(0, 2, Ordering::Relaxed, Ordering::Relaxed);
        }
        let entry = LogEntry {
            ts: chrono::Utc::now().timestamp_millis(),
            level: level.to_string(),
            target: meta.target().to_string(),
            message,
        };
        let mut buf = buffer().lock().unwrap_or_else(|e| e.into_inner());
        if buf.len() >= CAP {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::dangerous_warning;

    #[test]
    fn webview2_process_crashes_are_reported() {
        assert!(dangerous_warning(
            "ventus::content",
            "webview2 processfailed kind=4 reason=\"crashed\" process=network service"
        ));
        assert!(dangerous_warning(
            "ventus::content",
            "content process failed"
        ));
    }

    #[test]
    fn ordinary_warnings_stay_quiet() {
        assert!(!dangerous_warning(
            "ventus::perf",
            "main-thread handler slow"
        ));
        assert!(!dangerous_warning("ventus::content", "tab went to sleep"));
    }
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

fn dangerous_warning(target: &str, message: &str) -> bool {
    if target == "ventus::autolog" {
        return true;
    }
    let text = message.to_ascii_lowercase();
    if target == "ventus::content"
        && (text.contains("content process failed") || text.contains("webview2 processfailed"))
    {
        return true;
    }
    if target == "ventus::shutdown" && text.contains("profile still busy") {
        return true;
    }
    if target == "ventus::session"
        && (text.contains("stays black") || text.contains("last session may not restore"))
    {
        return true;
    }
    target == "ventus::startup" && text.contains("first content webview build failed")
}
