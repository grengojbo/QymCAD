//! THE HEALTH OF A MESH IN NUMBERS: its holes, its edges shared by three triangles or more, its triangles turned
//! inside out, the pieces it falls into and the volume it closes - each on a mesh whose answer is known.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_doc::print::mesh_report::{mesh_report, MeshReport};

/// A cube of side `side` at `at`, its eight corners shared by its twelve triangles, wound outward.
fn cube(side: f64, at: [f64; 3]) -> Mesh {
    let c = |i: usize| Point3::new(at[0] + (i & 1) as f64 * side, at[1] + ((i >> 1) & 1) as f64 * side, at[2] + ((i >> 2) & 1) as f64 * side);
    let tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

/// The meshes `parts` as one.
fn joined(parts: Vec<Mesh>) -> Mesh {
    let mut out = Mesh::default();
    for m in parts {
        let shift = out.verts.len() as u32;
        out.verts.extend(m.verts);
        out.tris.extend(m.tris.into_iter().map(|t| t.map(|v| v + shift)));
    }
    out
}

#[test]
fn a_closed_cube_is_whole() {
    let r: MeshReport = mesh_report(&cube(10.0, [0.0; 3]));
    assert_eq!(r.triangles, 12, "{r:?}");
    assert_eq!(r.holes, 0, "{r:?}");
    assert_eq!(r.open_edges, 0, "{r:?}");
    assert_eq!(r.non_manifold, 0, "{r:?}");
    assert_eq!(r.flipped, 0, "{r:?}");
    assert_eq!(r.pieces, 1, "{r:?}");
    assert!(r.closed, "{r:?}");
    assert!((r.volume - 1000.0).abs() < 1e-9, "{r:?}");
    assert!((r.area - 600.0).abs() < 1e-9, "{r:?}");
}

#[test]
fn a_cube_short_of_a_triangle_has_a_hole_of_three_edges() {
    let mut m = cube(10.0, [0.0; 3]);
    m.tris.remove(2);
    let r = mesh_report(&m);
    assert_eq!(r.holes, 1, "{r:?}");
    assert_eq!(r.open_edges, 3, "{r:?}");
    assert!(!r.closed, "{r:?}");
    // the border runs round half a face: two sides of 10 and the diagonal
    assert!((r.widest_hole - (20.0 + 200f64.sqrt())).abs() < 1e-9, "{r:?}");
}

#[test]
fn two_cubes_apart_are_two_pieces() {
    let r = mesh_report(&joined(vec![cube(10.0, [0.0; 3]), cube(5.0, [30.0, 0.0, 0.0])]));
    assert_eq!(r.pieces, 2, "{r:?}");
    assert!(r.closed, "{r:?}");
    assert!((r.volume - 1125.0).abs() < 1e-9, "{r:?}");
}

#[test]
fn a_triangle_wound_inward_is_counted() {
    let mut m = cube(10.0, [0.0; 3]);
    m.tris[4] = [m.tris[4][0], m.tris[4][2], m.tris[4][1]];
    let r = mesh_report(&m);
    assert_eq!(r.flipped, 1, "{r:?}");
    // turned back, it closes the cube all the same
    assert!(r.closed, "{r:?}");
    assert!((r.volume - 1000.0).abs() < 1e-9, "{r:?}");
}

#[test]
fn a_cube_with_corners_of_its_own_per_face_is_welded_whole() {
    let m = cube(10.0, [0.0; 3]);
    let mut apart = Mesh::default();
    for t in &m.tris {
        let base = apart.verts.len() as u32;
        apart.verts.extend(t.iter().map(|&v| m.verts[v as usize]));
        apart.tris.push([base, base + 1, base + 2]);
    }
    let r = mesh_report(&apart);
    assert!(r.closed && r.pieces == 1, "{r:?}");
}

/// A cube of 10 with a hollow cube of 2 in its middle, the hollow's faces looking into the solid.
fn hollow_cube() -> Mesh {
    let mut inner = cube(2.0, [4.0; 3]);
    for t in &mut inner.tris {
        t.swap(1, 2);
    }
    joined(vec![cube(10.0, [0.0; 3]), inner])
}

/// A HOLLOW INSIDE A SOLID is no triangle wound inward and no piece printed apart: one piece with one hollow, closing
/// 1 000 - 8 mm^3.
#[test]
fn a_hollow_inside_a_cube_is_told_as_a_hollow() {
    let r = mesh_report(&hollow_cube());
    assert_eq!(r.flipped, 0, "the hollow's triangles are counted as wound inward: {r:?}");
    assert_eq!(r.pieces, 1, "{r:?}");
    assert_eq!(r.hollows, 1, "{r:?}");
    assert!((r.volume - 992.0).abs() < 1e-9, "{r:?}");
}
