//! Sidebar section, ported from the upstream `SidebarStory` and the
//! `sidebar` example (its collapsible-modes demo).

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Side, Sizable as _, StyledExt as _,
    ThemeStyled as _,
    badge::Badge,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::{Button, DropdownButton},
    h_flex,
    menu::DropdownMenu as _,
    red_500,
    separator::Separator,
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
        SidebarMenuItem, SidebarToggleButton,
    },
    switch::Switch,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// Payload of the header company dropdown (upstream `SelectCompany`).
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_chrome, no_json)]
pub(crate) struct SelectCompany(SharedString);

/// Knobs of the upstream `SidebarStory` options menu.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_chrome, no_json)]
pub(crate) enum SidebarOption {
    Icon,
    Offcanvas,
    None,
    Right,
    ClickToOpen,
    DynamicChildren,
}

const GROUPS: [&[Item]; 2] = [
    &[
        Item::Playground,
        Item::Models,
        Item::Documentation,
        Item::Settings,
    ],
    &[
        Item::DesignEngineering,
        Item::SalesAndMarketing,
        Item::Travel,
    ],
];

pub struct SidebarSection {
    last_active_item: Item,
    active_subitem: Option<SubItem>,
    collapsed: bool,
    collapsible: SidebarCollapsible,
    side: Side,
    click_to_open_submenu: bool,
    show_dynamic_children: bool,
    checked: bool,
    company: SharedString,
    modes_collapsible: SidebarCollapsible,
    modes_collapsed: bool,
}

impl SidebarSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            last_active_item: Item::Playground,
            active_subitem: None,
            collapsed: false,
            collapsible: SidebarCollapsible::Icon,
            side: Side::Left,
            click_to_open_submenu: false,
            show_dynamic_children: false,
            checked: false,
            company: SharedString::from("Company Name"),
            modes_collapsible: SidebarCollapsible::Icon,
            modes_collapsed: false,
        })
    }

    /// Menu of the collapsible-modes demo, from the `sidebar` example.
    fn menu() -> SidebarMenu {
        SidebarMenu::new().children([
            SidebarMenuItem::new("Dashboard")
                .icon(IconName::LayoutDashboard)
                .active(true),
            SidebarMenuItem::new("Inbox").icon(IconName::Inbox),
            SidebarMenuItem::new("Calendar").icon(IconName::Calendar),
            SidebarMenuItem::new("Projects")
                .icon(IconName::Folder)
                .default_open(true)
                .click_to_toggle(true)
                .children([
                    SidebarMenuItem::new("Design"),
                    SidebarMenuItem::new("Engineering"),
                    SidebarMenuItem::new("Marketing"),
                ]),
            SidebarMenuItem::new("Settings").icon(IconName::Settings),
        ])
    }

    /// Per-mode copy of the collapsible-modes demo, from the `sidebar` example.
    fn modes_description(&self) -> &'static str {
        match self.modes_collapsible {
            SidebarCollapsible::Icon => {
                "The sidebar collapses to icon width, matching shadcn's collapsible=\"icon\" behavior."
            }
            SidebarCollapsible::Offcanvas => {
                "The sidebar releases its layout width when collapsed and keeps hidden controls out of keyboard navigation, matching shadcn's collapsible=\"offcanvas\" behavior."
            }
            SidebarCollapsible::None => {
                "The sidebar ignores the collapsed state and remains expanded, matching shadcn's collapsible=\"none\" behavior."
            }
        }
    }

    fn mode_button(
        &self,
        id: &'static str,
        label: &'static str,
        mode: SidebarCollapsible,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .small()
            .selected(self.modes_collapsible == mode)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.modes_collapsible = mode;
                cx.notify();
            }))
    }

    fn switch_checked_handler(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.checked = *checked;
        cx.notify();
    }

    fn render_content(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .min_h_0()
            .flex_1()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_semibold()
                                    .child(self.last_active_item.label()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("A quick view of your workspace activity."),
                            ),
                    ),
            )
            .child(
                h_flex().w_full().gap_3().children(
                    [
                        ("Active projects", "12", "+2 this week"),
                        ("Team members", "28", "4 online"),
                        ("Tasks completed", "84%", "+6% this month"),
                    ]
                    .into_iter()
                    .map(|(label, value, detail)| {
                        v_flex()
                            .min_w_0()
                            .flex_1()
                            .gap_2()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(label),
                            )
                            .child(div().text_2xl().font_semibold().child(value))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(detail),
                            )
                    }),
                ),
            )
            .child(
                v_flex()
                    .w_full()
                    .min_h_0()
                    .flex_1()
                    .mt_2()
                    .rounded(cx.theme().radius_lg)
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .child(div().font_medium().child("Recent activity"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Today"),
                            ),
                    )
                    .child(Separator::horizontal())
                    .children(
                        [
                            (
                                IconName::CircleCheck,
                                "Design review completed",
                                "12 minutes ago",
                            ),
                            (IconName::File, "Project brief updated", "1 hour ago"),
                            (
                                IconName::CircleUser,
                                "Maya joined the workspace",
                                "3 hours ago",
                            ),
                        ]
                        .into_iter()
                        .map(|(icon, title, time)| {
                            h_flex()
                                .items_center()
                                .gap_3()
                                .px_4()
                                .py_3()
                                .child(
                                    div()
                                        .size_8()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_full_style(cx)
                                        .bg(cx.theme().muted)
                                        .child(Icon::new(icon).small()),
                                )
                                .child(
                                    v_flex()
                                        .min_w_0()
                                        .flex_1()
                                        .gap_0p5()
                                        .child(div().text_sm().child(title))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(time),
                                        ),
                                )
                        }),
                    ),
            )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Playground,
    Models,
    Documentation,
    Settings,
    DesignEngineering,
    SalesAndMarketing,
    Travel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubItem {
    History,
    Starred,
    General,
    Team,
    Billing,
    Limits,
    Settings,
    Genesis,
    Explorer,
    Quantum,
    Introduction,
    GetStarted,
    Tutorial,
    Changelog,
}

