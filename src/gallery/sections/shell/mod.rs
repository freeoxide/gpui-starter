//! Shell sections: the native todo-list port and the script-surface punts.

mod demo;
mod js_story;
mod shell_story;
mod todo_list;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    todo_list::register(sections, window, cx);
    shell_story::register(sections, window, cx);
    js_story::register(sections, window, cx);
}
