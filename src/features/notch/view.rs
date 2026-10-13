//! NotchFloaterView: hover/pin state machine, clock tick, and render.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::*;

use super::geometry::NotchLayout;
use super::widgets;

#[cfg(test)]
#[path = "view.test.rs"]
mod view_test;

pub(crate) struct NotchFloaterView {
    hovered: bool,
    pinned: bool,
    minute: String,
    date: String,
    layout: NotchLayout,
    resizable: bool,
    focus_handle: FocusHandle,
}

impl NotchFloaterView {
    /// `resizable` is true only for the macOS notch panel; the fallback float
    /// swaps content at a fixed window size.
    pub(crate) fn new(layout: NotchLayout, resizable: bool, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            hovered: false,
            pinned: false,
            minute: String::new(),
            date: String::new(),
            layout,
            resizable,
            focus_handle: cx.focus_handle(),
        };
        this.start_clock(cx);
        this
    }

    fn start_clock(&mut self, cx: &mut Context<Self>) {
        let now = chrono::Local::now();
        self.minute = now.format("%H:%M").to_string();
        self.date = now.format("%a, %b %e").to_string();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let now = chrono::Local::now();
        let minute = now.format("%H:%M").to_string();
        let date = now.format("%a, %b %e").to_string();
        if minute != self.minute || date != self.date {
            self.minute = minute;
            self.date = date;
            cx.notify();
        }
    }

    fn is_expanded(&self) -> bool {
        self.hovered || self.pinned
    }

    fn set_hovered(&mut self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.hovered == hovered {
            return;
        }
        self.hovered = hovered;
        self.sync_frame(window, cx);
        cx.notify();
    }

    fn toggle_pinned(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_expanded = self.is_expanded();
        self.pinned = !self.pinned;
        if self.is_expanded() == was_expanded {
            return;
        }
        self.sync_frame(window, cx);
        cx.notify();
    }

    fn sync_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.resizable {
            return;
        }
        let target = if self.is_expanded() {
            self.layout.expanded
        } else {
            self.layout.collapsed
        };
        #[cfg(target_os = "macos")]
        super::macos::resize_to(window, target, !cx.reduce_motion());
        #[cfg(not(target_os = "macos"))]
        let _ = (window, target, cx);
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "macos", not(target_family = "wasm")))]
        {
            let _ = cx;
            window.remove_window();
        }
        #[cfg(all(not(target_os = "macos"), target_family = "wasm"))]
        {
            let _ = window;
            super::controller::close(cx);
        }
    }

    fn render_expanded(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .gap_4()
                    .justify_between()
                    .items_start()
                    .child(widgets::clock_block(&self.minute, &self.date, cx))
                    .child(widgets::now_playing(cx))
                    .child(widgets::battery_block(cx)),
            )
            .child(div().border_t_1().border_color(cx.theme().border))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("notch-collapse")
                            .ghost()
                            .small()
                            .label(crate::i18n::localize("notch_collapse"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.pinned = false;
                                this.hovered = false;
                                this.sync_frame(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("notch-close")
                            .ghost()
                            .small()
                            .label(crate::i18n::localize("notch_close"))
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    ),
            )
    }
}

impl Render for NotchFloaterView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .id("notch-floater")
            .size_full()
            .track_focus(&self.focus_handle)
            .on_hover(move |hovered, window, cx| {
                entity.update(cx, |this, cx| this.set_hovered(*hovered, window, cx));
            })
            .on_mouse_down(MouseButton::Left, {
                let entity = cx.entity();
                move |_, window, cx| {
                    entity.update(cx, |this, cx| this.toggle_pinned(window, cx));
                }
            })
            .bg(cx.theme().popover)
            .border_b_1()
            .border_color(cx.theme().border)
            .rounded_b(cx.theme().radius)
            .overflow_hidden()
            .child(if self.is_expanded() {
                self.render_expanded(cx).into_any_element()
            } else {
                widgets::collapsed_pill(&self.minute, cx).into_any_element()
            })
    }
}

impl Focusable for NotchFloaterView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