impl Item {
    fn label(&self) -> &'static str {
        match self {
            Self::Playground => "Playground",
            Self::Models => "Models",
            Self::Documentation => "Documentation",
            Self::Settings => "Settings",
            Self::DesignEngineering => "Design Engineering",
            Self::SalesAndMarketing => "Sales and Marketing",
            Self::Travel => "Travel",
        }
    }

    fn is_disabled(&self) -> bool {
        matches!(self, Self::Travel)
    }

    fn icon(&self) -> IconName {
        match self {
            Self::Playground => IconName::SquareTerminal,
            Self::Models => IconName::Bot,
            Self::Documentation => IconName::BookOpen,
            Self::Settings => IconName::Settings2,
            Self::DesignEngineering => IconName::Frame,
            Self::SalesAndMarketing => IconName::ChartPie,
            Self::Travel => IconName::Map,
        }
    }

    fn handler(
        &self,
    ) -> impl Fn(&mut SidebarSection, &ClickEvent, &mut Window, &mut Context<SidebarSection>) + 'static
    {
        let item = *self;
        move |this, _, _, cx| {
            this.last_active_item = item;
            this.active_subitem = None;
            cx.notify();
        }
    }

    fn items(&self) -> Vec<SubItem> {
        match self {
            Self::Playground => vec![SubItem::History, SubItem::Starred, SubItem::Settings],
            Self::Models => vec![SubItem::Genesis, SubItem::Explorer, SubItem::Quantum],
            Self::Documentation => vec![
                SubItem::Introduction,
                SubItem::GetStarted,
                SubItem::Tutorial,
                SubItem::Changelog,
            ],
            Self::Settings => vec![
                SubItem::General,
                SubItem::Team,
                SubItem::Billing,
                SubItem::Limits,
            ],
            _ => Vec::new(),
        }
    }
}

impl SubItem {
    fn label(&self) -> &'static str {
        match self {
            Self::History => "History",
            Self::Starred => "Starred",
            Self::Settings => "Settings",
            Self::Genesis => "Genesis",
            Self::Explorer => "Explorer",
            Self::Quantum => "Quantum",
            Self::Introduction => "Introduction",
            Self::GetStarted => "Get Started",
            Self::Tutorial => "Tutorial",
            Self::Changelog => "Changelog",
            Self::Team => "Team",
            Self::Billing => "Billing",
            Self::Limits => "Limits",
            Self::General => "General",
        }
    }

    fn is_disabled(&self) -> bool {
        matches!(self, Self::Quantum)
    }

    fn handler(
        &self,
        item: &Item,
    ) -> impl Fn(&mut SidebarSection, &ClickEvent, &mut Window, &mut Context<SidebarSection>) + 'static
    {
        let item = *item;
        let subitem = *self;
        move |this, _, _, cx| {
            this.last_active_item = item;
            this.active_subitem = Some(subitem);
            cx.notify();
        }
    }
}

