use global_hotkey::GlobalHotKeyEvent;
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

enum TrayAction {
    Hotkey {
        id: u32,
    },
    ToggleCharts {
        rect: crate::features::menubar_charts::IconRect,
    },
}

/// Tray + global-hotkey entry point; call once from `main()` on macOS.
pub fn setup(cx: &mut App) {
    tracing::info!(target: LOG, "Setting up tray icon");

    let Ok(tray) = TrayIconBuilder::new()
        .with_icon(icon(ICON_SIZE))
        .with_icon_as_template(true)
        .with_tooltip(format!(
            "{}  (⌥Space)",
            crate::i18n::localize("menubar_charts_tooltip")
        ))
        .with_menu_on_left_click(false)
        .build()
    else {
        tracing::error!("failed to create system tray icon");
        return;
    };
    Box::leak(Box::new(tray));
    tracing::debug!(target: LOG, "Tray icon created");

    // Tray/hotkey threads park on blocking crossbeam recvs and forward
    // ticks over flume: `AsyncApp` is `!Send`, so they cannot `cx.update`.
    let (tx, rx) = flume::unbounded::<TrayAction>();
    let tx_hotkey = tx.clone();

    if let Err(e) = std::thread::Builder::new()
        .name("gpui-tray-events".into())
        .spawn(move || {
            let tray_rx = TrayIconEvent::receiver();
            loop {
                let Ok(event) = tray_rx.recv() else {
                    return;
                };
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } = event
                {
                    tracing::info!(target: LOG, source = "tray_click", "Charts toggle trigger");
                    let _ = tx.send(TrayAction::ToggleCharts {
                        rect: crate::features::menubar_charts::IconRect {
                            x: rect.position.x,
                            y: rect.position.y,
                            width: rect.size.width,
                            height: rect.size.height,
                        },
                    });
                }
            }
        })
    {
        tracing::error!(
            target: LOG,
            error = %e,
            "failed to spawn gpui-tray-events thread; tray clicks will not be delivered"
        );
    }

    if let Err(e) = std::thread::Builder::new()
        .name("gpui-hotkey-events".into())
        .spawn(move || {
            let hotkey_rx = GlobalHotKeyEvent::receiver();
            loop {
                let Ok(event) = hotkey_rx.recv() else {
                    return;
                };
                // The crate emits Pressed and Released per press; forwarding
                // both would fire the accelerator twice per tap.
                if event.state() == global_hotkey::HotKeyState::Pressed {
                    let _ = tx_hotkey.send(TrayAction::Hotkey { id: event.id() });
                }
            }
        })
    {
        tracing::error!(
            target: LOG,
            error = %e,
            "failed to spawn gpui-hotkey-events thread; global hotkey will not be delivered"
        );
    }

    cx.spawn(async move |cx| {
        loop {
            // Parked until a tray click or hotkey fires — no periodic wake-ups.
            let Ok(action) = rx.recv_async().await else {
                break;
            };
            match action {
                TrayAction::Hotkey { id } => {
                    let _ = cx.update(|cx| crate::shortcuts::dispatch_hotkey(id, cx));
                }
                TrayAction::ToggleCharts { rect } => {
                    let _ =
                        cx.update(|cx| crate::features::menubar_charts::toggle_popover(rect, cx));
                }
            }
        }
    })
    .detach();
}
