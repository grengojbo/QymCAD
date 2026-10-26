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

/// JOINED ACROSS THE CUT: the bar of 500 cut on p1s with joints has two round pegs on its one cut, the pieces each one
/// sound solid; a bar of 4 x 4 is cut and left without, warned.
#[test]
fn a_bar_cut_with_joints_holds_two_pegs_on_its_cut() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 500, "y": 20, "z": 20 })));
    let r = ok(call(&mut ctx, "split_for_bed", json!({ "printer": "p1s", "joints": {} })));
    let joints = r["joints"].as_array().expect("the joints");
    assert_eq!(joints.len(), 1, "{r}");
    assert_eq!(joints[0]["spots"].as_array().map(Vec::len), Some(2), "{r}");
    assert_eq!(r["pieces"].as_array().map(Vec::len), Some(2), "{r}");
    for p in r["pieces"].as_array().expect("the pieces") {
        let health = ok(call(&mut ctx, "inspect", json!({ "body": { "body": p["body"] } })));
        assert_eq!(health["valid"], json!(true), "{health}");
        assert_eq!(health["solids"], json!(1), "{health}");
    }
    let mut thin = Ctx::blank();
    let _ = ok(call(&mut thin, "box", json!({ "x": 500, "y": 4, "z": 4 })));
    let r = ok(call(&mut thin, "split_for_bed", json!({ "printer": "p1s", "joints": {} })));
    assert!(r["warnings"].as_array().is_some_and(|w| w.contains(&json!("joints-left-without"))), "{r}");
}

/// A JOINT'S SIZES ARE ITS NUMBERS: edit_feature gives the pegs a diameter of 5, and the piece the pegs grow from holds
/// the two pegs of 5 rather than of 6 - less by 2 x pi x (9 - 6.25) x 8 mm^3.
#[test]
fn edit_feature_changes_the_size_of_the_pegs() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 500, "y": 20, "z": 20 })));
    let r = ok(call(&mut ctx, "split_for_bed", json!({ "printer": "p1s", "joints": {} })));
    let node = r["joints"][0]["feature"].clone();
    let volume = |ctx: &mut Ctx| -> f64 {
        let doc = ok(call(ctx, "get_document", json!({})));
        doc["document"]["bodies"].as_array().expect("the bodies").iter().filter(|b| b["visible"] != json!(false)).filter_map(|b| b["volume"].as_f64()).fold(f64::MIN, f64::max)
    };
    let before = volume(&mut ctx);
    let _ = ok(call(&mut ctx, "edit_feature", json!({ "feature": node, "values": { "diameter": 5 } })));
    let after = volume(&mut ctx);
    let less = 2.0 * std::f64::consts::PI * (9.0 - 6.25) * 8.0;
    assert!((before - after - less).abs() < 1e-2, "the piece with the pegs lost {} against {less}", before - after);
}
