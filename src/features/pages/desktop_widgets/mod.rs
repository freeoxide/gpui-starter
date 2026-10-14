//! Removable desktop-widgets manager. `DesktopWidgetsPage`, `WidgetKind`,
//! and the spawn setters are the only public surfaces. Removal = delete this
//! directory and revert its seam touches: the `pub mod desktop_widgets;` and
//! `pub use` lines in features/pages/mod.rs, the Page::DesktopWidgets arms
//! and all() entry in shell/sidebar.rs, the VALID_HOSTS entry in
//! shell/route.rs, the AppRoot wiring in shell/root/app_root/state.rs, the
//! init call in app/init.rs, the widgets_layout field in
//! state/config_store.rs plus the snapshot fixtures, the
//! OpenDesktopWidgets command arms in services/commands.rs, the page count
//! in tests/e2e_navigation.rs, and the widgets_*/nav_desktop_widgets/
//! command_desktop_widgets* keys in both locale files.

use std::collections::BTreeMap;

use gpui_kit::component::button::Toggle;
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::features::tutorial;
use crate::ui::floating::{
    CAPABILITY_ALWAYS_ON_TOP, CAPABILITY_FLOATING_CLICK_THROUGH, CAPABILITY_FLOATING_WINDOWS,
    CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH, CAPABILITY_LAYER_SHELL_WIDGETS, Placement9,
};

mod battery;
pub mod catalog;
mod clock;
mod metrics;
mod spawn;
mod system_sliders;

pub use catalog::WidgetKind;
pub use spawn::{
    set_widget_always_on_top, set_widget_click_through, set_widget_enabled, set_widget_layer,
    set_widget_placement, spawn_persisted, widget_id,
};

pub const CAPABILITY_DESKTOP_WIDGETS: &str = "desktop_widgets";

const CAPABILITY_KEYS: [&str; 6] = [
    CAPABILITY_DESKTOP_WIDGETS,
    CAPABILITY_FLOATING_WINDOWS,
    CAPABILITY_ALWAYS_ON_TOP,
    CAPABILITY_FLOATING_CLICK_THROUGH,
    CAPABILITY_LAYER_SHELL_WIDGETS,
    CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
];

pub(crate) fn row_kinds(_cx: &App) -> Vec<WidgetKind> {
    catalog::available_kinds()
}

pub(crate) fn open_map(cx: &App) -> BTreeMap<WidgetKind, bool> {
    let snapshot = crate::ui::floating::floating_snapshot(cx);
    catalog::available_kinds()
        .into_iter()
        .map(|kind| {
            let open = snapshot.iter().any(|e| e.id == spawn::widget_id(kind));
            (kind, open)
        })
        .collect()
}

pub struct DesktopWidgetsPage {
    _subscriptions: Vec<Subscription>,
}

