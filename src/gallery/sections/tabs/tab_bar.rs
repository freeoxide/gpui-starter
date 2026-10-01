//! Tabs section, ported from the upstream `TabsStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _, Size,
    button::{Button, ButtonGroup, ButtonVariants as _, DropdownButton},
    h_flex,
    menu::PopupMenu,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

/// The max tab widths to choose from; `None` leaves tabs uncapped.
const MAX_WIDTHS: [Option<f32>; 5] = [None, Some(60.), Some(90.), Some(120.), Some(160.)];

fn max_width_label(width: Option<f32>) -> String {
    match width {
        Some(width) => format!("{width:.0}px"),
        None => "Unlimited".into(),
    }
}

fn max_width_action(ix: usize) -> Box<dyn Action> {
    match ix {
        0 => Box::new(DemoToggle::TabsMaxWidthUnlimited),
        1 => Box::new(DemoToggle::TabsMaxWidth60),
        2 => Box::new(DemoToggle::TabsMaxWidth90),
        3 => Box::new(DemoToggle::TabsMaxWidth120),
        _ => Box::new(DemoToggle::TabsMaxWidth160),
    }
}

fn max_width_dropdown(ix: usize) -> DropdownButton {
    DropdownButton::new("tabs-max-width")
        .button(
            Button::new("tabs-max-width-trigger")
                .label(format!("Max width: {}", max_width_label(MAX_WIDTHS[ix]))),
        )
        .dropdown_menu(move |menu: PopupMenu, _, _| {
            MAX_WIDTHS
                .iter()
                .enumerate()
                .fold(menu, |menu, (width_ix, width)| {
                    menu.menu_with_check(
                        max_width_label(*width),
                        width_ix == ix,
                        max_width_action(width_ix),
                    )
                })
        })
}

pub struct TabsSection {
    active_tab_ix: usize,
    dynamic_active_tab_ix: usize,
    dynamic_tabs: Vec<usize>,
    dynamic_next_tab_id: usize,
    dynamic_scroll_handle: ScrollHandle,
    size: Size,
    menu: bool,
    max_width_ix: usize,
}

impl TabsSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            active_tab_ix: 0,
            dynamic_active_tab_ix: 0,
            dynamic_tabs: vec![0, 1, 2],
            dynamic_next_tab_id: 3,
            dynamic_scroll_handle: ScrollHandle::new(),
            size: Size::default(),
            menu: false,
            max_width_ix: 0,
        })
    }

    fn set_active_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        cx.notify();
    }

    fn set_dynamic_active_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.dynamic_active_tab_ix = ix;
        cx.notify();
    }

    fn add_dynamic_tab(&mut self, cx: &mut Context<Self>) {
        let id = self.dynamic_next_tab_id;
        self.dynamic_next_tab_id += 1;
        self.dynamic_tabs.push(id);
        self.dynamic_active_tab_ix = self.dynamic_tabs.len() - 1;
        self.dynamic_scroll_handle
            .scroll_to_item(self.dynamic_active_tab_ix);
        cx.notify();
    }

    fn remove_last_dynamic_tab(&mut self, cx: &mut Context<Self>) {
        if self.dynamic_tabs.len() <= 1 {
            return;
        }

        self.dynamic_tabs.pop();
        if self.dynamic_active_tab_ix >= self.dynamic_tabs.len() {
            self.dynamic_active_tab_ix = self.dynamic_tabs.len() - 1;
        }
        cx.notify();
    }
}

