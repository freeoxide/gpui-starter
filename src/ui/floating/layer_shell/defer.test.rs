use gpui_kit::{
    AnyView, App, AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div, px,
    size,
};

use super::*;
use crate::ui::floating::{FloatingHandle, FloatingWindowId, floating_snapshot, init};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn blank_view(_: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_| Blank).into()
}

#[test]
fn waits_only_while_displays_are_absent_and_budget_remains() {
    assert!(should_wait_for_displays(0, false));
    assert!(should_wait_for_displays(9, false));
    assert!(
        !should_wait_for_displays(10, false),
        "budget spent: the poll loop must fall through and spawn anyway"
    );
    assert!(!should_wait_for_displays(0, true));
}

#[gpui_kit::test]
async fn spawns_anyway_once_the_retry_budget_is_spent(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.update(|cx| {
        spawn_after_display_polls(
            FloatingSpec::new(FloatingWindowId("budget".into()), size(px(120.), px(90.))),
            blank_view,
            cx,
            |_| false,
        )
    });
    cx.run_until_parked();
    for _ in 0..9 {
        cx.executor().advance_clock(DISPLAY_POLL_INTERVAL);
        cx.run_until_parked();
    }
    assert_eq!(
        cx.read(floating_snapshot).len(),
        0,
        "nine polls spent; the budget still holds one"
    );
    cx.executor().advance_clock(DISPLAY_POLL_INTERVAL);
    cx.run_until_parked();
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(snapshot.len(), 1, "the exhausted budget must still spawn");
    assert_eq!(snapshot[0].id.0, "budget");
    assert!(matches!(snapshot[0].handle, FloatingHandle::Native(_)));
}
