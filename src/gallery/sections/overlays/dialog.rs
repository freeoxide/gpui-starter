//! Dialog section, ported from the upstream `DialogStory` and the
//! `dialog_overlay` example.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    date_picker::{DatePicker, DatePickerState},
    dialog::{
        Dialog, DialogAction, DialogClose, DialogDescription, DialogFooter, DialogHeader,
        DialogTitle,
    },
    h_flex,
    input::{Input, InputState},
    menu::ContextMenuExt as _,
    select::{SearchableVec, Select, SelectState},
    table::{Column, DataTable, TableDelegate, TableState},
    text::{TextView, markdown},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{ProbeAction, demo_toolbar, section};

// Menu actions for the context-menu area in the selection-isolation demo.
actions!(gallery_overlays_dialog, [Open, Delete, Export, Info]);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_overlays_dialog, no_json)]
enum ToggleDialogOption {
    Overlay,
    OverlayClosable,
    CloseButton,
    Keyboard,
}

const SCROLLABLE_CONTENT: &str = "\
# Sizing

Dialog width comes from the decision it asks for: a short confirmation stays \
narrow, a form takes a medium width, and complex work belongs in a window.

# Scrolling

The body owns scrolling. The title and footer stay pinned while the content \
area scrolls, so the commit actions never leave the screen.

Content is clamped between a top offset and a bottom margin, and width is \
capped at the viewport minus a margin on each side. A dialog that asks for \
more room than the window has simply uses the room that exists.

# Focus

Opening a dialog moves focus into it; dismissing returns focus to the trigger. \
Escape dismisses the topmost surface first, so nested dialogs unwind in order.

# Composition

The declarative builder hosts a trigger element and a content builder. The \
imperative window API opens the same dialog from any event handler. Both \
share the footer contract: a primary commit and a quiet cancel.

# Long content

This paragraph block exists to make the body overflow. A dialog that fits its \
content never shows a scrollbar; one that does not keeps its header and footer \
visible while the middle region scrolls. The scrollbar belongs to the body \
region and sits at its trailing edge.

Scroll behavior is the reason composed dialogs pin their header above the \
scroll owner and their footer below it: padding the whole dialog instead \
would let long content push the actions off screen or clip the title.";

pub struct DialogSection {
    selected_value: Option<SharedString>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    date: Entity<DatePickerState>,
    select: Entity<SelectState<SearchableVec<&'static str>>>,
    table: Entity<TableState<DialogTable>>,
    dialog_overlay: bool,
    close_button: bool,
    keyboard: bool,
    overlay_closable: bool,
}

struct DialogTable {
    columns: Vec<Column>,
}

impl DialogTable {
    fn new() -> Self {
        Self {
            columns: vec![
                Column::new("id", "ID").width(px(50.)),
                Column::new("name", "Name").width(px(150.)),
                Column::new("email", "Email").width(px(250.)),
                Column::new("role", "Role").width(px(150.)),
                Column::new("status", "Status").width(px(100.)),
            ],
        }
    }
}

impl TableDelegate for DialogTable {
    fn columns_count(&self, _: &App) -> usize {
        5
    }

    fn rows_count(&self, _: &App) -> usize {
        200
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        match col_ix {
            1 => format!("User {row_ix}").into_any_element(),
            2 => format!("user-{row_ix}@mail.com").into_any_element(),
            3 => "User".into_any_element(),
            4 => "Active".into_any_element(),
            _ => format!("{row_ix}").into_any_element(),
        }
    }
}

