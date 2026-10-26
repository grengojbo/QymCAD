//! THE SIDES OF TRIANGLES AS LINES: every side of the triangles asked for, once, the way "Mesh triangles" draws a mesh
//! over a body - a side two triangles share is one line, whether they share its vertices or each holds its own there.
use qymcad_core::geom::{Mesh, Point3};

/// Two squares of side 1 along x from 0, the second turned up about x = 1 by 90 deg, each two triangles and its own
/// vertices: the side at x = 1 stands twice, once in each square.
fn folded() -> Mesh {
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let verts = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0), p(0.0, 1.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 0.0, 1.0), p(1.0, 1.0, 1.0), p(1.0, 1.0, 0.0)];
    Mesh { verts, tris: vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]], ..Default::default() }
}

#[test]
fn every_side_of_the_triangles_asked_for_is_one_line() {
    let m = folded();
    // four sides and a diagonal a square, the side at the fold shared: 5 + 5 - 1
    assert_eq!(m.sides(&[0, 1, 2, 3]).len(), 9, "the sides of the two squares");
    assert_eq!(m.sides(&[0]).len(), 3, "the sides of one triangle");
    // the two triangles of the first square share the diagonal
    assert_eq!(m.sides(&[0, 1]).len(), 5, "the sides of one square");
    assert!(m.sides(&[]).is_empty(), "no triangle, no line");
    // a triangle out of the mesh is passed over, not a panic
    assert_eq!(m.sides(&[0, 99]).len(), 3, "a triangle past the mesh");
}
