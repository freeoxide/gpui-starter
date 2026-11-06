#![cfg_attr(windows, windows_subsystem = "windows")]

use gpui_starter::web::bootstrap;

fn main() {
    // The whole startup path lives in the lib so the wasm entry boots the same code.
    bootstrap();
}
