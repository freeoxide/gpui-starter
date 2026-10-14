use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::gallery::registry::GallerySection;
use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId, Placement9,
    close_floating, set_floating_always_on_top, set_floating_click_through, spawn_floating,
};

use super::demo::section;

const LOG: &str = "gpui_starter::gallery::window_shims";

struct ShimDemoBody;

impl Render for ShimDemoBody {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child("Widget float")
    }
}

pub struct WindowShimsSection {
    demo: Option<FloatingHandle>,
    click_through_on: bool,
    topmost_on: bool,
    last_applied: Option<(bool, bool)>,
}

impl WindowShimsSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            demo: None,
            click_through_on: false,
            topmost_on: false,
            last_applied: None,
        })
    }

    fn spawn(&mut self, level: FloatLevel, cx: &mut App) {
        if let Some(handle) = self.demo.take() {
            close_floating(&handle, cx);
        }
        let spec = FloatingSpec::new(
            FloatingWindowId("window-shims-demo".into()),
            size(px(320.), px(180.)),
        )
        .title("Widget float")
        .level(level)
        .click_through(false)
        .anchor(FloatAnchor::Screen {
            display: None,
            placement: Placement9::Center,
            offset: point(px(0.), px(0.)),
        })
        .focus(false);
        match spawn_floating(spec, |_, cx| cx.new(|_| ShimDemoBody).into(), cx) {
            Ok(handle) => {
                self.demo = Some(handle);
                self.click_through_on = false;
                self.topmost_on = level == FloatLevel::AlwaysOnTop;
                self.last_applied = None;
            }
            Err(err) => {
                tracing::warn!(target: LOG, error = %err, "demo float spawn failed");
            }
        }
    }

    fn toggle_click_through(&mut self, cx: &mut App) {
        let Some(handle) = self.demo.clone() else {
            return;
        };
        self.click_through_on = !self.click_through_on;
        let applied = set_floating_click_through(&handle, self.click_through_on, cx);
        self.last_applied = Some(match self.last_applied {
            Some((_, topmost)) => (applied, topmost),
            None => (applied, false),
        });
    }

    fn toggle_topmost(&mut self, cx: &mut App) {
        let Some(handle) = self.demo.clone() else {
            return;
        };
        self.topmost_on = !self.topmost_on;
        let applied = set_floating_always_on_top(&handle, self.topmost_on, cx);
        self.last_applied = Some(match self.last_applied {
            Some((click_through, _)) => (click_through, applied),
            None => (false, applied),
        });
    }

    fn close(&mut self, cx: &mut App) {
        if let Some(handle) = self.demo.take() {
            close_floating(&handle, cx);
        }
        self.click_through_on = false;
        self.topmost_on = false;
        self.last_applied = None;
    }
}

impl Render for WindowShimsSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_demo = self.demo.is_some();
        let toggle_state = self.last_applied;
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("window-shims-demo", "Live float")
                    .description(
                        "Spawn (topmost) opens the PopUp kind; Spawn (managed) opens a WM-managed window.",
                    )
                    .w(rems(32.5))
                    .child(
                        Button::new("window-shims-spawn-topmost")
                            .label("Spawn (topmost)")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.spawn(FloatLevel::AlwaysOnTop, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("window-shims-spawn-managed")
                            .label("Spawn (managed)")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.spawn(FloatLevel::Floating, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("window-shims-click-through")
                            .label("Click-through")
                            .disabled(!has_demo)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_click_through(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("window-shims-topmost")
                            .label("Always on top")
                            .disabled(!has_demo)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_topmost(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("window-shims-close")
                            .ghost()
                            .label("Close")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("window-shims-status", "Toggle results")
                    .description("What the last call to each shim returned on this platform.")
                    .w(rems(32.5))
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(match toggle_state {
                                None => "No toggles yet.".to_string(),
                                Some((click_through, topmost)) => format!(
                                    "Click-through: {}; Always on top: {}",
                                    applied_label(click_through),
                                    applied_label(topmost)
                                ),
                            }),
                    ),
            )
    }
}

fn applied_label(applied: bool) -> &'static str {
    if applied {
        "applied"
    } else {
        "not applied on this platform"
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "window_shims",
        "Click-Through & Topmost",
        "Toggle mouse pass-through and always-on-top on a live floating window.",
        WindowShimsSection::view(window, cx),
    ));
}
