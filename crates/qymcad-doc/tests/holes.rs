//! A HOLE THAT MEETS NO MATERIAL IS FOUND. Reported behaviour: four holes drawn on a plate whose points lay partly
//! past its edge - the plate stands centred on the origin - drilled one, and the three others cut nothing with no word
//! said. `ops::holes_in_air` names every hole whose axis crosses none of its body's faces within its depth.
use qymcad_core::model::{HoleTool, Id, Project};
use qymcad_doc::ops::holes_in_air;
use qymcad_doc::regen;
use std::collections::HashMap;

/// The plate 60 x 40 x 4 centred on the origin, rebuilt; the plate's body and the key of its top face.
fn plate() -> (Project, Id) {
    let mut p = Project::default();
    p.new_document();
    let body = p.add_box(60.0, 40.0, 4.0);
    let (mut shapes, mut seen) = (HashMap::new(), HashMap::new());
    let done = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!(done.report.errors.is_empty(), "the plate did not build: {:?}", done.report.errors);
    (p, body)
}

fn top(p: &Project, body: Id) -> qymcad_core::feature::FaceKey {
    let faces = &p.bodies[p.mesh_index(body).expect("a mesh")].faces;
    let (index, f) = faces.iter().enumerate().max_by(|a, b| a.1.centroid.z.total_cmp(&b.1.centroid.z)).expect("faces");
    qymcad_core::feature::FaceKey { index: index as u32, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id }
}

const M3: HoleTool = HoleTool { kind: 0, diameter: 3.4, depth: 10.0, dia2: 0.0, depth2: 0.0 };

#[test]
fn a_hole_past_the_edge_is_named_and_one_on_the_plate_is_not() {
    let (mut p, body) = plate();
    let face = top(&p, body);
    // a corner hole 8 in from both edges, and the same hole drawn as if the plate began at the origin
    let on = p.add_hole_at(body, face, [-22.0, -12.0, 4.0], M3);
    let off = p.add_hole_at(body, face, [52.0, 32.0, 4.0], M3);
    assert_eq!(holes_in_air(&p, on), Vec::<[f64; 3]>::new(), "a hole inside the plate is said to meet nothing");
    assert_eq!(holes_in_air(&p, off), vec![[52.0, 32.0, 4.0]], "a hole past the edge is not named");
}

#[test]
fn a_hole_on_the_rim_drilled_through_meets_the_plate() {
    // the very edge of the plate counts as the plate, and a hole deeper than the plate still meets it
    let (mut p, body) = plate();
    let face = top(&p, body);
    let rim = p.add_hole_at(body, face, [30.0, 0.0, 4.0], HoleTool { depth: 50.0, ..M3 });
    assert!(holes_in_air(&p, rim).is_empty(), "a hole on the rim of the plate is said to meet nothing");
}
