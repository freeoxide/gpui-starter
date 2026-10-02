//! Editor section, ported from the upstream `EditorStory`: a tabbed code
//! editor with font controls plus the text and range decoration collections.

use gpui_kit::component::{
    ActiveTheme as _,
    button::{Button, DropdownButton},
    h_flex,
    input::{
        Editor, EditorState, RangeDecoration, RangeDecorationCollection, RangeDecorationStyle,
        TabSize, TextDecoration, TextDecorationCollection,
    },
    menu::PopupMenuItem,
    tab::TabBar,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

const EXAMPLE_CODE: &str = include_str!("fixtures/editor_preview.rs");

const DECORATION_TEXT: &str = "Decoration styles\nColor highlights important text.\nItalic adds emphasis.\nUnderline marks review text.\n\nFill: marks a tracked range.\n\nFrame: outlines a tracked range.";

pub struct EditorSection {
    editor_state: Entity<EditorState>,
    decorations_state: Entity<EditorState>,
    _decorations: TextDecorationCollection,
    _range_decorations: RangeDecorationCollection,
    active_tab: usize,
    readonly: bool,
    font_family: Option<SharedString>,
    font_size: Pixels,
}

impl EditorSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("rust")
                .folding(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(EXAMPLE_CODE)
        });

        let decorations_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("text")
                .default_value(DECORATION_TEXT)
        });

        let marker = "Decoration styles";
        let color_range = "Color";
        let italic_range = "Italic";
        let underline_range = "Underline";
        let fill_range = "marks a tracked range.";
        let frame_range = "outlines a tracked range.";
        let marker_start = DECORATION_TEXT.find(marker).unwrap_or_default();
        let color_start = DECORATION_TEXT.find(color_range).unwrap_or_default();
        let italic_start = DECORATION_TEXT.find(italic_range).unwrap_or_default();
        let underline_start = DECORATION_TEXT.find(underline_range).unwrap_or_default();
        let fill_start = DECORATION_TEXT.find(fill_range).unwrap_or_default();
        let frame_start = DECORATION_TEXT.find(frame_range).unwrap_or_default();
        let decorations = decorations_state.update(cx, |state, cx| {
            state.create_decorations_collection(
                vec![
                    TextDecoration::new(
                        marker_start..marker_start + marker.len(),
                        HighlightStyle {
                            background_color: Some(cx.theme().warning.opacity(0.2)),
                            font_weight: Some(FontWeight::BOLD),
                            color: Some(cx.theme().danger),
                            ..Default::default()
                        },
                    ),
                    TextDecoration::new(
                        color_start..color_start + color_range.len(),
                        HighlightStyle {
                            color: Some(cx.theme().success),
                            font_weight: Some(FontWeight::BOLD),
                            font_style: Some(FontStyle::Italic),
                            ..Default::default()
                        },
                    ),
                    TextDecoration::new(
                        italic_start..italic_start + italic_range.len(),
                        HighlightStyle {
                            color: Some(cx.theme().info),
                            font_style: Some(FontStyle::Italic),
                            ..Default::default()
                        },
                    ),
                    TextDecoration::new(
                        underline_start..underline_start + underline_range.len(),
                        HighlightStyle {
                            underline: Some(UnderlineStyle {
                                color: Some(cx.theme().warning),
                                thickness: px(2.),
                                wavy: true,
                            }),
                            ..Default::default()
                        },
                    ),
                ],
                cx,
            )
        });

        // Geometry is a separate owner from text styling. Both follow edits,
        // including newlines inserted before these ranges.
        let range_decorations = decorations_state.update(cx, |state, cx| {
            state.create_range_decorations_collection(
                vec![
                    RangeDecoration::new(fill_start..fill_start + fill_range.len())
                        .with_style(RangeDecorationStyle::Fill),
                    RangeDecoration::new(frame_start..frame_start + frame_range.len()),
                ],
                cx,
            )
        });

        Self {
            editor_state,
            decorations_state,
            _decorations: decorations,
            _range_decorations: range_decorations,
            active_tab: 0,
            readonly: false,
            font_family: None,
            font_size: cx.theme().mono_font_size,
        }
    }

    /// The font families to offer beside the theme's own monospace one.
    const FONT_FAMILIES: [&'static str; 3] = ["Menlo", "Consolas", "Monaco"];

    /// The font sizes to switch the editor between.
    const FONT_SIZES: [Pixels; 4] = [px(11.), px(13.), px(16.), px(20.)];

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let story = cx.entity();
        let readonly = self.readonly;
        let family = self.font_family.clone();
        let size = self.font_size;

        DropdownButton::new("editor-options")
            .button(Button::new("editor-options-trigger").label("Options"))
            .dropdown_menu(move |menu, window, _| {
                let menu = menu.item(PopupMenuItem::new("Readonly").checked(readonly).on_click(
                    window.listener_for(&story, |this, _, _, cx| {
                        this.readonly = !this.readonly;
                        cx.notify();
                    }),
                ));

                let menu = std::iter::once(None)
                    .chain(Self::FONT_FAMILIES.map(|name| Some(SharedString::from(name))))
                    .fold(menu.separator().label("Font family"), |menu, item| {
                        let label = item.clone().unwrap_or_else(|| "Theme default".into());

                        menu.item(PopupMenuItem::new(label).checked(family == item).on_click(
                            window.listener_for(&story, move |this, _, _, cx| {
                                this.font_family = item.clone();
                                cx.notify();
                            }),
                        ))
                    });

                Self::FONT_SIZES
                    .iter()
                    .fold(menu.separator().label("Font size"), |menu, &item| {
                        menu.item(
                            PopupMenuItem::new(item.to_string())
                                .checked(size == item)
                                .on_click(window.listener_for(&story, move |this, _, _, cx| {
                                    this.font_size = item;
                                    cx.notify();
                                })),
                        )
                    })
            })
    }
}

impl Render for EditorSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        TabBar::new("editor-story-tabs")
                            .w_64()
                            .underline()
                            .selected_index(self.active_tab)
                            .on_click(cx.listener(|this, selected: &usize, _, cx| {
                                this.active_tab = *selected;
                                cx.notify();
                            }))
                            .child("Code")
                            .child("Decorations"),
                    )
                    .child(self.render_toolbar(cx)),
            )
            // No tree-sitter features are enabled, so no highlighter exists and
            // fold arrows never appear; gutter, decorations, and font controls are unaffected.
            .child(div().min_h_0().h(rems(30.)).child(if self.active_tab == 0 {
                Editor::new(&self.editor_state)
                    .when_some(self.font_family.clone(), |this, family| {
                        this.font_family(family)
                    })
                    .text_size(self.font_size)
                    .readonly(self.readonly)
                    .size_full()
                    .into_any_element()
            } else {
                Editor::new(&self.decorations_state)
                    .when_some(self.font_family.clone(), |this, family| {
                        this.font_family(family)
                    })
                    .text_size(self.font_size)
                    .readonly(self.readonly)
                    .size_full()
                    .into_any_element()
            }))
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "editor",
        "Editor",
        "A code editor with gutter, readonly state, and font controls, plus a decorations tab showing styled text ranges and tracked fill and frame ranges. Syntax highlighting and folding need the crate's tree-sitter features, which this app does not enable, so the buffers render plain.",
        EditorSection::view(window, cx),
    ));
}
