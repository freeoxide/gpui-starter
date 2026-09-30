//! Textarea section, ported from the upstream `TextareaStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, Size,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup,
        AttachmentMedia, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    h_flex,
    hover_card::HoverCard,
    input::{InputEvent, Textarea, TextareaState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};
use super::tokens::TokenComposer;

const DEMO_TEXT: &str = "\
Hello 世界，this is GPUI Kit.

GPUI Kit is a collection of UI components for the GPUI framework, including:

Button, Input, Checkbox, Radio, Dropdown, Tab, and more…

> This application is still under development, not published yet.

![image](screenshot-1.png)

![image](screenshot-2.png)

## Demo

If you want to see the demo, here are some demo applications.";

const NO_WRAP_TEXT: &str = "\
This is a very long line of text to test if the horizontal scrolling function is working properly, and it should not wrap automatically but display a horizontal scrollbar.\n\
The second line is also very long text, used to test the horizontal scrolling effect under multiple lines, and you can input more content to test.\n\
The third line: Here you can input other long text content that requires horizontal scrolling.";

const AUTO_GROW_TEXT: &str = "\
Hello 世界 this is a very long line of text \
to test if the horizontal scrolling function is working \
properly, and it should not wrap automatically but display \
a horizontal scrollbar.\n\
The second line is also very long text, used to test the \
horizontal scrolling effect under multiple lines, and you \
can input more content to test.\n\
The third line: Here you can input other long text content \
that requires horizontal scrolling.";

struct ComposerAttachment {
    id: u64,
    title: String,
    detail: String,
}

pub struct TextareaSection {
    tokens: Entity<TokenComposer>,
    textarea: Entity<TextareaState>,
    textarea_auto_grow: Entity<TextareaState>,
    textarea_no_wrap: Entity<TextareaState>,
    textarea_auto_grow_no_wrap: Entity<TextareaState>,
    chat_input: Entity<TextareaState>,
    chat_messages: Vec<String>,
    composer: Entity<TextareaState>,
    attachments: Vec<ComposerAttachment>,
    /// Counter for attachment ids; `Image::id` is a content hash, so pasting
    /// the same image twice would collide.
    next_attachment_id: u64,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl TextareaSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let textarea = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(10)
                .placeholder("Enter text here…")
                .searchable(true)
                .default_value(DEMO_TEXT)
        });

        let textarea_no_wrap = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(6)
                .soft_wrap(false)
                .default_value(NO_WRAP_TEXT)
        });

        let textarea_auto_grow = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 5)
                .placeholder("Enter text here…")
                .default_value(AUTO_GROW_TEXT)
        });

        let textarea_auto_grow_no_wrap = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 5)
                .soft_wrap(false)
                .placeholder("Enter text here…")
                .default_value("Hello 世界，this is GPUI Kit.")
        });

        let chat_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 5)
                .submit_on_enter(true)
                .placeholder("Type a message, Enter to send, Shift+Enter for newline")
        });

        let composer = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 5)
                .placeholder("Paste a screenshot here, it becomes an attachment above")
        });

        let _subscriptions = vec![cx.subscribe_in(
            &chat_input,
            window,
            |this: &mut Self, input, event, window, cx| match event {
                InputEvent::PressEnter { shift, .. } if !shift => {
                    let text = input.read(cx).value().trim().to_string();
                    if !text.is_empty() {
                        this.chat_messages.push(text);
                        input.update(cx, |state, cx| {
                            state.set_value("", window, cx);
                        });
                        cx.notify();
                    }
                }
                _ => {}
            },
        )];

        Self {
            tokens: TokenComposer::new("textarea", true, window, cx),
            textarea,
            textarea_auto_grow,
            textarea_no_wrap,
            textarea_auto_grow_no_wrap,
            chat_input,
            chat_messages: Vec::new(),
            composer,
            attachments: Vec::new(),
            next_attachment_id: 0,
            size: Size::Medium,
            _subscriptions,
        }
    }

    fn insert_text(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.textarea.update(cx, |input, cx| {
            input.insert("Hello 你好", window, cx);
        });
    }

    fn replace_text(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.textarea.update(cx, |input, cx| {
            input.replace("Hello 你好", window, cx);
        });
    }
}

