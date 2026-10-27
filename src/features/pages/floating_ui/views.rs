use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, h_flex,
    input::{InputEvent, Textarea, TextareaState},
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

use super::persistence;
use super::state::{ASSET_PRESETS, COLOR_PRESETS, PaletteId, PaletteSyncState, rgba_to_hex};

pub(crate) fn emit_color_change(hex: String, cx: &mut App) {
    crate::events::emit(
        crate::events::AppEventKind::PaletteChanged {
            palette: "color".into(),
            value: hex,
        },
        cx,
    );
}

pub(crate) fn emit_asset_change(glyph: String, cx: &mut App) {
    crate::events::emit(
        crate::events::AppEventKind::PaletteChanged {
            palette: "assets".into(),
            value: glyph,
        },
        cx,
    );
}

pub(crate) struct ColorPaletteView {
    _subscriptions: Vec<Subscription>,
}

impl ColorPaletteView {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_global::<PaletteSyncState>(|_, cx| {
            cx.notify();
        });
        Self {
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for ColorPaletteView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sync = cx
            .try_global::<PaletteSyncState>()
            .cloned()
            .unwrap_or_default();
        v_flex()
            .p_2()
            .gap_2()
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_1()
                    .children(COLOR_PRESETS.iter().map(|(slug, hex)| {
                        let hex_str = rgba_to_hex(rgba(*hex));
                        let selected = sync.color_hex == hex_str;
                        div()
                            .id(ElementId::Name(SharedString::from(format!(
                                "swatch-{slug}"
                            ))))
                            .size_6()
                            .rounded(cx.theme().radius)
                            .bg(rgba(*hex))
                            .when(selected, |this| {
                                this.border_1().border_color(cx.theme().primary)
                            })
                            .hover(|this| this.border_1().border_color(cx.theme().border))
                            .focus_visible(|this| this.border_1().border_color(cx.theme().primary))
                            .a11y(
                                Role::Button,
                                format!(
                                    "{} {hex_str}",
                                    crate::i18n::localize("floating_ui_color_swatch")
                                ),
                            )
                            .aria_toggled(if selected {
                                Toggled::True
                            } else {
                                Toggled::False
                            })
                            .on_click({
                                let hex_str = hex_str.clone();
                                move |_, _, cx| emit_color_change(hex_str.clone(), cx)
                            })
                    })),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(sync.color_hex.clone()),
            )
    }
}

pub(crate) struct AssetPaletteView {
    _subscriptions: Vec<Subscription>,
}

impl AssetPaletteView {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_global::<PaletteSyncState>(|_, cx| {
            cx.notify();
        });
        Self {
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for AssetPaletteView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sync = cx
            .try_global::<PaletteSyncState>()
            .cloned()
            .unwrap_or_default();
        h_flex()
            .p_2()
            .flex_wrap()
            .gap_1()
            .children(ASSET_PRESETS.iter().enumerate().map(|(ix, glyph)| {
                let selected = sync.last_asset.as_ref().is_some_and(|asset| asset == glyph);
                div()
                    .id(ElementId::Name(SharedString::from(format!("asset-{ix}"))))
                    .p_2()
                    .rounded(cx.theme().radius)
                    .text_lg()
                    .when(selected, |this| {
                        this.border_1().border_color(cx.theme().primary)
                    })
                    .hover(|this| this.border_1().border_color(cx.theme().border))
                    .focus_visible(|this| this.border_1().border_color(cx.theme().primary))
                    .a11y(
                        Role::Button,
                        crate::i18n::localize_with_args(
                            "floating_ui_asset_pick",
                            &[("asset", glyph)],
                        ),
                    )
                    .on_click(move |_, _, cx| emit_asset_change(glyph.to_string(), cx))
                    .child(*glyph)
            }))
    }
}

pub(crate) struct StickyNoteView {
    textarea: Entity<TextareaState>,
    _subscriptions: Vec<Subscription>,
}

impl StickyNoteView {
    pub(crate) fn new(id: PaletteId, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let seeded = persistence::slot(id, cx)
            .and_then(|entry| entry.note)
            .unwrap_or_default();
        let textarea = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .placeholder(crate::i18n::localize("floating_ui_note_placeholder"));
            state.set_value(seeded, window, cx);
            state
        });
        let subscription = cx.subscribe(&textarea, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let note = this.textarea.read(cx).value().to_string();
                persistence::update_slot(PaletteId::Note, cx, |entry| entry.note = Some(note));
            }
        });
        Self {
            textarea,
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for StickyNoteView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_2()
            .size_full()
            .child(Textarea::new(&self.textarea).h_full())
    }
}

/// Content factory handed to plan 02's `spawn_floating`; callable more than
/// once (respawn), creates a fresh view entity per call.
pub(crate) fn palette_view_factory(id: PaletteId) -> Rc<dyn Fn(&mut Window, &mut App) -> AnyView> {
    match id {
        PaletteId::Color => {
            Rc::new(|window, cx| cx.new(|cx| ColorPaletteView::new(window, cx)).into())
        }
        PaletteId::Assets => {
            Rc::new(|window, cx| cx.new(|cx| AssetPaletteView::new(window, cx)).into())
        }
        PaletteId::Note => {
            Rc::new(move |window, cx| cx.new(|cx| StickyNoteView::new(id, window, cx)).into())
        }
    }
}

#[cfg(test)]
#[path = "views.test.rs"]
mod views_test;
