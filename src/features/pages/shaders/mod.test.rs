use std::time::Duration;

use gpui_kit::{
    AppContext as _, Context, DevicePixels, IntoElement, Modifiers, ParentElement as _, Render,
    Size, Styled as _, TestAppContext, VisualTestContext, Window, div, point, px, size,
};

use super::ShaderCanvasPage;
use super::demos::{DEMOS, DemoId};
use crate::ui::canvas::CanvasShape;

fn open_page(cx: &mut TestAppContext) -> (gpui_kit::Entity<ShaderCanvasPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        ShaderCanvasPage::new(window, cx)
    });
    let page = handle.root(cx).unwrap();
    let cx = VisualTestContext::from_window(handle.into(), cx);
    (page, cx)
}

/// Engine init is environment-dependent (a headless CI runner has no wgpu
/// adapter), so the GPU-free tests below drive `switch_demo` directly: the
/// install path itself must never need the engine.
fn switch_demo(page: &gpui_kit::Entity<ShaderCanvasPage>, cx: &mut VisualTestContext, ix: usize) {
    page.update(cx, |page, cx| page.switch_demo(ix, cx));
}

fn installed_image(
    page: &gpui_kit::Entity<ShaderCanvasPage>,
    cx: &VisualTestContext,
) -> (gpui_kit::Bounds<gpui_kit::Pixels>, Size<DevicePixels>) {
    cx.read(|cx| {
        let page_view = page.read(cx);
        let shape_id = page_view
            .image_shape
            .expect("demo installed no image shape");
        match page_view.canvas.read(cx).document().shape(shape_id) {
            Some(CanvasShape::Image { bounds, image }) => (*bounds, image.size(0)),
            other => panic!("expected an image shape, got {other:?}"),
        }
    })
}

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
    page: gpui_kit::Entity<ShaderCanvasPage>,
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
    gpui_kit::Entity<ShaderCanvasPage>,
    gpui_kit::Entity<PageHost>,
    VisualTestContext,
) {
    // gpui_kit::init installs the base globals the click path needs (e.g.
    // text-selection suppression); theme::init is what render_native reads.
    cx.update(gpui_kit::init);
    cx.update(gpui_kit::component::theme::init);
    let mut page_slot = None;
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        let page = cx.new(|cx| ShaderCanvasPage::new(window, cx));
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
        assert!(page_view.runtime.is_none());
        assert!(page_view.shader.read(cx).runtime_error().is_none());
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
            page_view.runtime.is_some() || page_view.shader.read(cx).runtime_error().is_some();
        assert!(landed, "init must land a runtime or an error");
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
        cx.read(|cx| page.read(cx).shader.read(cx).runtime_error().is_none()),
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
        cx.read(|cx| page
            .read(cx)
            .shader
            .read(cx)
            .runtime_error()
            .map(str::to_string)),
        Some(expected.clone()),
        "the hung init must land in the error panel with the timeout message"
    );

    // The panel's Retry button is the only interactive element in the
    // centered column; scanning it proves the click path re-enters init.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let mut retried = false;
    for y in (240..560usize).step_by(8) {
        cx.simulate_click(point(px(400.), px(y as f32)), Modifiers::none());
        if cx.read(|cx| page.read(cx).shader.read(cx).runtime_error().is_none()) {
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
        cx.read(|cx| page
            .read(cx)
            .shader
            .read(cx)
            .runtime_error()
            .map(str::to_string)),
        Some(expected),
        "the re-entered init must hang and time out again"
    );
}

#[gpui_kit::test]
async fn every_demo_installs_its_document_and_tool_without_a_gpu(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    cx.run_until_parked();

    for (ix, spec) in DEMOS.iter().enumerate() {
        switch_demo(&page, &mut cx, ix);
        let (demo, tool_id, slider) = cx.read(|cx| {
            let page_view = page.read(cx);
            let shader = page_view.shader.read(cx);
            (
                shader.demo(),
                page_view.canvas.read(cx).active_tool_id().to_string(),
                shader.slider(),
            )
        });

        assert_eq!(demo, spec.id, "demo {ix} did not become active");
        let expected_tool = match spec.id {
            DemoId::Ripple => "shader-ripple",
            _ => "pan",
        };
        assert_eq!(tool_id, expected_tool, "demo {ix} installed the wrong tool");
        assert!(
            (slider - spec.slider.default).abs() < f32::EPSILON,
            "demo {ix} did not reset the slider to its default"
        );

        let (bounds, image_size) = installed_image(&page, &cx);
        assert_eq!(
            (bounds.size.width, bounds.size.height),
            (px(spec.width as f32), px(spec.height as f32)),
            "demo {ix} image shape has the wrong doc bounds"
        );
        assert_eq!(
            (image_size.width, image_size.height),
            (DevicePixels(1), DevicePixels(1)),
            "demo {ix} placeholder must stay the 1x1 blank until a frame lands"
        );
    }
}

#[gpui_kit::test]
#[ignore = "needs a working wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn the_page_loop_lands_sustained_frames_for_every_demo(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    // Engine init is lazy: the first render (not construction) kicks it.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    let init_error = cx.read(|cx| {
        page.read(cx)
            .shader
            .read(cx)
            .runtime_error()
            .map(str::to_string)
    });
    assert!(
        cx.read(|cx| page.read(cx).runtime.is_some()),
        "engine init failed: {init_error:?}"
    );

    for (ix, spec) in DEMOS.iter().enumerate() {
        switch_demo(&page, &mut cx, ix);
        let baseline = cx.read(|cx| page.read(cx).shader.read(cx).frames_rendered());
        for _ in 0..8 {
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                window.simulate_next_frame(cx);
            });
            cx.executor().advance_clock(Duration::from_millis(40));
            cx.run_until_parked();
        }
        let frames = cx.read(|cx| page.read(cx).shader.read(cx).frames_rendered());
        let error = cx.read(|cx| {
            page.read(cx)
                .shader
                .read(cx)
                .runtime_error()
                .map(str::to_string)
        });
        assert!(
            frames - baseline >= 2,
            "{:?} (demo {ix}) produced {} frames; error: {error:?}",
            spec.id,
            frames - baseline
        );
        let (_, image_size) = installed_image(&page, &cx);
        assert_eq!(
            (image_size.width, image_size.height),
            (
                DevicePixels(spec.width as i32),
                DevicePixels(spec.height as i32)
            ),
            "{:?} (demo {ix}) never replaced the placeholder with a rendered frame",
            spec.id
        );
    }
}
