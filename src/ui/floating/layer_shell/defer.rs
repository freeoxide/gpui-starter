use std::time::Duration;

use gpui_kit::{AnyView, App, Window};

use super::LOG;
use crate::ui::floating::{FloatingSpec, spawn_floating};

/// Spawn a float once `cx.displays()` is enumerable (zed#46378: the list
/// stays empty until a first frame renders). Immediate when non-empty;
/// else 100 ms retries, max 10, then spawns anyway. Never panics.
pub fn spawn_floating_after_displays(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) {
    if !cx.displays().is_empty() {
        if let Err(err) = spawn_floating(spec, build, cx) {
            tracing::warn!(target: LOG, error = %err, "deferred float spawn failed");
        }
        return;
    }
    cx.spawn(async move |cx| {
        for _ in 0..10 {
            if cx.update(|cx| !cx.displays().is_empty()) {
                break;
            }
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
        }
        let result = cx.update(|cx| spawn_floating(spec, build, cx));
        tracing::warn!(target: LOG, ?result, "deferred layer-shell spawn settled");
    })
    .detach();
}

#[cfg(test)]
#[path = "defer.test.rs"]
mod defer_test;
