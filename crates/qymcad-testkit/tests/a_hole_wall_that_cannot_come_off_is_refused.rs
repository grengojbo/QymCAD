//! FACES THAT CANNOT COME OFF ARE REFUSED, NOT A CRASH: the wall of a hole taken off a polyhedron made of a fine mesh,
//! the chamfer round its mouth left standing - the faces beside it cannot grow over the hole - gives a red node with its
//! reason, and the program goes on. Built with the live kernel.
//!
//! Reported behaviour: the plate drilled 10 with its rim cut back 1, meshed at 0.01 and made a polyhedron, its hole read
//! blind and its wall alone taken off - the process died with signal 6.
use qymcad_core::errors::CoreError;
use qymcad_core::feature::FaceKey;
use qymcad_core::model::{ImportNode, Project};
use qymcad_kernel::specimen::Specimen;
use qymcad_kernel::Shape;

/// The plate 40 x 30 x 10 drilled 10 through round (20, 15), the rim of the hole at the top cut back 1.
fn cut_back_plate() -> Shape {
    let plate = qymcad_kernel::specimen::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate");
    let at = [1.0, 0.0, 0.0, 20.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, -1.0];
    let drill = qymcad_kernel::specimen::cylinder(5.0, 12.0).expect("a cylinder").moved(&at).expect("placed");
    let p = plate.combined(&drill, 0).expect("drilled");
    let mouth: Vec<u32> =
        p.edges_info().into_iter().filter(|e| e.poly.iter().all(|q| (q[2] as f64 - 10.0).abs() < 1e-6 && ((q[0] as f64 - 20.0).hypot(q[1] as f64 - 15.0) - 5.0).abs() < 1e-3)).map(|e| e.id).collect();
    p.chamfered(1.0, &mouth).expect("the rim cut back")
}

#[test]
fn the_wall_of_a_hole_that_cannot_come_off_is_refused() {
    let mesh = qymcad_kernel::specimen::mesh_merged(&cut_back_plate(), 0.01).expect("a mesh").mesh;
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("plate.stl", vec![1]);
    let src = p.add_mesh(mesh);
    p.import_tree_as_parts(vec![ImportNode { name: "plate".into(), body: Some(src), mesh: true, ..Default::default() }], source, "plate").expect("the mesh came in");
    let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
    p.set_active_component(part);
    let body = p.add_mesh_solid(src, 0.0);
    let _ = qymcad_testkit::regenerate(&mut p);
    // the strips of the wall: upright, within the bore, below the chamfer
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let wall: Vec<FaceKey> = faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.normal[2].abs() < 0.1 && (f.centroid.x - 20.0).hypot(f.centroid.y - 15.0) < 5.01 && f.centroid.z < 9.0)
        .map(|(i, f)| FaceKey { index: i as u32, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id })
        .collect();
    assert!(!wall.is_empty(), "no strip of the wall");
    let node = p.add_remove_face(body, wall);
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    let refused = report.errors.iter().find(|(n, _)| *n == node).map(|(_, e)| e.clone());
    assert!(matches!(refused, Some(CoreError::RemoveFacesFailed { .. })), "the wall came off, or was refused for another reason: {refused:?}");
    // and the program goes on: the body before the node still stands
    assert!(p.mesh_index(body).is_some(), "the body before the removal is gone");
}
