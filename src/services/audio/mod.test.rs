use gpui_kit::TestAppContext;

use crate::capabilities::Capability;

#[gpui_kit::test]
fn initialize_registers_the_playback_capability(cx: &mut TestAppContext) {
    cx.update(|cx| crate::audio::initialize(cx));
    cx.run_until_parked();
    cx.update(|cx| {
        let status = crate::capabilities::snapshot(cx)
            .get(&Capability::AUDIO_PLAYBACK)
            .cloned()
            .expect("audio_playback capability registered");
        #[cfg(target_family = "wasm")]
        {
            assert!(!status.supported);
            assert!(!status.enabled);
            assert!(status.reason.is_some());
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let devices = super::output::enumerate();
            assert!(status.supported);
            assert!(status.enabled);
            assert_eq!(status.degraded, devices.is_empty());
            assert_eq!(status.reason.is_some(), devices.is_empty());
        }
    });
}
