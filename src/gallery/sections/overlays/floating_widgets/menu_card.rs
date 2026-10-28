use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    kbd::Kbd,
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId, Placement9,
    spawn_screen_floating,
};

pub(crate) const MENU_ID: &str = "chromeless-menu";

pub(crate) const ROW_DICTATION: &str = "menu-dictation";
pub(crate) const ROW_SKETCH: &str = "menu-sketch";
pub(crate) const ROW_CLOSE: &str = "menu-close";

pub(crate) struct MenuRow {
    pub(crate) id: &'static str,
    pub(crate) icon: IconName,
    pub(crate) label: &'static str,
}

pub(crate) fn menu_rows() -> &'static [MenuRow] {
    &[
        MenuRow {
            id: ROW_DICTATION,
            icon: IconName::Mic,
            label: "Dictation HUD",
        },
        MenuRow {
            id: ROW_SKETCH,
            icon: IconName::Palette,
            label: "Sketch pad",
        },
        MenuRow {
            id: ROW_CLOSE,
            icon: IconName::CircleX,
            label: "Close all examples",
        },
    ]
}

pub(crate) struct MenuCard;

impl MenuCard {
    pub(crate) fn new_view(_: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| Self).into()
    }

    pub(crate) fn activate(&mut self, row: &str, cx: &mut App) {
        match row {
            ROW_DICTATION => open_quietly(super::spawn_dictation, cx),
            ROW_SKETCH => open_quietly(super::spawn_sketch, cx),
            ROW_CLOSE => {
                super::close_all_examples(cx);
            }
            _ => {}
        }
    }
}

fn open_quietly(spawn: fn(&mut App) -> anyhow::Result<FloatingHandle>, cx: &mut App) {
    if let Err(err) = spawn(cx) {
        tracing::warn!(
            target: "gpui_starter::gallery",
            error = %err,
            "example float failed to open"
        );
    }
}

impl Render for MenuCard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Button> = menu_rows()
            .iter()
            .map(|row| {
                Button::new(row.id)
                    .ghost()
                    .small()
                    .icon(row.icon.clone())
                    .label(row.label)
                    .on_click(cx.listener({
                        let id = row.id;
                        move |this, _, _, cx| this.activate(id, cx)
                    }))
            })
            .collect();
        let hint_text = cx.theme().muted_foreground;
        v_flex().size_full().p_1().gap_1().children(rows).child(
            h_flex()
                .mt_1()
                .px_2()
                .justify_between()
                .items_center()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Kbd::new(Keystroke::parse("escape").expect("static chord")))
                        .child(div().text_xs().text_color(hint_text).child("dismiss")),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Kbd::new(Keystroke::parse("alt").expect("static chord")))
                        .child(div().text_xs().text_color(hint_text).child("drag moves")),
                ),
        )
    }
}

pub(crate) fn menu_spec() -> FloatingSpec {
    FloatingSpec::new(FloatingWindowId(MENU_ID.into()), size(px(240.), px(190.)))
        .title("Quick actions")
        .level(FloatLevel::AlwaysOnTop)
        .anchor(FloatAnchor::Screen {
            display: None,
            placement: Placement9::BottomRight,
            offset: point(px(-16.), px(-16.)),
        })
        .chromeless(true)
        .focus(true)
        .movable(true)
}

/// Open the menu card as a screen float; idempotent per id.
pub fn spawn_menu_card(cx: &mut App) -> anyhow::Result<FloatingHandle> {
    if let Some(handle) = super::entry_handle(MENU_ID, cx) {
        return Ok(handle);
    }
    spawn_screen_floating(menu_spec(), MenuCard::new_view, cx)
}

#[cfg(test)]
#[path = "menu_card.test.rs"]
mod menu_card_test;
