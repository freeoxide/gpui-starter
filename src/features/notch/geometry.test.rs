use gpui_kit::{Bounds, point, px, size};

use super::notch_layout;

fn screen(width: f32, height: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds::new(point(px(0.), px(0.)), size(px(width), px(height)))
}

#[test]
fn notched_display_places_pill_over_housing() {
    let housing = Bounds::new(point(px(656.), px(0.)), size(px(200.), px(37.)));
    let layout = notch_layout(screen(1512., 982.), Some(px(37.)), Some(housing));
    assert_eq!(layout.collapsed.origin, point(px(656.), px(0.)));
    assert_eq!(layout.collapsed.size, size(px(200.), px(37.)));
    assert_eq!(layout.expanded.origin, point(px(396.), px(0.)));
    assert_eq!(layout.expanded.size, size(px(720.), px(200.)));
}

#[test]
fn housing_width_clamps_to_screen_share() {
    let wide = Bounds::new(point(px(156.), px(0.)), size(px(1200.), px(37.)));
    let clamped_high = notch_layout(screen(1512., 982.), Some(px(37.)), Some(wide));
    assert!(
        (clamped_high.collapsed.size.width.as_f32() - 907.2).abs() < 0.05,
        "housing wider than 60% of the screen clamps to the screen share"
    );

    let narrow = Bounds::new(point(px(706.), px(0.)), size(px(100.), px(37.)));
    let clamped_low = notch_layout(screen(1512., 982.), Some(px(37.)), Some(narrow));
    assert_eq!(clamped_low.collapsed.size.width, px(160.));
}

#[test]
fn defaults_without_probe_data() {
    let layout = notch_layout(screen(1512., 982.), None, None);
    assert_eq!(layout.collapsed.size.height, px(24.));
    assert_eq!(layout.collapsed.size.width, px(240.));
}

#[test]
fn narrow_and_short_screens_clamp_expanded() {
    let narrow = notch_layout(screen(500., 982.), Some(px(24.)), None);
    assert_eq!(narrow.expanded.size.width, px(450.));

    let short = notch_layout(screen(1512., 100.), None, None);
    assert_eq!(short.expanded.size.height, px(76.));

    let tiny = notch_layout(screen(1512., 30.), None, None);
    assert_eq!(tiny.expanded.size.height, tiny.collapsed.size.height);
}
