//! Removable component gallery; removal = delete this directory and revert
//! seams in lib.rs, shell/page.rs, app_root, the tutorial tour, e2e_navigation.rs.

mod demo;
mod page;
pub(crate) mod registry;
mod sections;

pub use page::GalleryPage;
