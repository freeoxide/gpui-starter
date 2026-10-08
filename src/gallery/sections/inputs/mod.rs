//! Input-family sections, ported from the upstream stories: Input (with the
//! standalone `input` example folded in), Input Group, Number Input, OTP
//! Input, Speech, Stepper, and Textarea, plus the atomic-inline-token
//! composer the Input and Textarea stories share.

mod demo;
mod input;
mod input_group;
mod number_input;
mod otp_input;
mod speech;
mod stepper;
mod textarea;
mod tokens;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    input::register(sections, window, cx);
    input_group::register(sections, window, cx);
    number_input::register(sections, window, cx);
    otp_input::register(sections, window, cx);
    speech::register(sections, window, cx);
    stepper::register(sections, window, cx);
    textarea::register(sections, window, cx);
}
