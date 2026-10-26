//! A BODY LAID ON A FACE THROUGH THE SERVER: a box of 40 x 30 x 10 laid on its side of +X stands 40 tall on z = 0; laid
//! again on another face it takes the move on top, one node still; a box brought in as STL, whose faces have no keys, is
//! laid by the way its face looks; a face and a way at once, or a way of no length, are refused.

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

fn near(v: &Value, want: f64) -> bool {
    v.as_f64().is_some_and(|x| (x - want).abs() < 1e-3)
}

/// How many moves the timeline holds.
fn moves(ctx: &mut Ctx) -> usize {
    let doc = ok(call(ctx, "get_document", json!({})));
    doc["document"]["features"].as_array().map_or(0, |f| f.iter().filter(|n| n["kind"].as_str().is_some_and(|k| k.starts_with("Move"))).count())
}

#[test]
fn a_box_laid_on_its_side_stands_on_it_and_a_second_lay_is_the_same_node() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    let r = ok(call(&mut ctx, "place_on_face", json!({ "face": { "extreme": { "axis": "x", "max": true } } })));
    assert!(near(&r["size"][2], 40.0) && near(&r["size"][0], 10.0) && near(&r["lowest"], 0.0), "{r}");
    // laid next on the face of +Y it now has: 30 tall, still one move
    let again = ok(call(&mut ctx, "place_on_face", json!({ "body": { "body": r["body"] }, "face": { "extreme": { "axis": "y", "max": true } } })));
    assert!(near(&again["size"][2], 30.0) && near(&again["lowest"], 0.0), "{again}");
    assert_eq!(again["feature"], r["feature"], "a second lay is a node of its own: {again}");
    assert_eq!(moves(&mut ctx), 1, "the two lays are not one move");
}

#[test]
fn a_box_brought_in_as_a_mesh_is_laid_by_the_way_its_face_looks() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-place"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("box.stl").to_string_lossy().into_owned();
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    let _ = ok(call(&mut ctx, "export_mesh", json!({ "path": stl })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl })));
    let r = ok(call(&mut ctx, "place_on_face", json!({ "body": { "body": mesh["bodies"][0]["id"] }, "down": [1, 0, 0] })));
    assert!(near(&r["size"][2], 40.0) && near(&r["lowest"], 0.0), "the mesh is not laid on its side: {r}");
}

#[test]
fn a_face_and_a_way_at_once_or_a_way_of_no_length_are_refused() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let r = call(&mut ctx, "place_on_face", json!({ "face": "top", "down": [0, 0, 1] }));
    assert_eq!(r["error"]["code"], "arguments", "{r}");
    let r = call(&mut ctx, "place_on_face", json!({}));
    assert_eq!(r["error"]["code"], "arguments", "{r}");
    let r = call(&mut ctx, "place_on_face", json!({ "down": [0, 0, 0] }));
    assert_eq!(r["error"]["code"], "bad-direction", "{r}");
}
