// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, Pixels, ScrollDelta, ScrollEvent, TestAppContext,
    VisualTestContext, Window, black, point, px, size,
};

use crate::ui::canvas::{BuiltinTool, Canvas, CanvasShape, CanvasState, ShapeId, ShapeStyle};

struct CanvasHost {
    state: Entity<CanvasState>,
}

impl CanvasHost {
    fn new(state: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self { state }
    }
}

impl Render for CanvasHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Canvas::new("canvas", self.state.clone())
    }
}

fn mounted_host(cx: &mut TestAppContext) -> (Entity<CanvasState>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let state = cx.new(|_| CanvasState::new());
    let handle = cx.open_window(size(px(600.), px(400.)), {
        let state = state.clone();
        |_, cx| CanvasHost::new(state, cx)
    });
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (state, cx)
}

fn rect_doc_bounds() -> Bounds<Pixels> {
    Bounds {
        origin: point(px(100.), px(100.)),
        size: size(px(100.), px(100.)),
    }
}

fn seed_rect(cx: &mut VisualTestContext, state: &Entity<CanvasState>) -> ShapeId {
    state.update(cx, |s, cx| {
        let layer = s.add_layer(cx);
        s.add_shape(
            layer,
            CanvasShape::rect(rect_doc_bounds(), ShapeStyle::default()),
            cx,
        )
        .unwrap()
    })
}

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

/// Reads the newest stroke path the canvas cache built. Element state is only
/// reachable mid-paint, so the perf probe mirrors the built path for tests.
fn last_built_path() -> Option<gpui_kit::Path<Pixels>> {
    crate::ui::canvas::perf_probe::last_path_build()
}