impl DialogSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
            let input2 = cx
                .new(|cx| InputState::new(window, cx).placeholder("Type before opening a dialog"));
            let date = cx.new(|cx| DatePickerState::new(window, cx));
            let select = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(vec!["Option 1", "Option 2", "Option 3"]),
                    None,
                    window,
                    cx,
                )
            });
            let table = cx.new(|cx| TableState::new(DialogTable::new(), window, cx));

            Self {
                selected_value: None,
                input1,
                input2,
                date,
                select,
                table,
                dialog_overlay: true,
                close_button: true,
                keyboard: true,
                overlay_closable: true,
            }
        })
    }

    fn render_basic_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date = self.date.clone();
        let select = self.select.clone();
        let view = cx.entity();

        section("dialog-default", "Default")
            .description("Compose form controls and footer actions.")
            .child(
                Dialog::new(cx)
                    .trigger(Button::new("dialog-show").outline().label("Open Dialog"))
                    .overlay(dialog_overlay)
                    .keyboard(self.keyboard)
                    .close_button(self.close_button)
                    .overlay_closable(overlay_closable)
                    .on_ok({
                        let view = view.clone();
                        let input1 = input1.clone();
                        let date = date.clone();
                        move |_, window, cx| {
                            view.update(cx, |view, cx| {
                                view.selected_value = Some(
                                    format!(
                                        "Hello, {}, date: {}",
                                        input1.read(cx).value(),
                                        date.read(cx).date()
                                    )
                                    .into(),
                                );
                                cx.notify();
                            });
                            window.push_notification("You have pressed confirm.", cx);
                            true
                        }
                    })
                    .p_0()
                    .content({
                        move |content, _, cx| {
                            content
                                .child(
                                    DialogHeader::new()
                                        .p_4()
                                        .child(DialogTitle::new().child("Basic Dialog"))
                                        .child(DialogDescription::new().child(
                                            "This is a basic dialog created \
                                                using the declarative API.",
                                        )),
                                )
                                .child(
                                    v_flex()
                                        .px_4()
                                        .pb_4()
                                        .gap_3()
                                        .child("This is a dialog, you can put anything here.")
                                        .child(Input::new(&input1))
                                        .child(Select::new(&select))
                                        .child(DatePicker::new(&date).placeholder("Date of Birth")),
                                )
                                .child(
                                    DialogFooter::new()
                                        .p_4()
                                        .bg(cx.theme().muted)
                                        .justify_between()
                                        .child(
                                            Button::new("dialog-open-other")
                                                .label("Open Other Dialog")
                                                .outline()
                                                .on_click(move |_, window, cx| {
                                                    window.open_dialog(cx, move |dialog, _, _| {
                                                        dialog
                                                            .title("Other Dialog")
                                                            .child("This is another dialog.")
                                                            .min_h(px(100.))
                                                            .overlay_closable(overlay_closable)
                                                    });
                                                }),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(
                                                    DialogClose::new().child(
                                                        Button::new("dialog-cancel")
                                                            .label("Cancel")
                                                            .outline(),
                                                    ),
                                                )
                                                .child(
                                                    DialogAction::new().child(
                                                        Button::new("dialog-confirm")
                                                            .primary()
                                                            .label("Confirm"),
                                                    ),
                                                ),
                                        ),
                                )
                        }
                    }),
            )
    }

    fn render_focus_return_check(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex().w_full().justify_center().child(
            v_flex()
                .w_full()
                .max_w_96()
                .gap_3()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().muted.opacity(0.45))
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().font_medium().child("Focus return check"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Type here, then open and close any dialog."),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(Input::new(&self.input2).w_full()),
                        )
                        .child(
                            Button::new("dialog-run-action")
                                .outline()
                                .label("Run Action")
                                .flex_shrink_0()
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(Box::new(ProbeAction), cx);
                                })
                                .tooltip("Verify actions still dispatch after a dialog closes."),
                        ),
                ),
        )
    }

    fn render_dialog_without_title(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("dialog-without-title", "Without title")
            .description("Render content without a heading.")
            .child(
                Button::new("dialog-no-title")
                    .outline()
                    .label("Dialog without Title")
                    .on_click(move |_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog
                                .overlay(dialog_overlay)
                                .overlay_closable(overlay_closable)
                                .child(
                                    "This is a dialog without title, \
                                    you can use it when the title is not necessary.",
                                )
                        });
                    }),
            )
    }

    fn render_custom_buttons(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("dialog-custom-actions", "Custom actions")
            .description("Replace the default footer actions.")
            .child(
                Button::new("dialog-custom-buttons")
                    .outline()
                    .label("Custom Buttons")
                    .on_click(move |_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, cx| {
                            dialog
                                .rounded(cx.theme().radius_lg)
                                .overlay(dialog_overlay)
                                .overlay_closable(overlay_closable)
                                .child(
                                    v_flex()
                                        .gap_3()
                                        .items_center()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(cx.theme().radius_lg)
                                                .bg(cx.theme().warning.opacity(0.2))
                                                .size_12()
                                                .text_color(cx.theme().warning)
                                                .child(Icon::new(IconName::TriangleAlert).size_8()),
                                        )
                                        .child(
                                            "Update successful, \
                                            we need to restart the application.",
                                        ),
                                )
                                .footer(
                                    DialogFooter::new()
                                        .child(DialogClose::new().child(
                                            Button::new("dialog-later").label("Later").outline(),
                                        ))
                                        .child(
                                            DialogAction::new().child(
                                                Button::new("dialog-restart")
                                                    .label("Restart Now")
                                                    .primary(),
                                            ),
                                        ),
                                )
                                .on_ok(|_, window, cx| {
                                    window.push_notification("You have pressed restart.", cx);
                                    true
                                })
                                .on_cancel(|_, window, cx| {
                                    window.push_notification("You have pressed later.", cx);
                                    true
                                })
                        });
                    }),
            )
    }

    fn render_scrollable_dialog(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("dialog-scrollable", "Scrollable")
            .description("Keep long content inside a fixed dialog size.")
            .child(
                Button::new("dialog-scrollable")
                    .outline()
                    .label("Scrollable Dialog")
                    .on_click(move |_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog
                                .w(px(720.))
                                .h(px(600.))
                                .overlay(dialog_overlay)
                                .overlay_closable(overlay_closable)
                                .title("Dialog with scrollbar")
                                .child(markdown(SCROLLABLE_CONTENT))
                                .footer(
                                    DialogFooter::new()
                                        .gap_2()
                                        .child(
                                            DialogClose::new().child(
                                                Button::new("dialog-scroll-cancel")
                                                    .label("Cancel")
                                                    .outline(),
                                            ),
                                        )
                                        .child(
                                            DialogAction::new().child(
                                                Button::new("dialog-scroll-confirm")
                                                    .label("Confirm")
                                                    .primary(),
                                            ),
                                        ),
                                )
                        });
                    }),
            )
    }

    fn render_table_in_dialog(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("dialog-data-table", "Data table")
            .description("Embed a full interactive component.")
            .child(
                Button::new("dialog-table")
                    .outline()
                    .label("Table Dialog")
                    .on_click({
                        let table = self.table.clone();
                        move |_, window, cx| {
                            let table = table.clone();
                            window.open_dialog(cx, move |dialog, _, _| {
                                dialog
                                    .w(px(800.))
                                    .h(px(600.))
                                    .overlay(dialog_overlay)
                                    .overlay_closable(overlay_closable)
                                    .title("Dialog with Table")
                                    .child(
                                        v_flex()
                                            .size_full()
                                            .gap_3()
                                            .child("This dialog contains a table component.")
                                            .child(DataTable::new(&table)),
                                    )
                            });
                        }
                    }),
            )
    }

    fn render_custom_paddings(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        section("dialog-padding", "Padding")
            .description("Control spacing around dialog content.")
            .child(
                Button::new("dialog-custom-paddings")
                    .outline()
                    .label("Custom Paddings")
                    .on_click(|_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog.p_3().title("Custom Dialog Title").child(
                                "This is a custom dialog content, we can use \
                                paddings to control the layout and spacing within \
                                the dialog.",
                            )
                        });
                    }),
            )
    }

    fn render_custom_style(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        section("dialog-custom-style", "Custom style")
            .description("Customize color, radius, and foreground.")
            .child(
                Button::new("dialog-custom-style")
                    .outline()
                    .label("Custom Dialog Style")
                    .on_click(|_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, cx| {
                            dialog
                                .rounded(cx.theme().radius_lg)
                                .bg(cx.theme().cyan)
                                .text_color(cx.theme().info_foreground)
                                .title("Custom Dialog Title")
                                .child("This is a custom dialog content.")
                        });
                    }),
            )
    }

    fn render_dialog_with_content(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        section("dialog-custom-content", "Custom content")
            .description("Compose header, body, and footer explicitly.")
            .child(
                Button::new("dialog-custom-width")
                    .outline()
                    .label("Custom Width (400px)")
                    .on_click(|_, window, cx| {
                        window.open_dialog(cx, move |dialog, _, _| {
                            dialog.w(px(400.)).content(|content, _, _| {
                                content
                                    .child(
                                        DialogHeader::new()
                                            .child(DialogTitle::new().child("Custom Width"))
                                            .child(
                                                DialogDescription::new().child(
                                                    "This dialog has a custom width of 400px.",
                                                ),
                                            ),
                                    )
                                    .child(
                                        "Content area with custom width configuration, \
                                        and the footer is used flex 1 button widths.",
                                    )
                                    .child(
                                        DialogFooter::new()
                                            .justify_center()
                                            .child(
                                                Button::new("dialog-width-cancel")
                                                    .flex_1()
                                                    .outline()
                                                    .label("Cancel")
                                                    .on_click(|_, window, cx| {
                                                        window.close_dialog(cx);
                                                    }),
                                            )
                                            .child(
                                                Button::new("dialog-width-done")
                                                    .flex_1()
                                                    .primary()
                                                    .label("Done")
                                                    .on_click(|_, window, cx| {
                                                        window.close_dialog(cx);
                                                    }),
                                            ),
                                    )
                            })
                        })
                    }),
            )
    }

    fn render_textview_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        section("dialog-selectable-text", "Selectable text")
            .description("Embed selectable rich text.")
            .child(
                Dialog::new(cx)
                    .trigger(
                        Button::new("dialog-textview")
                            .outline()
                            .label("TextView Dialog"),
                    )
                    .overlay(dialog_overlay)
                    .keyboard(self.keyboard)
                    .close_button(self.close_button)
                    .overlay_closable(overlay_closable)
                    .p_0()
                    .content(move |content, _, cx| {
                        content
                            .child(
                                DialogHeader::new()
                                    .p_4()
                                    .child(DialogTitle::new().child("TextView Dialog")),
                            )
                            .child(
                                v_flex().px_4().pb_4().gap_3().child(
                                    TextView::markdown(
                                        "dialog-textview-content",
                                        "This is a dialog with a selectable \
                                        TextView in it. This text should be \
                                        selectable.",
                                    )
                                    .selectable(true),
                                ),
                            )
                            .child(
                                DialogFooter::new()
                                    .p_4()
                                    .bg(cx.theme().muted)
                                    .child(
                                        DialogClose::new().child(
                                            Button::new("dialog-textview-cancel")
                                                .label("Cancel")
                                                .outline(),
                                        ),
                                    )
                                    .child(
                                        DialogAction::new().child(
                                            Button::new("dialog-textview-confirm")
                                                .primary()
                                                .label("Confirm"),
                                        ),
                                    ),
                            )
                    }),
            )
    }

    fn render_selection_isolation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("dialog-selection-isolation", "Selection isolation")
            .description(
                "A selection started inside the dialog or sheet must stay off the text behind it.",
            )
            .v_flex()
            .items_start()
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        Button::new("dialog-isolation-open")
                            .outline()
                            .label("Open dialog")
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog
                                        .title("Selectable dialog")
                                        .child(
                                            TextView::markdown(
                                                "dialog-isolation-text",
                                                "Select **this** text, then drag the mouse *out of the dialog* over \
                                                 the paragraph behind it. The text behind must NOT get selected",
                                            )
                                            .selectable(true),
                                        )
                                });
                            }),
                    )
                    .child(
                        Button::new("sheet-isolation-open")
                            .outline()
                            .label("Open Sheet")
                            .on_click(|_, window, cx| {
                                window.open_sheet(cx, |sheet, _, _| {
                                    sheet
                                        .title("Selectable Sheet")
                                        .child(
                                            TextView::markdown(
                                                "sheet-isolation-text",
                                                "Select **this** text, then drag the mouse *out of the sheet* over \
                                                 the paragraph behind it. The text behind must NOT get selected",
                                            )
                                            .selectable(true),
                                        )
                                });
                            }),
                    ),
            )
            .child(
                TextView::markdown(
                    "dialog-isolation-background",
                    "**Background text** behind the modals. While a dialog or \
                     sheet is open, a selection started inside it must not \
                     extend onto this paragraph.",
                )
                .selectable(true),
            )
            .child(
                div()
                    .id("dialog-isolation-hover-area")
                    .v_flex()
                    .h_40()
                    .border_1()
                    .border_dashed()
                    .border_color(cx.theme().border)
                    .items_center()
                    .justify_center()
                    .hover(|this| this.bg(cx.theme().warning.opacity(0.2)))
                    .child("Hover test here.")
                    .child("Right click to show Context Menu")
                    .context_menu(|this, _, _| {
                        this.separator()
                            .menu("Open", Box::new(Open))
                            .menu("Delete", Box::new(Delete))
                            .menu("Export", Box::new(Export))
                            .menu("Info", Box::new(Info))
                            .separator()
                    }),
            )
    }
}

