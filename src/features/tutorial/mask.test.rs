use gpui_kit::{Bounds, FillOptions, FillRule, PathBuilder, PathStyle, Pixels, point, px, size};

use super::{cutout_bounds, place_card, rounded_rect_path};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds {
        origin: point(px(x), px(y)),
        size: size(px(w), px(h)),
    }
}

fn assert_px_eq(actual: Pixels, expected: f32, msg: &str) {
    assert!(
        (actual.as_f32() - expected).abs() < 0.01,
        "{msg}: expected ~{expected}px, got {}",
        actual.as_f32()
    );
}

#[test]
fn cutout_expands_symmetrically() {
    let target = rect(100., 150., 60., 40.);
    let cutout = cutout_bounds(target, px(8.));
    assert_px_eq(cutout.origin.x, 92., "left pad");
    assert_px_eq(cutout.origin.y, 142., "top pad");
    assert_px_eq(cutout.size.width, 76., "width grows by both pads");
    assert_px_eq(cutout.size.height, 56., "height grows by both pads");
}

#[test]
fn place_card_below_when_space() {
    let target = rect(100., 100., 60., 40.);
    let viewport = rect(0., 0., 800., 600.);
    let card = place_card(target, viewport, px(320.), px(192.), px(8.));
    assert_px_eq(card.origin.x, 100., "card keeps the target's left edge");
    assert_px_eq(
        card.origin.y,
        148.,
        "card top sits one gap below the target",
    );
}

#[test]
fn place_card_above_when_no_space_below() {
    let target = rect(100., 500., 60., 40.);
    let viewport = rect(0., 0., 800., 600.);
    let card = place_card(target, viewport, px(320.), px(192.), px(8.));
    assert_px_eq(
        card.bottom(),
        492.,
        "card bottom sits one gap above the target",
    );
}

#[test]
fn place_card_clamps_to_viewport() {
    let viewport = rect(0., 0., 400., 300.);
    let near_right = rect(350., 20., 40., 30.);
    let card = place_card(near_right, viewport, px(320.), px(100.), px(8.));
    assert_px_eq(
        card.origin.x,
        80.,
        "x clamps so the card's right edge hits the viewport",
    );

    let tiny = rect(0., 0., 200., 120.);
    let tall = rect(10., 10., 40., 30.);
    let card = place_card(tall, tiny, px(320.), px(100.), px(8.));
    assert_px_eq(
        card.origin.x,
        0.,
        "x clamps to the left edge when the card is wider",
    );
    assert_px_eq(
        card.origin.y,
        0.,
        "y clamps to the top when the above placement overshoots",
    );
}

#[test]
fn mask_path_builds() {
    let viewport = rect(0., 0., 800., 600.);
    let cutout = cutout_bounds(rect(100., 100., 200., 80.), px(8.));
    let mut builder = PathBuilder::fill().with_style(PathStyle::Fill(
        FillOptions::default().with_fill_rule(FillRule::EvenOdd),
    ));
    assert!(
        matches!(&builder.style, PathStyle::Fill(opts) if opts.fill_rule == FillRule::EvenOdd),
        "the hole-punching fill rule must be set at the build site"
    );
    builder.move_to(viewport.origin);
    builder.line_to(point(viewport.right(), viewport.origin.y));
    builder.line_to(point(viewport.right(), viewport.bottom()));
    builder.line_to(point(viewport.origin.x, viewport.bottom()));
    builder.close();
    rounded_rect_path(&mut builder, cutout, px(8.));
    let path = builder.build().expect("even-odd mask path must build");
    assert!(!path.vertices.is_empty());
}
