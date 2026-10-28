use std::time::Duration;

use super::*;
use crate::features::wgpu::fence::FenceError;

/// Pins the init seam for the test's lifetime; resets to Real on drop,
/// including on panic.
struct SeamGuard;

impl SeamGuard {
    fn arm(seam: super::InitSeam) -> Self {
        super::set_init_seam(seam);
        Self
    }
}

impl Drop for SeamGuard {
    fn drop(&mut self) {
        super::set_init_seam(super::InitSeam::Real);
    }
}

/// Creates an 8x8 RGBA8 texture cleared to `clear`; the shared body of
/// the readback tests.
fn cleared_texture(engine: &WgpuEngine, label: &'static str, clear: wgpu::Color) -> wgpu::Texture {
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut encoder = engine
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("wgpu-engine-smoke-encode"),
        });
    {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wgpu-engine-smoke-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    engine.queue().submit(Some(encoder.finish()));
    texture
}

#[test]
fn fenced_init_times_out_when_the_adapter_never_answers() {
    let _seam = SeamGuard::arm(super::InitSeam::Park);
    let started = std::time::Instant::now();
    let err = WgpuEngine::new_fenced_with_deadline(Duration::from_millis(250))
        .err()
        .expect("a parked init must trip the fence");
    assert!(matches!(
        err.downcast_ref::<FenceError>(),
        Some(FenceError::InitTimeout { .. })
    ));
    assert_eq!(
        err.to_string(),
        "wgpu adapter/device init timed out after 0s"
    );
    assert!(started.elapsed() >= Duration::from_millis(250));
}

#[test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
fn fenced_init_and_readback_round_trip() {
    let engine = std::sync::Arc::new(WgpuEngine::new_fenced().expect("fenced init"));
    let texture = cleared_texture(&engine, "wgpu-engine-fence-smoke", wgpu::Color::BLUE);
    let reading = engine.clone();
    let bytes = engine
        .render_fenced(move || reading.read_texture_bytes(&texture, 8, 8))
        .expect("fenced readback");
    assert_eq!(bytes.len(), 8 * 8 * 4);
    assert_eq!(&bytes[..4], &[0, 0, 255, 255]);
    assert!(!engine.poisoned());
}

#[test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
fn missed_render_deadline_poisons_the_engine() {
    let engine = std::sync::Arc::new(WgpuEngine::new_fenced().expect("fenced init"));
    let err = engine
        .render_fenced_with_deadline(Duration::from_millis(150), || -> anyhow::Result<()> {
            // Stand-in for readback wedged inside the driver; the fence
            // trips while this parks.
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            let _ = rx.recv();
            unreachable!("the live sender keeps recv blocking")
        })
        .expect_err("parked render work must trip the fence");
    assert!(matches!(
        err.downcast_ref::<FenceError>(),
        Some(FenceError::Deadline { .. })
    ));
    assert!(err.to_string().contains("engine poisoned"));
    assert!(engine.poisoned());

    let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let marker = ran.clone();
    let err = engine
        .render_fenced(move || {
            marker.store(true, std::sync::atomic::Ordering::Release);
            anyhow::Ok(())
        })
        .expect_err("a poisoned engine must refuse fenced work");
    assert!(matches!(
        err.downcast_ref::<FenceError>(),
        Some(FenceError::Poisoned)
    ));
    assert!(!ran.load(std::sync::atomic::Ordering::Acquire));
}

#[gpui_kit::test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn clears_and_reads_back_a_texture(_cx: &mut gpui_kit::TestAppContext) {
    let engine = WgpuEngine::new().await.expect("wgpu engine");
    let texture = cleared_texture(
        &engine,
        "wgpu-engine-smoke",
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        },
    );

    let bytes = engine.read_texture_bytes(&texture, 8, 8).expect("readback");

    assert_eq!(bytes.len(), 8 * 8 * 4);
    for row in bytes.chunks_exact(8 * 4) {
        assert_eq!(&row[..4], &[0, 0, 255, 255]);
    }
}

/// The readback buffer is reused per size; overlapping calls on one engine
/// must serialize behind its lock instead of double-mapping that buffer.
#[gpui_kit::test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn overlapping_readbacks_serialize_on_one_engine(_cx: &mut gpui_kit::TestAppContext) {
    let engine = std::sync::Arc::new(WgpuEngine::new().await.expect("wgpu engine"));
    let texture = cleared_texture(&engine, "wgpu-engine-overlap", wgpu::Color::GREEN);

    let first = std::thread::spawn({
        let engine = engine.clone();
        let texture = texture.clone();
        move || engine.read_texture_bytes(&texture, 8, 8)
    });
    let second = std::thread::spawn({
        let engine = engine.clone();
        let texture = texture.clone();
        move || engine.read_texture_bytes(&texture, 8, 8)
    });
    for handle in [first, second] {
        let bytes = handle.join().expect("readback thread").expect("readback");
        assert_eq!(bytes.len(), 8 * 8 * 4);
        assert_eq!(&bytes[..4], &[0, 255, 0, 255]);
    }
}
