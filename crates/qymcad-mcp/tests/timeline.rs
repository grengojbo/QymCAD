//! PARTS AND THE TIMELINE through the server: deleting the base of a part leaves what stood on it red, and undo brings
//! it back; a suppressed fillet leaves the block whole until it is restored; a feature given new sizes is rebuilt;
//! a second part takes what is laid in it, and stepping back into the first is no step of undo.

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

/// The block with its top front edge rounded 2: the keys of the sketch, the extrusion and the fillet.
struct Rounded {
    sketch: Value,
    extrusion: Value,
    fillet: Value,
}

fn rounded_block(ctx: &mut Ctx) -> Rounded {
    let made = ok(call(ctx, "create_sketch", json!({ "plane": "xy" })));
    let _ = ok(call(ctx, "sketch_add", json!({ "sketch": made["sketch"], "entities": [{ "rect": { "from": [0, 0], "to": [40, 30] } }] })));
    let laid = ok(call(ctx, "extrude", json!({ "sketch": made["sketch"], "distance": 10 })));
    let round = ok(call(ctx, "fillet", json!({ "edges": { "between": { "one": "top", "other": "front" } }, "radius": 2 })));
    Rounded { sketch: made["sketch"].clone(), extrusion: laid["feature"].clone(), fillet: round["feature"].clone() }
}

/// 12000 less the quarter of a 2 x 2 square along the 40 of the edge, less the quarter disc: 12000 - 160 (1 - pi/4).
fn rounded_volume() -> i64 {
    (12000.0 - 160.0 * (1.0 - std::f64::consts::FRAC_PI_4)).round() as i64
}

/// THE BASE DELETED: the fillet standing on it stays and turns red, named in the answer; undo brings the block back.
/// Deleted with its dependents, nothing of the part is left.
#[test]
fn a_deleted_base_leaves_its_dependents_red() {
    let mut ctx = Ctx::blank();
    let r = rounded_block(&mut ctx);
    let gone = ok(call(&mut ctx, "delete_feature", json!({ "feature": r.extrusion })));
    let red: Vec<&Value> = gone["red"].as_array().expect("red").iter().map(|f| &f["key"]).collect();
    assert!(red.contains(&&r.fillet), "the fillet on a deleted base is not named red: {gone}");
    let undone = ok(call(&mut ctx, "undo", json!({})));
    assert_eq!(undone["undone"]["step"], "status-delete-feature");
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    assert_eq!(volumes(&doc["document"]), [rounded_volume()], "undo did not bring the block back: {doc}");

    let all = ok(call(&mut ctx, "delete_feature", json!({ "feature": r.extrusion, "dependents": "go" })));
    assert_eq!(volumes(&all), Vec::<i64>::new(), "{all}");
    assert_eq!(all["red"], json!([]), "{all}");
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    let left: Vec<&Value> = doc["document"]["features"].as_array().expect("features").iter().map(|f| &f["key"]).collect();
    assert!(!left.contains(&&r.fillet) && left.contains(&&r.sketch), "the cascade took the wrong features: {doc}");
    let sketch_gone = ok(call(&mut ctx, "delete_feature", json!({ "feature": r.sketch })));
    assert_eq!(sketch_gone["deleted"], r.sketch);
    let missing = call(&mut ctx, "delete_feature", json!({ "feature": 987_654 }));
    assert_eq!(missing["error"]["code"], "no-feature", "{missing}");
}

/// A SUPPRESSED FILLET leaves the block whole and stays in the timeline; restored, the edge is round again.
#[test]
fn a_suppressed_feature_is_left_out_until_restored() {
    let mut ctx = Ctx::blank();
    let r = rounded_block(&mut ctx);
    let off = ok(call(&mut ctx, "suppress_feature", json!({ "feature": r.fillet, "state": "suppress" })));
    assert_eq!(volumes(&off), [12000], "{off}");
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    let kept = doc["document"]["features"].as_array().expect("features").iter().find(|f| f["key"] == r.fillet).expect("the fillet stays in the timeline").clone();
    assert_eq!(kept["suppressed"], json!(true), "{kept}");
    let on = ok(call(&mut ctx, "suppress_feature", json!({ "feature": r.fillet, "state": "restore" })));
    assert_eq!(volumes(&on), [rounded_volume()], "{on}");
}

