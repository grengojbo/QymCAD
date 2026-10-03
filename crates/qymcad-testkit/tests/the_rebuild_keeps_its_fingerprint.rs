//! THE REBUILD KEEPS ITS FINGERPRINT: a chain of a base, a pocket on its top face and a fillet chosen by a query is
//! rebuilt headless, and what comes out - the exact volume, the box, the faces, their names, the edges - is held to
//! the numbers measured on the rebuild as the window runs it today.
//!
//! The rebuild of the window and the rebuild of the headless harness are one sequence of steps written twice. This
//! check pins what that sequence gives on a chain that touches every part of it - a sketch on a face of the body
//! below, a cut that consumes its source, a reference that is a query rather than a number - so that moving the
//! sequence into one place, or any later edit of it, cannot change the document without turning this red.
//!
//! A second pass from scratch (every node dirty) must give the same fingerprint: the names of faces are interned
//! by role and source, not by the order they were met in.
use qymcad_core::feature::{FaceKey, SketchPlane};
use qymcad_core::model::{Id, Project};
use qymcad_core::refs::{Axis, Query, Ref};
use std::collections::HashMap;

/// What a body is, as far as a change of the rebuild could move it.
#[derive(Debug, PartialEq)]
struct Fingerprint {
    volume: f64,
    min: [f64; 3],
    max: [f64; 3],
    faces: usize,
    edges: usize,
    /// The persistent names of the faces, sorted: the names references hold on to.
    names: Vec<u32>,
}

/// The chain: a 40x30x10 block on XY, a 20x10 pocket 4 deep in the middle of its top face, every edge of the top
/// face rounded at 1. Returns the project and the body of the fillet.
fn chain() -> (Project, Id) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("the base contour");
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, qymcad_core::feature::Reach::Forward, 0.0, vec![]);
    let block = p.finish_base_body(e, 1);
    let _ = qymcad_testkit::regenerate(&mut p);

    let top = p.regen_faces.get(&block).and_then(|fs| fs.iter().find(|f| f.normal[2] > 0.9)).cloned().expect("the top face");
    let key = FaceKey { index: 0, centroid: [top.centroid.x, top.centroid.y, top.centroid.z], normal: top.normal, id: top.id };
    let s2 = p.new_sketch("pocket");
    let sid2 = p.sketches[s2].id;
    p.sketches[s2].plane = SketchPlane::Face(block, key);
    p.add_sketch_node(sid2, "pocket");
    // a sketch on an axis-aligned face keeps the world axes and the projected world origin: (20, 15) is the middle
    p.add_rect_entity(s2, 10.0, 10.0, 30.0, 20.0, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(s2);
    let cid2 = p.sketches[s2].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("the pocket contour");
    let span = qymcad_core::model::CombineSpan { height: 4.0, down: 0.0, extent: qymcad_core::feature::Extent { reach: qymcad_core::feature::Reach::Backward, ..Default::default() }, fill: &[] };
    let cut = p.add_combine_multi_op(block, sid2, vec![cid2], span, 0);
    let _ = qymcad_testkit::regenerate(&mut p);

    // the edges of the top face, by a query: the outer rim and the rim of the pocket
    let rim = Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })));
    let fillet = p.add_fillet_ref(cut, 1.0, rim);
    (p, fillet)
}

fn fingerprint(p: &Project, shapes: &HashMap<Id, qymcad_kernel::Shape>, body: Id) -> Fingerprint {
    let shape = shapes.get(&body).expect("the body has a live shape");
    let mesh = &p.bodies[p.mesh_index(body).expect("the body has a mesh")].mesh;
    let b = mesh.bounds().expect("the mesh has a box");
    let faces = p.regen_faces.get(&body).expect("the body has faces");
    let mut names: Vec<u32> = faces.iter().map(|f| f.id).collect();
    names.sort_unstable();
    Fingerprint { volume: shape.volume(), min: [b.min.x, b.min.y, b.min.z], max: [b.max.x, b.max.y, b.max.z], faces: faces.len(), edges: shape.edges_with_ids().1.len(), names }
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn the_rebuild_keeps_its_fingerprint() {
    let (mut p, fillet) = chain();
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the chain did not build: {:?}", report.errors);
    let seen = fingerprint(&p, &shapes, fillet);
    // Measured on the rebuild, and the volume agrees with the hand count:
    // 12000 of the block - 800 of the pocket - (1 - pi/4) x 1^2 over the 200 mm of the top face's edges = 11157.08;
    // 19 faces = 6 of the block + 5 of the pocket + 8 rounds.
    let want = Fingerprint { volume: 11157.079632644529, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0], faces: 19, edges: 40, names: (1073741824..=1073741843).filter(|n| *n != 1073741835).collect() };
    assert!(close(seen.volume, want.volume, 1e-6 * want.volume.abs()), "the volume moved: {} mm^3, measured {} mm^3", seen.volume, want.volume);
    for k in 0..3 {
        assert!(close(seen.min[k], want.min[k], 1e-6) && close(seen.max[k], want.max[k], 1e-6), "the box moved: {:?}..{:?}, measured {:?}..{:?}", seen.min, seen.max, want.min, want.max);
    }
    assert_eq!((seen.faces, seen.edges), (want.faces, want.edges), "the faces and edges moved");
    assert_eq!(seen.names, want.names, "the names of the faces moved");

    // the same document rebuilt from scratch gives the same body
    let (again, shapes_again) = qymcad_testkit::regenerate(&mut p);
    assert!(again.errors.is_empty(), "the second pass did not build: {:?}", again.errors);
    assert_eq!(fingerprint(&p, &shapes_again, fillet), seen, "a second pass from scratch gave another body");
}