impl Render for SidebarSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsible = self.collapsible;
        let icon_collapsed = self.collapsed && collapsible == SidebarCollapsible::Icon;
        let toggle_collapsed = self.collapsed && collapsible != SidebarCollapsible::None;
        let modes_collapsible = self.modes_collapsible;
        let modes_collapsed = self.modes_collapsed;
        let modes_icon_collapsed = modes_collapsed && modes_collapsible == SidebarCollapsible::Icon;
        let modes_show_toggle = modes_collapsible != SidebarCollapsible::None;
        let company = self.company.clone();

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &SidebarOption, _, cx| {
                match action {
                    SidebarOption::Icon => this.collapsible = SidebarCollapsible::Icon,
                    SidebarOption::Offcanvas => this.collapsible = SidebarCollapsible::Offcanvas,
                    SidebarOption::None => this.collapsible = SidebarCollapsible::None,
                    SidebarOption::Right => {
                        this.side = if this.side.is_right() {
                            Side::Left
                        } else {
                            Side::Right
                        }
                    }
                    SidebarOption::ClickToOpen => {
                        this.click_to_open_submenu = !this.click_to_open_submenu
                    }
                    SidebarOption::DynamicChildren => {
                        this.show_dynamic_children = !this.show_dynamic_children
                    }
                }
                cx.notify();
            }))
            .on_action(
                cx.listener(|this, action: &SelectCompany, _, cx| {
                    this.company = action.0.clone();
                    cx.notify();
                }),
            )
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_1()
                    .child(
                        DropdownButton::new("sidebar-options").button(
                            Button::new("sidebar-options-trigger").label("Options"),
                        ),
                    ),
            )
            .child(
                section("sidebar-demo-box", "Workspace")
                    .description(
                        "A header with a company switcher, two menu groups, a footer, submenus, suffixes, and a content pane.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        // The gallery pane is an unbounded column; the demo
                        // frame bounds the sidebar's h_full and the pane's flex.
                        h_flex()
                            .w_full()
                            .h(rems(37.5))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .when(self.side.is_right(), |this| this.flex_row_reverse())
                            .child(
                                // A pixel width drives the collapse animation's
                                // width transition; non-pixel widths skip it.
                                Sidebar::new("sidebar-demo")
                                    .side(self.side)
                                    .collapsible(collapsible)
                                    .collapsed(self.collapsed)
                                    .w(px(220.))
                                    .gap_0()
                                    .header(
                                        SidebarHeader::new()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(cx.theme().radius)
                                                    .bg(cx.theme().success)
                                                    .text_color(cx.theme().success_foreground)
                                                    .size_8()
                                                    .flex_shrink_0()
                                                    .when(!icon_collapsed, |this| {
                                                        this.child(Icon::new(
                                                            IconName::GalleryVerticalEnd,
                                                        ))
                                                    })
                                                    .when(icon_collapsed, |this| {
                                                        this.size_4()
                                                            .bg(cx.theme().transparent)
                                                            .text_color(cx.theme().foreground)
                                                            .child(Icon::new(
                                                                IconName::GalleryVerticalEnd,
                                                            ))
                                                    }),
                                            )
                                            .when(!icon_collapsed, |this| {
                                                this.child(
                                                    v_flex()
                                                        .gap_0()
                                                        .text_sm()
                                                        .flex_1()
                                                        .line_height(relative(1.25))
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .child(company)
                                                        .child(
                                                            div().child("Enterprise").text_xs(),
                                                        ),
                                                )
                                            })
                                            .when(!icon_collapsed, |this| {
                                                this.child(
                                                    Icon::new(IconName::ChevronsUpDown)
                                                        .size_4()
                                                        .flex_shrink_0(),
                                                )
                                            })
                                            .dropdown_menu(|menu, _, _| {
                                                menu.menu(
                                                    "Twitter Inc.",
                                                    Box::new(SelectCompany(SharedString::from(
                                                        "Twitter Inc.",
                                                    ))),
                                                )
                                                .menu(
                                                    "Meta Platforms",
                                                    Box::new(SelectCompany(SharedString::from(
                                                        "Meta Platforms",
                                                    ))),
                                                )
                                                .menu(
                                                    "Google Inc.",
                                                    Box::new(SelectCompany(SharedString::from(
                                                        "Google Inc.",
                                                    ))),
                                                )
                                            }),
                                    )
                                    .child(
                                        SidebarGroup::new("Platform").child(SidebarMenu::new().children(
                                            GROUPS[0].iter().enumerate().map(|(ix, item)| {
                                                let is_active = self.last_active_item == *item
                                                    && self.active_subitem.is_none();
                                                SidebarMenuItem::new(item.label())
                                                    .icon(item.icon())
                                                    .active(is_active)
                                                    .default_open(ix == 0)
                                                    .click_to_open(self.click_to_open_submenu)
                                                    // Upstream outlined the active item in
                                                    // white; foreground stays theme-aware.
                                                    .when(is_active, |this| {
                                                        this.border_1().border_color(
                                                            cx.theme().foreground,
                                                        )
                                                    })
                                                    .when(ix == 0, |this| {
                                                        this.context_menu(|menu, _, _| {
                                                            menu.link(
                                                                "About",
                                                                "https://github.com/longbridge/gpui-kit",
                                                            )
                                                        })
                                                    })
                                                    .children(
                                                        item.items().into_iter().enumerate().map(
                                                            |(ix, sub_item)| {
                                                                SidebarMenuItem::new(
                                                                    sub_item.label(),
                                                                )
                                                                .active(
                                                                    self.active_subitem
                                                                        == Some(sub_item),
                                                                )
                                                                .disable(sub_item.is_disabled())
                                                                .when(ix == 0, |this| {
                                                                    this.suffix({
                                                                        let checked =
                                                                            self.checked;
                                                                        let view = cx.entity();
                                                                        move |window, _| {
                                                                            Switch::new(
                                                                                "sidebar-switch",
                                                                            )
                                                                            .xsmall()
                                                                            .checked(
                                                                                checked,
                                                                            )
                                                                            .on_click(
                                                                                window
                                                                                    .listener_for(
                                                                                        &view,
                                                                                        Self::switch_checked_handler,
                                                                                    ),
                                                                            )
                                                                        }
                                                                    })
                                                                    .label_style(
                                                                        StyleRefinement::default()
                                                                            .text_color(red_500()),
                                                                    )
                                                                    .context_menu(
                                                                        |menu, _, _| {
                                                                            menu.label(
                                                                                "This is a label",
                                                                            )
                                                                        },
                                                                    )
                                                                })
                                                                .on_click(cx.listener(
                                                                    sub_item.handler(item),
                                                                ))
                                                            },
                                                        ),
                                                    )
                                                    .on_click(cx.listener(item.handler()))
                                            }),
                                        )),
                                    )
                                    .child(
                                        SidebarGroup::new("Projects").child(SidebarMenu::new().children(
                                            GROUPS[1].iter().enumerate().map(|(ix, item)| {
                                                let is_active = self.last_active_item == *item
                                                    && self.active_subitem.is_none();
                                                SidebarMenuItem::new(item.label())
                                                    .icon(item.icon())
                                                    .active(is_active)
                                                    .disable(item.is_disabled())
                                                    .click_to_open(self.click_to_open_submenu)
                                                    .when(
                                                        ix == 0 && self.show_dynamic_children,
                                                        |this| {
                                                            this.default_open(true).children(
                                                                vec![
                                                                    SidebarMenuItem::new("Child A"),
                                                                    SidebarMenuItem::new("Child B"),
                                                                ],
                                                            )
                                                        },
                                                    )
                                                    .when(ix == 0, |this| {
                                                        this.suffix(|_, _| {
                                                            Badge::new().dot().count(1).child(
                                                                div()
                                                                    .p_0p5()
                                                                    .child(Icon::new(
                                                                        IconName::Bell,
                                                                    )),
                                                            )
                                                        })
                                                    })
                                                    .when(ix == 1, |this| {
                                                        this.suffix(|_, _| {
                                                            Icon::new(IconName::Settings2)
                                                        })
                                                    })
                                                    .on_click(cx.listener(item.handler()))
                                            }),
                                        )),
                                    )
                                    .footer(
                                        SidebarFooter::new()
                                            .justify_between()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(IconName::CircleUser)
                                                    .when(!icon_collapsed, |this| {
                                                        this.child("Jason Lee")
                                                    }),
                                            )
                                            .when(!icon_collapsed, |this| {
                                                this.child(
                                                    Icon::new(IconName::ChevronsUpDown).size_4(),
                                                )
                                            }),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .h_full()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .gap_4()
                                    .p_4()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .when(
                                                self.side.is_right()
                                                    && collapsible
                                                        != SidebarCollapsible::None,
                                                |this| {
                                                    this.flex_row_reverse().justify_between()
                                                },
                                            )
                                            .when(
                                                collapsible != SidebarCollapsible::None,
                                                |this| {
                                                    this.child(
                                                        SidebarToggleButton::new()
                                                            .side(self.side)
                                                            .collapsed(toggle_collapsed)
                                                            .on_click(cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.collapsed =
                                                                        !this.collapsed;
                                                                    cx.notify();
                                                                },
                                                            )),
                                                    )
                                                    .child(Separator::vertical().h_4())
                                                },
                                            )
                                            .child(
                                                Breadcrumb::new()
                                                    .child("Breadcrumb")
                                                    .child(
                                                        BreadcrumbItem::new("Home").on_click(
                                                            cx.listener(
                                                                |this, _, _, cx| {
                                                                    this.last_active_item =
                                                                        Item::Playground;
                                                                    cx.notify();
                                                                },
                                                            ),
                                                        ),
                                                    )
                                                    .child(
                                                        BreadcrumbItem::new(
                                                            self.last_active_item.label(),
                                                        )
                                                        .on_click(cx.listener(
                                                            |this, _, _, cx| {
                                                                this.active_subitem = None;
                                                                cx.notify();
                                                            },
                                                        )),
                                                    )
                                                    .when_some(
                                                        self.active_subitem,
                                                        |this, subitem| {
                                                            this.child(
                                                                BreadcrumbItem::new(
                                                                    subitem.label(),
                                                                ),
                                                            )
                                                        },
                                                    ),
                                            ),
                                    )
                                    .child(self.render_content(window, cx)),
                            ),
                    ),
            )
            .child(
                section("sidebar-modes-box", "Collapsible modes")
                    .description(
                        "Collapse to icon width, release the layout width, or ignore collapse entirely.",
                    )
                    .v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .w_full()
                            .h(rems(25.))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .overflow_hidden()
                            .child(
                                Sidebar::new("sidebar-modes")
                                    .collapsible(modes_collapsible)
                                    .collapsed(modes_collapsed)
                                    .w(px(240.))
                                    .header(
                                        SidebarHeader::new()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .size_8()
                                                    .flex_shrink_0()
                                                    .rounded(cx.theme().radius)
                                                    .bg(cx.theme().sidebar_primary)
                                                    .text_color(cx.theme().sidebar_primary_foreground)
                                                    .when(modes_icon_collapsed, |this| {
                                                        this.size_4()
                                                            .bg(cx.theme().transparent)
                                                            .text_color(cx.theme().foreground)
                                                    })
                                                    .child(Icon::new(IconName::GalleryVerticalEnd)),
                                            )
                                            .when(!modes_icon_collapsed, |this| {
                                                this.child(
                                                    v_flex()
                                                        .flex_1()
                                                        .overflow_hidden()
                                                        .child("Acme Inc")
                                                        .child(
                                                            div().text_xs().child("Enterprise"),
                                                        ),
                                                )
                                            }),
                                    )
                                    .child(
                                        SidebarGroup::new("Application").child(Self::menu()),
                                    )
                                    .footer(
                                        SidebarFooter::new().child(
                                            h_flex()
                                                .gap_2()
                                                .child(IconName::CircleUser)
                                                .when(!modes_icon_collapsed, |this| {
                                                    this.child("Jason Lee")
                                                }),
                                        ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .h_full()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_4()
                                    .p_4()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .when(modes_show_toggle, |this| {
                                                this.child(
                                                    SidebarToggleButton::new()
                                                        .collapsed(modes_icon_collapsed)
                                                        .on_click(cx.listener(
                                                            |this, _, _, cx| {
                                                                this.modes_collapsed =
                                                                    !this.modes_collapsed;
                                                                cx.notify();
                                                            },
                                                        )),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .font_bold()
                                                    .child("Sidebar collapsible modes"),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(div().text_sm().child("Mode:"))
                                            .child(self.mode_button(
                                                "sidebar-mode-icon",
                                                "Icon",
                                                SidebarCollapsible::Icon,
                                                cx,
                                            ))
                                            .child(self.mode_button(
                                                "sidebar-mode-offcanvas",
                                                "Offcanvas",
                                                SidebarCollapsible::Offcanvas,
                                                cx,
                                            ))
                                            .child(self.mode_button(
                                                "sidebar-mode-none",
                                                "None",
                                                SidebarCollapsible::None,
                                                cx,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .rounded(cx.theme().radius)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .p_5()
                                            .child(self.modes_description()),
                                    ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "sidebar",
        "Sidebar",
        "A composable, themeable and customizable sidebar component.",
        SidebarSection::view(window, cx),
    ));
}
