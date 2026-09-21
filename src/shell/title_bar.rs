use gpui::{
    Anchor, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, Role, SharedString, Styled as _, Window, div, px,
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("title-bar")
            .a11y(Role::TitleBar, self.title.clone())
            .child(
                TitleBar::new()
                    .child(
                        div()
                            .flex()
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
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                Label::new("theme:")
                                    .secondary(cx.theme().theme_name())
                                    .text_sm(),
                            )
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

// ---------------------------------------------------------------------------
// Settings dropdown (font size, radius, scrollbar)
// ---------------------------------------------------------------------------

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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).font_size = px(font_size.0 as f32);
        window.refresh();
    }

    fn on_select_radius(
        &mut self,
        radius: &SelectRadius,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).radius = px(radius.0 as f32);
        Theme::global_mut(cx).radius_lg = if cx.theme().radius > px(0.) {
            cx.theme().radius + px(2.)
        } else {
            px(0.)
        };
        window.refresh();
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

/// Announced name for the icon-only search button, matching the cmd-k
/// ToggleSearch binding ("cmd" is the platform modifier in gpui keystrokes).
#[cfg(target_os = "macos")]
fn search_a11y_label() -> &'static str {
    "Search (Cmd+K)"
}

#[cfg(not(target_os = "macos"))]
fn search_a11y_label() -> &'static str {
    "Search (Super+K)"
}
