//! Touch Selection section, ported from the upstream `touch_selection`
//! example: the handles and the edit menu a long press leaves behind in an
//! `Input`, a `Textarea`, and a `TextView`, driven from a desktop.

use std::{cell::Cell, rc::Rc};

use gpui_kit::component::{
    ActiveTheme as _,
    button::Button,
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    text::TextView,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

pub struct TouchSelectionSection {
    input: Entity<InputState>,
    textarea: Entity<TextareaState>,
    /// Where each control was painted this frame, so a press can land in it.
    input_bounds: Rc<Cell<Bounds<Pixels>>>,
    textarea_bounds: Rc<Cell<Bounds<Pixels>>>,
    text_bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl TouchSelectionSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value("The quick brown fox jumps over the lazy dog")
            }),
            textarea: cx.new(|cx| {
                // Enough lines to scroll inside its fixed height.
                let lines = (1..=12)
                    .map(|n| format!("Line {n}: online content authoring isn't a solved problem."))
                    .collect::<Vec<_>>();
                TextareaState::new(window, cx).default_value(lines.join("\n"))
            }),
            input_bounds: Rc::default(),
            textarea_bounds: Rc::default(),
            text_bounds: Rc::default(),
        }
    }

    /// Wraps a control so its painted bounds are known to the press button.
    fn measured(control: impl IntoElement, bounds: &Rc<Cell<Bounds<Pixels>>>) -> impl IntoElement {
        let bounds = bounds.clone();
        div().relative().child(control).child(
            canvas(move |painted, _, _| bounds.set(painted), |_, _, _, _| {})
                .absolute()
                .inset_0(),
        )
    }

    /// A long press a finger's width into the control's first line of text.
    fn press_button(
        id: &'static str,
        bounds: &Rc<Cell<Bounds<Pixels>>>,
        window: &Window,
    ) -> impl IntoElement {
        let bounds = bounds.clone();
        // Press geometry in window coordinates, not layout: px is the unit
        // the dispatched event consumes.
        let first_line = window.line_height() * 0.5 + px(12.);
        Button::new(id)
            .label("Long press")
            .on_click(move |_, window, cx| {
                let bounds = bounds.get();
                let position = point(bounds.left() + px(48.), bounds.top() + first_line);
                // The click that runs this is itself being dispatched; an
                // event sent from inside it would find no listeners. Send the
                // gesture once the click is over.
                window.defer(cx, move |window, cx| {
                    for phase in [TouchPhase::Started, TouchPhase::Ended] {
                        window.dispatch_event(
                            LongPressEvent {
                                phase,
                                start_position: position,
                                position,
                            }
                            .to_platform_input(),
                            cx,
                        );
                    }
                });
            })
    }

    /// Paragraphs above and below the pressed one, so the pane scrolls with
    /// the selection somewhere in the middle of it.
    fn filler(id: &'static str, paragraphs: usize) -> impl IntoElement {
        let text = (0..paragraphs)
            .map(|_| {
                "Online content authoring isn't a solved problem. You might go with \
                 an HTML-based editor, and hope it gives you the kind of HTML you \
                 want. Or you might use a text-based markup format, and hope your \
                 users understand how to use it."
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        TextView::markdown(id, text).selectable(true)
    }

    fn row(label: &'static str, button: impl IntoElement, cx: &App) -> impl IntoElement {
        h_flex()
            .items_center()
            .justify_between()
            .child(div().text_color(cx.theme().muted_foreground).child(label))
            .child(button)
    }

    fn instructions(cx: &App) -> impl IntoElement {
        let lines = [
            "The word under the press is selected, with a grab handle at each end and an edit menu above it (Cut / Copy / Paste / Select All as they apply).",
            "Drag a handle with the mouse to move that end; the other end stays put.",
            "Drag one handle past the other: they swap, and the selection runs the other way.",
            "Scroll the pane or the textarea: a handle whose end left the view goes away, and the menu steps aside until the scroll ends.",
            "Click anywhere else to drop the handles; an Input also drops them when you type or press Escape, and brings the menu back when you click the selected text.",
        ];
        v_flex()
            .gap_1()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .children(lines.map(|line| div().child(line)))
    }
}

impl Render for TouchSelectionSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .child(Self::instructions(cx))
            .child(
                v_flex()
                    .gap_2()
                    .child(Self::row(
                        "Input",
                        Self::press_button(
                            "touch-selection-press-input",
                            &self.input_bounds,
                            window,
                        ),
                        cx,
                    ))
                    .child(Self::measured(Input::new(&self.input), &self.input_bounds)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Self::row(
                        "Textarea",
                        Self::press_button(
                            "touch-selection-press-textarea",
                            &self.textarea_bounds,
                            window,
                        ),
                        cx,
                    ))
                    .child(Self::measured(
                        Textarea::new(&self.textarea).h(rems(7.5)),
                        &self.textarea_bounds,
                    )),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Self::row(
                        "TextView",
                        Self::press_button("touch-selection-press-text", &self.text_bounds, window),
                        cx,
                    ))
                    // One paragraph above keeps the pressed one on screen at
                    // start; scroll the pane to move it out either way.
                    .child(Self::filler("touch-selection-text-before", 1))
                    .child(Self::measured(
                        TextView::markdown(
                            "touch-selection-text",
                            "ProseMirror tries to bridge the gap between **rich text** and \
                             structured content, by providing a drop-in editor component \
                             with a rigid semantic document model that can be customized \
                             to fit your application.",
                        )
                        .selectable(true),
                        &self.text_bounds,
                    ))
                    .child(Self::filler("touch-selection-text-after", 6)),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "touch-selection",
        "Touch Selection",
        "The touch handles and edit menu a long press leaves behind in an Input, a Textarea, and a TextView. A desktop has no long press, so each Long press button injects the gesture; from there everything is the real thing.",
        TouchSelectionSection::view(window, cx),
    ));
}
