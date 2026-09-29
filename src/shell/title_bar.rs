use gpui::{
    Anchor, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, Role, SharedString, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _, Theme, TitleBar,
    button::{Button, ButtonVariants as _},
    label::Label,
    menu::{AppMenuBar, DropdownMenu as _},
};

use crate::accessibility::A11yExt;
use crate::app::{SelectFont, SelectRadius};
use crate::menus;

pub struct AppTitleBar {
    title: SharedString,
    app_menu_bar: Entity<AppMenuBar>,
    settings: Entity<SettingsDropdown>,
}

impl AppTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title: SharedString = title.into();
        let app_menu_bar = menus::init(title.clone(), cx);
        let settings = cx.new(|cx| SettingsDropdown::new(window, cx));

        Self {
            title,
            app_menu_bar,
            settings,
        }
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Window controls eat a fixed ~105px beside the title bar, so at small
        // viewports the decorative label is the first thing to yield space.
        let show_theme_label = window.viewport_size().width >= px(640.);
        div()
            .id("title-bar")
            .a11y(Role::TitleBar, self.title.clone())
            .child(
                TitleBar::new()
                    .child(
                        // The menu bar scrolls internally once width-bound;
                        // without flex_1/min_w_0 its content width pushes the
                        // right cluster out of the window at small sizes.
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_2()
                            .child(self.app_menu_bar.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .px_2()
                            .gap_2()
                            .flex_shrink_0()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .when(show_theme_label, |this| {
                                this.child(
                                    Label::new("theme:")
                                        .secondary(cx.theme().theme_name())
                                        .text_sm(),
                                )
                            })
                            .child(self.settings.clone())
                            .child(
                                Button::new("search")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Search)
                                    .accessibility_label(search_a11y_label())
                                    .on_click(|_, _, cx| {
                                        crate::launcher::open_launcher(cx);
                                    }),
                            )
                            .child(
                                Button::new("bell")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Bell)
                                    .accessibility_label("Notifications")
                                    .on_click(|_, _, cx| {
                                        crate::events::emit(
                                            crate::events::AppEventKind::Navigate(
                                                crate::routes::AppRoute::page(
                                                    crate::sidebar::Page::Notifications,
                                                ),
                                            ),
                                            cx,
                                        );
                                    }),
                            ),
                    ),
            )
    }
}

struct SettingsDropdown {
    focus_handle: FocusHandle,
}

impl SettingsDropdown {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_font(
        &mut self,
        font_size: &SelectFont,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // update() rebuilds the Base projection and refreshes every window,
        // not just this one; a manual window.refresh() would be partial.
        Theme::update(cx, |theme| theme.font_size = px(font_size.0 as f32));
    }

    fn on_select_radius(
        &mut self,
        radius: &SelectRadius,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::update(cx, |theme| {
            theme.radius = px(radius.0 as f32);
            theme.radius_lg = if theme.radius > px(0.) {
                theme.radius + px(2.)
            } else {
                px(0.)
            };
        });
    }
}

impl Render for SettingsDropdown {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let font_size = cx.theme().font_size.as_f32() as i32;
        let radius = cx.theme().radius.as_f32() as i32;

        div()
            .id("settings-dropdown")
            .a11y(Role::Group, "Appearance")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_font))
            .on_action(cx.listener(Self::on_select_radius))
            .child(
                Button::new("settings-btn")
                    .small()
                    .ghost()
                    .icon(IconName::Settings2)
                    .accessibility_label("Appearance")
                    .dropdown_menu(move |menu, _window, _cx| {
                        menu.scrollable(true)
                            .label("Font Size")
                            .menu_with_check("Large", font_size == 18, Box::new(SelectFont(18)))
                            .menu_with_check(
                                "Medium (default)",
                                font_size == 16,
                                Box::new(SelectFont(16)),
                            )
                            .menu_with_check("Small", font_size == 14, Box::new(SelectFont(14)))
                            .separator()
                            .label("Border Radius")
                            .menu_with_check("8px", radius == 8, Box::new(SelectRadius(8)))
                            .menu_with_check(
                                "6px (default)",
                                radius == 6,
                                Box::new(SelectRadius(6)),
                            )
                            .menu_with_check("4px", radius == 4, Box::new(SelectRadius(4)))
                            .menu_with_check("0px", radius == 0, Box::new(SelectRadius(0)))
                    })
                    .anchor(Anchor::TopRight),
            )
    }
}

/// Announced name for the icon-only search button, derived from the
/// ToggleSearch binding so the modifier matches the platform keymap.
fn search_a11y_label() -> String {
    format!(
        "Search ({})",
        crate::app::keys::label(crate::app::keys::TOGGLE_SEARCH)
    )
}
