//! Chromeless interactive float examples: a dictation HUD with a live
//! waveform, a quick-sketch canvas pad, a launcher menu card, and the two
//! screen-anchor variant demos (no main window, off-window bounds); the
//! dictation HUD is Alt+Shift+D-toggleable via the shortcuts service.
//! Removal = delete this directory and revert the `mod floating_widgets;`
//! and `floating_widgets::register` lines in
//! src/gallery/sections/overlays/mod.rs.

mod dictation;
mod menu_card;
mod quick_float;
mod sketch;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::gallery::registry::GallerySection;
use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId,
    FloatingWindowRegistry, Placement9, close_floating, floating_snapshot, spawn_screen_floating,
};

use super::demo::section;

pub use dictation::spawn_dictation;
pub use menu_card::spawn_menu_card;
pub use quick_float::{install_quick_float, toggle_quick_float};
pub use sketch::spawn_sketch;

/// Every example float id starts with this prefix.
pub const ID_PREFIX: &str = "chromeless-";

pub(crate) fn entry_handle(id: &str, cx: &App) -> Option<FloatingHandle> {
    floating_snapshot(cx)
        .into_iter()
        .find(|entry| entry.id.0 == id)
        .map(|entry| entry.handle)
}

/// Close every example float this section spawned; returns how many closed.
pub fn close_all_examples(cx: &mut App) -> usize {
    let handles: Vec<FloatingHandle> = floating_snapshot(cx)
        .into_iter()
        .filter(|entry| entry.id.0.starts_with(ID_PREFIX))
        .map(|entry| entry.handle)
        .collect();
    handles
        .iter()
        .filter(|handle| close_floating(handle, cx))
        .count()
}

pub(crate) const INDEPENDENT_ID: &str = "chromeless-independent";
pub(crate) const OFF_WINDOW_ID: &str = "chromeless-off-window";

struct VariantDemoBody {
    headline: &'static str,
    detail: &'static str,
}

impl Render for VariantDemoBody {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_2()
            .justify_center()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(self.headline),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.detail),
            )
    }
}

pub(crate) fn independent_spec() -> FloatingSpec {
    FloatingSpec::new(
        FloatingWindowId(INDEPENDENT_ID.into()),
        size(px(320.), px(140.)),
    )
    .title("Independent float")
    .level(FloatLevel::AlwaysOnTop)
    .anchor(FloatAnchor::Screen {
        display: None,
        placement: Placement9::TopRight,
        offset: point(px(-16.), px(16.)),
    })
    .chromeless(true)
    .focus(true)
    .movable(true)
}

pub(crate) fn off_window_spec() -> FloatingSpec {
    FloatingSpec::new(
        FloatingWindowId(OFF_WINDOW_ID.into()),
        size(px(320.), px(140.)),
    )
    .title("Off-window float")
    .level(FloatLevel::AlwaysOnTop)
    .anchor(FloatAnchor::Screen {
        display: None,
        placement: Placement9::BottomLeft,
        offset: point(px(16.), px(-16.)),
    })
    .chromeless(true)
    .focus(true)
    .movable(true)
}

/// Open the no-main-window demo as a screen float; idempotent per id.
pub fn spawn_independent(cx: &mut App) -> anyhow::Result<FloatingHandle> {
    if let Some(handle) = entry_handle(INDEPENDENT_ID, cx) {
        return Ok(handle);
    }
    spawn_screen_floating(
        independent_spec(),
        |_, cx| {
            cx.new(|_| VariantDemoBody {
                headline: "No main window required",
                detail: "Anchored to the screen, not the app window; it opens and stays with no main window running.",
            })
            .into()
        },
        cx,
    )
}

/// Open the not-window-scoped demo as a screen float; idempotent per id.
pub fn spawn_off_window(cx: &mut App) -> anyhow::Result<FloatingHandle> {
    if let Some(handle) = entry_handle(OFF_WINDOW_ID, cx) {
        return Ok(handle);
    }
    spawn_screen_floating(
        off_window_spec(),
        |_, cx| {
            cx.new(|_| VariantDemoBody {
                headline: "Not window-scoped",
                detail: "This float sits outside the main window's bounds; only the display edge clamps it. Alt-drag moves, Esc dismisses.",
            })
            .into()
        },
        cx,
    )
}

