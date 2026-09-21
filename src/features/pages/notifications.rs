use gpui::{prelude::*, *};
use gpui_component::button::Button;
use gpui_component::{h_flex, v_flex};

use crate::accessibility::A11yExt as _;
use crate::notifications::inbox::{self, NotificationInboxItem};

pub struct NotificationsPage {
    _subscriptions: Vec<Subscription>,
}

impl NotificationsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions =
            vec![
                cx.observe_global_in::<inbox::NotificationInboxState>(window, |_, _, cx| {
                    cx.notify();
                }),
            ];
        Self {
            _subscriptions: subscriptions,
        }
    }
}

impl Render for NotificationsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = inbox::snapshot(cx);
        let unread = items.iter().filter(|item| !item.read).count();
        let total = items.len();
        let title = format!("Notifications ({unread} unread)");

        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .id("notifications-title")
                            .a11y(Role::Heading, title.clone())
                            // The count in the title is how a new notification
                            // is announced while this page is open.
                            .a11y_live(accesskit::Live::Polite)
                            .aria_level(1)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(title),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("notifications-mark-read")
                                    .outline()
                                    .label("Mark all read")
                                    .on_click(|_, _, cx| {
                                        inbox::mark_all_read(cx);
                                    }),
                            )
                            .child(
                                Button::new("notifications-clear-all")
                                    .outline()
                                    .label("Clear all")
                                    .on_click(|_, _, cx| {
                                        inbox::clear_all(cx);
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .id("notifications-list")
                    .a11y(Role::List, "Notifications")
                    .aria_orientation(Orientation::Vertical)
                    // AT-SPI derives each item's setsize from the nearest
                    // ancestor that declares one; item-level values are ignored.
                    .aria_size_of_set(total)
                    .children(
                        items
                            .into_iter()
                            .enumerate()
                            .map(move |(ix, item)| render_item(ix, total, item)),
                    ),
            )
    }
}

fn render_item(index: usize, total: usize, item: NotificationInboxItem) -> Stateful<Div> {
    let timestamp = item.created_at.to_rfc3339();
    let summary = item.summary_line();
    let title = item.title;
    let body = item.body;
    let error_summary = item.error_summary;

    let mut label = format!("{title}. {body}. {summary}");
    if let Some(error) = &error_summary {
        label.push_str(". error: ");
        label.push_str(error);
    }

    v_flex()
        .id(ElementId::Name(SharedString::from(format!(
            "inbox-item-{}",
            item.id
        ))))
        .a11y(Role::ListItem, label)
        // accesskit stores position_in_set 0-based; AT bridges report the
        // stored value +1.
        .aria_position_in_set(index)
        .aria_size_of_set(total)
        .gap_1()
        .p_3()
        .border_1()
        .rounded_lg()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(div().font_weight(FontWeight::BOLD).child(title))
                .child(div().text_xs().child(timestamp)),
        )
        .child(div().text_sm().child(body))
        .child(div().text_xs().child(summary))
        .when_some(error_summary, |this, error| {
            this.child(div().text_xs().child(format!("error: {error}")))
        })
}
