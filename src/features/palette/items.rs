//! The contract every palette row must satisfy to be scored by [`crate::features::palette::ItemFilter`].

/// A selectable, filterable palette row. `name` is the primary search target;
/// `description` is a fallback target shown as the secondary line.
pub trait PaletteEntry {
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str> {
        None
    }
}
