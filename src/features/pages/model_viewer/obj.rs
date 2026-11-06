//! Minimal OBJ parser: `v`/`vn`/`vt`/`f` statements with fan
//! triangulation. Everything else in the stream (groups, materials,
//! comments, free-form curves) is skipped. Positions are recentered on the
//! origin and scaled so the longest axis spans 2.0 units: the viewer's
//! camera orbits a fixed shell and needs a model-normalized scene.

use crate::features::wgpu::encode_f32_rows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::features::pages::model_viewer) struct MeshStats {
    pub(in crate::features::pages::model_viewer) triangles: usize,
    pub(in crate::features::pages::model_viewer) vertices: usize,
}

#[derive(Debug)]
pub(in crate::features::pages::model_viewer) struct Mesh {
    /// Interleaved `[pos3, normal3, uv2]` f32 rows in `MESH_VERTEX_STRIDE`
    /// layout, ready for a vertex-buffer upload.
    pub(in crate::features::pages::model_viewer) vertices: Vec<u8>,
    pub(in crate::features::pages::model_viewer) indices: Vec<u32>,
    pub(in crate::features::pages::model_viewer) stats: MeshStats,
}

pub(in crate::features::pages::model_viewer) fn parse(source: &str) -> Result<Mesh, String> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut rows: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut triangles = 0usize;

    for (number, line) in source.lines().enumerate() {
        let line_no = number + 1;
        let mut fields = line.split_whitespace();
        let kind = fields.next();
        match kind {
            Some("v") => {
                let [x, y, z] = triple(&mut fields, line_no, "v")?;
                positions.push([x, y, z]);
            }
            Some("vn") => {
                let [x, y, z] = triple(&mut fields, line_no, "vn")?;
                normals.push(normalize([x, y, z]));
            }
            Some("vt") => {
                let [u, v] = pair(&mut fields, line_no, "vt")?;
                uvs.push([u, v]);
            }
            Some("f") => {
                let corners: Vec<Corner> = fields
                    .map(|field| corner(field, &positions, &normals, &uvs, line_no))
                    .collect::<Result<_, _>>()?;
                if corners.len() < 3 {
                    return Err(format!("line {line_no}: face needs at least 3 vertices"));
                }
                let fallback_normal = face_normal(&corners);
                // Fan from corner 0: every polygon a convex OBJ face claims
                // to be triangulates correctly this way.
                for i in 1..corners.len() - 1 {
                    let base = rows.len() / 8;
                    for corner in [&corners[0], &corners[i], &corners[i + 1]] {
                        let normal = corner.normal.unwrap_or(fallback_normal);
                        rows.extend_from_slice(&corner.position);
                        rows.extend_from_slice(&normal);
                        rows.extend_from_slice(&corner.uv.unwrap_or([0.0, 0.0]));
                    }
                    indices.extend_from_slice(&[base as u32, (base + 1) as u32, (base + 2) as u32]);
                    triangles += 1;
                }
            }
            _ => {}
        }
    }

    if triangles == 0 {
        return Err("no triangulated faces in file".to_string());
    }
    normalize_rows(&mut rows);
    let vertices = rows.len() / 8;
    Ok(Mesh {
        vertices: encode_f32_rows(&rows),
        indices,
        stats: MeshStats {
            triangles,
            vertices,
        },
    })
}

