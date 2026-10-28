// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, point, px,
    size,
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

fn rect_doc_bounds() -> Bounds<gpui_kit::Pixels> {
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

fn assert_px_eq(actual: gpui_kit::Pixels, expected: gpui_kit::Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
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
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[shape_id]));
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
    let [path_builds, key_hashes, _] = perf_probe::snapshot();
    assert!(
        path_builds + key_hashes >= 1,
        "overlay generation must not satisfy a freed doc slot's stored generation: \
         {path_builds} builds / {key_hashes} hashes"
    );

    // Every later overlay mutation must still revalidate its own slot.
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
        let [path_builds, key_hashes, _] = perf_probe::snapshot();
        assert!(
            path_builds + key_hashes >= 1,
            "overlay paint must revalidate the slot it lands on, got \
             {path_builds} builds / {key_hashes} hashes"
        );
    }
    state.update(&mut cx, |s, cx| s.set_tool_overlay(None, cx));
}
