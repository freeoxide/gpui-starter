use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    AnyView, App, AppContext as _, Bounds, Context, IntoElement, Render, TestAppContext, Window,
    div, point, px, size,
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
    }
}

#[gpui_kit::test]
async fn set_entry_lands_in_the_snapshot(cx: &TestAppContext) {
    cx.update(|cx| set_entry(overlay_entry("alpha", 10.), cx));
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0, "alpha");
    assert_eq!(snapshot[0].title, "alpha");
    assert_eq!(snapshot[0].bounds.origin.x, px(10.));
    assert!(matches!(
        snapshot[0].handle,
        FloatingHandle::Overlay(ref id) if id.0 == "alpha"
    ));
}

#[gpui_kit::test]
async fn remove_entry_empties_the_snapshot(cx: &TestAppContext) {
    let id = FloatingWindowId("beta".into());
    cx.update(|cx| set_entry(overlay_entry("beta", 0.), cx));
    let removed = cx.update(|cx| remove_entry(&id, cx));
    assert!(removed.is_some());
    assert!(cx.read(|cx| floating_snapshot(cx)).is_empty());
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
