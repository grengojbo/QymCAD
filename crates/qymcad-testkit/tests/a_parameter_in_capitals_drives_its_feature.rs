//! A PARAMETER IN CAPITALS DRIVES ITS FEATURE: a name is the same name whatever its case, in the rebuild as in
//! the evaluation.
//!
//! Reported behaviour: a parameter `H` = 10 used as `H` in an extrude height; `H` became 15 in the parameter table
//! and the extrude stayed 10 tall. A lower-case name worked. The table marks dependents by comparing `param_map`
//! (lower-cased keys) with the values of the last rebuild and asks for the dependents of each changed name.
use qymcad_core::geom::Point2;
use qymcad_core::model::{Param, Project};
use std::collections::HashMap;

#[test]
fn an_extrude_of_height_h_follows_h() {
    let mut p = Project::default();
    p.new_document();
    p.parameters.push(Param { name: "H".into(), expr: "10".into(), value: 10.0 });
    let sid = p.add_line_sketch("Sketch 1", vec![Point2::new(0.0, 0.0), Point2::new(20.0, 0.0), Point2::new(20.0, 10.0), Point2::new(0.0, 10.0)], true);
    let ext = p.add_extrude_multi(sid, Vec::new(), 10.0, qymcad_core::feature::Reach::Forward, 0.0, Vec::new());
    p.set_feat_dim(ext, "height", "H".into());
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the extrude did not build: {:?}", report.errors);
    let tall = |shapes: &HashMap<u64, qymcad_kernel::Shape>| shapes.get(&ext).and_then(|s| s.bbox()).map(|b| b[5] - b[2]);
    assert!(tall(&shapes).is_some_and(|z| (z - 10.0).abs() < 1e-3), "H = 10 gives a body {:?} tall", tall(&shapes));

    // the edit of the table: the new value, then the comparison against the last rebuild, then a rebuild of what
    // is dirty
    let seen = p.param_map();
    if let Some(h) = p.parameters.iter_mut().find(|q| q.name == "H") {
        h.expr = "15".into();
        h.value = 15.0;
    }
    let now = p.param_map();
    let changed: Vec<String> = now.iter().filter(|(k, v)| seen.get(*k).is_none_or(|old| (*old - **v).abs() > 1e-12)).map(|(k, _)| k.clone()).collect();
    assert_eq!(changed, vec!["h".to_string()], "the comparison sees the lower-cased name");
    for name in &changed {
        p.mark_param_dependents_dirty_for(name);
    }
    let (report, shapes) = qymcad_testkit::regenerate_dirty_with_shapes(&mut p, shapes);
    assert!(report.errors.is_empty(), "the extrude of 15 did not build: {:?}", report.errors);
    assert!(tall(&shapes).is_some_and(|z| (z - 15.0).abs() < 1e-3), "H became 15 and the body is {:?} tall", tall(&shapes));
}
