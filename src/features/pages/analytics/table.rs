use gpui_kit::component::{
    ActiveTheme as _, Size, StyleSized as _, StyledExt as _,
    menu::PopupMenu,
    table::{Column, ColumnFixed, ColumnSort, TableDelegate, TableState},
};
use gpui_kit::{
    App, Context, Div, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, SharedString, Stateful, Styled as _, TextAlign, TextRun, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::model::{ChannelKey, ChannelRow};
use super::{OpenChannelDetails, format};

#[derive(Clone, Copy, PartialEq)]
enum SortKey {
    Page,
    Channel,
    Visitors,
    Sessions,
    Revenue,
    Conversion,
    AvgSession,
}

impl SortKey {
    fn from_col_ix(col_ix: usize) -> Self {
        match col_ix {
            0 => SortKey::Page,
            1 => SortKey::Channel,
            2 => SortKey::Visitors,
            3 => SortKey::Sessions,
            4 => SortKey::Revenue,
            5 => SortKey::Conversion,
            _ => SortKey::AvgSession,
        }
    }
}

pub(crate) struct ChannelTableDelegate {
    rows: Vec<ChannelRow>,
    columns: Vec<Column>,
    sort_key: SortKey,
    sort_order: ColumnSort,
    size: Size,
    loading: bool,
    menu_row: Option<usize>,
}

impl ChannelTableDelegate {
    pub(crate) fn new(rows: Vec<ChannelRow>) -> Self {
        let mut delegate = Self {
            rows,
            columns: vec![
                Column::new("page", crate::i18n::localize("analytics_col_page"))
                    .width(200.)
                    .fixed(ColumnFixed::Left)
                    .sortable(),
                Column::new("channel", crate::i18n::localize("analytics_col_channel"))
                    .width(110.)
                    .sortable(),
                Column::new("visitors", crate::i18n::localize("analytics_col_visitors"))
                    .width(90.)
                    .sortable()
                    .text_right()
                    .p_0(),
                Column::new("sessions", crate::i18n::localize("analytics_col_sessions"))
                    .width(90.)
                    .sortable()
                    .text_right()
                    .p_0(),
                Column::new("revenue", crate::i18n::localize("analytics_col_revenue"))
                    .width(100.)
                    .sortable()
                    .text_right()
                    .p_0()
                    .sort(ColumnSort::Descending),
                Column::new(
                    "conversion",
                    crate::i18n::localize("analytics_col_conversion"),
                )
                .width(80.)
                .sortable()
                .text_right()
                .p_0(),
                Column::new(
                    "avg_session",
                    crate::i18n::localize("analytics_col_avg_session"),
                )
                .width(100.)
                .sortable()
                .text_right()
                .p_0(),
            ],
            sort_key: SortKey::Revenue,
            sort_order: ColumnSort::Descending,
            size: Size::default(),
            loading: false,
            menu_row: None,
        };
        delegate.sort_rows();
        delegate
    }

    pub(crate) fn set_rows(&mut self, rows: Vec<ChannelRow>) {
        self.rows = rows;
        self.loading = false;
        self.sort_rows();
    }

    pub(crate) fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub(crate) fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub(crate) fn set_col_widths(&mut self, widths: &[Pixels]) {
        for (column, width) in self.columns.iter_mut().zip(widths) {
            column.width = *width;
        }
    }

    pub(crate) fn key_at(&self, row_ix: usize) -> Option<ChannelKey> {
        self.rows.get(row_ix).map(ChannelKey::of)
    }

    pub(crate) fn row_ix_for_key(&self, key: &ChannelKey) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| row.page == key.page && row.channel == key.channel)
    }

    pub(crate) fn menu_row(&self) -> Option<usize> {
        self.menu_row
    }

    #[cfg(test)]
    pub(crate) fn row(&self, row_ix: usize) -> Option<&ChannelRow> {
        self.rows.get(row_ix)
    }

    fn set_sort(&mut self, key: SortKey, order: ColumnSort) {
        self.sort_key = key;
        self.sort_order = order;
        self.sort_rows();
    }

    fn sort_rows(&mut self) {
        let descending = matches!(self.sort_order, ColumnSort::Descending);
        let by_number = |rows: &mut Vec<ChannelRow>, pick: fn(&ChannelRow) -> f64| {
            rows.sort_by(|a, b| {
                let cmp = pick(a)
                    .partial_cmp(&pick(b))
                    .unwrap_or(std::cmp::Ordering::Equal);
                if descending { cmp.reverse() } else { cmp }
            });
        };
        match self.sort_key {
            SortKey::Page => self.rows.sort_by(|a, b| {
                let cmp = a.page.cmp(&b.page);
                if descending { cmp.reverse() } else { cmp }
            }),
            SortKey::Channel => self.rows.sort_by(|a, b| {
                let cmp = a.channel.cmp(&b.channel);
                if descending { cmp.reverse() } else { cmp }
            }),
            SortKey::Visitors => by_number(&mut self.rows, |row| row.visitors),
            SortKey::Sessions => by_number(&mut self.rows, |row| row.sessions),
            SortKey::Revenue => by_number(&mut self.rows, |row| row.revenue),
            SortKey::Conversion => by_number(&mut self.rows, |row| row.conversion),
            SortKey::AvgSession => by_number(&mut self.rows, |row| row.avg_session_secs),
        }
    }

    fn value_cell(&self, col: &Column) -> Div {
        div()
            .h_full()
            .h_flex()
            .items_center()
            .when(col.paddings.is_some(), |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Right, |this| this.justify_end())
    }

    fn cell_text(row: &ChannelRow, col: &Column) -> String {
        match col.key.as_ref() {
            "page" => row.page.to_string(),
            "channel" => row.channel.to_string(),
            "visitors" => format::compact(row.visitors),
            "sessions" => format::compact(row.sessions),
            "revenue" => format::money(row.revenue),
            "conversion" => format::percent(row.conversion, 1),
            _ => {
                let secs = row.avg_session_secs as usize;
                format!("{}m {}s", secs / 60, secs % 60)
            }
        }
    }
}

