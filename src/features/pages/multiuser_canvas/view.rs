use gpui_kit::component::button::{Button, ButtonVariants as _, Toggle};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, badge::Badge, h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::{Canvas, CanvasState, CanvasTransform};
use collab_protocol::Peer;

use super::session::{CollabSession, Mode, PeerCursor, Status, wire_color_to_hsla};
use super::tools::{PenTool, SelectTool};
use super::{ClearBoard, DeleteSelection, MultiuserCanvasPage};

pub(crate) const IS_LIVE_AVAILABLE: bool = !cfg!(target_family = "wasm");

pub(crate) fn render_header(status: Status, _cx: &App) -> impl IntoElement {
    let (icon, label_key) = match status {
        Status::Demo => (IconName::Bot, "multiuser_status_demo"),
        Status::Offline => (IconName::Ban, "multiuser_status_offline"),
        Status::Connecting => (IconName::Ban, "multiuser_status_connecting"),
        Status::Connected => (IconName::Network, "multiuser_status_connected"),
        Status::Reconnecting => (IconName::RefreshCw, "multiuser_status_reconnecting"),
    };
    let label = crate::i18n::localize(label_key);
    let status_label = label.clone();
    h_flex()
        .w_full()
        .justify_between()
        .items_center()
        .flex_wrap()
        .gap_2()
        .child(
            div()
                .id("mc-title")
                .a11y(Role::Heading, crate::i18n::localize("multiuser_title"))
                .aria_level(1)
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child(crate::i18n::localize("multiuser_title")),
        )
        .child(
            div()
                .id("mc-status")
                .a11y(Role::Status, status_label)
                .a11y_live(accesskit::Live::Polite)
                .child(Badge::new().child(Icon::new(icon)).child(label)),
        )
}

pub(crate) fn render_toolbar(
    session: &Entity<CollabSession>,
    canvas: &Entity<CanvasState>,
    name_input: &Entity<InputState>,
    url_input: &Entity<InputState>,
    status: Status,
    mode: Mode,
    tool_id: &str,
    cx: &App,
) -> impl IntoElement {
    let session_for_demo = session.clone();
    let session_for_live = session.clone();
    let session_for_connect = session.clone();
    let session_for_disconnect = session.clone();
    let session_for_pen = session.clone();
    let session_for_select = session.clone();
    let canvas_for_pen = canvas.clone();
    let canvas_for_select = canvas.clone();
    let canvas_for_reset = canvas.clone();
    let url_for_connect = url_input.clone();
    let connect_label = if status == Status::Connected {
        crate::i18n::localize("multiuser_disconnect")
    } else {
        crate::i18n::localize("multiuser_connect")
    };

    let mut toolbar = h_flex()
        .id("mc-toolbar")
        .w_full()
        .flex_wrap()
        .items_center()
        .gap_2();

    if IS_LIVE_AVAILABLE {
        toolbar = toolbar.child(
            Toggle::new("mc-mode-demo")
                .label(crate::i18n::localize("multiuser_mode_demo"))
                .checked(mode == Mode::Demo)
                .on_click(move |checked, _, cx| {
                    if *checked {
                        session_for_demo.update(cx, |session, cx| session.set_mode(Mode::Demo, cx));
                    }
                }),
        );
    }

    toolbar = toolbar.child(
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(crate::i18n::localize("multiuser_name_label")),
            )
            .child(
                Input::new(name_input).aria_label(crate::i18n::localize("multiuser_name_label")),
            ),
    );

    if IS_LIVE_AVAILABLE {
        toolbar = toolbar
            .child(
                Toggle::new("mc-mode-live")
                    .label(crate::i18n::localize("multiuser_mode_live"))
                    .checked(mode == Mode::Live)
                    .on_click(move |checked, _, cx| {
                        if *checked {
                            session_for_live
                                .update(cx, |session, cx| session.set_mode(Mode::Live, cx));
                        }
                    }),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize("multiuser_server_label")),
                    )
                    .child(
                        Input::new(url_input)
                            .aria_label(crate::i18n::localize("multiuser_server_label"))
                            .w_64(),
                    ),
            )
            .child(
                Button::new("mc-connect")
                    .outline()
                    .label(connect_label)
                    .disabled(status == Status::Connecting)
                    .on_click(move |_, _, cx| {
                        if status == Status::Connected {
                            session_for_disconnect.update(cx, |session, cx| session.disconnect(cx));
                        } else {
                            let url = url_for_connect.read(cx).value().to_string();
                            session_for_connect.update(cx, |session, cx| session.connect(url, cx));
                        }
                    }),
            );
    } else {
        toolbar = toolbar.child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(crate::i18n::localize("multiuser_wasm_note")),
        );
    }

    toolbar
        .child(
            Button::new("mc-tool-pen")
                .ghost()
                .label(crate::i18n::localize("multiuser_tool_pen"))
                .toggled(tool_id == "pen")
                .on_click(move |_, _, cx| {
                    canvas_for_pen.update(cx, |state, cx| {
                        let pen = PenTool::new(session_for_pen.clone(), cx);
                        state.set_tool(Box::new(pen), cx)
                    });
                }),
        )
        .child(
            Button::new("mc-tool-select")
                .ghost()
                .label(crate::i18n::localize("multiuser_tool_select"))
                .toggled(tool_id == "select")
                .on_click(move |_, _, cx| {
                    canvas_for_select.update(cx, |state, cx| {
                        let select = SelectTool::new(session_for_select.clone());
                        state.set_tool(Box::new(select), cx)
                    });
                }),
        )
        .child(
            Button::new("mc-delete")
                .ghost()
                .label(crate::i18n::localize("multiuser_delete"))
                .on_click(|_, window, cx| window.dispatch_action(Box::new(DeleteSelection), cx)),
        )
        .child(
            Button::new("mc-clear")
                .ghost()
                .label(crate::i18n::localize("multiuser_clear"))
                .on_click(|_, window, cx| window.dispatch_action(Box::new(ClearBoard), cx)),
        )
        .child(
            Button::new("mc-reset-view")
                .ghost()
                .label(crate::i18n::localize("multiuser_reset_view"))
                .on_click(move |_, _, cx| {
                    canvas_for_reset.update(cx, |state, cx| state.reset_view(cx))
                }),
        )
}

