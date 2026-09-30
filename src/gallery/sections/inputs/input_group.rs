//! Input Group section, ported from the upstream `InputGroupStory` and its
//! `examples` companion module (merged here; the story crate split the demo
//! builders out only to keep its own file small).

use std::collections::BTreeMap;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, Size,
    button::{Button, ButtonVariants as _},
    form::{Field, Form},
    group_box::GroupBox,
    h_flex,
    input::{
        Input, InputContentType, InputEvent, InputGroup, InputGroupAddon,
        InputGroupAddonAlignment as Align, InputGroupButton, InputGroupInput, InputGroupText,
        InputGroupTextarea, InputState, TextareaState,
    },
    kbd::Kbd,
    label::Label,
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, ClipboardItem, Context, Entity, FontWeight, IntoElement, Keystroke,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// The demo inputs the story builds up front, keyed by the suffix every
/// element ID in their group carries.
const INPUT_KEYS: [(&str, &str, &str); 22] = [
    ("align-start", "Search…", ""),
    ("align-end", "Enter password", ""),
    ("align-top", "Enter your full name", ""),
    ("align-bottom", "0.00", ""),
    ("icon-email", "Enter your email", ""),
    ("icon-verified", "Username", "ada"),
    ("icon-multiple", "Website", "gpui-kit.com"),
    ("domain", "example", ""),
    ("username", "Enter your username", ""),
    ("tooltip-password", "Enter password", ""),
    ("tooltip-email", "Your email address", ""),
    ("dropdown-file", "Enter file name", "notes.txt"),
    ("dropdown-search", "Enter search query", ""),
    ("phone", "Phone number", ""),
    ("popover-url", "example.com", "gpui-kit.com"),
    ("label-username", "username", ""),
    ("label-email", "you@example.com", ""),
    ("button-actions", "Enter a project name", "Input Group"),
    ("loading-end", "Searching…", ""),
    ("loading-start", "Processing…", ""),
    ("loading-text", "Saving changes…", ""),
    ("profile-name", "Your name", "Ada Lovelace"),
];

const TEXTAREA_KEYS: [(&str, &str, &str); 7] = [
    ("textarea-plain", "Enter your text here…", ""),
    ("textarea-header", "Write your question…", ""),
    ("textarea-footer", "Enter your message", ""),
    ("textarea-disabled", "", "This textarea is disabled."),
    ("textarea-invalid", "Write a short summary…", ""),
    ("comment", "Share your thoughts…", ""),
    ("custom", "An automatically growing textarea…", ""),
];

const COMPONENT_NAMES: [&str; 12] = [
    "Button",
    "Input",
    "Textarea",
    "Input Group",
    "Select",
    "Combobox",
    "Checkbox",
    "Radio",
    "Slider",
    "Switch",
    "Calendar",
    "Color Picker",
];

