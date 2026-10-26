//! A FILE TAKEN AT ANOTHER SIZE THROUGH THE SERVER: a box written as STL and brought back is taken at 2.54 times its
//! size, undo takes it back; a body built in the document and a factor of nothing are refused.

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
fn a_mesh_brought_in_is_taken_at_the_factor_asked() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-scale"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("box.stl").to_string_lossy().into_owned();
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    let _ = ok(call(&mut ctx, "export_mesh", json!({ "path": stl })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl })));
    let body = json!({ "body": mesh["bodies"][0]["id"] });
    let r = ok(call(&mut ctx, "scale", json!({ "body": body, "factor": 2.54 })));
    assert_eq!(r["factor"], json!(2.54), "{r}");
    assert!((r["size"][0].as_f64().unwrap_or(0.0) - 101.6).abs() < 1e-3, "{r}");
    let _ = ok(call(&mut ctx, "undo", json!({})));
    let back = ok(call(&mut ctx, "scale", json!({ "body": body, "factor": 1 })));
    assert!((back["size"][0].as_f64().unwrap_or(0.0) - 40.0).abs() < 1e-3, "undo did not take the factor back: {back}");
}

#[test]
fn a_body_built_here_and_a_factor_of_nothing_are_refused() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let r = call(&mut ctx, "scale", json!({ "factor": 2 }));
    assert_eq!(r["error"]["code"], "scale-built-body", "{r}");
    let r = call(&mut ctx, "scale", json!({ "factor": 0 }));
    assert_eq!(r["error"]["code"], "bad-factor", "{r}");
}
