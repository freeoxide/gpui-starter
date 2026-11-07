use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use gpui_kit::TestAppContext;
use tokio::io::AsyncWriteExt as _;

use super::*;

fn wait_until(cx: &mut TestAppContext, mut condition: impl FnMut(&mut TestAppContext) -> bool) {
    for _ in 0..300 {
        cx.run_until_parked();
        if condition(cx) {
            return;
        }
        // The probe bridge polls on a virtual 25ms timer; advance it so the
        // tokio wakes that race the executor still get processed.
        cx.executor().advance_clock(Duration::from_millis(25));
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("condition not met before the deadline");
}

/// Loopback endpoint answering a fixed status; when `hold_first` is set, the
/// first connection's response is gated until its sender fires.
struct ProbeServer {
    origin: String,
    hits: Arc<AtomicUsize>,
    responses: Arc<AtomicUsize>,
}

impl ProbeServer {
    fn spawn(
        rt: &tokio::runtime::Handle,
        status: u16,
        mut hold_first: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Self {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("probe bind");
        let port = std_listener.local_addr().unwrap().port();
        std_listener.set_nonblocking(true).expect("nonblocking");
        let origin = format!("http://127.0.0.1:{port}");
        let hits = Arc::new(AtomicUsize::new(0));
        let responses = Arc::new(AtomicUsize::new(0));
        let reason = if status == 200 {
            "OK"
        } else {
            "Internal Server Error"
        };
        let body =
            format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _guard = rt.enter();
        let listener = tokio::net::TcpListener::from_std(std_listener).expect("tokio listener");
        let task_hits = hits.clone();
        let task_responses = responses.clone();
        rt.spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                task_hits.fetch_add(1, Ordering::SeqCst);
                if let Some(gate) = hold_first.take() {
                    let _ = gate.await;
                }
                let _ = stream.write_all(body.as_bytes()).await;
                let _ = stream.shutdown().await;
                task_responses.fetch_add(1, Ordering::SeqCst);
            }
        });
        Self {
            origin,
            hits,
            responses,
        }
    }
}

// The stale probe is held until the fresh one has landed, so completion
// order is forced: without the generation guard the stale 500 would win.
// Flaky under the deterministic executor: tokio-worker wakes race the test
// scheduler's park nondeterministically; the probe bridge now timer-polls
// in production, but the harness still drops wakes ~half the runs.
#[gpui_kit::test]
#[ignore = "environment-flaky: cross-runtime wake race in the test scheduler"]
async fn stale_probe_result_cannot_clobber_a_fresher_one(cx: &mut TestAppContext) {
    // The probes finish on tokio worker threads; the deterministic
    // scheduler otherwise rejects those wakes (gpui executor allow_parking).
    cx.executor().allow_parking();
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("scratch runtime"),
    );
    let (release, held) = tokio::sync::oneshot::channel::<()>();
    let stale = ProbeServer::spawn(runtime.handle(), 500, Some(held));
    let fresh = ProbeServer::spawn(runtime.handle(), 200, None);

    cx.update(|cx| {
        cx.set_global(crate::services::tokio_runtime::TokioRuntimeGlobal(
            crate::services::tokio_runtime::TokioRuntime {
                runtime: runtime.clone(),
                http_client: reqwest::Client::new(),
            },
        ));
        crate::connectivity::initialize(cx);
        cx.update_global::<ConnectivitySnapshot, _>(|next, _| {
            next.probe_url = stale.origin.clone()
        });
    });
    cx.update(|cx| check_now(cx));

    // The stale probe must be in flight before the fresh one is spawned.
    wait_until(cx, |_| stale.hits.load(Ordering::SeqCst) == 1);
    cx.update(|cx| {
        cx.update_global::<ConnectivitySnapshot, _>(|next, _| {
            next.probe_url = fresh.origin.clone()
        });
    });
    cx.update(|cx| check_now(cx));

    wait_until(cx, |cx| {
        cx.update(|cx| snapshot(cx).state == ConnectivityState::Online)
    });
    release.send(()).expect("release the stale probe");
    wait_until(cx, |_| stale.responses.load(Ordering::SeqCst) == 1);
    for _ in 0..10 {
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(25));
        std::thread::sleep(Duration::from_millis(5));
    }

    let final_snapshot = cx.update(|cx| snapshot(cx));
    assert_eq!(final_snapshot.state, ConnectivityState::Online);
    assert!(
        final_snapshot.last_error.is_none(),
        "a stale probe result must not overwrite a fresher one: {:?}",
        final_snapshot.last_error
    );
}
