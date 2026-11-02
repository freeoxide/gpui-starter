use super::*;

#[test]
fn wgsl_parses_with_naga() {
    naga::front::wgsl::parse_str(&wgsl_source())
        .unwrap_or_else(|err| panic!("naga rejected the model shader: {err}"));
}

#[test]
fn wgsl_passes_full_naga_validation() {
    // wgpu validates parse plus module inside create_shader_module; the
    // source must clear both, not just the front end.
    let module = naga::front::wgsl::parse_str(&wgsl_source())
        .unwrap_or_else(|err| panic!("naga rejected the model shader: {err}"));
    let mut validator = naga::valid::Validator::new(Default::default(), Default::default());
    validator
        .validate(&module)
        .unwrap_or_else(|err| panic!("naga validation failed for the model shader: {err}"));
}

#[test]
fn wgsl_carries_both_entry_points() {
    let source = wgsl_source();
    assert!(source.contains("@vertex"));
    assert!(source.contains("@fragment"));
}

#[gpui_kit::test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn renders_the_builtin_torus_to_a_nonempty_frame(_cx: &mut gpui_kit::TestAppContext) {
    let engine = Arc::new(WgpuEngine::new().await.expect("wgpu engine"));
    let mut renderer =
        ModelRenderer::new(engine, &super::super::obj::builtin_torus()).expect("renderer");
    let bytes = renderer
        .render(RenderJob {
            width: 64,
            height: 64,
            yaw: 0.6,
            pitch: 0.4,
            distance: 4.0,
            spin: 0.0,
            swap_mesh: None,
        })
        .expect("render");
    assert_eq!(bytes.len(), 64 * 64 * 4);
    // The torus covers the center pixel with something other than the clear
    // color (0.02, 0.02, 0.04, 1).
    let at = (32 * 64 + 32) * 4;
    assert!([bytes[at], bytes[at + 1], bytes[at + 2]] != [5, 5, 10]);
}
