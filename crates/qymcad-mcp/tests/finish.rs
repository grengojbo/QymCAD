//! FINISHING A BODY through the server: a block with a pocket has the rim of its top rounded by a query and measures
//! what the rebuild measures; the rounding follows a changed block; holes stand where they are put - off the face
//! centre, at the points of a sketch; what finds nothing or stands on a body taken into another is refused whole.

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

/// The one body standing.
fn the_body(reply: &Value) -> &Value {
    let bodies = reply["bodies"].as_array().unwrap_or_else(|| panic!("no bodies in {reply}"));
    assert_eq!(bodies.len(), 1, "the part stands as {} bodies: {reply}", bodies.len());
    &bodies[0]
}

fn volume(reply: &Value) -> f64 {
    the_body(reply)["volume"].as_f64().expect("a volume")
}

/// The 40 x 30 x 10 block from the origin, its extrusion `t` high; the reply of the extrusion.
fn block(ctx: &mut Ctx, height: Value) -> Value {
    let made = ok(call(ctx, "create_sketch", json!({ "plane": "xy" })));
    let _ = ok(call(ctx, "sketch_add", json!({ "sketch": made["sketch"], "entities": [{ "rect": { "from": [0, 0], "to": [40, 30] } }] })));
    ok(call(ctx, "extrude", json!({ "sketch": made["sketch"], "distance": height })))
}

/// THE CHAIN OF THE REBUILD'S FINGERPRINT through the server: the block, a 20 x 10 pocket 4 deep in its top, the rim of
/// the top rounded 1 by the query "every edge of the top face": 12000 - 800 - (1 - pi/4) x 200 = 11157.08 mm^3, 19
/// faces, 40 edges, one step each.
#[test]
fn a_pocketed_block_rounded_by_a_query_measures_the_rebuild() {
    let mut ctx = Ctx::blank();
    let laid = block(&mut ctx, json!(10));
    let top = ok(call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": laid["body"] }, "face": "top" } })));
    let _ = ok(call(&mut ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": [{ "rect": { "from": [10, 10], "to": [30, 20] } }] })));
    let _ = ok(call(&mut ctx, "extrude", json!({ "sketch": top["sketch"], "distance": 4, "op": "cut" })));
    let round = ok(call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 1 })));
    let b = the_body(&round);
    let want = 12000.0 - 800.0 - (1.0 - std::f64::consts::FRAC_PI_4) * 200.0;
    assert!((volume(&round) - want).abs() < 1e-6 * want, "the rounded block measures {}: {round}", volume(&round));
    assert_eq!(b["faces"], 19, "{round}");
    assert_eq!(b["edges"], 40, "{round}");
    assert_eq!(round["red"], json!([]), "{round}");
    let undone = ok(call(&mut ctx, "undo", json!({})));
    assert_eq!(undone["undone"]["step"], "f-fillet", "the rounding is not one step: {undone}");
}

/// A ROUNDING KEPT AS A QUERY follows its body: the block made 20 high by its parameter keeps every edge of its new top
/// rounded. The rim does not depend on the height, so the rounding takes exactly what it took at 10. (The take is not
/// pure r^2: where two rounded edges meet the corner adds an r^3 term, 30.04 r^2 - 0.38 r^3 here.)
#[test]
fn a_rounding_by_a_query_follows_its_body() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "set_parameter", json!({ "name": "t", "expr": "10" })));
    let _ = block(&mut ctx, json!("t"));
    let round = ok(call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 2 })));
    let taken = 12000.0 - volume(&round);
    assert!(taken > 100.0, "nothing was rounded: {round}");
    let taller = ok(call(&mut ctx, "set_parameter", json!({ "name": "t", "expr": "20" })));
    assert!((volume(&taller) - (24000.0 - taken)).abs() < 1e-3, "the rounding did not follow the block ({taken} taken at t = 10): {taller}");
    assert_eq!(the_body(&taller)["max"][2], json!(20.0), "{taller}");
    assert_eq!(taller["red"], json!([]), "{taller}");
}

