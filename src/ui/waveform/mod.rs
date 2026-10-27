//! Removable shared waveform widget library. `ui::waveform` is the only
//! public surface; features mount it as a child element, nothing routes to
//! it. Removal = delete this directory, remove the `pub mod waveform;` line
//! from src/ui/mod.rs (sibling module lines stay), and remove the
//! rtrb/rustfft entries plus regenerated Cargo.lock in Cargo.toml.

mod element;
mod history;
mod paint;
mod ring;
mod spectrum;
mod state;

pub use element::{Waveform, WaveformElement};
pub use ring::SampleProducer;
pub use spectrum::{SPECTRUM_BARS, SpectrumFrame};
pub use state::{
    DEFAULT_RETENTION, DEFAULT_WINDOW, MAX_RETENTION, MIN_RETENTION, MIN_WINDOW, WaveformMode,
    WaveformState,
};
