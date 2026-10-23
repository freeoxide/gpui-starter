use super::super::demos::{DemoId, ShaderParams, demo};
use super::*;

#[gpui_kit::test]
#[ignore = "needs a working wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn renders_a_non_uniform_gradient_frame(_cx: &mut gpui_kit::TestAppContext) {
    let engine = std::sync::Arc::new(WgpuEngine::new().await.expect("wgpu engine"));
    let runtime = ShaderRuntime::new(engine);

    let spec = demo(DemoId::Gradient);
    let params = ShaderParams {
        time: 0.5,
        delta: 1.0 / 60.0,
        slider: 0.5,
        resolution: (spec.width as f32, spec.height as f32),
        mouse: (0.5, 0.5),
    };
    let image = runtime.render_frame(spec, params).expect("render frame");

    let bytes = image.as_bytes(0).expect("frame bytes");
    assert_eq!(bytes.len(), (spec.width * spec.height * 4) as usize);
    let pixel = |x: u32, y: u32| {
        let at = ((y * spec.width + x) * 4) as usize;
        [bytes[at], bytes[at + 1], bytes[at + 2]]
    };
    let corner = pixel(4, 4);
    let center = pixel(spec.width / 2, spec.height / 2);
    assert_ne!(corner, center, "gradient frame is uniform");
}
