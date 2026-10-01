//! Table-family sections, ported from the upstream stories: Table,
//! DataTable (with the table-in-scrollable example folded in), List, Tree,
//! and VirtualList.

mod data_table;
mod demo;
mod list;
mod table;
mod tree;
mod virtual_list;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    tree::init(cx);
    table::register(sections, window, cx);
    data_table::register(sections, window, cx);
    list::register(sections, window, cx);
    tree::register(sections, window, cx);
    virtual_list::register(sections, window, cx);
}
