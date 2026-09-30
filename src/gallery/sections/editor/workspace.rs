//! Editor Workspace section, ported from the upstream `editor` example: a
//! resizable split of a language-sample file tree and the editor, with the
//! example's option row and go-to-line dialog.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{self, Editor, EditorState, Input, InputState, TabSize},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    status_bar::StatusBar,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

const DEFAULT_SOURCE: &str = include_str!("fixtures/test.rs");

/// One language sample the file tree can load, embedded at compile time.
struct Sample {
    file: &'static str,
    language: &'static str,
    source: &'static str,
}

const SAMPLES: &[Sample] = &[
    Sample {
        file: "test.astro",
        language: "astro",
        source: include_str!("fixtures/samples/test.astro"),
    },
    Sample {
        file: "test.c",
        language: "c",
        source: include_str!("fixtures/samples/test.c"),
    },
    Sample {
        file: "test.go",
        language: "go",
        source: include_str!("fixtures/samples/test.go"),
    },
    Sample {
        file: "test.html",
        language: "html",
        source: include_str!("fixtures/samples/test.html"),
    },
    Sample {
        file: "test.js",
        language: "javascript",
        source: include_str!("fixtures/samples/test.js"),
    },
    Sample {
        file: "test.json",
        language: "json",
        source: include_str!("fixtures/samples/test.json"),
    },
    Sample {
        file: "test.kt",
        language: "kotlin",
        source: include_str!("fixtures/samples/test.kt"),
    },
    Sample {
        file: "test.lua",
        language: "lua",
        source: include_str!("fixtures/samples/test.lua"),
    },
    Sample {
        file: "test.md",
        language: "markdown",
        source: include_str!("fixtures/samples/test.md"),
    },
    Sample {
        file: "test.nv",
        language: "navi",
        source: include_str!("fixtures/samples/test.nv"),
    },
    Sample {
        file: "test.php",
        language: "php",
        source: include_str!("fixtures/samples/test.php"),
    },
    Sample {
        file: "test.py",
        language: "python",
        source: include_str!("fixtures/samples/test.py"),
    },
    Sample {
        file: "test.rb",
        language: "ruby",
        source: include_str!("fixtures/samples/test.rb"),
    },
    Sample {
        file: "test.sql",
        language: "sql",
        source: include_str!("fixtures/samples/test.sql"),
    },
    Sample {
        file: "test.svelte",
        language: "svelte",
        source: include_str!("fixtures/samples/test.svelte"),
    },
    Sample {
        file: "test.ts",
        language: "typescript",
        source: include_str!("fixtures/samples/test.ts"),
    },
    Sample {
        file: "test.zig",
        language: "zig",
        source: include_str!("fixtures/samples/test.zig"),
    },
];

pub struct EditorWorkspaceSection {
    editor: Entity<EditorState>,
    tree_state: Entity<TreeState>,
    go_to_line_state: Entity<InputState>,
    line_number: bool,
    indent_guides: bool,
    soft_wrap: bool,
    show_whitespaces: bool,
    folding: bool,
    readonly: bool,
    scroll_beyond_last_line: Option<usize>,
    cursor_surrounding_lines: Option<usize>,
}

