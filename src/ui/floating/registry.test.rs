use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    AnyView, AnyWindowHandle, App, AppContext as _, Bounds, Context, IntoElement, Render,
    TestAppContext, Window, div, point, px, size,
};

use super::*;

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn factory() -> ViewFactory {
    Rc::new(|_, cx: &mut App| -> AnyView { cx.new(|_| Blank).into() })
}

fn overlay_entry(id: &str, x: f32) -> RegistryEntry {
    RegistryEntry {
        spec: FloatingSpec::new(FloatingWindowId(id.into()), size(px(120.), px(80.))),
        bounds: Bounds {
            origin: point(px(x), px(0.)),
            size: size(px(120.), px(80.)),
        },
        handle: FloatingHandle::Overlay(FloatingWindowId(id.into())),
        build: factory(),
        view: None,
        surface: FloatSurface::Window,
    }
}

#[gpui_kit::test]
async fn set_entry_lands_in_the_snapshot(cx: &TestAppContext) {
    cx.update(|cx| set_entry(overlay_entry("alpha", 10.), cx));
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0, "alpha");
    assert_eq!(snapshot[0].title, "alpha");
    assert_eq!(snapshot[0].bounds.origin.x, px(10.));
    assert!(matches!(
        snapshot[0].handle,
        FloatingHandle::Overlay(ref id) if id.0 == "alpha"
    ));
    assert_eq!(snapshot[0].surface, FloatSurface::Window);
}

#[gpui_kit::test]
async fn remove_entry_empties_the_snapshot(cx: &TestAppContext) {
    let id = FloatingWindowId("beta".into());
    cx.update(|cx| set_entry(overlay_entry("beta", 0.), cx));
    let removed = cx.update(|cx| remove_entry(&id, cx));
    assert!(removed.is_some());
    assert!(cx.read(floating_snapshot).is_empty());
    assert!(cx.update(|cx| remove_entry(&id, cx)).is_none());
}

#[gpui_kit::test]
async fn registry_mutation_notifies_global_observers(cx: &TestAppContext) {
    let notified = Rc::new(Cell::new(false));
    let observer_flag = notified.clone();
    cx.update(|cx| {
        cx.observe_global::<FloatingWindowRegistry>(move |_: &mut App| {
            observer_flag.set(true);
        })
        .detach();
        set_entry(overlay_entry("gamma", 0.), cx);
    });
    assert!(
        notified.get(),
        "default_global mutation must fire observe_global observers"
    );
}

#[gpui_kit::test]
async fn set_entry_level_rewrites_only_the_entry_matching_the_handle(cx: &mut TestAppContext) {
    let window_a: AnyWindowHandle = cx.open_window(size(px(100.), px(80.)), |_, _| Blank).into();
    let window_b: AnyWindowHandle = cx.open_window(size(px(100.), px(80.)), |_, _| Blank).into();
    let native_id = FloatingWindowId("level-native".into());
    let overlay_id = FloatingWindowId("level-overlay".into());
    cx.update(|cx| {
        set_entry(
            RegistryEntry {
                spec: FloatingSpec::new(native_id.clone(), size(px(120.), px(80.))),
                bounds: Bounds {
                    origin: point(px(0.), px(0.)),
                    size: size(px(120.), px(80.)),
                },
                handle: FloatingHandle::Native(window_a.clone()),
                build: factory(),
                view: None,
                surface: FloatSurface::Window,
            },
            cx,
        );
        set_entry(
            RegistryEntry {
                spec: FloatingSpec::new(overlay_id.clone(), size(px(120.), px(80.))),
                bounds: Bounds {
                    origin: point(px(0.), px(0.)),
                    size: size(px(120.), px(80.)),
                },
                handle: FloatingHandle::Overlay(overlay_id.clone()),
                build: factory(),
                view: None,
                surface: FloatSurface::Window,
            },
            cx,
        );
    });

    cx.update(|cx| {
        set_entry_level(
            &FloatingHandle::Native(window_b),
            FloatLevel::AlwaysOnTop,
            cx,
        );
        set_entry_level(
            &FloatingHandle::Overlay(FloatingWindowId("other".into())),
            FloatLevel::AlwaysOnTop,
            cx,
        );
    });
    assert!(
        cx.read(floating_snapshot)
            .iter()
            .all(|entry| entry.level == FloatLevel::Floating),
        "mismatched native and overlay handles must rewrite nothing"
    );

    cx.update(|cx| {
        set_entry_level(
            &FloatingHandle::Native(window_a),
            FloatLevel::AlwaysOnTop,
            cx,
        )
    });
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(
        snapshot
            .iter()
            .find(|entry| entry.id == native_id)
            .unwrap()
            .level,
        FloatLevel::AlwaysOnTop
    );
    assert_eq!(
        snapshot
            .iter()
            .find(|entry| entry.id == overlay_id)
            .unwrap()
            .level,
        FloatLevel::Floating,
        "a native match must not touch overlay entries"
    );

    cx.update(|cx| {
        set_entry_level(
            &FloatingHandle::Overlay(overlay_id.clone()),
            FloatLevel::AlwaysOnTop,
            cx,
        )
    });
    let snapshot = cx.read(floating_snapshot);
    assert_eq!(
        snapshot
            .iter()
            .find(|entry| entry.id == overlay_id)
            .unwrap()
            .level,
        FloatLevel::AlwaysOnTop
    );
}
