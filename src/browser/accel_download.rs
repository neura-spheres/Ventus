use crate::ui::events::AppEvent;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tao::event_loop::EventLoopProxy;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

const MAX_WORKERS: usize = 8;
const MIN_PARALLEL_BYTES: u64 = 4 * 1024 * 1024;
const MIN_PIECE: u64 = 1024 * 1024;
const MAX_PIECE: u64 = 64 * 1024 * 1024;
#[cfg(not(test))]
const READ_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const READ_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(not(test))]
const GIVE_UP_AFTER: Duration = Duration::from_secs(120);
#[cfg(test)]
const GIVE_UP_AFTER: Duration = Duration::from_secs(3);
const PROGRESS_EVERY: Duration = Duration::from_millis(200);
const CONTROL_POLL: Duration = Duration::from_millis(100);

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

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub final_url: String,
    pub total: u64,
    validator: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Done,
    Canceled,
    Failed(String),
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .pool_idle_timeout(Duration::from_secs(30))
        // One TCP connection per worker. Sharing an h2 connection defeats the split.
        .http1_only()
        .build()
        .unwrap_or_default()
}

/// `expected` is the size WebView2 reported. A probe without the page's cookies can land on a
/// login or error page that also supports ranges, so a different size means a different file.
pub async fn probe(url: &str, ua: &str, referer: &str, expected: u64) -> Option<ProbeResult> {
    if expected < MIN_PARALLEL_BYTES {
        return None;
    }
    let mut req = client()
        .get(url)
        .timeout(Duration::from_secs(5))
        .header("User-Agent", ua)
        .header("Accept-Encoding", "identity")
        .header("Range", "bytes=0-0");
    if !referer.is_empty() {
        req = req.header("Referer", referer);
    }
    let resp = req.send().await.ok()?;
    if resp.status().as_u16() != 206 || is_encoded(&resp) {
        return None;
    }
    let (start, _, total) = content_range(&resp)?;
    if start != 0 || total != expected {
        return None;
    }
    let text = |name: reqwest::header::HeaderName| {
        let value = resp.headers().get(name)?.to_str().ok()?;
        Some(value.to_string())
    };
    let validator = text(reqwest::header::ETAG)
        .filter(|tag| !tag.starts_with("W/"))
        .or_else(|| text(reqwest::header::LAST_MODIFIED));
    Some(ProbeResult {
        final_url: resp.url().to_string(),
        total,
        validator,
    })
}

fn is_encoded(resp: &reqwest::Response) -> bool {
    resp.headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .is_some_and(|v| v != "identity")
}

fn content_range(resp: &reqwest::Response) -> Option<(u64, u64, u64)> {
    let value = resp.headers().get(reqwest::header::CONTENT_RANGE)?;
    parse_content_range(value.to_str().ok()?)
}

fn parse_content_range(value: &str) -> Option<(u64, u64, u64)> {
    let (range, total) = value.trim().strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    Some((
        start.trim().parse().ok()?,
        end.trim().parse().ok()?,
        total.trim().parse().ok()?,
    ))
}

/// Pieces are handed out front to back. Writing far ahead in one huge file makes NTFS zero
/// everything before the write first, which froze multi-gigabyte downloads near the start.
fn split(total: u64) -> VecDeque<(u64, u64)> {
    let piece = (total / 64).clamp(MIN_PIECE, MAX_PIECE);
    let mut out = VecDeque::new();
    let mut start = 0;
    while start < total {
        let end = (start + piece).min(total);
        out.push_back((start, end));
        start = end;
    }
    out
}

/// Pieces left to fetch and workers alive. One lock covers both, so a piece can never be handed
/// back after the last worker has left.
struct Queue {
    pieces: VecDeque<(u64, u64)>,
    workers: usize,
}

struct Job {
    client: reqwest::Client,
    url: String,
    probe: ProbeResult,
    ua: String,
    referer: String,
    path: PathBuf,
    queue: Mutex<Queue>,
    received: AtomicU64,
    last_progress: Mutex<Instant>,
    ctl: AccelControl,
}

