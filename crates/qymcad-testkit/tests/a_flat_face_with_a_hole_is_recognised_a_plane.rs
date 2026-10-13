//! A FLAT FACE THAT HOLDS A HOLE IS RECOGNISED A PLANE. A plate 40 x 30 x 10 drilled 10 across right through comes in
//! as a mesh and is recognised into exact surfaces: its top and bottom, each a flat ring round the hole, are planes
//! like its four sides, and the top pushed up by 5 grows the plate by its own area. Built with the live kernel.
//!
//! Reported behaviour: the top and bottom came out free-form surfaces, [4 planes, 1 cylinder, 2 free-form], and the
//! top pushed up went red: a push refuses a free-form face beside a curved one.
use qymcad_core::feature::FaceKey;
use qymcad_core::model::{Id, ImportNode, Project};
use qymcad_kernel::Shape;

fn drilled() -> Shape {
    let plate = Shape::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate");
    let at = [1.0, 0.0, 0.0, 20.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, -1.0];
    let bore = Shape::cylinder(5.0, 12.0).expect("a cylinder").transformed(&at).expect("placed");
    plate.boolean(&bore, 0).expect("drilled")
}

/// The drilled plate come in as a mesh and recognised, with the node of the recognised body.
fn recognised() -> (Project, Id) {
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("plate.stl", vec![1]);
    let src = p.add_mesh(drilled().tessellate_merged(0.1).expect("a mesh").mesh);
    p.import_tree_as_parts(vec![ImportNode { name: "plate".into(), body: Some(src), mesh: true, ..Default::default() }], source, "plate").expect("came in");
    let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
    p.set_active_component(part);
    let body = p.add_mesh_recognised(src, 1.0, 30.0, 0.0);
    (p, body)
}

#[test]
fn the_rings_round_a_hole_are_planes() {
    let (mut p, body) = recognised();
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let solid = shapes.get(&body).unwrap_or_else(|| panic!("no body; the rebuild says {:?}", p.regen_errors));
    let kinds = solid.face_kinds().expect("face kinds");
    assert_eq!(kinds, [6, 1, 0, 0, 0, 0, 0], "plane, cylinder, cone, sphere, torus, free-form, other");
}

#[test]
fn the_top_round_a_hole_is_pushed_up() {
    let (mut p, body) = recognised();
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let before = shapes.get(&body).map(|s| s.volume()).unwrap_or_else(|| panic!("no body; the rebuild says {:?}", p.regen_errors));
    let top = p.regen_faces.get(&body).and_then(|fs| fs.iter().find(|f| f.normal[2] > 0.999).cloned()).expect("the top");
    let key = FaceKey { index: 0, centroid: [top.centroid.x, top.centroid.y, top.centroid.z], normal: top.normal, id: top.id };
    let push = p.add_push_face(body, key, 5.0);
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the push went red: {:?}", report.errors);
    let got = shapes.get(&push).map(|s| s.volume()).expect("the push is built");
    let want = before + (40.0 * 30.0 - std::f64::consts::PI * 25.0) * 5.0;
    assert!((got - want).abs() < 1e-3 * want, "pushed 5 it holds {got} mm^3, not {want}");
}
