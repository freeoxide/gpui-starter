//! Popover section, ported from the upstream `PopoverStory`.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    popover::Popover,
    separator::Separator,
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::section;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_overlays_popover, no_json)]
struct Info(usize);

actions!(
    gallery_overlays_popover,
    [Copy, Paste, Cut, SearchAll, ToggleCheck]
);

const CONTEXT: &str = "gallery-popover";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, Some(CONTEXT)),
    ]);
}

struct PopoverForm {
    parent: WeakEntity<PopoverSection>,
    input1: Entity<InputState>,
}

impl PopoverForm {
    fn new(parent: WeakEntity<PopoverSection>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            parent,
            input1: cx.new(|cx| InputState::new(window, cx)),
        })
    }
}

impl Focusable for PopoverForm {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input1.focus_handle(cx)
    }
}

impl Render for PopoverForm {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let parent = self.parent.clone();
        v_flex()
            .gap_2()
            .p_3()
            .size_full()
            .child("This is a form container.")
            .child("Click submit to dismiss the popover.")
            .child(Input::new(&self.input1))
            .child(
                Button::new("popover-form-submit")
                    .label("Submit")
                    .primary()
                    .on_click(move |_, _, cx| {
                        let _ = parent.update(cx, |this, cx| {
                            this.form_popover_open = false;
                            cx.notify();
                        });
                    }),
            )
    }
}

struct DropdownListDelegate {
    parent: WeakEntity<PopoverSection>,
}

impl ListDelegate for DropdownListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        10
    }

    fn render_item(
        &mut self,
        ix: gpui_kit::component::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        Some(ListItem::new(ix).child(format!("Item {}", ix.row)))
    }

    fn set_selected_index(
        &mut self,
        _: Option<gpui_kit::component::IndexPath>,
        _: &mut Window,
        _: &mut Context<gpui_kit::component::list::ListState<Self>>,
    ) {
    }

    fn confirm(&mut self, _: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }

    fn cancel(&mut self, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.parent.update(cx, |this, cx| {
            this.list_popover_open = false;
            cx.notify();
        });
    }
}

pub struct PopoverSection {
    form: Entity<PopoverForm>,
    list: Entity<ListState<DropdownListDelegate>>,
    form_popover_open: bool,
    list_popover_open: bool,
    arrow: bool,
    checked: bool,
    message: String,
}

impl PopoverSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let form = PopoverForm::new(cx.weak_entity(), window, cx);
            let parent = cx.weak_entity();
            let list = cx.new(|cx| {
                ListState::new(DropdownListDelegate { parent }, window, cx).searchable(true)
            });

            Self {
                form,
                list,
                checked: true,
                form_popover_open: false,
                list_popover_open: false,
                arrow: false,
                message: String::new(),
            }
        })
    }
}

