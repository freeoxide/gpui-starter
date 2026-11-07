//! Removable audio playback service. `AudioEngine` is the shared decode +
//! output stack; the Audio page is its only consumer today. Removal = delete
//! this directory and revert its seam touches: the `pub mod audio;` line in
//! services/mod.rs, the `audio` name in lib.rs's services re-export, the
//! AudioPage module/pub use in features/pages/mod.rs, the Page::Audio arms
//! and all() entry in shell/page.rs, the AppRoot wiring in
//! shell/root/app_root/state.rs, the page count in tests/e2e_navigation.rs,
//! the audio_* and nav_audio keys in both locale files, the initialize call
//! in app/init.rs, and the hound/symphonia/cpal entries in Cargo.toml plus
//! regenerated Cargo.lock.

use gpui_kit::{App, AppContext as _};

mod decode;
mod demo;
mod engine;
#[cfg(not(target_family = "wasm"))]
mod output;
mod peaks;
mod transport;

pub use decode::{AudioError, DecodedAudio, decode_bytes};
pub(crate) use demo::{chords_wav, sweep_wav};
pub use engine::{AudioEngine, AudioMeta, OutputDevice, PlaybackStatus};
pub use peaks::{PEAK_COLUMNS, PeakColumn, WaveformPeaks};

#[cfg(test)]
#[path = "mod.test.rs"]
mod mod_test;

/// Registers the "audio_playback" capability; call once from app init. On native
/// the entry lands once the background device probe completes.
pub fn initialize(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        crate::capabilities::Capability::AUDIO_PLAYBACK,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some("cpal output requires a native host".into()),
            last_error: None,
        },
        cx,
    );
    #[cfg(not(target_family = "wasm"))]
    cx.spawn(async move |cx| {
        let devices = cx
            .background_spawn(async move { output::enumerate() })
            .await;
        let status = if devices.is_empty() {
            crate::capabilities::CapabilityStatus::degraded("no output device found")
        } else {
            crate::capabilities::CapabilityStatus::supported_enabled()
        };
        cx.update(|cx| {
            crate::capabilities::set(crate::capabilities::Capability::AUDIO_PLAYBACK, status, cx)
        });
    })
    .detach();
}
