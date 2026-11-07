//! Large Text section, ported from the upstream `large-text` example: a
//! ten-thousand-line document in a soft-wrapping textarea with a go-to-line
//! dialog.

use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState, TabSize, Textarea, TextareaState},
    v_flex,
};
use gpui_kit::*;

use super::parse_go_to_line;
use crate::gallery::registry::GallerySection;

pub struct LargeTextSection {
    editor: Entity<TextareaState>,
    go_to_line_state: Entity<InputState>,
    soft_wrap: bool,
    _subscribes: Vec<Subscription>,
}

impl LargeTextSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = "这是一个中文演示段落，用于展示更多的 [Markdown GFM] 内容。您可以在此尝试使用使用**粗体**、*斜体*和`代码`等样式。これは日本語のデモ段落です。Markdown の多言語サポートを示すためのテキストが含まれています。例えば、、**ボールド**、_イタリック_、および`コード`のスタイルなどを試すことができます。\n".repeat(10000);

        let editor = cx.new(|cx| {
            TextareaState::new(window, cx)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(true)
                .placeholder("Enter your code here…")
                .default_value(text)
        });
        let go_to_line_state = cx.new(|cx| InputState::new(window, cx));

        // Cursor moves emit no InputEvent; observing catches them so the
        // readout tracks the caret.
        let _subscribes = vec![cx.observe(&editor, |_, _, cx| {
            cx.notify();
        })];

        Self {
            editor,
            go_to_line_state,
            soft_wrap: true,
            _subscribes,
        }
    }

    fn go_to_line(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        let input_state = self.go_to_line_state.clone();

        window.open_dialog(cx, move |dialog, window, cx| {
            input_state.update(cx, |state, cx| {
                let position = editor.read(cx).cursor_position();
                // 1-based like the parser, so typing the hint verbatim stays
                // at the cursor.
                state.set_placeholder(
                    format!("{}:{}", position.line + 1, position.character + 1),
                    window,
                    cx,
                );
                state.focus(window, cx);
            });

            dialog
                .title("Go to line")
                .child(Input::new(&input_state))
                .on_ok({
                    let editor = editor.clone();
                    let input_state = input_state.clone();
                    move |_, window, cx| {
                        let query = input_state.read(cx).value();
                        let Some(position) = parse_go_to_line(&query) else {
                            return false;
                        };

                        editor.update(cx, |state, cx| {
                            state.set_cursor_position(position, window, cx);
                        });

                        true
                    }
                })
        });
    }

    fn toggle_soft_wrap(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.soft_wrap = !self.soft_wrap;
        self.editor.update(cx, |state, cx| {
            state.set_soft_wrap(self.soft_wrap, window, cx);
        });
        cx.notify();
    }
}

impl Render for LargeTextSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().w_full().gap_3().p_4().child(
            v_flex()
                .w_full()
                .child(Textarea::new(&self.editor).bordered(false).h(rems(30.)))
                .child(
                    h_flex()
                        .justify_between()
                        .text_sm()
                        .bg(cx.theme().secondary)
                        .py_1p5()
                        .px_4()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .text_color(cx.theme().muted_foreground)
                        .child(h_flex().gap_3().child({
                            Button::new("large-text-soft-wrap")
                                .ghost()
                                .xsmall()
                                .label("Soft Wrap")
                                .selected(self.soft_wrap)
                                .on_click(cx.listener(Self::toggle_soft_wrap))
                        }))
                        .child({
                            let loc = self.editor.read(cx).cursor_position();
                            let cursor = self.editor.read(cx).cursor();

                            Button::new("large-text-line-column")
                                .ghost()
                                .xsmall()
                                .label(format!(
                                    "{}:{} ({} byte)",
                                    loc.line + 1,
                                    loc.character + 1,
                                    cursor
                                ))
                                .on_click(cx.listener(Self::go_to_line))
                        }),
                ),
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "large-text",
        "Large Text",
        "A ten-thousand-line document in one textarea: soft wrap reflows it, and the cursor readout opens a go-to-line dialog that scrolls the caret into view.",
        LargeTextSection::view(window, cx),
    ));
}
