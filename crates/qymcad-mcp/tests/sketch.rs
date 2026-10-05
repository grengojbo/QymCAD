//! A SKETCH DRAWN BY NAMING: a rectangle held to the origin by two named dimensions is fully held and makes one closed
//! contour, its names are read by formulas, a size it already holds becomes a reference one, a sketch stands on a face
//! of a body and on an offset datum, and a call that names nothing is refused whole.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn sketch_on(ctx: &mut Ctx, plane: Value) -> u64 {
    let made = call(ctx, "create_sketch", json!({ "plane": plane }));
    made["sketch"].as_u64().unwrap_or_else(|| panic!("no sketch in {made}"))
}

/// The rectangle 40 x 30 from the origin, its corner on the origin, its sides named `w` and `h`.
fn held_rectangle(ctx: &mut Ctx, sketch: u64) -> Value {
    call(
        ctx,
        "sketch_add",
        json!({ "sketch": sketch,
            "entities": [{ "rect": { "from": [0, 0], "to": [40, 30], "as": "r" } }],
            "constraints": [{ "coincident": ["r.bl", "origin"] }],
            "dimensions": [
                { "length": { "line": "r.bottom", "value": 40, "name": "w" } },
                { "length": { "line": "r.left", "value": 30, "name": "h" } },
            ] }),
    )
}

/// A RECTANGLE WITH ITS SIZES NAMED is fully held, makes one closed contour of 40 x 30, and its names are read by a
/// formula; the call is one step.
#[test]
fn a_rectangle_with_named_sizes_is_fully_held() {
    let mut ctx = Ctx::blank();
    let sketch = sketch_on(&mut ctx, json!("xy"));
    let drawn = held_rectangle(&mut ctx, sketch);
    assert_eq!(drawn["ok"], json!(true), "{drawn}");
    assert_eq!(drawn["dof"], 0, "the rectangle is not fully held: {drawn}");
    assert_eq!(drawn["redundant"], 0, "{drawn}");
    assert_eq!(drawn["conflicts"], json!([]), "{drawn}");
    let contours = drawn["contours"].as_array().expect("contours");
    assert_eq!(contours.len(), 1, "{drawn}");
    assert_eq!(contours[0]["closed"], json!(true));
    assert_eq!(contours[0]["profile"], json!(true), "an extrusion cannot take it: {drawn}");
    assert!((contours[0]["area"].as_f64().expect("an area") - 1200.0).abs() < 1e-6, "{drawn}");
    for n in ["r", "r.bottom", "r.top", "r.left", "r.right", "r.bl", "r.br", "r.tr", "r.tl"] {
        assert!(drawn["names"].get(n).is_some(), "{n} is not among the names: {}", drawn["names"]);
    }

    let names = call(&mut ctx, "list_parameters", json!({}));
    let text = names.to_string();
    assert!(text.contains("\"w\"") && text.contains("\"h\""), "the named sizes are not names formulas read: {names}");
    let read = call(&mut ctx, "set_parameter", json!({ "name": "area", "expr": "w * h" }));
    assert_eq!(read["parameter"]["value"], json!(1200.0), "{read}");

    let undone = call(&mut ctx, "undo", json!({}));
    assert_eq!(undone["undone"]["step"], "win-add-param");
    let undone = call(&mut ctx, "undo", json!({}));
    assert_eq!(undone["undone"]["step"], "sk-drawing", "the drawing is not one step: {undone}");
}

/// A SIZE GIVEN AS AN EXPRESSION follows its parameter; a size the drawing already holds becomes a reference one, a
/// relation it already holds is left out.
#[test]
fn a_held_size_becomes_a_reference_one() {
    let mut ctx = Ctx::blank();
    let _ = call(&mut ctx, "set_parameter", json!({ "name": "side", "expr": "25" }));
    let sketch = sketch_on(&mut ctx, json!("xy"));
    let drawn = call(
        &mut ctx,
        "sketch_add",
        json!({ "sketch": sketch,
            "entities": [{ "rect": { "from": [0, 0], "to": [20, 20], "as": "s" } }],
            "constraints": [{ "fixed": "s.bl" }, { "horizontal": "s.bottom" }],
            "dimensions": [
                { "length": { "line": "s.bottom", "value": "side" } },
                { "length": { "line": "s.left", "value": "side * 2" } },
                { "length": { "line": "s.top", "value": 25 } },
            ] }),
    );
    assert_eq!(drawn["ok"], json!(true), "{drawn}");
    assert_eq!(drawn["left_out"], json!([{ "horizontal": "s.bottom" }]), "the second horizontal was not left out in its own words: {drawn}");
    assert_eq!(drawn["reference"].as_array().map(Vec::len), Some(1), "the top, held by the bottom, did not become a reference: {drawn}");
    assert_eq!(drawn["dof"], 0, "{drawn}");
    let area = |v: &Value| v["contours"][0]["area"].as_f64().unwrap_or(f64::NAN);
    assert!((area(&drawn) - 25.0 * 50.0).abs() < 1e-6, "{drawn}");
    let _ = call(&mut ctx, "set_parameter", json!({ "name": "side", "expr": "10" }));
    let info = call(&mut ctx, "sketch_info", json!({ "sketch": sketch }));
    assert!((area(&info) - 10.0 * 20.0).abs() < 1e-6, "the sketch did not follow its parameter: {info}");
    assert_eq!(info["entities"].as_array().map(Vec::len), Some(4), "{info}");
}

