//! THE REBUILD WITHOUT A WINDOW does what the window's does: the faces of a rebuilt body land in the body, a changed
//! parameter rebuilds what reads it and nothing else, and a body no node owns goes, its live shape with it.
use qymcad_core::model::{Id, Param, Project};
use qymcad_doc::regen;
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// A block of `w` x `d` on XY, extruded by the expression `height`. Returns the project and the block's body.
fn block(w: f64, d: f64, height: &str) -> (Project, Id) {
    let mut p = Project::default();
    p.new_document();
    p.parameters.push(Param { name: "h".into(), expr: "10".into(), value: 10.0 });
    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    p.add_rect_entity(si, 0.0, 0.0, w, d, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("the contour");
    let e = p.add_extrude_multi(sid, vec![cid], 1.0, qymcad_core::feature::Reach::Forward, 0.0, vec![]);
    let body = p.finish_base_body(e, 1);
    p.set_feat_dim(body, "height", height.into());
    (p, body)
}

fn top_of(p: &Project, body: Id) -> f64 {
    p.bodies[p.mesh_index(body).expect("the body has a mesh")].mesh.bounds().expect("a box").max.z
}

#[test]
fn the_faces_of_a_rebuilt_body_go_into_the_body() {
    let (mut p, body) = block(40.0, 30.0, "h");
    let (mut shapes, mut seen) = (HashMap::new(), HashMap::new());
    let done = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!(done.report.errors.is_empty(), "the block did not build: {:?}", done.report.errors);
    let faces = &p.bodies[p.mesh_index(body).expect("a mesh")].faces;
    assert_eq!(faces.len(), 6, "the block's own faces are not in the block: {} of them", faces.len());
    assert_eq!(Some(faces), p.regen_faces.get(&body), "the faces in the body are not the faces the rebuild named");
    assert!(shapes.contains_key(&body), "the block has no live shape");
}

#[test]
fn a_changed_parameter_rebuilds_what_reads_it_and_nothing_else() {
    let (mut p, body) = block(40.0, 30.0, "h");
    let (mut shapes, mut seen) = (HashMap::new(), HashMap::new());
    let _ = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!((top_of(&p, body) - 10.0).abs() < 1e-9, "the block stands {} tall, not h = 10", top_of(&p, body));
    // the parameters are kept by their name in lower case, as formulas find them
    assert_eq!(seen.get("h"), Some(&10.0), "the values seen at the rebuild are not remembered: {seen:?}");

    // nothing changed: nothing is rebuilt
    let idle = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!(idle.report.built.is_empty(), "a rebuild with nothing changed built {} bodies", idle.report.built.len());

    // h changes: the block that reads it is rebuilt at the new height
    p.parameters[0].expr = "15".into();
    p.parameters[0].value = 15.0;
    let after = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!(after.report.built.iter().any(|(b, _)| *b == body), "the block reading h was not rebuilt");
    assert!((top_of(&p, body) - 15.0).abs() < 1e-9, "the block stands {} tall, not h = 15", top_of(&p, body));
}

#[test]
fn a_body_no_node_owns_goes_with_its_shape() {
    let (mut p, body) = block(40.0, 30.0, "10");
    let ghost = p.add_mesh(qymcad_kernel::box_mesh(5.0, 5.0, 5.0, 0.1));
    let mut shapes: HashMap<Id, Shape> = HashMap::new();
    shapes.insert(ghost, Shape::cylinder(1.0, 1.0).expect("a cylinder"));
    let mut seen = HashMap::new();
    let _ = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!(p.mesh_index(ghost).is_none(), "the mesh no node owns is still in the document");
    assert!(!shapes.contains_key(&ghost), "the live shape of the body gone stayed in the cache");
    assert!(shapes.contains_key(&body), "the block lost its live shape");
}
