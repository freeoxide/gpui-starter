use super::*;

fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 4] {
    let ext = [p[0], p[1], p[2], 1.0];
    let mut out = [0.0; 4];
    for r in 0..4 {
        out[r] = (0..4).map(|c| m[c * 4 + r] * ext[c]).sum();
    }
    out
}

#[test]
fn mat4_identity_and_mul_agree_with_hand_computed_values() {
    let identity = mat4_identity();
    assert_eq!(identity, mat4_mul(&identity, &identity));

    let translate = mat4_translate(1.0, 2.0, 3.0);
    assert_eq!(
        (translate[12], translate[13], translate[14]),
        (1.0, 2.0, 3.0)
    );

    let combined = mat4_mul(
        &mat4_translate(1.0, 0.0, 0.0),
        &mat4_translate(0.0, 1.0, 0.0),
    );
    let moved = transform_point(&combined, [0.0, 0.0, 0.0]);
    assert_eq!((moved[0], moved[1], moved[2]), (1.0, 1.0, 0.0));
}

#[test]
fn mat4_rotates_match_hand_computed_axes() {
    let quarter = std::f32::consts::FRAC_PI_2;
    let near = |a: f32, b: f32| (a - b).abs() < 1e-6;

    let rot_y = mat4_rotate_y(quarter);
    let y_spun = transform_point(&rot_y, [1.0, 0.0, 0.0]);
    assert!(near(y_spun[0], 0.0) && near(y_spun[1], 0.0) && near(y_spun[2], 1.0));

    let rot_x = mat4_rotate_x(quarter);
    let x_spun = transform_point(&rot_x, [0.0, 1.0, 0.0]);
    assert!(near(x_spun[0], 0.0) && near(x_spun[1], 0.0) && near(x_spun[2], -1.0));
}

#[test]
fn perspective_maps_near_to_zero_and_far_to_one() {
    let proj = perspective(std::f32::consts::FRAC_PI_4, 16.0 / 9.0, 0.1, 100.0);

    let near = transform_point(&proj, [0.0, 0.0, -0.1]);
    assert!((near[2] / near[3]).abs() < 1e-5);
    assert!(near[3] > 0.0);

    let far = transform_point(&proj, [0.0, 0.0, -100.0]);
    assert!((far[2] / far[3] - 1.0).abs() < 1e-5);
    assert!(far[3] > 0.0);

    let mid = transform_point(&proj, [0.0, 0.0, -50.05]);
    assert!((mid[2] / mid[3] - 0.998).abs() < 1e-2);
}

#[test]
fn sphere_mesh_layout_and_indices_stay_in_bounds() {
    let (vertices, indices) = sphere_mesh(16, 24);
    let vertex_count = (16 + 1) * 24 + 2;

    assert_eq!(vertices.len(), vertex_count * MESH_VERTEX_STRIDE);
    assert!(indices.iter().all(|&ix| (ix as usize) < vertex_count));
    assert_eq!(indices.len(), 6 * 24 * (16 + 1));
    assert_eq!(indices.len() % 3, 0);
}

#[test]
fn checker_texture_size_is_exact() {
    assert_eq!(checker_texture(256).len(), 256 * 256 * 4);
    assert_eq!(checker_texture(64).len(), 64 * 64 * 4);
}

#[test]
fn particle_init_is_deterministic_with_sized_points() {
    let (positions_a, velocities_a) = particle_init(1024);
    let (positions_b, velocities_b) = particle_init(1024);

    assert_eq!(positions_a, positions_b);
    assert_eq!(velocities_a, velocities_b);

    let sizes: Vec<f32> = positions_a
        .chunks_exact(16)
        .map(|row| f32::from_ne_bytes([row[12], row[13], row[14], row[15]]))
        .collect();
    assert!(sizes.iter().all(|w| (1.5..=2.5).contains(w)));
}