pub(crate) fn render_peer_panel(peers: &[PeerCursor], own: &Peer, cx: &App) -> impl IntoElement {
    let own_color = wire_color_to_hsla(own.color);
    let panel = v_flex()
        .w_56()
        .flex_shrink_0()
        .border_l_1()
        .border_color(cx.theme().border)
        .p_3()
        .gap_2()
        .child(
            h_flex()
                .id("mc-own-row")
                .gap_2()
                .items_center()
                .child(div().size_2().rounded_full().bg(own_color))
                .child(div().min_w_0().truncate().child(own.name.clone()))
                .child(
                    div()
                        .text_xs()
                        .flex_shrink_0()
                        .text_color(cx.theme().muted_foreground)
                        .child(crate::i18n::localize("multiuser_you")),
                ),
        );

    if peers.is_empty() {
        return panel.child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(crate::i18n::localize("multiuser_peer_list_empty")),
        );
    }

    let total = peers.len();
    let list_label = crate::i18n::localize("multiuser_peer_list_label");
    panel.child(
        v_flex()
            .id("mc-peer-list")
            .a11y(Role::List, list_label.clone())
            .aria_orientation(Orientation::Vertical)
            .aria_size_of_set(total)
            .gap_2()
            .children(peers.iter().enumerate().map(|(ix, entry)| {
                let color = wire_color_to_hsla(entry.peer.color);
                let label = entry.peer.name.clone();
                v_flex()
                    .id(ElementId::Name(SharedString::from(format!(
                        "mc-peer-{}",
                        entry.peer.id
                    ))))
                    .a11y(Role::ListItem, label.clone())
                    .aria_position_in_set(ix)
                    .aria_size_of_set(total)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().size_2().rounded_full().bg(color))
                            .child(div().min_w_0().truncate().child(label)),
                    )
            })),
    )
}

pub(crate) fn render_cursor_overlay(
    peers: &[PeerCursor],
    transform: CanvasTransform,
    cx: &App,
) -> impl IntoElement {
    div()
        .id("mc-cursors")
        .absolute()
        .size_full()
        .overflow_hidden()
        .a11y(
            Role::Label,
            crate::i18n::localize("multiuser_cursors_label"),
        )
        .children(peers.iter().filter_map(|entry| {
            let doc = entry.doc?;
            let screen = transform.to_screen(doc);
            let color = wire_color_to_hsla(entry.peer.color);
            Some(
                h_flex()
                    .absolute()
                    .left(px(screen.x.as_f32()))
                    .top(px(screen.y.as_f32()))
                    .gap_1()
                    .items_start()
                    .child(div().size_2().rounded_full().bg(color))
                    .child(
                        div()
                            .text_xs()
                            .px_1()
                            .rounded_sm()
                            .bg(color.opacity(0.9))
                            .text_color(cx.theme().foreground)
                            .child(entry.peer.name.clone()),
                    ),
            )
        }))
}

pub(crate) fn render_canvas_area(
    page: &MultiuserCanvasPage,
    peers: &[PeerCursor],
    transform: CanvasTransform,
    cx: &App,
) -> impl IntoElement {
    div()
        .relative()
        .flex_1()
        .min_w_0()
        .child(Canvas::new("mc-canvas", page.canvas.clone()))
        .child(render_cursor_overlay(peers, transform, cx))
}

#[cfg(test)]
#[path = "view.test.rs"]
mod view_test;