pub(crate) struct FloatingWidgetsSection;

impl FloatingWidgetsSection {
    pub(crate) fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe_global::<FloatingWindowRegistry>(|_, cx| cx.notify())
                .detach();
            Self
        })
    }

    fn open_example(
        &self,
        spawn: fn(&mut App) -> anyhow::Result<FloatingHandle>,
        cx: &mut Context<Self>,
    ) {
        if let Err(err) = spawn(cx) {
            tracing::warn!(
                target: "gpui_starter::gallery",
                error = %err,
                "example float failed to open"
            );
        }
        cx.notify();
    }
}

impl Render for FloatingWidgetsSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0.starts_with(ID_PREFIX))
            .count();
        v_flex()
            .w_full()
            .gap_4()
            .p_4()
            .child(
                section("fw-examples", "Chromeless floats")
                    .description(
                        "Screen floats without window chrome; Alt-drag moves, Esc dismisses.",
                    )
                    .w(rems(32.5))
                    .child(
                        Button::new("fw-dictation")
                            .label("Dictation HUD…")
                            .icon(IconName::Mic)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_example(spawn_dictation, cx)
                            })),
                    )
                    .child(
                        Button::new("fw-sketch")
                            .label("Sketch pad…")
                            .icon(IconName::Palette)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.open_example(spawn_sketch, cx)),
                            ),
                    )
                    .child(
                        Button::new("fw-menu")
                            .label("Menu card…")
                            .icon(IconName::Menu)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_example(spawn_menu_card, cx)
                            })),
                    )
                    .child(
                        h_flex()
                            .w(rems(32.5))
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{open} example floats open")),
                            )
                            .child(
                                Button::new("fw-close-all")
                                    .ghost()
                                    .small()
                                    .label("Close all examples")
                                    .disabled(open == 0)
                                    .on_click(|_, _, cx| {
                                        close_all_examples(cx);
                                    }),
                            ),
                    ),
            )
            .child(
                section("fw-independent", "No main window")
                    .description(
                        "Anchored to the screen, not the app window; this float stays open after the main window closes. Overlay pane on web.",
                    )
                    .w(rems(32.5))
                    .child(
                        Button::new("fw-no-main-window")
                            .label("Independent float…")
                            .icon(IconName::Frame)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_example(spawn_independent, cx)
                            })),
                    ),
            )
            .child(
                section("fw-off-window", "Not window-scoped")
                    .description(
                        "Screen floats clamp to the display edge, never the main window; Alt-drag this one past the window edge. Overlay pane on web.",
                    )
                    .w(rems(32.5))
                    .child(
                        Button::new("fw-off-window")
                            .label("Off-window float…")
                            .icon(IconName::Maximize)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_example(spawn_off_window, cx)
                            })),
                    ),
            )
            .child(
                section("fw-shortcut", "Quick shortcut")
                    .description(
                        "Alt+Shift+D toggles the dictation HUD: OS-global on macOS and Windows, in-app elsewhere.",
                    )
                    .w(rems(32.5))
                    .child(
                        Button::new("fw-toggle-quick")
                            .label("Toggle dictation HUD")
                            .on_click(|_, _, cx| toggle_quick_float(cx)),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    // Startup wiring (GalleryPage builds eagerly): route the quick-float
    // accelerator without touching app/init.rs.
    if let Err(err) = install_quick_float(cx) {
        tracing::warn!(
            target: "gpui_starter::gallery",
            error = %err,
            "quick-float shortcut handler registration failed"
        );
    }
    sections.push(GallerySection::new(
        "floating-widgets",
        "Chromeless Floats",
        "Interactive screen floats without window chrome: a dictation waveform HUD, a sketch pad, a menu card, plus independent and off-window float variants.",
        FloatingWidgetsSection::view(window, cx),
    ));
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod mod_test;
