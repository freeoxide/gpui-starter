use super::*;

#[test]
fn status_label_is_non_empty_on_every_platform() {
    assert!(!status().label().is_empty());
}

#[cfg(all(not(target_os = "macos"), not(target_family = "wasm")))]
#[test]
fn native_non_macos_status_is_authorized() {
    assert_eq!(status(), MicrophonePermissionState::Authorized);
}

#[cfg(target_family = "wasm")]
#[test]
fn wasm_status_is_unsupported() {
    assert_eq!(status(), MicrophonePermissionState::Unsupported);
}

#[cfg(not(target_os = "macos"))]
#[gpui_kit::test]
async fn non_macos_request_is_unsupported(cx: &gpui_kit::TestAppContext) {
    let result = cx.spawn(async move |_| request().await).await;
    assert_eq!(result, MicrophonePermissionState::Unsupported);
}
