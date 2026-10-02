//! Overlay-family sections, ported from the upstream stories: Alert,
//! AlertDialog, Dialog, HoverCard, Notification, Popover, Sheet, and Tooltip
//! (the last two also carry the `tooltip_top_edge` and `dialog_overlay`
//! examples).

mod alert;
mod alert_dialog;
mod demo;
mod dialog;
mod hover_card;
mod notification;
mod popover;
mod sheet;
mod tooltip;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    popover::init(cx);
    tooltip::init(cx);
    alert::register(sections, window, cx);
    alert_dialog::register(sections, window, cx);
    dialog::register(sections, window, cx);
    hover_card::register(sections, window, cx);
    notification::register(sections, window, cx);
    popover::register(sections, window, cx);
    sheet::register(sections, window, cx);
    tooltip::register(sections, window, cx);
}
