use std::time::Duration;

use gpui_kit::{DevicePixels, Size, TestAppContext, VisualTestContext, px, size};

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