impl Render for PopoverSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, _: &Copy, _, cx| {
                this.message = "You have clicked copy".to_string();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Cut, _, cx| {
                this.message = "You have clicked cut".to_string();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Paste, _, cx| {
                this.message = "You have clicked paste".to_string();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SearchAll, _, cx| {
                this.message = "You have clicked search all".to_string();
                cx.notify();
            }))
            .on_action(cx.listener(|this, info: &Info, _, cx| {
                this.message = format!("You have clicked info: {}", info.0);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleCheck, _, cx| {
                this.checked = !this.checked;
                this.message = format!("You have clicked toggle check: {}", this.checked);
                cx.notify();
            }))
            .child(
                section("popover-default", "Default")
                    .description("Display lightweight contextual content.")
                    .child(
                        Popover::new("popover-basic")
                            .max_w(rems(37.5))
                            .trigger(
                                Button::new("popover-basic-trigger")
                                    .outline()
                                    .label("Popover"),
                            )
                            .gap_2()
                            .text_sm()
                            .w(rems(25.))
                            .child("Hello, this is a Popover.")
                            .child(Separator::horizontal())
                            .child(
                                "You can put any content here, including text, \
                                buttons, forms, and more.",
                            ),
                    )
                    .child(
                        Popover::new("popover-default-open")
                            .default_open(true)
                            .trigger(
                                Button::new("popover-default-open-trigger")
                                    .label("Default Open")
                                    .outline(),
                            )
                            .child("This popover is open by default when first rendered."),
                    ),
            )
            .child(
                section("popover-form", "Form")
                    .description("Keep focus and controlled open state around a form.")
                    .child(
                        Popover::new("popover-form-popover")
                            .p_0()
                            .text_sm()
                            .trigger(
                                Button::new("popover-form-trigger")
                                    .outline()
                                    .label("Popup Form"),
                            )
                            .track_focus(&self.form.focus_handle(cx))
                            .open(self.form_popover_open)
                            .on_open_change(cx.listener(|this, open, _, cx| {
                                this.form_popover_open = *open;
                                cx.notify();
                            }))
                            .child(self.form.clone()),
                    ),
            )
            .child(
                section("popover-list", "List")
                    .description("Place a scrollable selection list in the popover.")
                    .child(
                        Popover::new("popover-list-popover")
                            .p_0()
                            .text_sm()
                            .open(self.list_popover_open)
                            .on_open_change(cx.listener(|this, open, _, cx| {
                                this.list_popover_open = *open;
                                cx.notify();
                            }))
                            .trigger(
                                Button::new("popover-list-trigger")
                                    .outline()
                                    .label("Popup List"),
                            )
                            .track_focus(&self.list.focus_handle(cx))
                            .child(List::new(&self.list))
                            .w_64()
                            .h(rems(12.5)),
                    ),
            )
            .child(
                section("popover-right-click", "Right click")
                    .description("Open from the secondary mouse button.")
                    .child(
                        Popover::new("popover-right-click-popover")
                            .mouse_button(MouseButton::Right)
                            .trigger(
                                Button::new("popover-right-click-trigger")
                                    .outline()
                                    .label("Right Click Popover"),
                            )
                            .max_w(rems(37.5))
                            .content(|_, _, cx| {
                                v_flex()
                                    .gap_2()
                                    .child("Hello, this is a Popover on the Bottom Right.")
                                    .child(Separator::horizontal())
                                    .child(
                                        Button::new("popover-right-click-dismiss")
                                            .primary()
                                            .label("Dismiss")
                                            .w_20()
                                            .on_click(cx.listener(|_, _, window, cx| {
                                                window.push_notification(
                                                    "You have clicked dismiss via DismissEvent.",
                                                    cx,
                                                );
                                                cx.emit(DismissEvent);
                                            })),
                                    )
                            }),
                    ),
            )
            .child(
                section("popover-custom-style", "Custom style")
                    .description("Customize appearance, radius, and shadow.")
                    .child(
                        Popover::new("popover-styled")
                            .trigger(
                                Button::new("popover-styled-trigger")
                                    .outline()
                                    .label("Style Popover"),
                            )
                            .appearance(false)
                            .py_1()
                            .px_2()
                            .bg(cx.theme().primary)
                            .text_color(cx.theme().primary_foreground)
                            .max_w(rems(37.5))
                            .rounded(cx.theme().radius)
                            .text_sm()
                            .shadow_2xl()
                            .child("A styled Popover with custom background and text color."),
                    ),
            )
            .child(
                section("popover-async-submenu", "Async submenu")
                    .description("Rebuild submenu content after asynchronous loading.")
                    .child(
                        Button::new("popover-async-menu")
                            .outline()
                            .label("Async Menu")
                            .dropdown_menu(|menu, window, cx| {
                                // The submenu is attached as a plain menu value; its
                                // content is loaded asynchronously via `rebuild`.
                                let submenu = PopupMenu::build(window, cx, |menu, _, _| {
                                    menu.label("Loading...")
                                });

                                cx.spawn_in(window, {
                                    let submenu = submenu.clone();
                                    async move |_, cx| {
                                        cx.background_executor()
                                            .timer(Duration::from_secs(1))
                                            .await;
                                        _ = submenu.update_in(cx, |menu, window, cx| {
                                            menu.rebuild(window, cx, |menu, _, _| {
                                                (1..=3).fold(menu, |menu, ix| {
                                                    menu.menu(
                                                        format!("Loaded Item {ix}"),
                                                        Box::new(Info(ix)),
                                                    )
                                                })
                                            });
                                        });
                                    }
                                })
                                .detach();

                                menu.menu("Copy", Box::new(Copy))
                                    .separator()
                                    .item(PopupMenuItem::submenu("Async Submenu", submenu))
                            }),
                    )
                    .child(self.message.clone()),
            )
            .child(
                section("popover-anchor", "Anchor")
                    .description("Position content from each edge of the trigger.")
                    .sub_title(
                        Checkbox::new("popover-anchor-arrow")
                            .label("Arrow")
                            .checked(self.arrow)
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.arrow = *checked;
                                cx.notify();
                            })),
                    )
                    .w_full()
                    .min_h(rems(14.))
                    .v_flex()
                    .justify_between()
                    .children(
                        [
                            vec![Anchor::TopLeft, Anchor::TopCenter, Anchor::TopRight],
                            vec![Anchor::LeftCenter, Anchor::RightCenter],
                            vec![
                                Anchor::BottomLeft,
                                Anchor::BottomCenter,
                                Anchor::BottomRight,
                            ],
                        ]
                        .into_iter()
                        .map(|anchors| {
                            h_flex()
                                .w_full()
                                .justify_between()
                                .children(anchors.into_iter().map(|anchor| {
                                    let label = format!("{anchor:?}");
                                    Popover::new(SharedString::from(format!(
                                        "popover-anchor-{label}"
                                    )))
                                    .anchor(anchor)
                                    .arrow(self.arrow)
                                    .trigger(
                                        Button::new(SharedString::from(format!(
                                            "popover-anchor-trigger-{label}"
                                        )))
                                        .small()
                                        .outline()
                                        .label(label),
                                    )
                                    .child("Popover content")
                                }))
                        }),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "popover",
        "Popover",
        "Show focused content beside a trigger.",
        PopoverSection::view(window, cx),
    ));
}
