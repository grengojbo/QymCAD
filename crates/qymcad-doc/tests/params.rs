//! A PARAMETER CHANGED REACHES THE BODY through a sketch: a rectangle whose width is the expression `w` is solved
//! again, its extrusion rebuilt, and undo takes the whole back.
use qymcad_core::feature::{Purpose, Reach};
use qymcad_core::model::{Constraint, Id, Param, Project};
use qymcad_doc::DocEngine;

/// A rectangle 0..40 x 0..30 whose width is held to the expression `w`, extruded 10; its body.
fn driven_block(p: &mut Project) -> Id {
    p.parameters.push(Param { name: "w".into(), expr: "40".into(), value: 40.0 });
    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    let _ = p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
    let a = p.sketch_point_at(si, 0.0, 0.0, 1e-6);
    let b = p.sketch_point_at(si, 40.0, 0.0, 1e-6);
    p.sketches[si].constraints.push(Constraint::Fixed { p: a });
    p.sketches[si].constraints.push(Constraint::Distance { a, b, d: 40.0, off: 0.0, expr: "w".into(), driven: false, axis: 0, at: None });
    p.solve_sketch(si);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, Reach::Forward, 0.0, vec![]);
    p.finish_base_body(e, 1)
}

fn volume(d: &DocEngine, body: Id) -> f64 {
    d.shape(body).map(|s| s.volume()).expect("the body is live")
}

#[test]
fn a_changed_parameter_reaches_the_body_through_its_sketch() {
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(driven_block(p))).expect("the block is laid");
    assert!((volume(&d, body) - 12000.0).abs() < 1e-6, "the block measures {}", volume(&d, body));
    d.edit("par-edit-step", |p| {
        p.parameters[0].expr = "60".into();
        qymcad_doc::params::settle(p);
        Ok(())
    })
    .expect("the parameter is changed");
    assert!((d.project().parameters[0].value - 60.0).abs() < 1e-12, "the value was not evaluated");
    assert!((volume(&d, body) - 18000.0).abs() < 1e-3, "w = 60 did not reach the block: {} mm^3", volume(&d, body));
    assert_eq!(d.undo().as_deref(), Some("par-edit-step"));
    assert!((volume(&d, body) - 12000.0).abs() < 1e-6, "undo did not take the block back: {} mm^3", volume(&d, body));
}
