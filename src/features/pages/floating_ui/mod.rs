//! Removable floating-UI page: three tool palettes as always-on-top windows
//! (plan 02) or in-window snap-to-edge panels, persisted in `AppConfig`.
//! Removal: delete this directory and revert the seam touches: the
//! `mod floating_ui;`/`pub use` lines in `src/features/pages/mod.rs`, the
//! `Page::FloatingUi` arms + `all()` entry in `src/shell/sidebar.rs`, the
//! `VALID_HOSTS` entry in `src/shell/route.rs`, the `floating_ui_session`/
//! `floating_ui_host` fields + constructor lines + the RootWindow restore
//! observer + the `PaletteChanged` drain arm in
//! `src/shell/root/app_root/state.rs`, the host child in
//! `src/shell/root/app_root/render.rs`, the `PaletteChanged` variant in
//! `src/runtime/events.rs`, the `floating_palettes` field in
//! `src/state/config_store.rs` + the snapshot fixtures in
//! `tests/snapshot_tests.rs`, the `floating_ui::init(cx)` line in
//! `src/app/init.rs`, every `floating_ui*`/`nav_floating_ui` locale key, and
//! the page count in `tests/e2e_navigation.rs`.

mod geometry;
mod panel_host;
pub(crate) mod persistence;
mod session;
mod state;
mod views;

pub use panel_host::PalettePanelHost;
pub use session::{PaletteSession, PaletteSurface};
pub use state::{PaletteId, PaletteSyncState, apply_change};

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _,
    badge::Badge,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use state::{COLOR_PRESETS, rgba_to_hex};

pub const CAPABILITY_FLOATING_UI: &str = "floating_ui_palettes";

/// Feature boot hook: seeds `PaletteSyncState` (first color preset) and
/// writes the `floating_ui_palettes` capability entry. Call once from
/// `app::init`, after `crate::ui::floating::init`. Boot restore of open
/// windows is AppRoot's RootWindow observer, not this function: an App-level
/// observer here has no entity handle to call.
pub fn init(cx: &mut App) {
    if cx.try_global::<PaletteSyncState>().is_none() {
        cx.set_global(PaletteSyncState {
            color_hex: rgba_to_hex(rgba(COLOR_PRESETS[0].1)),
            last_asset: None,
        });
    }
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        CAPABILITY_FLOATING_UI,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: true,
            degraded: true,
            reason: Some(
                "native palette windows unavailable on wasm; in-window panels are the fallback"
                    .into(),
            ),
            last_error: None,
        },
        cx,
    );
    #[cfg(not(target_family = "wasm"))]
    crate::capabilities::set(
        CAPABILITY_FLOATING_UI,
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
}

/// Page view; constructed by AppRoot with the session handle.
pub struct FloatingUiPage {
    session: Entity<PaletteSession>,
    _subscriptions: Vec<Subscription>,
}

impl FloatingUiPage {
    pub fn new(session: Entity<PaletteSession>, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe_global::<PaletteSyncState>(|_, cx| {
            cx.notify();
        })];
        Self {
            session,
            _subscriptions: subscriptions,
        }
    }

    fn render_sync_card(&self, cx: &mut Context<Self>) -> Div {
        let sync = cx
            .try_global::<PaletteSyncState>()
            .cloned()
            .unwrap_or_default();
        let color = state::hex_to_rgba(&sync.color_hex).unwrap_or_else(|| rgba(0x000000FF));
        let color_line = format!(
            "{} {}",
            crate::i18n::localize("floating_ui_sync_color"),
            sync.color_hex
        );
        h_flex()
            .gap_3()
            .p_3()
            .border_1()
            .rounded(cx.theme().radius)
            .child(div().size_8().rounded(cx.theme().radius).bg(color))
            .child(
                v_flex()
                    .gap_1()
                    .min_w_0()
                    .child(
                        div()
                            .id("floating-ui-sync-color")
                            .a11y(Role::Status, color_line.clone())
                            .a11y_live(accesskit::Live::Polite)
                            .text_sm()
                            .child(color_line),
                    )
                    .when_some(sync.last_asset.clone(), |this, asset| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "{} {asset}",
                                    crate::i18n::localize("floating_ui_sync_asset")
                                )),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize("floating_ui_persist_hint")),
                    ),
            )
    }

    fn render_palette_card(&self, id: PaletteId, native_windows: bool, cx: &App) -> AnyElement {
        let slug = id.slug();
        let session = self.session.read(cx);
        let window_open =
            session.is_open(id) && session.surface(id) == Some(PaletteSurface::Window);
        let panel_open = session.is_open(id) && session.surface(id) == Some(PaletteSurface::Panel);
        let badge = if window_open {
            Some(crate::i18n::localize("floating_ui_surface_window"))
        } else if panel_open {
            Some(crate::i18n::localize("floating_ui_surface_panel"))
        } else {
            None
        };
        let icon = match id {
            PaletteId::Color => IconName::Palette,
            PaletteId::Assets => IconName::Inbox,
            PaletteId::Note => IconName::FileText,
        };

        let session_handle = self.session.clone();
        let window_toggle = Button::new(format!("floating-ui-window-{slug}"))
            .outline()
            .label(crate::i18n::localize(if window_open {
                "floating_ui_close_window"
            } else {
                "floating_ui_open_window"
            }))
            .disabled(!native_windows)
            .on_click(move |_, _, cx| {
                session_handle.update(cx, |session, cx| {
                    if let Err(err) = session.toggle_window(id, cx) {
                        tracing::warn!(
                            target: "gpui_starter::floating_ui",
                            error = %err,
                            "palette window toggle failed"
                        );
                    }
                });
            });
        let panel_toggle = Button::new(format!("floating-ui-panel-{slug}"))
            .outline()
            .label(crate::i18n::localize(if panel_open {
                "floating_ui_hide_panel"
            } else {
                "floating_ui_show_panel"
            }))
            .on_click({
                let session = self.session.clone();
                move |_, _, cx| {
                    session.update(cx, |session, cx| {
                        session.set_panel_open(id, !panel_open, cx)
                    });
                }
            });
        let reset = Button::new(format!("floating-ui-reset-{slug}"))
            .ghost()
            .label(crate::i18n::localize("floating_ui_reset_position"))
            .on_click({
                let session = self.session.clone();
                move |_, _, cx| {
                    session.update(cx, |session, cx| session.reset_window_position(id, cx));
                }
            });

        v_flex()
            .gap_2()
            .p_3()
            .border_1()
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(icon).small())
                    .child(div().min_w_0().child(id.localized_title()))
                    .child(div().flex_1())
                    .when_some(badge, |this, badge| this.child(Badge::new().child(badge))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(window_toggle)
                    .child(panel_toggle)
                    .when(window_open, |this| this.child(reset)),
            )
            .into_any_element()
    }
}

impl Render for FloatingUiPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let native_windows = crate::ui::floating::probe(cx).native_windows;
        let title = crate::i18n::localize("floating_ui_title");
        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(
                div()
                    .id("floating-ui-title")
                    .a11y(Role::Heading, title.clone())
                    .a11y_live(accesskit::Live::Polite)
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(crate::i18n::localize("floating_ui_subtitle")),
            )
            .child(self.render_sync_card(cx))
            .children(
                PaletteId::ALL
                    .iter()
                    .copied()
                    .map(|id| self.render_palette_card(id, native_windows, cx)),
            )
            .when(!native_windows, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(crate::i18n::localize("floating_ui_window_unavailable")),
                )
            })
    }
}
