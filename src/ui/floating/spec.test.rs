use gpui_kit::{
    Bounds, Context, DisplayId, IntoElement, Pixels, Point, Render, TestAppContext, Window, div,
    point, px, size,
};

use super::*;

fn anchor() -> Bounds<Pixels> {
    Bounds {
        origin: point(px(100.), px(100.)),
        size: size(px(100.), px(100.)),
    }
}

fn clamp() -> Bounds<Pixels> {
    Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(1000.), px(1000.)),
    }
}

fn float_size() -> Size<Pixels> {
    size(px(40.), px(20.))
}

#[test]
fn resolves_all_nine_placements() {
    let cases: [(Placement9, f32, f32); 9] = [
        (Placement9::TopLeft, 100., 100.),
        (Placement9::Top, 130., 100.),
        (Placement9::TopRight, 160., 100.),
        (Placement9::Left, 100., 140.),
        (Placement9::Center, 130., 140.),
        (Placement9::Right, 160., 140.),
        (Placement9::BottomLeft, 100., 180.),
        (Placement9::Bottom, 130., 180.),
        (Placement9::BottomRight, 160., 180.),
    ];
    for (placement, x, y) in cases {
        let bounds = resolve_bounds(anchor(), clamp(), float_size(), placement, Point::default());
        assert_eq!(
            bounds,
            Bounds {
                origin: point(px(x), px(y)),
                size: float_size(),
            },
            "placement {placement:?}"
        );
    }
}

#[test]
fn offset_applies_after_placement() {
    let bounds = resolve_bounds(
        anchor(),
        clamp(),
        float_size(),
        Placement9::Center,
        point(px(10.), px(-5.)),
    );
    assert_eq!(bounds.origin, point(px(140.), px(135.)));
}

#[test]
fn clamps_origin_into_the_clamp_rect_at_each_edge() {
    let tight = Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(50.), px(50.)),
    };
    for placement in [
        Placement9::TopLeft,
        Placement9::Top,
        Placement9::TopRight,
        Placement9::Left,
        Placement9::Center,
        Placement9::Right,
        Placement9::BottomLeft,
        Placement9::Bottom,
        Placement9::BottomRight,
    ] {
        let bounds = resolve_bounds(tight, tight, float_size(), placement, Point::default());
        assert!(
            bounds.origin.x >= px(0.) && bounds.origin.x + bounds.size.width <= px(50.),
            "x clamped for {placement:?}"
        );
        assert!(
            bounds.origin.y >= px(0.) && bounds.origin.y + bounds.size.height <= px(50.),
            "y clamped for {placement:?}"
        );
    }
}

#[test]
fn centers_oversized_axes_in_the_clamp_rect() {
    let wide = size(px(1200.), px(30.));
    let bounds = resolve_bounds(
        anchor(),
        clamp(),
        wide,
        Placement9::TopLeft,
        Point::default(),
    );
    assert_eq!(bounds.origin, point(px(-100.), px(100.)));

    let tall = size(px(40.), px(1500.));
    let bounds = resolve_bounds(
        anchor(),
        clamp(),
        tall,
        Placement9::BottomRight,
        Point::default(),
    );
    assert_eq!(bounds.origin, point(px(160.), px(-250.)));
}

#[test]
fn spec_defaults_center_on_the_primary_display() {
    let spec = FloatingSpec::new(FloatingWindowId("demo".into()), float_size());
    assert_eq!(spec.id().0, "demo");
    assert_eq!(spec.size(), float_size());
    assert_eq!(spec.window_title(), "demo");
    assert_eq!(spec.float_level(), FloatLevel::Floating);
    assert!(matches!(
        spec.float_anchor(),
        FloatAnchor::Screen {
            display: None,
            placement: Placement9::Center,
            ..
        }
    ));
    assert!(!spec.is_focus());
    assert!(!spec.is_resizable());
    assert!(spec.is_movable());
    assert!(!spec.is_click_through());
    assert_eq!(spec.min_window_size(), None);
}

#[test]
fn title_builder_round_trips_the_chromeless_window_title() {
    let spec = FloatingSpec::new(FloatingWindowId("demo".into()), float_size()).title("Weather");
    assert_eq!(spec.window_title(), "Weather");
}