impl Job {
    fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn last_progress(&self) -> MutexGuard<'_, Instant> {
        self.last_progress.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn touch(&self) {
        *self.last_progress() = Instant::now();
    }

    fn stalled(&self) -> bool {
        self.last_progress().elapsed() >= GIVE_UP_AFTER
    }

    fn paused(&self) -> bool {
        self.ctl.paused.load(Ordering::Relaxed)
    }

    fn next(&self) -> Option<(u64, u64)> {
        let mut queue = self.queue();
        let piece = queue.pieces.pop_front();
        if piece.is_none() {
            queue.workers -= 1;
        }
        piece
    }

    /// Drops this worker and returns its piece, unless it is the last one.
    fn retire(&self, piece: (u64, u64)) -> bool {
        let mut queue = self.queue();
        if queue.workers <= 1 {
            return false;
        }
        queue.workers -= 1;
        queue.pieces.push_front(piece);
        true
    }
}

pub struct Request {
    pub url: String,
    pub probe: ProbeResult,
    pub ua: String,
    pub referer: String,
    pub part: PathBuf,
}

pub async fn run(
    request: Request,
    target: PathBuf,
    overwrite: bool,
    id: String,
    ctl: AccelControl,
    proxy: EventLoopProxy<AppEvent>,
) {
    let part = request.part.clone();
    let source = request.url.clone();
    let progress = {
        let (proxy, id) = (proxy.clone(), id.clone());
        move |received, total| {
            let _ = proxy.send_event(AppEvent::DownloadProgress {
                id: id.clone(),
                received,
                total: Some(total),
            });
        }
    };
    let mut path = None;
    let mut review = None;
    let mut canceled = false;
    let outcome = match fetch(request, ctl, progress).await {
        Outcome::Done => {
            mark_from_web(&part, &source);
            let from = part.clone();
            let moved = tokio::task::spawn_blocking(move || {
                super::downloads::settle(&from, &target, overwrite)
            })
            .await;
            match moved {
                Ok(Ok(super::downloads::Settled::Saved(done))) => {
                    path = Some(done.to_string_lossy().to_string());
                    Ok(())
                }
                Ok(Ok(super::downloads::Settled::Review)) => {
                    review = Some(overwrite);
                    Ok(())
                }
                Ok(Err(e)) => Err(e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
        Outcome::Canceled => {
            canceled = true;
            Err("canceled".to_string())
        }
        Outcome::Failed(reason) => Err(reason),
    };
    if let Err(reason) = &outcome {
        if !canceled {
            tracing::warn!(target: "ventus::dl", id = %id, reason = %reason, "accelerated download failed");
        }
        let _ = tokio::fs::remove_file(&part).await;
    }
    let _ = proxy.send_event(AppEvent::DownloadDone {
        id,
        success: outcome.is_ok(),
        canceled,
        path,
        review,
    });
}

/// Chromium tags its downloads so Windows warns before running them. Files written here need
/// the same tag.
fn mark_from_web(path: &Path, url: &str) {
    #[cfg(windows)]
    {
        let mut stream = path.as_os_str().to_os_string();
        stream.push(":Zone.Identifier");
        let host: String = url.chars().filter(|c| !c.is_control()).collect();
        let _ = std::fs::write(
            stream,
            format!("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={host}\r\n"),
        );
    }
    #[cfg(not(windows))]
    let _ = (path, url);
}

async fn fetch(request: Request, ctl: AccelControl, progress: impl Fn(u64, u64)) -> Outcome {
    let total = request.probe.total;
    let file = match tokio::fs::File::create(&request.part).await {
        Ok(f) => f,
        Err(e) => return Outcome::Failed(e.to_string()),
    };
    // Reserves the space now, so a full disk fails at the start instead of at the end.
    if let Err(e) = file.set_len(total).await {
        return Outcome::Failed(e.to_string());
    }
    drop(file);

    let pieces = split(total);
    let workers = pieces.len().min(MAX_WORKERS);
    let job = Arc::new(Job {
        client: client(),
        url: request.url,
        probe: request.probe,
        ua: request.ua,
        referer: request.referer,
        path: request.part,
        queue: Mutex::new(Queue { pieces, workers }),
        received: AtomicU64::new(0),
        last_progress: Mutex::new(Instant::now()),
        ctl,
    });
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..workers {
        tasks.spawn(worker(Arc::clone(&job)));
    }

    let mut tick = tokio::time::interval(CONTROL_POLL);
    let mut reported = Instant::now();
    loop {
        tokio::select! {
            done = tasks.join_next() => {
                let failure = match done {
                    None => break,
                    Some(Ok(Ok(()))) => continue,
                    Some(Ok(Err(reason))) => reason,
                    Some(Err(e)) => e.to_string(),
                };
                tasks.shutdown().await;
                return Outcome::Failed(failure);
            }
            _ = tick.tick() => {
                // Checked here, not in the workers, so a silent server cannot block Cancel.
                if job.ctl.cancel.load(Ordering::Relaxed) {
                    tasks.shutdown().await;
                    return Outcome::Canceled;
                }
                if job.paused() {
                    job.touch();
                } else if reported.elapsed() >= PROGRESS_EVERY {
                    reported = Instant::now();
                    progress(job.received.load(Ordering::Relaxed), total);
                }
            }
        }
    }
    if job.ctl.cancel.load(Ordering::Relaxed) {
        return Outcome::Canceled;
    }
    let received = job.received.load(Ordering::Relaxed);
    if received != total {
        return Outcome::Failed(format!("incomplete: {received} of {total}"));
    }
    progress(total, total);
    Outcome::Done
}

async fn worker(job: Arc<Job>) -> Result<(), String> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&job.path)
        .await
        .map_err(|e| e.to_string())?;
    let mut fails = 0u32;
    while let Some((mut pos, end)) = job.next() {
        while pos < end {
            if job.paused() {
                tokio::time::sleep(CONTROL_POLL).await;
                continue;
            }
            let before = pos;
            let result = read_piece(&job, &mut file, &mut pos, end, fails).await;
            if pos > before {
                fails = 0;
            }
            let reason = match result {
                Ok(()) => continue,
                Err(Fault::Disk(reason)) => return Err(reason),
                Err(Fault::Refused(reason)) => {
                    // Hosts often cap connections per visitor. Carry on with fewer.
                    if job.retire((pos, end)) {
                        tracing::debug!(target: "ventus::dl", reason = %reason, "download worker retired");
                        return Ok(());
                    }
                    reason
                }
                Err(Fault::Dropped(reason)) => reason,
            };
            if pos >= end {
                break;
            }
            if job.stalled() {
                return Err(reason);
            }
            fails += 1;
            tokio::time::sleep(Duration::from_millis(250 << fails.min(5))).await;
        }
    }
    Ok(())
}

enum Fault {
    Refused(String),
    Dropped(String),
    Disk(String),
}

async fn read_piece(
    job: &Job,
    file: &mut tokio::fs::File,
    pos: &mut u64,
    end: u64,
    fails: u32,
) -> Result<(), Fault> {
    use futures_util::StreamExt;

    let disk = |e: std::io::Error| Fault::Disk(e.to_string());
    // Signed redirect targets expire, so a failing one is refreshed from the original link.
    let url = if fails % 2 == 1 {
        &job.url
    } else {
        &job.probe.final_url
    };
    let mut req = job
        .client
        .get(url)
        .header("User-Agent", &job.ua)
        .header("Accept-Encoding", "identity")
        .header("Range", format!("bytes={}-{}", *pos, end - 1));
    if let Some(validator) = &job.probe.validator {
        req = req.header("If-Range", validator);
    }
    if !job.referer.is_empty() {
        req = req.header("Referer", &job.referer);
    }
    let resp = match tokio::time::timeout(READ_TIMEOUT, req.send()).await {
        Err(_) => return Err(Fault::Refused("server did not answer".into())),
        Ok(Err(e)) => return Err(Fault::Dropped(e.to_string())),
        Ok(Ok(resp)) => resp,
    };
    let status = resp.status().as_u16();
    if status != 206 || is_encoded(&resp) {
        return Err(Fault::Refused(format!("server answered {status}")));
    }
    if !matches!(content_range(&resp), Some((start, _, total)) if start == *pos && total == job.probe.total)
    {
        return Err(Fault::Refused("server returned the wrong range".into()));
    }
    file.seek(std::io::SeekFrom::Start(*pos))
        .await
        .map_err(disk)?;
    let mut stream = resp.bytes_stream();
    let mut dropped = None;
    while *pos < end && !job.paused() {
        match tokio::time::timeout(READ_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(bytes))) => {
                let want = (end - *pos).min(bytes.len() as u64) as usize;
                file.write_all(&bytes[..want]).await.map_err(disk)?;
                *pos += want as u64;
                job.received.fetch_add(want as u64, Ordering::Relaxed);
                job.touch();
            }
            Ok(Some(Err(e))) => dropped = Some(e.to_string()),
            Ok(None) => dropped = Some("connection closed early".into()),
            Err(_) => dropped = Some("connection stalled".into()),
        }
        if dropped.is_some() {
            break;
        }
    }
    // tokio reports a failed write on the next call, so the flush decides whether it is safe.
    file.flush().await.map_err(disk)?;
    match dropped {
        Some(reason) => Err(Fault::Dropped(reason)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const SIZE: usize = 6 * 1024 * 1024 + 123;

    #[derive(Clone, Copy, PartialEq)]
    enum Server {
        Good,
        OneConnection,
        SilentAfterOne,
        DropsMidway,
        IgnoresRange,
        WrongRange,
        Slow,
        Dead,
    }

    fn byte(i: usize) -> u8 {
        (i.wrapping_mul(2_654_435_761) >> 11) as u8
    }

    async fn serve(kind: Server) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file.bin", listener.local_addr().unwrap());
        let live = Arc::new(AtomicUsize::new(0));
        let hits = Arc::new(AtomicUsize::new(0));
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let (live, hits) = (Arc::clone(&live), Arc::clone(&hits));
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let head = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                    let Some(range) = head.split("range: bytes=").nth(1) else {
                        return;
                    };
                    let (a, b) = range.lines().next().unwrap().split_once('-').unwrap();
                    let (mut a, b): (usize, usize) = (a.parse().unwrap(), b.parse().unwrap());
                    let probe = b == 0;
                    let busy = !probe && live.fetch_add(1, Ordering::SeqCst) > 0;
                    let hit = hits.fetch_add(1, Ordering::SeqCst);
                    let refuse = match kind {
                        Server::OneConnection => busy,
                        Server::Dead => !probe,
                        _ => false,
                    };
                    if kind == Server::SilentAfterOne && busy {
                        live.fetch_sub(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        return;
                    } else if refuse {
                        let _ = sock
                            .write_all(b"HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                            .await;
                    } else if kind == Server::IgnoresRange && !probe {
                        let _ = sock
                            .write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {SIZE}\r\nConnection: close\r\n\r\n").as_bytes())
                            .await;
                    } else {
                        let len = b - a + 1;
                        if kind == Server::WrongRange && !probe {
                            a = 7;
                        }
                        let _ = sock
                            .write_all(format!("HTTP/1.1 206 Partial\r\nContent-Range: bytes {a}-{}/{SIZE}\r\nContent-Length: {len}\r\nETag: \"x\"\r\nConnection: close\r\n\r\n", a + len - 1).as_bytes())
                            .await;
                        let cut = kind == Server::DropsMidway && hit % 2 == 1;
                        let body: Vec<u8> =
                            (a..a + if cut { len / 3 } else { len }).map(byte).collect();
                        for chunk in body.chunks(64 * 1024) {
                            if sock.write_all(chunk).await.is_err() {
                                break;
                            }
                            if kind == Server::Slow {
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                    if !probe {
                        live.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }
        });
        url
    }

    async fn request(kind: Server) -> Request {
        let url = serve(kind).await;
        let probe = probe(&url, "ua", "", SIZE as u64).await.expect("probe");
        let part = std::env::temp_dir().join(format!("ventus-dl-{}.part", uuid::Uuid::new_v4()));
        Request {
            url,
            probe,
            ua: "ua".into(),
            referer: String::new(),
            part,
        }
    }

    async fn download(kind: Server) -> Outcome {
        let request = request(kind).await;
        let part = request.part.clone();
        let outcome = fetch(request, AccelControl::new(), |_, _| {}).await;
        if outcome == Outcome::Done {
            let data = std::fs::read(&part).unwrap();
            assert_eq!(data.len(), SIZE);
            assert!(data.iter().enumerate().all(|(i, b)| *b == byte(i)));
        }
        let _ = std::fs::remove_file(&part);
        outcome
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn normal_server_is_byte_exact() {
        assert_eq!(download(Server::Good).await, Outcome::Done);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn one_connection_host_finishes_with_fewer_workers() {
        assert_eq!(download(Server::OneConnection).await, Outcome::Done);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn silent_connections_do_not_hang_the_download() {
        assert_eq!(download(Server::SilentAfterOne).await, Outcome::Done);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dropped_connections_resume_where_they_stopped() {
        assert_eq!(download(Server::DropsMidway).await, Outcome::Done);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bad_ranges_fail_instead_of_corrupting() {
        for kind in [Server::IgnoresRange, Server::WrongRange, Server::Dead] {
            assert!(matches!(download(kind).await, Outcome::Failed(_)));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancel_works_while_the_server_is_silent() {
        let request = request(Server::SilentAfterOne).await;
        let part = request.part.clone();
        let ctl = AccelControl::new();
        let cancel = Arc::clone(&ctl.cancel);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            cancel.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        let outcome = fetch(request, ctl, |_, _| {}).await;
        assert!(outcome == Outcome::Canceled || outcome == Outcome::Done);
        assert!(started.elapsed() < Duration::from_secs(1));
        let _ = std::fs::remove_file(part);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pause_stops_bytes_and_resume_finishes() {
        let request = request(Server::Slow).await;
        let part = request.part.clone();
        let ctl = AccelControl::new();
        let seen = Arc::new(AtomicU64::new(0));
        let (seen_cb, paused) = (Arc::clone(&seen), Arc::clone(&ctl.paused));
        let task = tokio::spawn(fetch(request, ctl, move |n, _| {
            seen_cb.store(n, Ordering::Relaxed)
        }));
        tokio::time::sleep(Duration::from_millis(600)).await;
        paused.store(true, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let at_pause = seen.load(Ordering::Relaxed);
        // Longer than GIVE_UP_AFTER: waiting on the user is not a stall.
        tokio::time::sleep(Duration::from_millis(3500)).await;
        assert_eq!(seen.load(Ordering::Relaxed), at_pause);
        paused.store(false, Ordering::Relaxed);
        assert_eq!(task.await.unwrap(), Outcome::Done);
        let data = std::fs::read(&part).unwrap();
        assert!(data.iter().enumerate().all(|(i, b)| *b == byte(i)));
        let _ = std::fs::remove_file(part);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_rejects_a_different_file() {
        let url = serve(Server::Good).await;
        assert!(probe(&url, "ua", "", SIZE as u64 + 1).await.is_none());
    }

    #[test]
    fn pieces_cover_the_file_once_in_order() {
        for total in [1, MIN_PIECE, SIZE as u64, 30 << 30] {
            let pieces = split(total);
            let mut next = 0;
            for (start, end) in &pieces {
                assert_eq!(*start, next);
                assert!(end > start);
                next = *end;
            }
            assert_eq!(next, total);
        }
    }
}
