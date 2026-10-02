//! Markdown section, ported from the upstream `markdown` example: a source
//! editor beside its live preview, with custom Markdown plugins (mentions,
//! ticker quotes, user cards, math), per-block copy actions, table export,
//! find-in-preview with range highlights, and preview zoom.

use std::ops::Range;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, WindowExt as _,
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState, TabSize},
    menu::{DropdownMenu as _, PopupMenuItem},
    resizable::h_resizable,
    status_bar::StatusBar,
    text::{
        InlineElement, InlineRenderContext, MarkdownNode, MarkdownParseContext, MarkdownPlugin,
        RangeHighlight, RangeHighlightError, RenderedText, SelectionFormat, TextView,
        TextViewState, TextViewStyle, markdown_ast,
    },
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::mention::MentionPlugin;

const EXAMPLE: &str = include_str!("fixtures/test.md");

#[derive(Clone, Debug, PartialEq, Eq)]
struct MathNode {
    source: String,
    inline: bool,
}

#[derive(Clone, Copy)]
struct TickerQuote {
    name: &'static str,
    price: f64,
    change: f64,
}

#[derive(Clone)]
struct TickerNode {
    symbol: String,
}

#[derive(Clone)]
struct UserCardNode {
    id: String,
}

#[derive(Clone)]
struct TickerPlugin {
    apple_quote: TickerQuote,
    tesla_quote: TickerQuote,
}

impl TickerPlugin {
    fn new(apple_quote: TickerQuote, tesla_quote: TickerQuote) -> Self {
        Self {
            apple_quote,
            tesla_quote,
        }
    }

    fn quote(&self, symbol: &str) -> TickerQuote {
        match symbol {
            "AAPL.US" => self.apple_quote,
            "TSLA.US" => self.tesla_quote,
            _ => TickerQuote {
                name: "Unknown",
                price: 0.0,
                change: 0.0,
            },
        }
    }
}

#[derive(Clone)]
struct UserCardPlugin;

#[derive(Clone)]
struct MathPlugin;

#[derive(Clone)]
struct InlineMathPlugin;

fn html_tag_name(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix('<')?
        .split([' ', '/', '>'])
        .next()
}

fn html_attr(value: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = value.find(&pattern)? + pattern.len();
    let end = value[start..].find('"')?;
    Some(value[start..start + end].to_string())
}

fn ticker_symbol(value: &str) -> Option<&str> {
    let symbol = value.strip_prefix('$')?;
    if symbol.is_empty()
        || !symbol.contains('.')
        || !symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.')
    {
        return None;
    }
    Some(symbol)
}

fn math_markdown(source: &str, inline: bool) -> String {
    if inline {
        format!("${source}$")
    } else {
        format!("$$\n{source}\n$$")
    }
}

fn math_node(source: String, inline: bool, markdown: impl Into<String>) -> MarkdownNode {
    MarkdownNode::new(
        if inline { "inline-math" } else { "math" },
        MathNode {
            source: source.clone(),
            inline,
        },
    )
    .text(prettify_math_source(&source))
    .accessibility_label(format!("Formula: {}", prettify_math_source(&source)))
    .markdown(markdown.into())
}

fn block_math_source(source: &str) -> Option<&str> {
    let source = source.trim();
    let body = source.strip_prefix("$$")?.strip_suffix("$$")?.trim();
    (!body.is_empty()).then_some(body)
}

impl MarkdownPlugin for MathPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "math"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        if let markdown_ast::Node::Math(math) = node {
            return Some(math_node(
                math.value.clone(),
                false,
                cx.node_source(node)
                    .map(str::to_string)
                    .unwrap_or_else(|| math_markdown(&math.value, false)),
            ));
        }

        let markdown_ast::Node::Paragraph(_) = node else {
            return None;
        };
        let source = cx.node_source(node)?;

        if let Some(math) = block_math_source(source) {
            return Some(math_node(math.to_string(), false, source));
        }

        None
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let math = node.data::<MathNode>().expect("math markdown node data");
        let font_size = f32::from(window.text_style().font_size.to_pixels(window.rem_size()));

        div()
            .w_full()
            .flex()
            .justify_center()
            .py_1()
            .child(render_math_text(
                &math.source,
                math.inline,
                font_size,
                cx.theme().foreground,
            ))
    }
}