pub struct InputGroupSection {
    search: Entity<InputState>,
    url: Entity<InputState>,
    amount: Entity<InputState>,
    password: Entity<InputState>,
    email: Entity<InputState>,
    disabled: Entity<InputState>,
    disabled_invalid: Entity<InputState>,
    readonly: Entity<InputState>,
    loading: Entity<InputState>,
    shortcut: Entity<InputState>,
    notes: Entity<TextareaState>,
    message: Entity<TextareaState>,
    sizes: Vec<(Size, Entity<InputState>)>,
    copied: bool,
    starred: bool,
    runs: usize,
    attached: bool,
    last_message: Option<SharedString>,
    last_search: Option<SharedString>,
    example_inputs: BTreeMap<&'static str, Entity<InputState>>,
    example_textareas: BTreeMap<&'static str, Entity<TextareaState>>,
    search_scope: &'static str,
    country_code: &'static str,
    posted_comment: Option<SharedString>,
    submitted_custom: Option<SharedString>,
    saved_profile: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

/// A labeled field row, ported from the story's `labeled()` helper.
fn labeled(label: &'static str, description: &'static str, control: impl IntoElement) -> Field {
    Field::new()
        .label(label)
        .description(description)
        .child(control)
}

/// The narrow column every demo box stacks its groups in.
fn column() -> gpui_kit::Div {
    v_flex().w_full().max_w(rems(24.)).gap_4()
}

/// A compact labeled trigger, ported from the story's `compact_trigger()`.
fn compact_trigger(id: SharedString, label: impl Into<SharedString>) -> InputGroupButton {
    InputGroupButton::new(id).label(label)
}

impl InputGroupSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search components…"));
            let url = cx.new(|cx| InputState::new(window, cx).default_value("gpui-kit.com"));
            let amount = cx.new(|cx| InputState::new(window, cx).placeholder("0.00"));
            let password = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Enter password")
                    .masked(true)
            });
            let email = cx.new(|cx| InputState::new(window, cx).placeholder("you@example.com"));
            let disabled = cx.new(|cx| InputState::new(window, cx).default_value("Unavailable"));
            let disabled_invalid =
                cx.new(|cx| InputState::new(window, cx).default_value("Invalid saved value"));
            let readonly =
                cx.new(|cx| InputState::new(window, cx).default_value("https://gpui-kit.com"));
            let loading = cx.new(|cx| InputState::new(window, cx).placeholder("Searching…"));
            let shortcut = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
            let notes = cx.new(|cx| {
                TextareaState::new(window, cx)
                    .default_value("console.log('Hello, GPUI Kit!');")
                    .rows(4)
            });
            let message = cx.new(|cx| {
                TextareaState::new(window, cx)
                    .placeholder("Write a message…")
                    .auto_grow(2, 6)
            });
            let sizes = [Size::XSmall, Size::Small, Size::Medium, Size::Large]
                .into_iter()
                .map(|size| {
                    let state = cx.new(|cx| {
                        InputState::new(window, cx).placeholder(format!("{} input", size.as_str()))
                    });
                    (size, state)
                })
                .collect();
            let example_inputs: BTreeMap<&'static str, Entity<InputState>> = INPUT_KEYS
                .into_iter()
                .map(|(id, placeholder, value)| {
                    let state = cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder(placeholder)
                            .default_value(value)
                            .masked(matches!(id, "align-end" | "tooltip-password"))
                    });
                    (id, state)
                })
                .collect();
            let example_textareas: BTreeMap<&'static str, Entity<TextareaState>> = TEXTAREA_KEYS
                .into_iter()
                .map(|(id, placeholder, value)| {
                    let state = cx.new(|cx| {
                        let state = TextareaState::new(window, cx)
                            .placeholder(placeholder)
                            .default_value(value)
                            .rows(3);
                        if id == "custom" {
                            state.auto_grow(1, 8)
                        } else {
                            state
                        }
                    });
                    (id, state)
                })
                .collect();
            let mut subscriptions = [&search, &url, &email]
                .into_iter()
                .map(|state| {
                    cx.subscribe(state, |_: &mut Self, _, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            cx.notify();
                        }
                    })
                })
                .collect::<Vec<_>>();
            subscriptions.push(cx.subscribe(&message, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }));
            subscriptions.push(
                cx.subscribe(&shortcut, |this, state, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.last_search = Some(state.read(cx).value());
                        cx.notify();
                    }
                }),
            );
            for state in example_inputs.values() {
                subscriptions.push(cx.subscribe(state, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                }));
            }
            for state in example_textareas.values() {
                subscriptions.push(cx.subscribe(state, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                }));
            }
            Self {
                search,
                url,
                amount,
                password,
                email,
                disabled,
                disabled_invalid,
                readonly,
                loading,
                shortcut,
                notes,
                message,
                sizes,
                copied: false,
                starred: false,
                runs: 0,
                attached: false,
                last_message: None,
                last_search: None,
                example_inputs,
                example_textareas,
                search_scope: "Documentation",
                country_code: "+1",
                posted_comment: None,
                submitted_custom: None,
                saved_profile: None,
                _subscriptions: subscriptions,
            }
        })
    }

    /// Every element ID in this section carries the `input-group-` prefix the
    /// gallery's uniqueness contract requires; upstream ids were bare.
    fn id(&self, suffix: &str) -> SharedString {
        SharedString::from(format!("input-group-{suffix}"))
    }

    fn input(&self, key: &'static str, label: &'static str) -> InputGroup {
        InputGroup::new(self.id(key)).input(
            InputGroupInput::new(&self.example_inputs[key])
                .aria_label(label)
                .when(matches!(key, "align-end" | "tooltip-password"), |input| {
                    input.content_type(InputContentType::Password)
                }),
        )
    }

    fn textarea(&self, key: &'static str, label: &'static str) -> InputGroup {
        InputGroup::new(self.id(key))
            .input(InputGroupTextarea::new(&self.example_textareas[key]).aria_label(label))
    }

    fn copy_url(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.readonly.read(cx).value().to_string(),
        ));
        self.copied = true;
        cx.notify();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let message = self.message.read(cx).value();
        if message.trim().is_empty() || message.chars().count() > 280 {
            return;
        }
        self.last_message = Some(message);
        self.message.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        self.attached = false;
        cx.notify();
    }

    fn clear_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.example_textareas["comment"].update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    fn render_alignment(&self) -> impl IntoElement {
        section("input-group-alignment", "Alignment")
            .description("Place addons before, after, above, or below the control.")
            .child(
                column()
                    .child(labeled(
                        "Inline start",
                        "A leading search icon.",
                        self.input("align-start", "Leading icon search").addon(
                            InputGroupAddon::new(self.id("align-start-addon"))
                                .child(Icon::new(IconName::Search).size_4()),
                        ),
                    ))
                    .child(labeled(
                        "Inline end",
                        "A trailing icon with a masked input.",
                        self.input("align-end", "Trailing icon password").addon(
                            InputGroupAddon::new(self.id("align-end-addon"))
                                .align(Align::InlineEnd)
                                .child(Icon::new(IconName::EyeOff).size_4()),
                        ),
                    ))
                    .child(labeled(
                        "Block start",
                        "The header is inside the shared frame.",
                        self.input("align-top", "Full name").addon(
                            InputGroupAddon::new(self.id("align-top-addon"))
                                .align(Align::BlockStart)
                                .child(InputGroupText::new().child("Full Name")),
                        ),
                    ))
                    .child(labeled(
                        "Block end",
                        "The unit sits below the single-line input.",
                        self.input("align-bottom", "Amount with footer").addon(
                            InputGroupAddon::new(self.id("align-bottom-addon"))
                                .align(Align::BlockEnd)
                                .child(InputGroupText::new().child("USD")),
                        ),
                    )),
            )
    }

    fn render_icons(&self) -> impl IntoElement {
        section("input-group-icons", "Icons")
            .description("Leading, paired, and multiple trailing icons.")
            .child(
                column()
                    .child(
                        self.input("icon-email", "Email with icon").addon(
                            InputGroupAddon::new(self.id("icon-email-addon"))
                                .child(Icon::new(IconName::Inbox).size_4()),
                        ),
                    )
                    .child(
                        self.input("icon-verified", "Verified username")
                            .addon(
                                InputGroupAddon::new(self.id("icon-user-addon"))
                                    .child(Icon::new(IconName::User).size_4()),
                            )
                            .addon(
                                InputGroupAddon::new(self.id("icon-check-addon"))
                                    .align(Align::InlineEnd)
                                    .child(Icon::new(IconName::Check).size_4()),
                            ),
                    )
                    .child(
                        self.input("icon-multiple", "Website with multiple icons")
                            .addon(
                                InputGroupAddon::new(self.id("icon-multiple-addon"))
                                    .align(Align::InlineEnd)
                                    .child(Icon::new(IconName::Star).size_4())
                                    .child(Icon::new(IconName::Info).size_4()),
                            ),
                    ),
            )
    }

    fn render_tooltips(&self) -> impl IntoElement {
        section("input-group-tooltips", "Tooltips")
            .description("Compact help triggers keep the explanation beside its field.")
            .child(
                column().children(
                    [
                        (
                            "tooltip-password",
                            "Password help",
                            "Use at least 8 characters.",
                        ),
                        (
                            "tooltip-email",
                            "Email help",
                            "Used for notifications about this workspace.",
                        ),
                    ]
                    .map(|(key, label, help)| {
                        self.input(key, label).addon(
                            InputGroupAddon::new(self.id(&format!("tooltip-addon-{key}")))
                                .align(Align::InlineEnd)
                                .child(
                                    InputGroupButton::new(
                                        self.id(&format!("tooltip-button-{key}")),
                                    )
                                    .icon(IconName::Info)
                                    .accessibility_label(label)
                                    .tooltip(help),
                                ),
                        )
                    }),
                ),
            )
    }

    fn render_dropdowns(&self, cx: &Context<Self>) -> impl IntoElement {
        let file_view = cx.entity().downgrade();
        let scope_view = cx.entity().downgrade();
        let phone_view = cx.entity().downgrade();
        let scope = self.search_scope;
        let country = self.country_code;
        section(
            "input-group-dropdowns",
            "Dropdown menus",
        )
        .description("Menus act on the filename or choose the search scope and country code.")
        .child(
            column()
                .child(
                    self.input("dropdown-file", "File name").addon(
                        InputGroupAddon::new(self.id("file-menu-addon"))
                            .align(Align::InlineEnd)
                            .child(
                                compact_trigger(self.id("file-menu"), "More").dropdown_menu(
                                    move |menu, _, _| {
                                        ["Copy filename", "Reset filename", "Clear filename"]
                                            .into_iter()
                                            .fold(menu, |menu, label| {
                                                let view = file_view.clone();
                                                menu.item(
                                                    PopupMenuItem::new(label).on_click(
                                                        move |_, window, cx| {
                                                            let _ = view.update(
                                                                cx,
                                                                |this: &mut Self, cx| {
                                                                    let state =
                                                                        &this.example_inputs
                                                                            ["dropdown-file"];
                                                                    if label == "Copy filename" {
                                                                        cx.write_to_clipboard(
                                                                            ClipboardItem::new_string(
                                                                                state.read(cx).value().to_string(),
                                                                            ),
                                                                        );
                                                                    } else {
                                                                        state.update(
                                                                            cx,
                                                                            |state, cx| {
                                                                                state.set_value(
                                                                                    if label
                                                                                        == "Reset filename"
                                                                                    {
                                                                                        "notes.txt"
                                                                                    } else {
                                                                                        ""
                                                                                    },
                                                                                    window,
                                                                                    cx,
                                                                                );
                                                                            },
                                                                        );
                                                                    }
                                                                    cx.notify();
                                                                },
                                                            );
                                                        },
                                                    ),
                                                )
                                            })
                                    },
                                ),
                            ),
                    ),
                )
                .child(
                    self.input("dropdown-search", "Scoped search").addon(
                        InputGroupAddon::new(self.id("scope-menu-addon"))
                            .align(Align::InlineEnd)
                            .child(
                                compact_trigger(self.id("scope-menu"), scope)
                                    .dropdown_caret(true)
                                    .dropdown_menu(move |menu, _, _| {
                                        ["Documentation", "Blog posts", "Changelog"]
                                            .into_iter()
                                            .fold(menu, |menu, label| {
                                                let view = scope_view.clone();
                                                menu.item(
                                                    PopupMenuItem::new(label)
                                                        .checked(label == scope)
                                                        .on_click(move |_, _, cx| {
                                                            let _ = view.update(
                                                                cx,
                                                                |this: &mut Self, cx| {
                                                                    this.search_scope = label;
                                                                    cx.notify();
                                                                },
                                                            );
                                                        }),
                                                )
                                            })
                                    }),
                            ),
                    ),
                )
                .child(
                    self.input("phone", "Phone number").addon(
                        InputGroupAddon::new(self.id("country-menu-addon")).child(
                            compact_trigger(self.id("country-menu"), country)
                                .dropdown_caret(true)
                                .dropdown_menu(move |menu, _, _| {
                                    ["+1", "+44", "+46"].into_iter().fold(menu, |menu, label| {
                                        let view = phone_view.clone();
                                        menu.item(
                                            PopupMenuItem::new(label)
                                                .checked(label == country)
                                                .on_click(move |_, _, cx| {
                                                    let _ = view.update(
                                                        cx,
                                                        |this: &mut Self, cx| {
                                                            this.country_code = label;
                                                            cx.notify();
                                                        },
                                                    );
                                                }),
                                        )
                                    })
                                }),
                        ),
                    ),
                ),
        )
    }

    fn render_popover(&self, cx: &Context<Self>) -> impl IntoElement {
        let address = self.example_inputs["popover-url"].read(cx).value();
        section("input-group-popover", "Popover")
            .description("A native popover keeps contextual details attached to its trigger.")
            .child(
                column().child(
                    self.input("popover-url", "Website with details").addon(
                        InputGroupAddon::new(self.id("address-details-addon")).child(
                            Popover::new(self.id("address-details"))
                                .trigger(
                                    InputGroupButton::new(self.id("address-details-trigger"))
                                        .icon(IconName::Info)
                                        .accessibility_label("Address details")
                                        .tooltip("Address details"),
                                )
                                .w(rems(18.))
                                .gap_2()
                                .text_sm()
                                .child(
                                    div()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Address details"),
                                )
                                .child(format!("https://{address}"))
                                .child(
                                    "The protocol prefix stays separate from the editable hostname.",
                                ),
                        ),
                    ),
                ),
            )
    }

    fn render_labels(&self, cx: &Context<Self>) -> impl IntoElement {
        section("input-group-labels", "Labels and descriptions").child(
            column()
                .child(labeled(
                    "Username",
                    "Clicking the @ addon focuses the input.",
                    self.input("label-username", "Username with label").addon(
                        InputGroupAddon::new(self.id("label-username-addon"))
                            .child(Label::new("@")),
                    ),
                ))
                .child(
                    self.input("label-email", "Notification email").addon(
                        InputGroupAddon::new(self.id("label-email-addon"))
                            .align(Align::BlockStart)
                            .child(Label::new("Email").text_color(cx.theme().foreground))
                            .child(
                                InputGroupButton::new(self.id("label-email-help"))
                                    .ml_auto()
                                    .icon(IconName::Info)
                                    .accessibility_label("Notification email help")
                                    .tooltip("We'll use this address for workspace notifications."),
                            ),
                    ),
                ),
        )
    }

    fn render_button_actions(&self, cx: &Context<Self>) -> impl IntoElement {
        section("input-group-actions", "Text and icon actions")
            .description(
                "Use larger buttons for a clear text action; keep secondary actions compact.",
            )
            .child(
                column().child(
                    self.input("button-actions", "Project name").addon(
                        InputGroupAddon::new(self.id("project-actions"))
                            .align(Align::BlockEnd)
                            .child(
                                InputGroupButton::new(self.id("project-clear"))
                                    .small()
                                    .label("Clear")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.example_inputs["button-actions"]
                                            .update(cx, |state, cx| {
                                                state.set_value("", window, cx)
                                            });
                                    })),
                            )
                            .child(
                                InputGroupButton::new(self.id("project-reset"))
                                    .small()
                                    .secondary()
                                    .label("Reset")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.example_inputs["button-actions"]
                                            .update(cx, |state, cx| {
                                                state.set_value("Input Group", window, cx)
                                            });
                                    })),
                            )
                            .child(
                                InputGroupButton::new(self.id("project-copy"))
                                    .ml_auto()
                                    .small()
                                    .icon(IconName::Copy)
                                    .accessibility_label("Copy project name")
                                    .tooltip("Copy project name")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            this.example_inputs["button-actions"]
                                                .read(cx)
                                                .value()
                                                .to_string(),
                                        ));
                                    })),
                            ),
                    ),
                ),
            )
    }

    fn render_loading(&self) -> impl IntoElement {
        section("input-group-loading", "Spinner placement")
            .description("Progress can lead, trail, or sit next to status text.")
            .child(
                column()
                    .child(
                        self.input("loading-end", "Search loading state")
                            .readonly(true)
                            .addon(
                                InputGroupAddon::new(self.id("spinner-end"))
                                    .align(Align::InlineEnd)
                                    .child(Spinner::new().small()),
                            ),
                    )
                    .child(
                        self.input("loading-start", "Processing loading state")
                            .readonly(true)
                            .addon(
                                InputGroupAddon::new(self.id("spinner-start"))
                                    .child(Spinner::new().small()),
                            ),
                    )
                    .child(
                        self.input("loading-text", "Saving loading state")
                            .readonly(true)
                            .addon(
                                InputGroupAddon::new(self.id("spinner-text"))
                                    .align(Align::InlineEnd)
                                    .child(InputGroupText::new().child("Saving…"))
                                    .child(Spinner::new().small()),
                            ),
                    ),
            )
    }

    fn render_textarea_examples(&self, cx: &Context<Self>) -> impl IntoElement {
        let remaining = 120isize
            - self.example_textareas["textarea-footer"]
                .read(cx)
                .value()
                .chars()
                .count() as isize;
        let summary = self.example_textareas["textarea-invalid"].read(cx).value();
        section("input-group-textarea-variants", "Textarea variants").child(
            column()
                .child(labeled(
                    "Without addons",
                    "The text viewport owns wrapping and scrolling.",
                    self.textarea("textarea-plain", "Plain grouped textarea"),
                ))
                .child(
                    self.textarea("textarea-header", "Textarea with header")
                        .addon(
                            InputGroupAddon::new(self.id("textarea-header-addon"))
                                .align(Align::BlockStart)
                                .child(InputGroupText::new().child("Ask, search, or chat…")),
                        ),
                )
                .child(
                    self.textarea("textarea-footer", "Textarea with remaining count")
                        .invalid(remaining < 0)
                        .addon(
                            InputGroupAddon::new(self.id("textarea-footer-addon"))
                                .align(Align::BlockEnd)
                                .child(
                                    InputGroupText::new()
                                        .child(format!("{remaining} characters left")),
                                ),
                        ),
                )
                .child(labeled(
                    "Invalid",
                    "Enter a summary to clear the error.",
                    self.textarea("textarea-invalid", "Required summary")
                        .invalid(summary.trim().is_empty()),
                ))
                .child(labeled(
                    "Disabled",
                    "Text and addon actions are unavailable.",
                    self.textarea("textarea-disabled", "Disabled textarea")
                        .disabled(true)
                        .addon(
                            InputGroupAddon::new(self.id("textarea-disabled-addon"))
                                .align(Align::BlockEnd)
                                .child(
                                    InputGroupButton::new(self.id("textarea-disabled-post"))
                                        .label("Post"),
                                ),
                        ),
                )),
        )
    }

    fn render_comment(&self, cx: &Context<Self>) -> impl IntoElement {
        let value = self.example_textareas["comment"].read(cx).value();
        section("input-group-comment", "Comment composer")
            .description(
                "Cancel clears the draft; Post keeps the submitted text below the composer.",
            )
            .child(
                column()
                    .child(
                        self.textarea("comment", "Comment draft").addon(
                            InputGroupAddon::new(self.id("comment-actions"))
                                .align(Align::BlockEnd)
                                .child(
                                    InputGroupText::new()
                                        .child(format!("{} characters", value.chars().count())),
                                )
                                .child(
                                    InputGroupButton::new(self.id("comment-cancel"))
                                        .ml_auto()
                                        .small()
                                        .label("Cancel")
                                        .disabled(value.is_empty())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.clear_comment(window, cx)
                                        })),
                                )
                                .child(
                                    InputGroupButton::new(self.id("comment-post"))
                                        .small()
                                        .primary()
                                        .label("Post")
                                        .disabled(value.trim().is_empty())
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.posted_comment = Some(
                                                this.example_textareas["comment"].read(cx).value(),
                                            );
                                            this.clear_comment(window, cx);
                                        })),
                                ),
                        ),
                    )
                    .when_some(self.posted_comment.clone(), |this, value| {
                        this.child(div().text_sm().child(format!("Posted: {value}")))
                    }),
            )
    }

    fn render_custom_textarea(&self, cx: &Context<Self>) -> impl IntoElement {
        let state = &self.example_textareas["custom"];
        section("input-group-custom-textarea", "Auto-growing textarea")
            .description(
                "The textarea keeps its own typography; the footer holds a primary submit action.",
            )
            .child(
                column()
                    .child(
                        InputGroup::new(self.id("custom"))
                            .input(
                                InputGroupTextarea::new(state)
                                    .aria_label("Auto-growing draft")
                                    .text_base()
                                    .font_family(cx.theme().mono_font_family.clone()),
                            )
                            .addon(
                                InputGroupAddon::new(self.id("custom-footer"))
                                    .align(Align::BlockEnd)
                                    .child(InputGroupText::new().child("Plain text"))
                                    .child(
                                        InputGroupButton::new(self.id("custom-submit"))
                                            .ml_auto()
                                            .primary()
                                            .label("Submit")
                                            .icon(IconName::ArrowUp)
                                            .disabled(state.read(cx).value().trim().is_empty())
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let state = &this.example_textareas["custom"];
                                                this.submitted_custom =
                                                    Some(state.read(cx).value());
                                                state.update(cx, |state, cx| {
                                                    state.set_value("", window, cx)
                                                });
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    .when_some(self.submitted_custom.clone(), |this, value| {
                        this.child(div().text_sm().child(format!("Submitted: {value}")))
                    }),
            )
    }

    fn render_profile(&self, cx: &Context<Self>) -> impl IntoElement {
        section("input-group-profile", "Form composition")
            .description("Use Field and GroupBox to keep labels, descriptions, and the save action together.")
            .child(
                column()
                    .child(
                        GroupBox::new()
                            .title("Contact details")
                            .child(
                                Form::new()
                                    .child(
                                        Field::new().label("Display name").child(
                                            Input::new(&self.example_inputs["profile-name"])
                                                .aria_label("Profile display name"),
                                        ),
                                    )
                                    .child(
                                        Field::new()
                                            .label("Email")
                                            .description("Shown in this example only.")
                                            .child(
                                                self.input("profile-email", "Profile email").addon(
                                                    InputGroupAddon::new(
                                                        self.id("profile-email-icon"),
                                                    )
                                                    .child(Icon::new(IconName::Inbox).size_4()),
                                                ),
                                            ),
                                    )
                                    .footer(
                                        h_flex().justify_end().child(
                                            Button::new(self.id("profile-save"))
                                                .primary()
                                                .label("Save contact")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.saved_profile = Some(
                                                        format!(
                                                            "{} — {}",
                                                            this.example_inputs["profile-name"]
                                                                .read(cx)
                                                                .value(),
                                                            this.example_inputs["profile-email"]
                                                                .read(cx)
                                                                .value()
                                                        )
                                                        .into(),
                                                    );
                                                    cx.notify();
                                                })),
                                        ),
                                    ),
                            ),
                    )
                    .when_some(self.saved_profile.clone(), |this, value| {
                        this.child(div().text_sm().child(format!("Saved: {value}")))
                    }),
            )
    }
}

