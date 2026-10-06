//! HOW LONG A CALL TAKES, in a release build: a simple change of a part of close to 200 faces under 300 ms, the
//! bracket rebuilt from a changed parameter under 300 ms, a picture 768 px wide under 500 ms. Each is the median of
//! five calls, so one slow call of a busy machine does not decide.
//!
//! A measurement of the machine as much as of the program, so it is run by hand:
//! `cargo test --release -p qymcad-mcp --test bench -- --ignored --test-threads=1`. A debug build is ten times slower and says nothing
//! about these budgets; there the checks are passed over. One at a time: the kernel takes one call at a time, and three
//! measured at once wait for each other - the bracket took 483 ms beside the others, 30 ms alone.

use std::time::{Duration, Instant};

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    let reply = answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone();
    assert_eq!(reply["ok"], json!(true), "{name}: {reply}");
    reply
}

/// The median of five runs of `work`, each on a document `setup` made afresh.
fn median(setup: fn() -> Ctx, work: fn(&mut Ctx)) -> Duration {
    let mut times: Vec<Duration> = (0..5)
        .map(|_| {
            let mut ctx = setup();
            let start = Instant::now();
            work(&mut ctx);
            start.elapsed()
        })
        .collect();
    times.sort();
    times[2]
}

/// A plate 120 x 80 x 6 with 9 x 7 counterbored holes through it: three faces a hole - its wall, the recess's wall and
/// its floor - and the plate's six, 195 faces.
fn perforated() -> Ctx {
    let mut ctx = Ctx::blank();
    let laid = call(&mut ctx, "box", json!({ "x": 120, "y": 80, "z": 6 }));
    let top = call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": laid["body"] }, "face": "top" } }));
    let points: Vec<Value> = (0..9).flat_map(|i| (0..7).map(move |j| json!({ "point": { "at": [-48 + 12 * i, -33 + 11 * j] } }))).collect();
    let _ = call(&mut ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": points }));
    let _ = call(&mut ctx, "hole", json!({ "sketch": top["sketch"], "diameter": 6, "depth": 10, "kind": "counterbore", "recess_diameter": 10, "recess_depth": 2 }));
    ctx
}

/// The bracket 60 x 40 at the thickness `t`, with four countersunk holes and its corners rounded.
fn bracket() -> Ctx {
    let mut ctx = Ctx::blank();
    let _ = call(&mut ctx, "set_parameter", json!({ "name": "t", "expr": "4" }));
    let sk = call(&mut ctx, "create_sketch", json!({ "plane": "xy" }));
    let _ = call(&mut ctx, "sketch_add", json!({ "sketch": sk["sketch"], "entities": [{ "rect": { "from": [-30, -20], "to": [30, 20] } }] }));
    let plate = call(&mut ctx, "extrude", json!({ "sketch": sk["sketch"], "distance": "t" }));
    let top = call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": plate["body"] }, "face": "top" } }));
    let points: Vec<Value> = [[-22, -12], [22, -12], [-22, 12], [22, 12]].iter().map(|p| json!({ "point": { "at": p } })).collect();
    let _ = call(&mut ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": points }));
    let _ = call(&mut ctx, "hole", json!({ "sketch": top["sketch"], "diameter": 3.4, "depth": 20, "kind": "countersink", "recess_diameter": 6.5, "recess_depth": 1.6 }));
    let side = |axis: &str, max: bool| json!({ "extreme": { "axis": axis, "max": max } });
    let corner = |x: bool, y: bool| json!({ "between": { "one": side("x", x), "other": side("y", y) } });
    let vertical = json!({ "union": [corner(false, false), corner(false, true), corner(true, false), corner(true, true)] });
    let _ = call(&mut ctx, "fillet", json!({ "edges": vertical, "radius": 5 }));
    ctx
}

/// In a debug build the budgets mean nothing: the check is passed over, and says so.
fn release_only() -> bool {
    if cfg!(debug_assertions) {
        eprintln!("PASSED OVER: the budgets of time hold for a release build only (cargo test --release)");
        return false;
    }
    true
}

#[test]
#[ignore = "a measurement of time: run by hand in release - cargo test --release -p qymcad-mcp --test bench -- --ignored --test-threads=1"]
fn a_simple_change_of_a_part_of_close_to_200_faces_is_under_300_ms() {
    if !release_only() {
        return;
    }
    let faces = call(&mut perforated(), "get_document", json!({}))["document"]["bodies"][0]["faces"].as_u64().unwrap_or(0);
    let took = median(perforated, |ctx| {
        let _ = call(ctx, "push_face", json!({ "face": "top", "distance": 1 }));
    });
    eprintln!("a face pushed on a part of {faces} faces: {took:?}");
    assert!(faces >= 190, "the part has {faces} faces - too few to measure the budget by");
    assert!(took < Duration::from_millis(300), "a face pushed on a part of {faces} faces took {took:?}");
}

#[test]
#[ignore = "a measurement of time: run by hand in release - cargo test --release -p qymcad-mcp --test bench -- --ignored --test-threads=1"]
fn the_bracket_rebuilds_from_its_parameter_under_300_ms() {
    if !release_only() {
        return;
    }
    let took = median(bracket, |ctx| {
        let _ = call(ctx, "set_parameter", json!({ "name": "t", "expr": "5" }));
    });
    eprintln!("the bracket rebuilt at t = 5: {took:?}");
    assert!(took < Duration::from_millis(300), "the bracket rebuilt from its parameter in {took:?}");
}

#[test]
#[ignore = "a measurement of time: run by hand in release - cargo test --release -p qymcad-mcp --test bench -- --ignored --test-threads=1"]
fn a_picture_768_wide_is_under_500_ms() {
    if !release_only() {
        return;
    }
    let took = median(perforated, |ctx| {
        let _ = call(ctx, "render", json!({ "view": "iso", "width": 768, "height": 576 }));
    });
    eprintln!("a picture 768 x 576 of the perforated plate: {took:?}");
    assert!(took < Duration::from_millis(500), "a picture 768 wide took {took:?}");
}
