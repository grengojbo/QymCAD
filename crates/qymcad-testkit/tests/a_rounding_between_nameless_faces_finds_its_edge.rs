//! AN EDGE BETWEEN TWO FACES THAT HAVE NO NAME IS FOUND BY THE FACES IT PARTS. A body made of a mesh - and a body come
//! from a STEP - carries positional face numbers, and its edges take no name from them; a rounding asked "between face
//! a and face b" read the pair off the edge's name only, found no edge, and the node went red. The kernel says which
//! two faces every edge parts, and that is enough. Built with the live kernel: a mock has no edges.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_core::model::{Id, ImportNode, Project};
use qymcad_core::refs::{Query, Ref};

/// A cube of `side` from twelve triangles, each turned out of the solid.
fn cube(side: f64) -> Mesh {
    let v = |x: f64, y: f64, z: f64| Point3::new(x * side, y * side, z * side);
    Mesh {
        verts: vec![v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(1.0, 1.0, 0.0), v(0.0, 1.0, 0.0), v(0.0, 0.0, 1.0), v(1.0, 0.0, 1.0), v(1.0, 1.0, 1.0), v(0.0, 1.0, 1.0)],
        tris: vec![[0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7], [0, 1, 5], [0, 5, 4], [1, 2, 6], [1, 6, 5], [2, 3, 7], [2, 7, 6], [3, 0, 4], [3, 4, 7]],
        ..Default::default()
    }
}

/// The face of `body` whose normal is nearest `dir`.
fn face_towards(p: &Project, body: Id, dir: [f64; 3]) -> u32 {
    let faces = p.regen_faces.get(&body).expect("the faces of the solid");
    let dot = |n: [f64; 3]| n[0] * dir[0] + n[1] * dir[1] + n[2] * dir[2];
    faces.iter().max_by(|a, b| dot(a.normal).total_cmp(&dot(b.normal))).map(|f| f.id).expect("a face")
}

#[test]
fn a_rounding_between_two_faces_of_a_mesh_solid_rounds_their_edge() {
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("cube.stl", vec![1]);
    let src = p.add_mesh(cube(10.0));
    p.import_tree_as_parts(vec![ImportNode { name: "cube".into(), body: Some(src), mesh: true, ..Default::default() }], source, "cube").expect("came in");
    let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
    p.set_active_component(part);
    let body = p.add_mesh_solid(src, 0.0);
    let _ = qymcad_testkit::regenerate(&mut p);
    let (x, y) = (face_towards(&p, body, [1.0, 0.0, 0.0]), face_towards(&p, body, [0.0, 1.0, 0.0]));
    let edge = Ref::many(Query::Between(Box::new(Query::Id(x)), Box::new(Query::Id(y))));
    let round = p.add_fillet_ref(body, 2.0, edge);
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the rounding between faces {x} and {y} went red: {:?}", report.errors);
    // one edge of 10 rounded by 2: (4 - pi) mm^2 of its corner taken along its length
    let want = 1000.0 - (4.0 - std::f64::consts::PI) * 10.0;
    let got = shapes.get(&round).map(|s| s.volume()).expect("the rounding is built");
    assert!((got - want).abs() < 1e-3, "the rounded cube holds {got} mm^3, not {want}");
}