impl Render for InputGroupSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search.read(cx).value().to_lowercase();
        let count = COMPONENT_NAMES
            .into_iter()
            .filter(|name| name.to_lowercase().contains(&query))
            .count();
        let email = self.email.read(cx).value();
        let invalid = !email.is_empty() && (!email.contains('@') || !email.contains('.'));
        let message = self.message.read(cx).value();
        let characters = message.chars().count();
        let icon = |name| Icon::new(name).size_4();

        v_flex()
            .w_full()
            .gap_4()
            .p_4()
            .child(
                section("input-group-default", "Default").child(
                    InputGroup::new(self.id("search"))
                        .max_w(rems(24.))
                        .input(
                            InputGroupInput::new(&self.search).aria_label("Search components"),
                        )
                        .addon(
                            InputGroupAddon::new(self.id("search-icon"))
                                .child(icon(IconName::Search)),
                        )
                        .addon(
                            InputGroupAddon::new(self.id("search-results"))
                                .align(Align::InlineEnd)
                                .child(
                                    InputGroupText::new()
                                        .child(format!("{count} results")),
                                ),
                        ),
                ),
            )
            .child(self.render_alignment())
            .child(self.render_icons())
            .child(
                section("input-group-text", "Text")
                    .description("Leading and trailing text share the input frame.")
                    .child(
                        column()
                            .child(
                                InputGroup::new(self.id("url"))
                                    .input(
                                        InputGroupInput::new(&self.url)
                                            .aria_label("Website")
                                            .content_type(InputContentType::Url),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("url-scheme")).child(
                                            InputGroupText::new().child("https://"),
                                        ),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("url-action"))
                                            .align(Align::InlineEnd)
                                            .child(
                                                InputGroupButton::new(self.id("favorite-url"))
                                                    .icon(IconName::Star)
                                                    .accessibility_label("Favorite website")
                                                    .tooltip("Favorite website")
                                                    .when(self.starred, |button| {
                                                        button.text_color(cx.theme().primary)
                                                    })
                                                    .on_click(cx.listener(
                                                        |this, _, _, cx| {
                                                            this.starred = !this.starred;
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                InputGroup::new(self.id("amount"))
                                    .input(
                                        InputGroupInput::new(&self.amount)
                                            .aria_label("Amount"),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("currency-symbol")).child(
                                            InputGroupText::new().child("$"),
                                        ),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("currency-code"))
                                            .align(Align::InlineEnd)
                                            .child(
                                                InputGroupText::new().child("USD"),
                                            ),
                                    ),
                            )
                            .child(
                                self.input("domain", "Domain")
                                    .addon(
                                        InputGroupAddon::new(self.id("domain-protocol")).child(
                                            InputGroupText::new().child("https://"),
                                        ),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("domain-suffix"))
                                            .align(Align::InlineEnd)
                                            .child(InputGroupText::new().child(".com")),
                                    ),
                            )
                            .child(
                                self.input("username", "Work username").addon(
                                    InputGroupAddon::new(self.id("username-domain"))
                                        .align(Align::InlineEnd)
                                        .child(
                                            InputGroupText::new().child("@company.com"),
                                        ),
                                ),
                            ),
                    ),
            )
            .child(
                section("input-group-buttons", "Buttons")
                    .description("Multiple native actions retain their own behavior and focus.")
                    .child(
                        column()
                            .child(
                                InputGroup::new(self.id("copy"))
                                    .readonly(true)
                                    .input(
                                        InputGroupInput::new(&self.readonly)
                                            .aria_label("Documentation URL"),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("copy-actions"))
                                            .align(Align::InlineEnd)
                                            .child(
                                                InputGroupButton::new(self.id("copy-url"))
                                                    .icon(if self.copied {
                                                        IconName::Check
                                                    } else {
                                                        IconName::Copy
                                                    })
                                                    .accessibility_label("Copy URL")
                                                    .tooltip("Copy URL")
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.copy_url(window, cx)
                                                        },
                                                    )),
                                            )
                                            .child(
                                                InputGroupButton::new(self.id("select-url"))
                                                    .label("Select all")
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.readonly.update(
                                                                cx,
                                                                |state, cx| {
                                                                    state.focus(window, cx);
                                                                    state.select_all(
                                                                        window, cx,
                                                                    );
                                                                },
                                                            );
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                InputGroup::new(self.id("password"))
                                    .input(
                                        InputGroupInput::new(&self.password)
                                            .aria_label("Password")
                                            .content_type(InputContentType::Password),
                                    )
                                    .addon(
                                        InputGroupAddon::new(self.id("password-action"))
                                            .align(Align::InlineEnd)
                                            .child(
                                                InputGroupButton::new(self.id("toggle-password"))
                                                    .icon(
                                                        if self
                                                            .password
                                                            .read(cx)
                                                            .presentation()
                                                            .is_masked()
                                                        {
                                                            IconName::Eye
                                                        } else {
                                                            IconName::EyeOff
                                                        },
                                                    )
                                                    .accessibility_label(
                                                        "Toggle password visibility",
                                                    )
                                                    .tooltip("Toggle password visibility")
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.password.update(cx, |state, cx| {
                                                                state.toggle_masked(window, cx);
                                                            });
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    ),
                            ),
                    ),
            )
            .child(self.render_tooltips())
            .child(self.render_dropdowns(cx))
            .child(self.render_popover(cx))
            .child(self.render_labels(cx))
            .child(self.render_button_actions(cx))
            .child(
                section("input-group-shortcut", "Keyboard shortcut and loading").child(
                    column()
                        .child(
                            InputGroup::new(self.id("shortcut"))
                                .input(
                                    InputGroupInput::new(&self.shortcut).aria_label("Search"),
                                )
                                .addon(
                                    InputGroupAddon::new(self.id("shortcut-icon"))
                                        .child(icon(IconName::Search)),
                                )
                                .addon(
                                    InputGroupAddon::new(self.id("shortcut-key"))
                                        .align(Align::InlineEnd)
                                        .child(Kbd::new(Keystroke::parse("enter").unwrap())),
                                ),
                        )
                        .when_some(self.last_search.clone(), |this, query| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Submitted search: {query}")),
                            )
                        })
                        .child(
                            InputGroup::new(self.id("loading-input"))
                                .readonly(true)
                                .input(
                                    InputGroupInput::new(&self.loading)
                                        .aria_label("Search in progress"),
                                )
                                .addon(
                                    InputGroupAddon::new(self.id("loading-spinner"))
                                        .child(Spinner::new().small()),
                                )
                                .addon(
                                    InputGroupAddon::new(self.id("loading-text"))
                                        .align(Align::InlineEnd)
                                        .child(
                                            InputGroupText::new().child("Please wait…"),
                                        ),
                                ),
                        ),
                ),
            )
            .child(self.render_loading())
            .child(
                section("input-group-validation", "Validation and disabled").child(
                    column()
                        .child(
                            v_flex().gap_2()
                                .child(
                                    InputGroup::new(self.id("email"))
                                        .invalid(invalid)
                                        .input(
                                            InputGroupInput::new(&self.email)
                                                .aria_label("Email address")
                                                .content_type(InputContentType::EmailAddress),
                                        )
                                        .addon(
                                            InputGroupAddon::new(self.id("email-icon"))
                                                .child(icon(IconName::Info)),
                                        ),
                                )
                                .child(
                                    div().text_sm()
                                        .text_color(if invalid {
                                            cx.theme().danger
                                        } else {
                                            cx.theme().muted_foreground
                                        })
                                        .child(if invalid {
                                            "Enter an email address such as you@example.com."
                                        } else {
                                            "Validation comes from the application's field value."
                                        }),
                                ),
                        )
                        .child(
                            InputGroup::new(self.id("disabled"))
                                .disabled(true)
                                .input(
                                    InputGroupInput::new(&self.disabled)
                                        .aria_label("Disabled input"),
                                )
                                .addon(
                                    InputGroupAddon::new(self.id("disabled-icon"))
                                        .child(icon(IconName::Search)),
                                )
                                .addon(
                                    InputGroupAddon::new(self.id("disabled-action"))
                                        .align(Align::InlineEnd)
                                        .child(
                                            InputGroupButton::new(self.id("disabled-search"))
                                                .label("Search"),
                                        ),
                                ),
                        )
                        .child(
                            v_flex().gap_2()
                                .child(
                                    InputGroup::new(self.id("disabled-invalid"))
                                        .disabled(true)
                                        .invalid(true)
                                        .input(
                                            InputGroupInput::new(&self.disabled_invalid)
                                                .aria_label("Disabled invalid input"),
                                        ),
                                )
                                .child(
                                    div().text_sm().text_color(cx.theme().danger)
                                        .child("The error remains visible while editing is unavailable."),
                                ),
                        ),
                ),
            )
            .child(self.render_textarea_examples(cx))
            .child(
                section("input-group-notes", "Textarea toolbars").child(
                    column().child(
                        InputGroup::new(self.id("notes"))
                            .input(
                                InputGroupTextarea::new(&self.notes).aria_label("Script text"),
                            )
                            .addon(
                                InputGroupAddon::new(self.id("notes-header"))
                                    .align(Align::BlockStart)
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        InputGroupText::new()
                                            .child(icon(IconName::File))
                                            .child("script.js"),
                                    )
                                    .child(
                                        InputGroupButton::new(self.id("copy-notes"))
                                            .ml_auto()
                                            .icon(IconName::Copy)
                                            .accessibility_label("Copy script")
                                            .tooltip("Copy script")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    this.notes.read(cx).value().to_string(),
                                                ));
                                            })),
                                    ),
                            )
                            .addon(
                                InputGroupAddon::new(self.id("notes-footer"))
                                    .align(Align::BlockEnd)
                                    .border_t_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        InputGroupText::new()
                                            .child(format!("Run count: {}", self.runs)),
                                    )
                                    .child(
                                        InputGroupButton::new(self.id("run-notes"))
                                            .ml_auto()
                                            .secondary()
                                            .label("Run")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.runs += 1;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    ),
                ),
            )
            .child(self.render_comment(cx))
            .child(self.render_custom_textarea(cx))
            .child(self.render_profile(cx))
            .child(
                section("input-group-message", "Chat textarea")
                    .description("The retained textarea grows with the message; its actions remain below it.")
                    .child(
                        column()
                            .child(
                                InputGroup::new(self.id("message"))
                                    .invalid(characters > 280)
                                    .input(
                                        InputGroupTextarea::new(&self.message)
                                            .aria_label("Message"),
                                    )
                                    .when(self.attached, |group| {
                                        group.addon(
                                            InputGroupAddon::new(self.id("message-attachment"))
                                                .align(Align::BlockStart)
                                                .child(
                                                    InputGroupText::new()
                                                        .child(icon(IconName::File))
                                                        .child("notes.txt"),
                                                ),
                                        )
                                    })
                                    .addon(
                                        InputGroupAddon::new(self.id("message-actions"))
                                            .align(Align::BlockEnd)
                                            .child(
                                                InputGroupText::new()
                                                    .child(format!("{characters}/280")),
                                            )
                                            .child(
                                                InputGroupButton::new(self.id("attach-message"))
                                                    .ml_auto()
                                                    .icon(IconName::Plus)
                                                    .accessibility_label(
                                                        "Toggle sample attachment",
                                                    )
                                                    .tooltip("Toggle sample attachment")
                                                    .on_click(cx.listener(
                                                        |this, _, _, cx| {
                                                            this.attached = !this.attached;
                                                            cx.notify();
                                                        },
                                                    )),
                                            )
                                            .child(
                                                InputGroupButton::new(self.id("send-message"))
                                                    .primary()
                                                    .label("Send")
                                                    .disabled(
                                                        message.trim().is_empty()
                                                            || characters > 280,
                                                    )
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.send(window, cx)
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                            .when_some(self.last_message.clone(), |this, message| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("Sent: {message}")),
                                )
                            }),
                    ),
            )
            .child(
                section("input-group-sizes", "Sizes").child(
                    column().children(self.sizes.iter().map(|(size, state)| {
                        InputGroup::new(("input-group-size", state.entity_id()))
                            .with_size(*size)
                            .input(
                                InputGroupInput::new(state)
                                    .aria_label(format!("{} input", size.as_str())),
                            )
                            .addon(
                                InputGroupAddon::new((
                                    "input-group-size-icon",
                                    state.entity_id(),
                                ))
                                .child(icon(IconName::Search)),
                            )
                    })),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "input-group",
        "Input Group",
        "Compose inputs and textareas with icons, text, actions, and toolbars in one frame.",
        InputGroupSection::view(window, cx),
    ));
}
