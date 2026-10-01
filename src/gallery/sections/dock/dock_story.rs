//! Dock section, ported from the upstream `DockStory`: a split center with a
//! collapsible bottom dock, drag-and-drop between tab groups, and the skin's
//! close-button setting.

use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _,
    button::{Button, DropdownButton},
    dock::{
        BasePanel, DockArea, DockLayout, DockPlacement, DockSkin, Panel, PanelEvent, panel_handle,
    },
    h_flex, v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// Knob of the `DockStory` options menu.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_dock, no_json)]
pub(crate) enum DockOption {
    TabCloseButtons,
}

struct DemoPanel {
    name: &'static str,
    title: SharedString,
    body: SharedString,
    focus_handle: FocusHandle,
    closable: bool,
}

impl DemoPanel {
    fn new(
        name: &'static str,
        title: &'static str,
        body: &'static str,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            name,
            title: title.into(),
            body: body.into(),
            focus_handle: cx.focus_handle(),
            closable: true,
        })
    }
}

impl EventEmitter<PanelEvent> for DemoPanel {}

impl Focusable for DemoPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for DemoPanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }

    fn closable(&self, _: &App) -> bool {
        self.closable
    }
}

impl Panel for DemoPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.title.clone()
    }
}

impl Render for DemoPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .text_color(cx.theme().foreground)
            .child(self.body.clone())
    }
}

pub struct DockSection {
    dock_area: Entity<DockArea>,
    skin: Rc<DockSkin>,
    close_button_visible: bool,
}

impl DockSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let (dock_area, skin) = DockSkin::dock_area("dock-story", Some(1), window, cx);
        let explorer = DemoPanel::new(
            "DockStoryExplorer",
            "Explorer",
            "Drag this tab into another group.",
            cx,
        );
        let search = DemoPanel::new(
            "DockStorySearch",
            "Search",
            "Two panels can share one tab group.",
            cx,
        );
        let editor = DemoPanel::new(
            "DockStoryEditor",
            "Editor",
            "Drop a tab near an edge to split this group.",
            cx,
        );
        editor.update(cx, |editor, _| editor.closable = false);
        let terminal = DemoPanel::new(
            "DockStoryTerminal",
            "Terminal",
            "The bottom dock shares the workspace column.",
            cx,
        );
        let problems = DemoPanel::new("DockStoryProblems", "Problems", "No problems detected.", cx);

        dock_area.update(cx, |area, cx| {
            area.set_center(
                DockLayout::h_split()
                    .child(
                        DockLayout::tabs()
                            .panel_view(panel_handle(explorer), cx)
                            .panel_view(panel_handle(search), cx),
                        Some(px(240.)),
                    )
                    .child(
                        DockLayout::tabs().panel_view(panel_handle(editor), cx),
                        None,
                    ),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Bottom,
                DockLayout::tabs()
                    .panel_view(panel_handle(terminal), cx)
                    .panel_view(panel_handle(problems), cx),
                window,
                cx,
            );
            area.set_dock_size(DockPlacement::Bottom, px(160.), window, cx);
            area.set_dock_collapsible(DockPlacement::Bottom, true, window, cx);
        });
        skin.set_toggle_button_visible(true, cx);

        cx.new(|_| Self {
            dock_area,
            skin,
            close_button_visible: false,
        })
    }

    fn on_action_option(&mut self, _: &DockOption, _: &mut Window, cx: &mut Context<Self>) {
        self.close_button_visible = !self.close_button_visible;
        self.skin
            .set_close_button_visible(self.close_button_visible, cx);
        cx.notify();
    }
}

impl Render for DockSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close_button_visible = self.close_button_visible;
        v_flex()
            .p_4()
            .on_action(cx.listener(Self::on_action_option))
            .child(
                h_flex().w_full().justify_end().gap_1().child(
                    DropdownButton::new("dock-options")
                        .button(Button::new("dock-options-trigger").label("Options"))
                        .dropdown_menu(move |menu, _, _| {
                            menu.menu_with_check(
                                "Tab close buttons",
                                close_button_visible,
                                Box::new(DockOption::TabCloseButtons),
                            )
                        }),
                ),
            )
            .child(
                section("dock-box", "Dock area").child(
                    div()
                        .w_full()
                        .h(rems(37.5))
                        // The gallery pane is an unbounded scrolling column; a
                        // dock needs a definite height to split.
                        .child(self.dock_area.clone()),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "dock",
        "Dock",
        "Drag tabs between groups or towards an edge to split the workspace.",
        DockSection::view(window, cx),
    ));
}
