//! Removable component gallery; removal = delete this directory and revert
//! seams in lib.rs, page.rs, app_root/state.rs, e2e_navigation.rs.

mod demo;
mod page;
pub(crate) mod registry;
mod sections;

pub use page::GalleryPage;
