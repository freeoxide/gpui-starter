//! Shared scaffolding for the button-family sections: the common demo-state
//! knobs and the toolbar dropdowns that drive them.

use gpui_kit::component::{
    ActiveTheme as _, Size,
    button::{Button, DropdownButton},
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle};

pub(crate) use crate::gallery::demo::{demo_toolbar, section, size_label};

/// Knobs the upstream stories expose through their options menu; several
/// sections drive every control they render from one copy of this state.
#[derive(Default)]
pub(crate) struct DemoState {
    pub(crate) disabled: bool,
    pub(crate) loading: bool,
    pub(crate) selected: bool,
    pub(crate) compact: bool,
    pub(crate) size: Size,
}

/// Shared check-menu toggles for the button-family sections. Each section
/// offers only the variants it demonstrates, so the rest stay unreachable.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_buttons, no_json)]
pub(crate) enum DemoToggle {
    Disabled,
    Loading,
    Selected,
    Compact,
    Shadow,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

impl DemoToggle {
    /// The size this toggle selects; size-only sections listen for these.
    pub(crate) fn size(&self) -> Option<Size> {
        match self {
            DemoToggle::SizeXSmall => Some(Size::XSmall),
            DemoToggle::SizeSmall => Some(Size::Small),
            DemoToggle::SizeMedium => Some(Size::Medium),
            DemoToggle::SizeLarge => Some(Size::Large),
            _ => None,
        }
    }
}

impl SizeToggle for DemoToggle {
    fn xsmall() -> Self {
        Self::SizeXSmall
    }
    fn small() -> Self {
        Self::SizeSmall
    }
    fn medium() -> Self {
        Self::SizeMedium
    }
    fn large() -> Self {
        Self::SizeLarge
    }
}

impl DemoState {
    /// Apply one shared toggle; returns whether render state changed. The
    /// `Shadow` variant flips the global theme flag instead, as upstream did.
    pub(crate) fn apply(&mut self, toggle: &DemoToggle, window: &mut Window, cx: &mut App) -> bool {
        match toggle {
            DemoToggle::Shadow => {
                gpui_kit::component::Theme::update(cx, |theme| theme.shadow = !theme.shadow);
                window.refresh();
                false
            }
            DemoToggle::Disabled => {
                self.disabled = !self.disabled;
                true
            }
            DemoToggle::Loading => {
                self.loading = !self.loading;
                true
            }
            DemoToggle::Selected => {
                self.selected = !self.selected;
                true
            }
            DemoToggle::Compact => {
                self.compact = !self.compact;
                true
            }
            DemoToggle::SizeXSmall => {
                self.size = Size::XSmall;
                true
            }
            DemoToggle::SizeSmall => {
                self.size = Size::Small;
                true
            }
            DemoToggle::SizeMedium => {
                self.size = Size::Medium;
                true
            }
            DemoToggle::SizeLarge => {
                self.size = Size::Large;
                true
            }
        }
    }
}

/// The size switcher over this area's toggle namespace.
pub(crate) fn size_dropdown(id: &'static str, size: Size) -> DropdownButton {
    demo::size_dropdown::<DemoToggle>(id, size)
}

/// The `Options` menu carrying the shared demo knobs.
pub(crate) fn options_dropdown(id: &'static str, demo: &DemoState) -> DropdownButton {
    let disabled = demo.disabled;
    let loading = demo.loading;
    let selected = demo.selected;
    let compact = demo.compact;
    DropdownButton::new(id)
        .button(Button::new(format!("{id}-trigger")).label("Options"))
        .dropdown_menu(move |menu, _, cx| {
            menu.menu_with_check("Disabled", disabled, Box::new(DemoToggle::Disabled))
                .menu_with_check("Loading", loading, Box::new(DemoToggle::Loading))
                .menu_with_check("Selected", selected, Box::new(DemoToggle::Selected))
                .menu_with_check("Compact", compact, Box::new(DemoToggle::Compact))
                .separator()
                .menu_with_check("Shadow", cx.theme().shadow, Box::new(DemoToggle::Shadow))
        })
}
