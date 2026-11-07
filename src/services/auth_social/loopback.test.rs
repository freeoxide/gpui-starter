use std::time::Duration;

use super::*;

fn scratch_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("scratch runtime")
}

#[test]
fn captures_a_code_redirect_and_serves_the_close_tab_page() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let handle = LoopbackServer::spawn(rt.handle()).expect("spawn loopback");
        assert_ne!(handle.port, 0);
        let url = format!("http://127.0.0.1:{}/?code=x&state=y", handle.port);
        let response = reqwest::get(&url).await.expect("get redirect");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.expect("body");
        assert!(body.contains("close this tab"));
        let raw = tokio::time::timeout(Duration::from_secs(1), handle.wait)
            .await
            .expect("capture resolves")
            .expect("sender alive");
        assert_eq!(raw, url);
    });
}

#[test]
fn favicon_probe_gets_a_404_without_resolving_the_receiver() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let mut handle = LoopbackServer::spawn(rt.handle()).expect("spawn loopback");
        let probe = reqwest::get(format!("http://127.0.0.1:{}/favicon.ico", handle.port))
            .await
            .expect("probe");
        assert_eq!(probe.status(), reqwest::StatusCode::NOT_FOUND);
        assert!(handle.wait.try_recv().is_err());
        let url = format!("http://127.0.0.1:{}/?code=real&state=s", handle.port);
        let response = reqwest::get(&url).await.expect("second request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let raw = tokio::time::timeout(Duration::from_secs(1), handle.wait)
            .await
            .expect("capture resolves")
            .expect("sender alive");
        assert_eq!(raw, url);
    });
}

#[test]
fn dropped_receiver_exits_the_task_after_a_capture() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let handle = LoopbackServer::spawn(rt.handle()).expect("spawn loopback");
        let LoopbackHandle {
            port,
            wait: receiver,
            abort,
        } = handle;
        drop(receiver);
        let response = reqwest::get(format!("http://127.0.0.1:{port}/?code=z&state=w",))
            .await
            .expect("capture request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        for _ in 0..200 {
            if abort.is_finished() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("listener task did not exit after capture with a dropped receiver");
    });
}

#[test]
fn two_spawned_servers_get_distinct_nonzero_ports() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let first = LoopbackServer::spawn(rt.handle()).expect("first spawn");
        let second = LoopbackServer::spawn(rt.handle()).expect("second spawn");
        assert_ne!(first.port, 0);
        assert_ne!(second.port, 0);
        assert_ne!(first.port, second.port);
        drop(first);
        drop(second);
    });
}

#[test]
fn error_redirects_carry_the_error_query_through() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let handle = LoopbackServer::spawn(rt.handle()).expect("spawn loopback");
        let url = format!("http://127.0.0.1:{}/?error=access_denied", handle.port);
        let response = reqwest::get(&url).await.expect("get error redirect");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let raw = tokio::time::timeout(Duration::from_secs(1), handle.wait)
            .await
            .expect("capture resolves")
            .expect("sender alive");
        assert_eq!(raw, url);
    });
}

#[test]
fn garbage_requests_get_a_404_and_the_listener_survives() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let handle = LoopbackServer::spawn(rt.handle()).expect("spawn loopback");

        // Exactly the 8 KiB header cap, with no terminator: rejected, 404.
        let mut oversized = tokio::net::TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .expect("connect oversized");
        oversized
            .write_all(&[b'G'; 8 * 1024])
            .await
            .expect("write capped request");
        let mut response = Vec::new();
        oversized
            .read_to_end(&mut response)
            .await
            .expect("read 404 response");
        let response = String::from_utf8_lossy(&response);
        assert!(response.contains("404 Not Found"), "{response}");

        // A connection dying mid-request-line must not kill the listener.
        let mut aborted = tokio::net::TcpStream::connect(("127.0.0.1", handle.port))
            .await
            .expect("connect aborted");
        aborted
            .write_all(b"GET /callback?code=ha")
            .await
            .expect("write partial request line");
        drop(aborted);

        let url = format!("http://127.0.0.1:{}/?code=survivor&state=s", handle.port);
        let capture = reqwest::get(&url).await.expect("capture after garbage");
        assert_eq!(capture.status(), reqwest::StatusCode::OK);
        let raw = tokio::time::timeout(Duration::from_secs(1), handle.wait)
            .await
            .expect("capture resolves")
            .expect("sender alive");
        assert_eq!(raw, url);
    });
}
