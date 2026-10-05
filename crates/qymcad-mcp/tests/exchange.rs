//! FILES IN AND OUT THROUGH THE SERVER: an STL in inches comes in at 25.4 times its numbers, a STEP goes out and comes
//! back the same solid, a 3MF of two parts keeps them apart, and a mesh is refused by an exact file with a code.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-exchange")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

fn file(dir: &std::path::Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

/// A document holding one 10 mm cube.
fn cube() -> Ctx {
    let mut ctx = Ctx::blank();
    let _ = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(10.0, 10.0, 10.0))).expect("the cube is laid");
    ctx
}

/// The bodies standing on their own in an answer, by volume, to the cubic millimetre.
fn volumes(reply: &Value) -> Vec<i64> {
    let mut v: Vec<i64> = reply["bodies"].as_array().unwrap_or_else(|| panic!("no bodies in {reply}")).iter().map(|b| b["volume"].as_f64().expect("a volume").round() as i64).collect();
    v.sort_unstable();
    v
}

/// AN STL CARRIES NO UNIT: taken as it is, the cube is 10 mm; taken in inches, 254 mm and 254^3 mm^3; taken a
/// hundredth, 0.1 mm - which the answer flags.
#[test]
fn an_stl_comes_in_at_its_unit() {
    let dir = out_dir("stl");
    let stl = file(&dir, "cube.stl");
    let mut ctx = cube();
    let out = call(&mut ctx, "export_mesh", json!({ "path": stl, "quality": "draft" }));
    assert_eq!(out["ok"], json!(true), "{out}");
    assert_eq!(out["written"]["bodies"], 1, "{out}");

    let _ = call(&mut ctx, "new_project", json!({}));
    let plain = call(&mut ctx, "import_mesh", json!({ "path": stl }));
    assert_eq!(plain["ok"], json!(true), "{plain}");
    assert_eq!(plain["imported"]["unitless"], json!(true));
    assert_eq!(plain["imported"]["size_mm"], json!([10.0, 10.0, 10.0]), "{plain}");
    assert!(plain.get("warning").is_none(), "a 10 mm cube was flagged: {plain}");
    assert_eq!(volumes(&plain), [1000]);

    let _ = call(&mut ctx, "new_project", json!({}));
    let inches = call(&mut ctx, "import_mesh", json!({ "path": stl, "unit": "inch" }));
    let size = inches["imported"]["size_mm"][0].as_f64().expect("a size");
    assert!((size - 254.0).abs() < 1e-9, "10 inches came in at {size} mm: {inches}");
    assert_eq!(volumes(&inches), [(254.0f64 * 254.0 * 254.0).round() as i64], "{inches}");

    let _ = call(&mut ctx, "new_project", json!({}));
    let tiny = call(&mut ctx, "import_mesh", json!({ "path": stl, "scale": 0.01 }));
    assert_eq!(tiny["warning"]["code"], "scale-suspicious", "a 0.1 mm part was not flagged: {tiny}");
    let undone = call(&mut ctx, "undo", json!({}));
    assert_eq!(undone["undone"]["step"], "name-import", "the import is not one step: {undone}");

    let wrong = call(&mut ctx, "import_cad", json!({ "path": stl }));
    assert_eq!(wrong["error"]["code"], "wrong-extension", "an STL went in as an exact file: {wrong}");
}

/// A STEP WRITTEN AND READ BACK, and written again from what came in, is the same solid.
#[test]
fn a_step_goes_out_and_comes_back_the_same() {
    let dir = out_dir("step");
    let mut ctx = Ctx::blank();
    let _ = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(10.0, 20.0, 30.0))).expect("the box is laid");
    let first = file(&dir, "box.step");
    let out = call(&mut ctx, "export_cad", json!({ "path": first }));
    assert_eq!(out["written"]["exact"], 1, "{out}");

    let _ = call(&mut ctx, "new_project", json!({}));
    let back = call(&mut ctx, "import_cad", json!({ "path": first }));
    assert_eq!(back["ok"], json!(true), "{back}");
    assert_eq!(volumes(&back), [6000], "{back}");
    assert_eq!(back["bodies"][0]["faces"], 6, "{back}");

    let second = file(&dir, "again.stp");
    let again = call(&mut ctx, "export_cad", json!({ "path": second }));
    assert_eq!(again["ok"], json!(true), "the imported solid did not go out again: {again}");
    let _ = call(&mut ctx, "new_project", json!({}));
    let twice = call(&mut ctx, "import_cad", json!({ "path": second }));
    assert_eq!(volumes(&twice), [6000], "{twice}");

    let kept = call(&mut ctx, "export_cad", json!({ "path": second }));
    assert_eq!(kept["error"]["code"], "path-taken", "a file was written over unasked: {kept}");
}

/// A 3MF OF TWO PARTS comes back as two bodies in two parts.
#[test]
fn a_3mf_of_two_parts_keeps_them_apart() {
    let dir = out_dir("3mf");
    let path = file(&dir, "two.3mf");
    let mut ctx = Ctx::blank();
    let _ = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(10.0, 10.0, 10.0))).expect("the first box is laid");
    let _ = ctx
        .doc
        .edit("cmd-box", |p| {
            p.active_component = Some(p.root);
            let part = p.add_part("Second");
            p.active_component = Some(part);
            Ok(p.add_box(20.0, 10.0, 10.0))
        })
        .expect("the second box is laid");
    let out = call(&mut ctx, "export_mesh", json!({ "path": path, "quality": { "deflection": 0.1 } }));
    assert_eq!(out["written"]["bodies"], 2, "{out}");

    let _ = call(&mut ctx, "new_project", json!({ "template": "empty" }));
    let back = call(&mut ctx, "import_mesh", json!({ "path": path }));
    assert_eq!(back["ok"], json!(true), "{back}");
    assert_eq!(back["imported"]["unitless"], json!(false), "a 3MF carries its unit");
    assert_eq!(volumes(&back), [1000, 2000], "{back}");
    let doc = call(&mut ctx, "get_document", json!({}));
    let parts: Vec<&Value> = doc["document"]["parts"].as_array().expect("parts").iter().filter(|p| p["assembly"] == json!(false)).collect();
    assert!(parts.len() >= 2, "the two parts came back as one: {doc}");
}

/// A MESH HAS NO EXACT FACES: an exact file refuses it with a code, and a mesh file takes it.
#[test]
fn a_mesh_is_refused_by_an_exact_file() {
    let dir = out_dir("mesh-to-step");
    let stl = file(&dir, "cube.stl");
    let mut ctx = cube();
    let _ = call(&mut ctx, "export_mesh", json!({ "path": stl }));
    let _ = call(&mut ctx, "new_project", json!({}));
    let _ = call(&mut ctx, "import_mesh", json!({ "path": stl }));
    let refused = call(&mut ctx, "export_cad", json!({ "path": file(&dir, "cube.step") }));
    assert_eq!(refused["ok"], json!(false), "{refused}");
    assert_eq!(refused["error"]["code"], "io-exact-no-brep", "{refused}");
    assert_eq!(refused["error"]["stage"], "io");
    let mesh = call(&mut ctx, "export_mesh", json!({ "path": file(&dir, "cube.3mf") }));
    assert_eq!(mesh["written"]["mesh_only"], 1, "{mesh}");
    let bad = call(&mut ctx, "export_mesh", json!({ "path": file(&dir, "cube.3mf"), "overwrite": true, "quality": { "deflection": 0 } }));
    assert_eq!(bad["error"]["code"], "bad-deflection", "{bad}");
}