impl MarkdownPlugin for InlineMathPlugin {
    fn name(&self) -> &str {
        "inline-math"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        context: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        parse_inline_math(node, context.node_source(node))
    }

    fn render_inline(
        &self,
        node: &MarkdownNode,
        context: &InlineRenderContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<InlineElement> {
        let math = node.data::<MathNode>()?;
        let font_size = f32::from(context.font_size());
        let foreground = context.text_style().color;
        Some(InlineElement::new(render_math_text(
            &math.source,
            math.inline,
            font_size,
            foreground,
        )))
    }
}

fn parse_inline_math(node: &markdown_ast::Node, source: Option<&str>) -> Option<MarkdownNode> {
    let markdown_ast::Node::InlineMath(math) = node else {
        return None;
    };
    Some(math_node(
        math.value.clone(),
        true,
        source
            .map(str::to_string)
            .unwrap_or_else(|| math_markdown(&math.value, true)),
    ))
}

fn prettify_math_source(source: &str) -> String {
    let mut out = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let replacements = [
        (r"\alpha", "\u{03b1}"),
        (r"\beta", "\u{03b2}"),
        (r"\gamma", "\u{03b3}"),
        (r"\delta", "\u{03b4}"),
        (r"\pi", "\u{03c0}"),
        (r"\sum", "\u{2211}"),
        (r"\sqrt", "\u{221a}"),
        (r"\times", "\u{00d7}"),
        (r"\cdot", "\u{22c5}"),
        (r"\leq", "\u{2264}"),
        (r"\geq", "\u{2265}"),
        (r"\neq", "\u{2260}"),
        (r"\infty", "\u{221e}"),
        (r"\left", ""),
        (r"\right", ""),
    ];

    for (from, to) in replacements {
        out = out.replace(from, to);
    }

    compact_math_scripts(&out)
}

fn compact_math_scripts(source: &str) -> String {
    let chars = source.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut ix = 0;

    while ix < chars.len() {
        match chars[ix] {
            '^' | '_' => {
                let superscript = chars[ix] == '^';
                let (script, next_ix) = take_script(&chars, ix + 1);
                if script.is_empty() {
                    out.push(chars[ix]);
                    ix += 1;
                    continue;
                }

                for ch in script.chars() {
                    out.push(script_char(ch, superscript));
                }
                ix = next_ix;
            }
            ch => {
                out.push(ch);
                ix += 1;
            }
        }
    }

    out
}

fn take_script(chars: &[char], start_ix: usize) -> (String, usize) {
    let Some(first) = chars.get(start_ix).copied() else {
        return (String::new(), start_ix);
    };

    if first != '{' {
        return (first.to_string(), start_ix + 1);
    }

    let mut depth = 1;
    let mut ix = start_ix + 1;
    let mut script = String::new();
    while let Some(ch) = chars.get(ix).copied() {
        match ch {
            '{' => {
                depth += 1;
                script.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return (script, ix + 1);
                }
                script.push(ch);
            }
            _ => script.push(ch),
        }
        ix += 1;
    }

    (script, ix)
}

fn script_char(ch: char, superscript: bool) -> char {
    if superscript {
        match ch {
            '0' => '\u{2070}',
            '1' => '\u{00b9}',
            '2' => '\u{00b2}',
            '3' => '\u{00b3}',
            '4' => '\u{2074}',
            '5' => '\u{2075}',
            '6' => '\u{2076}',
            '7' => '\u{2077}',
            '8' => '\u{2078}',
            '9' => '\u{2079}',
            '+' => '\u{207a}',
            '-' => '\u{207b}',
            '=' => '\u{207c}',
            '(' => '\u{207d}',
            ')' => '\u{207e}',
            'i' => '\u{2071}',
            'n' => '\u{207f}',
            _ => ch,
        }
    } else {
        match ch {
            '0' => '\u{2080}',
            '1' => '\u{2081}',
            '2' => '\u{2082}',
            '3' => '\u{2083}',
            '4' => '\u{2084}',
            '5' => '\u{2085}',
            '6' => '\u{2086}',
            '7' => '\u{2087}',
            '8' => '\u{2088}',
            '9' => '\u{2089}',
            '+' => '\u{208a}',
            '-' => '\u{208b}',
            '=' => '\u{208c}',
            '(' => '\u{208d}',
            ')' => '\u{208e}',
            'a' => '\u{2090}',
            'e' => '\u{2091}',
            'h' => '\u{2095}',
            'i' => '\u{1d62}',
            'j' => '\u{2c7c}',
            'k' => '\u{2096}',
            'l' => '\u{2097}',
            'm' => '\u{2098}',
            'n' => '\u{2099}',
            'o' => '\u{2092}',
            'p' => '\u{209a}',
            'r' => '\u{1d63}',
            's' => '\u{209b}',
            't' => '\u{209c}',
            'u' => '\u{1d64}',
            'v' => '\u{1d65}',
            'x' => '\u{2093}',
            _ => ch,
        }
    }
}

