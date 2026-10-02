//! Markdown Table section, ported from the upstream `markdown_table`
//! example: one report cycled through the three table layouts.

use gpui_kit::component::{
    button::Button,
    text::{TextView, TextViewStyle},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

const SOURCE: &str = include_str!("fixtures/report.md");

#[derive(Clone, Copy, PartialEq)]
enum TableMode {
    Wrap,
    Adaptive,
    Nowrap,
}

impl TableMode {
    fn next(self) -> Self {
        match self {
            Self::Wrap => Self::Adaptive,
            Self::Adaptive => Self::Nowrap,
            Self::Nowrap => Self::Wrap,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Wrap => "Table: wrap",
            Self::Adaptive => "Table: scroll (adaptive)",
            Self::Nowrap => "Table: scroll (nowrap)",
        }
    }

    /// The three layouts: wrap fits cells to the frame, adaptive scrolls once
    /// columns reach their floor, nowrap keeps every cell on one line.
    fn style(self) -> TextViewStyle {
        if self == Self::Wrap {
            return TextViewStyle::default();
        }

        let mut table = StyleRefinement::default();
        table.overflow.x = Some(Overflow::Scroll);
        let style = TextViewStyle::default().table(table);

        if self == Self::Nowrap {
            let mut cell = StyleRefinement::default();
            cell.text.white_space = Some(WhiteSpace::Nowrap);
            style.table_cell(cell)
        } else {
            style
        }
    }
}

pub struct TableSection {
    mode: TableMode,
}

impl TableSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            mode: TableMode::Adaptive,
        })
    }
}

impl Render for TableSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                Button::new("markdown-table-toggle")
                    .label(self.mode.label())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.mode = this.mode.next();
                        cx.notify();
                    })),
            )
            // The gallery pane owns scrolling, so the document flows at its
            // full height and wide tables scroll inside their own frame.
            .child(
                TextView::markdown("markdown-table-report", SOURCE)
                    .style(self.mode.style())
                    .p_4()
                    .selectable(true),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "markdown-table",
        "Markdown Table",
        "A long report whose tables cycle between three layouts: wrapped cells, content-width columns that scroll once the frame narrows past their floor, and single-line cells that always scroll.",
        TableSection::view(window, cx),
    ));
}
