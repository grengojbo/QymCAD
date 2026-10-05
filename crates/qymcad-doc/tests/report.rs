//! THE ACCOUNT OF A DOCUMENT gives the numbers the document has: a block with a pocket and its top rounded measures
//! what that chain measures in the rebuild's own fingerprint, a red feature carries its error, and the parameters and
//! the parts stand as they were put in.
use qymcad_core::feature::{FaceKey, Purpose, Reach, SketchPlane};
use qymcad_core::model::{CombineSpan, Id, Param, Project};
use qymcad_core::refs::{Axis, Query, Ref};
use qymcad_doc::report::Report;
use qymcad_doc::DocEngine;

/// A 40 x 30 x 10 block on XY; its body.
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

/// A 20 x 10 pocket 4 deep in the middle of the top face of `body`; the body of the cut.
fn pocket(p: &mut Project, body: Id) -> Id {
    let top = p.regen_faces.get(&body).and_then(|fs| fs.iter().find(|f| f.normal[2] > 0.9)).cloned().expect("the top face");
    let key = FaceKey { index: 0, centroid: [top.centroid.x, top.centroid.y, top.centroid.z], normal: top.normal, id: top.id };
    let si = p.new_sketch("pocket");
    let sid = p.sketches[si].id;
    p.sketches[si].plane = SketchPlane::Face(body, key);
    p.add_sketch_node(sid, "pocket");
    p.add_rect_entity(si, 10.0, 10.0, 30.0, 20.0, Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("the pocket contour");
    let span = CombineSpan { height: 4.0, down: 0.0, extent: qymcad_core::feature::Extent { reach: Reach::Backward, ..Default::default() }, fill: &[] };
    p.add_combine_multi_op(body, sid, vec![cid], span, 0)
}

fn shown(stored: &str) -> String {
    stored.to_string()
}

fn the_body(r: &Report, id: Id) -> &qymcad_doc::report::Body {
    r.bodies.iter().find(|b| b.id == id).unwrap_or_else(|| panic!("no body {id} in the account"))
}

/// THE CHAIN OF THE REBUILD'S FINGERPRINT, each step an action: 12000 - 800 - (1 - pi/4) x 200 = 11157.08 mm^3, 19
/// faces, 40 edges, the box of the block.
#[test]
fn a_pocketed_block_measures_what_the_rebuild_measures() {
    let mut d = DocEngine::blank();
    let base = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    let cut = d.edit("pocket", |p| Ok(pocket(p, base))).expect("the pocket is laid");
    let rim = Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })));
    let fillet = d.edit("fillet", |p| Ok(p.add_fillet_ref(cut, 1.0, rim))).expect("the fillet is laid");
    let r = d.document(&shown);

    let b = the_body(&r, fillet);
    let expected = 12000.0 - 800.0 - (1.0 - std::f64::consts::FRAC_PI_4) * 200.0;
    assert!((b.volume - expected).abs() < 1e-6 * expected, "the rounded block measures {} mm^3, not {expected}", b.volume);
    assert_eq!(b.faces, 19, "the faces of the rounded block");
    assert_eq!(b.edges, Some(40), "the edges of the rounded block");
    let near = |got: [f64; 3], want: [f64; 3]| got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-6);
    assert!(near(b.min, [0.0; 3]) && near(b.max, [40.0, 30.0, 10.0]), "the box is {:?}..{:?}, not the block's", b.min, b.max);
    assert!(b.area > 0.0 && !b.consumed && !b.sheet, "{b:?}");
    assert!(the_body(&r, base).consumed && the_body(&r, cut).consumed, "the block and the cut are taken into the fillet and stand in the account as such");
    assert_eq!(r.bodies.iter().filter(|b| !b.consumed).count(), 1, "one body stands on its own");

    assert_eq!(r.parts.len(), 1, "the new document has one part: {:?}", r.parts);
    let part = r.parts[0].key;
    assert_eq!(b.part, Some(part), "the body is not in the part");
    let kinds: Vec<&str> = r.features.iter().map(|f| f.kind.as_str()).collect();
    assert_eq!(kinds.last(), Some(&"Fillet"), "the timeline ends elsewhere: {kinds:?}");
    assert!(r.features.iter().all(|f| f.error.is_none() && f.part == Some(part)), "a feature stands red or outside the part: {:?}", r.features);
}

/// A FEATURE THAT DOES NOT BUILD stands red in the account with its error; a parameter stands as it was put in.
#[test]
fn a_red_feature_carries_its_error() {
    let mut d = DocEngine::blank();
    let base = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    d.edit("parameter", |p| {
        p.parameters.push(Param { name: "wall".into(), expr: "2 + 1".into(), value: 3.0 });
        Ok(())
    })
    .expect("the parameter is laid");
    let rim = Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })));
    let _ = d.edit("fillet", |p| Ok(p.add_fillet_ref(base, 50.0, rim))).expect("the fillet is laid, red");
    let r = d.document(&shown);
    let node = r.features.iter().find(|f| f.kind == "Fillet").expect("the fillet is in the timeline");
    let error = node.error.as_ref().unwrap_or_else(|| panic!("a 50 mm round on a 10 mm block stands green: {node:?}"));
    assert!(error.key().starts_with("error-"), "the error has no code: {}", error.key());
    assert_eq!(r.parameters, vec![qymcad_doc::report::Parameter { name: "wall".into(), expr: "2 + 1".into(), value: 3.0 }]);
}
