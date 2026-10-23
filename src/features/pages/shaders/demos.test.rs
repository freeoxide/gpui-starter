use super::*;

#[test]
fn uniform_bytes_match_the_wgsl_layout() {
    let params = ShaderParams {
        time: 1.5,
        delta: 0.25,
        slider: 0.75,
        resolution: (384.0, 256.0),
        mouse: (0.25, 0.75),
    };
    let bytes = params.to_bytes();
    assert_eq!(&bytes[0..4], &1.5f32.to_le_bytes());
    assert_eq!(&bytes[4..8], &0.25f32.to_le_bytes());
    assert_eq!(&bytes[8..12], &0.75f32.to_le_bytes());
    assert_eq!(&bytes[12..16], &0f32.to_le_bytes());
    assert_eq!(&bytes[16..20], &384.0f32.to_le_bytes());
    assert_eq!(&bytes[20..24], &256.0f32.to_le_bytes());
    assert_eq!(&bytes[24..28], &0.25f32.to_le_bytes());
    assert_eq!(&bytes[28..32], &0.75f32.to_le_bytes());
}

#[test]
fn demo_lookup_is_exhaustive() {
    for spec in DEMOS.iter() {
        assert_eq!(demo(spec.id).id, spec.id);
    }
}

#[test]
fn specs_pin_resolutions_and_the_single_textured_demo() {
    assert_eq!(DEMOS.iter().filter(|s| s.uses_texture).count(), 1);
    for spec in DEMOS.iter() {
        assert!(spec.width > 0 && spec.height > 0);
    }
    let raymarch = demo(DemoId::Raymarch);
    assert_eq!((raymarch.width, raymarch.height), (384, 384));
    assert!(demo(DemoId::Warp).uses_texture);
}

#[test]
fn every_wgsl_module_parses_with_naga() {
    for spec in DEMOS.iter() {
        let module = naga::front::wgsl::parse_str(spec.wgsl)
            .unwrap_or_else(|err| panic!("naga rejected {:?}: {err}", spec.id));
        let entry_points: Vec<&str> = module
            .entry_points
            .iter()
            .map(|ep| ep.name.as_str())
            .collect();
        assert!(entry_points.contains(&"vs_main"), "{:?} vs_main", spec.id);
        assert!(entry_points.contains(&"fs_main"), "{:?} fs_main", spec.id);
    }
}

#[test]
fn every_fragment_keeps_the_bgra_readback_swizzle() {
    for spec in DEMOS.iter() {
        assert!(
            spec.wgsl.contains("vec4<f32>(col.b, col.g, col.r"),
            "{:?} lost the readback swizzle",
            spec.id
        );
        assert!(
            !spec.wgsl.contains("let target ="),
            "{:?} uses the reserved WGSL keyword `target`",
            spec.id
        );
    }
}

#[test]
fn warp_source_is_opaque_rgba_at_full_size() {
    let bytes = warp_source_bytes();
    assert_eq!(
        bytes.len(),
        (WARP_SOURCE_SIZE * WARP_SOURCE_SIZE * 4) as usize
    );
    let last = bytes.len() - 4;
    for at in [0, bytes.len() / 2, last] {
        assert_eq!(bytes[at + 3], 255);
    }
}

#[test]
fn warp_source_tiles_differ_between_checker_cells() {
    let bytes = warp_source_bytes();
    let pixel = |x: u32, y: u32| {
        let at = ((y * WARP_SOURCE_SIZE + x) * 4) as usize;
        [bytes[at], bytes[at + 1], bytes[at + 2]]
    };
    let even_tile_center = pixel(16, 16);
    let odd_tile_center = pixel(48, 16);
    assert_ne!(even_tile_center, odd_tile_center);
    let tile_edge = pixel(2, 16);
    assert_ne!(even_tile_center, tile_edge);
}