// Math renders as prettified formula text: the SVG path shells out to a local
// Node.js + MathJax install the application cannot depend on.
fn render_math_text(source: &str, inline: bool, font_size: f32, color: Hsla) -> AnyElement {
    let font_size = if inline {
        font_size.max(10.0)
    } else {
        (font_size * 1.18).max(12.0)
    };

    div()
        .flex_none()
        .line_height(relative(if inline { 1.0 } else { 1.2 }))
        .text_size(px(font_size))
        .text_color(color)
        .italic()
        .child(prettify_math_source(source))
        .into_any_element()
}

impl MarkdownPlugin for TickerPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "ticker"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Paragraph(paragraph) = node else {
            return None;
        };
        let [markdown_ast::Node::Text(text)] = paragraph.children.as_slice() else {
            return None;
        };
        let symbol = ticker_symbol(&text.value)?;
        Some(
            MarkdownNode::new(
                "ticker",
                TickerNode {
                    symbol: symbol.to_string(),
                },
            )
            .text(format!("${symbol}"))
            .markdown(cx.node_source(node).unwrap_or(text.value.as_str())),
        )
    }

    fn render(&self, node: &MarkdownNode, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ticker = node
            .data::<TickerNode>()
            .expect("ticker markdown node data");
        let symbol = ticker.symbol.as_str();
        let quote = self.quote(symbol);
        let up = quote.change >= 0.0;
        let trend = if up { cx.theme().green } else { cx.theme().red };

        v_flex()
            .w(rems(15.))
            .gap_1p5()
            .px_3()
            .py_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .line_height(relative(1.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("${symbol}")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .line_height(relative(1.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(quote.name),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_0p5()
                            .px_1()
                            .py_0p5()
                            .rounded(cx.theme().radius)
                            .bg(trend.opacity(0.12))
                            .text_xs()
                            .line_height(relative(1.))
                            .text_color(trend)
                            .child(
                                Icon::new(if up {
                                    IconName::ArrowUp
                                } else {
                                    IconName::ArrowDown
                                })
                                .xsmall(),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("{:+.1}%", quote.change)),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .line_height(relative(1.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{:.2}", quote.price)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .line_height(relative(1.))
                            .text_color(cx.theme().muted_foreground)
                            .child("Last"),
                    ),
            )
    }
}

impl MarkdownPlugin for UserCardPlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "user-card"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Html(raw) = node else {
            return None;
        };
        if html_tag_name(&raw.value) != Some("UserCard") {
            return None;
        }
        let id = html_attr(&raw.value, "id")?;
        Some(
            MarkdownNode::new("user-card", UserCardNode { id: id.clone() })
                .text(id)
                .markdown(cx.node_source(node).unwrap_or(raw.value.as_str())),
        )
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let user = node
            .data::<UserCardNode>()
            .expect("user-card markdown node data");
        let id = user.id.as_str();
        let (name, avatar) = match id {
            "huacnlee" => (
                "Jason Lee",
                "https://avatars.githubusercontent.com/u/5518?v=4",
            ),
            "madcodelife" => (
                "Floyd Wang",
                "https://avatars.githubusercontent.com/u/28998859?v=4",
            ),
            _ => ("Unknown", ""),
        };

        let following = window.use_keyed_state(
            SharedString::from(format!("markdown-user-card-follow-{id}")),
            cx,
            |_, _| false,
        );
        let is_following = *following.read(cx);

        h_flex()
            .w(rems(18.75))
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                Avatar::new()
                    .name(name)
                    .with_size(px(24.))
                    .when(!avatar.is_empty(), |this| this.src(avatar)),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(name),
            )
            .child(
                Button::new(SharedString::from(format!("markdown-follow-{id}")))
                    .outline()
                    .small()
                    .label(if is_following { "Following" } else { "Follow" })
                    .on_click(move |_, _, cx| {
                        following.update(cx, |v, cx| {
                            *v = !*v;
                            cx.notify();
                        });
                    }),
            )
    }
}

