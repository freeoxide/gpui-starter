use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::component::v_flex;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyView, AnyWindowHandle, App, Bounds, Context, Element, ElementId, Entity, GlobalElementId,
    InspectorElementId, LayoutId, Modifiers, Pixels, Render, Size, Style, TestAppContext,
    VisualTestContext, Window, div, point, px, size,
};

use super::super::spec::FloatingSpec;
use super::FloatingWindowId;
use super::OverlayFloatHost;
use super::registry::{self, FloatingHandle};
use super::set_pane_chromeless;

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
        surface: super::super::FloatSurface::Window,
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

type ProbeSlot = Rc<RefCell<Option<Bounds<Pixels>>>>;

/// Records the bounds layout hands it, so tests can assert where the pane
/// places the body without reading pixels.
struct BoundsProbe {
    slot: ProbeSlot,
}

impl IntoElement for BoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for BoundsProbe {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            ..Default::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        _: &mut Window,
        _: &mut App,
    ) {
        *self.slot.borrow_mut() = Some(bounds);
    }
}

struct ProbeView {
    slot: ProbeSlot,
}

impl Render for ProbeView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        BoundsProbe {
            slot: self.slot.clone(),
        }
    }
}

#[gpui_kit::test]
async fn a_chromeless_overlay_pane_hands_the_pane_to_the_body(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let bare_slot: ProbeSlot = Default::default();
    let headed_slot: ProbeSlot = Default::default();
    let window = cx.open_window(size(px(400.), px(300.)), |window, cx| {
        let host = cx.new(|cx| OverlayFloatHost::new(window, cx));
        HostAlone { host }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        let bare_view: AnyView = cx
            .new(|_| ProbeView {
                slot: bare_slot.clone(),
            })
            .into();
        let headed_view: AnyView = cx
            .new(|_| ProbeView {
                slot: headed_slot.clone(),
            })
            .into();
        insert_overlay_for_test(overlay_entry("bare-pane", 0., bare_view), cx);
        insert_overlay_for_test(overlay_entry("headed-pane", 140., headed_view), cx);
        set_pane_chromeless(&FloatingWindowId("bare-pane".into()), true, cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));

    assert_eq!(
        *bare_slot.borrow(),
        Some(Bounds {
            origin: point(px(1.), px(1.)),
            size: size(px(118.), px(78.)),
        }),
        "chromeless pane: only the 1px border insets the body, no header strip"
    );
    let headed_bounds = headed_slot
        .borrow()
        .expect("the headed pane paints its body");
    assert!(
        headed_bounds.origin.y > px(16.) && headed_bounds.origin.x > px(140.),
        "the header strip still offsets the body in a headed pane"
    );
}

#[gpui_kit::test]
async fn escape_dismisses_a_chromeless_overlay_pane(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::floating::init);
    let window = cx.open_window(size(px(400.), px(300.)), |window, cx| {
        let host = cx.new(|cx| OverlayFloatHost::new(window, cx));
        HostAlone { host }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        let view: AnyView = cx.new(|_| Blank).into();
        insert_overlay_for_test(overlay_entry("bare-esc", 12., view), cx);
        set_pane_chromeless(&FloatingWindowId("bare-esc".into()), true, cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(72.), px(40.)), Modifiers::none());
    cx.simulate_keystrokes("escape");
    assert!(
        cx.read(|cx| registry::overlay_panes(cx)).is_empty(),
        "Esc must dismiss the chromeless pane"
    );
}
