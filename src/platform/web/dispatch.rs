//! Re-entry bridge: DOM callbacks fire outside GPUI's update loop, so they
//! enqueue jobs here and one GPUI task drains them inside `cx.update`.

use std::sync::OnceLock;

use gpui_kit::App;

const LOG: &str = "gpui_starter::web::dispatch";

static QUEUE: OnceLock<flume::Sender<Box<dyn FnOnce(&mut App) + Send>>> = OnceLock::new();

/// Spawn the drain task and stash the sender; runs once from [`super::install`].
/// The task lives for the page lifetime.
pub fn install(cx: &mut App) {
    let (tx, rx) = flume::unbounded::<Box<dyn FnOnce(&mut App) + Send>>();
    let _ = QUEUE.set(tx);
    cx.spawn(async move |cx| {
        while let Ok(job) = rx.recv_async().await {
            // AsyncApp::update panics if the app was released, but this task is
            // owned by that same app, so the panic path is unreachable.
            cx.update(|cx| job(cx));
        }
    })
    .detach();
}

/// Enqueue a closure for the next drain. Best-effort: before [`install`] the
/// job is dropped — a stale `hashchange` must never crash the page.
pub fn dispatch(job: impl FnOnce(&mut App) + Send + 'static) {
    match QUEUE.get() {
        Some(tx) => {
            let _ = tx.send(Box::new(job));
        }
        None => {
            tracing::warn!(
                target: LOG,
                "browser event dispatched before web::install; dropping job"
            );
        }
    }
}
