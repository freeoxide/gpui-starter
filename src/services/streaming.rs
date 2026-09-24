//! Async-stream-to-GPUI plumbing: drive an async [`Stream`] of tokens into a
//! [`flume::Receiver`] on the shared tokio runtime (on wasm: GPUI's executor).

use flume::Receiver;
use futures_util::Stream;
use gpui::{App, AsyncApp, Context, Task, WeakEntity};

/// Upper bound on buffered tokens before the producer waits on the consumer.
const CHANNEL_CAPACITY: usize = 1024;

/// Dropping the receiver cancels the producer.
pub fn spawn_token_stream<S, E>(cx: &App, stream: S) -> (Task<()>, Receiver<String>)
where
    S: Stream<Item = Result<String, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let (tx, rx) = flume::bounded::<String>(CHANNEL_CAPACITY);

    let pump = async move {
        use futures_util::StreamExt as _;
        let mut stream = std::pin::pin!(stream);
        while let Some(item) = stream.next().await {
            match item {
                Ok(token) => {
                    if tx.send_async(token).await.is_err() {
                        // Receiver dropped — cancel the stream.
                        break;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        target: "gpui_starter::streaming",
                        error = %err,
                        "token stream produced an error; terminating"
                    );
                    // Surface the error as a final chunk so callers that render
                    // inline still see something, then stop.
                    let _ = tx.send_async(err.to_string()).await;
                    break;
                }
            }
        }
        // tx drops at end of scope → receiver sees channel close.
    };

    (producer_task(cx, pump), rx)
}

/// The returned [`Task`] cancels polling when dropped or replaced.
pub fn spawn_token_poller<T, F>(
    rx: Receiver<String>,
    weak: WeakEntity<T>,
    cx: &mut Context<T>,
    mut on_token: F,
) -> Task<()>
where
    T: 'static,
    F: FnMut(&mut T, &str) + Send + 'static,
{
    cx.spawn(async move |_this: WeakEntity<T>, cx: &mut AsyncApp| {
        while let Ok(token) = rx.recv_async().await {
            let _ = cx.update(|cx: &mut App| {
                if let Some(this) = weak.upgrade() {
                    let _ = this.update(cx, |this, cx| {
                        on_token(this, &token);
                        cx.notify();
                    });
                }
            });
        }
        // Channel closed = stream finished. Notify the view so it can flip its
        // "streaming" flag off if it hasn't already.
        let _ = cx.update(|cx: &mut App| {
            if let Some(this) = weak.upgrade() {
                let _ = this.update(cx, |_, cx| {
                    cx.notify();
                });
            }
        });
    })
}

/// Run `pump` on the shared tokio runtime (native) or inline on GPUI's
/// executor (wasm, where the tokio shim is never driven).
fn producer_task(cx: &App, pump: impl Future<Output = ()> + Send + 'static) -> Task<()> {
    #[cfg(not(target_family = "wasm"))]
    {
        let runtime = cx
            .try_global::<crate::services::tokio_runtime::TokioRuntimeGlobal>()
            .map(|g| g.0.runtime.clone());
        cx.spawn(async move |_cx: &mut AsyncApp| {
            let Some(runtime) = runtime else {
                tracing::warn!(
                    target: "gpui_starter::streaming",
                    "TokioRuntimeGlobal not set; token stream cannot start"
                );
                return;
            };
            runtime.spawn(pump).await.ok();
        })
    }

    #[cfg(target_family = "wasm")]
    cx.spawn(async move |_cx: &mut AsyncApp| pump.await)
}
