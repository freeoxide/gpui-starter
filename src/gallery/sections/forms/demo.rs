//! Shared scaffolding for the form-area sections: the area's shared action
//! set and the size switcher built on it.

use gpui_kit::component::{Size, button::DropdownButton};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle};

pub(crate) use crate::gallery::demo::{demo_toolbar, section};

/// Shared check-menu toggles; the size variants drive the shared size
/// switcher and the Form section also listens for its two layout toggles.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_forms, no_json)]
pub(crate) enum FormToggle {
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
    Horizontal,
    MultipleColumns,
}

impl FormToggle {
    /// The size this toggle selects; the size switcher entries map here.
    pub(crate) fn size(&self) -> Option<Size> {
        match self {
            FormToggle::SizeXSmall => Some(Size::XSmall),
            FormToggle::SizeSmall => Some(Size::Small),
            FormToggle::SizeMedium => Some(Size::Medium),
            FormToggle::SizeLarge => Some(Size::Large),
            _ => None,
        }
    }
}

impl SizeToggle for FormToggle {
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
    demo::size_dropdown::<FormToggle>(id, size)
}
