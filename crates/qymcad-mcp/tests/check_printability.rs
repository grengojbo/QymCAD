//! WHETHER A BODY PRINTS, THROUGH THE SERVER: a cube of 10 passes on the room assumed and says it was assumed; a T on
//! its stem hangs the two arms of its bar, 100 mm^2 each, over the air; a box of 200 does not fit the room of the
//! smallest printer named; a printer of no name, and a printer with a room besides, are refused; a cup of walls of 0.6 is
//! too thin for a nozzle of 0.4.

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

#[test]
fn a_cube_passes_on_the_room_assumed() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let r = ok(call(&mut ctx, "check_printability", json!({})));
    assert_eq!(r["verdict"], "pass", "{r}");
    assert_eq!(r["warnings"], json!(["bed-assumed"]), "{r}");
    assert_eq!(r["build"], json!([256.0, 256.0, 256.0]), "{r}");
    assert!(near(&r["checks"]["contact"]["area"], 100.0), "{r}");
    assert_eq!(r["checks"]["valid"]["valid"], json!(true), "{r}");
}

#[test]
fn a_t_on_its_stem_hangs_both_arms() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 20 })));
    let _ = ok(call(&mut ctx, "box", json!({ "x": 30, "y": 10, "z": 5, "at": [0, 0, 20] })));
    let r = ok(call(&mut ctx, "check_printability", json!({ "printer": "p1s" })));
    assert_eq!(r["verdict"], "warn", "{r}");
    let hang = &r["checks"]["overhang"];
    assert!(near(&hang["area"], 200.0), "the arms of the bar hang 200 mm^2: {r}");
    let places = hang["places"].as_array().expect("the places");
    assert_eq!(places.len(), 2, "two arms, two places: {r}");
    for p in places {
        assert!(near(&p["area"], 100.0) && near(&p["at"][2], 20.0) && (near(&p["at"][0], 10.0) || near(&p["at"][0], -10.0)), "an arm is not 100 mm^2 at x = 10 or -10 under the bar: {p}");
    }
    assert!(r.get("warnings").is_none(), "a printer named is said to be assumed: {r}");
}

#[test]
fn a_box_past_the_room_of_the_printer_named_fails() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 200, "y": 20, "z": 20 })));
    let r = ok(call(&mut ctx, "check_printability", json!({ "printer": "A1_MINI" })));
    assert_eq!(r["verdict"], "fail", "{r}");
    assert_eq!(r["checks"]["bed"]["fit"], "no", "{r}");
    let r = ok(call(&mut ctx, "check_printability", json!({ "bed": [210, 210, 210] })));
    assert_eq!(r["checks"]["bed"]["verdict"], "pass", "{r}");
}

#[test]
fn a_printer_of_no_name_or_with_a_room_besides_is_refused() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let r = call(&mut ctx, "check_printability", json!({ "printer": "a-printer-of-no-name" }));
    assert_eq!(r["error"]["code"], "no-printer", "{r}");
    assert!(r["error"]["hint"].as_str().is_some_and(|h| h.contains("p1s")), "the hint does not name the printers known: {r}");
    let r = call(&mut ctx, "check_printability", json!({ "printer": "p1s", "bed": [100, 100, 100] }));
    assert_eq!(r["error"]["code"], "arguments", "{r}");
}

/// A CUP OF WALLS OF 0.6 IS TOO THIN for two lines of a nozzle of 0.4, and holds for one of 0.2: a box of 20 hollowed
/// to 0.6, open at the top.
#[test]
fn a_cup_of_thin_walls_is_too_thin_for_the_nozzle() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 20 })));
    let _ = ok(call(&mut ctx, "shell", json!({ "thickness": 0.6, "open": "top" })));
    let r = ok(call(&mut ctx, "check_printability", json!({ "printer": "p1s" })));
    let walls = &r["checks"]["walls"];
    assert_eq!(walls["verdict"], "fail", "{r}");
    assert!(walls["least"].as_f64().is_some_and(|w| (w - 0.6).abs() < 0.05), "walls of 0.6 read {}", walls["least"]);
    assert_eq!(walls["required"], json!(0.8), "{r}");
    assert!(!walls["places"].as_array().is_some_and(Vec::is_empty), "no place is told thin: {r}");
    assert_eq!(r["verdict"], "fail", "{r}");
    let fine = ok(call(&mut ctx, "check_printability", json!({ "printer": "p1s", "nozzle": 0.2 })));
    assert_eq!(fine["checks"]["walls"]["verdict"], "pass", "walls of 0.6 do not hold for a nozzle of 0.2: {fine}");
}

/// A HOLE TOO NARROW CLOSES UP: a hole of 0.2 across in a plate is told as a hole of radius 0.1 for a nozzle of 0.4, with
/// the key of its face; a mesh brought in has no exact face to read and says so.
#[test]
fn a_hole_narrower_than_the_nozzle_is_told() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 5 })));
    let _ = ok(call(&mut ctx, "hole", json!({ "face": "top", "diameter": 0.2, "depth": 5 })));
    let r = ok(call(&mut ctx, "check_printability", json!({ "printer": "p1s" })));
    let small = &r["checks"]["small"];
    assert_eq!(small["verdict"], "warn", "{r}");
    let rounds = small["rounds"].as_array().expect("the round faces");
    assert!(rounds.iter().any(|f| f["kind"] == "hole" && near(&f["radius"], 0.1) && f["face"].is_u64()), "the hole of 0.2 is not told: {r}");
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-small"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("plate.stl").to_string_lossy().into_owned();
    let _ = ok(call(&mut ctx, "export_mesh", json!({ "path": stl })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl })));
    let m = ok(call(&mut ctx, "check_printability", json!({ "body": { "body": mesh["bodies"][0]["id"] }, "printer": "p1s" })));
    assert_eq!(m["checks"]["small"], Value::Null, "{m}");
    assert!(m["warnings"].as_array().is_some_and(|w| w.contains(&json!("small-unread-on-mesh"))), "{m}");
}