impl Render for TextareaSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let loc = self.textarea.read(cx).cursor_position();
        let size = self.size;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("textarea-size", size).into_any_element(),
            ]))
            .child(
                section("textarea-default", "Textarea")
                    .w(rems(35.))
                    .child(
                        v_flex()
                            .gap_2()
                            .w_full()
                            .child(
                                Textarea::new(&self.textarea)
                                    .with_size(size)
                                    .h(rems(20.)),
                            )
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                Button::new("textarea-insert-text")
                                                    .outline()
                                                    .xsmall()
                                                    .label("Insert Text")
                                                    .on_click(cx.listener(Self::insert_text)),
                                            )
                                            .child(
                                                Button::new("textarea-replace-text")
                                                    .outline()
                                                    .xsmall()
                                                    .label("Replace Text")
                                                    .on_click(cx.listener(Self::replace_text)),
                                            ),
                                    )
                                    .child(format!("{}:{}", loc.line, loc.character)),
                            ),
                    ),
            )
            .child(
                section("textarea-no-wrap", "No Wrap")
                    .w(rems(35.))
                    .child(
                        Textarea::new(&self.textarea_no_wrap)
                            .with_size(size)
                            .h(rems(12.5)),
                    ),
            )
            .child(
                section("textarea-auto-grow", "Auto Grow")
                    .w(rems(35.))
                    .child(Textarea::new(&self.textarea_auto_grow).with_size(size)),
            )
            .child(
                section("textarea-auto-grow-no-wrap", "Auto Grow with No Wrap")
                    .w(rems(35.))
                    .child(
                        Textarea::new(&self.textarea_auto_grow_no_wrap).with_size(size),
                    ),
            )
            .child(
                section("textarea-submit-on-enter", "Submit on Enter (Chat)")
                    .w(rems(35.))
                    .child(
                        v_flex()
                            .gap_2()
                            .w_full()
                            .child(v_flex().gap_1().children(
                                self.chat_messages.iter().enumerate().map(|(i, msg)| {
                                    div()
                                        .id(("textarea-chat-msg", i))
                                        .px_2()
                                        .py_1()
                                        .rounded(cx.theme().radius)
                                        .bg(cx.theme().muted)
                                        .child(msg.clone())
                                }),
                            ))
                            .child(Textarea::new(&self.chat_input).with_size(size)),
                    ),
            )
            .child(
                section("textarea-paste-images", "Paste Images (Composer)")
                    .description("Paste a screenshot, it lands as an attachment pill above.")
                    .w(rems(35.))
                    .child(
                        v_flex()
                            .gap_2()
                            .w_full()
                            .when(!self.attachments.is_empty(), |this| {
                                this.child(
                                    AttachmentGroup::new("textarea-composer-attachments").children(
                                        self.attachments.iter().map(|attachment| {
                                            let id = attachment.id;
                                            HoverCard::new(("textarea-pasted-preview", id))
                                                .trigger(
                                                    Attachment::new()
                                                        .media(AttachmentMedia::new().child(
                                                            Icon::new(IconName::FileText).small(),
                                                        ))
                                                        .content(
                                                            AttachmentContent::new().title(
                                                                AttachmentTitle::new(
                                                                    attachment.title.clone(),
                                                                ),
                                                            )
                                                            .description(
                                                                AttachmentDescription::new(
                                                                    attachment.detail.clone(),
                                                                ),
                                                            ),
                                                        )
                                                        .actions(
                                                            AttachmentActions::new().child(
                                                                Button::new((
                                                                    "textarea-remove-pasted",
                                                                    id,
                                                                ))
                                                                .ghost()
                                                                .xsmall()
                                                                .icon(IconName::Close)
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        this.attachments.retain(
                                                                            |item| item.id != id,
                                                                        );
                                                                        cx.notify();
                                                                    },
                                                                )),
                                                        ),
                                                ),
                                            )
                                            .child(
                                                    div()
                                                        .text_sm()
                                                        .child(attachment.detail.clone()),
                                                )
                                                .into_any_element()
                                        }),
                                    ),
                                )
                            })
                            .child({
                                let view = cx.entity().downgrade();
                                Textarea::new(&self.composer)
                                    .with_size(size)
                                    .on_paste(move |item, _, cx| {
                                        let images: Vec<_> = item
                                            .entries()
                                            .iter()
                                            .filter_map(|entry| match entry {
                                                ClipboardEntry::Image(image) => {
                                                    Some(image.clone())
                                                }
                                                _ => None,
                                            })
                                            .collect();
                                        if images.is_empty() {
                                            return false;
                                        }
                                        view.update(cx, |this: &mut Self, cx| {
                                            for image in images {
                                                let id = this.next_attachment_id;
                                                this.next_attachment_id += 1;
                                                this.attachments.push(ComposerAttachment {
                                                    id,
                                                    title: format!("pasted-image-{id}.png"),
                                                    detail: format!(
                                                        "{:?} - {} bytes",
                                                        image.format,
                                                        image.bytes.len()
                                                    ),
                                                });
                                            }
                                            cx.notify();
                                        })
                                        .ok();
                                        true
                                    })
                            }),
                    ),
            )
            .child(
                section("textarea-tokens", "Atomic inline tokens")
                    .description(
                        "References keep their identity through selection, deletion and undo. Copy returns the underlying text.",
                    )
                    .w_full()
                    .child(self.tokens.clone()),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "textarea",
        "Textarea",
        "Input with multi-line mode.",
        TextareaSection::view(window, cx),
    ));
}