/// Serialize a table to CSV: `,` separated, quoting only cells that contain
/// `"`, `,` or a newline, with `"` doubled inside quotes.
fn table_to_csv(headers: &[String], rows: &[Vec<String>]) -> String {
    let field = |cell: &str| {
        if cell.contains(['"', ',', '\n']) {
            format!("\"{}\"", cell.replace('"', "\"\""))
        } else {
            cell.to_string()
        }
    };
    let line = |cells: &[String]| cells.iter().map(|c| field(c)).collect::<Vec<_>>().join(",");

    std::iter::once(line(headers))
        .chain(rows.iter().map(|row| line(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Serialize a table to TSV: tab separated, never quoted; tabs and newlines
/// inside a cell become literal `\t` / `\n` so rows stay intact.
fn table_to_tsv(headers: &[String], rows: &[Vec<String>]) -> String {
    let field = |cell: &str| {
        cell.replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    };
    let line = |cells: &[String]| {
        cells
            .iter()
            .map(|c| field(c))
            .collect::<Vec<_>>()
            .join("\t")
    };

    std::iter::once(line(headers))
        .chain(rows.iter().map(|row| line(row)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct MarkdownSection {
    input_state: Entity<EditorState>,
    text_view: Entity<TextViewState>,
    preview_zoom: f32,
    /// When `true`, tables wrap cell content to fit the width; when `false`
    /// (the default), tables keep cells on one line and scroll horizontally.
    table_wrap: bool,
    /// Whether copying a selection yields the rendered text or its Markdown
    /// source.
    selection_format: SelectionFormat,
    find_state: Entity<InputState>,
    /// The preview text the find query was last searched in.
    searched: Option<RenderedText>,
    matches: Vec<Range<usize>>,
    /// The index of the current match in `matches`.
    current_match: usize,
    _subscriptions: Vec<Subscription>,
}

impl MarkdownSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    hard_tabs: false,
                })
                .searchable(true)
                .placeholder("Enter your Markdown here…")
                .default_value(EXAMPLE)
        });

        let text_view = cx.new(|cx| TextViewState::markdown(EXAMPLE, cx));
        let find_state = cx.new(|cx| InputState::new(window, cx).placeholder("Find in preview"));

        let _subscriptions = vec![
            cx.subscribe(&input_state, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&find_state, |this, _, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    this.searched = None;
                    this.current_match = 0;
                    this.highlight_matches(cx);
                }
                InputEvent::PressEnter { shift, .. } => this.go_to_match(!shift, cx),
                _ => {}
            }),
            // Search again whenever the preview's content changes.
            cx.observe(&text_view, |this, _, cx| this.highlight_matches(cx)),
        ];
        Self {
            text_view,
            preview_zoom: 1.0,
            input_state,
            // Default to horizontal scrolling for tables.
            table_wrap: false,
            selection_format: SelectionFormat::Plain,
            find_state,
            searched: None,
            matches: Vec::new(),
            current_match: 0,
            _subscriptions,
        }
    }

    /// Search the preview for the find query, unless the preview text it
    /// was last searched in is still current, and highlight the matches.
    fn highlight_matches(&mut self, cx: &mut Context<Self>) {
        let text = self.text_view.read(cx).rendered_text();
        if self.searched.as_ref() == Some(&text) {
            return;
        }

        let query = self.find_state.read(cx).value();
        self.matches = if query.is_empty() {
            Vec::new()
        } else {
            text.as_str()
                .match_indices(query.as_str())
                .map(|(start, found)| start..start + found.len())
                .collect()
        };
        self.current_match = self.current_match.min(self.matches.len().saturating_sub(1));
        let query_changed = self.searched.is_none();
        self.searched = Some(text);
        // Typing a query scrolls to its first match; content changing
        // under an unchanged query leaves the view where it is.
        self.paint_matches(query_changed, cx);
    }

    /// Step to the next match, or the previous one, and scroll to it.
    fn go_to_match(&mut self, forward: bool, cx: &mut Context<Self>) {
        let count = self.matches.len();
        if count == 0 {
            return;
        }
        self.current_match = if forward {
            (self.current_match + 1) % count
        } else {
            (self.current_match + count - 1) % count
        };
        self.paint_matches(true, cx);
    }

    /// Highlight the matches, the current one stronger, and scroll to it
    /// when `reveal` is set.
    fn paint_matches(&mut self, reveal: bool, cx: &mut Context<Self>) {
        let Some(searched) = self.searched.clone() else {
            return;
        };
        let color = cx.theme().warning.opacity(0.3);
        let current_color = cx.theme().warning;
        let highlights = self.matches.iter().enumerate().map(|(ix, range)| {
            RangeHighlight::new(
                range.clone(),
                if ix == self.current_match {
                    current_color
                } else {
                    color
                },
            )
        });
        let current = self.matches.get(self.current_match).cloned();
        let result = self.text_view.update(cx, |state, cx| {
            // The matches are ranges of the text they were found in.
            if state.rendered_text() != searched {
                return Ok(());
            }
            state.set_range_highlights(highlights, cx)?;
            if reveal && let Some(current) = current {
                state.reveal_range(current, cx)?;
            }
            Ok::<_, RangeHighlightError>(())
        });
        if let Err(error) = result {
            tracing::warn!(
                target: "gpui_starter::gallery",
                error = %error,
                "could not highlight the markdown search matches"
            );
        }
        cx.notify();
    }

    /// Build the markdown style: tables scroll horizontally unless `table_wrap`
    /// is on, in which case the default wrapping layout is used.
    fn text_view_style(&self) -> TextViewStyle {
        if self.table_wrap {
            return TextViewStyle::default();
        }
        let mut table = StyleRefinement::default();
        table.overflow.x = Some(Overflow::Scroll);
        TextViewStyle::default().table(table)
    }

    fn render_preview(&self) -> TextView {
        TextView::new(&self.text_view)
            .plugin(InlineMathPlugin)
            .plugin(MentionPlugin)
            .code_block_actions(|code_block, _window, _cx| {
                let code = code_block.code();
                let lang = code_block.lang();

                h_flex()
                    .gap_1()
                    .child(Clipboard::new("markdown-copy-code").value(code.clone()))
                    .when_some(lang, |this, lang| {
                        // Only show run terminal button for certain languages
                        if lang.as_ref() == "rust" || lang.as_ref() == "python" {
                            this.child(
                                Button::new("markdown-run-terminal")
                                    .icon(IconName::SquareTerminal)
                                    .ghost()
                                    .xsmall()
                                    .tooltip("Run in terminal")
                                    .on_click(move |_, window, cx| {
                                        window.push_notification(
                                            format!(
                                                "No terminal is wired up, so the {lang} block was not run."
                                            ),
                                            cx,
                                        );
                                    }),
                            )
                        } else {
                            this
                        }
                    })
            })
            .table_actions(|table, _window, cx| {
                // The hook hands the table over as plain data; it runs on
                // every render, so only cheap clones belong here.
                let markdown = table.markdown.clone();
                let headers = table.headers.clone();
                let rows = table.rows.clone();
                let shape = format!("{} × {}", table.rows.len(), table.headers.len());

                h_flex()
                    .w_full()
                    .justify_end()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(shape),
                    )
                    .child(
                        Clipboard::new("markdown-copy-table")
                            .value(markdown.clone())
                            .tooltip("Copy as Markdown"),
                    )
                    .child(
                        Button::new("markdown-export-table")
                            .icon(IconName::Ellipsis)
                            .ghost()
                            .xsmall()
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                                // The builder is a `Fn`, so captured values
                                // are cloned on every rebuild.
                                let (csv_headers, csv_rows) = (headers.clone(), rows.clone());
                                let (tsv_headers, tsv_rows) = (headers.clone(), rows.clone());

                                menu.item(
                                    PopupMenuItem::new("Copy as CSV").on_click(
                                        move |_, _, cx| {
                                            let csv =
                                                table_to_csv(&csv_headers, &csv_rows);
                                            cx.write_to_clipboard(ClipboardItem::new_string(csv));
                                        },
                                    ),
                                )
                                .item(
                                    PopupMenuItem::new("Copy as TSV").on_click(
                                        move |_, _, cx| {
                                            let tsv =
                                                table_to_tsv(&tsv_headers, &tsv_rows);
                                            cx.write_to_clipboard(ClipboardItem::new_string(tsv));
                                        },
                                    ),
                                )
                            }),
                    )
            })
            .plugin(TickerPlugin::new(
                TickerQuote {
                    name: "Apple Inc.",
                    price: 300.21,
                    change: 5.2,
                },
                TickerQuote {
                    name: "Tesla, Inc.",
                    price: 412.05,
                    change: -2.13,
                },
            ))
            .plugin(UserCardPlugin)
            .plugin(MathPlugin)
            .on_link_click(|url, event, _window, cx| {
                if !event.is_right_click() {
                    cx.open_url(url);
                }
            })
            // Tables scroll horizontally by default; the status bar toggle
            // switches to wrapping.
            .style(self.text_view_style())
            .text_size(rems(self.preview_zoom))
            .flex_none()
            .px_5()
            .selectable(true)
            .selection_format(self.selection_format)
    }
}

