use std::time::Duration;

use gpui_kit::{
    AppContext as _, Context, IntoElement, Modifiers, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, div, point, px, size,
};

use super::WgpuPage;

/// Pins the init seam for the test's lifetime; resets to Real on drop,
/// including on panic.
struct SeamGuard;

impl SeamGuard {
    fn arm(seam: super::InitSeam) -> Self {
        super::set_init_seam(seam);
        Self
    }
}

impl Drop for SeamGuard {
    fn drop(&mut self) {
        super::set_init_seam(super::InitSeam::Real);
    }
}

/// Pins the frame seam for the test's lifetime; resets to Real on drop,
/// including on panic.
struct FrameSeamGuard;

impl FrameSeamGuard {
    fn arm(seam: super::FrameSeam) -> Self {
        super::set_frame_seam(seam);
        Self
    }
}

impl Drop for FrameSeamGuard {
    fn drop(&mut self) {
        super::set_frame_seam(super::FrameSeam::Real);
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
    let _seam = SeamGuard::arm(super::InitSeam::Park);
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

#[gpui_kit::test]
async fn success_after_the_timeout_clears_the_error_panel(cx: &mut TestAppContext) {
    // Late Ok must clear the timeout panel; a late Err (adapter-less hosts)
    // replaces it with the real cause.
    let _seam = SeamGuard::arm(super::InitSeam::Delay(
        super::INIT_TIMEOUT + Duration::from_secs(5),
    ));
    let (page, host, mut cx) = open_hosted_page(cx);
    host.update(&mut cx, |host, cx| {
        host.visited = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    cx.executor()
        .advance_clock(super::INIT_TIMEOUT + Duration::from_secs(1));
    cx.run_until_parked();
    let timeout_message = format!(
        "wgpu adapter/device init timed out after {}s",
        super::INIT_TIMEOUT.as_secs()
    );
    assert_eq!(
        runtime_error(&page, &cx),
        Some(timeout_message.clone()),
        "the watchdog must fire while the attempt is still delayed"
    );

    cx.executor().advance_clock(Duration::from_secs(6));
    cx.run_until_parked();
    let (engine, error) = cx.read(|cx| {
        let page_view = page.read(cx);
        (
            page_view.engine.is_some(),
            page_view.state.read(cx).runtime_error().map(str::to_string),
        )
    });
    assert_ne!(
        error.as_deref(),
        Some(timeout_message.as_str()),
        "a landed attempt must replace or clear the stale timeout error"
    );
    if engine {
        assert!(
            error.is_none(),
            "a late success must clear the timeout error panel"
        );
    }
}

#[gpui_kit::test]
#[ignore = "needs a working wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn a_wedged_frame_poisons_the_engine_and_retry_builds_a_fresh_one(cx: &mut TestAppContext) {
    let _seam = FrameSeamGuard::arm(super::FrameSeam::Wedge);
    let (page, host, mut cx) = open_hosted_page(cx);
    host.update(&mut cx, |host, cx| {
        host.visited = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    let first_engine = cx.read(|cx| {
        page.read(cx)
            .engine
            .clone()
            .expect("adapter hosts must land an engine")
    });
    assert!(!first_engine.poisoned());

    // One kicked frame parks inside the fence until the deadline trips.
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.simulate_next_frame(cx);
    });
    cx.executor().advance_clock(Duration::from_millis(40));
    cx.run_until_parked();
    let expected = format!(
        "wgpu render missed its {}s deadline; engine poisoned (wedged render thread detached)",
        crate::features::wgpu::fence::FRAME_DEADLINE.as_secs()
    );
    assert_eq!(
        runtime_error(&page, &cx),
        Some(expected),
        "the fenced deadline must land in the error panel"
    );
    assert!(
        cx.read(|cx| page.read(cx).engine.as_ref().is_some_and(|e| e.poisoned())),
        "the missed deadline must poison the engine"
    );

    // Retry must build a fresh engine; a poisoned one is never reused.
    super::set_frame_seam(super::FrameSeam::Real);
    // The 100k tier keeps the resumed frames comfortably inside the fence.
    page.update(&mut cx, |p, cx| {
        p.state
            .update(cx, |s, cx| s.set_particle_count(100_000, cx))
    });
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
    cx.executor().advance_clock(Duration::from_millis(40));
    cx.run_until_parked();

    let fresh_engine = cx.read(|cx| {
        page.read(cx)
            .engine
            .clone()
            .expect("retry must land a fresh engine")
    });
    assert!(
        !fresh_engine.poisoned(),
        "retry must build an unpoisoned engine"
    );
    assert!(
        !std::sync::Arc::ptr_eq(&first_engine, &fresh_engine),
        "retry must build a new engine instance, not reuse the poisoned one"
    );

    for _ in 0..8 {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
        });
        cx.executor().advance_clock(Duration::from_millis(40));
        cx.run_until_parked();
    }
    let frames = cx.read(|cx| page.read(cx).state.read(cx).frames_rendered());
    assert!(frames >= 1, "playback must resume on the fresh engine");
    assert!(
        runtime_error(&page, &cx).is_none(),
        "resumed playback must stay out of the error panel"
    );
}

#[gpui_kit::test]
#[ignore = "needs a working wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn a_wedged_renderer_creation_poisons_the_engine_and_retry_rebuilds(cx: &mut TestAppContext) {
    let _seam = FrameSeamGuard::arm(super::FrameSeam::WedgeCreate);
    let (page, host, mut cx) = open_hosted_page(cx);
    host.update(&mut cx, |host, cx| {
        host.visited = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    let first_engine = cx.read(|cx| {
        page.read(cx)
            .engine
            .clone()
            .expect("adapter hosts must land an engine")
    });

    // The first kicked frame wedges inside the fenced renderer creation.
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.simulate_next_frame(cx);
    });
    cx.executor().advance_clock(Duration::from_millis(40));
    cx.run_until_parked();
    let expected = format!(
        "wgpu render missed its {}s deadline; engine poisoned (wedged render thread detached)",
        crate::features::wgpu::fence::FRAME_DEADLINE.as_secs()
    );
    assert_eq!(
        runtime_error(&page, &cx),
        Some(expected),
        "a wedged creation must land in the error panel"
    );
    assert!(
        cx.read(|cx| page.read(cx).engine.as_ref().is_some_and(|e| e.poisoned())),
        "a wedged creation must poison the engine"
    );
    assert!(
        cx.read(|cx| page.read(cx).renderer.is_none()),
        "a wedged creation must not leave a renderer behind"
    );

