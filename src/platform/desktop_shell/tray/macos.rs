use gpui_kit::App;
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use super::LOG;

#[cfg(test)]
#[path = "macos.test.rs"]
mod macos_test;

// 36px is the macOS menu-bar glyph canvas at 2x.
const ICON_SIZE: usize = 36;

// Brand G from assets/app-icon/gpui-starter.svg (ring + crossbar, square
// dropped: only alpha survives templating), as canvas fractions at 80% fill.
const RING_INNER: f32 = 0.256;
const RING_OUTER: f32 = 0.4;
const BAR_X0: f32 = 0.603;
const BAR_X1: f32 = 0.9;
const BAR_Y0: f32 = 0.428;
const BAR_Y1: f32 = 0.572;

fn icon(size: usize) -> tray_icon::Icon {
    tray_icon::Icon::from_rgba(icon_pixels(size), size as u32, size as u32)
        .expect("tray icon pixel buffer matches the requested dimensions by construction")
}

fn icon_pixels(size: usize) -> Vec<u8> {
    let mut px = vec![0u8; size * size * 4];

    let s = size as f32;
    let (cx, cy) = (s * 0.5, s * 0.5);
    let r_inner = s * RING_INNER;
    let r_outer = s * RING_OUTER;
    let r_mid = (r_inner + r_outer) * 0.5;
    let (bx0, bx1, by0, by1) = (s * BAR_X0, s * BAR_X1, s * BAR_Y0, s * BAR_Y1);

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            let ring_a: f32 = if dist >= r_inner && dist <= r_outer {
                1.0
            } else if dist < r_inner {
                (dist - (r_inner - 1.0)).clamp(0.0, 1.0)
            } else {
                (1.0 - (dist - r_outer)).clamp(0.0, 1.0)
            };
            // The mouth removes the ring's right ±45°; feathered over ~1px
            // of arc length at the ring's mid radius.
            let arc_past_mouth = (dy.atan2(dx).abs() - std::f32::consts::FRAC_PI_4) * r_mid;
            let mouth_a = (0.5 + arc_past_mouth).clamp(0.0, 1.0);

            let out_x = (bx0 - fx).max(fx - bx1).max(0.0);
            let out_y = (by0 - fy).max(fy - by1).max(0.0);
            let bar_d = (out_x * out_x + out_y * out_y).sqrt();
            let bar_a = (1.0 - bar_d).clamp(0.0, 1.0);

            let i = (y * size + x) * 4;
            px[i + 3] = ((ring_a * mouth_a).max(bar_a) * 255.0) as u8;
        }
    }

    px
}

pub(super) enum TrayAction {
    ToggleCharts { x: f64 },
}

pub(super) fn build_tray() -> Result<tray_icon::TrayIcon, String> {
    TrayIconBuilder::new()
        .with_icon(icon(ICON_SIZE))
        .with_icon_as_template(true)
        .with_tooltip(format!(
            "{}  (⌥Space)",
            crate::i18n::localize("menubar_charts_tooltip")
        ))
        .with_menu_on_left_click(false)
        .build()
        .map_err(|err| err.to_string())
}

// Left-button release toggles the charts popover at the click's x position;
// no menu is attached on macOS, so every other event is ignored.
fn action_from_tray_event(event: &TrayIconEvent) -> Option<TrayAction> {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        rect,
        ..
    } = event
    {
        tracing::info!(target: LOG, source = "tray_click", "Charts toggle trigger");
        return Some(TrayAction::ToggleCharts { x: rect.position.x });
    }
    None
}

// The drain parks on tray-icon's global crossbeam receiver for the process
// lifetime and forwards over flume: AsyncApp is !Send, so no cx.update here.
pub(super) fn spawn_event_drain(tx: flume::Sender<TrayAction>) -> Result<(), String> {
    std::thread::Builder::new()
        .name("gpui-tray-events".into())
        .spawn(move || {
            let tray_rx = TrayIconEvent::receiver();
            loop {
                let Ok(event) = tray_rx.recv() else {
                    return;
                };
                if let Some(action) = action_from_tray_event(&event) {
                    let _ = tx.send(action);
                }
            }
        })
        .map(|_| ())
        .map_err(|err| format!("failed to spawn gpui-tray-events thread: {err}"))
}

pub(super) fn apply_action(action: TrayAction, cx: &mut App) {
    let TrayAction::ToggleCharts { x } = action;
    crate::features::menubar_charts::toggle_popover(x, cx);
}