/// A SKETCH ON A FACE stands on that face: on the top of a 10 mm box its frame stands at Z 10; a circle there makes a
/// closed contour. A sketch offset 15 from XY stands on a datum at Z 15.
#[test]
fn a_sketch_stands_on_a_face_and_on_an_offset_datum() {
    let mut ctx = Ctx::blank();
    let laid = call(&mut ctx, "box", json!({ "x": 40, "y": 40, "z": 10 }));
    let body = laid["body"].as_u64().expect("the box");
    let made = call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": body }, "face": "top" } }));
    assert_eq!(made["ok"], json!(true), "{made}");
    assert!((made["frame"]["origin"][2].as_f64().expect("a frame") - 10.0).abs() < 1e-9, "the sketch is not on the top: {made}");
    let sketch = made["sketch"].as_u64().expect("a sketch");
    let drawn = call(
        &mut ctx,
        "sketch_add",
        json!({ "sketch": sketch, "entities": [{ "circle": { "centre": [0, 0], "radius": 5, "as": "c" } }], "dimensions": [{ "diameter": { "circle": "c", "value": 10 } }] }),
    );
    assert_eq!(drawn["contours"][0]["closed"], json!(true), "{drawn}");
    assert!((drawn["contours"][0]["area"].as_f64().expect("an area") - std::f64::consts::PI * 25.0).abs() < 0.5, "{drawn}");

    let offset = call(&mut ctx, "create_sketch", json!({ "plane": "xy", "offset": 15 }));
    assert!(offset["datum"].as_u64().is_some(), "no datum was laid: {offset}");
    assert!((offset["frame"]["origin"][2].as_f64().expect("a frame") - 15.0).abs() < 1e-9, "{offset}");

    let two = call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": body }, "face": "largest" } }));
    assert_eq!(two["error"]["code"], "ref-ambiguous", "two largest faces were taken for one plane: {two}");
}

/// A CALL THAT NAMES NOTHING is refused whole: the rectangle it drew before the bad name is not left behind.
#[test]
fn a_call_that_names_nothing_is_refused_whole() {
    let mut ctx = Ctx::blank();
    let sketch = sketch_on(&mut ctx, json!("xz"));
    let steps = ctx.doc.history().undo_names().len();
    let bad = call(&mut ctx, "sketch_add", json!({ "sketch": sketch, "entities": [{ "rect": { "from": [0, 0], "to": [10, 10], "as": "r" } }], "constraints": [{ "vertical": "q" }] }));
    assert_eq!(bad["error"]["code"], "unknown-name", "{bad}");
    assert_eq!(ctx.doc.history().undo_names().len(), steps, "a refused call left a step");
    let info = call(&mut ctx, "sketch_info", json!({ "sketch": sketch }));
    assert_eq!(info["entities"], json!([]), "the rectangle of a refused call stayed: {info}");

    let kind = call(&mut ctx, "sketch_add", json!({ "sketch": sketch, "entities": [{ "line": { "from": [0, 0], "to": [10, 0], "as": "l" } }], "constraints": [{ "concentric": ["l", "l"] }] }));
    assert_eq!(kind["error"]["code"], "wrong-kind", "{kind}");
    let unread = call(&mut ctx, "sketch_add", json!({ "sketch": sketch, "dimensions": [{ "length": { "line": "x_axis", "value": 5 } }, { "lenght": {} }] }));
    assert_eq!(unread["error"]["code"], "arguments", "{unread}");
    assert!(unread["error"]["message"].as_str().is_some_and(|m| m.starts_with("dimensions[1]")), "the item is not named by its place: {unread}");
    let none = call(&mut ctx, "sketch_add", json!({ "sketch": 987_654 }));
    assert_eq!(none["error"]["code"], "no-sketch", "{none}");
    let taken = call(&mut ctx, "sketch_add", json!({ "sketch": sketch, "entities": [{ "line": { "from": [0, 0], "to": [10, 0], "as": "origin" } }] }));
    assert_eq!(taken["error"]["code"], "unfit-name", "{taken}");
}
