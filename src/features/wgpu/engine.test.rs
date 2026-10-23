use super::*;

#[gpui_kit::test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn clears_and_reads_back_a_texture(_cx: &mut gpui_kit::TestAppContext) {
    let engine = WgpuEngine::new().await.expect("wgpu engine");
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("wgpu-engine-smoke"),
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
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    }),
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
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("wgpu-engine-overlap"),
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
            label: Some("wgpu-engine-overlap-encode"),
        });
    {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wgpu-engine-overlap-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
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
