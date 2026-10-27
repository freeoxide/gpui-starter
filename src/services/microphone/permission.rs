use super::types::MicrophonePermissionState;

pub(crate) fn status() -> MicrophonePermissionState {
    #[cfg(target_os = "macos")]
    {
        macos_status()
    }
    #[cfg(all(not(target_os = "macos"), not(target_family = "wasm")))]
    {
        MicrophonePermissionState::Authorized
    }
    #[cfg(target_family = "wasm")]
    {
        MicrophonePermissionState::Unsupported
    }
}

pub(crate) async fn request() -> MicrophonePermissionState {
    #[cfg(target_os = "macos")]
    {
        macos_request().await
    }
    #[cfg(not(target_os = "macos"))]
    {
        MicrophonePermissionState::Unsupported
    }
}

pub(crate) fn open_microphone_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
            .spawn()
            .map(|_| ())
            .map_err(|err| err.to_string())
    }
    #[cfg(all(not(target_os = "macos"), target_os = "windows"))]
    {
        open::that_detached("ms-settings:privacy-microphone").map_err(|err| err.to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("unsupported on this platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn bundled_runtime() -> Result<(), String> {
    use objc2_foundation::NSBundle;

    match NSBundle::mainBundle().bundleIdentifier() {
        Some(_) => Ok(()),
        None => Err(
            "macOS bundle identifier missing; launch the bundled app from scripts/macos-dev-app.sh instead of raw cargo run"
                .to_string(),
        ),
    }
}

#[cfg(target_os = "macos")]
fn macos_status() -> MicrophonePermissionState {
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    if let Err(reason) = bundled_runtime() {
        return MicrophonePermissionState::Unavailable(reason);
    }
    // SAFETY: extern static read; AVFoundation initializes the constant
    // before main, so the reference is always valid.
    let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
        return MicrophonePermissionState::Unavailable("AVMediaTypeAudio unavailable".to_string());
    };
    // SAFETY: AVMediaTypeAudio is one of the two media types the API accepts;
    // a NULL static would have been rejected above.
    let raw = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
    match raw {
        AVAuthorizationStatus::Authorized => MicrophonePermissionState::Authorized,
        AVAuthorizationStatus::Denied => MicrophonePermissionState::Denied,
        AVAuthorizationStatus::Restricted => MicrophonePermissionState::Restricted,
        AVAuthorizationStatus::NotDetermined => MicrophonePermissionState::NotDetermined,
        other => MicrophonePermissionState::Unavailable(format!("{other:?}")),
    }
}

#[cfg(target_os = "macos")]
async fn macos_request() -> MicrophonePermissionState {
    use std::cell::RefCell;

    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};

    if let Err(reason) = bundled_runtime() {
        return MicrophonePermissionState::Unavailable(reason);
    }
    // SAFETY: extern static read; AVFoundation initializes the constant
    // before main, so the reference is always valid.
    let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
        return MicrophonePermissionState::Unavailable("AVMediaTypeAudio unavailable".to_string());
    };
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    // SAFETY: Apple invokes the completion handler exactly once; the
    // RefCell take prevents a double-send if it ever fired twice.
    unsafe {
        let tx = RefCell::new(Some(tx));
        let block = RcBlock::new(move |granted: Bool| {
            if let Some(tx) = tx.take()
                && tx.send(granted.as_bool()).is_err()
            {
                tracing::warn!(
                    target: "gpui_starter::microphone",
                    "microphone permission receiver dropped"
                );
            }
        });
        AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &block);
    }
    match rx.await {
        Ok(true) => macos_status(),
        Ok(false) => MicrophonePermissionState::Denied,
        Err(err) => MicrophonePermissionState::Unavailable(err.to_string()),
    }
}

#[cfg(test)]
#[path = "permission.test.rs"]
mod permission_test;