impl Render for MarkdownSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.input_state.read(cx).value();
        self.text_view
            .update(cx, |state, cx| state.set_text(&source, cx));

        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                div().w_full().min_h_0().h(rems(37.5)).child(
                    h_resizable("markdown-container")
                        .child(
                            div()
                                .size_full()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size)
                                .child(
                                    Editor::new(&self.input_state)
                                        .h(relative(1.))
                                        .p_0()
                                        .border_0(),
                                )
                                .into_any(),
                        )
                        // The gallery pane owns scrolling, so the preview
                        // clips instead of scrolling inside its panel.
                        .child(
                            div()
                                .size_full()
                                .overflow_hidden()
                                .child(self.render_preview())
                                .into_any(),
                        ),
                ),
            )
            .child(
                StatusBar::new()
                    .left(
                        h_flex()
                            .gap_2()
                            .child(
                                Input::new(&self.find_state)
                                    .xsmall()
                                    .w(rems(12.5))
                                    .focus_bordered(false),
                            )
                            .when(!self.find_state.read(cx).value().is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(if self.matches.is_empty() {
                                            "No matches".to_string()
                                        } else {
                                            format!(
                                                "{} of {}",
                                                self.current_match + 1,
                                                self.matches.len()
                                            )
                                        }),
                                )
                            })
                            .child(
                                Button::new("markdown-previous-match")
                                    .icon(IconName::ChevronUp)
                                    .ghost()
                                    .xsmall()
                                    .disabled(self.matches.is_empty())
                                    .tooltip("Previous Match")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.go_to_match(false, cx)),
                                    ),
                            )
                            .child(
                                Button::new("markdown-next-match")
                                    .icon(IconName::ChevronDown)
                                    .ghost()
                                    .xsmall()
                                    .disabled(self.matches.is_empty())
                                    .tooltip("Next Match")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.go_to_match(true, cx)),
                                    ),
                            ),
                    )
                    .right(
                        Button::new("markdown-preview-zoom")
                            .ghost()
                            .xsmall()
                            .label(format!("Zoom: {:.0}%", self.preview_zoom * 100.0))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preview_zoom = if this.preview_zoom >= 2.0 {
                                    0.75
                                } else {
                                    this.preview_zoom + 0.25
                                };
                                this.text_view
                                    .update(cx, |view, cx| view.invalidate_inline_layout(cx));
                                cx.notify();
                            })),
                    )
                    .right(
                        Button::new("markdown-selection-format")
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
                    )
                    .right(
                        Button::new("markdown-table-wrap")
                            .ghost()
                            .xsmall()
                            .label(if self.table_wrap {
                                "Table: Wrap"
                            } else {
                                "Table: Scroll"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.table_wrap = !this.table_wrap;
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "markdown",
        "Markdown",
        "Edit Markdown source on the left and read its rendered form on the right: custom plugins (mentions, ticker quotes, user cards, math), code-block and table copy actions, find in the preview, preview zoom, and a table wrap toggle. Syntax colors in the source editor need the crate's tree-sitter features, which this app does not enable.",
        MarkdownSection::view(window, cx),
    ));
}
