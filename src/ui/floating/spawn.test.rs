use std::rc::Rc;

use gpui_kit::{
    AnyView, App, AppContext as _, Bounds, Context, IntoElement, Render, TestAppContext, Window,
    div, point, px, size,
};

use super::*;
use crate::ui::floating::{Placement9, init};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn blank_view(_: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_| Blank).into()
}

#[gpui_kit::test]
async fn spawns_a_screen_centered_native_float(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(
                    FloatingWindowId("screen-center".into()),
                    size(px(200.), px(120.)),
                ),
                blank_view,
                cx,
            )
        })
        .unwrap();
    assert!(matches!(handle, FloatingHandle::Native(_)));
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0, "screen-center");
    assert_eq!(snapshot[0].title, "screen-center");
    assert_eq!(
        snapshot[0].bounds,
        Bounds {
            origin: point(px(860.), px(480.)),
            size: size(px(200.), px(120.)),
        }
    );
}

#[gpui_kit::test]
async fn spawns_a_window_anchored_float_at_the_anchor_placement(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(FloatingWindowId("anchored".into()), size(px(100.), px(80.)))
                    .anchor(FloatAnchor::Window {
                        window: window.into(),
                        placement: Placement9::TopRight,
                        offset: point(px(8.), px(8.)),
                    }),
                blank_view,
                cx,
            )
        })
        .unwrap();
    assert!(matches!(handle, FloatingHandle::Native(_)));
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].bounds.origin, point(px(708.), px(8.)));
}

#[gpui_kit::test]
async fn close_floating_closes_once_and_empties_the_registry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(FloatingWindowId("closed".into()), size(px(120.), px(90.))),
                blank_view,
                cx,
            )
        })
        .unwrap();
    let first = cx.update(|cx| close_floating(&handle, cx));
    assert!(first);
    assert!(cx.read(|cx| floating_snapshot(cx)).is_empty());
    let second = cx.update(|cx| close_floating(&handle, cx));
    assert!(!second);
}

#[gpui_kit::test]
async fn close_all_floating_returns_the_prior_count_and_clears_the_registry(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    cx.update(|cx| {
        for id in ["one", "two", "three"] {
            spawn_floating(
                FloatingSpec::new(FloatingWindowId(id.into()), size(px(120.), px(90.))),
                blank_view,
                cx,
            )
            .unwrap();
        }
    });
    let closed = cx.update(close_all_floating);
    assert_eq!(closed, 3);
    assert!(cx.read(|cx| floating_snapshot(cx)).is_empty());
}

#[gpui_kit::test]
async fn respawn_floating_reopens_a_known_id_and_rejects_unknown_ones(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    let id = FloatingWindowId("respawn".into());
    cx.update(|cx| {
        spawn_floating(
            FloatingSpec::new(id.clone(), size(px(120.), px(90.))),
            blank_view,
            cx,
        )
        .unwrap();
    });
    let respawned = cx.update(|cx| respawn_floating(&id, cx));
    assert!(respawned);
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot.len(), 1, "respawn must not duplicate the entry");
    assert_eq!(snapshot[0].id.0, "respawn");
    let unknown = cx.update(|cx| respawn_floating(&FloatingWindowId("missing".into()), cx));
    assert!(!unknown);
}

#[gpui_kit::test]
async fn window_close_tracker_removes_native_entries(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(FloatingWindowId("tracked".into()), size(px(120.), px(90.))),
                blank_view,
                cx,
            )
        })
        .unwrap();
    let FloatingHandle::Native(window) = handle else {
        panic!("expected a native handle");
    };
    cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert!(cx.read(|cx| floating_snapshot(cx)).is_empty());
}

#[gpui_kit::test]
async fn spawning_with_click_through_succeeds_on_the_test_platform(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(
                    FloatingWindowId("pass-through".into()),
                    size(px(120.), px(90.)),
                )
                .click_through(true),
                blank_view,
                cx,
            )
        })
        .unwrap();
    assert!(matches!(handle, FloatingHandle::Native(_)));
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0, "pass-through");
}

#[gpui_kit::test]
async fn runtime_toggles_return_false_on_the_test_platform(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(FloatingWindowId("toggled".into()), size(px(120.), px(90.))),
                blank_view,
                cx,
            )
        })
        .unwrap();
    let click_through = cx.update(|cx| set_floating_click_through(&handle, true, cx));
    assert!(!click_through);
    let topmost = cx.update(|cx| set_floating_always_on_top(&handle, true, cx));
    assert!(!topmost);
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot[0].level, FloatLevel::Floating);
}

#[gpui_kit::test]
async fn runtime_toggles_return_false_for_overlay_handles(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let overlay = FloatingHandle::Overlay(FloatingWindowId("overlay".into()));
    cx.update(|cx| {
        set_entry(
            RegistryEntry {
                spec: FloatingSpec::new(
                    FloatingWindowId("overlay".into()),
                    size(px(120.), px(90.)),
                ),
                bounds: Bounds {
                    origin: point(px(0.), px(0.)),
                    size: size(px(120.), px(90.)),
                },
                handle: overlay.clone(),
                build: Rc::new(|_, cx: &mut App| -> AnyView { cx.new(|_| Blank).into() }),
                view: None,
            },
            cx,
        );
    });
    assert!(!cx.update(|cx| set_floating_click_through(&overlay, true, cx)));
    assert!(!cx.update(|cx| set_floating_always_on_top(&overlay, true, cx)));
}

#[gpui_kit::test]
async fn topmost_toggle_tolerates_a_closed_native_handle(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(FloatingWindowId("gone".into()), size(px(120.), px(90.))),
                blank_view,
                cx,
            )
        })
        .unwrap();
    cx.update(|cx| close_floating(&handle, cx));
    assert!(!cx.update(|cx| set_floating_always_on_top(&handle, true, cx)));
    assert!(!cx.update(|cx| set_floating_click_through(&handle, true, cx)));
}
