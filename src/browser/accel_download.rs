use crate::ui::events::AppEvent;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::event_loop::EventLoopProxy;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

const MAX_SEGMENTS: u64 = 8;
const MIN_PARALLEL_BYTES: u64 = 4 * 1024 * 1024;
const MIN_SEGMENT_BYTES: u64 = 1024 * 1024;
const MAX_SEGMENT_RETRIES: u32 = 6;
const PROGRESS_EVERY: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub struct AccelControl {
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
}

impl AccelControl {
    pub fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct ProbeResult {
    pub final_url: String,
    pub total: u64,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

pub async fn probe(url: &str, ua: &str, referer: &str) -> Option<ProbeResult> {
    let c = client();
    let mut req = c
        .get(url)
        .timeout(Duration::from_secs(10))
        .header("User-Agent", ua)
        .header("Range", "bytes=0-0");
    if !referer.is_empty() {
        req = req.header("Referer", referer);
    }
    let resp = req.send().await.ok()?;
    if resp.status().as_u16() != 206 {
        return None;
    }
    let final_url = resp.url().to_string();
    let range = resp.headers().get(reqwest::header::CONTENT_RANGE)?;
    let total = range.to_str().ok()?.rsplit('/').next()?.trim();
    let total: u64 = total.parse().ok()?;
    if total < MIN_PARALLEL_BYTES {
        return None;
    }
    Some(ProbeResult { final_url, total })
}

fn segment_count(total: u64) -> u64 {
    let by_size = total / MIN_SEGMENT_BYTES;
    by_size.clamp(1, MAX_SEGMENTS)
}

pub async fn run(
    final_url: String,
    total: u64,
    ua: String,
    referer: String,
    path: PathBuf,
    id: String,
    ctl: AccelControl,
    proxy: EventLoopProxy<AppEvent>,
) {
    let file = match tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)
        .await
    {
        Ok(f) => f,
        Err(_) => {
            finish(&proxy, &id, false, false);
            return;
        }
    };
    if file.set_len(total).await.is_err() {
        finish(&proxy, &id, false, false);
        return;
    }
    drop(file);

    let received = Arc::new(AtomicU64::new(0));
    let n = segment_count(total);
    let seg = total / n;
    let c = client();
    let mut tasks = Vec::new();
    for i in 0..n {
        let start = i * seg;
        let end = if i == n - 1 {
            total - 1
        } else {
            (i + 1) * seg - 1
        };
        let task = segment(
            c.clone(),
            final_url.clone(),
            ua.clone(),
            referer.clone(),
            path.clone(),
            start,
            end,
            Arc::clone(&received),
            ctl.clone(),
        );
        tasks.push(tokio::spawn(task));
    }

    let ticker = tokio::spawn(report_progress(
        proxy.clone(),
        id.clone(),
        total,
        Arc::clone(&received),
        ctl.clone(),
    ));

    let mut ok = true;
    for t in tasks {
        match t.await {
            Ok(true) => {}
            _ => ok = false,
        }
    }
    ticker.abort();

    let canceled = ctl.cancel.load(Ordering::Relaxed);
    if !ok || canceled {
        let _ = tokio::fs::remove_file(&path).await;
        finish(&proxy, &id, false, canceled);
        return;
    }
    let _ = proxy.send_event(AppEvent::DownloadProgress {
        id: id.clone(),
        received: total,
        total: Some(total),
    });
    finish(&proxy, &id, true, false);
}

async fn segment(
    c: reqwest::Client,
    url: String,
    ua: String,
    referer: String,
    path: PathBuf,
    start: u64,
    end: u64,
    received: Arc<AtomicU64>,
    ctl: AccelControl,
) -> bool {
    use futures_util::StreamExt;

    let mut pos = start;
    let mut tries = 0u32;
    loop {
        if ctl.cancel.load(Ordering::Relaxed) {
            return false;
        }
        let attempt_start = pos;
        let mut file = match tokio::fs::OpenOptions::new().write(true).open(&path).await {
            Ok(f) => f,
            Err(_) => return false,
        };
        if file.seek(std::io::SeekFrom::Start(pos)).await.is_err() {
            return false;
        }
        let mut req = c
            .get(&url)
            .header("User-Agent", &ua)
            .header("Range", format!("bytes={}-{}", pos, end));
        if !referer.is_empty() {
            req = req.header("Referer", &referer);
        }
        let resp = match req.send().await {
            Ok(r) if r.status().as_u16() == 206 => r,
            _ => {
                tries += 1;
                if tries > MAX_SEGMENT_RETRIES {
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(500 * tries as u64)).await;
                continue;
            }
        };
        let mut stream = resp.bytes_stream();
        loop {
            if ctl.cancel.load(Ordering::Relaxed) {
                return false;
            }
            while ctl.paused.load(Ordering::Relaxed) && !ctl.cancel.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
            let next = tokio::time::timeout(Duration::from_secs(30), stream.next()).await;
            let bytes = match next {
                Ok(Some(Ok(b))) => b,
                Ok(Some(Err(_))) => break,
                Ok(None) => break,
                Err(_) => break,
            };
            let want = (end + 1 - pos) as usize;
            let slice = if bytes.len() > want {
                &bytes[..want]
            } else {
                &bytes[..]
            };
            if file.write_all(slice).await.is_err() {
                return false;
            }
            pos += slice.len() as u64;
            received.fetch_add(slice.len() as u64, Ordering::Relaxed);
            if pos > end {
                break;
            }
        }
        let _ = file.flush().await;
        if pos > end {
            return true;
        }
        if pos > attempt_start {
            tries = 0;
            continue;
        }
        tries += 1;
        if tries > MAX_SEGMENT_RETRIES {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(500 * tries as u64)).await;
    }
}

async fn report_progress(
    proxy: EventLoopProxy<AppEvent>,
    id: String,
    total: u64,
    received: Arc<AtomicU64>,
    ctl: AccelControl,
) {
    let mut last = Instant::now();
    loop {
        tokio::time::sleep(PROGRESS_EVERY).await;
        if ctl.cancel.load(Ordering::Relaxed) {
            return;
        }
        if ctl.paused.load(Ordering::Relaxed) {
            continue;
        }
        if last.elapsed() < PROGRESS_EVERY {
            continue;
        }
        last = Instant::now();
        let _ = proxy.send_event(AppEvent::DownloadProgress {
            id: id.clone(),
            received: received.load(Ordering::Relaxed),
            total: Some(total),
        });
    }
}

fn finish(proxy: &EventLoopProxy<AppEvent>, id: &str, success: bool, canceled: bool) {
    let _ = proxy.send_event(AppEvent::DownloadDone {
        id: id.to_string(),
        success,
        canceled,
    });
}
