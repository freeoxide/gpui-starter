use std::rc::Rc;

use gpui_kit::component::v_flex;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyView, AnyWindowHandle, App, Bounds, Context, Entity, Render, TestAppContext, Window, div,
    point, px, size,
};

use super::super::spec::FloatingSpec;
use super::FloatingWindowId;
use super::OverlayFloatHost;
use super::registry::{self, FloatingHandle};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn insert_overlay_for_test(entry: registry::RegistryEntry, cx: &mut App) {
    registry::set_entry(entry, cx);
}

fn overlay_entry(id: &str, x: f32, view: AnyView) -> registry::RegistryEntry {
    registry::RegistryEntry {
        spec: FloatingSpec::new(FloatingWindowId(id.into()), size(px(120.), px(80.))),
        bounds: Bounds {
            origin: point(px(x), px(0.)),
            size: size(px(120.), px(80.)),
        },
        handle: FloatingHandle::Overlay(FloatingWindowId(id.into())),
        build: Rc::new(|_, cx: &mut App| -> AnyView { cx.new(|_| Blank).into() }),
        view: Some(view),
    }
}

struct HostAlone {
    host: Entity<OverlayFloatHost>,
}

impl Render for HostAlone {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.host.clone())
    }
}

struct HostInColumn {
    host: Entity<OverlayFloatHost>,
}

impl Render for HostInColumn {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(div().h_10().flex_shrink_0())
            .child(self.host.clone())
    }
}

fn draw_host(cx: &mut TestAppContext, window: AnyWindowHandle, host: &Entity<OverlayFloatHost>) {
    cx.update(|cx| {
        window
            .update(cx, |_, window, app| {
                host.update(app, |host, host_cx| {
                    let _frame = host.render(window, host_cx);
                });
            })
            .unwrap();
    });
}

#[gpui_kit::test]
async fn host_renders_the_same_pane_count_alone_and_inside_the_app_column(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let alone_window = cx.open_window(size(px(400.), px(300.)), |window, cx| {
        let host = cx.new(|cx| OverlayFloatHost::new(window, cx));
        HostAlone { host }
    });
    let alone_host = cx.update(|cx| {
        alone_window
            .read_with(cx, |view, _| view.host.clone())
            .unwrap()
    });

    let column_window = cx.open_window(size(px(400.), px(300.)), |window, cx| {
        let host = cx.new(|cx| OverlayFloatHost::new(window, cx));
        HostInColumn { host }
    });
    let column_host = cx.update(|cx| {
        column_window
            .read_with(cx, |view, _| view.host.clone())
            .unwrap()
    });

    assert!(cx.read(|cx| alone_host.read(cx).is_empty()));
    draw_host(cx, alone_window.into(), &alone_host);
    draw_host(cx, column_window.into(), &column_host);

    cx.update(|cx| {
        let view: AnyView = cx.new(|_| Blank).into();
        insert_overlay_for_test(overlay_entry("pane", 12., view), cx);
    });

    let alone_count = cx.read(|cx| registry::overlay_panes(cx).len());
    let column_count = cx.read(|cx| registry::overlay_panes(cx).len());
    draw_host(cx, alone_window.into(), &alone_host);
    draw_host(cx, column_window.into(), &column_host);
    assert_eq!(alone_count, column_count);
    assert_eq!(alone_count, 1);
    assert!(!cx.read(|cx| alone_host.read(cx).is_empty()));
    assert!(!cx.read(|cx| column_host.read(cx).is_empty()));

    cx.update(|cx| {
        registry::remove_entry(&FloatingWindowId("pane".into()), cx);
    });
    draw_host(cx, alone_window.into(), &alone_host);
    assert!(cx.read(|cx| alone_host.read(cx).is_empty()));
}
