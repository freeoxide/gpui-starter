// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use std::sync::Arc;

use gpui_kit::{AppContext as _, TestAppContext};

use super::ViewerState;

fn state(cx: &mut TestAppContext) -> gpui_kit::Entity<ViewerState> {
    cx.new(|cx| ViewerState::new(cx))
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

#[gpui_kit::test]
fn orbit_clamps_pitch_and_keeps_yaw_unbounded(cx: &mut TestAppContext) {
    let state = state(cx);
    state.update(cx, |s, cx| s.orbit_by(0.5, -10.0, cx));
    let (pitch, yaw) = cx.read(|cx| {
        let s = state.read(cx);
        (s.pitch, s.yaw)
    });
    assert!(near(pitch, -1.45));
    // Default yaw is 0.6; the orbit adds 0.5 on top.
    assert!(near(yaw, 1.1));
}

#[gpui_kit::test]
fn zoom_clamps_distance(cx: &mut TestAppContext) {
    let state = state(cx);
    state.update(cx, |s, cx| s.zoom_by(0.001, cx));
    assert!(near(cx.read(|cx| state.read(cx).distance), 1.6));
    state.update(cx, |s, cx| s.zoom_by(100.0, cx));
    assert!(near(cx.read(|cx| state.read(cx).distance), 16.0));
}

#[gpui_kit::test]
fn install_mesh_sets_pending_swap_until_a_frame_lands(cx: &mut TestAppContext) {
    let state = state(cx);
    let mesh = Arc::new(super::super::obj::builtin_torus());
    state.update(cx, |s, cx| s.install_mesh(mesh, "test.obj", cx));
    assert!(cx.read(|cx| state.read(cx).mesh_name() == "test.obj"));
    assert!(cx.read(|cx| state.read(cx).take_render_job().swap_mesh.is_some()));
    assert!(
        cx.read(|cx| state.read(cx).take_render_job().swap_mesh.is_some()),
        "pending swap survives until a frame lands"
    );

    let blank = crate::features::wgpu::bgra_to_render_image(1, 1, vec![0, 0, 0, 0]);
    state.update(cx, |s, cx| {
        s.frame_done(crate::platform::clock::Instant::now(), blank, cx)
    });
    assert!(cx.read(|cx| state.read(cx).take_render_job().swap_mesh.is_none()));
}

#[gpui_kit::test]
fn paused_state_only_wants_frames_after_a_kick(cx: &mut TestAppContext) {
    let state = state(cx);
    state.update(cx, |s, cx| {
        if s.is_auto_rotating() {
            s.toggle_auto_rotate(cx);
        }
        s.clear_error(cx);
    });
    // Fresh state still owes its first frame.
    state.update(cx, |s, cx| {
        s.mark_frame_started(crate::platform::clock::Instant::now(), cx)
    });
    assert!(!cx.read(|cx| state.read(cx).wants_frame()));

    state.update(cx, |s, cx| s.orbit_by(0.1, 0.1, cx));
    assert!(cx.read(|cx| state.read(cx).wants_frame()));
    state.update(cx, |s, cx| {
        s.mark_frame_started(crate::platform::clock::Instant::now(), cx)
    });
    assert!(!cx.read(|cx| state.read(cx).wants_frame()));
}

#[gpui_kit::test]
fn viewport_change_quantizes_and_fires_once(cx: &mut TestAppContext) {
    let state = state(cx);
    assert!(state.update(cx, |s, cx| s.viewport_changed((300, 200), cx)));
    assert!(!state.update(cx, |s, cx| s.viewport_changed((303, 207), cx)));
    assert!(!state.update(cx, |s, cx| s.viewport_changed((297, 200), cx)));
    assert!(state.update(cx, |s, cx| s.viewport_changed((310, 210), cx)));
    // Degenerate sizes clamp to the minimum instead of zero targets.
    assert!(state.update(cx, |s, cx| s.viewport_changed((0, 0), cx)));
    let job = cx.read(|cx| state.read(cx).take_render_job());
    assert_eq!((job.width, job.height), (64, 64));
}

#[gpui_kit::test]
fn frame_done_advances_spin_only_while_auto_rotating(cx: &mut TestAppContext) {
    let state = state(cx);
    state.update(cx, |s, cx| {
        if s.is_auto_rotating() {
            s.toggle_auto_rotate(cx);
        }
    });
    let blank = crate::features::wgpu::bgra_to_render_image(1, 1, vec![0, 0, 0, 0]);
    state.update(cx, |s, cx| {
        s.frame_done(crate::platform::clock::Instant::now(), blank.clone(), cx)
    });
    assert!(near(cx.read(|cx| state.read(cx).spin), 0.0));

    state.update(cx, |s, cx| s.toggle_auto_rotate(cx));
    state.update(cx, |s, cx| {
        s.frame_done(crate::platform::clock::Instant::now(), blank, cx)
    });
    assert!(cx.read(|cx| state.read(cx).spin) > 0.0);
}
