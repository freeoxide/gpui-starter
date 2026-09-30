//! Toolbar section, ported from the upstream `ToolbarStory`.

use gpui_kit::assets::IconName as AssetIconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, IndexPath, Sizable as _, Size,
    button::{Button, DropdownButton, Toggle},
    combobox::{Combobox, ComboboxState},
    input::{Input, InputState},
    searchable_list::SearchableVec,
    select::{Select, SelectState},
    separator::Separator,
    toolbar::{Toolbar, ToolbarGroup},
    v_flex,
};
use gpui_kit::{
    Action, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window, div, rems,
};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_label};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_toolbar_section, no_json)]
struct ToggleDisabled;

pub struct ToolbarSection {
    focus_handle: FocusHandle,
    size: Size,
    disabled: bool,
    formats: [bool; 3],
    font: Entity<SelectState<SearchableVec<&'static str>>>,
    market: Entity<SelectState<SearchableVec<&'static str>>>,
    status: Entity<ComboboxState<SearchableVec<&'static str>>>,
    query: Entity<InputState>,
}

impl ToolbarSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let font = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Inter", "SF Pro", "Helvetica", "Georgia"]),
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            });
            let market = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["All markets", "US market", "Hong Kong", "Singapore"]),
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            });
            let status = cx.new(|cx| {
                ComboboxState::new(
                    SearchableVec::new(vec!["Open", "In progress", "Filled", "Cancelled"]),
                    vec![],
                    window,
                    cx,
                )
                .searchable(true)
            });
            let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));

            Self {
                focus_handle: cx.focus_handle(),
                size: Size::Medium,
                disabled: false,
                formats: [true, false, false],
                font,
                market,
                status,
                query,
            }
        })
    }
}

impl Focusable for ToolbarSection {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn icon_button(id: &'static str, icon: IconName, tooltip: &'static str, disabled: bool) -> Button {
    Button::new(id)
        .icon(icon)
        .tooltip(tooltip)
        .disabled(disabled)
}

impl Render for ToolbarSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let disabled = self.disabled;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &ToggleDisabled, _, cx| {
                this.disabled = !this.disabled;
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                DropdownButton::new("toolbar-options")
                    .button(
                        Button::new("toolbar-options-trigger")
                            .label(format!("Size: {}", size_label(size))),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "XSmall",
                            size == Size::XSmall,
                            Box::new(DemoToggle::SizeXSmall),
                        )
                        .menu_with_check(
                            "Small",
                            size == Size::Small,
                            Box::new(DemoToggle::SizeSmall),
                        )
                        .menu_with_check(
                            "Medium",
                            size == Size::Medium,
                            Box::new(DemoToggle::SizeMedium),
                        )
                        .separator()
                        .menu_with_check("Disabled", disabled, Box::new(ToggleDisabled))
                    })
                    .into_any_element(),
            ]))
            .child(
                section("toolbar-default", "Default")
                    .description(
                        "Keep document, history, and formatting commands in one compact editor toolbar.",
                    )
                    .w(rems(40.))
                    .child(
                        Toolbar::new("toolbar-default-row")
                            .w_full()
                            .with_size(size)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                ToolbarGroup::new("toolbar-document-group")
                                    .label("Document")
                                    .gap_1()
                                    .child(
                                        Button::new("toolbar-new-document")
                                            .icon(IconName::Plus)
                                            .label("New")
                                            .disabled(disabled),
                                    )
                                    .child(
                                        Button::new("toolbar-save-document")
                                            .icon(AssetIconName::Save)
                                            .label("Save")
                                            .disabled(disabled),
                                    ),
                            )
                            .content(Separator::vertical().h_5())
                            .child(
                                ToolbarGroup::new("toolbar-history-group")
                                    .label("History")
                                    .gap_1()
                                    .child(icon_button(
                                        "toolbar-undo",
                                        IconName::Undo2,
                                        "Undo",
                                        disabled,
                                    ))
                                    .child(icon_button(
                                        "toolbar-redo",
                                        IconName::Redo2,
                                        "Redo",
                                        disabled,
                                    )),
                            )
                            .content(Separator::vertical().h_5())
                            .child(
                                ToolbarGroup::new("toolbar-formatting-group")
                                    .label("Formatting")
                                    .gap_1()
                                    .child(
                                        Toggle::new("toolbar-bold")
                                            .label("B")
                                            .checked(self.formats[0])
                                            .disabled(disabled)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.formats[0] = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Toggle::new("toolbar-italic")
                                            .label("I")
                                            .checked(self.formats[1])
                                            .disabled(disabled)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.formats[1] = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Toggle::new("toolbar-underline")
                                            .label("U")
                                            .checked(self.formats[2])
                                            .disabled(disabled)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.formats[2] = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .content(Separator::vertical().h_5())
                            .child(
                                Select::new(&self.font)
                                    .placeholder("Font")
                                    .disabled(disabled)
                                    .w_40(),
                            ),
                    ),
            )
            .child(
                section("toolbar-mixed", "Mixed controls")
                    .description(
                        "Select and Combobox inherit the same density while preserving their own popup behavior.",
                    )
                    .w(rems(40.))
                    .child(
                        Toolbar::new("toolbar-mixed-row")
                            .w_full()
                            .with_size(size)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                Input::new(&self.query)
                                    .prefix(IconName::Search)
                                    .disabled(disabled)
                                    .w_40(),
                            )
                            .child(
                                Select::new(&self.market)
                                    .placeholder("Market")
                                    .disabled(disabled)
                                    .w_32(),
                            )
                            .child(
                                Combobox::new(&self.status)
                                    .placeholder("Order status")
                                    .disabled(disabled)
                                    .w_40(),
                            )
                            .content(div().flex_1())
                            .child(icon_button(
                                "toolbar-refresh",
                                IconName::RotateCw,
                                "Refresh",
                                disabled,
                            ))
                            .child(icon_button(
                                "toolbar-settings",
                                IconName::Settings2,
                                "Configure columns",
                                disabled,
                            )),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "toolbar",
        "Toolbar",
        "Groups commands and controls into one keyboard-navigable row.",
        ToolbarSection::view(window, cx),
    ));
}
