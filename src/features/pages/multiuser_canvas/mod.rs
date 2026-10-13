//! Removable collaborative-canvas page. `MultiuserCanvasPage` is the only
//! public surface; the session, tools, net thread, and fake peers stay
//! private. Removal = delete this directory, the collab-protocol and
//! collab-server workspace members, and revert the seam touches: the
//! Page::MultiuserCanvas arms and all() entry in shell/sidebar.rs, the
//! VALID_HOSTS entry in shell/route.rs, the AppRoot wiring in
//! shell/root/app_root/state.rs, the mod/pub use lines in
//! features/pages/mod.rs, the nav/multiuser locale keys, the page count in
//! tests/e2e_navigation.rs, the workspace members + two dependency lines +
//! the justfile collab-server recipe in Cargo.toml/justfile, and the
//! regenerated Cargo.lock.

pub(crate) mod fake;
pub(crate) mod session;
pub(crate) mod tools;
pub(crate) mod view;

#[cfg(not(target_family = "wasm"))]
pub(crate) mod net;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::ui::canvas::CanvasState;

use session::{CollabSession, PeerCursor};
use tools::PenTool;

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3001";

/// Delete every selected shape on the shared board.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = multiuser, no_json)]
pub struct DeleteSelection;

/// Remove every shape from the shared board.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = multiuser, no_json)]
pub struct ClearBoard;

pub struct MultiuserCanvasPage {
    session: Entity<CollabSession>,
    canvas: Entity<CanvasState>,
    name_input: Entity<InputState>,
    url_input: Entity<InputState>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl MultiuserCanvasPage {
    /// Mounts the canvas engine and the collab session, starts the demo
    /// peers, and activates the pen. Constructed once by AppRoot.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|_| CanvasState::new());
        let session = cx.new(|cx| CollabSession::new(canvas.clone(), cx));
        canvas.update(cx, |state, cx| {
            state.set_tool(Box::new(PenTool::new(session.clone(), cx)), cx)
        });

        let default_name = session.read(cx).identity().name.clone();
        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(crate::i18n::localize("multiuser_name_label"))
        });
        name_input.update(cx, |state, cx| state.set_value(default_name, window, cx));
        let url_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(crate::i18n::localize("multiuser_server_label"))
        });
        url_input.update(cx, |state, cx| {
            state.set_value(DEFAULT_SERVER_URL, window, cx)
        });

        cx.bind_keys([KeyBinding::new(
            "delete",
            DeleteSelection,
            Some("MultiuserCanvas"),
        )]);

        let name_session = session.clone();
        let name_for_subscription = name_input.clone();
        let subscriptions = vec![
            cx.observe(&canvas, |_, _, cx| cx.notify()),
            cx.observe(&session, |_, _, cx| cx.notify()),
            cx.subscribe(&name_input, move |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let name = name_for_subscription.read(cx).value().to_string();
                    name_session.update(cx, |session, cx| session.set_name(name, cx));
                }
            }),
        ];

        Self {
            session,
            canvas,
            name_input,
            url_input,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }
}

impl Render for MultiuserCanvasPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (status, mode) = {
            let session = self.session.read(cx);
            (session.status(), session.mode())
        };
        let tool_id = self.canvas.read(cx).active_tool_id().to_string();
        let transform = self.canvas.read(cx).transform();
        let peers: Vec<PeerCursor> = self.session.read(cx).peers().cloned().collect();
        let identity = self.session.read(cx).identity().clone();

        v_flex()
            .id("multiuser-canvas-page")
            .track_focus(&self.focus_handle)
            .key_context("MultiuserCanvas")
            .size_full()
            .on_action(cx.listener(|this, _: &DeleteSelection, _, cx| {
                this.session
                    .update(cx, |session, cx| session.delete_selected(cx))
            }))
            .on_action(cx.listener(|this, _: &ClearBoard, _, cx| {
                this.session
                    .update(cx, |session, cx| session.clear_board(cx))
            }))
            .child(
                v_flex()
                    .p_6()
                    .gap_4()
                    .flex_shrink_0()
                    .child(view::render_header(status, cx))
                    .child(view::render_toolbar(
                        &self.session,
                        &self.canvas,
                        &self.name_input,
                        &self.url_input,
                        status,
                        mode,
                        &tool_id,
                        cx,
                    )),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(view::render_canvas_area(self, &peers, transform, cx))
                    .child(view::render_peer_panel(&peers, &identity, cx)),
            )
    }
}
