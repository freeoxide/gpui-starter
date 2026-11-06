//! Shared scaffolding for the table-family sections: the area's shared
//! action set, the size switcher built on it, and the deterministic
//! stand-in for the `fake`/`rand` crates the upstream stories generated
//! their demo data with.

use std::sync::{LazyLock, Mutex};

use gpui_kit::component::{Size, button::DropdownButton};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle};

pub(crate) use crate::gallery::demo::{demo_toolbar, section, size_label};

/// Size knob shared by the Table and DataTable toolbars; the data table also
/// offers the 48px custom size its context menu sets.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
pub(crate) enum DemoToggle {
    Size48Px,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

impl DemoToggle {
    /// The size this toggle selects.
    pub(crate) fn size(&self) -> Option<Size> {
        match self {
            DemoToggle::Size48Px => Some(Size::Size(px(48.))),
            DemoToggle::SizeXSmall => Some(Size::XSmall),
            DemoToggle::SizeSmall => Some(Size::Small),
            DemoToggle::SizeMedium => Some(Size::Medium),
            DemoToggle::SizeLarge => Some(Size::Large),
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

// Neither `fake` nor `rand` is an app dependency, so the demo data comes from
// one shared deterministic sequence instead of those crates.
static RNG: LazyLock<Mutex<Lcg>> = LazyLock::new(|| Mutex::new(Lcg(0x4D59_5F53_4545_4401)));

struct Lcg(u64);

impl Lcg {
    fn next_f64(&mut self, min: f64, max: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        min + (max - min) * unit
    }

    // The unit draw is strictly below 1.0, so the truncation stays in range.
    fn next_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        min + (self.next_f64(0., 1.) * (max - min) as f64) as usize
    }
}

/// A deterministic value in `min..max`, standing in for the `fake` crate's
/// ranged draws.
pub(crate) fn rng_f64(min: f64, max: f64) -> f64 {
    RNG.lock().unwrap().next_f64(min, max)
}

/// A deterministic value in `min..max`, standing in for `rand::random::<usize>()`.
pub(crate) fn rng_usize(min: usize, max: usize) -> usize {
    RNG.lock().unwrap().next_usize(min, max)
}

/// A deterministic pick from `items`, standing in for `SliceRandom::choose`.
pub(crate) fn rng_pick<T>(items: &[T]) -> Option<&T> {
    if items.is_empty() {
        None
    } else {
        items.get(rng_usize(0, items.len()))
    }
}