impl DesktopWidgetsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.observe_global_in::<crate::ui::floating::FloatingWindowRegistry>(
                window,
                |_, _, cx| cx.notify(),
            ),
        ];
        #[cfg(target_family = "wasm")]
        window.defer(cx, |_, cx| spawn_persisted(cx));
        #[cfg(not(target_family = "wasm"))]
        let _ = window;
        Self { _subscriptions }
    }

    fn render_row(&self, kind: WidgetKind, open: bool, cx: &mut Context<Self>) -> Div {
        let title = crate::i18n::localize(kind.title_key());
        let header = h_flex()
            .gap_2()
            .items_center()
            .justify_between()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .when_some(kind_icon(kind), |row, icon| {
                        row.child(div().size_4().flex_shrink_0().child(Icon::new(icon)))
                    })
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title)),
            )
            .child(
                Switch::new(format!("widgets-switch-{}", kind.as_str()))
                    .checked(open)
                    .label(crate::i18n::localize("widgets_enabled"))
                    .on_change(cx.listener(move |_, on: &bool, _, cx| {
                        set_widget_enabled(kind, *on, cx);
                    })),
            );
        let mut card = v_flex()
            .gap_2()
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(header);
        if open {
            card = card.child(self.render_controls(kind, cx));
        }
        card
    }

    fn render_controls(&self, kind: WidgetKind, cx: &mut Context<Self>) -> Div {
        let entry = catalog::slot(kind, cx).unwrap_or_else(|| spawn::default_entry(kind));
        let is_wayland = cx.compositor_name() == "Wayland";
        v_flex()
            .gap_3()
            .pl_6()
            .child(self.render_placement(kind, &entry, cx))
            .child(self.render_toggles(kind, &entry, is_wayland, cx))
            .child(self.render_layer(kind, &entry, is_wayland, cx))
    }

    fn render_placement(
        &self,
        kind: WidgetKind,
        entry: &catalog::PersistedWidget,
        cx: &mut Context<Self>,
    ) -> Div {
        let muted = cx.theme().muted_foreground;
        let current = catalog::placement_from_str(&entry.placement)
            .unwrap_or(catalog::defaults(kind).placement);
        let keypad = [
            [
                (Placement9::TopLeft, "TL"),
                (Placement9::Top, "T"),
                (Placement9::TopRight, "TR"),
            ],
            [
                (Placement9::Left, "L"),
                (Placement9::Center, "C"),
                (Placement9::Right, "R"),
            ],
            [
                (Placement9::BottomLeft, "BL"),
                (Placement9::Bottom, "B"),
                (Placement9::BottomRight, "BR"),
            ],
        ];
        let mut grid = v_flex().gap_1();
        for row in keypad {
            let mut cells = h_flex().gap_1();
            for (cell, code) in row {
                cells = cells.child(
                    Toggle::new(format!("widgets-pos-{}-{}", kind.as_str(), code))
                        .small()
                        .checked(current == cell)
                        .label(code)
                        .tooltip(crate::i18n::localize(&format!(
                            "widgets_placement_{}",
                            catalog::placement_to_str(cell)
                        )))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            set_widget_placement(kind, cell, cx);
                        })),
                );
            }
            grid = grid.child(cells);
        }
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(crate::i18n::localize("widgets_placement")),
            )
            .child(grid)
    }

    fn render_toggles(
        &self,
        kind: WidgetKind,
        entry: &catalog::PersistedWidget,
        is_wayland: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let muted = cx.theme().muted_foreground;
        let mut on_top = v_flex().gap_1().child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .child(crate::i18n::localize("widgets_always_on_top")),
                )
                .child(
                    Switch::new(format!("widgets-top-{}", kind.as_str()))
                        .checked(entry.always_on_top)
                        .on_change(cx.listener(move |_, on: &bool, _, cx| {
                            set_widget_always_on_top(kind, *on, cx);
                        })),
                ),
        );
        if is_wayland {
            on_top = on_top.child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(crate::i18n::localize("widgets_toggle_respawn")),
            );
        }
        v_flex().gap_3().child(on_top).child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .child(crate::i18n::localize("widgets_click_through")),
                )
                .child(
                    Switch::new(format!("widgets-pass-{}", kind.as_str()))
                        .checked(entry.click_through)
                        .on_change(cx.listener(move |_, on: &bool, _, cx| {
                            set_widget_click_through(kind, *on, cx);
                        })),
                ),
        )
    }

    fn render_layer(
        &self,
        kind: WidgetKind,
        entry: &catalog::PersistedWidget,
        is_wayland: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let muted = cx.theme().muted_foreground;
        let choice: Div = if is_wayland {
            let selected = usize::from(entry.layer == "pinned");
            h_flex().gap_2().items_center().child(
                RadioGroup::horizontal(format!("widgets-layer-{}", kind.as_str()))
                    .selected_index(Some(selected))
                    .on_change(cx.listener(move |_, ix: &usize, _, cx| {
                        set_widget_layer(kind, if *ix == 1 { "pinned" } else { "above" }, cx);
                    }))
                    .child(
                        Radio::new(format!("widgets-layer-{}-above", kind.as_str()))
                            .label(crate::i18n::localize("widgets_layer_above")),
                    )
                    .child(
                        Radio::new(format!("widgets-layer-{}-pinned", kind.as_str()))
                            .label(crate::i18n::localize("widgets_layer_pinned")),
                    ),
            )
        } else {
            h_flex().child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(crate::i18n::localize("widgets_layer_off_wayland")),
            )
        };
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .child(crate::i18n::localize("widgets_layer")),
            )
            .child(choice)
    }

    fn render_capabilities(&self, cx: &mut Context<Self>) -> Div {
        let muted = cx.theme().muted_foreground;
        let warning = cx.theme().warning;
        let snapshot = crate::capabilities::snapshot(cx);
        let mut section = v_flex().gap_2().child(
            div()
                .text_sm()
                .text_color(muted)
                .child(crate::i18n::localize("widgets_capabilities")),
        );
        for key in CAPABILITY_KEYS {
            let Some(status) = snapshot.get(key) else {
                continue;
            };
            let (state_key, color) = if !status.supported || !status.enabled {
                ("widgets_cap_error", warning)
            } else if status.degraded {
                ("widgets_cap_degraded", warning)
            } else {
                ("widgets_cap_supported", muted)
            };
            let mut text = crate::i18n::localize(state_key);
            if let Some(reason) = &status.reason {
                text.push_str(": ");
                text.push_str(reason);
            }
            section = section.child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(key.to_string()))
                    .child(div().text_sm().text_color(color).child(text)),
            );
        }
        section
    }
}

