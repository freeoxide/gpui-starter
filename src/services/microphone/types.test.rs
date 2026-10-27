use super::*;

#[test]
fn dbfs_formats_mid_amplitude() {
    assert_eq!(dbfs(0.5), "-6.0 dB");
}

#[test]
fn dbfs_formats_silence_as_minus_infinity() {
    assert_eq!(dbfs(0.0), "-∞ dB");
}

#[test]
fn dbfs_formats_full_scale_as_zero() {
    assert_eq!(dbfs(1.0), "0.0 dB");
}

#[test]
fn permission_labels_resolve_from_the_catalog() {
    assert_eq!(
        MicrophonePermissionState::Unknown.label().as_ref(),
        "Unknown"
    );
    assert_eq!(
        MicrophonePermissionState::NotDetermined.label().as_ref(),
        "Not requested"
    );
    assert_eq!(
        MicrophonePermissionState::Authorized.label().as_ref(),
        "Granted"
    );
    assert_eq!(MicrophonePermissionState::Denied.label().as_ref(), "Denied");
    assert_eq!(
        MicrophonePermissionState::Restricted.label().as_ref(),
        "Restricted"
    );
    assert_eq!(
        MicrophonePermissionState::Unsupported.label().as_ref(),
        "Not applicable"
    );
    assert_eq!(
        MicrophonePermissionState::Unavailable("raw binary".to_string())
            .label()
            .as_ref(),
        "Unavailable"
    );
}
