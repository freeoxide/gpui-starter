use super::*;

#[test]
fn every_wgsl_module_parses_with_naga() {
    for (name, source) in shader_modules() {
        naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|err| panic!("naga rejected {name}: {err}"));
    }
}

#[test]
fn every_wgsl_module_passes_full_naga_validation() {
    // wgpu validates parse plus module inside create_shader_module; the
    // concatenated demo sources must clear both, not just the front end.
    for (name, source) in shader_modules() {
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|err| panic!("naga rejected {name}: {err}"));
        let mut validator = naga::valid::Validator::new(Default::default(), Default::default());
        validator
            .validate(&module)
            .unwrap_or_else(|err| panic!("naga validation failed for {name}: {err}"));
    }
}

#[test]
fn shader_modules_carry_their_entry_points_and_bgra_swizzle() {
    let modules = shader_modules();
    let sources: std::collections::HashMap<&str, &str> = modules
        .iter()
        .map(|(name, source)| (*name, source.as_str()))
        .collect();

    assert!(sources["wgpu-particles-compute"].contains("@compute"));
    assert!(sources["wgpu-particles-draw"].contains("@vertex"));
    assert!(sources["wgpu-particles-draw"].contains("@fragment"));
    assert!(sources["wgpu-mesh"].contains("@vertex"));
    assert!(sources["wgpu-mesh"].contains("@fragment"));
    assert!(
        sources["wgpu-output-identity"].contains("vec4<f32>(c.b, c.g, c.r"),
        "the identity shader must swizzle to BGRA for RenderImage"
    );
    assert!(
        sources["wgpu-output-post"].contains("vec4<f32>(c.b, c.g, c.r"),
        "the post shader must swizzle to BGRA for RenderImage"
    );
}

#[test]
fn specs_cover_one_particle_demo_two_mesh_demos_and_one_post() {
    assert_eq!(SPECS.iter().filter(|(_, s)| s.uses_particles).count(), 1);
    assert_eq!(SPECS.iter().filter(|(_, s)| s.uses_mesh).count(), 2);
    assert_eq!(
        SPECS
            .iter()
            .filter(|(_, s)| s.post)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![DemoId::MeshPost]
    );
}

#[gpui_kit::test]
#[ignore = "needs a wgpu adapter (GPU or lavapipe); run with --ignored locally"]
async fn renders_a_distinct_frame_per_demo(_cx: &mut gpui_kit::TestAppContext) {
    let engine = std::sync::Arc::new(WgpuEngine::new().await.expect("wgpu engine"));
    let base = RenderJob {
        demo: DemoId::Particles,
        particle_count: 4096,
        dt: 1.0 / 60.0,
        elapsed: 0.5,
        width: 64,
        height: 64,
    };
    let mut renderer = SceneRenderer::new(engine, &base).expect("scene renderer");
    let cleared: [u8; 4] = [10, 5, 5, 255];
    let pixel = |bytes: &[u8], x: usize, y: usize| -> [u8; 4] {
        let at = (y * 64 + x) * 4;
        [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
    };

    let mut frames = Vec::new();
    for demo in [DemoId::Particles, DemoId::Mesh, DemoId::MeshPost] {
        let job = RenderJob { demo, ..base };
        let bytes = renderer.render(job).expect("render");
        assert_eq!(bytes.len(), 64 * 64 * 4);
        assert_ne!(pixel(&bytes, 32, 32), cleared, "{demo:?} center pixel");
        assert!(
            !bytes.chunks_exact(4).all(|p| p == cleared.as_slice()),
            "{demo:?} frame is uniformly the clear color"
        );
        frames.push(bytes);
    }

    assert_ne!(frames[1], frames[2]);
}

/// Plan 11's 1M-particle budget: median render() wall time under ~16 ms on
/// a hardware adapter. Software rasterizers cannot meet that budget (plan
/// risk 2), so there the run only proves the 1M path renders real frames.
#[gpui_kit::test]
#[ignore = "needs a wgpu adapter; run with --ignored locally"]
async fn renders_one_million_particles_and_meets_the_budget_on_hardware(
    _cx: &mut gpui_kit::TestAppContext,
) {
    let engine = std::sync::Arc::new(WgpuEngine::new().await.expect("wgpu engine"));
    let adapter_label = engine.adapter_label().to_string();
    let software = {
        let label = adapter_label.to_lowercase();
        ["llvmpipe", "lavapipe", "swiftshader", "software"]
            .iter()
            .any(|marker| label.contains(marker))
    };
    let base = RenderJob {
        demo: DemoId::Particles,
        particle_count: 1_000_000,
        dt: 1.0 / 60.0,
        elapsed: 0.0,
        width: 1280,
        height: 720,
    };
    let mut renderer = SceneRenderer::new(engine, &base).expect("scene renderer");
    let cleared: [u8; 4] = [10, 5, 5, 255];

    const WARMUP: u32 = 4;
    const MEASURED: u32 = 20;
    let mut samples = Vec::new();
    for frame in 0..(WARMUP + MEASURED) {
        let job = RenderJob {
            elapsed: frame as f32 / 60.0,
            ..base
        };
        let started = std::time::Instant::now();
        let bytes = renderer.render(job).expect("render");
        assert_eq!(bytes.len(), 1280 * 720 * 4);
        assert!(
            !bytes.chunks_exact(4).all(|p| p == cleared.as_slice()),
            "the 1M-particle frame must not be uniformly the clear color"
        );
        if frame >= WARMUP {
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
        }
    }

    samples.sort_by(|a, b| a.total_cmp(b));
    let median = samples[samples.len() / 2];
    if software {
        eprintln!(
            "software adapter [{adapter_label}]: 1M-particle median render {median:.1} ms; \
             the ~16 ms budget is hardware-gated (plan 11 risk 2)"
        );
    } else {
        assert!(
            median < 16.0,
            "median render time {median:.2} ms at 1M particles exceeds the ~16 ms frame budget"
        );
    }
}
