//! HTML Render section, ported from the upstream `html` example: edit HTML
//! source beside its live rendered preview, with a copy-format switch.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    input::{Editor, EditorState, InputEvent, TabSize},
    resizable::h_resizable,
    status_bar::StatusBar,
    text::{SelectionFormat, html},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

const EXAMPLE: &str = include_str!("fixtures/samples/test.html");

pub struct HtmlRenderSection {
    input_state: Entity<EditorState>,
    /// Whether copying a selection yields the rendered text or its source.
    selection_format: SelectionFormat,
    _subscribe: Subscription,
}

impl HtmlRenderSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("html")
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(EXAMPLE)
                .placeholder("Enter your HTML here…")
        });

        let _subscribe = cx.subscribe(&input_state, |_, _, _: &InputEvent, cx| {
            cx.notify();
        });

        Self {
            input_state,
            selection_format: SelectionFormat::Plain,
            _subscribe,
        }
    }
}

impl Render for HtmlRenderSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                div().w_full().min_h_0().h(rems(37.5)).child(
                    h_resizable("html-render-container")
                        .child(
                            div()
                                .size_full()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size)
                                .child(
                                    Editor::new(&self.input_state)
                                        .h(relative(1.))
                                        .appearance(false),
                                )
                                .into_any(),
                        )
                        // The gallery pane owns scrolling, so the preview
                        // clips instead of scrolling inside its panel.
                        .child(
                            div()
                                .size_full()
                                .overflow_hidden()
                                .child(
                                    html(self.input_state.read(cx).value())
                                        .px_5()
                                        .selectable(true)
                                        .selection_format(self.selection_format),
                                )
                                .into_any(),
                        ),
                ),
            )
            .child(
                StatusBar::new().right(
                    Button::new("html-render-selection-format")
                        .ghost()
                        .xsmall()
                        .label(match self.selection_format {
                            SelectionFormat::Plain => "Selection: Plain",
                            SelectionFormat::Source => "Selection: Source",
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.selection_format = match this.selection_format {
                                SelectionFormat::Plain => SelectionFormat::Source,
                                SelectionFormat::Source => SelectionFormat::Plain,
                            };
                            cx.notify();
                        })),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "html-render",
        "HTML Render",
        "Edit HTML source on the left and read its rendered form on the right; the divider between them drags. The status bar switch changes whether copying a selection from the preview yields the rendered text or the underlying source.",
        HtmlRenderSection::view(window, cx),
    ));
}