    // Retry must build a fresh engine and renderer; the poisoned one is
    // never reused.
    super::set_frame_seam(super::FrameSeam::Real);
    // The 100k tier keeps the resumed frames comfortably inside the fence.
    page.update(&mut cx, |p, cx| {
        p.state
            .update(cx, |s, cx| s.set_particle_count(100_000, cx))
    });
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
    cx.executor().advance_clock(Duration::from_millis(40));
    cx.run_until_parked();

    let fresh_engine = cx.read(|cx| {
        page.read(cx)
            .engine
            .clone()
            .expect("retry must land a fresh engine")
    });
    assert!(
        !fresh_engine.poisoned(),
        "retry must build an unpoisoned engine"
    );
    assert!(
        !std::sync::Arc::ptr_eq(&first_engine, &fresh_engine),
        "retry must build a new engine instance, not reuse the poisoned one"
    );

    for _ in 0..8 {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
        });
        cx.executor().advance_clock(Duration::from_millis(40));
        cx.run_until_parked();
    }
    assert!(
        cx.read(|cx| page.read(cx).renderer.is_some()),
        "the fresh engine must complete its renderer creation"
    );
    let frames = cx.read(|cx| page.read(cx).state.read(cx).frames_rendered());
    assert!(frames >= 1, "playback must resume on the fresh engine");
    assert!(
        runtime_error(&page, &cx).is_none(),
        "resumed playback must stay out of the error panel"
    );
}
