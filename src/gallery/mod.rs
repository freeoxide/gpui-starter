//! Removable component gallery bolted onto the app shell. `GalleryPage` is
//! the only public surface; everything else stays private to the crate.
//! Removal = delete this directory and revert its five seam touches: the
//! `pub mod gallery;` line in lib.rs, the Page::Gallery arms and all()
//! entry in shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the
//! AppRoot wiring in shell/root/app_root/state.rs, and the page count in
//! tests/e2e_navigation.rs.

mod page;
pub(crate) mod registry;
mod sections;

pub use page::GalleryPage;
