//! A BOX AND A PRISM NAME THEIR FACES as an extrusion names them - a wall to every side, the two caps - so their edges
//! take names from the faces they part, and a query about edges finds them. Reported behaviour: "round every edge of
//! the top" of a box found no edge at all: its six faces carried positional numbers, and no edge had a name.
use qymcad_core::model::{Id, Project};
use qymcad_core::refs::{Axis, Cardinality, Query, Ref};

/// Every edge of the top face.
fn rim() -> Ref {
    Ref { expect: Cardinality::Some, ..Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true }))) }
}

/// The body rounded 2 along the rim of its top: its volume, the faces it has, and whether the rebuild was clean.
struct Rounded {
    volume: f64,
    faces: usize,
}

fn round_the_top(p: &mut Project, body: Id) -> Rounded {
    let _ = qymcad_testkit::regenerate(p);
    let unnamed = p.edge_pool(body).iter().filter(|c| p.names.edge(c.desc).is_none()).count();
    assert_eq!(unnamed, 0, "{unnamed} of the {} edges of the primitive have no name", p.edge_pool(body).len());
    let f = p.add_fillet_ref(body, 2.0, rim());
    let (report, shapes) = qymcad_testkit::regenerate(p);
    assert!(report.errors.is_empty(), "the rounding of the top went red: {:?}", report.errors);
    Rounded { volume: shapes.get(&f).map(|s| s.volume()).expect("the rounding is built"), faces: p.regen_faces.get(&f).map_or(0, Vec::len) }
}

#[test]
fn a_box_rounds_the_rim_of_its_top() {
    let mut p = Project::default();
    p.new_document();
    let b = p.add_box(40.0, 30.0, 10.0);
    let r = round_the_top(&mut p, b);
    // four edges of the top rounded: the block's six faces and four rounds, the corners of the top two more
    assert!(r.faces > 6 && r.volume < 12000.0 - 100.0 && r.volume > 12000.0 - 200.0, "the box rounded {} faces to {} mm^3", r.faces, r.volume);
}

#[test]
fn a_prism_rounds_the_rim_of_its_top() {
    let mut p = Project::default();
    p.new_document();
    let b = p.add_prism(10.0, 6, 20.0);
    let r = round_the_top(&mut p, b);
    let whole = 1.5 * 3f64.sqrt() * 100.0 * 20.0;
    assert!(r.faces > 8 && r.volume < whole && r.volume > whole - 200.0, "the prism rounded {} faces to {} of {whole} mm^3", r.faces, r.volume);
}
