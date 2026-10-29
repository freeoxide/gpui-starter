use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyView, App, Bounds, Context, Element, ElementId, GlobalElementId, InspectorElementId,
    LayoutId, Modifiers, Pixels, Render, Size, Style, TestAppContext, VisualTestContext, Window,
    div, point, px, size,
};

use super::FloatingShell;
use crate::ui::floating::{
    FloatingHandle, FloatingSpec, FloatingWindowId, floating_snapshot, init, spawn_floating,
};

type ProbeSlot = Rc<RefCell<Option<Bounds<Pixels>>>>;

/// Records the bounds layout hands it, so tests can assert where the shell
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

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct ClickBody {
    clicks: Rc<std::cell::Cell<u32>>,
}

impl Render for ClickBody {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        Button::new("probe")
            .label("Probe target")
            .on_click(move |_, _, _| {
                clicks.set(clicks.get() + 1);
            })
    }
}

fn open_shell_window(
    id: &str,
    chromeless: bool,
    slot: ProbeSlot,
    cx: &mut TestAppContext,
) -> VisualTestContext {
    let window = cx.open_window(size(px(200.), px(120.)), |_, cx| {
        let content: AnyView = cx.new(|_| ProbeView { slot }).into();
        FloatingShell::new(
            FloatingWindowId(id.into()),
            id.into(),
            content,
            chromeless,
            cx,
        )
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx
}

#[gpui_kit::test]
async fn a_chromeless_shell_hands_the_whole_window_to_the_body(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let bare_slot: ProbeSlot = Default::default();
    let headed_slot: ProbeSlot = Default::default();
    let mut bare = open_shell_window("bare", true, bare_slot.clone(), cx);
    open_shell_window("headed", false, headed_slot.clone(), cx);

    assert_eq!(
        *bare_slot.borrow(),
        Some(Bounds {
            origin: point(px(1.), px(1.)),
            size: size(px(198.), px(118.)),
        }),
        "chromeless: only the 1px border insets the body, no header strip"
    );
    let headed_bounds = headed_slot
        .borrow()
        .expect("the headed shell paints its body");
    assert!(
        headed_bounds.origin.y > px(16.),
        "the header strip still offsets the body in a headed shell"
    );
    bare.simulate_click(point(px(30.), px(8.)), Modifiers::none());
    assert_eq!(
        *bare_slot.borrow(),
        Some(Bounds {
            origin: point(px(1.), px(1.)),
            size: size(px(198.), px(118.)),
        }),
        "a plain click at the old header line is just a body click"
    );
}

#[gpui_kit::test]
async fn clicks_inside_a_chromeless_body_reach_the_content(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let clicks = Rc::new(std::cell::Cell::new(0u32));
    let window = cx.open_window(size(px(200.), px(120.)), |_, cx| {
        let clicks = clicks.clone();
        let content: AnyView = cx.new(|_| ClickBody { clicks }).into();
        FloatingShell::new(
            FloatingWindowId("interactive".into()),
            "interactive".into(),
            content,
            true,
            cx,
        )
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_click(point(px(30.), px(12.)), Modifiers::none());
    assert_eq!(
        clicks.get(),
        1,
        "a click on the body button must not be swallowed by the shell"
    );
}

#[gpui_kit::test]
async fn escape_dismisses_a_chromeless_float_spawned_with_the_flag(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(
                    FloatingWindowId("bare-esc".into()),
                    size(px(200.), px(120.)),
                )
                .chromeless(true),
                |_, cx: &mut App| -> AnyView { cx.new(|_| Blank).into() },
                cx,
            )
        })
        .unwrap();
    let FloatingHandle::Native(window) = handle else {
        panic!("expected a native handle");
    };
    let mut cx = VisualTestContext::from_window(window, cx);
    cx.simulate_click(point(px(30.), px(8.)), Modifiers::none());
    cx.simulate_keystrokes("escape");
    assert!(
        cx.read(|cx| floating_snapshot(cx)).is_empty(),
        "Esc must still dismiss with the shell chrome gone"
    );
}