/// A FEATURE GIVEN NEW SIZES is rebuilt: the extrusion made 20 doubles the block under the same fillet, an expression
/// keeps following its parameter, and a size the feature does not have is refused with the ones it has.
#[test]
fn a_feature_given_new_sizes_is_rebuilt() {
    let mut ctx = Ctx::blank();
    let r = rounded_block(&mut ctx);
    let taller = ok(call(&mut ctx, "edit_feature", json!({ "feature": r.extrusion, "values": { "height": 20 } })));
    assert_eq!(volumes(&taller), [rounded_volume() + 12000], "{taller}");
    let height = taller["sizes"].as_array().and_then(|s| s.iter().find(|d| d["key"] == "height")).cloned().expect("the height among the sizes");
    assert_eq!(height["value"], json!(20.0), "the answer reads the old height: {taller}");
    assert_eq!(height["expr"], Value::Null, "a typed number reads as an expression: {taller}");
    let _ = ok(call(&mut ctx, "set_parameter", json!({ "name": "r", "expr": "3" })));
    let bigger = ok(call(&mut ctx, "edit_feature", json!({ "feature": r.fillet, "values": { "radius": "r" } })));
    let want = (24000.0 - 360.0 * (1.0 - std::f64::consts::FRAC_PI_4)).round() as i64;
    assert_eq!(volumes(&bigger), [want], "{bigger}");
    let follows = ok(call(&mut ctx, "set_parameter", json!({ "name": "r", "expr": "1" })));
    let want = (24000.0 - 40.0 * (1.0 - std::f64::consts::FRAC_PI_4)).round() as i64;
    assert_eq!(volumes(&follows), [want], "the radius did not follow r: {follows}");
    let wrong = call(&mut ctx, "edit_feature", json!({ "feature": r.fillet, "values": { "depth": 5 } }));
    assert_eq!(wrong["error"]["code"], "no-size", "{wrong}");
    assert!(wrong["error"]["message"].as_str().is_some_and(|m| m.contains("radius")), "the sizes it has are not named: {wrong}");
}

/// A SECOND PART takes what is laid in it; stepping back into the first is no step of undo, and a tool given no body
/// then works on the first part's body.
#[test]
fn a_second_part_takes_what_is_laid_in_it() {
    let mut ctx = Ctx::blank();
    let first = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let first_part = first["bodies"][0]["part"].clone();
    let made = ok(call(&mut ctx, "add_part", json!({ "name": "Lid" })));
    let lid = made["part"].clone();
    assert_ne!(lid, first_part);
    let laid = ok(call(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 2, "at": [0, 0, 20] })));
    let in_part = |part: &Value| laid["bodies"].as_array().expect("bodies").iter().filter(|b| b["part"] == *part).map(|b| b["volume"].as_f64().unwrap_or(0.0).round() as i64).collect::<Vec<i64>>();
    assert_eq!(in_part(&lid), [800], "the lid did not go into its own part: {laid}");
    assert_eq!(in_part(&first_part), [1000], "the first part changed: {laid}");

    let steps = ctx.doc.history().undo_names().len();
    let _ = ok(call(&mut ctx, "set_active_part", json!({ "part": first_part })));
    assert_eq!(ctx.doc.history().undo_names().len(), steps, "stepping into a part made a step");
    let round = ok(call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 1 })));
    let cube = round["bodies"].as_array().expect("bodies").iter().find(|b| b["part"] == first_part).expect("the first part's body").clone();
    assert!(cube["volume"].as_f64().expect("a volume") < 1000.0, "the fillet went elsewhere: {round}");
    let nowhere = call(&mut ctx, "set_active_part", json!({ "part": 987_654 }));
    assert_eq!(nowhere["error"]["code"], "no-part", "{nowhere}");
}
