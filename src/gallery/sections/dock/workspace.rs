//! Dock Workspace section, ported from the upstream `dock` example: edge
//! docks around a tabbed center, runtime panel commands, keyboard zoom and
//! close, and a layout snapshot kept in memory.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dock::{
        ClosePanel, DockArea, DockAreaState, DockEvent, DockLayout, DockPlacement, DockSkin,
        ToggleZoom, panel_handle, register_panel,
    },
    h_flex,
    menu::DropdownMenu as _,
    status_bar::StatusBar,
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoPanel, PanelInfo, PanelSpec};
use crate::gallery::demo::section;

/// Payload of the add-panel menu, carried by the placement it lands at.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_dock, no_json)]
pub(crate) struct AddPanel(DockPlacement);

/// Payload of the panel-visibility checks, carrying the panel name.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_dock, no_json)]
pub(crate) struct TogglePanelVisible(SharedString);

actions!(gallery_dock, [ToggleDockToggleButton]);

/// The dock subtree that owns focus while the keybindings are live; the kit's
/// own dock actions handle them from there.
const CONTEXT: &str = "gallery-dock-workspace";

/// Layout edits batch into one snapshot this long after the last change.
const SAVE_DEBOUNCE: Duration = Duration::from_secs(1);

const WORKSPACE_DOCK: DockAreaTab = DockAreaTab {
    id: "dock-workspace",
    version: 5,
};

struct DockAreaTab {
    id: &'static str,
    version: usize,
}

const EDITOR: PanelSpec = PanelSpec {
    name: "GalleryDockEditor",
    title: "Editor",
    body: "Drop a tab near an edge of this group to split it there.",
};
const PREVIEW: PanelSpec = PanelSpec {
    name: "GalleryDockPreview",
    title: "Preview",
    body: "Panels in one group share the tab strip; click a tab to switch.",
};
const EXPLORER: PanelSpec = PanelSpec {
    name: "GalleryDockExplorer",
    title: "Explorer",
    body: "The left dock collapses from its tab bar or the status bar.",
};
const SEARCH: PanelSpec = PanelSpec {
    name: "GalleryDockSearch",
    title: "Search",
    body: "Two tab groups stacked by a v_split share the left dock.",
};
const TERMINAL: PanelSpec = PanelSpec {
    name: "GalleryDockTerminal",
    title: "Terminal",
    body: "The bottom dock spans the width below the center region.",
};
const PROBLEMS: PanelSpec = PanelSpec {
    name: "GalleryDockProblems",
    title: "Problems",
    body: "No problems detected.",
};
const OUTLINE: PanelSpec = PanelSpec {
    name: "GalleryDockOutline",
    title: "Outline",
    body: "The right dock mirrors the left dock on the other side.",
};
const INSPECTOR: PanelSpec = PanelSpec {
    name: "GalleryDockInspector",
    title: "Inspector",
    body: "A closed dock keeps its panels; reopening restores them.",
};

const PANELS: &[PanelSpec] = &[
    EDITOR, PREVIEW, EXPLORER, SEARCH, TERMINAL, PROBLEMS, OUTLINE, INSPECTOR,
];

/// The four panels the visibility checks toggle; hiding every panel of a dock
/// gives up its slot until one is shown again.
const TOGGLE_PANELS: [&PanelSpec; 4] = [&EXPLORER, &SEARCH, &OUTLINE, &INSPECTOR];

/// Panels hidden through the workspace menu. A global because a registry-built
/// panel has no other route back to the section.
struct HiddenPanels(Entity<Vec<SharedString>>);

impl Global for HiddenPanels {}

pub fn init(cx: &mut App) {
    let hidden = cx.new(|_| Vec::new());
    cx.set_global(HiddenPanels(hidden));
    for spec in PANELS {
        register_panel(cx, spec.name, |_, _, cx| {
            let hidden = cx.global::<HiddenPanels>().0.clone();
            panel_handle(DemoPanel::with_toolbar(spec, hidden, cx))
        });
    }
    cx.bind_keys([
        KeyBinding::new("shift-escape", ToggleZoom, Some(CONTEXT)),
        KeyBinding::new("ctrl-w", ClosePanel, Some(CONTEXT)),
    ]);
}

pub struct WorkspaceSection {
    dock_area: Entity<DockArea>,
    skin: Rc<DockSkin>,
    hidden: Entity<Vec<SharedString>>,
    last_layout_state: Option<DockAreaState>,
    saved_json: Option<String>,
    next_panel_ix: usize,
    _save_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl WorkspaceSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let (dock_area, skin) =
            DockSkin::dock_area(WORKSPACE_DOCK.id, Some(WORKSPACE_DOCK.version), window, cx);
        let hidden = cx.global::<HiddenPanels>().0.clone();
        Self::reset_default_layout(&dock_area, window, cx);

