//! Removable guided-tour overlay. Public surface: [`target`] to mark a
//! spotlight target, [`TourStep`]/[`TourDefinition`]/[`demo_tour`] to author
//! tours, [`start`]/[`start_tour`]/[`dismiss`] to run them, [`resolve`] to
//! read published bounds, [`initialize`] to bind the tour keys. Removal =
//! delete this directory and revert the seam touches: the `pub mod tutorial;`
//! line in features/mod.rs, the AppRoot wiring (state.rs field + TourHandle
//! global, render.rs root wrap + Gallery row wrap), the home.rs and
//! gallery/page.rs target wraps + tour button, the StartTour command in
//! services/commands.rs, the tutorial::initialize call and "tutorial"
//! capability in app/init.rs, the tutorial_* keys in both locale catalogs,
//! and the tour bullet in docs/accessibility-checklist.md.

mod controller;
pub(crate) use controller::TourHandle;
mod mask;
mod target;
mod tour;

pub use controller::{
    TourBack, TourController, TourDismiss, TourNext, dismiss, initialize, start, start_tour,
};
pub use target::{TourTarget, resolve, target};
pub use tour::{TourDefinition, TourProgress, TourStep, demo_tour};