impl Render for TabsSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let more_menu = self.menu;
        // The width menu names its values in px, so the cap stays pixel-based.
        let max_width = MAX_WIDTHS[self.max_width_ix].map(px);
        let max_width_ix = self.max_width_ix;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                } else {
                    match action {
                        DemoToggle::TabsMoreMenu => this.menu = !this.menu,
                        DemoToggle::TabsMaxWidthUnlimited => this.max_width_ix = 0,
                        DemoToggle::TabsMaxWidth60 => this.max_width_ix = 1,
                        DemoToggle::TabsMaxWidth90 => this.max_width_ix = 2,
                        DemoToggle::TabsMaxWidth120 => this.max_width_ix = 3,
                        DemoToggle::TabsMaxWidth160 => this.max_width_ix = 4,
                        _ => {}
                    }
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                size_dropdown("tabs-size", size).into_any_element(),
                max_width_dropdown(max_width_ix).into_any_element(),
                DropdownButton::new("tabs-options")
                    .button(Button::new("tabs-options-trigger").label("Options"))
                    .dropdown_menu(move |menu: PopupMenu, _, _| {
                        menu.menu_with_check(
                            "More menu",
                            more_menu,
                            Box::new(DemoToggle::TabsMoreMenu),
                        )
                    })
                    .into_any_element(),
            ]))
            .child(
                section("tabs-default-box", "Tabs").w_full().child(
                    TabBar::new("tabs-default")
                        .w_full()
                        .with_size(size)
                        .menu(more_menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, _, cx| {
                            this.set_active_tab(*ix, cx);
                        }))
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .prefix(
                            h_flex()
                                .mx_1()
                                .child(
                                    Button::new("tabs-back")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowLeft),
                                )
                                .child(
                                    Button::new("tabs-forward")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowRight),
                                ),
                        )
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License"))
                        .suffix(
                            h_flex()
                                .mx_1()
                                .child(
                                    Button::new("tabs-inbox")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Inbox),
                                )
                                .child(
                                    Button::new("tabs-more")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Ellipsis),
                                ),
                        ),
                ),
            )
            .child(
                section("tabs-underline-box", "Underline Tabs")
                    .w_full()
                    .child(
                        TabBar::new("tabs-underline")
                            .w_full()
                            .underline()
                            .with_size(size)
                            .menu(more_menu)
                            .when_some(max_width, |this, max_width| this.max_width(max_width))
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                this.set_active_tab(*ix, cx);
                            }))
                            .child("Account")
                            .child("Profile")
                            .child("Documents")
                            .child("Mail")
                            .child("Appearance")
                            .child("Settings")
                            .child("About")
                            .child("License"),
                    ),
            )
            .child(
                section("tabs-pill-box", "Pill Tabs").w_full().child(
                    TabBar::new("tabs-pill")
                        .w_full()
                        .pill()
                        .with_size(size)
                        .menu(more_menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, _, cx| {
                            this.set_active_tab(*ix, cx);
                        }))
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents & Files"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License")),
                ),
            )
            .child(
                section("tabs-outline-box", "Outline Tabs").w_full().child(
                    TabBar::new("tabs-outline")
                        .w_full()
                        .outline()
                        .with_size(size)
                        .menu(more_menu)
                        .when_some(max_width, |this, max_width| this.max_width(max_width))
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, _, cx| {
                            this.set_active_tab(*ix, cx);
                        }))
                        .child(Tab::new().label("Account"))
                        .child(Tab::new().label("Profile").disabled(true))
                        .child(Tab::new().label("Documents & Files"))
                        .child(Tab::new().label("Mail"))
                        .child(Tab::new().label("Appearance"))
                        .child(Tab::new().label("Settings"))
                        .child(Tab::new().label("About"))
                        .child(Tab::new().label("License")),
                ),
            )
            .child(
                section("tabs-segmented-box", "Segmented Tabs")
                    .w_full()
                    .child(
                        TabBar::new("tabs-segmented")
                            .w_full()
                            .segmented()
                            .with_size(size)
                            .menu(more_menu)
                            .when_some(max_width, |this, max_width| this.max_width(max_width))
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                this.set_active_tab(*ix, cx);
                            }))
                            .child(IconName::Bot)
                            .child(IconName::Calendar)
                            .child(IconName::Map)
                            .children(vec!["Appearance", "Settings", "About", "License"]),
                    ),
            )
            .child(
                section("tabs-dynamic-box", "Dynamic Tabs")
                    .description(
                        "Tabs can be added, removed, and composed with prefix and suffix content.",
                    )
                    .w_full()
                    .child(
                        ButtonGroup::new("tabs-dynamic-actions")
                            .outline()
                            .compact()
                            .child(
                                Button::new("tabs-add")
                                    .label("Add Tab")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.add_dynamic_tab(cx);
                                    })),
                            )
                            .child(Button::new("tabs-remove").label("Remove Last").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.remove_last_dynamic_tab(cx);
                                }),
                            )),
                    )
                    .child(
                        TabBar::new("tabs-dynamic")
                            .track_scroll(&self.dynamic_scroll_handle)
                            .w_full()
                            .segmented()
                            .with_size(size)
                            .when_some(max_width, |this, max_width| this.max_width(max_width))
                            .selected_index(self.dynamic_active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                this.set_dynamic_active_tab(*ix, cx);
                            }))
                            .children(self.dynamic_tabs.iter().enumerate().map(|(ix, id)| {
                                let label = format!("Tab {id}");
                                Tab::new()
                                    .px_2()
                                    .prefix(Icon::new(IconName::BookOpen))
                                    .label(label)
                                    .suffix(
                                        Button::new(format!("tabs-dynamic-close-{id}"))
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Close),
                                    )
                                    .selected(self.dynamic_active_tab_ix == ix)
                            })),
                    ),
            )
            .child(
                section("tabs-filling-box", "Filling Space")
                    .description("Segmented tabs can share the available width equally.")
                    .w_full()
                    .child(
                        TabBar::new("tabs-filling")
                            .w_full()
                            .segmented()
                            .with_size(size)
                            .when_some(max_width, |this, max_width| this.max_width(max_width))
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                this.set_active_tab(*ix, cx);
                            }))
                            .child(Tab::new().flex_1().label("About"))
                            .child(Tab::new().flex_1().label("Profile")),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "tabs",
        "Tabs",
        "A set of layered sections of content, known as tab panels, displayed one at a time.",
        TabsSection::view(window, cx),
    ));
}
