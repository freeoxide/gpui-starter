use std::time::Duration;

use gpui_kit::{AnyView, App, Window};

use super::LOG;
use crate::ui::floating::{FloatingSpec, spawn_floating};

const DISPLAY_POLL_BUDGET: usize = 10;
const DISPLAY_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Spawn a float once `cx.displays()` is non-empty (zed#46378: empty until a first frame
/// renders); retries every 100 ms, at most 10 times, then spawns anyway. Never panics.
pub fn spawn_floating_after_displays(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) {
    spawn_after_display_polls(spec, build, cx, |cx| !cx.displays().is_empty());
}

/// `displays_enumerable` is injectable so tests can drive the retry budget;
/// production always polls the real display list.
fn spawn_after_display_polls(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
    displays_enumerable: impl Fn(&App) -> bool + 'static,
) {
    if displays_enumerable(cx) {
        if let Err(err) = spawn_floating(spec, build, cx) {
            tracing::warn!(target: LOG, error = %err, "deferred float spawn failed");
        }
        return;
    }
    cx.spawn(async move |cx| {
        let mut polls = 0;
        while should_wait_for_displays(polls, cx.update(|cx| displays_enumerable(cx))) {
            polls += 1;
            cx.background_executor().timer(DISPLAY_POLL_INTERVAL).await;
        }
        let result = cx.update(|cx| spawn_floating(spec, build, cx));
        tracing::warn!(target: LOG, ?result, "deferred layer-shell spawn settled");
    })
    .detach();
}

/// Keep polling only while displays are absent and budget remains; an
/// exhausted budget spawns anyway (wrong output beats never appearing).
fn should_wait_for_displays(polls_done: usize, displays_enumerable: bool) -> bool {
    !displays_enumerable && polls_done < DISPLAY_POLL_BUDGET
}

#[cfg(test)]
#[path = "defer.test.rs"]
mod defer_test;