impl Render for DialogSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;
        let close_button = self.close_button;
        let keyboard = self.keyboard;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &ToggleDialogOption, _, cx| {
                match action {
                    ToggleDialogOption::Overlay => this.dialog_overlay = !this.dialog_overlay,
                    ToggleDialogOption::OverlayClosable => {
                        this.overlay_closable = !this.overlay_closable;
                    }
                    ToggleDialogOption::CloseButton => this.close_button = !this.close_button,
                    ToggleDialogOption::Keyboard => this.keyboard = !this.keyboard,
                }
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &ProbeAction, window, cx| {
                window.push_notification("Probe action dispatched.", cx);
            }))
            .child(demo_toolbar(vec![
                DropdownButton::new("dialog-options")
                    .button(
                        Button::new(SharedString::from("dialog-options-trigger")).label("Options"),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Overlay",
                            overlay,
                            Box::new(ToggleDialogOption::Overlay),
                        )
                        .menu_with_check(
                            "Close on overlay click",
                            overlay_closable,
                            Box::new(ToggleDialogOption::OverlayClosable),
                        )
                        .menu_with_check(
                            "Close button",
                            close_button,
                            Box::new(ToggleDialogOption::CloseButton),
                        )
                        .menu_with_check(
                            "Keyboard",
                            keyboard,
                            Box::new(ToggleDialogOption::Keyboard),
                        )
                    })
                    .into_any_element(),
            ]))
            .child(self.render_basic_dialog(cx))
            .child(self.render_custom_buttons(cx))
            .child(self.render_scrollable_dialog(cx))
            .child(self.render_table_in_dialog(cx))
            .child(self.render_dialog_without_title(cx))
            .child(self.render_custom_paddings(cx))
            .child(self.render_custom_style(cx))
            .child(self.render_dialog_with_content(cx))
            .child(self.render_textview_dialog(cx))
            .child(self.render_selection_isolation(cx))
            .when(cfg!(not(target_family = "wasm")), |this| {
                this.child(self.render_focus_return_check(cx))
            })
            .when_some(self.selected_value.clone(), |this, selected_value| {
                this.child(
                    h_flex().gap_1().child("You have confirmed:").child(
                        div()
                            .child(selected_value.to_string())
                            .text_color(cx.theme().danger),
                    ),
                )
            })
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "dialog",
        "Dialog",
        "Present focused content above the current view.",
        DialogSection::view(window, cx),
    ));
}
