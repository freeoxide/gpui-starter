//! Recipes section, ported from the tested consumer recipes in the upstream
//! `ai_recipes` example crate: bootstrap, controlled value, the settings
//! form, and command control.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    form::{Field, Form},
    input::{Input, InputEvent, InputState},
    radio::RadioGroup,
    switch::Switch,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

/// The bootstrap recipe's view; the gallery page plays the role of the
/// `Root` wrap its `run()` installs around a window's root view.
struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w_full().child("My application")
    }
}

/// The controlled-value recipe: the owner holds the value, the Checkbox only
/// renders it back and reports requests through `on_change`.
struct ControlledCheckbox {
    checked: bool,
}

impl ControlledCheckbox {
    fn new() -> Self {
        Self { checked: false }
    }

    fn is_checked(&self) -> bool {
        self.checked
    }
}

impl Render for ControlledCheckbox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Checkbox::new("recipes-marketing-emails")
            .label("Receive product updates")
            .checked(self.checked)
            .on_change(cx.listener(|this, checked, _, cx| {
                this.checked = *checked;
                cx.notify();
            }))
    }
}

/// The settings recipe: retained input state, a change subscription that
/// survives redraws, controlled checkbox/switch/radio values, a typed footer.
struct ProfileSettings {
    name: Entity<InputState>,
    preview: SharedString,
    changes: usize,
    enabled: bool,
    remember: bool,
    delivery: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ProfileSettings {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let subscription = cx.subscribe_in(&name, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.preview = state.read(cx).value().to_string().into();
                this.changes += 1;
                cx.notify();
            }
        });
        Self {
            name,
            preview: "".into(),
            changes: 0,
            enabled: false,
            remember: false,
            delivery: Some(0),
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for ProfileSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .gap_3()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("Profile")
            .child(
                Form::new()
                    .child(Field::new().label("Name").child(Input::new(&self.name)))
                    .child(Field::new().label("Preview").child(self.preview.clone()))
                    .child(
                        Field::new()
                            .label("Changes")
                            .child(format!("{}", self.changes)),
                    )
                    .child(
                        Field::new().label_indent(false).child(
                            Checkbox::new("recipes-remember")
                                .label("Remember name")
                                .checked(self.remember)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.remember = *value;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        Field::new().label_indent(false).child(
                            Switch::new("recipes-enabled")
                                .label("Enable notifications")
                                .checked(self.enabled)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.enabled = *value;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        Field::new().label("Delivery").child(
                            RadioGroup::new("recipes-delivery")
                                .children(["Immediately", "Daily summary"])
                                .selected_index(self.delivery)
                                .on_change(cx.listener(|this, value, _, cx| {
                                    this.delivery = Some(*value);
                                    cx.notify();
                                })),
                        ),
                    )
                    .footer(
                        Button::new("recipes-about")
                            .label("About…")
                            .icon(IconName::Info)
                            .on_click(|_, window, cx| {
                                window.open_dialog(cx, |dialog, _, _| {
                                    dialog.title("About").child("A complete GPUI Kit window")
                                });
                            }),
                    ),
            )
    }
}

/// The command-control recipe: one definition of the primary command button.
fn primary_command() -> impl IntoElement {
    Button::new("recipes-save-command")
        .primary()
        .small()
        .label("Save changes")
}

pub struct RecipesSection {
    bootstrap: Entity<BootstrapView>,
    controlled: Entity<ControlledCheckbox>,
    profile: Entity<ProfileSettings>,
    _subscriptions: Vec<Subscription>,
}

impl RecipesSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let bootstrap = cx.new(|_| BootstrapView);
        let controlled = cx.new(|_| ControlledCheckbox::new());
        let profile = cx.new(|cx| ProfileSettings::new(window, cx));

        // The owner's value must stay visible in the section, not only inside
        // the checkbox, so redraw the host when the child entity changes.
        let observer = cx.observe(&controlled, |_, _, cx| cx.notify());

        Self {
            bootstrap,
            controlled,
            profile,
            _subscriptions: vec![observer],
        }
    }
}

impl Render for RecipesSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.controlled.read(cx).is_checked();

        v_flex()
            .id("recipes-section")
            .w_full()
            .p_4()
            .gap_4()
            .child(
                section("recipes-bootstrap", "Bootstrap")
                    .description(
                        "application() boots, init(cx) runs first, and the window's root view is a Root wrapping your view, so dialogs, sheets, and notifications render above it. This gallery page is built the same way.",
                    )
                    .v_flex()
                    .child(self.bootstrap.clone()),
            )
            .child(
                section("recipes-controlled-value", "Controlled value")
                    .description("The owner holds the value; the Checkbox renders it back and reports requests through on_change.")
                    .v_flex()
                    .child(self.controlled.clone())
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if checked {
                                "Owner state: checked"
                            } else {
                                "Owner state: unchecked"
                            }),
                    ),
            )
            .child(
                section("recipes-profile", "Settings form")
                    .description("Retained input state, a change subscription that survives unrelated redraws, controlled checkbox/switch/radio values, and a footer that owns its action.")
                    .w(rems(25.))
                    .v_flex()
                    .child(self.profile.clone()),
            )
            .child(
                section("recipes-command-control", "Command control")
                    .description("One definition of the window's primary command; every surface reuses it so variant, size, and label stay in sync.")
                    .child(primary_command()),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "recipes",
        "Recipes",
        "Tested consumer recipes: bootstrap, controlled values, retained form state, and the primary command button.",
        RecipesSection::view(window, cx),
    ));
}
