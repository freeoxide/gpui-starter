//! Removable component gallery bolted onto the app shell. `GalleryPage` is
//! the only public surface; everything else stays private to the crate.
//! Removal = delete this directory and revert the five seam touches listed
//! in `.z-gpui-workflow/STATE.md` (lib.rs mod line, Page::Gallery arms,
//! VALID_HOSTS entry, AppRoot wiring, e2e page count).

mod page;
pub(crate) mod registry;
mod sections;

pub use page::GalleryPage;
