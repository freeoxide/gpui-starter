// The bridge's native renderers are the only consumers; wasm compiles the
// module without them.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

use std::f32::consts::PI;

pub(in crate::features::wgpu) type Mat4 = [f32; 16];

pub(in crate::features::wgpu) const MESH_VERTEX_STRIDE: usize = 32;

pub(in crate::features::wgpu) fn mat4_identity() -> Mat4 {
    let mut m = [0.0; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

/// Column-major product `a * b`: applies `b` first, then `a`.
pub(in crate::features::wgpu) fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            out[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    out
}

pub(in crate::features::wgpu) fn mat4_rotate_y(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    [
        c, 0.0, s, 0.0, 0.0, 1.0, 0.0, 0.0, -s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

pub(in crate::features::wgpu) fn mat4_rotate_x(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    [
        1.0, 0.0, 0.0, 0.0, 0.0, c, -s, 0.0, 0.0, s, c, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

pub(in crate::features::wgpu) fn mat4_translate(x: f32, y: f32, z: f32) -> Mat4 {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
    ]
}

/// Right-handed perspective with the wgpu NDC depth range z in [0, 1].
pub(in crate::features::wgpu) fn perspective(
    fov_y_rad: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> Mat4 {
    let f = 1.0 / (fov_y_rad / 2.0).tan();
    let mut m = [0.0; 16];
    m[0] = f / aspect;
    m[5] = f;
    m[10] = far / (near - far);
    m[11] = -1.0;
    m[14] = near * far / (near - far);
    m
}

fn push_vertex(out: &mut Vec<f32>, pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) {
    out.extend_from_slice(&pos);
    out.extend_from_slice(&normal);
    out.extend_from_slice(&uv);
}

/// UV sphere of radius 1: `(lat + 1) * lon + 2` vertices (unit normals equal
/// to positions), triangle indices including the two pole fans.
pub(in crate::features::wgpu) fn sphere_mesh(lat: usize, lon: usize) -> (Vec<u8>, Vec<u32>) {
    let mut vertices: Vec<f32> = Vec::with_capacity(((lat + 1) * lon + 2) * 8);
    push_vertex(&mut vertices, [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]);
    for i in 1..=(lat + 1) {
        let polar = PI * i as f32 / (lat + 2) as f32;
        for j in 0..lon {
            let azimuth = 2.0 * PI * j as f32 / lon as f32;
            let pos = [
                polar.sin() * azimuth.cos(),
                polar.cos(),
                polar.sin() * azimuth.sin(),
            ];
            let uv = [j as f32 / lon as f32, polar / PI];
            push_vertex(&mut vertices, pos, pos, uv);
        }
    }
    let south = vertices.len() / 8;
    push_vertex(
        &mut vertices,
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 1.0],
    );

    let ring = |i: usize, j: usize| -> u32 { (1 + (i - 1) * lon + j % lon) as u32 };
    let mut indices: Vec<u32> = Vec::with_capacity(lon * 6 * (lat + 1));
    for j in 0..lon {
        indices.extend_from_slice(&[0, ring(1, j), ring(1, j + 1)]);
    }
    for i in 1..=lat {
        for j in 1..=lon {
            let (a, b) = (ring(i, j), ring(i, j + 1));
            let (c, d) = (ring(i + 1, j + 1), ring(i + 1, j));
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    for j in 0..lon {
        indices.extend_from_slice(&[south as u32, ring(lat + 1, j + 1), ring(lat + 1, j)]);
    }

    (encode_f32_rows(&vertices), indices)
}

/// 8x8 checker of light/dark tiles with a red diagonal stripe every 64 px.
pub(in crate::features::wgpu) fn checker_texture(size: u32) -> Vec<u8> {
    let cell = size / 8;
    let mut out = Vec::with_capacity(size as usize * size as usize * 4);
    for y in 0..size {
        for x in 0..size {
            let checker = (x / cell + y / cell).is_multiple_of(2);
            let color: [u8; 4] = if (x + y) % 64 < 8 {
                [200, 70, 60, 255]
            } else if checker {
                [230, 230, 235, 255]
            } else {
                [38, 38, 46, 255]
            };
            out.extend_from_slice(&color);
        }
    }
    out
}

fn hash01(i: u32, seed: f32) -> f32 {
    (((i as f32 + seed) * 127.1).sin() * 43758.545)
        .fract()
        .abs()
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

pub(in crate::features::wgpu) fn encode_f32_rows(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_ne_bytes());
    }
    out
}

/// Deterministic particle spawn: positions on a golden-spiral shell with
/// point size in `.w`, tangential velocities. Returns (positions, velocities)
/// as `[f32; 4]` rows.
pub(in crate::features::wgpu) fn particle_init(count: u32) -> (Vec<u8>, Vec<u8>) {
    let mut positions = Vec::with_capacity(count as usize * 4);
    let mut velocities = Vec::with_capacity(count as usize * 4);
    for i in 0..count {
        let (h1, h2, h3) = (hash01(i, 0.0), hash01(i, 11.3), hash01(i, 23.7));
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
        let ring_radius = (1.0 - y * y).sqrt();
        let theta = i as f32 * 2.399_963;
        let dir = [ring_radius * theta.cos(), y, ring_radius * theta.sin()];
        let shell = 0.9 + 0.4 * h1;
        positions.extend_from_slice(&[dir[0] * shell, dir[1] * shell, dir[2] * shell, 1.5 + h2]);
        let tangent = normalize(cross([0.0, 1.0, 0.0], dir));
        let speed = 0.2 + 0.3 * h3;
        velocities.extend_from_slice(&[
            tangent[0] * speed,
            tangent[1] * speed,
            tangent[2] * speed,
            0.0,
        ]);
    }
    (encode_f32_rows(&positions), encode_f32_rows(&velocities))
}

#[cfg(test)]
#[path = "math.test.rs"]
mod math_test;
