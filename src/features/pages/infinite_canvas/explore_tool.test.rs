use gpui_kit::{
    AppContext as _, CursorStyle, Entity, Modifiers, MouseButton, Pixels, ScrollDelta,
    TestAppContext, point, px,
};

use crate::ui::canvas::{
    CanvasPointerEvent, CanvasScrollEvent, CanvasState, CanvasTransform, ToolContext,
};

use super::explore_tool::{
    ExploreTool, LABEL_LOD_ZOOM, LINE_PX, PAGE_MAX_ZOOM, PAGE_MIN_ZOOM, clamp_zoom, lines,
    page_factor,
};

#[test]
fn page_factor_is_identity_mid_range() {
    assert_eq!(page_factor(1.0, 1.25), 1.25);
    assert_eq!(page_factor(2.0, 0.5), 0.5);
}

#[test]
fn page_factor_is_a_no_op_at_both_clamps() {
    assert_eq!(page_factor(PAGE_MAX_ZOOM, 2.0), 1.0);
    assert_eq!(page_factor(PAGE_MIN_ZOOM, 0.5), 1.0);
}

#[test]
fn page_factor_lands_exactly_on_the_ceiling() {
    assert_eq!(page_factor(4.0, 4.0), 2.0);
}

#[test]
fn clamp_zoom_pins_products_that_cross_a_bound() {
    assert_eq!(clamp_zoom(16.0), PAGE_MAX_ZOOM);
    assert_eq!(clamp_zoom(0.01), PAGE_MIN_ZOOM);
    assert_eq!(clamp_zoom(1.0), 1.0);
}

#[test]
fn lines_converts_lines_and_pixels_to_line_units() {
    assert_eq!(lines(&ScrollDelta::Lines(point(0., 2.))), point(0., 2.));
    assert_eq!(
        lines(&ScrollDelta::Pixels(point(px(40.), px(20.)))),
        point(1., 0.5)
    );
}

#[test]
fn lod_and_line_constants_match_the_plan_budgets() {
    assert_eq!(LABEL_LOD_ZOOM, 0.5);
    assert_eq!(LINE_PX, 40.);
}

fn secondary() -> Modifiers {
    Modifiers {
        control: true,
        platform: true,
        ..Default::default()
    }
}

fn pointer(screen: (f32, f32)) -> CanvasPointerEvent {
    let screen_position = point(px(screen.0), px(screen.1));
    CanvasPointerEvent {
        doc_position: screen_position,
        screen_position,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::none(),
    }
}

fn tool_ctx(transform: CanvasTransform) -> ToolContext {
    ToolContext {
        viewport: gpui_kit::Bounds {
            origin: point(px(0.), px(0.)),
            size: gpui_kit::size(px(800.), px(600.)),
        },
        transform,
        hit: None,
    }
}

fn scroll_at(screen: (f32, f32), delta: ScrollDelta, modifiers: Modifiers) -> CanvasScrollEvent {
    CanvasScrollEvent {
        screen_position: point(px(screen.0), px(screen.1)),
        doc_position: point(px(screen.0), px(screen.1)),
        delta,
        modifiers,
    }
}

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[gpui_kit::test]
fn drag_sequence_pans_by_the_pointer_delta(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.set_tool(Box::new(ExploreTool::new()), cx));
    let ctx = tool_ctx(CanvasTransform::IDENTITY);
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| {
            tool.pointer_down(&pointer((10., 10.)), &ctx, s, cx)
        })
    });
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| {
            tool.pointer_move(&pointer((30., 20.)), &ctx, s, cx)
        })
    });
    let pan = cx.read(|cx| state.read(cx).pan());
    assert_px_eq(pan.x, px(20.));
    assert_px_eq(pan.y, px(10.));
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| {
            tool.pointer_up(&pointer((30., 20.)), &ctx, s, cx)
        })
    });
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| {
            tool.pointer_move(&pointer((90., 90.)), &ctx, s, cx)
        })
    });
    let pan = cx.read(|cx| state.read(cx).pan());
    assert_px_eq(pan.x, px(20.));
}

#[gpui_kit::test]
fn secondary_wheel_zooms_with_a_stable_anchor(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.set_tool(Box::new(ExploreTool::new()), cx));
    let anchor = point(px(300.), px(200.));
    let before = cx.read(|cx| {
        let s = state.read(cx);
        (s.zoom(), s.transform().to_doc(anchor))
    });
    let event = scroll_at((300., 200.), ScrollDelta::Lines(point(0., 2.)), secondary());
    let ctx = tool_ctx(CanvasTransform::IDENTITY);
    let consumed = state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| tool.scroll(&event, &ctx, s, cx))
    });
    assert!(consumed);
    let after = cx.read(|cx| {
        let s = state.read(cx);
        (s.zoom(), s.transform().to_doc(anchor))
    });
    assert_ne!(after.0, before.0);
    assert_px_eq(after.1.x, before.1.x);
    assert_px_eq(after.1.y, before.1.y);
}

#[gpui_kit::test]
fn secondary_wheel_at_the_ceiling_is_a_no_op(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.set_tool(Box::new(ExploreTool::new()), cx);
        s.set_zoom(PAGE_MAX_ZOOM, cx);
    });
    let before = cx.read(|cx| {
        let s = state.read(cx);
        (s.zoom(), s.pan())
    });
    let event = scroll_at(
        (300., 200.),
        ScrollDelta::Lines(point(0., -2.)),
        secondary(),
    );
    let ctx = tool_ctx(CanvasTransform::new(before.1, PAGE_MAX_ZOOM));
    let consumed = state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| tool.scroll(&event, &ctx, s, cx))
    });
    assert!(consumed);
    let after = cx.read(|cx| {
        let s = state.read(cx);
        (s.zoom(), s.pan())
    });
    assert_eq!(after.0, before.0);
    assert_px_eq(after.1.x, before.1.x);
    assert_px_eq(after.1.y, before.1.y);
}

#[gpui_kit::test]
fn plain_pixel_wheel_pans_by_the_negative_delta(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.set_tool(Box::new(ExploreTool::new()), cx));
    let event = scroll_at(
        (100., 100.),
        ScrollDelta::Pixels(point(px(10.), px(5.))),
        Modifiers::none(),
    );
    let ctx = tool_ctx(CanvasTransform::IDENTITY);
    let consumed = state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| tool.scroll(&event, &ctx, s, cx))
    });
    assert!(consumed);
    let pan = cx.read(|cx| state.read(cx).pan());
    assert_px_eq(pan.x, px(-10.));
    assert_px_eq(pan.y, px(-5.));
}

#[gpui_kit::test]
fn tool_reports_its_id_and_cursor(cx: &mut TestAppContext) {
    let state: Entity<CanvasState> = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.set_tool(Box::new(ExploreTool::new()), cx));
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "explore"));
    assert_eq!(
        cx.read(|cx| state.read(cx).active_tool_cursor()),
        CursorStyle::OpenHand
    );
}
