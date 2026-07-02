use anyhow::Result;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

const DONE_PAGE: &str = r#"<!doctype html><html><head><meta charset="utf-8"><title>Ventus</title>
<style>html,body{height:100%;margin:0}body{display:flex;align-items:center;justify-content:center;background:#0d0f13;color:#e8e8ea;font-family:system-ui,Segoe UI,sans-serif}
.box{text-align:center}.box h1{font-size:18px;font-weight:600;margin:0 0 6px}.box p{font-size:13px;color:#9aa0a6;margin:0}</style></head>
<body><div class="box"><h1>You're signed in</h1><p>You can close this window and return to Ventus.</p></div></body></html>"#;

pub fn bind() -> Result<(TcpListener, u16)> {
    let std_listener = std::net::TcpListener::bind(("127.0.0.1", 0u16))?;
    std_listener.set_nonblocking(true)?;
    let port = std_listener.local_addr()?.port();
    let listener = TcpListener::from_std(std_listener)?;
    Ok((listener, port))
}

pub async fn serve(listener: TcpListener, html: String, tx: oneshot::Sender<String>) {
    let mut tx = Some(tx);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    loop {
        let accepted = tokio::time::timeout_at(deadline, listener.accept()).await;
        let Ok(Ok((mut stream, _))) = accepted else {
            break;
        };
        let Some((method, path, body)) = read_request(&mut stream).await else {
            respond(
                &mut stream,
                "400 Bad Request",
                "text/plain; charset=utf-8",
                "bad request",
            )
            .await;
            continue;
        };
        if method == "POST" && path.starts_with("/complete") {
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", DONE_PAGE).await;
            if let Some(sender) = tx.take() {
                let _ = sender.send(body);
            }
            break;
        } else if method == "OPTIONS" {
            respond(
                &mut stream,
                "204 No Content",
                "text/plain; charset=utf-8",
                "",
            )
            .await;
        } else if method == "GET" {
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", &html).await;
        } else {
            respond(
                &mut stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                "not found",
            )
            .await;
        }
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

async fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
    let mut data: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end;
    loop {
        let n = stream.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subsequence(&data, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if data.len() > 1_000_000 {
            return None;
        }
    }
    let head = String::from_utf8_lossy(&data[..header_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }
    while data.len() < header_end + content_length {
        let n = stream.read(&mut tmp).await.ok()?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&tmp[..n]);
    }
    let end = (header_end + content_length).min(data.len());
    let body = String::from_utf8_lossy(&data[header_end..end]).to_string();
    Some((method, path, body))
}

async fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}
