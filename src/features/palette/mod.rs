//! Fuzzy filtering and the entry model behind the command palette in
//! [`crate::features::command_palette`].

mod filter;
mod items;

pub use filter::ItemFilter;
pub use items::PaletteEntry;
