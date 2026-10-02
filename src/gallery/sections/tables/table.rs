//! Table section, ported from the upstream `TableStory`: the declarative,
//! stateless table with headers, footers, captions, column spans, and striped
//! rows.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, StyledExt as _,
    table::{
        Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow,
    },
    tag::Tag,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct TableSection {
    size: Size,
}

impl TableSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            size: Size::default(),
        })
    }
}

fn status_tag(status: &str) -> Tag {
    match status {
        "Paid" => Tag::success().outline().child(status.to_string()),
        "Pending" => Tag::warning().outline().child(status.to_string()),
        "Unpaid" => Tag::danger().outline().child(status.to_string()),
        _ => Tag::new().child(status.to_string()),
    }
    .xsmall()
}

impl Render for TableSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let invoices: Vec<(&str, &str, &str, &str, &str)> = vec![
            ("INV001", "Paid", "Credit Card", "$250.00", "2024-01-15"),
            ("INV002", "Pending", "PayPal", "$150.00", "2024-02-01"),
            ("INV003", "Unpaid", "Bank Transfer", "$350.00", "2024-02-15"),
            (
                "INV004",
                "Paid",
                "Credit Card\nMaster Card / Visa",
                "$450.00",
                "2024-03-01",
            ),
            ("INV005", "Paid", "PayPal", "$550.00", "2024-03-15"),
            (
                "INV006",
                "Pending",
                "Bank Transfer",
                "$200.00",
                "2024-04-01",
            ),
            ("INV007", "Unpaid", "Credit Card", "$300.00", "2024-04-15"),
        ];

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("table-size", self.size).into_any_element(),
            ]))
            .child(
                section("table-default", "Default")
                    .description(
                        "Headers, body rows, a footer total, and a caption in one element.",
                    )
                    .w_full()
                    .v_flex()
                    .child(
                        Table::new()
                            .with_size(self.size)
                            .child(
                                TableHeader::new().child(
                                    TableRow::new()
                                        .child(TableHead::new().w(rems(9.375)).child("Invoice"))
                                        .child(TableHead::new().col_span(2).child("Status"))
                                        .child(TableHead::new().text_right().child("Amount"))
                                        .child(TableHead::new().text_right().child("Date")),
                                ),
                            )
                            .child(TableBody::new().children(invoices.iter().map(
                                |(invoice, status, method, amount, date)| {
                                    TableRow::new()
                                        .child(
                                            TableCell::new()
                                                .w(rems(9.375))
                                                .child(invoice.to_string()),
                                        )
                                        .child(TableCell::new().child(status_tag(status)))
                                        .child(TableCell::new().child(method.to_string()))
                                        .child(
                                            TableCell::new().text_right().child(amount.to_string()),
                                        )
                                        .child(
                                            TableCell::new().text_right().child(date.to_string()),
                                        )
                                },
                            )))
                            .child(
                                TableFooter::new().child(
                                    TableRow::new()
                                        .child(TableCell::new().col_span(3).child("Total"))
                                        .child(
                                            TableCell::new()
                                                .col_span(2)
                                                .text_right()
                                                .child("$2,250.00"),
                                        ),
                                ),
                            )
                            .child(TableCaption::new().child("A list of your recent invoices.")),
                    ),
            )
            .child(
                section("table-bordered", "Bordered")
                    .description("A border and rounded corners, with every other row tinted.")
                    .w_full()
                    .v_flex()
                    .child(
                        Table::new()
                            .with_size(self.size)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                TableHeader::new().child(
                                    TableRow::new()
                                        .child(TableHead::new().w(rems(6.25)).child("Invoice"))
                                        .child(TableHead::new().child("Method"))
                                        .child(TableHead::new().text_right().child("Amount"))
                                        .child(TableHead::new().text_right().child("Date")),
                                ),
                            )
                            .child(TableBody::new().children(
                                invoices.iter().enumerate().take(6).map(
                                    |(ix, (invoice, _, method, amount, date))| {
                                        TableRow::new()
                                            .when(ix % 2 != 0, |this| {
                                                this.bg(cx.theme().table_even)
                                            })
                                            .child(
                                                TableCell::new()
                                                    .w(rems(6.25))
                                                    .child(invoice.to_string()),
                                            )
                                            .child(TableCell::new().child(method.to_string()))
                                            .child(
                                                TableCell::new()
                                                    .text_right()
                                                    .child(amount.to_string()),
                                            )
                                            .child(
                                                TableCell::new()
                                                    .text_right()
                                                    .child(date.to_string()),
                                            )
                                    },
                                ),
                            )),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "table",
        "Table",
        "A basic table component for directly rendering tabular data.",
        TableSection::view(window, cx),
    ));
}
