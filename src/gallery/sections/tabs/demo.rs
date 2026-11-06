//! Shared scaffolding for the tabs-area sections: the area's shared action
//! set and the toolbar items built on it.

use gpui_kit::component::{Size, button::DropdownButton};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle};

pub(crate) use crate::gallery::demo::{demo_toolbar, section};

/// Knobs of the tabs, scrollbar, and per-section options toolbars.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tabs, no_json)]
pub(crate) enum DemoToggle {
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
    TabsMaxWidthUnlimited,
    TabsMaxWidth60,
    TabsMaxWidth90,
    TabsMaxWidth120,
    TabsMaxWidth160,
    TabsMoreMenu,
    ScrollbarDatasetStandard,
    ScrollbarDatasetWide,
    ScrollbarDatasetStress,
    ScrollbarDatasetShort,
}

impl DemoToggle {
    /// The size this toggle selects.
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
