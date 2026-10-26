//! A PIECE OF A MESH TAKES A MENDED MESH IN PLACE: its mesh is its geometry, kept by the document, so a repair writes
//! the new triangles into it and each new triangle takes the colour of the old one it came of. A body that is no piece
//! of a mesh is left as it is.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_core::model::{Colours, ImportNode, PieceMesh, Project};

/// Three triangles side by side along x, each with corners of its own.
fn strip(n: u32) -> Mesh {
    let mut m = Mesh::default();
    for k in 0..n {
        let x = f64::from(k);
        m.verts.extend([Point3::new(x, 0.0, 0.0), Point3::new(x + 1.0, 0.0, 0.0), Point3::new(x, 1.0, 0.0)]);
        m.tris.push([3 * k, 3 * k + 1, 3 * k + 2]);
    }
    m
}

const RED: [u8; 3] = [200, 0, 0];
const GREEN: [u8; 3] = [0, 200, 0];
const BLUE: [u8; 3] = [0, 0, 200];

/// A document and the body of the piece of a mesh in it.
struct Piece {
    p: Project,
    body: u64,
}

/// A document with a piece of three triangles coloured red, green and blue.
fn coloured_piece() -> Piece {
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("scan.3mf", vec![1]);
    let body = p.add_mesh(strip(3));
    let piece = ImportNode { name: "scan".into(), body: Some(body), mesh: true, tri_colors: vec![RED, GREEN, BLUE], ..Default::default() };
    p.import_tree_as_parts(vec![piece], source, "scan").expect("came in");
    Piece { p, body }
}

/// The colour every triangle of `body` is shown in.
fn colours_of(p: &Project, body: u64) -> Vec<[u8; 3]> {
    let (palette, places) = p.tri_colors.get(&p.lineage_root(body)).expect("the piece keeps its triangles' colours");
    places.iter().map(|&k| palette[k as usize]).collect()
}

#[test]
fn a_piece_takes_the_mended_mesh_and_its_colours_follow() {
    let Piece { mut p, body } = coloured_piece();
    let mended = PieceMesh { mesh: strip(4), faces: Vec::new(), colours: Colours::From(vec![2, 0, 1, 1]) };
    assert!(p.replace_piece_mesh(body, mended), "a piece of a mesh did not take its mended mesh");
    let mi = p.mesh_index(body).expect("the body stays");
    assert_eq!(p.bodies[mi].mesh.tris.len(), 4, "the piece does not hold the mended triangles");
    assert_eq!(colours_of(&p, body), [BLUE, RED, GREEN, GREEN], "the new triangles do not take the colours of the old ones they came of");
    assert!(p.timeline.iter().all(|n| !n.dirty), "a piece of a mesh is marked for a rebuild it has nothing to rebuild from");
}

#[test]
fn colours_dropped_leave_the_piece_in_its_own_colour() {
    let Piece { mut p, body } = coloured_piece();
    assert!(p.replace_piece_mesh(body, PieceMesh { mesh: strip(2), faces: Vec::new(), colours: Colours::Dropped }));
    assert!(!p.tri_colors.contains_key(&p.lineage_root(body)), "colours of triangles that are no more are kept");
}

#[test]
fn a_body_that_is_no_piece_of_a_mesh_is_left_as_it_is() {
    let mut p = Project::default();
    p.new_document();
    let body = p.add_mesh(strip(3));
    assert!(!p.replace_piece_mesh(body, PieceMesh { mesh: strip(1), faces: Vec::new(), colours: Colours::Dropped }), "a body of no file took a mesh");
    assert_eq!(p.bodies[0].mesh.tris.len(), 3, "the body was changed");
}
