use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const TIMEOUT: Duration = Duration::from_secs(300);
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const CAPTURED_PAGE: &str = "<!DOCTYPE html><html><body><p>Sign-in captured. You can close this tab and return to the app.</p></body></html>";

pub(crate) struct LoopbackServer;

/// Single-shot `127.0.0.1` redirect capture (RFC 8252 §7.3): the port dies
/// with the task as soon as a redirect (or the 300 s deadline) lands.
impl LoopbackServer {
    pub(crate) fn spawn(rt: &tokio::runtime::Handle) -> std::io::Result<LoopbackHandle> {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = std_listener.local_addr()?.port();
        std_listener.set_nonblocking(true)?;
        let _guard = rt.enter();
        let listener = TcpListener::from_std(std_listener)?;
        let (tx, rx) = oneshot::channel::<String>();
        let task = rt.spawn(accept_loop(listener, port, tx));
        tracing::info!(
            target: "gpui_starter::auth_social::loopback",
            port,
            "loopback redirect listener bound"
        );
        Ok(LoopbackHandle {
            port,
            wait: rx,
            abort: task.abort_handle(),
        })
    }
}

pub(crate) struct LoopbackHandle {
    pub(crate) port: u16,
    pub(crate) wait: oneshot::Receiver<String>,
    pub(crate) abort: tokio::task::AbortHandle,
}

async fn accept_loop(listener: TcpListener, port: u16, sender: oneshot::Sender<String>) {
    let captured = tokio::time::timeout(TIMEOUT, async {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            let Some((method, target)) = read_request(&mut stream).await else {
                respond(&mut stream, "404 Not Found", "").await;
                continue;
            };
            if method != "GET" || (!target.contains("code=") && !target.contains("error=")) {
                respond(&mut stream, "404 Not Found", "").await;
                continue;
            }
            respond(&mut stream, "200 OK", CAPTURED_PAGE).await;
            if target.starts_with("http://") || target.starts_with("https://") {
                return target;
            }
            return format!("http://127.0.0.1:{port}{target}");
        }
    })
    .await;
    match captured {
        Ok(url) => {
            tracing::info!(
                target: "gpui_starter::auth_social::loopback",
                port,
                "loopback redirect captured"
            );
            let _ = sender.send(url);
        }
        Err(_elapsed) => {
            tracing::warn!(
                target: "gpui_starter::auth_social::loopback",
                port,
                "loopback redirect listener timed out"
            );
        }
    }
}

/// Redirects are GETs whose payload is the query string; only the request
/// line matters here. Full header/body parsing lives in the cfg(test) fake IdP.
async fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        if let Some(at) = find_header_end(&buf) {
            break at;
        }
        if buf.len() >= MAX_REQUEST_BYTES {
            return None;
        }
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..read]);
    };
    let mut lines = buf[..header_end].split(|byte| *byte == b'\n');
    let request_line = lines.next()?;
    let mut parts = request_line.split(|byte: &u8| byte.is_ascii_whitespace());
    let method = std::str::from_utf8(parts.next()?).ok()?.to_string();
    let target = std::str::from_utf8(parts.next()?).ok()?.to_string();
    Some((method, target))
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
#[path = "loopback.test.rs"]
mod loopback_test;
