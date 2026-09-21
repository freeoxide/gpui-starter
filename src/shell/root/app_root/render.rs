use std::rc::Rc;

use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Collapsible, Icon, IconName, Sizable as _,
    menu::PopupMenu,
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarItem, SidebarMenuItem},
    v_flex,
};

use crate::accessibility::A11yExt;
use crate::app::ToggleSearch;
use crate::routes::AppRoute;
use crate::sidebar::Page;
use crate::views::{ReloadCurrentPage, RenderErrorPage, TriggerRenderError};

use super::super::actions::{NavigateToPage, RefreshPage, is_rtl_locale};
use super::super::frame_time;
use super::state::AppRoot;

impl Render for AppRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // gpui has no a11y activation callback (it refreshes the window on AT
        // connect/disconnect); deferred so refresh runs at App level.
        if window.is_a11y_active() != crate::accessibility::snapshot(cx).bridge_enabled {
            cx.defer(crate::accessibility::refresh);
        }

        // clock (not std::time): Instant::now panics at runtime on wasm.
        let render_started = crate::platform::clock::Instant::now();
        let sheet_layer = gpui_component::Root::render_sheet_layer(window, cx);
        let dialog_layer = gpui_component::Root::render_dialog_layer(window, cx);
        let notification_layer = gpui_component::Root::render_notification_layer(window, cx);
        let page_title = if self.render_error {
            "Render Error"
        } else {
            self.active_route.title()
        };
        let active_page = self.active_route.page_for_render();
        let rtl = is_rtl_locale(&crate::app::current_locale(cx));

        let sidebar =
            Sidebar::new("app-sidebar")
                .w(relative(1.))
                .border_0()
                .collapsed(self.collapsed)
                .header(
                    v_flex().w_full().gap_4().child(
                        SidebarHeader::new().w_full().child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(cx.theme().radius_lg)
                                .bg(cx.theme().primary)
                                .text_color(cx.theme().primary_foreground)
                                .size_8()
                                .flex_shrink_0()
                                .child(Icon::new(IconName::Star)),
                        ),
                    ),
                )
                .child(SidebarGroup::new("Navigation").children(
                    Page::all().iter().enumerate().map(|(ix, page)| {
                        let page = *page;
                        let context_menu: Rc<
                            dyn Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu,
                        > = Rc::new(move |menu, _window, _cx| {
                            // Context menu: right-click on sidebar items.
                            menu.menu_with_icon(
                                "Navigate",
                                Icon::new(IconName::ArrowRight),
                                Box::new(NavigateToPage(page as usize)),
                            )
                            .separator()
                            .menu_with_icon(
                                "Refresh",
                                Icon::new(IconName::Redo2),
                                Box::new(RefreshPage),
                            )
                            .separator()
                            .menu_with_icon(
                                "Settings",
                                Icon::new(IconName::Settings2),
                                Box::new(NavigateToPage(Page::Settings as usize)),
                            )
                        });
                        NavItem {
                            page,
                            active: !self.render_error && active_page == page,
                            collapsed: false,
                            // accesskit stores position_in_set 0-based; AT
                            // bridges report the stored value +1.
                            position: ix,
                            total: Page::all().len(),
                            on_click: Rc::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.set_route(AppRoute::page(page), cx);
                            })),
                            context_menu,
                        }
                    }),
                ));

        // RTL: reverse sidebar position and flex direction
        let sidebar_panel = resizable_panel()
            .size(px(255.))
            .size_range(px(60.)..px(320.))
            .child(
                div()
                    .id("sidebar-nav")
                    .a11y(Role::Navigation, "Main navigation")
                    // AT-SPI derives each item's setsize from the nearest
                    // ancestor that declares one; item-level values are ignored.
                    .aria_size_of_set(Page::all().len())
                    .child(sidebar),
            );

        let content_panel = resizable_panel().child(
            v_flex()
                .id("main")
                .role(Role::Main)
                .flex_1()
                .h_full()
                .overflow_x_hidden()
                .child(
                    div()
                        .id("header")
                        .p_4()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .id("page-title")
                                .a11y(Role::Heading, page_title)
                                .aria_level(1)
                                // Navigation moves no keyboard focus, so the
                                // new page title is announced via this live region.
                                .a11y_live(accesskit::Live::Polite)
                                .text_xl()
                                .font_weight(FontWeight::BOLD)
                                .child(page_title),
                        ),
                )
                .child(
                    div()
                        .id("page")
                        .a11y(Role::Region, page_title)
                        .flex_1()
                        .overflow_y_scroll()
                        .child({
                            let _render_guard = crate::lifecycle::enter_render_path();
                            self.active_page_view(cx)
                        }),
                ),
        );

        // In RTL locales the sidebar appears on the right; swap panel order.
        let mut layout = h_resizable("app-layout");
        if rtl {
            layout = layout.child(content_panel).child(sidebar_panel);
        } else {
            layout = layout.child(sidebar_panel).child(content_panel);
        }

        let content_area = div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|_, _: &ToggleSearch, _, cx| {
                crate::launcher::open_launcher(cx);
            }))
            // Cmd+1..9 → NavigateToPage handler
            .on_action(cx.listener(|this, action: &NavigateToPage, _, cx| {
                let pages = Page::all();
                if let Some(&page) = pages.get(action.0) {
                    this.set_route(AppRoute::page(page), cx);
                }
            }))
            // Context menu action handlers
            .on_action(cx.listener(|this, _: &RefreshPage, _, cx| {
                let current = this.active_route.page_for_render();
                // Force a re-render by calling notify, since set_route
                // no-ops when the route is unchanged.
                cx.notify();
                tracing::info!(target: "gpui_starter::root", page = ?current, "page refreshed");
            }))
            // Error boundary: reload clears the error state and retries the page.
            .on_action(cx.listener(|this, _: &ReloadCurrentPage, _, cx| {
                tracing::info!(
                    target: "gpui_starter::root",
                    "reloading page after render error"
                );
                this.render_error = false;
                this.error_page = None;
                cx.notify();
            }))
            // Test the boundary UI without a real render panic (fatal in GPUI).
            .on_action(cx.listener(|this, action: &TriggerRenderError, _, cx| {
                tracing::info!(
                    target: "gpui_starter::root",
                    message = %action.message,
                    "error boundary activated via TriggerRenderError action"
                );
                this.render_error = true;
                this.error_page = Some(cx.new(|_| RenderErrorPage::new(action.message.clone())));
                cx.notify();
            }))
            .flex_1()
            .overflow_hidden()
            .child(layout);

        let elapsed_us = render_started.elapsed().as_micros() as u64;

        // Persist frame time for the status-bar readout.
        frame_time::store_frame_time(elapsed_us);

        if frame_time::is_slow_frame(elapsed_us) {
            tracing::warn!(
                target: "gpui_starter::root::render",
                route = %self.active_route.title(),
                elapsed_us,
                threshold_us = frame_time::SLOW_FRAME_THRESHOLD_US,
                "slow frame detected"
            );
        }

        v_flex()
            .size_full()
            .child(self.title_bar.clone())
            .child(content_area)
            .child(crate::status_bar::render(&self.active_route, cx))
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}

/// One sidebar page entry: wraps [`SidebarMenuItem`] with the accessibility
/// node the component does not create — its clickable div has no role, so
/// without this wrapper the items are absent from the a11y tree entirely.
#[derive(Clone)]
struct NavItem {
    page: Page,
    active: bool,
    collapsed: bool,
    position: usize,
    total: usize,
    on_click: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
    context_menu: Rc<dyn Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu>,
}

impl Collapsible for NavItem {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl SidebarItem for NavItem {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();
        let label = self.page.title();
        let item_on_click = self.on_click.clone();
        let activate = self.on_click;
        let context_menu = self.context_menu.clone();
        let item = SidebarMenuItem::new(label)
            .icon(Icon::new(self.page.icon()).small())
            .active(self.active)
            .collapsed(self.collapsed)
            .on_click(move |ev, window, cx| item_on_click(ev, window, cx))
            .context_menu(move |menu, window, cx| context_menu(menu, window, cx));

        div()
            .id(id.clone())
            .a11y(Role::ListItem, label)
            .aria_selected(self.active)
            .aria_position_in_set(self.position)
            .aria_size_of_set(self.total)
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                activate(&ClickEvent::default(), window, cx);
            })
            .child(item.render(id, window, cx))
    }
}
