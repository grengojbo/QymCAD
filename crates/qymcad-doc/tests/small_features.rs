//! WHAT IS TOO SMALL FOR THE NOZZLE, on bodies whose size is known: a pin of radius 0.15 is thinner than a nozzle of
//! 0.4 and is told as a pin where it stands; a plate 0.3 thick has its four upright edges shorter than the nozzle; a
//! cylinder of radius 5 and a cube of 10 have nothing too small.
use qymcad_core::model::Id;
use qymcad_doc::print::checks::Verdict;
use qymcad_doc::print::small::{small_features, Round};
use qymcad_doc::DocEngine;

/// A document and its one body.
struct One {
    d: DocEngine,
    body: Id,
}

/// A document holding the base body `make` lays.
fn one_body(make: impl FnOnce(&mut qymcad_core::model::Project) -> Id) -> One {
    let mut d = DocEngine::blank();
    let body = d.edit("body", |p| {
        let b = make(p);
        Ok(p.finish_base_body(b, 1))
    });
    let body = body.expect("the body is laid");
    One { d, body }
}

#[test]
fn a_pin_thinner_than_the_nozzle_is_told() {
    let One { d, body } = one_body(|p| p.add_cylinder(0.15, 5.0));
    let s = small_features(d.project(), d.shape(body).expect("a live body"), body, 0.4);
    assert_eq!(s.verdict, Verdict::Warn, "{s:?}");
    assert_eq!(s.rounds.len(), 1, "{s:?}");
    assert_eq!(s.rounds[0].kind, Round::Pin, "{s:?}");
    assert!((s.rounds[0].radius - 0.15).abs() < 1e-6, "{s:?}");
}

#[test]
fn a_plate_thinner_than_the_nozzle_has_short_edges() {
    let One { d, body } = one_body(|p| p.add_box(10.0, 10.0, 0.3));
    let s = small_features(d.project(), d.shape(body).expect("a live body"), body, 0.4);
    assert_eq!(s.verdict, Verdict::Warn, "{s:?}");
    assert_eq!(s.edges.len(), 4, "the four upright edges of 0.3: {s:?}");
    assert!(s.edges.iter().all(|e| (e.length - 0.3).abs() < 1e-6), "{s:?}");
}

#[test]
fn bodies_of_a_size_have_nothing_too_small() {
    for One { d, body } in [one_body(|p| p.add_cylinder(5.0, 10.0)), one_body(|p| p.add_box(10.0, 10.0, 10.0))] {
        let s = small_features(d.project(), d.shape(body).expect("a live body"), body, 0.4);
        assert_eq!(s.verdict, Verdict::Pass, "{s:?}");
    }
}
