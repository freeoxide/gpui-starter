use gpui_kit::prelude::*;
use gpui_kit::{Entity, TestAppContext, VisualTestContext, Window, point, px, size};

use crate::platform::clock::Instant;
use crate::ui::canvas::perf_probe;

use super::InfiniteCanvasPage;
use super::minimap::minimap_probe;

use super::perf::{
    PAGE_SLOW_RENDER_THRESHOLD_US, is_slow_page_render, last_gen_us, last_page_render_us,
    store_gen_us, store_page_render_us,
};

#[test]
fn render_and_gen_atomics_roundtrip() {
    store_page_render_us(1_234);
    assert_eq!(last_page_render_us(), 1_234);
    store_gen_us(56_789);
    assert_eq!(last_gen_us(), 56_789);
}

#[test]
fn slow_render_boundary_is_exclusive() {
    assert!(!is_slow_page_render(PAGE_SLOW_RENDER_THRESHOLD_US));
    assert!(is_slow_page_render(PAGE_SLOW_RENDER_THRESHOLD_US + 1));
}

fn drawn_page(cx: &mut TestAppContext) -> (Entity<InfiniteCanvasPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        InfiniteCanvasPage::new(window, cx)
    });
    let page = handle.root(cx).unwrap();
    let mut vcx = VisualTestContext::from_window(handle.into(), cx);
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    (page, vcx)
}

fn pan_and_draw(page: &Entity<InfiniteCanvasPage>, vcx: &mut VisualTestContext, frame: usize) {
    let bounce = if frame % 2 == 0 { 1. } else { -1. };
    page.update(vcx, |page, cx| {
        page.state.update(cx, |state, cx| {
            state.pan_by(point(px(7. * bounce), px(5. * bounce)), cx)
        })
    });
    vcx.update(|window, cx| window.draw(cx).clear(cx));
}

/// Two pans returning to the starting offset; warmup over the same pair
/// keeps cull-boundary first appearances out of the measured pass.
fn pan_pair(page: &Entity<InfiniteCanvasPage>, vcx: &mut VisualTestContext) {
    pan_and_draw(page, vcx, 0);
    pan_and_draw(page, vcx, 1);
}

fn set_zoom(page: &Entity<InfiniteCanvasPage>, vcx: &mut VisualTestContext, zoom: f32) {
    page.update(vcx, |page, cx| {
        page.state.update(cx, |state, cx| state.set_zoom(zoom, cx))
    });
    vcx.update(|window, cx| window.draw(cx).clear(cx));
}

fn assert_quiet_pans(page: &Entity<InfiniteCanvasPage>, vcx: &mut VisualTestContext) {
    perf_probe::reset();
    minimap_probe::reset();
    pan_pair(page, vcx);
    let [path_builds, key_hashes, text_shapings] = perf_probe::snapshot();
    assert_eq!(path_builds, 0, "steady pan must not rebuild cached paths");
    assert_eq!(key_hashes, 0, "steady pan must not rehash shape content");
    assert_eq!(text_shapings, 0, "steady pan must not reshape labels");
    assert_eq!(
        minimap_probe::dot_anchor_passes(),
        0,
        "steady pan must not reproject minimap anchors"
    );
}

/// Pan must not rebuild paths, rehash content, reshape labels, or reproject
/// minimap anchors: those depend on document and zoom only.
#[gpui_kit::test]
fn pan_frames_reuse_every_paint_cache(cx: &mut TestAppContext) {
    let (page, mut vcx) = drawn_page(cx);
    page.update(&mut vcx, |page, cx| page.fit_view(cx));
    set_zoom(&page, &mut vcx, 0.6);
    pan_pair(&page, &mut vcx);
    assert!(
        crate::ui::canvas::last_paint_us() > 0,
        "the canvas element must record its paint duration"
    );
    assert_quiet_pans(&page, &mut vcx);

    // A redundant zoom write keeps the generation, so nothing revalidates.
    set_zoom(&page, &mut vcx, 0.6);
    assert_quiet_pans(&page, &mut vcx);

    // A real zoom change revalidates every visible shape exactly once...
    perf_probe::reset();
    minimap_probe::reset();
    set_zoom(&page, &mut vcx, 0.7);
    let [path_builds, key_hashes, _] = perf_probe::snapshot();
    assert!(
        path_builds > 0 || key_hashes > 0,
        "a zoom change must revalidate the visible shapes"
    );
    assert_eq!(
        minimap_probe::dot_anchor_passes(),
        0,
        "zoom does not move minimap anchors"
    );
    // ...and the new generation is then steady across pans.
    assert_quiet_pans(&page, &mut vcx);

    // A document edit bumps the revision; the revalidating paint keeps
    // unchanged shapes cached, and pans stay quiet afterwards.
    let labels = vcx.read(|cx| page.read(cx).layers.1);
    page.update(&mut vcx, |page, cx| {
        page.state.update(cx, |state, cx| {
            state.edit_document(cx, |doc| doc.set_layer_visible(labels, true));
        })
    });
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    assert_quiet_pans(&page, &mut vcx);
}

/// Wall-clock guard: medians are per `window.draw` (the harness paints the
/// tree twice per draw), with headroom for CI variance. Release-tuned.
#[gpui_kit::test]
fn pan_frame_budget_stays_under_ceiling(cx: &mut TestAppContext) {
    let frames = if cfg!(debug_assertions) { 5 } else { 24 };
    // Debug medians run ~60ms here but the host varies ~2x between sessions,
    // so the debug ceiling keeps >4x headroom; release is the tuned lane.
    let ceiling_us = if cfg!(debug_assertions) {
        250_000
    } else {
        40_000
    };
    let (page, mut vcx) = drawn_page(cx);

    let mut scenario = |label: &str, zoom: Option<f32>, vcx: &mut VisualTestContext| {
        if let Some(zoom) = zoom {
            set_zoom(&page, vcx, zoom);
        }
        for frame in 0..2 {
            pan_and_draw(&page, vcx, frame);
        }
        let mut times = Vec::with_capacity(frames);
        for frame in 0..frames {
            let started = Instant::now();
            pan_and_draw(&page, vcx, frame);
            times.push(started.elapsed().as_micros() as u64);
        }
        times.sort_unstable();
        let median = times[frames / 2];
        println!("{label}: median {median} us · max {}", times[frames - 1]);
        assert!(
            median <= ceiling_us,
            "{label}: median frame {median} us exceeds the {ceiling_us} us ceiling"
        );
    };

    // Fit view: fit wants ~0.04 zoom but clamps to 0.1, the most-visible
    // zoom the page allows (~1.3k of the bodies on screen); labels LOD'd off.
    page.update(&mut vcx, |page, cx| page.fit_view(cx));
    scenario("fit_5k_pan", None, &mut vcx);
    // Labels-on view: the label layer is visible from zoom 0.5 up.
    scenario("labels_5k_pan", Some(0.6), &mut vcx);
    // 20k preset, fit view: the heaviest shipped field (same visible count).
    page.update(&mut vcx, |page, cx| page.regenerate(20_000, cx));
    page.update(&mut vcx, |page, cx| page.fit_view(cx));
    scenario("fit_20k_pan", None, &mut vcx);
}
