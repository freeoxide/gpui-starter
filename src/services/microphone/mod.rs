//! Removable microphone capture feature. The `microphone` service and
//! `MicrophonePage` are the only public surfaces. Removal = delete this
//! directory and src/features/pages/microphone.rs, then revert the seam
//! touches: the pub mod + re-export lines in services/mod.rs and
//! features/pages/mod.rs, the microphone entry in lib.rs's services re-export,
//! the initialize call in app/init.rs, the Page::Microphone arms and all()
//! entry in shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the
//! AppRoot wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the mic_* fields in state/config_store.rs and both
//! snapshot_tests.rs snapshots, the microphone_* + nav_microphone keys in
//! locales/en.yml and locales/zh-CN.yml, the cpal/objc2-av-foundation lines
//! and regenerated Cargo.lock in Cargo.toml (keep the cpal line if the audio
//! playback feature still uses it), libasound2-dev in
//! scripts/install-linux-deps.sh, and NSMicrophoneUsageDescription in
//! scripts/macos-dev-app.sh (the release plist's copy belongs to the release
//! bundling plan, not this one).

#[cfg(not(target_family = "wasm"))]
mod capture;
mod controller;
mod demo;
mod meter;
mod permission;
mod types;

pub use controller::MicrophoneState;
#[cfg(test)]
pub(crate) use controller::force_demo_for_tests;
#[cfg(test)]
pub(crate) use controller::seed_devices_for_tests;
pub use controller::{
    MicrophoneSnapshot, demo_only, devices, gain_db, initialize, meter, open_microphone_settings,
    permission_state, refresh_devices, refresh_permission, request_permission, select_device,
    set_gain_db, set_gain_db_live, snapshot, start, status, stop,
};
pub use demo::DemoSignal;
pub use meter::LevelMeter;
pub use types::{CaptureStatus, DeviceOption, InputDeviceInfo, MicrophonePermissionState, dbfs};
