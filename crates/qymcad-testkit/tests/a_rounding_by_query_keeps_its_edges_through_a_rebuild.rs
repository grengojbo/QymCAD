//! A ROUNDING KEPT AS A QUERY ROUNDS WHAT IT ASKS FOR, and a chamfer cuts what it asks for, on every rebuild, also when its base is built again in the same
//! pass. Reported behaviour: the top front edge of a block rounded by "between the top and the front", the base
//! suppressed and taken back in - and the whole block came back rounded, 26 faces where 7 were, the node green.
use qymcad_core::feature::{Purpose, Reach};
use qymcad_core::model::{Id, Project};
use qymcad_core::refs::{Axis, Query, Ref};

/// A 40 x 30 x 10 block extruded from a rectangle on XY; the node of the extrusion.
fn block(p: &mut Project) -> Id {
    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, Reach::Forward, 0.0, vec![]);
    p.finish_base_body(e, 1)
}

/// The faces and the volume of the rounding.
struct Rounded {
    faces: usize,
    volume: f64,
}

fn rounded(p: &mut Project, fillet: Id) -> Rounded {
    let (report, shapes) = qymcad_testkit::regenerate(p);
    assert!(report.errors.is_empty(), "the rebuild went red: {:?}", report.errors);
    let volume = shapes.get(&fillet).map(|s| s.volume()).expect("the rounding is built");
    Rounded { faces: p.regen_faces.get(&fillet).map_or(0, Vec::len), volume }
}

#[test]
fn a_rounding_by_query_keeps_its_edge_when_its_base_comes_back() {
    let mut p = Project::default();
    p.new_document();
    let base = block(&mut p);
    let _ = qymcad_testkit::regenerate(&mut p);
    let edge = Query::Between(Box::new(Query::Extreme { axis: Axis::Z, max: true }), Box::new(Query::Extreme { axis: Axis::Y, max: false }));
    let fillet = p.add_fillet_ref(base, 2.0, Ref { expect: qymcad_core::refs::Cardinality::Some, ..Ref::many(edge) });
    // 12000 less a quarter of a 2 x 2 square along the 40 of the edge, less the quarter disc
    let want = 12000.0 - 160.0 * (1.0 - std::f64::consts::FRAC_PI_4);
    let first = rounded(&mut p, fillet);
    assert_eq!(first.faces, 7, "the first rounding took more than its edge");
    assert!((first.volume - want).abs() < 1e-6, "the first rounding measures {}", first.volume);

    let ti = p.timeline.iter().position(|n| n.id == base).expect("the extrusion in the timeline");
    let _ = p.set_feature_suppressed(ti, true);
    let _ = qymcad_testkit::regenerate(&mut p);
    let _ = p.set_feature_suppressed(ti, false);
    let again = rounded(&mut p, fillet);
    assert_eq!(again.faces, 7, "the base built again, the rounding came back with {} faces", again.faces);
    assert!((again.volume - want).abs() < 1e-6, "the base built again, the rounding measures {} not {want}", again.volume);
}

/// A QUERY THAT FINDS NO EDGE is a lost reference, red, and never "every edge": an empty list of edges rounds the whole
/// body, which is what "round everything" asks and not what a description that matched nothing asks.
#[test]
fn a_rounding_whose_query_finds_nothing_goes_red() {
    let mut p = Project::default();
    p.new_document();
    let base = block(&mut p);
    let _ = qymcad_testkit::regenerate(&mut p);
    let nothing = Query::Oriented { dir: [1.0, 1.0, 1.0], tol_deg: 1.0 };
    for r in [Ref::many(nothing.clone()), Ref { expect: qymcad_core::refs::Cardinality::Some, ..Ref::many(nothing.clone()) }] {
        let fillet = p.add_fillet_ref(base, 2.0, r);
        let _ = qymcad_testkit::regenerate(&mut p);
        let faces = p.regen_faces.get(&fillet).map_or(0, Vec::len);
        assert!(p.regen_errors.contains_key(&fillet), "a rounding that found no edge stands green with {faces} faces");
        assert!(faces <= 6, "a rounding that found no edge rounded the block: {faces} faces");
        let _ = p.delete_feature_with_dependents(fillet);
    }
    let chamfer = p.add_chamfer_ref(base, 1.0, Ref::many(nothing));
    let _ = qymcad_testkit::regenerate(&mut p);
    let faces = p.regen_faces.get(&chamfer).map_or(0, Vec::len);
    assert!(p.regen_errors.contains_key(&chamfer) && faces <= 6, "a chamfer that found no edge cut the block: {faces} faces");
}

/// A ROUNDING REBUILT ON A BASE WHOSE EDGES THE MODEL DOES NOT HOLD - a document just opened from its file, which keeps
/// meshes and faces but not edges - finds its edges on the live body. Reported behaviour: the rounding reopened right
/// after opening said "none of the 0 named edges is left" and went red, until anything rebuilt the base.
#[test]
fn a_rounding_rebuilt_alone_finds_its_edges_on_the_live_base() {
    let mut p = Project::default();
    p.new_document();
    let base = block(&mut p);
    let _ = qymcad_testkit::regenerate(&mut p);
    let edge = Query::Between(Box::new(Query::Extreme { axis: Axis::Z, max: true }), Box::new(Query::Extreme { axis: Axis::Y, max: false }));
    let fillet = p.add_fillet_ref(base, 2.0, Ref { expect: qymcad_core::refs::Cardinality::Some, ..Ref::many(edge) });
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    // as an opened file stands: the base live and clean, no edges in the model, the rounding to be built again alone
    p.regen_edges.clear();
    if let Some(n) = p.timeline.iter_mut().find(|n| n.id == fillet) {
        n.dirty = true;
    }
    let (report, _) = qymcad_testkit::regenerate_dirty_with_shapes(&mut p, shapes);
    assert!(report.errors.is_empty(), "the rounding rebuilt alone went red: {:?}", report.errors);
    let faces = p.regen_faces.get(&fillet).map_or(0, Vec::len);
    assert_eq!(faces, 7, "the rounding rebuilt alone came back with {faces} faces");
}
