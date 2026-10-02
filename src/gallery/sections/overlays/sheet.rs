//! Sheet section, ported from the upstream `SheetStory`.

use std::{sync::Arc, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Placement, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _, DropdownButton},
    date_picker::{DatePicker, DatePickerState},
    h_flex,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{ProbeAction, demo_toolbar, section};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_overlays_sheet, no_json)]
enum ToggleSheetOption {
    Overlay,
    OverlayClosable,
}

const SHEET_ITEMS: [&str; 49] = [
    "Baguette (France)",
    "Baklava (Turkey)",
    "Beef Wellington (UK)",
    "Biryani (India)",
    "Borscht (Ukraine)",
    "Bratwurst (Germany)",
    "Bulgogi (Korea)",
    "Burrito (USA)",
    "Ceviche (Peru)",
    "Chicken Tikka Masala (India)",
    "Churrasco (Brazil)",
    "Couscous (North Africa)",
    "Croissant (France)",
    "Dim Sum (China)",
    "Empanada (Argentina)",
    "Fajitas (Mexico)",
    "Falafel (Middle East)",
    "Feijoada (Brazil)",
    "Fish and Chips (UK)",
    "Fondue (Switzerland)",
    "Goulash (Hungary)",
    "Haggis (Scotland)",
    "Kebab (Middle East)",
    "Kimchi (Korea)",
    "Lasagna (Italy)",
    "Maple Syrup Pancakes (Canada)",
    "Moussaka (Greece)",
    "Pad Thai (Thailand)",
    "Paella (Spain)",
    "Pancakes (USA)",
    "Pasta Carbonara (Italy)",
    "Pavlova (Australia)",
    "Peking Duck (China)",
    "Pho (Vietnam)",
    "Pierogi (Poland)",
    "Pizza (Italy)",
    "Poutine (Canada)",
    "Pretzel (Germany)",
    "Ramen (Japan)",
    "Rendang (Indonesia)",
    "Sashimi (Japan)",
    "Satay (Indonesia)",
    "Shepherd's Pie (Ireland)",
    "Sushi (Japan)",
    "Tacos (Mexico)",
    "Tandoori Chicken (India)",
    "Tortilla (Spain)",
    "Tzatziki (Greece)",
    "Wiener Schnitzel (Austria)",
];

struct SheetListDelegate {
    story: WeakEntity<SheetSection>,
    confirmed_index: Option<usize>,
    selected_index: Option<usize>,
    items: Vec<Arc<str>>,
    matches: Vec<Arc<str>>,
}

impl ListDelegate for SheetListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.matches.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        let query = query.to_string();
        cx.spawn(async move |this, cx| {
            // Simulates a slow search with a fixed delay (upstream drew the
            // duration from the `fake` crate).
            cx.background_executor()
                .timer(Duration::from_millis(75))
                .await;

            this.update(cx, |this, cx| {
                let query = query.to_lowercase();
                this.delegate_mut().matches = this
                    .delegate()
                    .items
                    .iter()
                    .filter(|item| item.to_lowercase().contains(&query))
                    .cloned()
                    .collect();
                cx.notify();
            })
            .ok();
        })
    }

    fn render_item(
        &mut self,
        ix: gpui_kit::component::IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let confirmed = Some(ix.row) == self.confirmed_index;

        if let Some(item) = self.matches.get(ix.row) {
            let list_item = ListItem::new(("sheet-item", ix.row))
                .check_icon(IconName::Check)
                .confirmed(confirmed)
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(item.to_string()),
                )
                .suffix(|_, _| {
                    Button::new("sheet-item-like")
                        .tab_stop(false)
                        .icon(IconName::Heart)
                        .with_variant(ButtonVariant::Ghost)
                        .size(px(18.))
                        .on_click(|_, window, cx| {
                            cx.stop_propagation();
                            window.prevent_default();
                        })
                });
            Some(list_item)
        } else {
            None
        }
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                Icon::new(IconName::Inbox)
                    .size(px(50.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child("No matches found")
            .items_center()
            .justify_center()
            .p_3()
            .bg(cx.theme().muted)
            .text_color(cx.theme().muted_foreground)
    }

    fn confirm(&mut self, _secondary: bool, _: &mut Window, cx: &mut Context<ListState<Self>>) {
        let confirmed_index = self.selected_index;
        let selected = confirmed_index.and_then(|ix| self.matches.get(ix).cloned());
        let _ = self.story.update(cx, |this, _| {
            self.confirmed_index = confirmed_index;
            if let Some(item) = selected {
                this.selected_value = Some(SharedString::from(item.to_string()));
            }
        });
    }

    fn set_selected_index(
        &mut self,
        ix: Option<gpui_kit::component::IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix.map(|ix| ix.row);

        if ix.is_some() {
            cx.notify();
        }
    }
}

pub struct SheetSection {
    selected_value: Option<SharedString>,
    list: Entity<ListState<SheetListDelegate>>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    date: Entity<DatePickerState>,
    overlay: bool,
    overlay_closable: bool,
}

