//! Scroll bounce section, ported from the upstream `scroll_bounce` example:
//! edge-stretch scrolling over a virtual list and over short content.

use gpui_kit::base::ScrollBounce;
use gpui_kit::component::{ActiveTheme as _, button::Button, h_flex, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

const ROWS: usize = 120;

/// Example-only adapter: keep targeting the viewport where the drag began,
/// even when the pointer moves over another column or outside that viewport.
struct MouseScroll {
    anchor: Point<Pixels>,
    previous: Point<Pixels>,
}

impl MouseScroll {
    fn event(&mut self, position: Point<Pixels>, phase: TouchPhase) -> ScrollWheelEvent {
        let delta = if phase == TouchPhase::Moved {
            position.y - self.previous.y
        } else {
            px(0.)
        };
        self.previous = position;
        ScrollWheelEvent {
            position: self.anchor,
            delta: ScrollDelta::Pixels(point(px(0.), delta)),
            touch_phase: phase,
            ..Default::default()
        }
    }
}

fn send_scroll(event: ScrollWheelEvent, window: &mut Window, cx: &mut App) {
    // Mouse listeners are temporarily taken out during dispatch. Re-entering
    // dispatch synchronously would miss the viewport's scroll listeners.
    window.defer(cx, move |window, cx| {
        window.dispatch_event(PlatformInput::ScrollWheel(event), cx);
    });
}

pub struct ScrollBounceSection {
    list: ListState,
    short: ScrollHandle,
    enabled: bool,
    generation: usize,
    drag: Option<MouseScroll>,
}

impl ScrollBounceSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            list: ListState::new(ROWS, ListAlignment::Top, px(200.)).measure_all(),
            short: ScrollHandle::new(),
            enabled: true,
            generation: 0,
            drag: None,
        })
    }

    fn begin_drag(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
        let mut drag = MouseScroll {
            anchor: position,
            previous: position,
        };
        send_scroll(drag.event(position, TouchPhase::Started), window, cx);
        self.drag = Some(drag);
    }

    fn end_drag(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(mut drag) = self.drag.take() {
            send_scroll(drag.event(drag.previous, TouchPhase::Ended), window, cx);
        }
    }

    fn render_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_wrap()
            .justify_center()
            .gap_3()
            .child(
                Button::new("scroll-bounce-toggle")
                    .outline()
                    .label(if self.enabled {
                        "Bounce: on"
                    } else {
                        "Bounce: off"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.enabled = !this.enabled;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("scroll-bounce-top")
                    .outline()
                    .label("Top")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.list.scroll_to_reveal_item(0);
                        this.generation += 1;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("scroll-bounce-bottom")
                    .outline()
                    .label("Bottom")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.list.scroll_to_end();
                        this.generation += 1;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("scroll-bounce-reduced-motion")
                    .outline()
                    .label(if cx.reduce_motion() {
                        "Reduced motion: on"
                    } else {
                        "Reduced motion: off"
                    })
                    .on_click(cx.listener(|_, _, window, cx| {
                        let reduced = !cx.reduce_motion();
                        cx.set_reduce_motion(reduced);
                        window.refresh();
                    })),
            )
    }

    fn render_viewports(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let generation = self.generation;
        h_flex()
            .items_stretch()
            .w_full()
            .h(rems(20.))
            .gap_4()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .child(div().text_sm().child(format!("Virtual list · {ROWS} rows")))
                    .child(
                        div()
                            .id("scroll-bounce-list")
                            .flex_1()
                            .min_h_0()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                                    this.begin_drag(event.position, window, cx);
                                    cx.stop_propagation();
                                }),
                            )
                            .child(
                                ScrollBounce::new(
                                    ("scroll-bounce-long", generation),
                                    &self.list,
                                    list(self.list.clone(), |ix, _, cx| {
                                        div()
                                            .p_4()
                                            .border_b_1()
                                            .border_color(cx.theme().border)
                                            .bg(cx.theme().background)
                                            .child(format!(
                                                "Message {} · swipe, hold, release, reverse",
                                                ix + 1
                                            ))
                                            .into_any_element()
                                    })
                                    .flex_1(),
                                )
                                .enabled(self.enabled),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .child(div().text_sm().child("Short content · both edges"))
                    .child(
                        ScrollBounce::new(
                            ("scroll-bounce-short", generation),
                            &self.short,
                            div()
                                .id("scroll-bounce-short")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .track_scroll(&self.short)
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius)
                                .child(
                                    v_flex()
                                        .p_4()
                                        .gap_4()
                                        .child(div().text_sm().child(
                                            "This content fits inside the viewport. Pull \
                                                 down or up.",
                                        ))
                                        .child(
                                            div()
                                                .id("scroll-bounce-horizontal")
                                                .overflow_x_scroll()
                                                .h_16()
                                                .child(div().w(rems(48.)).child(
                                                    "← Horizontal content · drag sideways \
                                                         across this row to verify nested \
                                                         scrolling →",
                                                )),
                                        ),
                                ),
                        )
                        .enabled(self.enabled),
                    ),
            )
    }
}

impl Render for ScrollBounceSection {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().w_full().items_center().p_4().child(
            section("scroll-bounce-demo", "Edge stretch")
                .description(
                    "Hold the left mouse button on the list and drag past an edge, then \
                         release. Trackpad scrolling also works.",
                )
                .child(
                    div()
                        .id("scroll-bounce-stage")
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                            if event.pressed_button != Some(MouseButton::Left) {
                                this.end_drag(window, cx);
                            } else if let Some(drag) = &mut this.drag {
                                send_scroll(
                                    drag.event(event.position, TouchPhase::Moved),
                                    window,
                                    cx,
                                );
                            }
                        }))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| this.end_drag(window, cx)),
                        )
                        .on_mouse_up_out(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| this.end_drag(window, cx)),
                        )
                        .child(self.render_controls(cx))
                        .child(self.render_viewports(cx)),
                ),
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "scroll-bounce",
        "Scroll Bounce",
        "Edge-stretch scrolling over a virtual list and short content.",
        ScrollBounceSection::view(window, cx),
    ));
}
