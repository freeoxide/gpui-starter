use gpui_kit::{TestAppContext, VisualTestContext, px, size};

use super::ModelViewerPage;

fn open_page(cx: &mut TestAppContext) -> (gpui_kit::Entity<ModelViewerPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        ModelViewerPage::new(window, cx)
    });
    let page = handle.root(cx).unwrap();
    let cx = VisualTestContext::from_window(handle.into(), cx);
    (page, cx)
}

/// Engine init is environment-dependent (a headless CI runner has no wgpu
/// adapter), so GPU-free tests only drive paths that never need the engine.
/// The first render may schedule the init task, but the engine itself only
/// ever appears from the background task, never synchronously.
#[gpui_kit::test]
fn construction_and_first_render_stay_synchronous_gpu_free(cx: &mut TestAppContext) {
    let (page, cx) = open_page(cx);
    cx.read(|cx| {
        let page_view = page.read(cx);
        assert!(page_view.native.renderer.is_none());
        assert!(page_view.native.engine.is_none());
    });
}

#[gpui_kit::test]
fn install_mesh_flips_the_name_and_swap_flag(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    let state = cx.read(|cx| page.read(cx).state.clone());
    state.update(&mut cx, |s, cx| {
        s.install_mesh(
            std::sync::Arc::new(super::obj::builtin_torus()),
            "second.obj",
            cx,
        )
    });
    assert!(cx.read(|cx| state.read(cx).mesh_name() == "second.obj"));
    assert!(cx.read(|cx| state.read(cx).has_loaded_model()));
    let swap = cx.read(|cx| state.read(cx).take_render_job().swap_mesh.is_some());
    assert!(swap, "install must queue a renderer buffer swap");
}
