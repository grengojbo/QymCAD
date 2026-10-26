//! A BODY CUT TO FIT THROUGH THE SERVER: a bar of 500 on a printer of 256 is cut into two pieces that fit, one step undo
//! takes back; a body that fits comes back whole; a mesh brought in is refused until it is a body of faces.

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

#[test]
fn a_bar_is_cut_to_fit_and_undo_takes_it_back() {
    let mut ctx = Ctx::blank();
    let bar = ok(call(&mut ctx, "box", json!({ "x": 500, "y": 20, "z": 20 })));
    let r = ok(call(&mut ctx, "split_for_bed", json!({ "printer": "p1s" })));
    let pieces = r["pieces"].as_array().expect("the pieces");
    assert_eq!(pieces.len(), 2, "{r}");
    assert!(pieces.iter().all(|p| p["size"][0].as_f64().is_some_and(|x| x <= 251.0 + 1e-3)), "{r}");
    let _ = ok(call(&mut ctx, "undo", json!({})));
    let whole = ok(call(&mut ctx, "split_for_bed", json!({ "body": { "body": bar["body"] }, "bed": [600, 600, 600] })));
    assert_eq!(whole["pieces"].as_array().map(Vec::len), Some(1), "a bar that fits is cut: {whole}");
}

#[test]
fn a_mesh_brought_in_is_refused_until_it_has_faces() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-split"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("bar.stl").to_string_lossy().into_owned();
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 500, "y": 20, "z": 20 })));
    let _ = ok(call(&mut ctx, "export_mesh", json!({ "path": stl })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl })));
    let r = call(&mut ctx, "split_for_bed", json!({ "body": { "body": mesh["bodies"][0]["id"] } }));
    assert_eq!(r["error"]["code"], "split-needs-faces", "{r}");
}
