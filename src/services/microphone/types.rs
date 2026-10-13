use gpui_kit::SharedString;
use gpui_kit::component::select::SelectItem;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureStatus {
    Idle,
    Starting,
    Capturing { device: String },
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MicrophonePermissionState {
    Unknown,
    NotDetermined,
    Authorized,
    Denied,
    Restricted,
    Unsupported,
    Unavailable(String),
}

impl MicrophonePermissionState {
    pub fn label(&self) -> SharedString {
        let key = match self {
            MicrophonePermissionState::Unknown => "microphone_permission_unknown",
            MicrophonePermissionState::NotDetermined => "microphone_permission_not_determined",
            MicrophonePermissionState::Authorized => "microphone_permission_authorized",
            MicrophonePermissionState::Denied => "microphone_permission_denied",
            MicrophonePermissionState::Restricted => "microphone_permission_restricted",
            MicrophonePermissionState::Unsupported => "microphone_permission_unsupported",
            MicrophonePermissionState::Unavailable(_) => "microphone_permission_unavailable",
        };
        crate::i18n::localize(key).into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceOption {
    pub id: String,
    pub name: String,
}

impl SelectItem for DeviceOption {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &String {
        &self.id
    }
}

/// "-12.3 dB" / "-∞ dB" for a 0..=1 amplitude. Pure, testable.
pub fn dbfs(amplitude: f32) -> String {
    if amplitude <= 1e-5 {
        "-∞ dB".to_string()
    } else {
        format!("{:.1} dB", 20.0 * amplitude.log10())
    }
}

#[cfg(test)]
#[path = "types.test.rs"]
mod types_test;