impl SheetSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let items: Vec<Arc<str>> = SHEET_ITEMS.iter().map(|s| Arc::from(*s)).collect();

            let story = cx.weak_entity();
            let delegate = SheetListDelegate {
                story,
                selected_index: None,
                confirmed_index: None,
                items: items.clone(),
                matches: items,
            };
            let list = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));
            let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
            let input2 = cx.new(|cx| {
                InputState::new(window, cx).placeholder("For test focus back on dialog close.")
            });
            let date = cx.new(|cx| DatePickerState::new(window, cx));

            Self {
                selected_value: None,
                list,
                input1,
                input2,
                date,
                overlay: true,
                overlay_closable: true,
            }
        })
    }

    fn open_sheet_at(&mut self, placement: Placement, window: &mut Window, cx: &mut Context<Self>) {
        let list = self.list.clone();

        let drawer_h = match placement {
            Placement::Left | Placement::Right => px(400.),
            Placement::Top | Placement::Bottom => px(540.),
        };

        let overlay = self.overlay;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date = self.date.clone();
        window.open_sheet_at(placement, cx, move |this, _, cx| {
            this.overlay(overlay)
                .overlay_closable(overlay_closable)
                .size(drawer_h)
                .title("Sheet Title")
                .child(
                    v_flex()
                        .size_full()
                        .gap_3()
                        .child(Input::new(&input1))
                        .child(DatePicker::new(&date).placeholder("Date of Birth"))
                        .child(
                            Button::new("sheet-test-notification")
                                .child("Test Notification")
                                .on_click(|_, window, cx| {
                                    window
                                        .push_notification("Hello this is message from Sheet.", cx)
                                }),
                        )
                        .child(
                            Button::new("sheet-open-confirm-dialog")
                                .child("Open Confirm Dialog")
                                .on_click(|_, window, cx| {
                                    window.open_alert_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .child("Confirm dialog opened from sheet.")
                                            .on_ok(|_, window, cx| {
                                                window
                                                    .push_notification("You have pressed ok.", cx);
                                                true
                                            })
                                            .on_cancel(|_, window, cx| {
                                                window.push_notification(
                                                    "You have pressed cancel.",
                                                    cx,
                                                );
                                                true
                                            })
                                    });
                                }),
                        )
                        .child(
                            List::new(&list)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded(cx.theme().radius),
                        ),
                )
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(
                            Button::new("sheet-confirm")
                                .primary()
                                .label("Confirm")
                                .on_click(|_, window, cx| {
                                    window.close_sheet(cx);
                                }),
                        )
                        .child(Button::new("sheet-cancel").label("Cancel").on_click(
                            |_, window, cx| {
                                window.close_sheet(cx);
                            },
                        )),
                )
        });
    }
}

impl Render for SheetSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let overlay = self.overlay;
        let overlay_closable = self.overlay_closable;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|_, _: &ProbeAction, window, cx| {
                window.push_notification("Probe action dispatched.", cx);
            }))
            .on_action(cx.listener(|this, action: &ToggleSheetOption, _, cx| {
                match action {
                    ToggleSheetOption::Overlay => this.overlay = !this.overlay,
                    ToggleSheetOption::OverlayClosable => {
                        this.overlay_closable = !this.overlay_closable;
                    }
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                DropdownButton::new("sheet-options")
                    .button(
                        Button::new(SharedString::from("sheet-options-trigger")).label("Options"),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Overlay",
                            overlay,
                            Box::new(ToggleSheetOption::Overlay),
                        )
                        .menu_with_check(
                            "Close on overlay click",
                            overlay_closable,
                            Box::new(ToggleSheetOption::OverlayClosable),
                        )
                    })
                    .into_any_element(),
            ]))
            .child(
                section("sheet-placement", "Placement")
                    .description("Open a sheet from any edge of the window.")
                    .child(
                        Button::new("sheet-show-left")
                            .outline()
                            .label("Left Sheet…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_sheet_at(Placement::Left, window, cx)
                            })),
                    )
                    .child(
                        Button::new("sheet-show-top")
                            .outline()
                            .label("Top Sheet…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_sheet_at(Placement::Top, window, cx)
                            })),
                    )
                    .child(
                        Button::new("sheet-show-right")
                            .outline()
                            .label("Right Sheet…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_sheet_at(Placement::Right, window, cx)
                            })),
                    )
                    .child(
                        Button::new("sheet-show-bottom")
                            .outline()
                            .label("Bottom Sheet…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_sheet_at(Placement::Bottom, window, cx)
                            })),
                    ),
            )
            .child(
                section("sheet-scrollable", "Scrollable Sheet")
                    .w(rems(30.))
                    .child(
                        Button::new("sheet-show-scrollable")
                            .outline()
                            .label("Scrollable Sheet…")
                            .on_click(|_, window, cx| {
                                window.open_sheet_at(Placement::Right, cx, |this, _, _| {
                                    this.title("Scrollable Sheet")
                                        .child("This is a scrollable sheet.\n".repeat(150))
                                });
                            }),
                    ),
            )
            .child(
                section("sheet-focus-back", "Focus back test")
                    .w_128()
                    .child(Input::new(&self.input2))
                    .child(
                        Button::new("sheet-test-action")
                            .outline()
                            .label("Test Action")
                            .flex_shrink_0()
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(ProbeAction), cx);
                            })
                            .tooltip(
                                "This button for test dispatch action, \
                                to make sure when Dialog close, \
                                \nthis still can handle the action.",
                            ),
                    ),
            )
            .when_some(self.selected_value.clone(), |this, selected_value| {
                this.child(
                    h_flex().gap_1().child("You have selected:").child(
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
        "sheet",
        "Sheet",
        "Open a panel from any edge of the window.",
        SheetSection::view(window, cx),
    ));
}
