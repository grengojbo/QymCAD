//! A QUERY FINDS WHAT IT NAMES on a block, a cylinder and a block with two holes: the short names, the written-out
//! forms, edges through faces, a size as a number or an expression, a body by its part - and `expect: one` refuses two
//! with the count instead of taking the first.

use qymcad_core::model::{HoleTool, Id, Project};
use qymcad_doc::DocEngine;
use qymcad_mcp::args::{self, Amount, BodyRef, Element, Expect, QueryArg};
use serde_json::{json, Value};

/// A 40 x 30 x 10 block, centred on Z, standing on XY.
fn block(p: &mut Project) -> Id {
    p.add_box(40.0, 30.0, 10.0)
}

/// A cylinder of radius 10 and height 20 on XY.
fn cylinder(p: &mut Project) -> Id {
    p.add_cylinder(10.0, 20.0)
}

/// The block with two plain through holes of 5 mm in its top face, 10 mm either side of the centre along X.
/// A document and the body a check is about.
struct Laid {
    d: DocEngine,
    body: Id,
}

fn drilled() -> Laid {
    let mut d = DocEngine::blank();
    let mut body = d.edit("cmd-box", |p| Ok(block(p))).expect("the block is laid");
    for u in [-10.0, 10.0] {
        body = d
            .edit("cmd-hole", |p| {
                let top = p
                    .bodies
                    .iter()
                    .find(|b| b.id == body)
                    .and_then(|b| b.faces.iter().enumerate().find(|(_, f)| f.normal[2] > 0.999 && (f.centroid.z - 10.0).abs() < 1e-6))
                    .map(|(i, f)| qymcad_core::feature::FaceKey { index: i as u32, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id });
                let key = top.ok_or("no top face")?;
                let at = qymcad_doc::ops::FaceFrame { centre: key.centroid, normal: key.normal }.point_at(qymcad_doc::ops::FaceOffset { u, v: 0.0 });
                Ok(p.add_hole_at(body, key, at, HoleTool { kind: 0, diameter: 5.0, depth: 20.0, dia2: 0.0, depth2: 0.0 }))
            })
            .expect("the hole is drilled");
    }
    Laid { d, body }
}

fn laid(lay: fn(&mut Project) -> Id) -> Laid {
    let mut d = DocEngine::blank();
    let body = d.edit("cmd-prim", |p| Ok(lay(p))).expect("the body is laid");
    Laid { d, body }
}

fn query(q: Value) -> QueryArg {
    serde_json::from_value(q.clone()).unwrap_or_else(|e| panic!("{q} is not a query: {e}"))
}

/// Where the found faces stand: their centres, rounded to a micrometre and sorted.
fn centres(d: &DocEngine, body: Id, found: &[u32]) -> Vec<[f64; 3]> {
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    let mut c: Vec<[f64; 3]> = d.project().face_pool(body).iter().filter(|c| found.contains(&c.desc)).map(|c| c.centroid.map(round)).collect();
    c.sort_by(|a, b| a.partial_cmp(b).expect("finite centres"));
    c
}

/// One row: a query and how many faces it finds.
struct Row {
    query: Value,
    found: usize,
}

fn row(query: Value, found: usize) -> Row {
    Row { query, found }
}

/// Every row resolved as faces with `expect: any`; the rows that miss are reported together.
fn table(d: &DocEngine, body: Id, rows: &[Row]) -> Vec<String> {
    rows.iter()
        .filter_map(|r| {
            let at = args::resolve(d.project(), body, &args::reference(&query(r.query.clone()), Expect::Any), Element::Faces);
            match at {
                Ok(f) if f.len() == r.found => None,
                Ok(f) => Some(format!("{} found {} faces, not {}", r.query, f.len(), r.found)),
                Err(e) => Some(format!("{} was refused: {} {}", r.query, e.code, e.message)),
            }
        })
        .collect()
}

