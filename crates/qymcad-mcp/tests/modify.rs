//! CHANGING A WHOLE BODY through the server, beyond what the window's contracts hold: the pieces of a split joined
//! and cut by a boolean, two moves kept as one node, a grid of copies standing apart as the pieces of one body, a
//! mirror in a world plane, a face pushed by an expression, and what cuts nothing refused.

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

/// The bodies standing, by volume to the cubic millimetre.
fn volumes(reply: &Value) -> Vec<i64> {
    let mut v: Vec<i64> = reply["bodies"].as_array().unwrap_or_else(|| panic!("no bodies in {reply}")).iter().map(|b| b["volume"].as_f64().expect("a volume").round() as i64).collect();
    v.sort_unstable();
    v
}

/// The 40 x 30 x 10 block from the origin; the key of its body.
fn block(ctx: &mut Ctx) -> Value {
    let made = ok(call(ctx, "create_sketch", json!({ "plane": "xy" })));
    let _ = ok(call(ctx, "sketch_add", json!({ "sketch": made["sketch"], "entities": [{ "rect": { "from": [0, 0], "to": [40, 30] } }] })));
    ok(call(ctx, "extrude", json!({ "sketch": made["sketch"], "distance": 10 })))["body"].clone()
}

/// THE PIECES OF A SPLIT are bodies of one part: XY moved 4 up cuts the block into 4800 and 7200; joined they are the
/// block again, one body of six faces.
#[test]
fn the_pieces_of_a_split_join_again() {
    let mut ctx = Ctx::blank();
    let _ = block(&mut ctx);
    let split = ok(call(&mut ctx, "split_body", json!({ "plane": "xy", "offset": 4 })));
    assert_eq!(volumes(&split), [4800, 7200], "{split}");
    let pieces = split["pieces"].as_array().expect("the pieces").clone();
    assert_eq!(pieces.len(), 2, "{split}");
    let joined = ok(call(&mut ctx, "boolean", json!({ "target": { "body": pieces[0] }, "tool": { "body": pieces[1] }, "op": "join" })));
    assert_eq!(volumes(&joined), [12000], "{joined}");
    assert_eq!(joined["bodies"][0]["faces"], 6, "the seam stayed: {joined}");
    let undone = ok(call(&mut ctx, "undo", json!({})));
    assert_eq!(undone["undone"]["step"], "f-operation");
    let same = call(&mut ctx, "boolean", json!({ "target": { "body": pieces[0] }, "tool": { "body": pieces[0] }, "op": "cut" }));
    assert_eq!(same["error"]["code"], "arguments", "{same}");
    let nothing = call(&mut ctx, "split_body", json!({ "body": { "body": pieces[0] }, "plane": "xy", "offset": 50 }));
    assert_eq!(nothing["error"]["code"], "cuts-nothing", "a plane above the body was taken: {nothing}");
}

/// TWO MOVES ARE ONE NODE: the block carried 100 along X, then turned a quarter about Z, stands where the two together
/// put it, and the timeline holds one move.
#[test]
fn two_moves_are_one_node() {
    let mut ctx = Ctx::blank();
    let _ = block(&mut ctx);
    let first = ok(call(&mut ctx, "move", json!({ "at": [100, 0, 0] })));
    assert_eq!(first["bodies"][0]["min"], json!([100.0, 0.0, 0.0]), "{first}");
    let second = ok(call(&mut ctx, "move", json!({ "rotate": [{ "axis": [0, 0, 1], "degrees": 90 }] })));
    let b = &second["bodies"][0];
    let at = |k: &str, i: usize| b[k][i].as_f64().expect("a box");
    // (100..140, 0..30) turned a quarter about Z goes to (-30..0, 100..140)
    assert!((at("min", 0) + 30.0).abs() < 0.05 && (at("max", 1) - 140.0).abs() < 0.05, "{second}");
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    let moves = doc["document"]["features"].as_array().expect("features").iter().filter(|f| f["kind"] == "Move").count();
    assert_eq!(moves, 1, "two moves made {moves} nodes: {doc}");
    let still = call(&mut ctx, "move", json!({}));
    assert_eq!(still["error"]["code"], "arguments", "{still}");
}

/// A GRID OF COPIES standing apart is one body of separate pieces: 3 along X 50 apart by 2 along Y 40 apart, six
/// blocks; a mirror of the block in XZ joins it with its reflection into one block 60 deep.
#[test]
fn a_grid_stands_as_pieces_and_a_mirror_joins() {
    let mut ctx = Ctx::blank();
    let _ = block(&mut ctx);
    let grid = ok(call(&mut ctx, "linear_pattern", json!({ "along": "x", "step": 50, "count": 3, "second": { "along": "y", "step": 40, "count": 2 } })));
    assert_eq!(grid["bodies"][0]["pieces"], 6, "{grid}");
    assert_eq!(volumes(&grid), [72000], "{grid}");

    let mut ctx = Ctx::blank();
    let _ = block(&mut ctx);
    let both = ok(call(&mut ctx, "mirror", json!({ "plane": "xz" })));
    assert_eq!(volumes(&both), [24000], "{both}");
    assert_eq!(both["bodies"][0]["min"][1], json!(-30.0), "{both}");
}

/// A FACE PUSHED BY AN EXPRESSION follows its parameter; a push of nothing is refused.
#[test]
fn a_pushed_face_follows_its_parameter() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "set_parameter", json!({ "name": "lift", "expr": "5" })));
    let _ = block(&mut ctx);
    let pushed = ok(call(&mut ctx, "push_face", json!({ "face": "top", "distance": "lift" })));
    assert_eq!(volumes(&pushed), [18000], "{pushed}");
    let more = ok(call(&mut ctx, "set_parameter", json!({ "name": "lift", "expr": "-4" })));
    assert_eq!(volumes(&more), [7200], "the push did not follow lift: {more}");
    let zero = call(&mut ctx, "push_face", json!({ "face": "bottom", "distance": 0 }));
    assert_eq!(zero["error"]["code"], "bad-size", "{zero}");
}
