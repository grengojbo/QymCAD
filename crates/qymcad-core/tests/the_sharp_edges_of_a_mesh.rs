//! THE SHARP EDGES OF A MESH: where two faces meet at an angle, and where the mesh is open - not where two faces run on
//! smoothly into each other, and not across the triangles of one face. The lines the view draws so the faces of a body
//! read apart.
use qymcad_core::geom::{Mesh, MeshFace, Point3};

/// A face of the triangles `triangles`.
fn face(triangles: &[u32]) -> MeshFace {
    MeshFace { triangles: triangles.to_vec(), normal: [0.0, 0.0, 1.0], centroid: Point3::new(0.0, 0.0, 0.0), area: 1.0, id: 0 }
}

/// Two squares of side 1 along x from 0, the second turned up about x = 1 by `deg`, each two triangles and its own
/// vertices: a solid is meshed face by face.
fn folded(deg: f64) -> Mesh {
    let (c, s) = (deg.to_radians().cos(), deg.to_radians().sin());
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let verts = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0), p(0.0, 1.0, 0.0), p(1.0, 0.0, 0.0), p(1.0 + c, 0.0, s), p(1.0 + c, 1.0, s), p(1.0, 1.0, 0.0)];
    Mesh { verts, tris: vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]], ..Default::default() }
}

/// The edges whose both ends stand at x = 1 on the floor: the fold.
fn at_the_fold(m: &Mesh, edges: &[[u32; 2]]) -> usize {
    edges.iter().filter(|e| e.iter().all(|&v| (m.verts[v as usize].x - 1.0).abs() < 1e-9 && m.verts[v as usize].z.abs() < 1e-9)).count()
}

#[test]
fn a_fold_between_two_faces_is_an_edge_and_a_smooth_seam_is_not() {
    let faces = [face(&[0, 1]), face(&[2, 3])];
    let sharp = folded(90.0);
    assert_eq!(at_the_fold(&sharp, &sharp.sharp_edges(&faces, 20.0)), 1, "a fold of 90 deg between two faces is not one edge");
    let smooth = folded(10.0);
    assert_eq!(at_the_fold(&smooth, &smooth.sharp_edges(&faces, 20.0)), 0, "a seam turning 10 deg is drawn as an edge");
    // within one face a fold is no edge: the triangles are one face
    let one = [face(&[0, 1, 2, 3])];
    assert_eq!(at_the_fold(&sharp, &sharp.sharp_edges(&one, 20.0)), 0, "a fold inside one face is drawn as an edge");
    // the diagonals of the squares are inside a face: none of them
    let diagonals = sharp
        .sharp_edges(&faces, 20.0)
        .iter()
        .filter(|e| {
            let (a, b) = (sharp.verts[e[0] as usize], sharp.verts[e[1] as usize]);
            (a.x - b.x).abs() > 1e-9 && (a.y - b.y).abs() > 1e-9
        })
        .count();
    assert_eq!(diagonals, 0, "a diagonal inside a face is drawn as an edge");
    // the open rims of the two squares: three sides each
    assert_eq!(sharp.sharp_edges(&faces, 20.0).len(), 7, "the open rims and the fold are not seven edges");
}
