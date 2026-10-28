use std::time::Duration;

use gpui_kit::{
    AppContext as _, Context, IntoElement, Modifiers, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, div, point, px, size,
};

use super::WgpuPage;

/// Arms the never-resolving init seam for the test's lifetime.
struct HangInit;

impl HangInit {
    fn arm() -> Self {
        super::set_hang_init(true);
        Self
    }
}

impl Drop for HangInit {
    fn drop(&mut self) {
        super::set_hang_init(false);
    }
}

/// Mounts the page the way AppRoot does: constructed at window open, but
/// rendered only once `visited` flips, so construction stays GPU-free.
struct PageHost {
    page: gpui_kit::Entity<WgpuPage>,
    visited: bool,
}

impl Render for PageHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.visited {
            div().size_full().child(self.page.clone())
        } else {
            div().size_full()
        }
    }
}

fn open_hosted_page(
    cx: &mut TestAppContext,
) -> (
    gpui_kit::Entity<WgpuPage>,
    gpui_kit::Entity<PageHost>,
    VisualTestContext,
) {
    // gpui_kit::init installs the base globals the click path needs (e.g.
    // text-selection suppression); theme::init is what render_native reads.
    cx.update(gpui_kit::init);
    cx.update(gpui_kit::component::theme::init);
    let mut page_slot = None;
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        let page = cx.new(|cx| WgpuPage::new(window, cx));
        page_slot = Some(page.clone());
        PageHost {
            page,
            visited: false,
        }
    });
    let page = page_slot.expect("the host must capture the page entity");
    let host = handle.root(cx).unwrap();
    let cx = VisualTestContext::from_window(handle.into(), cx);
    (page, host, cx)
}

fn runtime_error(page: &gpui_kit::Entity<WgpuPage>, cx: &VisualTestContext) -> Option<String> {
    cx.read(|cx| {
        page.read(cx)
            .state
            .read(cx)
            .runtime_error()
            .map(str::to_string)
    })
}

#[gpui_kit::test]
async fn engine_init_waits_for_the_first_render(cx: &mut TestAppContext) {
    let (page, host, mut cx) = open_hosted_page(cx);
    cx.run_until_parked();
    // Constructed but never rendered: no attempt started, no outcome landed.
    cx.read(|cx| {
        let page_view = page.read(cx);
        assert!(
            !page_view.init_started,
            "construction must not start engine init"
        );
        assert!(page_view.engine.is_none());
        assert!(page_view.state.read(cx).runtime_error().is_none());
    });

    host.update(&mut cx, |host, cx| {
        host.visited = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.read(|cx| {
        let page_view = page.read(cx);
        assert!(page_view.init_started, "the first render must start init");
        let landed =
            page_view.engine.is_some() || page_view.state.read(cx).runtime_error().is_some();
        assert!(landed, "init must land an engine or an error");
    });
}

#[gpui_kit::test]
async fn hung_init_times_out_into_the_error_panel_and_retry_re_enters(cx: &mut TestAppContext) {
    let _hang = HangInit::arm();
    let (page, host, mut cx) = open_hosted_page(cx);
    host.update(&mut cx, |host, cx| {
        host.visited = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    // Pre-deadline: the watchdog stays silent and the page keeps waiting.
    cx.executor().advance_clock(Duration::from_secs(14));
    cx.run_until_parked();
    assert!(
        runtime_error(&page, &cx).is_none(),
        "the watchdog must not fire before the deadline"
    );

    // Later frames must not re-arm the watchdog (init is once per visit).
    for _ in 0..3 {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    let expected = format!(
        "wgpu adapter/device init timed out after {}s",
        super::INIT_TIMEOUT.as_secs()
    );
    assert_eq!(
        runtime_error(&page, &cx),
        Some(expected.clone()),
        "the hung init must land in the error panel with the timeout message"
    );

    // The panel's Retry button is the only interactive element in the
    // centered column; scanning it proves the click path re-enters init.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let mut retried = false;
    for y in (240..560usize).step_by(8) {
        cx.simulate_click(point(px(400.), px(y as f32)), Modifiers::none());
        if runtime_error(&page, &cx).is_none() {
            retried = true;
            break;
        }
    }
    assert!(
        retried,
        "the Retry click must re-enter init and clear the error"
    );

    cx.executor().advance_clock(Duration::from_secs(16));
    cx.run_until_parked();
    assert_eq!(
        runtime_error(&page, &cx),
        Some(expected),
        "the re-entered init must hang and time out again"
    );
}
