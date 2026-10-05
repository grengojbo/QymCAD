//! FEATURES FROM A SKETCH through the server: a pocket cut from a sketch on the top of a block goes into the block by
//! itself, a cut taken forward instead cuts the void, a cut through goes the whole way, a size of the sketch named and
//! changed reaches the body, two contours are not chosen between, and a revolution turns about a line of its sketch.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn ok(reply: Value) -> Value {
    assert_eq!(reply["ok"], json!(true), "{reply}");
    reply
}

/// The one body standing: its volume, faces and edges.
struct Measured {
    volume: f64,
    faces: u64,
    edges: u64,
}

fn measured(reply: &Value) -> Measured {
    let bodies = reply["bodies"].as_array().unwrap_or_else(|| panic!("no bodies in {reply}"));
    assert_eq!(bodies.len(), 1, "the part stands as {} bodies: {reply}", bodies.len());
    let b = &bodies[0];
    Measured { volume: b["volume"].as_f64().expect("a volume"), faces: b["faces"].as_u64().expect("faces"), edges: b["edges"].as_u64().unwrap_or(0) }
}

/// A sketch on `plane` holding what `entities` draws; its key.
fn sketch(ctx: &mut Ctx, plane: Value, entities: Value) -> u64 {
    let made = ok(call(ctx, "create_sketch", json!({ "plane": plane })));
    let key = made["sketch"].as_u64().expect("a sketch");
    let _ = ok(call(ctx, "sketch_add", json!({ "sketch": key, "entities": entities })));
    key
}

/// The 40 x 30 x 10 block from the origin on XY; the key of its body.
fn block(ctx: &mut Ctx) -> u64 {
    let base = sketch(ctx, json!("xy"), json!([{ "rect": { "from": [0, 0], "to": [40, 30] } }]));
    let laid = ok(call(ctx, "extrude", json!({ "sketch": base, "distance": 10 })));
    let m = measured(&laid);
    assert!((m.volume - 12000.0).abs() < 1e-6, "{laid}");
    laid["body"].as_u64().expect("a body")
}

/// A POCKET from a sketch on the top face goes into the block with no direction given: 12000 - 20 x 10 x 4 = 11200,
/// the block's six faces and the pocket's five, its twelve edges and the pocket's twelve.
#[test]
fn a_pocket_on_a_face_goes_into_the_body() {
    let mut ctx = Ctx::blank();
    let body = block(&mut ctx);
    let top = sketch(&mut ctx, json!({ "body": { "body": body }, "face": "top" }), json!([{ "rect": { "from": [10, 10], "to": [30, 20] } }]));
    let cut = ok(call(&mut ctx, "extrude", json!({ "sketch": top, "distance": 4, "op": "cut" })));
    let m = measured(&cut);
    assert!((m.volume - 11200.0).abs() < 1e-6, "the pocket measures {}: {cut}", m.volume);
    assert_eq!(m.faces, 11, "{cut}");
    assert_eq!(m.edges, 24, "{cut}");
    assert_eq!(cut["red"], json!([]), "{cut}");

    let undone = ok(call(&mut ctx, "undo", json!({})));
    assert_eq!(undone["undone"]["step"], "f-extrusion", "the cut is not one step: {undone}");
    let void = call(&mut ctx, "extrude", json!({ "sketch": top, "distance": 4, "op": "cut", "direction": "forward" }));
    let m = measured(&void);
    assert!((m.volume - 12000.0).abs() < 1e-6, "a cut taken forward off the top took something: {void}");
}

/// A CUT THROUGH goes the whole way, however far that is: a circle of radius 5 on the top takes pi x 25 x 10.
#[test]
fn a_cut_through_goes_the_whole_way() {
    let mut ctx = Ctx::blank();
    let body = block(&mut ctx);
    let top = sketch(&mut ctx, json!({ "body": { "body": body }, "face": "top" }), json!([{ "circle": { "centre": [20, 15], "radius": 5 } }]));
    let cut = ok(call(&mut ctx, "extrude", json!({ "sketch": top, "op": "cut", "reaches": "through" })));
    let want = 12000.0 - std::f64::consts::PI * 25.0 * 10.0;
    assert!((measured(&cut).volume - want).abs() < 1e-3, "{cut}");
    let alone = call(&mut ctx, "extrude", json!({ "sketch": top, "op": "cut" }));
    assert_eq!(alone["error"]["code"], "arguments", "a cut with no distance and not through went: {alone}");
}

