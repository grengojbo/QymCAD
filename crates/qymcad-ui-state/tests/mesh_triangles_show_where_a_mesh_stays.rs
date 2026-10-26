//! "MESH TRIANGLES" SHOWS WHERE A MESH STAYS: on a piece of a mesh every triangle, on a body of faces only the faces a
//! recognition left as mesh - a face of one triangle each - and not the tessellation of an exact face.
use qymcad_core::geom::{Mesh, MeshFace, Point3};
use qymcad_core::model::Body;
use qymcad_ui_state::{mesh_lines_of, MadeAs};

/// A face of the triangles `triangles`.
fn face(triangles: &[u32]) -> MeshFace {
    MeshFace { triangles: triangles.to_vec(), normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 0.0), area: 1.0, id: 0 }
}

/// A square of side 1 in two triangles, and beside it at x = 1..2 two more triangles of a square: four triangles, the
/// squares sharing the side at x = 1.
fn two_squares() -> Mesh {
    let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
    let verts = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0), p(2.0, 0.0), p(2.0, 1.0)];
    Mesh { verts, tris: vec![[0, 1, 2], [0, 2, 3], [1, 4, 5], [1, 5, 2]], ..Default::default() }
}

#[test]
fn a_piece_of_a_mesh_shows_every_triangle() {
    let body = Body { mesh: two_squares(), ..Default::default() };
    // two squares of four sides and a diagonal each, the side at x = 1 shared
    assert_eq!(mesh_lines_of(&body, MadeAs::MeshPiece).len(), 9, "the lines over a piece of a mesh");
}

#[test]
fn a_body_of_faces_shows_only_the_faces_left_as_mesh() {
    // the first square an exact face of two triangles, the second left as mesh: a face a triangle
    let body = Body { mesh: two_squares(), faces: vec![face(&[0, 1]), face(&[2]), face(&[3])], ..Default::default() };
    let lines = mesh_lines_of(&body, MadeAs::Faces);
    assert_eq!(lines.len(), 5, "the sides of the two triangles left as mesh");
    let on_exact = lines.iter().filter(|e| e.iter().all(|&v| body.mesh.verts[v as usize].x < 1.0 - 1e-9 || v == 1 || v == 2)).count();
    assert_eq!(on_exact, 1, "only the side the two kinds share stands at the exact square: {lines:?}");
    // a body all of exact faces shows nothing
    let exact = Body { mesh: two_squares(), faces: vec![face(&[0, 1]), face(&[2, 3])], ..Default::default() };
    assert!(mesh_lines_of(&exact, MadeAs::Faces).is_empty(), "lines over an exact body");
}
