//! Tree section, ported from the upstream `TreeStory`: a file tree with
//! expand/collapse, keyboard navigation, and a context menu.

use gpui_kit::component::{
    ActiveTheme as _, IconName, WindowExt as _,
    button::Button,
    h_flex,
    label::Label,
    list::ListItem,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{rng_pick, section};

actions!(gallery_tables, [Rename, OpenFile, Delete]);

const CONTEXT: &str = "gallery-tree";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("enter", Rename, Some(CONTEXT))]);
}

// The gallery module's own layout, standing in for the upstream story's walk
// of the repository working directory from disk.
fn file_items() -> Vec<TreeItem> {
    vec![
        TreeItem::new("gallery", "gallery")
            .expanded(true)
            .children([
                TreeItem::new("gallery/mod.rs", "mod.rs"),
                TreeItem::new("gallery/page.rs", "page.rs"),
                TreeItem::new("gallery/registry.rs", "registry.rs"),
                TreeItem::new("gallery/sections", "sections")
                    .expanded(true)
                    .children([
                        TreeItem::new("gallery/sections/tables", "tables")
                            .expanded(true)
                            .children([
                                TreeItem::new("gallery/sections/tables/mod.rs", "mod.rs"),
                                TreeItem::new("gallery/sections/tables/demo.rs", "demo.rs"),
                                TreeItem::new(
                                    "gallery/sections/tables/data_table.rs",
                                    "data_table.rs",
                                ),
                                TreeItem::new("gallery/sections/tables/list.rs", "list.rs"),
                                TreeItem::new("gallery/sections/tables/table.rs", "table.rs"),
                                TreeItem::new("gallery/sections/tables/tree.rs", "tree.rs"),
                                TreeItem::new(
                                    "gallery/sections/tables/virtual_list.rs",
                                    "virtual_list.rs",
                                ),
                            ]),
                        TreeItem::new("gallery/sections/buttons", "buttons").children([
                            TreeItem::new("gallery/sections/buttons/button.rs", "button.rs"),
                            TreeItem::new("gallery/sections/buttons/tag.rs", "tag.rs"),
                        ]),
                        TreeItem::new("gallery/sections/overlays", "overlays").children([
                            TreeItem::new("gallery/sections/overlays/dialog.rs", "dialog.rs"),
                            TreeItem::new("gallery/sections/overlays/tooltip.rs", "tooltip.rs"),
                        ]),
                        TreeItem::new("gallery/sections/welcome", "welcome")
                            .children([TreeItem::new("gallery/sections/welcome/mod.rs", "mod.rs")]),
                    ]),
            ]),
    ]
}

pub struct TreeSection {
    tree_state: Entity<TreeState>,
    items: Vec<TreeItem>,
}

impl TreeSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let items = file_items();
            let tree_state = cx.new(|cx| TreeState::new(cx).items(items.clone()));

            Self { tree_state, items }
        })
    }

    fn on_action_rename(&mut self, _: &Rename, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            window.push_notification(format!("Renaming item: {} ({})", item.label, item.id), cx);
        }
    }

    fn on_action_open(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            window.push_notification(format!("Opening item: {} ({})", item.label, item.id), cx);
        }
    }

    fn on_action_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.tree_state.read(cx).selected_entry() {
            let item = entry.item();
            window.push_notification(format!("Deleting item: {} ({})", item.label, item.id), cx);
        }
    }
}

impl Render for TreeSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .id("tree-section")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_rename))
            .on_action(cx.listener(Self::on_action_open))
            .on_action(cx.listener(Self::on_action_delete))
            .child(
                h_flex().gap_3().child(
                    Button::new("tree-select-item")
                        .outline()
                        .label("Select Item")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(random_item) = rng_pick(&this.items) {
                                this.tree_state.update(cx, |state, cx| {
                                    state.set_selected_item(Some(random_item), cx);
                                });
                            }
                        })),
                ),
            )
            .child(
                section("tree-files", "File tree")
                    .sub_title("Press `enter` to rename. Right-click for context menu.")
                    .w(rems(30.))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                tree(
                                    &self.tree_state,
                                    move |_ix, entry, _selected, _window, cx| {
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
                                            .px_3()
                                            .pl(px(16.) * entry.depth() + px(12.))
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(icon)
                                                    .child(item.label.clone()),
                                            )
                                    },
                                )
                                .context_menu(|_ix, entry, menu, _window, _cx| {
                                    let is_folder = entry.is_folder();
                                    menu.when(!is_folder, |m| m.menu("Open", Box::new(OpenFile)))
                                        .menu("Rename", Box::new(Rename))
                                        .separator()
                                        .menu("Delete", Box::new(Delete))
                                })
                                .p_1()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius)
                                .h(rems(33.75)),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .gap_3()
                                    .children(
                                        self.tree_state
                                            .read(cx)
                                            .selected_index()
                                            .map(|ix| format!("Selected Index: {}", ix)),
                                    )
                                    .children(self.tree_state.read(cx).selected_item().map(
                                        |item| Label::new("Selected:").secondary(item.id.clone()),
                                    )),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "tree",
        "Tree",
        "A hierarchical tree with expand/collapse, keyboard navigation, and a context menu.",
        TreeSection::view(window, cx),
    ));
}