/// THE BLOCK: six faces, one to each side; the largest are top and bottom (40 x 30).
#[test]
fn a_block_answers_every_side() {
    let Laid { d, body } = laid(block);
    let faces = d.project().face_pool(body).len();
    assert_eq!(faces, 6, "a block of {faces} faces");
    let rows = [
        row(json!("all"), 6),
        row(json!("top"), 1),
        row(json!("bottom"), 1),
        row(json!("front"), 1),
        row(json!("back"), 1),
        row(json!("left"), 1),
        row(json!("right"), 1),
        row(json!("largest"), 2),
        row(json!({ "facing": { "dir": [1, 0, 0] } }), 1),
        row(json!({ "facing": { "dir": [1, 0, 1], "tol_deg": 46 } }), 2),
        row(json!({ "extreme": { "axis": "z", "max": false } }), 1),
        row(json!({ "union": ["top", "bottom", "top"] }), 2),
        row(json!({ "minus": { "from": "all", "take": "largest" } }), 4),
        row(json!({ "filter": { "from": "largest", "keep": "top" } }), 1),
    ];
    let wrong = table(&d, body, &rows);
    assert!(wrong.is_empty(), "the block:\n{}", wrong.join("\n"));

    let top = args::resolve(d.project(), body, &args::reference(&query(json!("top")), Expect::One), Element::Faces).expect("one top");
    assert_eq!(centres(&d, body, &top), [[0.0, 0.0, 10.0]], "top is not the face at Z 10");
    let front = args::resolve(d.project(), body, &args::reference(&query(json!("front")), Expect::One), Element::Faces).expect("one front");
    assert_eq!(centres(&d, body, &front), [[0.0, -15.0, 5.0]], "front is not the face at Y -15");
    let back = args::resolve(d.project(), body, &args::reference(&query(json!({ "ids": top })), Expect::One), Element::Faces).expect("the key finds its face");
    assert_eq!(back, top, "a face key did not find the same face");
}

/// THE CYLINDER: a side and two caps; the side is the largest (2 pi 10 20 = 1257 mm^2 against 314 for a cap).
#[test]
fn a_cylinder_answers_its_caps_and_its_side() {
    let Laid { d, body } = laid(cylinder);
    let rows = [row(json!("all"), 3), row(json!("top"), 1), row(json!("bottom"), 1), row(json!({ "facing": { "dir": [0, 0, -1] } }), 1)];
    let wrong = table(&d, body, &rows);
    assert!(wrong.is_empty(), "the cylinder:\n{}", wrong.join("\n"));
    let largest = args::resolve(d.project(), body, &args::reference(&query(json!("largest")), Expect::One), Element::Faces).expect("one largest");
    let caps = args::resolve(d.project(), body, &args::reference(&query(json!({ "union": ["top", "bottom"] })), Expect::Some), Element::Faces).expect("the caps");
    assert!(!caps.contains(&largest[0]), "a cap came out larger than the side");
    let top = args::resolve(d.project(), body, &args::reference(&query(json!("top")), Expect::One), Element::Faces).expect("one top");
    assert_eq!(centres(&d, body, &top), [[0.0, 0.0, 20.0]], "top is not the cap at Z 20");
}

/// THE DRILLED BLOCK: the walls of the holes are the hole's own, the top still one face, and the edges of the top
/// are its rim and the two rims of the holes.
#[test]
fn a_drilled_block_names_its_holes() {
    let Laid { d, body } = drilled();
    let p = d.project();
    let hole = p.timeline.last().map(|n| n.id).expect("the second hole");
    let rows = [
        row(json!("top"), 1),
        row(json!({ "of_feature": { "feature": hole, "role": "Hole" } }), 1),
        row(json!({ "minus": { "from": "all", "take": { "facing": { "dir": [0, 0, 1], "tol_deg": 89 } } } }), 7),
    ];
    let wrong = table(&d, body, &rows);
    assert!(wrong.is_empty(), "the drilled block:\n{}", wrong.join("\n"));
    let rim = args::resolve(p, body, &args::reference(&query(json!({ "adjacent": "top" })), Expect::Some), Element::Edges).expect("the edges of the top");
    assert!(rim.len() >= 6, "the top has {} edges: four of the rim and at least one to each hole", rim.len());
    let refused = args::resolve(p, body, &args::reference(&query(json!({ "adjacent": "top" })), Expect::Some), Element::Faces).expect_err("edges asked as faces");
    assert_eq!(refused.code, "wrong-element");
}

