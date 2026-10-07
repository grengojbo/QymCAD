//! WHO MADE A FACE OR AN EDGE: a block rounded along its top answers, for every face of the rounding, the rounding
//! itself - its key, its kind, the role the face plays in it, how many faces of the body it made, and its sizes with
//! the expression the radius follows - and for the faces the rounding left alone, the block. An edge is told by the
//! later of the two faces it lies between. A solid that came from a STEP file carries no recipe, and its faces say so
//! rather than naming a feature that did not make them.

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

/// The keys of the features of the document, in the order of its timeline, with their kinds.
fn features(ctx: &mut Ctx) -> Vec<(i64, String)> {
    let doc = ok(call(ctx, "get_document", json!({})));
    doc["document"]["features"].as_array().expect("features").iter().map(|f| (f["key"].as_i64().expect("a key"), f["kind"].as_str().expect("a kind").to_string())).collect()
}

/// A block 40 x 30 x 10 from a sketch, its top edges rounded at `r` (a parameter); the key of the rounding and of the
/// extrusion.
fn rounded_block(ctx: &mut Ctx) -> (i64, i64) {
    let _ = ok(call(ctx, "set_parameter", json!({ "name": "r", "expr": "2" })));
    let sk = ok(call(ctx, "create_sketch", json!({ "plane": "xy" })));
    let _ = ok(call(ctx, "sketch_add", json!({ "sketch": sk["sketch"], "entities": [{ "rect": { "from": [0, 0], "to": [40, 30] } }] })));
    let _ = ok(call(ctx, "extrude", json!({ "sketch": sk["sketch"], "distance": 10 })));
    let _ = ok(call(ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": "r" })));
    let all = features(ctx);
    let key = |kind: &str| all.iter().find(|(_, k)| k == kind).map(|(k, _)| *k).unwrap_or_else(|| panic!("no {kind} in {all:?}"));
    (key("Fillet"), key("Extrude"))
}

#[test]
fn a_face_names_the_feature_that_made_it() {
    let mut ctx = Ctx::blank();
    let (fillet, extrude) = rounded_block(&mut ctx);
    let faces = ok(call(&mut ctx, "list_faces", json!({})))["faces"].as_array().cloned().expect("faces");

    let rounds: Vec<&Value> = faces.iter().filter(|f| f["made_by"]["feature"] == json!(fillet)).collect();
    assert!(rounds.len() >= 4, "the four rounds are not told as the rounding's: {faces:?}");
    for f in &rounds {
        let by = &f["made_by"];
        assert_eq!(by["kind"], "Fillet", "{f}");
        assert!(by["role"] == "Blend" || by["role"] == "Corner", "a face of the rounding plays no rounding role: {f}");
        assert_eq!(by["faces_made"], json!(rounds.len()), "the rounding does not say how many faces it made: {f}");
        let radius = by["sizes"].as_array().and_then(|s| s.iter().find(|s| s["key"] == "radius")).unwrap_or_else(|| panic!("no radius among the sizes: {f}"));
        assert_eq!(radius["expr"], "r", "the radius does not say what it follows: {f}");
    }
    let walls: Vec<&Value> = faces.iter().filter(|f| f["made_by"]["feature"] == json!(extrude)).collect();
    assert_eq!(walls.len(), 6, "the six faces of the block are not told as the extrusion's: {faces:?}");
    assert!(walls.iter().all(|f| f["made_by"]["kind"] == "Extrude"), "{walls:?}");
    assert_eq!(rounds.len() + walls.len(), faces.len(), "a face names nobody: {faces:?}");
}

#[test]
fn an_edge_names_the_later_of_its_two_faces() {
    let mut ctx = Ctx::blank();
    let (fillet, extrude) = rounded_block(&mut ctx);
    let edges = ok(call(&mut ctx, "list_edges", json!({})))["edges"].as_array().cloned().expect("edges");
    let by = |e: &Value| e["made_by"]["feature"].as_i64();
    // the four upright corners of the block lie between two walls of the extrusion, below the rounding
    let upright: Vec<&Value> = edges.iter().filter(|e| e["a"][0] == e["b"][0] && e["a"][1] == e["b"][1] && e["a"][2] != e["b"][2]).collect();
    assert!(!upright.is_empty(), "no upright edge found: {edges:?}");
    assert!(upright.iter().all(|e| by(e) == Some(extrude)), "an edge between two walls is not the extrusion's: {upright:?}");
    // the rims where the rounding meets the walls run level at 10 - r = 8
    let rims: Vec<&Value> = edges.iter().filter(|e| e["a"][2] == json!(8.0) && e["b"][2] == json!(8.0)).collect();
    assert_eq!(rims.len(), 4, "the four rims of the rounding are not found at 8: {edges:?}");
    assert!(rims.iter().all(|e| by(e) == Some(fillet)), "a rim of the rounding is not the rounding's: {rims:?}");
}

#[test]
fn a_solid_from_a_file_names_no_feature() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-made-by"));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let path = dir.join(format!("block-{}.stp", std::process::id()));
    let path = path.to_str().expect("a path in UTF-8");
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 10 })));
    let _ = ok(call(&mut ctx, "export_cad", json!({ "path": path, "overwrite": true })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let came = ok(call(&mut ctx, "import_cad", json!({ "path": path })));
    let _ = std::fs::remove_file(path);
    let faces = ok(call(&mut ctx, "list_faces", json!({ "body": { "body": came["bodies"][0]["id"] } })))["faces"].as_array().cloned().expect("faces");
    assert_eq!(faces.len(), 6, "{faces:?}");
    assert!(faces.iter().all(|f| f.get("made_by") == Some(&Value::Null)), "a face of a solid from a file names a feature: {faces:?}");
}
