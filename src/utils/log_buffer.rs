use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
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
static ERROR_PENDING: AtomicBool = AtomicBool::new(false);

fn buffer() -> &'static Mutex<VecDeque<LogEntry>> {
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAP)))
}

pub fn snapshot(max: usize) -> Vec<LogEntry> {
    let buf = buffer().lock().unwrap_or_else(|e| e.into_inner());
    let start = buf.len().saturating_sub(max);
    buf.iter().skip(start).cloned().collect()
}

pub fn take_error_pending() -> bool {
    ERROR_PENDING.swap(false, Ordering::Relaxed)
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
            ERROR_PENDING.store(true, Ordering::Relaxed);
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
