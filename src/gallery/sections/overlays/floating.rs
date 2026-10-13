use gpui_kit::component::{
    ActiveTheme as _, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::gallery::registry::GallerySection;
use crate::ui::floating::{
    self, FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId, Placement9, floating_snapshot,
    probe,
};

use super::demo::section;

struct FloatDemoBody {
    id: SharedString,
}

impl Render for FloatDemoBody {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(format!("Float content for {}", self.id))
    }
}

pub struct FloatingSection {
    always_on_top: bool,
}

impl FloatingSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe_global::<floating::FloatingWindowRegistry>(|_, cx| cx.notify())
                .detach();
            Self {
                always_on_top: false,
            }
        })
    }

    fn spawn(&self, id: &'static str, anchor: FloatAnchor, cx: &mut App) {
        let level = if self.always_on_top {
            FloatLevel::AlwaysOnTop
        } else {
            FloatLevel::Floating
        };
        let spec = FloatingSpec::new(FloatingWindowId(id.into()), size(px(260.), px(160.)))
            .title(format!("Float {id}"))
            .level(level)
            .anchor(anchor);
        let body_id: SharedString = id.into();
        let spawned = floating::spawn_floating(
            spec,
            move |_, cx| {
                let id = body_id.clone();
                cx.new(move |_| FloatDemoBody { id }).into()
            },
            cx,
        );
        if let Err(err) = spawned {
            tracing::warn!(
                target: "gpui_starter::gallery::floating",
                error = %err,
                "float spawn failed"
            );
        }
    }

    fn anchor_top_right(cx: &App) -> FloatAnchor {
        let placement = Placement9::TopRight;
        let offset = point(px(8.), px(8.));
        match crate::app::window::root_window(cx) {
            Some(window) => FloatAnchor::Window {
                window,
                placement,
                offset,
            },
            None => FloatAnchor::Screen {
                display: None,
                placement,
                offset,
            },
        }
    }

    fn screen_center() -> FloatAnchor {
        FloatAnchor::Screen {
            display: None,
            placement: Placement9::Center,
            offset: Point::default(),
        }
    }
}

impl Render for FloatingSection {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let info = probe(cx);
        let open = floating_snapshot(cx).len();
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("floating-spawn", "Spawn")
                    .description("Anchored utility windows over the app, or overlay panes on web.")
                    .w(rems(32.5))
                    .child(
                        Button::new("floating-anchor-top-right")
                            .label("Anchor top right")
                            .icon(IconName::WindowRestore)
                            .on_click(cx.listener(|this, _, _, cx| {
                                let anchor = Self::anchor_top_right(cx);
                                this.spawn("top-right", anchor, cx);
                            })),
                    )
                    .child(
                        Button::new("floating-screen-center")
                            .label("Screen center")
                            .icon(IconName::Maximize)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.spawn("screen-center", Self::screen_center(), cx);
                            })),
                    )
                    .child(
                        Button::new("floating-always-on-top")
                            .label("Always on top")
                            .icon(IconName::PanelLeft)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.always_on_top = !this.always_on_top;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("floating-close-all")
                            .ghost()
                            .label("Close all")
                            .on_click(|_, _, cx| {
                                floating::close_all_floating(cx);
                            }),
                    ),
            )
            .child(
                section("floating-status", "Platform status")
                    .description("Live facts from the floating platform probe.")
                    .w(rems(32.5))
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Open floats: {open}"))
                            .child(format!(
                                "Native windows: {}",
                                if info.native_windows { "yes" } else { "no" }
                            ))
                            .child(format!(
                                "Always on top: {}",
                                if info.always_on_top { "yes" } else { "no" }
                            ))
                            .child(format!(
                                "Focus steal: {}",
                                if info.non_focus_stealing {
                                    "prevented"
                                } else {
                                    "not prevented"
                                }
                            ))
                            .child(match info.menu_bar {
                                Some(height) => format!("Menu bar: {height}"),
                                None => "Menu bar: n/a".to_string(),
                            })
                            .child(format!(
                                "Notch: {}",
                                if info.notch.is_some() {
                                    "present"
                                } else {
                                    "none"
                                }
                            )),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "floating",
        "Floating Windows",
        "Child utility windows anchored to the window or screen; overlay fallback on web.",
        FloatingSection::view(window, cx),
    ));
}