/// A PARAMETER DRIVING A SIZE OF THE SKETCH reaches the body: the width held to `w` made 50 gives 50 x 30 x 10, and the
/// height given as an expression follows its parameter. A named size of the sketch is read, not set: the parameter is
/// the door.
#[test]
fn a_parameter_driving_a_size_of_the_sketch_reaches_the_body() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "set_parameter", json!({ "name": "t", "expr": "10" })));
    let _ = ok(call(&mut ctx, "set_parameter", json!({ "name": "w", "expr": "40" })));
    let made = ok(call(&mut ctx, "create_sketch", json!({ "plane": "xy" })));
    let s = made["sketch"].as_u64().expect("a sketch");
    let _ = ok(call(
        &mut ctx,
        "sketch_add",
        json!({ "sketch": s, "entities": [{ "rect": { "from": [0, 0], "to": [40, 30], "as": "r" } }], "constraints": [{ "coincident": ["r.bl", "origin"] }], "dimensions": [{ "length": { "line": "r.bottom", "value": "w" } }, { "length": { "line": "r.left", "value": 30, "name": "depth" } }] }),
    ));
    let laid = ok(call(&mut ctx, "extrude", json!({ "sketch": s, "distance": "t" })));
    assert!((measured(&laid).volume - 12000.0).abs() < 1e-6, "{laid}");
    let wider = ok(call(&mut ctx, "set_parameter", json!({ "name": "w", "expr": "50" })));
    assert!((measured(&wider).volume - 15000.0).abs() < 1e-6, "w = 50 did not reach the body: {wider}");
    let thicker = ok(call(&mut ctx, "set_parameter", json!({ "name": "t", "expr": "4" })));
    assert!((measured(&thicker).volume - 6000.0).abs() < 1e-6, "t = 4 did not reach the height: {thicker}");
    let read = ok(call(&mut ctx, "set_parameter", json!({ "name": "half", "expr": "depth / 2" })));
    assert_eq!(read["parameter"]["value"], json!(15.0), "the named size is not read: {read}");
    let set = call(&mut ctx, "set_parameter", json!({ "name": "depth", "expr": "20" }));
    assert_eq!(set["error"]["code"], "taken-name", "a named size of a sketch was set as a parameter: {set}");
}

/// TWO CONTOURS ARE NOT CHOSEN BETWEEN: with no contours named the call is refused with both keys; named, both are
/// taken in one node.
#[test]
fn two_contours_are_named_not_guessed() {
    let mut ctx = Ctx::blank();
    let s = sketch(&mut ctx, json!("xy"), json!([{ "rect": { "from": [0, 0], "to": [10, 10] } }, { "rect": { "from": [20, 0], "to": [30, 10] } }]));
    let refused = call(&mut ctx, "extrude", json!({ "sketch": s, "distance": 5 }));
    assert_eq!(refused["error"]["code"], "several-contours", "{refused}");
    let info = ok(call(&mut ctx, "sketch_info", json!({ "sketch": s })));
    let keys: Vec<Value> = info["contours"].as_array().expect("contours").iter().map(|c| c["id"].clone()).collect();
    let both = ok(call(&mut ctx, "extrude", json!({ "sketch": s, "distance": 5, "contours": keys })));
    let total: f64 = both["bodies"].as_array().expect("bodies").iter().map(|b| b["volume"].as_f64().unwrap_or(0.0)).sum();
    assert!((total - 1000.0).abs() < 1e-6, "two squares of 10 x 10 x 5: {both}");
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    let extrusions = doc["document"]["features"].as_array().expect("features").iter().filter(|f| f["kind"] == "Extrude").count();
    assert_eq!(extrusions, 1, "two contours made more than one node: {doc}");
}

/// A REVOLUTION ABOUT A LINE OF ITS SKETCH: a 10 x 20 rectangle beside a construction line at x = -5 turned a whole
/// way makes a ring of radii 5 and 15, pi x (225 - 25) x 20.
#[test]
fn a_revolution_turns_about_a_line_of_its_sketch() {
    let mut ctx = Ctx::blank();
    let made = ok(call(&mut ctx, "create_sketch", json!({ "plane": "xz" })));
    let s = made["sketch"].as_u64().expect("a sketch");
    let drawn = ok(call(
        &mut ctx,
        "sketch_add",
        json!({ "sketch": s, "entities": [{ "rect": { "from": [0, 0], "to": [10, 20] } }, { "line": { "from": [-5, -10], "to": [-5, 30], "as": "axis", "purpose": "construction" } }] }),
    ));
    let info = ok(call(&mut ctx, "sketch_info", json!({ "sketch": s })));
    let ends = drawn["names"]["axis"]["line"].clone();
    let line = info["entities"].as_array().expect("entities").iter().find(|e| e["points"] == ends).map(|e| e["key"].clone()).expect("the axis line");
    let ring = ok(call(&mut ctx, "revolve", json!({ "sketch": s, "axis": { "line": line } })));
    let want = std::f64::consts::PI * (225.0 - 25.0) * 20.0;
    assert!((measured(&ring).volume - want).abs() < 1e-3 * want, "{ring}");
    let undone = ok(call(&mut ctx, "undo", json!({})));
    assert_eq!(undone["undone"]["step"], "f-revolution");
    let half = ok(call(&mut ctx, "revolve", json!({ "sketch": s, "axis": { "line": line }, "angle": "90 * 2" })));
    assert!((measured(&half).volume - want / 2.0).abs() < 1e-3 * want, "{half}");
    let wrong = call(&mut ctx, "revolve", json!({ "sketch": s, "axis": { "line": 987_654 } }));
    assert_eq!(wrong["error"]["code"], "wrong-kind", "{wrong}");
}