#[test]
fn movable_defaults_true_and_round_trips() {
    let spec = FloatingSpec::new(FloatingWindowId("demo".into()), float_size());
    assert!(
        spec.is_movable(),
        "drag-to-move is the widget drag affordance"
    );
    assert!(!spec.movable(false).is_movable());
}

#[test]
fn click_through_builder_round_trips() {
    let spec = FloatingSpec::new(FloatingWindowId("demo".into()), float_size()).click_through(true);
    assert!(spec.is_click_through());
    let spec = spec.click_through(false);
    assert!(!spec.is_click_through());
}

#[test]
fn layer_shell_builder_round_trips() {
    let spec = FloatingSpec::new(FloatingWindowId("demo".into()), float_size());
    assert!(spec.layer_shell_spec().is_none());
    let spec = spec.layer_shell(LayerShellSpec::new().namespace("widgets.local"));
    assert_eq!(
        spec.layer_shell_spec()
            .map(|ls| ls.requested_namespace().to_string()),
        Some("widgets.local".to_string())
    );
}

#[test]
fn set_level_rewrites_the_stored_level() {
    let mut spec = FloatingSpec::new(FloatingWindowId("demo".into()), float_size());
    assert_eq!(spec.level, FloatLevel::Floating);
    spec.set_level(FloatLevel::AlwaysOnTop);
    assert_eq!(spec.level, FloatLevel::AlwaysOnTop);
    assert_eq!(spec.float_level(), FloatLevel::AlwaysOnTop);
}

#[gpui_kit::test]
async fn anchor_rect_resolves_window_anchor_against_its_display(cx: &mut TestAppContext) {
    struct Blank;
    impl Render for Blank {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let resolved = cx.update(|cx| {
        anchor_rect(
            &FloatAnchor::Window {
                window: window.into(),
                placement: Placement9::TopRight,
                offset: Point::default(),
            },
            cx,
        )
    });
    let (anchor_bounds, clamp_bounds) = resolved.expect("window anchor resolves");
    assert_eq!(
        anchor_bounds,
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(800.), px(600.)),
        }
    );
    assert_eq!(
        clamp_bounds,
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(1920.), px(1080.)),
        }
    );
}

#[gpui_kit::test]
async fn anchor_rect_uses_display_visible_bounds_for_screen_anchor(cx: &TestAppContext) {
    let resolved = cx.update(|cx| {
        anchor_rect(
            &FloatAnchor::Screen {
                display: None,
                placement: Placement9::Center,
                offset: Point::default(),
            },
            cx,
        )
    });
    let (anchor_bounds, clamp_bounds) = resolved.expect("screen anchor resolves");
    assert_eq!(anchor_bounds, clamp_bounds);
}

#[gpui_kit::test]
async fn anchor_rect_returns_none_for_an_unknown_display(cx: &TestAppContext) {
    let resolved = cx.update(|cx| {
        anchor_rect(
            &FloatAnchor::Screen {
                display: Some(DisplayId::new(999)),
                placement: Placement9::Center,
                offset: Point::default(),
            },
            cx,
        )
    });
    assert!(resolved.is_none());
}

#[gpui_kit::test]
async fn anchor_rect_returns_none_while_the_anchor_window_is_mid_dispatch(cx: &mut TestAppContext) {
    struct Blank;
    impl Render for Blank {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    // gpui cannot re-enter a window whose dispatch is on the stack; the
    // spawn layer must bridge this (defer past the cycle), not anchor_rect.
    let resolved = cx.update(|cx| {
        window.update(cx, |_, _, cx| {
            anchor_rect(
                &FloatAnchor::Window {
                    window: window.into(),
                    placement: Placement9::Center,
                    offset: Point::default(),
                },
                cx,
            )
        })
    });
    assert!(
        matches!(resolved, Ok(None)),
        "mid-dispatch resolution must report None, not error"
    );
}

#[gpui_kit::test]
async fn window_is_open_stays_true_while_dispatching_and_false_after_close(
    cx: &mut TestAppContext,
) {
    struct Blank;
    impl Render for Blank {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let handle: gpui_kit::AnyWindowHandle = window.into();
    let mid_dispatch = cx.update(|cx| {
        handle
            .update(cx, |_, _, cx| window_is_open(&handle, cx))
            .unwrap()
    });
    assert!(mid_dispatch, "the dispatch lease must not read as closed");
    cx.update(|cx| {
        handle
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert!(!cx.update(|cx| window_is_open(&handle, cx)));
}