        cx.new(|cx| {
            let subscription = cx.subscribe(
                &dock_area,
                |this: &mut Self, _, event: &DockEvent, cx: &mut Context<Self>| {
                    if matches!(event, DockEvent::LayoutChanged) {
                        this.save_layout_later(cx);
                    }
                },
            );
            Self {
                dock_area,
                skin,
                hidden,
                last_layout_state: None,
                saved_json: None,
                next_panel_ix: 0,
                _save_task: None,
                _subscriptions: vec![subscription],
            }
        })
    }

    fn reset_default_layout(dock_area: &Entity<DockArea>, window: &mut Window, cx: &mut App) {
        let hidden = cx.global::<HiddenPanels>().0.clone();
        let center = DockLayout::v_split().child(
            DockLayout::tabs()
                .panel_view(
                    panel_handle(DemoPanel::with_toolbar(&EDITOR, hidden.clone(), cx)),
                    cx,
                )
                .panel_view(
                    panel_handle(DemoPanel::with_toolbar(&PREVIEW, hidden.clone(), cx)),
                    cx,
                ),
            None,
        );

        let left_panels = DockLayout::v_split()
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(DemoPanel::with_toolbar(&EXPLORER, hidden.clone(), cx)),
                    cx,
                ),
                None,
            )
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(DemoPanel::with_toolbar(&SEARCH, hidden.clone(), cx)),
                    cx,
                ),
                Some(px(160.)),
            );

        let bottom_panels = DockLayout::v_split().child(
            DockLayout::tabs()
                .panel_view(
                    panel_handle(DemoPanel::with_toolbar(&TERMINAL, hidden.clone(), cx)),
                    cx,
                )
                .panel_view(
                    panel_handle(DemoPanel::with_toolbar(&PROBLEMS, hidden.clone(), cx)),
                    cx,
                ),
            None,
        );

        let right_panels = DockLayout::v_split()
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(DemoPanel::with_toolbar(&OUTLINE, hidden.clone(), cx)),
                    cx,
                ),
                None,
            )
            .child(
                DockLayout::tabs().panel_view(
                    panel_handle(DemoPanel::with_toolbar(&INSPECTOR, hidden.clone(), cx)),
                    cx,
                ),
                None,
            );

        dock_area.update(cx, |area, cx| {
            area.set_center(center, window, cx);
            for (placement, layout, size) in [
                (DockPlacement::Left, left_panels, px(200.)),
                (DockPlacement::Bottom, bottom_panels, px(160.)),
                (DockPlacement::Right, right_panels, px(200.)),
            ] {
                area.set_dock(placement, layout, window, cx);
                area.set_dock_size(placement, size, window, cx);
            }
        });
    }

    fn save_layout_later(&mut self, cx: &mut Context<Self>) {
        let dock_area = self.dock_area.clone();
        // Replacing the pending task cancels its timer, so only the last edit
        // of a burst reaches the snapshot.
        self._save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DEBOUNCE).await;
            let _ = this.update(cx, |this, cx| this.save_layout(&dock_area, cx));
        }));
    }

    fn save_layout(&mut self, dock_area: &Entity<DockArea>, cx: &mut Context<Self>) {
        let state = dock_area.read(cx).dump(cx);
        if Some(&state) == self.last_layout_state.as_ref() {
            return;
        }
        // DockAreaState has no non-string map keys, so serialization cannot
        // fail; the binding keeps the flow total.
        let Ok(json) = serde_json::to_string_pretty(&state) else {
            return;
        };
        self.saved_json = Some(json);
        self.last_layout_state = Some(state);
        cx.notify();
    }

    fn on_click_load(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(json) = self.saved_json.as_ref() else {
            return;
        };
        let restored = serde_json::from_str::<DockAreaState>(json)
            .map_err(|err| err.to_string())
            .and_then(|state| {
                self.dock_area.update(cx, |area, cx| {
                    area.load(state, window, cx).map_err(|err| err.to_string())
                })
            });
        if let Err(message) = restored {
            window.push_notification(format!("Couldn't restore the layout: {message}"), cx);
        }
        cx.notify();
    }

    fn on_click_reset(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        Self::reset_default_layout(&self.dock_area, window, cx);
        cx.notify();
    }

    fn on_action_add_panel(
        &mut self,
        action: &AddPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A rotation over the catalog keeps the demo deterministic.
        let spec = &PANELS[self.next_panel_ix % PANELS.len()];
        self.next_panel_ix += 1;
        let panel = panel_handle(DemoPanel::with_toolbar(spec, self.hidden.clone(), cx));
        self.dock_area.update(cx, |area, cx| {
            area.add_panel_view(panel, action.0, None, window, cx);
        });
    }

    fn on_action_toggle_panel_visible(
        &mut self,
        action: &TogglePanelVisible,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = action.0.clone();
        self.hidden.update(cx, |names, cx| {
            if names.contains(&name) {
                names.retain(|other| other != &name);
            } else {
                names.push(name);
            }
            cx.notify();
        });
        cx.notify();
    }

    fn on_action_toggle_dock_toggle_button(
        &mut self,
        _: &ToggleDockToggleButton,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The skin redraws the area; the section renders nothing from this
        // state.
        let visible = !self.skin.is_toggle_button_visible();
        self.skin.set_toggle_button_visible(visible, cx);
    }

    fn on_action_panel_info(&mut self, _: &PanelInfo, window: &mut Window, cx: &mut Context<Self>) {
        window.push_notification("You clicked panel info.", cx);
    }

    fn dock_toggle(
        &self,
        id: &'static str,
        placement: DockPlacement,
        icon: IconName,
        tooltip: &'static str,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .xsmall()
            .icon(icon)
            .tooltip(tooltip)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.dock_area.update(cx, |area, cx| {
                    area.toggle_dock(placement, window, cx);
                });
            }))
    }
}