impl Render for DesktopWidgetsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = open_map(cx);
        let open_count = open.values().filter(|open| **open).count();
        let title = crate::i18n::localize("widgets_title");
        let subtitle = crate::i18n::localize("widgets_subtitle");
        let status = crate::i18n::localize_with_args(
            "widgets_open_count",
            &[("count", &open_count.to_string())],
        );

        let mut rows = Vec::new();
        for kind in row_kinds(cx) {
            let is_open = open.get(&kind).copied().unwrap_or(false);
            rows.push(self.render_row(kind, is_open, cx));
        }

        let heading_region = v_flex()
            .gap_1()
            .child(
                div()
                    .id("widgets-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(subtitle),
            );
        let list_region = v_flex().gap_3().children(rows);
        let controls_region = self.render_capabilities(cx);

        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(tutorial::target("widgets-heading", heading_region))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(status),
            )
            .child(tutorial::target("widgets-list", list_region))
            .child(tutorial::target("widgets-controls", controls_region))
    }
}

fn kind_icon(kind: WidgetKind) -> Option<IconName> {
    match kind {
        WidgetKind::Clock => None,
        WidgetKind::Battery => Some(IconName::Battery),
        WidgetKind::System => Some(IconName::Sun),
        WidgetKind::Metrics => Some(IconName::Cpu),
    }
}

/// Registers CAPABILITY_DESKTOP_WIDGETS (native supported_enabled; wasm
/// supported:false, enabled, degraded, overlay reason) and, native only,
/// runs spawn_persisted. Called once from app::init after floating::init.
pub fn init(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    let status = crate::capabilities::CapabilityStatus {
        supported: false,
        enabled: true,
        degraded: true,
        reason: Some("widgets render as in-window overlay panes on the web build".into()),
        last_error: None,
    };
    #[cfg(not(target_family = "wasm"))]
    let status = crate::capabilities::CapabilityStatus::supported_enabled();
    crate::capabilities::set(CAPABILITY_DESKTOP_WIDGETS, status, cx);
    #[cfg(not(target_family = "wasm"))]
    spawn_persisted(cx);
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod desktop_widgets_test;