/// Built-in demo mesh: a plain torus (major 1.0, minor 0.4) so the viewer
/// always has something on screen before a load.
pub(in crate::features::pages::model_viewer) fn builtin_torus() -> Mesh {
    let (major_segments, minor_segments) = (48usize, 24usize);
    let (major_radius, minor_radius) = (1.0f32, 0.4f32);
    let mut rows: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Seam rows/columns duplicate their wrap-around ring so UVs stay clean.
    for major in 0..=major_segments {
        let theta = major as f32 * std::f32::consts::TAU / major_segments as f32;
        for minor in 0..=minor_segments {
            let phi = minor as f32 * std::f32::consts::TAU / minor_segments as f32;
            let (sin_theta, cos_theta) = theta.sin_cos();
            let (sin_phi, cos_phi) = phi.sin_cos();
            rows.extend_from_slice(&[
                (major_radius + minor_radius * cos_phi) * cos_theta,
                minor_radius * sin_phi,
                (major_radius + minor_radius * cos_phi) * sin_theta,
            ]);
            rows.extend_from_slice(&[cos_phi * cos_theta, sin_phi, cos_phi * sin_theta]);
            rows.extend_from_slice(&[
                major as f32 / major_segments as f32,
                minor as f32 / minor_segments as f32,
            ]);
        }
    }
    let ring = minor_segments + 1;
    let grid = |major: usize, minor: usize| -> u32 { (major * ring + minor) as u32 };
    for major in 0..major_segments {
        for minor in 0..minor_segments {
            let a = grid(major, minor);
            let b = grid(major + 1, minor);
            let c = grid(major + 1, minor + 1);
            let d = grid(major, minor + 1);
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    let vertices = rows.len() / 8;
    let triangles = indices.len() / 3;
    Mesh {
        vertices: encode_f32_rows(&rows),
        indices,
        stats: MeshStats {
            triangles,
            vertices,
        },
    }
}

struct Corner {
    position: [f32; 3],
    normal: Option<[f32; 3]>,
    uv: Option<[f32; 2]>,
}

fn corner(
    field: &str,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    line_no: usize,
) -> Result<Corner, String> {
    let resolve = |raw: &str, len: usize, what: &str| -> Result<usize, String> {
        let index: i64 = raw
            .parse()
            .map_err(|_| format!("line {line_no}: bad {what} index {raw:?}"))?;
        if index == 0 {
            return Err(format!("line {line_no}: {what} index 0 (OBJ is 1-based)"));
        }
        let resolved = if index > 0 {
            (index - 1) as usize
        } else {
            len.checked_add_signed(index as isize)
                .ok_or_else(|| format!("line {line_no}: {what} index {index} out of range"))?
        };
        if resolved >= len {
            return Err(format!("line {line_no}: {what} index {index} out of range"));
        }
        Ok(resolved)
    };
    let mut parts = field.split('/');
    let position_index = resolve(parts.next().unwrap_or(""), positions.len(), "vertex")?;
    let position = positions[position_index];
    let uv = match parts.next() {
        Some("") | None => None,
        Some(raw) => Some(uvs[resolve(raw, uvs.len(), "uv")?]),
    };
    let normal = match parts.next() {
        Some("") | None => None,
        Some(raw) => Some(normals[resolve(raw, normals.len(), "normal")?]),
    };
    Ok(Corner {
        position,
        normal,
        uv,
    })
}

fn face_normal(corners: &[Corner]) -> [f32; 3] {
    if corners.len() < 3 {
        return [0.0, 1.0, 0.0];
    }
    let a = corners[0].position;
    let b = corners[1].position;
    let c = corners[2].position;
    let edge_ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let edge_ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let crossed = [
        edge_ab[1] * edge_ac[2] - edge_ab[2] * edge_ac[1],
        edge_ab[2] * edge_ac[0] - edge_ab[0] * edge_ac[2],
        edge_ab[0] * edge_ac[1] - edge_ab[1] * edge_ac[0],
    ];
    let normal = normalize(crossed);
    if normal[0].is_finite() && normal[1].is_finite() && normal[2].is_finite() {
        normal
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len <= f32::EPSILON || !len.is_finite() {
        return [0.0, 1.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn normalize_rows(rows: &mut [f32]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for row in rows.as_chunks::<8>().0 {
        for axis in 0..3 {
            min[axis] = min[axis].min(row[axis]);
            max[axis] = max[axis].max(row[axis]);
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let extent = (max[0] - min[0])
        .max(max[1] - min[1])
        .max(max[2] - min[2])
        .max(f32::EPSILON);
    let scale = 2.0 / extent;
    for row in rows.as_chunks_mut::<8>().0 {
        for axis in 0..3 {
            row[axis] = (row[axis] - center[axis]) * scale;
        }
    }
}

fn next_f32<'a, I>(fields: &mut I, line_no: usize, what: &str) -> Result<f32, String>
where
    I: Iterator<Item = &'a str>,
{
    let raw = fields
        .next()
        .ok_or_else(|| format!("line {line_no}: {what} needs more components"))?;
    raw.parse()
        .map_err(|_| format!("line {line_no}: bad {what} component {raw:?}"))
}

fn triple<'a, I>(fields: &mut I, line_no: usize, what: &str) -> Result<[f32; 3], String>
where
    I: Iterator<Item = &'a str>,
{
    let x = next_f32(fields, line_no, what)?;
    let y = next_f32(fields, line_no, what)?;
    let z = next_f32(fields, line_no, what)?;
    Ok([x, y, z])
}

fn pair<'a, I>(fields: &mut I, line_no: usize, what: &str) -> Result<[f32; 2], String>
where
    I: Iterator<Item = &'a str>,
{
    let u = next_f32(fields, line_no, what)?;
    let v = next_f32(fields, line_no, what)?;
    Ok([u, v])
}

#[cfg(test)]
#[path = "obj.test.rs"]
mod obj_test;
