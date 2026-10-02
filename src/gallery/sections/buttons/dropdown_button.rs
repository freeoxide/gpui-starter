//! DropdownButton section, ported from the upstream `DropdownButtonStory`.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex, v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoState, DemoToggle, demo_toolbar, options_dropdown, section, size_dropdown};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_dropdown_button_section, no_json)]
enum DropdownButtonAction {
    ExportCsv,
    ExportPdf,
    SaveCopy,
    SaveTemplate,
    OpenQuarterlyReport,
    OpenWatchlistLayout,
}

pub struct DropdownButtonSection {
    demo: DemoState,
    last_action: SharedString,
}

impl DropdownButtonSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            demo: DemoState::default(),
            last_action: "Nothing yet".into(),
        })
    }

    fn record(&mut self, action: &DropdownButtonAction, cx: &mut Context<Self>) {
        self.last_action = match action {
            DropdownButtonAction::ExportCsv => "Exported as CSV",
            DropdownButtonAction::ExportPdf => "Exported as PDF",
            DropdownButtonAction::SaveCopy => "Saved as a new file",
            DropdownButtonAction::SaveTemplate => "Saved as a template",
            DropdownButtonAction::OpenQuarterlyReport => "Opened Quarterly Report.gpui",
            DropdownButtonAction::OpenWatchlistLayout => "Opened Watchlist Layout.gpui",
        }
        .into();
        cx.notify();
    }
}

impl Render for DropdownButtonSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let demo = &self.demo;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, window, cx| {
                if this.demo.apply(action, window, cx) {
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, action: &DropdownButtonAction, _, cx| {
                this.record(action, cx);
            }))
            .child(demo_toolbar(vec![
                size_dropdown("dropdown-button-size", demo.size).into_any_element(),
                options_dropdown("dropdown-button-options", demo).into_any_element(),
            ]))
            .child(
                h_flex()
                    .gap_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Last action:")
                    .child(self.last_action.clone()),
            )
            .child(
                section("dropdown-button-split", "Basic split").child(
                    DropdownButton::new("dropdown-button-export")
                        .with_size(demo.size)
                        .primary()
                        .button(
                            Button::new("dropdown-button-export-main")
                                .label("Export")
                                .when(demo.compact, |this| this.compact())
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.last_action = "Exported current view".into();
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .disabled(demo.disabled)
                        .selected(demo.selected)
                        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                            menu.menu(
                                "Export all rows (.csv)",
                                Box::new(DropdownButtonAction::ExportCsv),
                            )
                            .menu(
                                "Download report (.pdf)",
                                Box::new(DropdownButtonAction::ExportPdf),
                            )
                        }),
                ),
            )
            .child(
                section("dropdown-button-inner-options", "Inner button options").child(
                    DropdownButton::new("dropdown-button-save")
                        .with_size(demo.size)
                        .outline()
                        .button(
                            Button::new("dropdown-button-save-main")
                                .label("Save")
                                .tooltip("Save the current document")
                                .when(demo.compact, |this| this.compact())
                                .loading(demo.loading)
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.last_action = "Saved document".into();
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .disabled(demo.disabled)
                        .dropdown_menu(move |menu, _, _| {
                            menu.menu(
                                "Save as new file…",
                                Box::new(DropdownButtonAction::SaveCopy),
                            )
                            .menu(
                                "Save as template…",
                                Box::new(DropdownButtonAction::SaveTemplate),
                            )
                        }),
                ),
            )
            .child(
                section("dropdown-button-inherited", "Inherited styling").child(
                    DropdownButton::new("dropdown-button-recent")
                        .button(
                            Button::new("dropdown-button-recent-main")
                                .label("Open latest")
                                .ghost()
                                .small()
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.last_action = "Opened latest file".into();
                                            cx.notify();
                                        });
                                    }
                                }),
                        )
                        .selected(demo.selected)
                        .disabled(demo.disabled)
                        .dropdown_menu(move |menu, _, _| {
                            menu.menu(
                                "Quarterly Report.gpui",
                                Box::new(DropdownButtonAction::OpenQuarterlyReport),
                            )
                            .menu(
                                "Watchlist Layout.gpui",
                                Box::new(DropdownButtonAction::OpenWatchlistLayout),
                            )
                        }),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "dropdown-button",
        "Dropdown Button",
        "A button with an attached dropdown menu for additional options.",
        DropdownButtonSection::view(window, cx),
    ));
}
