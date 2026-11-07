use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::component::v_flex;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyView, AnyWindowHandle, App, Bounds, Context, Element, ElementId, Entity, Focusable,
    GlobalElementId, InspectorElementId, LayoutId, Modifiers, Pixels, Render, Size, Style,
    TestAppContext, VisualTestContext, Window, div, point, px, size,
};

use super::OverlayFloatHost;
use super::registry::{self, FloatingHandle};
use crate::ui::floating::{FloatSurface, FloatingSpec, FloatingWindowId};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn overlay_entry(
    id: &str,
    x: f32,
    view: AnyView,
    chromeless: bool,
    focus: bool,
) -> registry::RegistryEntry {
    registry::RegistryEntry {
        spec: FloatingSpec::new(FloatingWindowId(id.into()), size(px(120.), px(80.)))
            .chromeless(chromeless)
            .focus(focus),
        bounds: Bounds {
            origin: point(px(x), px(0.)),
            size: size(px(120.), px(80.)),
        },
        handle: FloatingHandle::Overlay(FloatingWindowId(id.into())),
        build: Rc::new(|_, cx: &mut App| -> AnyView { cx.new(|_| Blank).into() }),
        view: Some(view),
        surface: FloatSurface::Window,
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

fn draw_window(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                window.draw(cx).clear(cx);
            })
            .unwrap();
    });
}

#[gpui_kit::test]
async fn panes_paint_at_the_same_absolute_bounds_alone_and_inside_the_app_column(
    cx: &mut TestAppContext,
) {
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
    let slot: ProbeSlot = Default::default();
    cx.update(|cx| {
        let view: AnyView = cx.new(|_| ProbeView { slot: slot.clone() }).into();
        registry::set_entry(overlay_entry("pane", 12., view, false, false), cx);
    });

    draw_window(cx, alone_window.into());
    let alone_bounds = *slot.borrow();
    draw_window(cx, column_window.into());
    let column_bounds = *slot.borrow();

    // One registry feeds both windows; equal absolute bounds in each prove the
    // app column cannot squash the host's absolute layer.
    assert_eq!(
        alone_bounds,
        Some(Bounds {
            origin: point(px(13.), px(33.)),
            size: size(px(118.), px(46.)),
        }),
        "alone: the body sits inside the pane's 1px border under the 32px header"
    );
    assert_eq!(column_bounds, alone_bounds);
    assert!(!cx.read(|cx| alone_host.read(cx).is_empty()));
    assert!(!cx.read(|cx| column_host.read(cx).is_empty()));

    cx.update(|cx| {
        registry::remove_entry(&FloatingWindowId("pane".into()), cx);
    });
    draw_window(cx, alone_window.into());
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
        registry::set_entry(overlay_entry("bare-pane", 0., bare_view, true, false), cx);
        registry::set_entry(
            overlay_entry("headed-pane", 140., headed_view, false, false),
            cx,
        );
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
        registry::set_entry(overlay_entry("bare-esc", 12., view, true, false), cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(72.), px(40.)), Modifiers::none());
    cx.simulate_keystrokes("escape");
    assert!(
        cx.read(registry::overlay_panes).is_empty(),
        "Esc must dismiss the chromeless pane"
    );
}

#[gpui_kit::test]
async fn a_focus_requested_pane_takes_the_keyboard_and_hands_it_back_on_close(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::floating::init);
    let window = cx.open_window(size(px(400.), px(300.)), |window, cx| {
        let host = cx.new(|cx| OverlayFloatHost::new(window, cx));
        HostAlone { host }
    });
    let host = cx.update(|cx| window.read_with(cx, |view, _| view.host.clone()).unwrap());
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        let view: AnyView = cx.new(|_| Blank).into();
        registry::set_entry(overlay_entry("keyboard-pane", 12., view, false, true), cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    cx.simulate_keystrokes("escape");
    assert!(
        cx.read(registry::overlay_panes).is_empty(),
        "a focus-requested pane must take the keyboard without a click first"
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let host_handle = cx.update(|_, cx| host.read(cx).focus_handle(cx));
    assert!(
        cx.update(|window, _| host_handle.is_focused(window)),
        "the host root must take the focus back so keystrokes never strand"
    );
}

#[gpui_kit::test]
async fn a_shrunk_window_pulls_stranded_panes_back_inside(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let slot: ProbeSlot = Default::default();
    let window = cx.open_window(size(px(400.), px(300.)), |window, cx| {
        let host = cx.new(|cx| OverlayFloatHost::new(window, cx));
        HostAlone { host }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|_, cx| {
        let view: AnyView = cx.new(|_| ProbeView { slot: slot.clone() }).into();
        registry::set_entry(overlay_entry("stranded", 200., view, true, false), cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        *slot.borrow(),
        Some(Bounds {
            origin: point(px(201.), px(1.)),
            size: size(px(118.), px(78.)),
        }),
        "a pane fully inside the window paints at its stored bounds"
    );

    cx.update(|window, cx| {
        window.resize(size(px(240.), px(300.)));
        window.bounds_changed(cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        *slot.borrow(),
        Some(Bounds {
            origin: point(px(121.), px(1.)),
            size: size(px(118.), px(78.)),
        }),
        "after the shrink the pane clamps to viewport width - pane width"
    );
}
