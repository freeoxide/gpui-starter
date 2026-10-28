use gpui_kit::component::h_flex;
use gpui_kit::prelude::*;
use gpui_kit::{Bounds, Entity, TestAppContext, VisualTestContext, Window, div, point, px, size};

use super::InfiniteCanvasPage;
use super::minimap::{
    MiniProjection, fit_projection, project_doc_to_minimap, unproject_minimap_to_doc,
};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds {
        origin: point(px(x), px(y)),
        size: size(px(w), px(h)),
    }
}

fn assert_px_near(actual: gpui_kit::Pixels, expected: gpui_kit::Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.001,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[test]
fn fit_projection_of_a_known_pair_is_exact() {
    let pr: MiniProjection = fit_projection(rect(0., 0., 1000., 500.), rect(100., 50., 160., 96.));
    let expected_scale: f32 = 160. / 1000.;
    assert_eq!(pr.scale.to_bits(), expected_scale.to_bits());
    assert_px_near(pr.offset.x, px(100.));
    assert_px_near(pr.offset.y, px(50. + (96. - 500. * expected_scale) / 2.));
}

#[test]
fn portrait_field_centers_the_leftover_evenly() {
    let pr = fit_projection(rect(0., 0., 200., 800.), rect(10., 20., 160., 96.));
    let expected_scale: f32 = 96. / 800.;
    assert_eq!(pr.scale.to_bits(), expected_scale.to_bits());
    assert_px_near(pr.offset.x, px(10. + (160. - 200. * expected_scale) / 2.));
    assert_px_near(pr.offset.y, px(20.));
    let left_margin = pr.offset.x - px(10.);
    let right_margin = px(10. + 160.) - (pr.offset.x + px(200. * expected_scale));
    assert_px_near(left_margin, right_margin);
}

#[test]
fn projection_roundtrips_within_a_thousandth_of_a_pixel() {
    let pr = fit_projection(rect(0., 0., 12820., 12820.), rect(200., 300., 160., 96.));
    for p in [
        point(px(0.), px(0.)),
        point(px(6410.), px(3205.)),
        point(px(12820.), px(12820.)),
    ] {
        let roundtrip = unproject_minimap_to_doc(project_doc_to_minimap(p, &pr), &pr);
        assert_px_near(roundtrip.x, p.x);
        assert_px_near(roundtrip.y, p.y);
    }
}

struct CanvasHost {
    page: Entity<InfiniteCanvasPage>,
}

impl Render for CanvasHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .child(div().id("lead").w_16())
            .child(self.page.clone())
    }
}

fn mounted_host(cx: &mut TestAppContext) -> (Entity<InfiniteCanvasPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        let page = cx.new(|cx| InfiniteCanvasPage::new(window, cx));
        CanvasHost { page }
    });
    let host = handle.root(cx).unwrap();
    let page = cx.read(|cx| host.read(cx).page.clone());
    let mut cx = VisualTestContext::from_window(handle.into(), &cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (page, cx)
}

fn assert_doc_at_local_center(
    cx: &VisualTestContext,
    page: &Entity<InfiniteCanvasPage>,
    expected: gpui_kit::Point<gpui_kit::Pixels>,
) {
    let state = cx.read(|cx| page.read(cx).state.clone());
    let viewport = cx.read(|cx| page.read(cx).viewport);
    let local_center = point(viewport.size.width * 0.5, viewport.size.height * 0.5);
    let doc = cx.read(|cx| state.read(cx).transform().to_doc(local_center));
    assert!(
        (doc.x - expected.x).as_f32().abs() < 1.,
        "expected doc.x ~{}, got {}",
        expected.x,
        doc.x
    );
    assert!(
        (doc.y - expected.y).as_f32().abs() < 1.,
        "expected doc.y ~{}, got {}",
        expected.y,
        doc.y
    );
}

#[gpui_kit::test]
fn clicking_the_minimap_center_jumps_the_viewport_to_the_field_center(cx: &mut TestAppContext) {
    let (page, mut cx) = mounted_host(cx);
    let span_center = cx.read(|cx| page.read(cx).field.span().center());
    let click = cx.read(|cx| page.read(cx).minimap_bounds.center());
    cx.simulate_click(click, gpui_kit::Modifiers::none());
    assert_doc_at_local_center(&cx, &page, span_center);
}

#[gpui_kit::test]
fn clicking_an_interior_minimap_point_jumps_there(cx: &mut TestAppContext) {
    let (page, mut cx) = mounted_host(cx);
    let (span, minimap_bounds) = cx.read(|cx| {
        let page = page.read(cx);
        (page.field.span(), page.minimap_bounds)
    });
    let probe_doc = span.origin + point(span.size.width, span.size.height) * 0.25;
    let click = project_doc_to_minimap(probe_doc, &fit_projection(span, minimap_bounds));
    cx.simulate_click(click, gpui_kit::Modifiers::none());
    assert_doc_at_local_center(&cx, &page, probe_doc);
}

#[test]
fn rasterized_dots_paint_at_projected_positions_in_palette_colors() {
    use super::content::{NoteField, PALETTE_LEN};
    use super::minimap::rasterize_dots;
    use gpui_kit::{DevicePixels, Hsla, Size};

    let field = NoteField { seed: 7, count: 4 };
    let palette: [Hsla; PALETTE_LEN] = [
        Hsla::red(),
        Hsla::green(),
        Hsla::blue(),
        Hsla::black(),
        Hsla::white(),
    ];
    let minimap = rect(10., 20., 160., 96.);
    let pr: MiniProjection = fit_projection(field.span(), minimap);
    let image = rasterize_dots(
        &field,
        &palette,
        &pr,
        minimap.origin,
        Size {
            width: gpui_kit::px(160.),
            height: gpui_kit::px(96.),
        },
        1.,
    );
    assert_eq!(
        image.size(0),
        gpui_kit::size(DevicePixels(160), DevicePixels(96))
    );
    let frame = image.as_bytes(0).unwrap();
    let px_at = |x: u32, y: u32| {
        let start = ((y * 160 + x) * 4) as usize;
        frame[start..start + 4].to_vec()
    };
    assert_eq!(
        px_at(0, 0),
        vec![0, 0, 0, 0],
        "background stays transparent"
    );
    for anchor in field.anchors() {
        let local = project_doc_to_minimap(anchor.center, &pr) - minimap.origin;
        let x = local.x.as_f32().round() as u32;
        let y = local.y.as_f32().round() as u32;
        let rgba = palette[anchor.palette_ix].to_rgb();
        let expected = vec![
            (rgba.b * 255.).round() as u8,
            (rgba.g * 255.).round() as u8,
            (rgba.r * 255.).round() as u8,
            (rgba.a * 255.).round() as u8,
        ];
        assert_eq!(
            px_at(x, y),
            expected,
            "dot at ({x},{y}) must be BGRA {expected:?}"
        );
    }
}
