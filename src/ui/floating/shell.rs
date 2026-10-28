#[cfg(not(target_family = "wasm"))]
use gpui_kit::component::v_flex;
use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::{prelude::*, *};

#[cfg(not(target_family = "wasm"))]
use super::DismissFloating;
use super::spec::FloatingWindowId;

#[cfg(not(target_family = "wasm"))]
pub(crate) struct FloatingShell {
    id: FloatingWindowId,
    title: SharedString,
    content: AnyView,
    focus_handle: FocusHandle,
}

#[cfg(not(target_family = "wasm"))]
impl FloatingShell {
    pub(crate) fn new(
        id: FloatingWindowId,
        title: SharedString,
        content: AnyView,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id,
            title,
            content,
            focus_handle: cx.focus_handle(),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
impl Render for FloatingShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .key_context("floating")
            .track_focus(&self.focus_handle)
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .on_action(cx.listener(|_, _: &DismissFloating, window, _| {
                window.remove_window();
            }))
            .child(float_header(&self.id, &self.title, cx))
            .child(div().size_full().child(self.content.clone()))
    }
}

#[cfg(not(target_family = "wasm"))]
impl Focusable for FloatingShell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// The float header carries the title and the drag affordance only; no
/// window-control button (widgets are chromeless; Esc dismisses).
pub(crate) fn float_header(_id: &FloatingWindowId, title: &SharedString, cx: &App) -> Div {
    let header = h_flex()
        .w_full()
        .h_8()
        .px_2()
        .items_center()
        .flex_shrink_0()
        .border_b_1()
        .border_color(cx.theme().border)
        .cursor_grab()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(title.clone()),
        );
    #[cfg(not(target_os = "macos"))]
    let header = header.on_mouse_down(MouseButton::Left, |_, window, _| {
        window.start_window_move();
    });
    header
}