impl Render for WorkspaceSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let hidden = self.hidden.clone();
        let saved_len = self.saved_json.as_ref().map(|json| json.len());

        v_flex()
            .p_4()
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_add_panel))
            .on_action(cx.listener(Self::on_action_toggle_panel_visible))
            .on_action(cx.listener(Self::on_action_toggle_dock_toggle_button))
            .on_action(cx.listener(Self::on_action_panel_info))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("dock-workspace-load")
                                    .label("Load saved layout")
                                    .small()
                                    .disabled(saved_len.is_none())
                                    .on_click(cx.listener(Self::on_click_load)),
                            )
                            .child(
                                Button::new("dock-workspace-reset")
                                    .label("Reset default layout")
                                    .small()
                                    .on_click(cx.listener(Self::on_click_reset)),
                            ),
                    )
                    .child(
                        Button::new("dock-workspace-add-panel")
                            .icon(IconName::LayoutDashboard)
                            .tooltip("Add panel")
                            .small()
                            .ghost()
                            .dropdown_menu(move |menu, _, cx| {
                                let hidden = hidden.read(cx).clone();
                                let mut menu = menu
                                    .menu(
                                        "Add Panel to Center",
                                        Box::new(AddPanel(DockPlacement::Center)),
                                    )
                                    .separator()
                                    .menu(
                                        "Add Panel to Left",
                                        Box::new(AddPanel(DockPlacement::Left)),
                                    )
                                    .menu(
                                        "Add Panel to Right",
                                        Box::new(AddPanel(DockPlacement::Right)),
                                    )
                                    .menu(
                                        "Add Panel to Bottom",
                                        Box::new(AddPanel(DockPlacement::Bottom)),
                                    )
                                    .separator()
                                    .menu(
                                        "Show / Hide Dock Toggle Button",
                                        Box::new(ToggleDockToggleButton),
                                    )
                                    .separator();
                                for spec in TOGGLE_PANELS {
                                    let visible =
                                        !hidden.iter().any(|name| name.as_ref() == spec.name);
                                    menu = menu.menu_with_check(
                                        spec.title,
                                        visible,
                                        Box::new(TogglePanelVisible(SharedString::from(spec.name))),
                                    );
                                }
                                menu
                            }),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(match saved_len {
                        Some(len) => format!("Saved layout in memory: {len} bytes."),
                        None => "The layout snapshot saves one second after you stop editing."
                            .to_string(),
                    }),
            )
            .child(
                section("dock-workspace-box", "Workspace")
                    .description(
                        "Edge docks around a tabbed center; the status bar toggles each dock.",
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                div()
                                    .w_full()
                                    .h(rems(37.5))
                                    // The gallery pane is an unbounded scrolling
                                    // column; a dock needs a definite height to split.
                                    .child(self.dock_area.clone()),
                            )
                            .child(
                                StatusBar::new()
                                    .left(self.dock_toggle(
                                        "dock-workspace-toggle-left",
                                        DockPlacement::Left,
                                        IconName::PanelLeft,
                                        "Toggle Left Dock",
                                        cx,
                                    ))
                                    .left(self.dock_toggle(
                                        "dock-workspace-toggle-bottom",
                                        DockPlacement::Bottom,
                                        IconName::PanelBottom,
                                        "Toggle Bottom Dock",
                                        cx,
                                    ))
                                    .child(self.dock_toggle(
                                        "dock-workspace-toggle-right",
                                        DockPlacement::Right,
                                        IconName::PanelRight,
                                        "Toggle Right Dock",
                                        cx,
                                    )),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "dock-workspace",
        "Dock Workspace",
        "A dockable workspace with edge docks, panel commands, keyboard shortcuts, and a restorable layout.",
        WorkspaceSection::view(window, cx),
    ));
}
