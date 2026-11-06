// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use super::Mesh;
use crate::features::wgpu::MESH_VERTEX_STRIDE;

fn row(mesh: &Mesh, index: usize) -> [f32; 8] {
    let offset = index * MESH_VERTEX_STRIDE;
    let mut row = [0f32; 8];
    for (axis, slot) in row.iter_mut().enumerate() {
        let start = offset + axis * 4;
        *slot = f32::from_ne_bytes(
            mesh.vertices[start..start + 4]
                .try_into()
                .expect("row bytes"),
        );
    }
    row
}

fn extent(mesh: &Mesh, axis: usize) -> f32 {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for index in 0..mesh.stats.vertices {
        let value = row(mesh, index)[axis];
        min = min.min(value);
        max = max.max(value);
    }
    max - min
}

#[test]
fn quad_face_fan_triangulates_into_two_triangles() {
    let source = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";
    let mesh = super::parse(source).expect("parses");
    assert_eq!(mesh.stats.triangles, 2);
    assert_eq!(mesh.stats.vertices, 6);
    assert_eq!(mesh.indices.len(), 6);
    assert_eq!(mesh.vertices.len(), 6 * MESH_VERTEX_STRIDE);
    assert!(mesh.indices.iter().all(|&ix| ix < 6));
}

#[test]
fn face_forms_and_index_kinds_resolve() {
    let source = "\
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 1/1/1 2/2/1 3/3/1
f 3//1 2 1
f -3 -2 -1
";
    let mesh = super::parse(source).expect("parses");
    assert_eq!(mesh.stats.triangles, 3);
    // Corner 0 of triangle 0 carries the resolved uv.
    assert_eq!([row(&mesh, 0)[6], row(&mesh, 0)[7]], [0.0, 0.0]);
    // The explicit vn normal (0,0,1) survives the parser.
    assert_eq!(
        [row(&mesh, 0)[3], row(&mesh, 0)[4], row(&mesh, 0)[5]],
        [0.0, 0.0, 1.0]
    );
}

#[test]
fn missing_normals_fall_back_to_the_face_normal() {
    let source = "\
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
";
    let mesh = super::parse(source).expect("parses");
    let normal = [row(&mesh, 0)[3], row(&mesh, 0)[4], row(&mesh, 0)[5]];
    let near = |a: f32, b: f32| (a - b).abs() < 1e-6;
    assert!(near(normal[0], 0.0) && near(normal[1], 0.0) && near(normal[2], 1.0));
}

#[test]
fn positions_recenter_and_scale_to_a_two_unit_extent() {
    let source = "\
v 10 10 10
v 12 10 10
v 10 12 10
v 10 10 12
f 1 2 3
f 1 3 4
f 1 4 2
";
    let mesh = super::parse(source).expect("parses");
    let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
    for axis in 0..3 {
        assert!(near(extent(&mesh, axis), 2.0), "axis {axis}");
    }
    // The bbox (not the vertex average) lands centered on the origin.
    let corner = row(&mesh, 0);
    assert!(
        near(corner[0], -1.0) && near(corner[1], -1.0) && near(corner[2], -1.0),
        "corner at -1.0: {:?}",
        &corner[0..3]
    );
}

#[test]
fn malformed_input_reports_the_line_number() {
    let bad_float = super::parse("v 1 x 3\nf 1 1 1");
    assert!(bad_float.unwrap_err().contains("line 1"));

    let short_face = super::parse("v 0 0 0\nv 1 0 0\nf 1 2");
    assert!(short_face.unwrap_err().contains("line 3"));

    let out_of_range = super::parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 9");
    assert!(out_of_range.unwrap_err().contains("out of range"));

    let zero_index = super::parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 1 2");
    assert!(zero_index.unwrap_err().contains("1-based"));
}

#[test]
fn comments_groups_and_materials_are_skipped_and_empty_sources_fail() {
    let skipped = "# comment\no group\ng g\nusemtl m\ns off\n";
    assert_eq!(
        super::parse(skipped).unwrap_err(),
        "no triangulated faces in file"
    );
    assert_eq!(
        super::parse("").unwrap_err(),
        "no triangulated faces in file"
    );
}

#[test]
fn builtin_torus_covers_a_closed_grid_with_valid_indices() {
    let mesh = super::builtin_torus();
    // 48x24 quads over a (48+1)x(24+1) seam-duplicated grid.
    let expected_vertices = 49 * 25;
    assert_eq!(mesh.stats.vertices, expected_vertices);
    assert_eq!(mesh.stats.triangles, 48 * 24 * 2);
    assert_eq!(mesh.vertices.len(), expected_vertices * MESH_VERTEX_STRIDE);
    assert!(mesh.indices.iter().all(|&ix| ix < expected_vertices as u32));
    // Every surface normal is unit length.
    let unit = (0..expected_vertices).all(|index| {
        let row = row(&mesh, index);
        let len = (row[3] * row[3] + row[4] * row[4] + row[5] * row[5]).sqrt();
        (len - 1.0).abs() < 1e-5
    });
    assert!(unit);
}