/// HOLES STAND WHERE THEY ARE PUT: one 10 off the centre of the top along its X, through; then a counterbore at each
/// lone point of a sketch on the bottom.
#[test]
fn holes_stand_where_they_are_put() {
    let mut ctx = Ctx::blank();
    let _ = block(&mut ctx, json!(10));
    let one = ok(call(&mut ctx, "hole", json!({ "face": "top", "offset": [10, 0], "diameter": 6, "depth": 15 })));
    let through = 12000.0 - std::f64::consts::PI * 9.0 * 10.0;
    assert!((volume(&one) - through).abs() < 1e-3, "{one}");
    // a second hole at the centre stands apart from the first: had the offset been dropped, it would take nothing
    let centre = ok(call(&mut ctx, "hole", json!({ "face": "top", "diameter": 6, "depth": 15 })));
    assert!((volume(&centre) - (through - std::f64::consts::PI * 9.0 * 10.0)).abs() < 1e-3, "the two holes stand in one place: {centre}");
    let _ = ok(call(&mut ctx, "undo", json!({})));
    let body = the_body(&one)["id"].clone();
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    assert!(doc.to_string().contains("Hole"), "{doc}");

    let made = ok(call(&mut ctx, "create_sketch", json!({ "plane": "xy" })));
    let _ = ok(call(&mut ctx, "sketch_add", json!({ "sketch": made["sketch"], "entities": [{ "point": { "at": [5, 5] } }, { "point": { "at": [35, 25] } }] })));
    // a sketch on XY stands under the block: its holes go along the normal, up into the block
    let up = ok(call(
        &mut ctx,
        "hole",
        json!({ "body": { "body": body }, "sketch": made["sketch"], "diameter": 3, "depth": 15, "kind": "counterbore", "recess_diameter": 6, "recess_depth": 2, "direction": "along_normal" }),
    ));
    let pi = std::f64::consts::PI;
    let two = 2.0 * (pi * 2.25 * 8.0 + pi * 9.0 * 2.0);
    assert!((volume(&up) - (through - two)).abs() < 1e-3, "two counterbores 3/6 x 2 through 10 take {two}: {up}");
    let _ = ok(call(&mut ctx, "undo", json!({})));
    let void = ok(call(&mut ctx, "hole", json!({ "body": { "body": body }, "sketch": made["sketch"], "diameter": 3, "depth": 15 })));
    assert!((volume(&void) - through).abs() < 1e-3, "holes against the normal of a sketch under the block took something: {void}");

    let bare = call(&mut ctx, "hole", json!({ "face": "top", "diameter": 6, "depth": 5, "kind": "countersink" }));
    assert_eq!(bare["error"]["code"], "arguments", "a countersink with no recess went: {bare}");
    let both = call(&mut ctx, "hole", json!({ "face": "top", "sketch": made["sketch"], "diameter": 6, "depth": 5 }));
    assert_eq!(both["error"]["code"], "arguments", "{both}");
}

/// WHAT FINDS NOTHING, or stands on a body taken into another, is refused before anything is laid.
#[test]
fn what_finds_nothing_is_refused_whole() {
    let mut ctx = Ctx::blank();
    let laid = block(&mut ctx, json!(10));
    let first = laid["body"].clone();
    let _ = ok(call(&mut ctx, "chamfer", json!({ "edges": { "between": { "one": "top", "other": "front" } }, "distance": 1, "second": { "angle": 30 } })));
    let steps = ctx.doc.history().undo_names().len();
    let gone = call(&mut ctx, "fillet", json!({ "body": { "body": first }, "edges": { "adjacent": "top" }, "radius": 1 }));
    assert_eq!(gone["error"]["code"], "consumed-body", "a fillet stood on a body taken into the chamfer: {gone}");
    let none = call(&mut ctx, "fillet", json!({ "edges": { "facing": { "dir": [1, 1, 1], "tol_deg": 1 } }, "radius": 1 }));
    assert_eq!(none["error"]["code"], "ref-lost", "{none}");
    let faces = call(&mut ctx, "shell", json!({ "open": { "adjacent": "top" }, "thickness": 1 }));
    assert_eq!(faces["error"]["code"], "wrong-element", "edges were taken as faces to open: {faces}");
    let steep = call(&mut ctx, "chamfer", json!({ "edges": { "adjacent": "bottom" }, "distance": 1, "second": { "angle": 90 } }));
    assert_eq!(steep["error"]["code"], "bad-size", "{steep}");
    assert_eq!(ctx.doc.history().undo_names().len(), steps, "a refusal left a step");
}

/// A HOLE THAT CUTS NOTHING IS NAMED IN THE ANSWER. Reported behaviour: four holes drawn on a plate, their points laid
/// as if the plate began at the origin - it stands centred on it - drilled one; the three others cut nothing and the
/// answer said nothing. The answer now warns of each, with where it stands and where the body is.
#[test]
fn holes_that_cut_nothing_are_named() {
    let mut ctx = Ctx::blank();
    let laid = ok(call(&mut ctx, "box", json!({ "x": 60, "y": 40, "z": 4 })));
    let top = ok(call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "part": laid["bodies"][0]["part"] }, "face": "top" } })));
    let points: Vec<Value> = [[8, 8], [52, 8], [8, 32], [52, 32]].iter().map(|p| json!({ "point": { "at": p } })).collect();
    let _ = ok(call(&mut ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": points })));
    let drilled = ok(call(&mut ctx, "hole", json!({ "sketch": top["sketch"], "diameter": 3.4, "depth": 10 })));
    let warned = drilled["warnings"].as_array().cloned().unwrap_or_default();
    let air: Vec<&Value> = warned.iter().filter(|w| w["code"] == "hole-in-air").collect();
    assert_eq!(air.len(), 3, "three of the four holes stand past the plate: {drilled}");
    let at: Vec<Value> = air.iter().map(|w| w["at"].clone()).collect();
    for want in [json!([52.0, 8.0, 4.0]), json!([8.0, 32.0, 4.0]), json!([52.0, 32.0, 4.0])] {
        assert!(at.contains(&want), "no warning at {want}: {drilled}");
    }
    // the holes all on the plate: no warning
    let _ = ok(call(&mut ctx, "undo", json!({})));
    let inside = ok(call(&mut ctx, "hole", json!({ "face": "top", "diameter": 3.4, "depth": 10 })));
    assert!(inside.get("warnings").is_none(), "a hole on the plate is warned of: {inside}");
}