#[gpui_kit::test]
fn pan_tool_drag_through_real_window_events(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| s.set_builtin_tool(BuiltinTool::Pan, cx));
    cx.simulate_mouse_down(
        point(px(300.), px(200.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(320.), px(210.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        point(px(320.), px(210.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    let pan = cx.read(|cx| state.read(cx).pan());
    assert_px_eq(pan.x, px(20.));
    assert_px_eq(pan.y, px(10.));
}

#[gpui_kit::test]
fn select_and_move_through_real_window_events(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    let shape_id = seed_rect(&mut cx, &state);
    state.update(&mut cx, |s, cx| s.set_builtin_tool(BuiltinTool::Select, cx));
    cx.simulate_click(point(px(150.), px(150.)), Modifiers::none());
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == [shape_id]));
    cx.simulate_mouse_down(
        point(px(150.), px(150.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(180.), px(150.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        point(px(180.), px(150.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    let origin = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shape(shape_id)
            .unwrap()
            .doc_bounds()
            .origin
    });
    assert_px_eq(origin.x, px(130.));
    assert_px_eq(origin.y, px(100.));
}

#[gpui_kit::test]
fn brush_stroke_through_real_window_events(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| {
        s.set_builtin_tool(BuiltinTool::Brush(Default::default()), cx)
    });
    cx.simulate_mouse_down(
        point(px(100.), px(100.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(150.), px(120.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(200.), px(140.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_some()));
    cx.simulate_mouse_up(
        point(px(200.), px(140.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    let (path_count, point_count, has_overlay) = cx.read(|cx| {
        let s = state.read(cx);
        let mut path_count = 0;
        let mut point_count = 0;
        for (_, _, shape) in s.document().shapes() {
            if let CanvasShape::Path { points, .. } = shape {
                path_count += 1;
                point_count = points.len();
            }
        }
        (path_count, point_count, s.tool_overlay().is_some())
    });
    assert_eq!(path_count, 1);
    assert_eq!(point_count, 3);
    assert!(!has_overlay);
}

/// A shape-count drop frees doc slots the overlay then lands on; its
/// generation must never equal the freed slot's stored doc generation.
#[gpui_kit::test]
fn overlay_revalidates_slots_freed_by_a_removed_shape(cx: &mut TestAppContext) {
    use crate::ui::canvas::perf_probe;

    let (state, mut cx) = mounted_host(cx);
    let (_first, second) = state.update(&mut cx, |s, cx| {
        let layer = s.add_layer(cx);
        let stroke = ShapeStyle::stroked(crate::ui::canvas::StrokeStyle::default());
        let first = s
            .add_shape(
                layer,
                CanvasShape::rect(
                    Bounds {
                        origin: point(px(50.), px(50.)),
                        size: size(px(100.), px(100.)),
                    },
                    stroke.clone(),
                ),
                cx,
            )
            .unwrap();
        let second = s
            .add_shape(
                layer,
                CanvasShape::rect(
                    Bounds {
                        origin: point(px(250.), px(50.)),
                        size: size(px(100.), px(100.)),
                    },
                    stroke,
                ),
                cx,
            )
            .unwrap();
        (first, second)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    // Freed slot 2 (the removed shape's) is exactly where the next overlay paints.
    state.update(&mut cx, |s, cx| {
        s.remove_shape(second, cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));

    // Aliasing repro: bump the overlay revision to the freed slot's stored
    // one (3) inside ONE update (flushes paint per update), after the reset.
    perf_probe::reset();
    state.update(&mut cx, |s, cx| {
        for _ in 0..3 {
            s.set_tool_overlay(
                Some(CanvasShape::rect(
                    Bounds {
                        origin: point(px(20.), px(20.)),
                        size: size(px(5.), px(5.)),
                    },
                    ShapeStyle::stroked(crate::ui::canvas::StrokeStyle::default()),
                )),
                cx,
            );
        }
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [path_builds, key_hashes, _, _] = perf_probe::snapshot();
    assert!(
        path_builds >= 1,
        "the overlay's content differs from the freed slot's stored shape, so \
         only a rebuild can paint it: {path_builds} builds / {key_hashes} hashes"
    );

    for _ in 0..3 {
        perf_probe::reset();
        state.update(&mut cx, |s, cx| {
            s.set_tool_overlay(
                Some(CanvasShape::rect(
                    Bounds {
                        origin: point(px(25.), px(25.)),
                        size: size(px(5.), px(5.)),
                    },
                    ShapeStyle::stroked(crate::ui::canvas::StrokeStyle::default()),
                )),
                cx,
            )
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let [path_builds, key_hashes, _, _] = perf_probe::snapshot();
        assert!(
            path_builds + key_hashes >= 1,
            "overlay paint must revalidate the slot it lands on, got \
             {path_builds} builds / {key_hashes} hashes"
        );
    }
    state.update(&mut cx, |s, cx| s.set_tool_overlay(None, cx));
}

#[gpui_kit::test]
fn offscreen_shapes_cull_before_the_path_cache(cx: &mut TestAppContext) {
    use crate::ui::canvas::perf_probe;

    let (state, mut cx) = mounted_host(cx);
    let shape_id = state.update(&mut cx, |s, cx| {
        let layer = s.add_layer(cx);
        let stroke = ShapeStyle::stroked(crate::ui::canvas::StrokeStyle::default());
        s.add_shape(layer, CanvasShape::rect(rect_doc_bounds(), stroke), cx)
            .unwrap()
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));

    // Content changed while off-screen; only cull-before-cache keeps both counters at zero.
    perf_probe::reset();
    state.update(&mut cx, |s, cx| {
        s.pan_by(point(px(1000.), px(0.)), cx);
        s.edit_document(cx, |doc| {
            doc.update_shape(shape_id, |shape| shape.translate(point(px(5.), px(0.))))
        });
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [builds, hashes, _, _] = perf_probe::snapshot();
    assert_eq!(builds, 0, "off-screen paint must not build a path");
    assert_eq!(hashes, 0, "off-screen paint must not rehash the key");

    perf_probe::reset();
    state.update(&mut cx, |s, cx| s.pan_by(point(px(-1000.), px(0.)), cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [builds, hashes, _, _] = perf_probe::snapshot();
    assert!(builds >= 1, "a revealed, changed shape must rebuild");
    assert!(hashes >= 1, "a revealed, changed shape must rehash its key");
}

#[gpui_kit::test]
fn path_vertices_scale_by_zoom_around_the_doc_bounds_origin(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| {
        let layer = s.add_layer(cx);
        let stroke = crate::ui::canvas::StrokeStyle {
            width: px(2.),
            ..Default::default()
        };
        s.add_shape(
            layer,
            CanvasShape::path(
                vec![
                    point(px(10.), px(10.)),
                    point(px(60.), px(10.)),
                    point(px(60.), px(60.)),
                ],
                stroke,
            ),
            cx,
        )
        .unwrap();
    });
    state.update(&mut cx, |s, cx| s.set_zoom(2., cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let path = last_built_path().expect("scaled stroke built");
    // Doc points (10,10)-(60,60) minus the half-stroke origin (9,9), times zoom 2,
    // land at (2,2)-(102,102); the 4px stroke and its joins add a few px at most.
    let b = path.bounds;
    // Butt caps end exactly on the centerline and contains() excludes the far
    // edge, so the hull gets float-noise slack.
    let slack = px(0.05);
    let hull = Bounds {
        origin: b.origin - point(slack, slack),
        size: b.size + size(slack * 2., slack * 2.),
    };
    for corner in [
        point(px(2.), px(2.)),
        point(px(102.), px(2.)),
        point(px(2.), px(102.)),
        point(px(102.), px(102.)),
    ] {
        assert!(
            hull.contains(&corner),
            "centerline corner {corner:?} outside {b:?}"
        );
    }
    assert!((100. ..=112.).contains(&b.size.width.as_f32()), "{b:?}");
    assert!((100. ..=112.).contains(&b.size.height.as_f32()), "{b:?}");
}

#[gpui_kit::test]
fn single_point_paths_paint_dots_and_cull_offscreen(cx: &mut TestAppContext) {
    use crate::ui::canvas::perf_probe;

    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| {
        let layer = s.add_layer(cx);
        s.add_shape(
            layer,
            CanvasShape::path(
                vec![point(px(50.), px(50.))],
                crate::ui::canvas::StrokeStyle {
                    width: px(6.),
                    ..Default::default()
                },
            ),
            cx,
        )
        .unwrap();
    });
    // The update above flushed its own paint; count from this draw on.
    perf_probe::reset();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [_, _, _, dots] = perf_probe::snapshot();
    assert_eq!(dots, 1, "a one-point path must paint its dot");

    perf_probe::reset();
    state.update(&mut cx, |s, cx| s.pan_by(point(px(5000.), px(0.)), cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [_, _, _, dots] = perf_probe::snapshot();
    assert_eq!(dots, 0, "an off-screen dot must cull");
}

#[gpui_kit::test]
fn text_shapes_shape_once_per_content_change_not_per_frame_or_pan(cx: &mut TestAppContext) {
    use crate::ui::canvas::perf_probe;

    let (state, mut cx) = mounted_host(cx);
    let shape_id = state.update(&mut cx, |s, cx| {
        let layer = s.add_layer(cx);
        s.add_shape(
            layer,
            CanvasShape::text(
                point(px(50.), px(50.)),
                "hello",
                px(10.),
                black(),
                size(px(28.), px(12.)),
            ),
            cx,
        )
        .unwrap()
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [_, _, shapings, _] = perf_probe::snapshot();
    assert_eq!(shapings, 1);

    cx.update(|window, cx| window.draw(cx).clear(cx));
    state.update(&mut cx, |s, cx| s.pan_by(point(px(30.), px(0.)), cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [_, _, shapings, _] = perf_probe::snapshot();
    assert_eq!(shapings, 1, "repaints and pans must reuse the shaped line");

    state.update(&mut cx, |s, cx| {
        s.edit_document(cx, |doc| {
            doc.update_shape(shape_id, |shape| shape.translate(point(px(5.), px(0.))))
        })
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let [_, _, shapings, _] = perf_probe::snapshot();
    assert_eq!(shapings, 2, "moved text must reshape exactly once");
}

#[gpui_kit::test]
fn wheel_events_pan_the_canvas_through_the_element(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| s.set_builtin_tool(BuiltinTool::Pan, cx));
    let at = point(px(300.), px(200.));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_event(ScrollEvent {
        position: at,
        delta: ScrollDelta::Lines(point(0., 2.)),
        ..Default::default()
    });
    let pan = cx.read(|cx| state.read(cx).pan());
    assert_px_eq(pan.x, px(0.));
    assert_px_eq(pan.y, px(-80.));

    cx.simulate_event(ScrollEvent {
        position: at,
        delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
        ..Default::default()
    });
    let pan = cx.read(|cx| state.read(cx).pan());
    assert_px_eq(pan.y, px(-40.));
}

#[gpui_kit::test]
fn ctrl_wheel_zooms_at_the_cursor_position(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| s.set_builtin_tool(BuiltinTool::Pan, cx));
    let at = point(px(300.), px(200.));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_event(ScrollEvent {
        position: at,
        delta: ScrollDelta::Lines(point(0., 2.)),
        modifiers: Modifiers::control(),
        ..Default::default()
    });
    let (zoom, doc_at_anchor) = cx.read(|cx| {
        let s = state.read(cx);
        (s.zoom(), s.transform().to_doc(at))
    });
    assert!((zoom - (-0.2f32).exp()).abs() < 0.001, "zoom {zoom}");
    assert_px_eq(doc_at_anchor.x, px(300.));
    assert_px_eq(doc_at_anchor.y, px(200.));

    // Pixels deltas convert through the same /40 line scale: 80px = 2 lines.
    cx.simulate_event(ScrollEvent {
        position: at,
        delta: ScrollDelta::Pixels(point(px(0.), px(80.))),
        modifiers: Modifiers::control(),
        ..Default::default()
    });
    let zoom = cx.read(|cx| state.read(cx).zoom());
    let expected = (-0.2f32).exp().powi(2);
    assert!((zoom - expected).abs() < 0.001, "zoom {zoom}");
}

#[gpui_kit::test]
fn a_chorded_right_up_does_not_end_an_in_flight_left_stroke(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| {
        s.set_builtin_tool(BuiltinTool::Brush(Default::default()), cx)
    });
    cx.simulate_mouse_down(
        point(px(100.), px(100.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(130.), px(100.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        point(px(130.), px(100.)),
        MouseButton::Right,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(160.), px(100.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    let (committed, overlay_points) = cx.read(|cx| {
        let s = state.read(cx);
        let committed = s
            .document()
            .shapes()
            .filter(|(_, _, shape)| matches!(shape, CanvasShape::Path { .. }))
            .count();
        let overlay_points = s.tool_overlay().and_then(|shape| match shape {
            CanvasShape::Path { points, .. } => Some(points.len()),
            _ => None,
        });
        (committed, overlay_points)
    });
    assert_eq!(committed, 0, "right-up must not commit a left stroke");
    assert_eq!(overlay_points, Some(3), "the left drag must still extend");
    cx.simulate_mouse_up(
        point(px(160.), px(100.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    let committed = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shapes()
            .filter(|(_, _, shape)| matches!(shape, CanvasShape::Path { .. }))
            .count()
    });
    assert_eq!(committed, 1);
}

#[gpui_kit::test]
fn a_move_with_no_button_held_ends_an_orphaned_gesture(cx: &mut TestAppContext) {
    let (state, mut cx) = mounted_host(cx);
    state.update(&mut cx, |s, cx| {
        s.set_builtin_tool(BuiltinTool::Brush(Default::default()), cx)
    });
    cx.simulate_mouse_down(
        point(px(100.), px(100.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(120.), px(110.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    // The up was consumed above the canvas; the next buttonless move must
    // commit the stroke instead of extending it with no button held.
    cx.simulate_mouse_move(point(px(140.), px(130.)), None, Modifiers::none());
    let (committed, has_overlay, pointer_active) = cx.read(|cx| {
        let s = state.read(cx);
        let committed = s
            .document()
            .shapes()
            .filter(|(_, _, shape)| matches!(shape, CanvasShape::Path { .. }))
            .count();
        (committed, s.tool_overlay().is_some(), s.is_pointer_active())
    });
    assert_eq!(committed, 1);
    assert!(!has_overlay);
    assert!(!pointer_active);
}
