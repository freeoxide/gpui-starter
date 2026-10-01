//! Native Menu section, ported from the upstream `NativeMenuStory`: a menu
//! the operating system renders where the platform provides one, with a drawn
//! fallback everywhere else.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, ElementExt, Icon, IconName, button::Button, native_menu::NativeMenu, v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// Dispatched by native menu items; "Word Wrap" updates its checked state.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_menus_native, no_json)]
struct MenuClick(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_menus_native, no_json)]
struct OpenGitHub;

const CONTEXT: &str = "gallery-native-menu";

/// A menu item dispatching `MenuClick(label)`.
fn click(label: &str) -> Box<dyn Action> {
    Box::new(MenuClick(label.to_string().into()))
}

/// Demo menu: normal items, a disabled item, a checked item (reflecting
/// `word_wrap`, which the section toggles), and nested submenus.
fn demo_menu(word_wrap: bool) -> NativeMenu {
    NativeMenu::new()
        .menu("Cut", click("Cut"))
        .menu("Copy", click("Copy"))
        .menu("Paste", click("Paste"))
        .separator()
        .menu_with_icon("Github", IconName::Github, Box::new(OpenGitHub))
        .menu_with_icon("Inbox", IconName::Inbox, click("Inbox"))
        .menu_with_icon(
            "Search (SVG bytes)",
            Icon::default().data(include_bytes!("assets/search.svg")),
            click("Search"),
        )
        .separator()
        .menu_with_disabled("Disabled item", true, click("Disabled"))
        .menu_with_check("Word Wrap", word_wrap, click("Word Wrap"))
        .separator()
        .submenu(
            "Open Recent",
            NativeMenu::new()
                .menu("project-a", click("project-a"))
                .menu("project-b", click("project-b"))
                .separator()
                .submenu(
                    "More",
                    NativeMenu::new()
                        .menu("project-c", click("project-c"))
                        .menu("project-d", click("project-d")),
                ),
        )
        .separator()
        .menu("Select All", click("Select All"))
}

pub struct NativeMenuSection {
    focus_handle: FocusHandle,
    /// Demo checked state, toggled when the "Word Wrap" item is selected.
    word_wrap: bool,
}

impl NativeMenuSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            word_wrap: true,
        })
    }

    fn on_click(&mut self, click: &MenuClick, _: &mut Window, cx: &mut Context<Self>) {
        if click.0.as_ref() == "Word Wrap" {
            self.word_wrap = !self.word_wrap;
        }
        cx.notify();
    }

    fn open_github(&mut self, _: &OpenGitHub, _: &mut Window, cx: &mut Context<Self>) {
        cx.open_url("https://github.com");
    }

    fn trigger(&self, label: &str, cx: &mut App) -> Div {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_full()
            .h_24()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .text_color(cx.theme().muted_foreground)
            .child(SharedString::from(label.to_string()))
    }
}

impl Focusable for NativeMenuSection {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NativeMenuSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let focus_handle = self.focus_handle.clone();

        v_flex()
            .track_focus(&self.focus_handle)
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_click))
            .on_action(cx.listener(Self::open_github))
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("native-menu-builder-box", "Builder API")
                    .description("Supports disabled items, checked states, and submenus.")
                    .w(rems(32.5))
                    .child(self.trigger("Right-click here", cx).on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            // Focus the section so the dispatched action reaches `on_click`.
                            this.focus_handle.focus(window, cx);
                            // Nudge right so the cursor doesn't land on the first item.
                            let position = Point {
                                x: ev.position.x + px(4.),
                                y: ev.position.y,
                            };
                            demo_menu(this.word_wrap).show(position, window, cx);
                        }),
                    )),
            )
            .child(
                section("native-menu-items-box", "Menu Items")
                    .description("Existing GPUI menu definitions can be reused directly.")
                    .w(rems(32.5))
                    .child(self.trigger("Right-click here", cx).on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            this.focus_handle.focus(window, cx);
                            let position = Point {
                                x: ev.position.x + px(4.),
                                y: ev.position.y,
                            };
                            // Reuse a GPUI menu definition (incl. a submenu) directly.
                            NativeMenu::from(gpui_kit::Menu::new("Edit").items([
                                gpui_kit::MenuItem::action("Copy", MenuClick("Copy".into())),
                                gpui_kit::MenuItem::action("Paste", MenuClick("Paste".into())),
                                gpui_kit::MenuItem::separator(),
                                gpui_kit::MenuItem::submenu(gpui_kit::Menu::new("Share").items([
                                    gpui_kit::MenuItem::action("Email", MenuClick("Email".into())),
                                    gpui_kit::MenuItem::action(
                                        "Message",
                                        MenuClick("Message".into()),
                                    ),
                                ])),
                            ]))
                            .show(position, window, cx);
                        }),
                    )),
            )
            .child(
                section("native-menu-dropdown-box", "Dropdown")
                    .description("A native menu can open from any anchored control.")
                    .w(rems(32.5))
                    .child({
                        // `show` takes any position, so a native menu can open from a
                        // control too; the captured bounds open it at its bottom-left.
                        let trigger_bounds: Rc<Cell<Bounds<Pixels>>> =
                            Rc::new(Cell::new(Bounds::default()));
                        let bounds_writer = trigger_bounds.clone();

                        div()
                            .on_prepaint(move |bounds, _, _| bounds_writer.set(bounds))
                            .child(
                                Button::new("native-menu-open")
                                    .outline()
                                    .label("Open Menu")
                                    .on_click(move |_: &ClickEvent, window, cx| {
                                        let bounds = trigger_bounds.get();
                                        let position = Point {
                                            x: bounds.origin.x,
                                            // Just below the button, with a small gap.
                                            y: bounds.origin.y + bounds.size.height + px(8.),
                                        };
                                        focus_handle.focus(window, cx);
                                        demo_menu(view.read(cx).word_wrap)
                                            .show(position, window, cx);
                                    }),
                            )
                    }),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "native-menu",
        "Native Menu",
        "A menu the operating system renders on macOS and Windows; other platforms \
        show a drawn fallback with the same API. An OS-rendered menu can extend \
        beyond the window bounds, which is useful for small windows.",
        NativeMenuSection::view(window, cx),
    ));
}