impl EditorWorkspaceSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("rust")
                .line_number(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value(DEFAULT_SOURCE)
                .placeholder("Enter your code here…")
        });

        let go_to_line_state = cx.new(|cx| InputState::new(window, cx));

        // The example walks the working directory; the gallery embeds its
        // samples, so the tree is one expanded folder of language files.
        let samples = TreeItem::new("editor-workspace-samples", "samples")
            .expanded(true)
            .children(SAMPLES.iter().map(|sample| {
                TreeItem::new(
                    format!("editor-workspace-file-{}", sample.file),
                    sample.file,
                )
            }));
        let tree_state = cx.new(|cx| {
            let mut state = TreeState::new(cx);
            state.set_items(vec![samples], cx);
            state
        });

        Self {
            editor,
            tree_state,
            go_to_line_state,
            line_number: true,
            indent_guides: true,
            soft_wrap: false,
            show_whitespaces: false,
            folding: true,
            readonly: false,
            scroll_beyond_last_line: None,
            cursor_surrounding_lines: None,
        }
    }

    fn open_sample(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sample) = SAMPLES.iter().find(|sample| sample.file == file) else {
            return;
        };

        self.editor.update(cx, |state, cx| {
            state.set_highlighter(sample.language, cx);
            state.set_value(sample.source, window, cx);
        });
        cx.notify();
    }

    fn go_to_line(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        let input_state = self.go_to_line_state.clone();

        window.open_alert_dialog(cx, move |dialog, window, cx| {
            input_state.update(cx, |state, cx| {
                let cursor_pos = editor.read(cx).cursor_position();
                state.set_placeholder(
                    format!("{}:{}", cursor_pos.line, cursor_pos.character),
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
                        let mut parts = query
                            .split(':')
                            .map(|s| s.trim().parse::<usize>().ok())
                            .collect::<Vec<_>>()
                            .into_iter();
                        let Some(line) = parts.next().and_then(|l| l) else {
                            return false;
                        };
                        let column = parts.next().and_then(|c| c).unwrap_or(1);
                        let position = input::Position::new(
                            line.saturating_sub(1) as u32,
                            column.saturating_sub(1) as u32,
                        );

                        editor.update(cx, |state, cx| {
                            state.set_cursor_position(position, window, cx);
                        });

                        true
                    }
                })
        });
    }

    fn render_file_tree(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        tree(
            &self.tree_state,
            move |_ix, entry, _selected, _window, cx| {
                view.update(cx, |_, cx| {
                    let item = entry.item();
                    let icon = if !entry.is_folder() {
                        IconName::File
                    } else if entry.is_expanded() {
                        IconName::FolderOpen
                    } else {
                        IconName::Folder
                    };

                    ListItem::new(item.id.clone())
                        .w_full()
                        .rounded(cx.theme().radius)
                        .py_0p5()
                        .px_2()
                        .pl(px(16.) * entry.depth() + px(8.))
                        .child(h_flex().gap_2().child(icon).child(item.label.clone()))
                        .on_click(cx.listener({
                            let item = item.clone();
                            move |this, _, window, cx| {
                                if item.is_folder() {
                                    return;
                                }

                                this.open_sample(&item.label, window, cx);
                            }
                        }))
                })
            },
        )
        .text_sm()
        .p_1()
        .bg(cx.theme().sidebar)
        .text_color(cx.theme().sidebar_foreground)
        .h_full()
    }

    fn render_line_number_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("editor-workspace-line-number")
            .when(self.line_number, |this| this.icon(IconName::Check))
            .label("Line Number")
            .ghost()
            .xsmall()
            .on_click(cx.listener(|this, _, window, cx| {
                this.line_number = !this.line_number;
                this.editor.update(cx, |state, cx| {
                    state.set_line_number(this.line_number, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_soft_wrap_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("editor-workspace-soft-wrap")
            .ghost()
            .xsmall()
            .when(self.soft_wrap, |this| this.icon(IconName::Check))
            .label("Soft Wrap")
            .on_click(cx.listener(|this, _, window, cx| {
                this.soft_wrap = !this.soft_wrap;
                this.editor.update(cx, |state, cx| {
                    state.set_soft_wrap(this.soft_wrap, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_show_whitespaces_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("editor-workspace-show-whitespace")
            .ghost()
            .xsmall()
            .when(self.show_whitespaces, |this| this.icon(IconName::Check))
            .label("Show Whitespaces")
            .on_click(cx.listener(|this, _, window, cx| {
                this.show_whitespaces = !this.show_whitespaces;
                this.editor.update(cx, |state, cx| {
                    state.set_show_whitespaces(this.show_whitespaces, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_indent_guides_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("editor-workspace-indent-guides")
            .ghost()
            .xsmall()
            .when(self.indent_guides, |this| this.icon(IconName::Check))
            .label("Indent Guides")
            .on_click(cx.listener(|this, _, window, cx| {
                this.indent_guides = !this.indent_guides;
                this.editor.update(cx, |state, cx| {
                    state.set_indent_guides(this.indent_guides, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_folding_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("editor-workspace-folding")
            .ghost()
            .xsmall()
            .when(self.folding, |this| this.icon(IconName::Check))
            .label("Folding")
            .on_click(cx.listener(|this, _, window, cx| {
                this.folding = !this.folding;
                this.editor.update(cx, |state, cx| {
                    state.set_folding(this.folding, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_readonly_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("editor-workspace-readonly")
            .ghost()
            .xsmall()
            .when(self.readonly, |this| this.icon(IconName::Check))
            .label("Read only")
            .on_click(cx.listener(|this, _, _, cx| {
                this.readonly = !this.readonly;
                cx.notify();
            }))
    }

    /// Cycle an `Option<usize>` row setting through a few demo values.
    fn cycle_rows(v: Option<usize>) -> Option<usize> {
        match v {
            None => Some(0),
            Some(0) => Some(3),
            Some(3) => Some(8),
            _ => None,
        }
    }

    fn rows_label(v: Option<usize>) -> String {
        match v {
            None => "default".to_string(),
            Some(n) => n.to_string(),
        }
    }

    fn render_scroll_beyond_last_line_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new("editor-workspace-scroll-beyond-last-line")
            .ghost()
            .xsmall()
            .label(format!(
                "Scroll Beyond: {}",
                Self::rows_label(self.scroll_beyond_last_line)
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.scroll_beyond_last_line = Self::cycle_rows(this.scroll_beyond_last_line);
                this.editor.update(cx, |state, cx| {
                    state.set_scroll_beyond_last_line(this.scroll_beyond_last_line, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_cursor_surrounding_lines_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new("editor-workspace-cursor-surrounding-lines")
            .ghost()
            .xsmall()
            .label(format!(
                "Cursor Surrounding: {}",
                Self::rows_label(self.cursor_surrounding_lines)
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.cursor_surrounding_lines = Self::cycle_rows(this.cursor_surrounding_lines);
                this.editor.update(cx, |state, cx| {
                    state.set_cursor_surrounding_lines(this.cursor_surrounding_lines, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_go_to_line_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        let position = self.editor.read(cx).cursor_position();
        let cursor = self.editor.read(cx).cursor();

        Button::new("editor-workspace-line-column")
            .ghost()
            .xsmall()
            .label(format!(
                "{}:{} ({} byte)",
                position.line + 1,
                position.character + 1,
                cursor
            ))
            .tooltip("Go to Line/Column")
            .on_click(cx.listener(Self::go_to_line))
    }
}

impl Render for EditorWorkspaceSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                div().w_full().min_h_0().h(rems(30.)).child(
                    h_resizable("editor-workspace-container")
                        .child(
                            resizable_panel()
                                .size(px(240.))
                                .child(self.render_file_tree(window, cx)),
                        )
                        .child(
                            Editor::new(&self.editor)
                                .readonly(self.readonly)
                                .bordered(false)
                                .p_0()
                                .h(relative(1.))
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size)
                                .into_any_element(),
                        ),
                ),
            )
            .child(
                StatusBar::new()
                    .left(self.render_line_number_button(window, cx))
                    .left(self.render_soft_wrap_button(window, cx))
                    .left(self.render_show_whitespaces_button(window, cx))
                    .left(self.render_indent_guides_button(window, cx))
                    .left(self.render_folding_button(window, cx))
                    .left(self.render_readonly_button(window, cx))
                    .left(self.render_scroll_beyond_last_line_button(window, cx))
                    .left(self.render_cursor_surrounding_lines_button(window, cx))
                    .right(self.render_go_to_line_button(window, cx)),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "editor-workspace",
        "Editor Workspace",
        "A resizable split of a file tree and the editor it loads samples into, with the example's gutter, wrap, whitespace, and scrolling options plus a go-to-line dialog. The status bar button shows the cursor position; selecting a file switches the editor's language. Syntax highlighting and fold markers need the crate's tree-sitter features, which this app does not enable.",
        EditorWorkspaceSection::view(window, cx),
    ));
}