impl TableDelegate for ChannelTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let col = self.column(col_ix, cx);
        div()
            .when(col.paddings.is_some(), |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().w_full().justify_end()
            })
            .child(col.name.clone())
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        let id = self
            .rows
            .get(row_ix)
            .map(|row| format!("{}:{}", row.page, row.channel))
            .unwrap_or_else(|| row_ix.to_string());
        div().id(ElementId::Name(id.into()))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(row) = self.rows.get(row_ix) else {
            return div().into_any_element();
        };
        let Some(col) = self.columns.get(col_ix) else {
            return div().into_any_element();
        };

        let text = Self::cell_text(row, col);
        match col.key.as_ref() {
            "page" => self
                .value_cell(col)
                .child(div().truncate().child(text))
                .into_any_element(),
            "channel" => self
                .value_cell(col)
                .text_color(cx.theme().muted_foreground)
                .child(text)
                .into_any_element(),
            _ => self.value_cell(col).child(text).into_any_element(),
        }
    }

    fn fit_column(
        &mut self,
        col_ix: usize,
        window: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> Option<Pixels> {
        let col = self.columns.get(col_ix)?.clone();
        let rem = window.rem_size();
        let font_size = match self.size {
            Size::XSmall => rem * 0.75,
            Size::Large | Size::Size(_) => rem,
            _ => rem * 0.875,
        };
        let font = window.text_style().font();
        let text_width = |text: &str| -> Pixels {
            window
                .text_system()
                .shape_line(
                    SharedString::from(text),
                    font_size,
                    &[TextRun {
                        len: text.len(),
                        font: font.clone(),
                        color: Hsla::default(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width
        };

        let cells = self
            .rows
            .iter()
            .map(|row| text_width(&Self::cell_text(row, &col)))
            .fold(px(0.), Pixels::max);
        let sort_icon = rem * 0.75 + px(4.);
        let header = text_width(col.name.as_ref()) + sort_icon;
        let mut width = cells.max(header);
        // The numeric columns are p_0 but draw the default cell padding inside
        // value_cell, so the table's zero-padding fit math needs a top-up.
        if col.paddings.is_some() {
            let padding = self.size.table_cell_padding();
            width += padding.left + padding.right;
        }
        Some(width)
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.set_sort(SortKey::from_col_ix(col_ix), sort);
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        self.menu_row = Some(row_ix);
        menu.menu(
            crate::i18n::localize("analytics_open_details"),
            Box::new(OpenChannelDetails),
        )
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }
}

#[cfg(test)]
#[path = "table.test.rs"]
mod table_test;