/// `expect: one` REFUSES TWO with the count; `some` refuses none; a key the body never had is lost.
#[test]
fn one_expected_and_two_found_is_refused() {
    let Laid { d, body } = laid(block);
    let two = args::resolve(d.project(), body, &args::reference(&query(json!("largest")), Expect::One), Element::Faces).expect_err("two largest taken as one");
    assert_eq!(two.code, "ref-ambiguous", "{}", two.message);
    assert!(two.message.contains('2'), "the count is not in the refusal: {}", two.message);
    let none = args::resolve(d.project(), body, &args::reference(&query(json!({ "facing": { "dir": [1, 1, 1], "tol_deg": 1 } })), Expect::Some), Element::Faces).expect_err("none taken as some");
    assert_eq!(none.code, "ref-lost", "{}", none.message);
    let any = args::resolve(d.project(), body, &args::reference(&query(json!({ "facing": { "dir": [1, 1, 1], "tol_deg": 1 } })), Expect::Any), Element::Faces).expect("any takes none");
    assert!(any.is_empty());
    let gone = args::resolve(d.project(), body, &args::reference(&query(json!({ "ids": [999_999] })), Expect::One), Element::Faces).expect_err("a key never there");
    assert_eq!(gone.code, "ref-lost");
}

/// A QUERY MISWRITTEN is refused while it is read: a short name not in the list, an object of an unknown key.
#[test]
fn a_miswritten_query_does_not_read() {
    for bad in [json!("upper"), json!({ "faces_up": true }), json!({ "facing": { "dir": [0, 0, 1], "tolerance": 3 } })] {
        assert!(serde_json::from_value::<QueryArg>(bad.clone()).is_err(), "{bad} was read as a query");
    }
    let names: Vec<Value> = args::query_schema()["oneOf"][0]["enum"].as_array().expect("the short names").clone();
    for n in &names {
        assert!(serde_json::from_value::<QueryArg>(n.clone()).is_ok(), "the schema offers {n}, which does not read");
    }
}

/// A SIZE is a number, or an expression the feature keeps; a plain number in a string is a number.
#[test]
fn a_size_is_a_number_or_an_expression() {
    let mut d = DocEngine::blank();
    d.edit("par-add", |p| {
        p.parameters.push(qymcad_core::model::Param { name: "w".into(), expr: "40".into(), value: 40.0 });
        qymcad_doc::params::settle(p);
        Ok(())
    })
    .expect("the parameter is set");
    let read = |a: Value| serde_json::from_value::<Amount>(a).expect("a size").read(d.project(), "depth");
    assert_eq!(read(json!(12.5)).expect("a number"), args::Read { value: 12.5, expr: None });
    assert_eq!(read(json!(" 7 ")).expect("a number in a string"), args::Read { value: 7.0, expr: None });
    assert_eq!(read(json!("w/2 + 1")).expect("an expression"), args::Read { value: 21.0, expr: Some("w/2 + 1".into()) });
    let bad = read(json!("q * 2")).expect_err("an unknown name");
    assert!(bad.code.starts_with("error-expr"), "{}", bad.code);
    assert!(bad.message.starts_with("depth"), "the field is not named: {}", bad.message);
}

/// A BODY BY ITS PART is the one body the part stands as; a body taken into another is refused and its heir named.
#[test]
fn a_body_by_its_part_and_one_taken_into_another() {
    let Laid { d, body: drilled_body } = drilled();
    let p = d.project();
    let part = p.body_owner(drilled_body).expect("the body has a part");
    assert_eq!(BodyRef::Part(part).resolve(p).expect("the part's body"), drilled_body);
    let first = p.bodies.first().map(|b| b.id).expect("the block");
    let taken = BodyRef::Body(first).resolve(p).expect_err("a consumed body taken");
    assert_eq!(taken.code, "consumed-body");
    assert!(taken.hint.as_deref().is_some_and(|h| h.contains(&drilled_body.to_string())), "the heir is not named: {:?}", taken.hint);
    assert_eq!(BodyRef::Body(987_654).resolve(p).expect_err("no such body").code, "no-body");
    assert_eq!(BodyRef::Part(987_654).resolve(p).expect_err("no such part").code, "no-part");
}
