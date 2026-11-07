//! Removable guided-tour overlay. Public surface: [`target`] to mark a
//! spotlight target, [`start`] to run the demo tour, [`initialize`] to bind
//! the tour keys. Removal =
//! delete this directory and revert the seam touches: the `pub mod tutorial;`
//! line in features/mod.rs, the AppRoot wiring (state.rs field + TourHandle
//! global, render.rs root wrap + Gallery row wrap), the home.rs and
//! gallery/page.rs target wraps + tour button, the StartTour command in
//! services/commands.rs, the tutorial::initialize call and "tutorial"
//! capability in app/init.rs, the tutorial_* keys in both locale catalogs,
//! and the tour bullet in docs/accessibility-checklist.md.

mod controller;
mod mask;
mod target;
mod tour;

pub(crate) use controller::TourHandle;
pub use controller::{TourController, initialize, start};
pub use target::target;

pub(crate) const LOG: &str = "gpui_starter::tutorial";
