//! Shared scaffolding for the input-family sections: the area's shared
//! action set and the toolbar items built on it.

use gpui_kit::component::{
    Size,
    button::{Button, DropdownButton},
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle};

pub(crate) use crate::gallery::demo::{demo_toolbar, section};

/// Shared check-menu toggles; each section offers only the variants it
/// demonstrates, so the rest stay unreachable.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_inputs, no_json)]
pub(crate) enum DemoToggle {
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
    OtpMasked,
    StepperDisabled,
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

/// The size switcher over this area's toggle namespace.
pub(crate) fn size_dropdown(id: &'static str, size: Size) -> DropdownButton {
    demo::size_dropdown::<DemoToggle>(id, size)
}

/// The `Options` menu carrying the knobs one story exposes.
pub(crate) fn options_dropdown(
    id: &'static str,
    items: Vec<(&'static str, bool, DemoToggle)>,
) -> DropdownButton {
    DropdownButton::new(id)
        .button(Button::new(format!("{id}-trigger")).label("Options"))
        .dropdown_menu(move |menu, _, _| {
            items.iter().fold(menu, |menu, (label, checked, action)| {
                menu.menu_with_check(*label, *checked, Box::new(action.clone()))
            })
        })
}
