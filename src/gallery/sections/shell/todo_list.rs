//! Todo list section, a native port of the upstream `examples/js_todolist`
//! JavaScript application rebuilt on kit components.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Selectable as _, StyledExt as _,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    dialog::{DialogAction, DialogClose, DialogFooter},
    h_flex,
    input::{Input, InputEvent, InputState},
    separator::Separator,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// One todo row. `id` is the stable domain identity, never a position.
struct Todo {
    id: u32,
    caption: SharedString,
    done: bool,
}

/// Which rows the list shows; upstream keeps this filter through every mutation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Active,
    Done,
}

impl Filter {
    fn id(self) -> &'static str {
        match self {
            Self::All => "todo-filter-all",
            Self::Active => "todo-filter-active",
            Self::Done => "todo-filter-done",
        }
    }

    fn caption(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Active => "Active",
            Self::Done => "Done",
        }
    }
}

const FILTERS: [Filter; 3] = [Filter::All, Filter::Active, Filter::Done];

pub struct TodoSection {
    draft: Entity<InputState>,
    items: Vec<Todo>,
    filter: Filter,
    next_id: u32,
    _subscription: Subscription,
}

impl TodoSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let draft = cx.new(|cx| InputState::new(window, cx).placeholder("What needs doing?"));
        // Enter is the primary path for adding a row; the Add button serves the pointer.
        let subscription = cx.subscribe_in(&draft, window, |this, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.add(window, cx);
            }
        });
        Self {
            draft,
            items: Vec::new(),
            filter: Filter::All,
            next_id: 1,
            _subscription: subscription,
        }
    }

    fn remaining(&self) -> usize {
        self.items.iter().filter(|todo| !todo.done).count()
    }

    fn completed(&self) -> usize {
        self.items.iter().filter(|todo| todo.done).count()
    }

    fn visible(&self) -> impl Iterator<Item = &Todo> {
        self.items.iter().filter(|todo| match self.filter {
            Filter::All => true,
            Filter::Active => !todo.done,
            Filter::Done => todo.done,
        })
    }

    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let caption = self.draft.read(cx).value().trim().to_string();
        if caption.is_empty() {
            return;
        }
        self.items.push(Todo {
            id: self.next_id,
            caption: caption.into(),
            done: false,
        });
        self.next_id += 1;
        self.draft
            .update(cx, |draft, cx| draft.set_value("", window, cx));
        cx.notify();
    }

    fn toggle(&mut self, id: u32, done: bool, cx: &mut Context<Self>) {
        if let Some(todo) = self.items.iter_mut().find(|todo| todo.id == id) {
            todo.done = done;
        }
        cx.notify();
    }

    fn remove(&mut self, id: u32, cx: &mut Context<Self>) {
        self.items.retain(|todo| todo.id != id);
        cx.notify();
    }

    fn clear_completed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.completed();
        if count == 0 {
            return;
        }
        let noun = if count == 1 { "item" } else { "items" };
        let section = cx.entity();
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .w(px(360.))
                .title(format!("Delete {count} completed {noun}?"))
                .child("This cannot be undone.")
                .on_ok({
                    let section = section.clone();
                    move |_, window, cx| {
                        section.update(cx, |section, cx| {
                            section.items.retain(|todo| !todo.done);
                            cx.notify();
                        });
                        window.push_notification(format!("Deleted {count} {noun}"), cx);
                        true
                    }
                })
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new().child(
                                Button::new("todo-confirm-cancel").label("Cancel").outline(),
                            ),
                        )
                        .child(
                            DialogAction::new()
                                .child(Button::new("todo-confirm-delete").danger().label("Delete")),
                        ),
                )
        });
    }

    fn empty_copy(&self) -> (&'static str, &'static str) {
        if self.items.is_empty() {
            return ("No items yet", "Type above and press Add.");
        }
        if self.filter == Filter::Done {
            return ("Nothing finished yet", "Tick an item to see it here.");
        }
        ("All done", "Switch to All to review what you finished.")
    }

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        let status = if self.items.is_empty() {
            "Nothing yet".to_string()
        } else {
            format!("{} of {} remaining", self.remaining(), self.items.len())
        };
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::GalleryVerticalEnd).size_4())
                    .child(div().text_sm().font_semibold().child("Todo")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(status),
            )
    }

    fn composer(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .gap_3()
            .child(Input::new(&self.draft).flex_1())
            .child(
                Button::new("todo-add")
                    .primary()
                    .label("Add")
                    .icon(IconName::Plus)
                    .on_click(cx.listener(|this, _, window, cx| this.add(window, cx))),
            )
    }

    fn toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .px_3()
            .py_2()
            .child(h_flex().gap_1().children(FILTERS.map(|filter| {
                Button::new(filter.id())
                    .ghost()
                    .selected(self.filter == filter)
                    .label(filter.caption())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.filter = filter;
                        cx.notify();
                    }))
            })))
            .child(
                Button::new("todo-clear")
                    .ghost()
                    .label("Clear completed…")
                    .disabled(self.completed() == 0)
                    .on_click(cx.listener(|this, _, window, cx| this.clear_completed(window, cx))),
            )
    }

    fn row(&self, todo: &Todo, cx: &Context<Self>) -> impl IntoElement {
        let id = todo.id;
        Checkbox::new(("todo-item", todo.id))
            .checked(todo.done)
            .on_change(cx.listener(move |this, checked, _, cx| {
                this.toggle(id, *checked, cx);
            }))
            .w_full()
            .px_3()
            .py_2()
            .hover(|style| style.bg(cx.theme().muted))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .when(todo.done, |caption| {
                                caption
                                    .text_color(cx.theme().muted_foreground)
                                    .line_through()
                            })
                            .child(todo.caption.clone()),
                    )
                    .child(
                        Button::new(("todo-remove", todo.id))
                            .ghost()
                            .icon(IconName::Delete)
                            .tooltip(format!("Remove \u{201c}{}\u{201d}", todo.caption))
                            .on_click(cx.listener(move |this, _, _, cx| this.remove(id, cx))),
                    ),
            )
    }

    fn rows(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .py_1()
            .children(self.visible().map(|todo| self.row(todo, cx)))
    }

    fn empty_state(&self, cx: &Context<Self>) -> impl IntoElement {
        let (heading, hint) = self.empty_copy();
        v_flex()
            .w_full()
            .items_center()
            .gap_1()
            .py_8()
            .child(div().text_sm().child(heading))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint),
            )
    }

    fn footer(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Not saved: the gallery grants no storage, so the list lasts for this \
                         run only",
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{} completed", self.completed())),
            )
    }
}

impl Render for TodoSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.visible().next().is_none();
        v_flex().gap_4().w_full().items_center().p_4().child(
            section("todo-app", "Todo list")
                .description(
                    "Rebuilt from the JavaScript example on kit components; the footer \
                         reports the storage this host grants, which is none.",
                )
                .w(rems(30.))
                .v_flex()
                .items_start()
                .gap_4()
                .child(self.header(cx))
                .child(self.composer(cx))
                .child(
                    v_flex()
                        .w_full()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().border)
                        .overflow_hidden()
                        .child(self.toolbar(cx))
                        .child(Separator::horizontal())
                        .child(if empty {
                            self.empty_state(cx).into_any_element()
                        } else {
                            self.rows(cx).into_any_element()
                        }),
                )
                .child(self.footer(cx)),
        )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "todo-list",
        "Todo List",
        "A working todo list: type a task and press Enter, tick rows done, filter by \
         state, and clear the finished rows through a confirm dialog.",
        TodoSection::view(window, cx),
    ));
}
