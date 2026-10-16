//! A SECTION OF A MESH THROUGH THE SERVER: a cylinder written as an STL and brought back is read across as one circle
//! (mesh_section) and drawn into a sketch defined to the end (sketch_from_section); a body of exact surfaces and a plane
//! beside the body are refused with a code.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

/// A document holding a cylinder of 10 across and 20 tall brought in as a mesh, and the folder its file lies in.
struct Meshed {
    ctx: Ctx,
    body: Value,
    _dir: std::path::PathBuf,
}

fn meshed_cylinder() -> Meshed {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-section")).join(format!("{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("cylinder.stl").to_string_lossy().into_owned();
    let mut ctx = Ctx::blank();
    let _ = ctx.doc.edit("cmd-cylinder", |p| Ok(p.add_cylinder(5.0, 20.0))).expect("the cylinder is laid");
    // refused while exact: the cylinder has its exact edges to take
    let exact = call(&mut ctx, "sketch_from_section", json!({}));
    assert_eq!(exact["error"]["code"], "exact-body", "an exact body was drawn by its section: {exact}");
    let out = call(&mut ctx, "export_mesh", json!({ "path": stl, "quality": { "deflection": 0.01 } }));
    assert_eq!(out["ok"], json!(true), "{out}");
    let _ = call(&mut ctx, "new_project", json!({}));
    let back = call(&mut ctx, "import_mesh", json!({ "path": stl }));
    assert_eq!(back["ok"], json!(true), "{back}");
    let body = json!({ "body": back["bodies"][0]["id"].clone() });
    Meshed { ctx, body, _dir: dir }
}

#[test]
fn a_cylinder_of_a_mesh_is_read_across_as_one_circle_and_drawn() {
    let Meshed { mut ctx, body, _dir } = meshed_cylinder();
    let read = call(&mut ctx, "mesh_section", json!({ "body": body, "plane": "xy" }));
    let pieces = &read["loops"][0]["pieces"];
    assert_eq!(read["loops"].as_array().map(Vec::len), Some(1), "one loop across the cylinder: {read}");
    assert_eq!(pieces[0]["kind"], "circle", "the cylinder is read across as {pieces}");
    let r = pieces[0]["radius"].as_f64().expect("a radius");
    assert!((r - 5.0).abs() < 0.01, "the circle is {r} across the half, not 5");
    assert!((read["offset"].as_f64().expect("an offset") - 10.0).abs() < 1e-6, "the plane is not at the middle of the body: {read}");
    let missed = call(&mut ctx, "mesh_section", json!({ "body": body, "offset": 25 }));
    assert_eq!(missed["error"]["code"], "off-the-body", "a plane above the body was read: {missed}");
    let drawn = call(&mut ctx, "sketch_from_section", json!({ "body": body, "offset": 7.5 }));
    assert_eq!(drawn["ok"], json!(true), "{drawn}");
    assert_eq!(drawn["held"]["free"], json!(0), "the sketch is left free: {drawn}");
}
